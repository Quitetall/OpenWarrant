// SPDX-License-Identifier: Apache-2.0
//! Interface documents, read (t-3c16c W6; `docs/INTERFACES.md`).
//!
//! An interface is a Format 1.0 document of the `interface` profile, whose
//! `clause` records state what both sides of a boundary build against. This
//! is what the rest of `war` asks of one: its clauses, each with its status.
//! It reads through [`crate::vfs`], so it answers the same for this repository
//! and for another at its pin.

use std::collections::BTreeMap;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Report, Severity};
use crate::format_doc;
use crate::repo::Repository;
use crate::repos;
use openwarrant_core::format::FieldValue;

/// The profile an interface document names.
pub const PROFILE: &str = "interface";
/// The record type of a clause.
pub const CLAUSE: &str = "clause";
/// What this repository conforms to, by interface.
pub const BASELINE: &str = "docs/interfaces.lock";
/// How long one clause's `check` may run.
const CHECK_DEADLINE: Duration = Duration::from_secs(600);

/// One clause, with the document that holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Clause {
    pub id: String,
    pub title: String,
    /// `agreed`, `tbd` or `tbr`; empty when the document leaves it out, which
    /// `war check` refuses.
    pub status: String,
    pub verify: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    /// The repository-relative path of its document.
    pub source: String,
    pub line: usize,
    /// The digest of its own text, so a change to the clause beside it does
    /// not reach anyone who depends on this one.
    pub digest: String,
}

impl Clause {
    /// Whether it is undecided: what holds work that depends on it.
    #[must_use]
    pub fn is_tbd(&self) -> bool {
        self.status == "tbd"
    }

    /// Whether it is still open at all: `tbd`, or `tbr`.
    #[must_use]
    pub fn is_open(&self) -> bool {
        matches!(self.status.as_str(), "tbd" | "tbr")
    }

    /// `tbd (owner hpr, due 2026-11-20)`.
    #[must_use]
    pub fn standing(&self) -> String {
        let mut detail = Vec::new();
        if let Some(o) = &self.owner {
            detail.push(format!("owner {o}"));
        }
        if let Some(d) = &self.due {
            detail.push(format!("due {d}"));
        }
        if detail.is_empty() {
            self.status.clone()
        } else {
            format!("{} ({})", self.status, detail.join(", "))
        }
    }
}

fn one(fields: &std::collections::BTreeMap<String, FieldValue>, key: &str) -> Option<String> {
    match fields.get(key) {
        Some(FieldValue::One(s)) => Some(s.trim().to_owned()),
        Some(FieldValue::Many(v)) => Some(v.join(", ")),
        None => None,
    }
    .filter(|s| !s.is_empty())
}

/// One interface document of a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Interface {
    pub source: String,
    pub version: Option<String>,
    pub owner: Option<String>,
    pub parties: Vec<String>,
    /// The digest of its bytes: the revision a side says it conforms to.
    pub digest: String,
    pub clauses: Vec<Clause>,
}

/// Every interface document of `repo`, in source order.
#[must_use]
pub fn all(repo: &Repository) -> Vec<Interface> {
    format_doc::load(repo)
        .docs
        .into_iter()
        .filter(|d| d.doc.profile == PROFILE && d.admitted())
        .map(|d| Interface {
            version: one(&d.doc.fields, "version"),
            owner: one(&d.doc.fields, "owner"),
            parties: one(&d.doc.fields, "parties")
                .map(|p| p.split(',').map(|s| s.trim().to_owned()).collect())
                .unwrap_or_default(),
            digest: openwarrant_core::format::sha256(&d.bytes),
            clauses: d
                .doc
                .units
                .iter()
                .filter(|u| u.record_type == CLAUSE)
                .map(|u| Clause {
                    id: u.id.clone(),
                    title: u.title.clone(),
                    status: one(&u.fields, "status").unwrap_or_default(),
                    verify: one(&u.fields, "verify").unwrap_or_default(),
                    check: one(&u.fields, "check"),
                    owner: one(&u.fields, "owner"),
                    due: one(&u.fields, "due"),
                    source: d.source.clone(),
                    line: u.line,
                    digest: openwarrant_core::format::sha256(
                        d.bytes.get(u.start..u.end).unwrap_or_default(),
                    ),
                })
                .collect(),
            source: d.source,
        })
        .collect()
}

/// The clause `id` of `repo`.
#[must_use]
pub fn clause(repo: &Repository, id: &str) -> Option<Clause> {
    all(repo)
        .into_iter()
        .flat_map(|i| i.clauses)
        .find(|c| c.id == id)
}

