// SPDX-License-Identifier: Apache-2.0
//! The per-user list of OpenWarrant repositories (OW-WAR-0115).
//!
//! `war` from anywhere opens the hub, and the hub needs to know which
//! repositories this user works in. Nobody registers them: any `war` command
//! that opened a repository touches the list, best-effort — a failure to
//! write it never changes a command's result. `war projects` lists, adds and
//! forgets entries.
//!
//! The list stores where a project is and when it was last seen, never what
//! it says: every fact the hub shows is read from that project's own records
//! through the CLI's functions. A path that no longer holds a repository is
//! reported as missing, not dropped — a moved checkout should be found, not
//! forgotten silently.
//!
//! `$XDG_CONFIG_HOME/openwarrant/projects.toml` (default `~/.config/…`).
//! `OPENWARRANT_NO_PROJECTS=1` turns the touching off.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Report};

pub const SCHEMA: &str = "oh.war/projects/v1";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Projects {
    #[serde(default = "schema")]
    pub schema: String,
    /// Absolute repository root → last seen (RFC 3339).
    #[serde(default)]
    pub projects: BTreeMap<String, String>,
}

fn schema() -> String {
    SCHEMA.to_owned()
}

/// Where the list lives, or `None` when neither XDG_CONFIG_HOME nor HOME is
/// set (then there is no list, and nothing is touched).
#[must_use]
pub fn path() -> Option<Utf8PathBuf> {
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(Utf8PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .filter(|s| !s.is_empty())
                .map(|h| Utf8PathBuf::from(h).join(".config"))
        })?;
    Some(base.join("openwarrant").join("projects.toml"))
}

#[must_use]
pub fn load() -> Projects {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| toml::from_str(&t).ok())
        .unwrap_or_else(|| Projects {
            schema: SCHEMA.to_owned(),
            projects: BTreeMap::new(),
        })
}

fn save(p: &Projects) -> Result<(), String> {
    let path = path().ok_or("no XDG_CONFIG_HOME or HOME")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = toml::to_string_pretty(p).map_err(|e| e.to_string())?;
    // Write-then-rename: a crash never leaves half a list.
    let tmp = path.with_extension(format!("toml.{}", std::process::id()));
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

fn now() -> String {
    crate::gate_cmd::receipt::now_rfc3339_public()
}

/// Remember a repository this user just used. Best-effort by design: the
/// caller ignores the result, and the opt-out writes nothing.
pub fn touch(root: &Utf8Path) {
    if std::env::var("OPENWARRANT_NO_PROJECTS").is_ok_and(|v| !v.is_empty() && v != "0") {
        return;
    }
    let Ok(root) = root.canonicalize_utf8() else {
        return;
    };
    let mut p = load();
    p.projects.insert(root.to_string(), now());
    let _ = save(&p);
}

/// One row of `war projects`.
#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub root: String,
    pub last_seen: String,
    pub missing: bool,
    pub program: Option<String>,
    pub namespace: Option<String>,
    /// The checked-out branch, as git names it.
    pub branch: Option<String>,
    /// The SAS revision in force.
    pub sas: Option<String>,
    /// Acts awaiting a human signature: `war sign --list`'s count.
    pub pending: Option<usize>,
    /// Open questions that block a Warrant.
    pub blocking: Option<usize>,
    /// Warrants, and how many of them are resolved.
    pub warrants: Option<usize>,
    pub resolved: Option<usize>,
    /// Roadmap phases, and how many are achieved: the roadmap record's when
    /// there is one (OW-WAR-0114), the §98 Objectives otherwise.
    pub phases: Option<usize>,
    pub phases_achieved: Option<usize>,
    /// The project's `war watch` fingerprint when these facts were read: a
    /// reader re-reads a row only when it moves.
    #[serde(skip)]
    pub fingerprint: Option<u64>,
    /// Why a fact above could not be read, when one could not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unreadable: Option<String>,
}

/// Every project with its facts read: `war projects`.
#[must_use]
pub fn rows() -> Vec<Row> {
    let mut rows = listed();
    for r in &mut rows {
        if !r.missing {
            read_facts(r);
        }
    }
    rows
}

/// Every project, named but with no fact read yet: what the hub shows at
/// once, before it reads each project one idle tick at a time.
#[must_use]
pub fn listed() -> Vec<Row> {
    load()
        .projects
        .into_iter()
        .map(|(root, last_seen)| {
            let config = Utf8Path::new(&root).join(crate::init::CONFIG_FILE);
            let parsed = std::fs::read_to_string(&config)
                .ok()
                .and_then(|t| toml::from_str::<toml::Value>(&t).ok());
            let field = |k: &str| {
                parsed
                    .as_ref()
                    .and_then(|v| v.get("project")?.get(k)?.as_str().map(str::to_owned))
            };
            Row {
                missing: parsed.is_none(),
                program: field("name"),
                namespace: field("namespace"),
                branch: None,
                sas: None,
                pending: None,
                blocking: None,
                warrants: None,
                resolved: None,
                phases: None,
                phases_achieved: None,
                fingerprint: None,
                unreadable: None,
                root,
                last_seen,
            }
        })
        .collect()
}

/// The fingerprint `war watch` keeps for a project, or `None` when it no
/// longer opens.
#[must_use]
pub fn fingerprint(root: &str) -> Option<u64> {
    let repo = crate::repo::Repository::open(Utf8PathBuf::from(root)).ok()?;
    Some(crate::watch::fingerprint(&crate::watch::watched_dirs(
        &repo,
    )))
}

