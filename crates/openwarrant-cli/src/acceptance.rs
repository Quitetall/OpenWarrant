// SPDX-License-Identifier: Apache-2.0
//! Acceptance validity before merge (OW-WAR-0134): is the candidate about to
//! land still the candidate a human accepted?
//!
//! A resolution is true of what it saw. Its `[locator]` names the commit whose
//! tree held the delivered bytes when the human signed (OW-ADR-0021), and the
//! record never changes after that. What can change is the tree around it: a
//! rebase, a merge from the target branch, a conflict resolved by hand. The
//! pinned deliverables are guarded already (`deliverable.digest-drift`), and a
//! recompiled contract is `resolution.stale`. Every OTHER file the accepted
//! behaviour rests on was guarded by nothing, and this module is that guard.
//!
//! # What it answers, per resolved Warrant
//!
//! - `unchanged`: every path that moved between the accepted tree and the
//!   candidate is out of scope.
//! - `moved` (`acceptance.candidate-moved`): an in-scope path moved, or a
//!   pinned deliverable's bytes no longer match the digest the human signed
//!   for. Each path is named. The resolution is NOT edited, disputed or
//!   annulled: it is still true of the candidate it accepted.
//! - `UNKNOWN` (`acceptance.unknown`): no locator (resolved before
//!   OW-ADR-0021), or a locator commit git cannot read. Law 15: never
//!   "unchanged", and never an error about the resolution either.
//! - `landed` (`--base <rev>` only): the resolution already exists at the
//!   base, so it merged earlier and is not a candidate before merge.
//!
//! # Scope (OW-WAR-0133 Q-001 (c))
//!
//! A Gate Definition may declare the paths it reads as `inputs` globs; a path
//! is in scope when a glob of a gate the Warrant cites matches it. A gate that
//! declares no inputs, or a Warrant that cites a gate the registry does not
//! hold, falls back to the whole tree. The field is read straight from the
//! definition file, so this module needs nothing from OW-WAR-0133's code: until
//! a definition declares `inputs`, every Warrant is judged on the whole tree,
//! which is the strict reading, not the lenient one. Pinned deliverables are
//! always in scope.
//!
//! The Warrant's own records are not part of the candidate: its directory
//! (contract, verifications, journal, receipts: `resolution.stale`, the
//! resolution's manifest digest and `war check` guard those), the corpus
//! projections under `<warrants>/generated/` and `docs/generated/` (rewritten
//! by `war compile`, drift-checked by `war check --generated`), and its signed
//! responses under
//! `docs/authority/responses/<alias>.*`. Without that, committing the
//! resolution would itself read as a move.
//!
//! # What clears a move (Q-001, answered (b))
//!
//! An independent verifier re-establishes every declared obligation on the new
//! candidate, through the existing `war verify --response` seam. No new record
//! kind: the tool reads the verification files at the candidate and counts one
//! only when (1) it is admissible and permits `satisfied` (`established`, or
//! accepted with residual risk, as §38.6 reads it for the resolution), and (2) the Warrant's
//! journal at the candidate records ingesting exactly those bytes AFTER the
//! resolution was recorded. The commit that last wrote each such file names the
//! tree the verifier examined; the comparison then starts from its parent, so
//! anything committed alongside or after the verification is still judged.
//! A second in-scope commit therefore raises the finding again, and the human's
//! acceptance carries forward only as far as a verifier followed it.
//!
//! A pinned deliverable whose bytes moved is Q-001 (a): the human re-accepts,
//! which today is a signed correction (`war correct`). Re-verification never
//! clears it; the correction chain's head does.
//!
//! # Why this is in `war pins` and not `war check`
//!
//! `war check` reads no git and stays offline and deterministic. This answer
//! is about history, so it lives beside `war pins --history`, which already
//! asks git whether a pin verifies at its locator.

use std::collections::BTreeMap;

use camino::Utf8Path;
use serde::Serialize;

use crate::diagnostic::{Diagnostic, Report, Severity};
use crate::repo::{RepoError, Repository};

pub const SCHEMA: &str = "oh.war/acceptance/v1";

/// The rule an in-scope move reports under.
pub const MOVED: &str = "acceptance.candidate-moved";
/// The rule for a question history cannot answer.
pub const UNKNOWN: &str = "acceptance.unknown";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Unchanged,
    Moved,
    Unknown,
    Landed,
}