/// Whether anything here could concern an interface: one read costs a corpus
/// walk, and most repositories have none. A hosted run asks the same files.
#[must_use]
pub fn in_use(repo: &Repository) -> bool {
    crate::vfs::is_file(repo.root.join("profiles").join(format!("{PROFILE}.toml")))
        || repos::declared(&repo.root).is_ok_and(|all| !all.is_empty())
}

/// Every interface this repository reaches: its own, then each named
/// repository's at its pin, labelled `hpr:<path>`. What could not be read is
/// the second list, in words.
#[must_use]
pub fn reached(repo: &Repository) -> (Vec<(String, Interface)>, Vec<String>) {
    let mut out: Vec<(String, Interface)> = all(repo)
        .into_iter()
        .map(|i| (i.source.clone(), i))
        .collect();
    let mut unread = Vec::new();
    for name in repos::pinned(&repo.root) {
        let name = &name;
        match repos::through_pin(&repo.root, name, None, all) {
            Ok((_, found)) => {
                out.extend(
                    found
                        .into_iter()
                        .map(|i| (format!("{name}:{}", i.source), i)),
                );
            }
            Err(why) => unread.push(why),
        }
    }
    (out, unread)
}

/// `war check`: the open clauses, counted; those past `due`, flagged; a
/// `[repos]` table that does not parse, refused. Nothing of another repository
/// can fail this: what cannot be read of it is a warning.
pub fn check(repo: &Repository, today: chrono::NaiveDate, report: &mut Report) {
    if let Err(why) = repos::declared(&repo.root) {
        report.push(Diagnostic::error(
            repos::CONFIG_RULE,
            crate::init::CONFIG_FILE.to_owned(),
            why,
        ));
        return;
    }
    if !in_use(repo) {
        return;
    }
    let (reached, unread) = reached(repo);
    let (mut tbd, mut tbr) = (0, 0);
    for (label, interface) in &reached {
        for c in &interface.clauses {
            match c.status.as_str() {
                "tbd" => tbd += 1,
                "tbr" => tbr += 1,
                _ => continue,
            }
            let due = c
                .due
                .as_deref()
                .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
            if due.is_some_and(|d| d < today) {
                report.push(Diagnostic::warn(
                    "interface.overdue",
                    label.clone(),
                    format!("{} is {}, past its date", c.id, c.standing()),
                ));
            }
        }
    }
    if !reached.is_empty() {
        report.push(Diagnostic::pass(
            "interface.open",
            format!("{tbd} tbd, {tbr} tbr across {} interface(s)", reached.len()),
        ));
    }
    for why in unread {
        report.push(Diagnostic::warn(
            repos::UNKNOWN_RULE,
            crate::init::CONFIG_FILE.to_owned(),
            format!("UNKNOWN: {why}"),
        ));
    }
}

/// What running one clause came to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClauseRun {
    /// `CL-step`, or `hpr:CL-step`.
    pub clause: String,
    pub source: String,
    /// `PASS`, `FAIL` or `UNKNOWN`.
    pub result: String,
    pub detail: String,
}

/// `Ok` for a run that finished, with whether it passed; `Err` when nothing
/// can be said of it.
fn run_check(root: &camino::Utf8Path, command: &str) -> Result<(bool, String), String> {
    if !crate::vfs::is_disk() {
        return Err("a hosted or recording run starts no process".to_owned());
    }
    let mut child = Command::new("sh")
        .args(["-c", command])
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not run sh: {e}"))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let word = status
                    .code()
                    .map_or_else(|| "a signal".to_owned(), |c| format!("exit {c}"));
                return Ok((status.success(), word));
            }
            Ok(None) if started.elapsed() > CHECK_DEADLINE => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("ran past {}s", CHECK_DEADLINE.as_secs()));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(format!("could not wait for it: {e}")),
        }
    }
}