/// Every fact on a row, read through the functions the CLI answers with in
/// that project — never stored in the list, never computed here.
pub fn read_facts(row: &mut Row) {
    let root = Utf8PathBuf::from(&row.root);
    row.unreadable = None;
    let repo = match crate::repo::Repository::open(root.clone()) {
        Ok(r) => r,
        Err(e) => {
            row.unreadable = Some(e.to_string());
            return;
        }
    };
    row.branch = std::process::Command::new("git")
        .args(["-C", root.as_str(), "rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());
    row.fingerprint = Some(crate::watch::fingerprint(&crate::watch::watched_dirs(
        &repo,
    )));
    row.sas = repo.latest_sas_revision().ok().flatten().map(|r| r.version);
    if let Ok((_, view)) = crate::roadmap_cmd::view(&repo)
        && !view.phases.is_empty()
    {
        row.phases = Some(view.phases.len());
        row.phases_achieved = Some(
            view.phases
                .iter()
                .filter(|p| p.achieved == "achieved")
                .count(),
        );
    }
    match crate::console::board(&repo) {
        Ok(b) => {
            row.pending = Some(b.acts.len());
            row.blocking = Some(b.questions.iter().filter(|q| q.blocking).count());
        }
        Err(e) => row.unreadable = Some(e.to_string()),
    }
    match crate::status::build(&repo) {
        Ok(s) => {
            if row.phases.is_none() {
                let states: Vec<String> = s
                    .objectives
                    .iter()
                    .filter_map(|o| {
                        serde_json::to_value(o).ok()?["achieved"]["state"]
                            .as_str()
                            .map(str::to_owned)
                    })
                    .collect();
                row.phases = Some(s.objectives.len());
                row.phases_achieved = Some(states.iter().filter(|s| *s == "achieved").count());
            }
            row.warrants = Some(s.warrants.len());
            row.resolved = Some(
                s.warrants
                    .iter()
                    .filter(|w| w.rung == openwarrant_core::status::WarrantRung::Resolved)
                    .count(),
            );
        }
        Err(e) => row.unreadable = Some(e.to_string()),
    }
}

/// `war projects [--add <path> | --forget <path>]`.
pub fn run(add: Option<&Utf8Path>, forget: Option<&Utf8Path>) -> (Report, Vec<Row>) {
    let mut report = Report::default();
    if let Some(a) = add {
        match a.canonicalize_utf8() {
            Ok(root) if root.join(crate::init::CONFIG_FILE).is_file() => {
                let mut p = load();
                p.projects.insert(root.to_string(), now());
                match save(&p) {
                    Ok(()) => report.push(Diagnostic::pass("projects.added", root.to_string())),
                    Err(e) => report.push(Diagnostic::error(
                        "projects.unwritable",
                        root.to_string(),
                        e,
                    )),
                }
            }
            _ => report.push(Diagnostic::error(
                "projects.not-a-repository",
                a.to_string(),
                format!("{a} holds no openwarrant.toml"),
            )),
        }
    }
    if let Some(f) = forget {
        let key = f
            .canonicalize_utf8()
            .map_or_else(|_| f.to_string(), |p| p.to_string());
        let mut p = load();
        if p.projects.remove(&key).is_some() {
            match save(&p) {
                Ok(()) => report.push(Diagnostic::pass("projects.forgotten", key)),
                Err(e) => report.push(Diagnostic::error("projects.unwritable", key, e)),
            }
        } else {
            report.push(Diagnostic::error(
                "projects.unknown",
                key.clone(),
                format!("{key} is not on the list; `war projects` shows it"),
            ));
        }
    }
    let rows = rows();
    for r in &rows {
        if r.missing {
            report.push(Diagnostic::warn(
                "projects.missing",
                r.root.clone(),
                format!(
                    "{} holds no openwarrant.toml any more; `war projects --forget {}` removes it",
                    r.root, r.root
                ),
            ));
        }
    }
    (report, rows)
}

#[must_use]
pub fn render(rows: &[Row]) -> String {
    if rows.is_empty() {
        return "no projects yet: run any `war` command inside a repository and it is remembered\n"
            .to_owned();
    }
    let mut s = String::new();
    for r in rows {
        s.push_str(&format!(
            "{:<28} {:<6} {}{}\n",
            r.program.as_deref().unwrap_or("?"),
            r.namespace.as_deref().unwrap_or(""),
            r.root,
            if r.missing { "  (missing)" } else { "" }
        ));
        if !r.missing {
            s.push_str(&format!("    {}\n", summary(r)));
        }
    }
    s
}

/// One line of a project's state, for `war projects` and the hub's row.
#[must_use]
pub fn summary(r: &Row) -> String {
    let n = |v: Option<usize>| v.map_or_else(|| "?".to_owned(), |n| n.to_string());
    format!(
        "{} · SAS {} · {} awaiting a signature · {} blocking question(s) · phases {} of {} achieved · {} of {} Warrants resolved{}",
        r.branch.as_deref().unwrap_or("?"),
        r.sas.as_deref().unwrap_or("none"),
        n(r.pending),
        n(r.blocking),
        n(r.phases_achieved),
        n(r.phases),
        n(r.resolved),
        n(r.warrants),
        r.unreadable
            .as_deref()
            .map_or_else(String::new, |e| format!(" · unreadable: {e}"))
    )
}