/// One resolved Warrant against one candidate.
#[derive(Debug, Clone, Serialize)]
pub struct Acceptance {
    pub warrant: String,
    pub state: State,
    /// The locator commit the human accepted, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_at: Option<String>,
    /// The commit that recorded a re-verification of a later candidate, when
    /// one counts (Q-001 (b)).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reverified_at: Option<String>,
    /// The tree the candidate was compared with.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compared_from: Option<String>,
    /// `whole tree (<why>)` or `inputs: <globs>`.
    pub scope: String,
    /// In-scope paths that moved.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub in_scope: Vec<String>,
    /// Pinned deliverables whose bytes no longer match the signed digest.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pinned: Vec<String>,
    /// Paths that moved and are out of scope.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub out_of_scope: Vec<String>,
    /// Why the answer is UNKNOWN (or why it landed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    pub schema: &'static str,
    /// The revision asked about, as given, and the commit it names.
    pub candidate: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    pub warrants: Vec<Acceptance>,
}

/// Run git in `root`; stdout when it succeeded, `None` for anything else.
fn git(root: &Utf8Path, args: &[&str]) -> Option<Vec<u8>> {
    std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| o.stdout)
}

/// The commit `rev` names, or `None` when git cannot read one.
fn commit(root: &Utf8Path, rev: &str) -> Option<String> {
    let spec = format!("{rev}^{{commit}}");
    let out = git(root, &["rev-parse", "--verify", "--quiet", &spec])?;
    let sha = String::from_utf8_lossy(&out).trim().to_owned();
    (!sha.is_empty()).then_some(sha)
}

fn show(root: &Utf8Path, rev: &str, path: &str) -> Option<Vec<u8>> {
    git(root, &["show", &format!("{rev}:{path}")])
}

fn is_ancestor(root: &Utf8Path, older: &str, newer: &str) -> bool {
    git(root, &["merge-base", "--is-ancestor", older, newer]).is_some()
}

fn short(sha: &str) -> &str {
    &sha[..std::cmp::min(12, sha.len())]
}

/// Whether `path` matches `pattern`: `*` and `?` inside one segment, `**` for
/// any number of segments (including none), and a trailing `/` for everything
/// under a directory.
#[must_use]
pub fn glob_matches(pattern: &str, path: &str) -> bool {
    let pattern = pattern.trim().trim_start_matches("./");
    if let Some(dir) = pattern.strip_suffix('/') {
        return path.starts_with(&format!("{dir}/"));
    }
    let p: Vec<&str> = pattern.split('/').collect();
    let s: Vec<&str> = path.split('/').collect();
    segments(&p, &s)
}

fn segments(p: &[&str], s: &[&str]) -> bool {
    match p.split_first() {
        None => s.is_empty(),
        Some((&"**", rest)) => (0..=s.len()).any(|i| segments(rest, &s[i..])),
        Some((first, rest)) => match s.split_first() {
            Some((seg, srest)) => {
                segment(first.as_bytes(), seg.as_bytes()) && segments(rest, srest)
            }
            None => false,
        },
    }
}

fn segment(p: &[u8], s: &[u8]) -> bool {
    match p.split_first() {
        None => s.is_empty(),
        Some((b'*', rest)) => (0..=s.len()).any(|i| segment(rest, &s[i..])),
        Some((b'?', rest)) => !s.is_empty() && segment(rest, &s[1..]),
        Some((c, rest)) => s.first() == Some(c) && segment(rest, &s[1..]),
    }
}

/// Which paths a Warrant's acceptance rests on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Every path, and why the fallback applies.
    WholeTree(String),
    /// The union of the cited gates' declared `inputs`.
    Inputs(Vec<String>),
}

impl Scope {
    #[must_use]
    pub fn contains(&self, path: &str) -> bool {
        match self {
            Self::WholeTree(_) => true,
            Self::Inputs(globs) => globs.iter().any(|g| glob_matches(g, path)),
        }
    }

    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::WholeTree(why) => format!("whole tree ({why})"),
            Self::Inputs(globs) => format!("inputs: {}", globs.join(", ")),
        }
    }
}