/// `war check --clauses`: run each clause's `check` here, its own and those
/// of the interfaces it reaches at their pins. A clause with none, or one not
/// decided, reads UNKNOWN: nothing was established, and nothing failed.
#[must_use]
pub fn run_clauses(repo: &Repository) -> (Report, Vec<ClauseRun>) {
    let mut report = Report::default();
    let (reached, unread) = reached(repo);
    for why in unread {
        report.push(Diagnostic::unknown(
            repos::UNKNOWN_RULE,
            crate::init::CONFIG_FILE.to_owned(),
            why,
        ));
    }
    let mut runs = Vec::new();
    for (label, interface) in &reached {
        let prefix = label
            .split_once(':')
            .map_or(String::new(), |(n, _)| format!("{n}:"));
        for c in &interface.clauses {
            let clause = format!("{prefix}{}", c.id);
            let outcome = match (&c.check, c.is_tbd()) {
                (_, true) => Err(format!("{} is not decided", c.id)),
                (None, _) => Err(format!("{} names no check ({})", c.id, c.verify)),
                (Some(command), false) => run_check(&repo.root, command),
            };
            let (result, detail) = match outcome {
                Ok((true, word)) => {
                    report.push(Diagnostic::pass("clause.pass", format!("{clause}: {word}")));
                    ("PASS", word)
                }
                Ok((false, word)) => {
                    report.push(Diagnostic::error(
                        "clause.fail",
                        label.clone(),
                        format!("{clause}: {word}"),
                    ));
                    ("FAIL", word)
                }
                Err(why) => {
                    report.push(Diagnostic::unknown(
                        "clause.unknown",
                        label.clone(),
                        format!("{clause}: {why}"),
                    ));
                    ("UNKNOWN", why)
                }
            };
            runs.push(ClauseRun {
                clause,
                source: label.clone(),
                result: result.to_owned(),
                detail,
            });
        }
    }
    (report, runs)
}

/// One revision a repository says it conforms to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Conformance {
    pub digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

type Baseline = BTreeMap<String, Conformance>;

fn read_baseline(repo: &Repository) -> Result<Baseline, String> {
    let path = repo.root.join(BASELINE);
    match crate::vfs::read_to_string(&path) {
        Ok(text) => toml::from_str(&text).map_err(|e| format!("{BASELINE}: {}", e.message())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Baseline::new()),
        Err(e) => Err(format!("{BASELINE}: {e}")),
    }
}

/// `war check --clauses --record`: say this repository conforms to every
/// interface it reaches, as they stand. A clause that failed records nothing:
/// the caller checks that first. Returns what it wrote.
pub fn record_baseline(repo: &Repository) -> Result<Baseline, String> {
    let (reached, unread) = reached(repo);
    if let Some(why) = unread.first() {
        return Err(why.clone());
    }
    let baseline: Baseline = reached
        .into_iter()
        .map(|(label, i)| {
            (
                label,
                Conformance {
                    digest: i.digest,
                    version: i.version,
                },
            )
        })
        .collect();
    let text = toml::to_string(&baseline).map_err(|e| e.to_string())?;
    let path = repo.root.join(BASELINE);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{dir}: {e}"))?;
    }
    crate::compile::atomic::write(&path, text.into_bytes()).map_err(|e| format!("{path}: {e}"))?;
    Ok(baseline)
}

fn brief(digest: &str) -> &str {
    let hex = digest.strip_prefix("sha256:").unwrap_or(digest);
    &hex[..hex.len().min(12)]
}

/// The one line `war status` says of what this repository conforms to; `None`
/// when no interface concerns it.
#[must_use]
pub fn drift_line(repo: &Repository) -> Option<String> {
    if !in_use(repo) {
        return None;
    }
    let (reached, unread) = reached(repo);
    if reached.is_empty() && unread.is_empty() {
        return None;
    }
    let baseline = match read_baseline(repo) {
        Ok(b) => b,
        Err(why) => return Some(format!("Interfaces: UNKNOWN ({why})")),
    };
    let mut drifted = Vec::new();
    for (label, i) in &reached {
        match baseline.get(label) {
            Some(c) if c.digest == i.digest => {}
            Some(c) => drifted.push(format!(
                "{label} is {} ({}), this side conforms to {} ({})",
                i.version.as_deref().unwrap_or("?"),
                brief(&i.digest),
                c.version.as_deref().unwrap_or("?"),
                brief(&c.digest),
            )),
            None => drifted.push(format!("{label} has no baseline")),
        }
    }
    let unknown = unread.len();
    Some(match (drifted.first(), unknown) {
        (Some(first), _) if drifted.len() > 1 => {
            format!("Interfaces: drift: {first} (+{})", drifted.len() - 1)
        }
        (Some(first), _) => format!("Interfaces: drift: {first}"),
        (None, 0) => format!("Interfaces: {} conform", reached.len()),
        (None, n) => format!("Interfaces: {} conform, {n} UNKNOWN", reached.len()),
    })
}

/// How the run should end, for a caller that records a baseline after it.
#[must_use]
pub fn any_failed(report: &Report) -> bool {
    report.count(Severity::Error) > 0
}