/// `gate_id@version` → the definition's `inputs`, `None` when it declares none.
///
/// Read from the definition files directly: OW-WAR-0133 adds `inputs` to the
/// Gate Definition, and until it does, the key is simply absent and every
/// gate reads as "declares none".
fn gate_inputs(repo: &Repository) -> BTreeMap<String, Option<Vec<String>>> {
    let mut out = BTreeMap::new();
    let dir = repo.root.join(&repo.config.paths.gates);
    let Ok(entries) = dir.read_dir_utf8() else {
        return out;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.into_path();
        if !path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(doc) = openwarrant_core::structured::parse(&text) else {
            continue;
        };
        let (Some(id), Some(version)) = (doc.scalar("gate_id"), doc.scalar("version")) else {
            continue;
        };
        let inputs = doc
            .get("inputs")
            .and_then(openwarrant_core::structured::StructuredValue::as_list)
            .map(<[String]>::to_vec)
            .filter(|l| !l.is_empty());
        out.insert(format!("{id}@{version}"), inputs);
    }
    out
}

/// The scope rule for one Warrant, from the gates its assurance atoms cite.
fn scope_for(one: &crate::repo::Loaded, registry: &BTreeMap<String, Option<Vec<String>>>) -> Scope {
    let cited = crate::resolve::cited_gate_keys(one);
    if cited.is_empty() {
        return Scope::WholeTree("cites no gate".to_owned());
    }
    let mut globs: Vec<String> = Vec::new();
    for key in &cited {
        match registry.get(key) {
            Some(Some(inputs)) => {
                for g in inputs {
                    if !globs.contains(g) {
                        globs.push(g.clone());
                    }
                }
            }
            Some(None) => return Scope::WholeTree(format!("gate://{key} declares no inputs")),
            None => return Scope::WholeTree(format!("gate://{key} is not in the registry")),
        }
    }
    Scope::Inputs(globs)
}

fn after(later: &str, earlier: &str) -> bool {
    match (
        chrono::DateTime::parse_from_rfc3339(later),
        chrono::DateTime::parse_from_rfc3339(earlier),
    ) {
        (Ok(l), Ok(e)) => l > e,
        _ => false,
    }
}

/// Q-001 (b): the commit whose PARENT is the tree an independent verifier
/// re-established every declared obligation on, after the resolution. `None`
/// when any obligation lacks such a verification at the candidate.
fn reverified(
    repo: &Repository,
    rel_dir: &str,
    one: &crate::repo::Loaded,
    recorded_at: &str,
    locator: &str,
    candidate: &str,
) -> Option<(String, String)> {
    let root = &repo.root;
    let declared = crate::resolve::declared_obligations(one);
    if declared.is_empty() {
        return None;
    }
    let assurance = one
        .validated
        .as_ref()
        .map(|v| v.assurance_level.to_string())
        .unwrap_or_else(|| "basic".to_owned());
    let journal = show(
        root,
        candidate,
        &format!("{rel_dir}/{}", crate::journal_cmd::FILE),
    )?;
    // (obligation, record digest) ingested after the resolution was recorded.
    let mut ingested: Vec<(String, String)> = Vec::new();
    for line in String::from_utf8_lossy(&journal).lines() {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if event.get("type").and_then(|t| t.as_str())
            != Some(crate::journal_cmd::VERIFICATION_RECORDED)
        {
            continue;
        }
        let Some(at) = event.get("occurred_at").and_then(|t| t.as_str()) else {
            continue;
        };
        if !after(at, recorded_at) {
            continue;
        }
        let Some(payload) = event
            .get("payload")
            .and_then(|p| p.as_str())
            .and_then(|p| serde_json::from_str::<serde_json::Value>(p).ok())
        else {
            continue;
        };
        // A later file write does not bind an old observation to a new
        // candidate. Legacy journal events remain history, never a re-review.
        // Exact candidate/source comparison is the next subject-binding slice.
        let Some(reviewed) = payload
            .get("reviewed_subject")
            .and_then(|v| serde_json::from_value::<crate::verify::ReviewedSubject>(v.clone()).ok())
        else {
            continue;
        };
        if reviewed.contract_digest.is_empty() {
            continue;
        }
        if let (Some(o), Some(d)) = (
            payload.get("obligation").and_then(|v| v.as_str()),
            payload.get("record_digest").and_then(|v| v.as_str()),
        ) {
            ingested.push((o.to_owned(), d.to_owned()));
        }
    }
    let mut parents: Vec<String> = Vec::new();
    let mut last = String::new();
    for id in &declared {
        let path = format!("{rel_dir}/verifications/{id}.toml");
        let bytes = show(root, candidate, &path)?;
        let v: openwarrant_core::verification::Verification =
            toml::from_str(&String::from_utf8_lossy(&bytes)).ok()?;
        if v.obligation != *id
            || !v.disposition.permits_satisfied()
            || v.admissible_for(&assurance).is_err()
        {
            return None;
        }
        let digest = format!("sha256:{}", openwarrant_compiler::sha256_hex(&bytes));
        if !ingested.iter().any(|(o, d)| o == id && *d == digest) {
            return None;
        }
        let wrote = git(root, &["log", "-1", "--format=%H", candidate, "--", &path])?;
        let wrote = String::from_utf8_lossy(&wrote).trim().to_owned();
        if wrote.is_empty() || wrote == locator || !is_ancestor(root, locator, &wrote) {
            return None;
        }
        parents.push(commit(root, &format!("{wrote}^"))?);
        last = wrote;
    }
    let mut args = vec!["merge-base", "--octopus"];
    args.extend(parents.iter().map(String::as_str));
    let from = git(root, &args)?;
    let from = String::from_utf8_lossy(&from).trim().to_owned();
    if from.is_empty() || !is_ancestor(root, locator, &from) {
        return None;
    }
    Some((last, from))
}

/// Paths that are this Warrant's own records rather than the candidate.
fn own_record(path: &str, rel_dir: &str, warrants: &str, alias: &str) -> bool {
    path.starts_with(&format!("{rel_dir}/"))
        || path.starts_with(&format!("{}/generated/", warrants.trim_end_matches('/')))
        || path.starts_with("docs/generated/")
        || path.starts_with(&format!("docs/authority/responses/{alias}."))
}

/// `war pins --candidate <rev> [--base <rev>]`.
pub fn assess(
    repo: &Repository,
    candidate: &str,
    base: Option<&str>,
) -> Result<(Report, Candidate), RepoError> {
    let root = repo.root.clone();
    let mut report = Report::default();
    let mut out = Candidate {
        schema: SCHEMA,
        candidate: candidate.to_owned(),
        candidate_commit: None,
        base: base.map(str::to_owned),
        warrants: Vec::new(),
    };
    let Some(cand) = commit(&root, candidate) else {
        report.push(Diagnostic::new(
            Severity::Unknown,
            UNKNOWN,
            None,
            format!(
                "candidate {candidate}: git cannot read a commit there, so no acceptance can be \
                 compared with it"
            ),
        ));
        return Ok((report, out));
    };
    out.candidate_commit = Some(cand.clone());
    let base_commit = match base {
        None => None,
        Some(b) => {
            if let Some(c) = commit(&root, b) {
                Some(c)
            } else {
                report.push(Diagnostic::new(
                    Severity::Unknown,
                    UNKNOWN,
                    None,
                    format!(
                        "base {b}: git cannot read a commit there, so which acceptances are new \
                         cannot be told"
                    ),
                ));
                return Ok((report, out));
            }
        }
    };
    let registry = gate_inputs(repo);
    let warrants_dir = repo.config.paths.warrants.clone();

    for dir in repo.warrant_dirs()? {
        let Some(alias) = dir.file_name().map(str::to_owned) else {
            continue;
        };
        let Some(record) = repo.load_resolution(&dir)? else {
            continue;
        };
        let rel_dir = repo.relative(&dir);
        let rel_resolution = format!("{rel_dir}/resolution.toml");
        let one = repo.load_warrant(&dir)?;
        let scope = scope_for(&one, &registry);
        let mut a = Acceptance {
            warrant: alias.clone(),
            state: State::Unknown,
            accepted_at: None,
            reverified_at: None,
            compared_from: None,
            scope: scope.describe(),
            in_scope: Vec::new(),
            pinned: Vec::new(),
            out_of_scope: Vec::new(),
            reason: None,
        };

        if let Some(b) = &base_commit
            && show(&root, b, &rel_resolution).is_some()
        {
            a.state = State::Landed;
            a.reason = Some(format!("resolved at base {}; merged already", short(b)));
            report.push(Diagnostic::pass(
                "acceptance.landed",
                format!(
                    "{alias}: already resolved at base {} — landed earlier, not a candidate \
                     before merge",
                    short(b)
                ),
            ));
            out.warrants.push(a);
            continue;
        }

        let Some(locator) = record.locator.as_ref() else {
            let why = "no locator; resolved before OW-ADR-0021".to_owned();
            report.push(Diagnostic::unknown(
                UNKNOWN,
                rel_resolution.clone(),
                format!(
                    "{alias}: UNKNOWN ({why}). Which tree was accepted was never recorded, so \
                     whether {} moved it cannot be said; the resolution itself stands",
                    short(&cand)
                ),
            ));
            a.reason = Some(why);
            out.warrants.push(a);
            continue;
        };
        a.accepted_at = Some(locator.commit_sha.clone());
        if commit(&root, &locator.commit_sha).is_none() {
            let why = format!("commit {} not readable", short(&locator.commit_sha));
            report.push(Diagnostic::unknown(
                UNKNOWN,
                rel_resolution.clone(),
                format!(
                    "{alias}: UNKNOWN ({why}). The accepted tree is not in this repository's \
                     history (a squash, a rebase, or a shallow clone), so the candidate cannot \
                     be compared with it; the resolution itself stands"
                ),
            ));
            a.reason = Some(why);
            out.warrants.push(a);
            continue;
        }

        let from = match reverified(
            repo,
            &rel_dir,
            &one,
            &record.resolution.recorded_at,
            &locator.commit_sha,
            &cand,
        ) {
            Some((at, from)) => {
                a.reverified_at = Some(at);
                from
            }
            None => locator.commit_sha.clone(),
        };
        a.compared_from = Some(from.clone());

        let Some(diff) = git(
            &root,
            &["diff", "--name-only", "--no-renames", "-z", &from, &cand],
        ) else {
            let why = format!(
                "git could not diff {} against {}",
                short(&from),
                short(&cand)
            );
            report.push(Diagnostic::unknown(
                UNKNOWN,
                rel_resolution.clone(),
                format!("{alias}: UNKNOWN ({why})"),
            ));
            a.reason = Some(why);
            out.warrants.push(a);
            continue;
        };
        let changed: Vec<String> = diff
            .split(|b| *b == 0)
            .filter(|p| !p.is_empty())
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .collect();

        // Pinned deliverables are judged by the digest a human signed for (or
        // the head of its correction chain), never by the diff: a path dirty at
        // ingest was accepted with bytes that were never at the locator.
        let deliverables = repo.load_deliverables(&dir)?;
        let corrections = repo.load_corrections(&dir)?;
        let mut pinned_paths: Vec<String> = Vec::new();
        let mut unknowable: Vec<String> = Vec::new();
        for d in &deliverables.records {
            let recorded = d.provenance.as_ref().map(|p| p.content_digest.clone());
            match (d.content_addressed, recorded) {
                (true, Some(recorded)) => {
                    pinned_paths.push(d.target_ref.clone());
                    let head = crate::correct::head_for(&corrections, &d.id, &recorded)
                        .1
                        .unwrap_or(recorded);
                    let want = head.trim_start_matches("sha256:");
                    let now = show(&root, &cand, &d.target_ref)
                        .map(|b| openwarrant_compiler::sha256_hex(&b));
                    if now.as_deref() != Some(want) {
                        a.pinned.push(format!("{} ({})", d.target_ref, d.id));
                    }
                }
                _ => {
                    if locator.paths_dirty.iter().any(|p| p == &d.target_ref) {
                        unknowable.push(d.target_ref.clone());
                    }
                }
            }
        }
        let declared: Vec<&str> = deliverables
            .records
            .iter()
            .map(|d| d.target_ref.as_str())
            .collect();
        for path in changed {
            if own_record(&path, &rel_dir, &warrants_dir, &alias) || pinned_paths.contains(&path) {
                continue;
            }
            if scope.contains(&path) || declared.contains(&path.as_str()) {
                a.in_scope.push(path);
            } else {
                a.out_of_scope.push(path);
            }
        }

        if !a.in_scope.is_empty() || !a.pinned.is_empty() {
            a.state = State::Moved;
            let mut parts = Vec::new();
            if !a.in_scope.is_empty() {
                parts.push(format!(
                    "{} in-scope path(s) moved: {}",
                    a.in_scope.len(),
                    a.in_scope.join(", ")
                ));
            }
            if !a.pinned.is_empty() {
                parts.push(format!(
                    "pinned deliverable(s) no longer at the signed digest: {}",
                    a.pinned.join(", ")
                ));
            }
            let next = if a.pinned.is_empty() {
                format!(
                    "Q-001 (b): re-run the cited gates and have an independent verifier \
                     re-establish every obligation on this candidate (`war verify {alias} \
                     --performer <you>`, then `war verify {alias} --response <file>`); the \
                     human's acceptance carries forward"
                )
            } else {
                format!(
                    "Q-001 (a): a pinned deliverable changed, so a human re-accepts it \
                     (`war correct {alias} <D-id>` emits what they sign); re-verification \
                     does not clear this"
                )
            };
            report.push(Diagnostic::error(
                MOVED,
                rel_resolution.clone(),
                format!(
                    "{alias}: accepted at {}, compared from {}; candidate {} differs — {}. The \
                     resolution is not wrong: it is true of the candidate it accepted. {next}",
                    short(&locator.commit_sha),
                    short(&from),
                    short(&cand),
                    parts.join("; ")
                ),
            ));
        } else if !unknowable.is_empty() {
            let why = format!(
                "{} uncommitted at ingest and not content-addressed, so the accepted bytes \
                 were never recorded",
                unknowable.join(", ")
            );
            report.push(Diagnostic::unknown(
                UNKNOWN,
                rel_resolution.clone(),
                format!("{alias}: UNKNOWN ({why})"),
            ));
            a.reason = Some(why);
        } else {
            a.state = State::Unchanged;
            report.push(Diagnostic::pass(
                "acceptance.unchanged",
                format!(
                    "{alias}: unchanged at {} since {}{} — {} path(s) moved, none in scope \
                     ({})",
                    short(&cand),
                    short(&from),
                    if a.reverified_at.is_some() {
                        " (re-verified)"
                    } else {
                        ""
                    },
                    a.out_of_scope.len(),
                    a.scope
                ),
            ));
        }
        out.warrants.push(a);
    }
    if out.warrants.is_empty() {
        report.push(Diagnostic::pass(
            "acceptance.none",
            "no resolved Warrant to compare with the candidate".to_owned(),
        ));
    }
    report.note(
        "A moved acceptance is a finding about the candidate, not about the resolution: \
         nothing here writes resolution.toml, and the resolution stays true of the tree it \
         accepted.",
    );
    report.note(
        "Scope is each cited gate's declared `inputs` (OW-WAR-0133 Q-001 (c)); a gate that \
         declares none puts the whole tree in scope.",
    );
    Ok((report, out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs_match_within_and_across_segments() {
        assert!(glob_matches("src/**", "src/a/b.rs"));
        assert!(glob_matches("src/**", "src/a.rs"));
        assert!(glob_matches("**/*.rs", "a/b/c.rs"));
        assert!(glob_matches("src/*.rs", "src/a.rs"));
        assert!(glob_matches("docs/", "docs/x/y.md"));
        assert!(glob_matches("a?c.txt", "abc.txt"));
    }

    /// The refusals: a glob that matched everything would put every change in
    /// scope and make OBL-002's control meaningless.
    #[test]
    fn globs_refuse_what_they_do_not_name() {
        assert!(!glob_matches("src/**", "README.txt"));
        assert!(!glob_matches("src/*.rs", "src/a/b.rs"));
        assert!(!glob_matches("docs/", "docsx/y.md"));
        assert!(!glob_matches("a?c.txt", "ac.txt"));
    }

    #[test]
    fn the_whole_tree_contains_everything_and_inputs_do_not() {
        assert!(Scope::WholeTree("x".into()).contains("anything"));
        let s = Scope::Inputs(vec!["src/**".into()]);
        assert!(s.contains("src/core.txt"));
        assert!(!s.contains("README.txt"));
    }

    #[test]
    fn own_records_are_not_the_candidate() {
        let w = "docs/warrants";
        assert!(own_record(
            "docs/warrants/X-WAR-0001/journal.jsonl",
            "docs/warrants/X-WAR-0001",
            w,
            "X-WAR-0001"
        ));
        assert!(own_record(
            "docs/authority/responses/X-WAR-0001.resolution.response.toml",
            "docs/warrants/X-WAR-0001",
            w,
            "X-WAR-0001"
        ));
        assert!(!own_record(
            "docs/warrants/X-WAR-0002/journal.jsonl",
            "docs/warrants/X-WAR-0001",
            w,
            "X-WAR-0001"
        ));
        assert!(!own_record(
            "docs/authority/responses/X-WAR-00010.toml",
            "docs/warrants/X-WAR-0001",
            w,
            "X-WAR-0001"
        ));
    }

    #[test]
    fn a_time_that_does_not_parse_is_never_after() {
        assert!(after("2026-09-24T10:00:01Z", "2026-09-24T10:00:00Z"));
        assert!(!after("2026-09-24T10:00:00Z", "2026-09-24T10:00:00Z"));
        assert!(!after("soon", "2026-09-24T10:00:00Z"));
    }
}
