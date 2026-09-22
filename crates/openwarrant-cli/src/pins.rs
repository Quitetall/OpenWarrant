// SPDX-License-Identifier: Apache-2.0
//! `war pins` — every file some Warrant's `deliverables.toml` pins, with the
//! Warrant's state, so an agent (or a hook) can refuse the edit BEFORE it
//! drifts a resolved record.
//!
//! `war check` diagnoses drift after the fact. By then the edit is made, the
//! digest has moved, and for a resolved Warrant the only way forward is a
//! correction signed by a human (OW-WAR-0064). A list of what is pinned, asked
//! before editing, turns that into "open a new Warrant instead" — which is what
//! the rule wanted all along. The Claude Code plugin's PreToolUse hook reads
//! this with `--json --resolved-only`.

use serde::Serialize;

use crate::repo::{RepoError, Repository};

pub const SCHEMA: &str = "oh.war/pins/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PinState {
    /// A resolution binds the manifest: only a correction can move this file.
    Resolved,
    /// Authorized, not resolved: regenerate the record after editing.
    Authorized,
    /// Draft: the record is free to change with the file.
    Draft,
}

#[derive(Debug, Clone, Serialize)]
pub struct Pin {
    pub path: String,
    pub warrant: String,
    pub deliverable_id: String,
    pub state: PinState,
    pub content_addressed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_digest: Option<String>,
    /// Corrections on file for this deliverable (OW-WAR-0064).
    pub corrections: u32,
    /// OW-ADR-0021 — a resolved pin that a LATER authorized Warrant's recorded
    /// set governs. Still recorded, still verifiable at its own resolution, and
    /// not drift. The hook lets an edit through when this is true.
    pub historical: bool,
    /// `<alias>/<D-id>` of the Warrant that governs this path now, when it is
    /// not this pin's own Warrant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governed_by: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Pins {
    pub schema: &'static str,
    pub pins: Vec<Pin>,
}

/// Every pin in the corpus, sorted by path then Warrant.
pub fn list(repo: &Repository, resolved_only: bool) -> Result<Pins, RepoError> {
    let mut pins = Vec::new();
    let ownership = crate::ownership::Ownership::index(repo)?;
    for dir in repo.warrant_dirs()? {
        let Some(alias) = dir.file_name().map(str::to_owned) else {
            continue;
        };
        let deliverables = repo.load_deliverables(&dir)?;
        if deliverables.records.is_empty() {
            continue;
        }
        let authorization = repo.load_authorization(&dir)?;
        let state = if repo.load_resolution(&dir)?.is_some() {
            PinState::Resolved
        } else if authorization.is_some() {
            PinState::Authorized
        } else {
            PinState::Draft
        };
        let authorized_at = authorization
            .as_ref()
            .and_then(|a| a.revision.authorization.as_ref())
            .map(|a| a.effective_time.clone());
        if resolved_only && state != PinState::Resolved {
            continue;
        }
        let corrections = repo.load_corrections(&dir)?;
        for d in &deliverables.records {
            let newer = ownership.newer_than(&d.target_ref, &alias, authorized_at.as_deref());
            pins.push(Pin {
                path: d.target_ref.clone(),
                warrant: alias.clone(),
                deliverable_id: d.id.clone(),
                state,
                content_addressed: d.content_addressed,
                content_digest: d.provenance.as_ref().map(|p| p.content_digest.clone()),
                corrections: u32::try_from(corrections.for_deliverable(&d.id).len())
                    .unwrap_or(u32::MAX),
                historical: state == PinState::Resolved && newer.is_some(),
                governed_by: newer.map(|o| format!("{}/{}", o.alias, o.deliverable_id)),
            });
        }
    }
    pins.sort_by(|a, b| a.path.cmp(&b.path).then(a.warrant.cmp(&b.warrant)));
    Ok(Pins {
        schema: SCHEMA,
        pins,
    })
}

/// One line per pin, for humans.
#[must_use]
pub fn render(p: &Pins) -> String {
    let mut s = String::new();
    for pin in &p.pins {
        s.push_str(&format!(
            "{:<10} {:<12} {:<6} {}{}{}\n",
            match (pin.state, pin.historical) {
                (PinState::Resolved, true) => "historical",
                (PinState::Resolved, false) => "resolved",
                (PinState::Authorized, _) => "authorized",
                (PinState::Draft, _) => "draft",
            },
            pin.warrant,
            pin.deliverable_id,
            pin.path,
            if pin.corrections > 0 {
                format!("  (corrected {}×)", pin.corrections)
            } else {
                String::new()
            },
            match &pin.governed_by {
                Some(g) => format!("  → governed by {g}"),
                None => String::new(),
            }
        ));
    }
    if s.is_empty() {
        s.push_str("no deliverables are pinned\n");
    }
    s
}

/// `war pins --history <path>` — every Warrant that ever governed a path,
/// oldest first, and whether each delivery still verifies from history.
///
/// A historical pin is a claim about bytes at a moment. The moment is the
/// resolution's `[locator]` when one was recorded (OW-ADR-0021); before that
/// ADR nothing recorded a commit, and the honest answer for those is UNKNOWN,
/// not a guess from `git log`.
pub fn history(repo: &Repository, path: &str) -> Result<String, RepoError> {
    let ownership = crate::ownership::Ownership::index(repo)?;
    let mut s = String::new();
    let mut rows: Vec<(String, String, String, String)> = Vec::new();
    // Every pin on this path, owner or not: legacy Warrants own nothing but
    // did deliver, and the lineage is incomplete without them.
    for dir in repo.warrant_dirs()? {
        let Some(alias) = dir.file_name().map(str::to_owned) else {
            continue;
        };
        let deliverables = repo.load_deliverables(&dir)?;
        for d in deliverables.records.iter().filter(|d| d.target_ref == path) {
            let resolved = repo.load_resolution(&dir)?;
            let recorded = d
                .provenance
                .as_ref()
                .map(|p| p.content_digest.clone())
                .unwrap_or_else(|| "(no digest)".into());
            // A broken chain has no head; the recorded digest stands in and
            // the row says so through `verifies`, never through a guess.
            let head = crate::correct::head_for(&repo.load_corrections(&dir)?, &d.id, &recorded)
                .1
                .unwrap_or_else(|_| recorded.clone());
            let standing = match (&resolved, ownership.current(path)) {
                (None, _) => "draft or authorized".to_owned(),
                (Some(_), Some(cur)) if cur.alias == alias => "current".to_owned(),
                (Some(_), Some(cur)) => format!("historical → {}", cur.alias),
                (Some(_), None) => "resolved, ungoverned".to_owned(),
            };
            let verifies = match resolved.as_ref().map(|r| r.locator.as_ref()) {
                // An unresolved pin is a note about a file, not a delivery;
                // asking whether it verifies from history is the wrong question.
                None => "not yet resolved".to_owned(),
                Some(None) => "UNKNOWN (no locator; resolved before OW-ADR-0021)".to_owned(),
                Some(Some(loc)) => {
                    let want = head.trim_start_matches("sha256:");
                    let out = std::process::Command::new("git")
                        .args(["show", &format!("{}:{}", loc.commit_sha, path)])
                        .current_dir(&repo.root)
                        .output();
                    match out {
                        Ok(o)
                            if o.status.success()
                                && openwarrant_compiler::sha256_hex(&o.stdout) == want =>
                        {
                            format!("verifies at {}", &loc.commit_sha[..12])
                        }
                        Ok(o) if o.status.success() => {
                            format!("DIFFERS at {}", &loc.commit_sha[..12])
                        }
                        _ => format!("UNKNOWN (commit {} not readable)", &loc.commit_sha[..12]),
                    }
                }
            };
            let when = resolved
                .as_ref()
                .map(|r| r.resolution.effective_at.clone())
                .unwrap_or_default();
            rows.push((
                when,
                format!("{alias}/{}", d.id),
                format!("{standing}  {}", &head[..std::cmp::min(19, head.len())]),
                verifies,
            ));
        }
    }
    rows.sort();
    if rows.is_empty() {
        s.push_str(&format!("no Warrant declares {path}\n"));
        return Ok(s);
    }
    s.push_str(&format!("{path}\n"));
    for (when, who, what, verifies) in rows {
        s.push_str(&format!(
            "  {:<22} {:<20} {:<40} {}\n",
            if when.is_empty() {
                "—".to_owned()
            } else {
                when
            },
            who,
            what,
            verifies
        ));
    }
    Ok(s)
}

/// `war pins --refresh [alias]` — bring an unresolved Warrant's recorded
/// digests back to the bytes on disk.
///
/// The gap this fills: a Warrant is written at the start of the work, its
/// deliverables name files that are still being written, and every commit moves
/// them. Before this existed the only way to re-record a digest was to edit
/// `deliverables.toml` by hand, so `war check` reported drift on every run and
/// the operator learned to ignore it — which is worse than not checking, because
/// the same rule guards resolved records where it matters.
///
/// Refused for a RESOLVED Warrant, whose §56.2 record binds
/// `sha256(deliverables.toml)`: moving a pin there changes what was accepted,
/// and `war correct` is the act that exists for it. An authorization binds the
/// contract, not the bytes, so an authorized-but-unresolved Warrant refreshes
/// like any draft.
pub fn refresh(
    repo: &Repository,
    alias: Option<&str>,
) -> Result<crate::diagnostic::Report, RepoError> {
    use crate::diagnostic::{Diagnostic, Report};
    let mut report = Report::default();
    for dir in repo.warrant_dirs()? {
        let Some(name) = dir.file_name().map(str::to_owned) else {
            continue;
        };
        if alias.is_some_and(|want| want != name) {
            continue;
        }
        let one = repo.load_warrant(&dir)?;
        if crate::check::resolution_binds(repo, &one) {
            if alias.is_some() {
                report.push(Diagnostic::error(
                    "pins.signed",
                    repo.relative(&dir.join("deliverables.toml")),
                    format!(
                        "{name} is resolved: its §56.2 record binds sha256(deliverables.toml). \
                         Moving a pin here would change what was accepted — \
                         `war correct {name} <D-id>` records why the file moved"
                    ),
                ));
            }
            continue;
        }
        let deliverables = repo.load_deliverables(&dir)?;
        let path = dir.join("deliverables.toml");
        let Ok(original) = std::fs::read_to_string(&path) else {
            continue;
        };
        let mut text = original.clone();
        let mut moved = Vec::new();
        for d in deliverables.records.iter().filter(|d| d.content_addressed) {
            let Some(provenance) = &d.provenance else {
                continue;
            };
            let recorded = provenance.content_digest.trim_start_matches("sha256:");
            let Ok(bytes) = std::fs::read(repo.root.join(&d.target_ref)) else {
                report.push(Diagnostic::warn(
                    "pins.unreadable",
                    repo.relative(&path),
                    format!(
                        "{name}: {} names {} and it cannot be read; its pin is left as recorded",
                        d.id, d.target_ref
                    ),
                ));
                continue;
            };
            let actual = openwarrant_compiler::sha256_hex(&bytes);
            if actual == recorded {
                continue;
            }
            // Textual, on the exact recorded digest: the file is hand-written
            // and hand-commented, and a round trip through the parser would
            // rewrite the operator's prose along with the digest.
            if !text.contains(recorded) {
                report.push(Diagnostic::error(
                    "pins.not-found",
                    repo.relative(&path),
                    format!(
                        "{name}: {} records sha256:{recorded} and that string is not in the file; \
                         nothing is rewritten",
                        d.id
                    ),
                ));
                continue;
            }
            text = text.replace(recorded, &actual);
            moved.push((
                d.id.clone(),
                d.target_ref.clone(),
                recorded.to_owned(),
                actual,
            ));
        }
        if moved.is_empty() {
            if alias.is_some() {
                report.push(Diagnostic::pass(
                    "pins.current",
                    format!("{name}: every content-addressed pin matches its bytes"),
                ));
            }
            continue;
        }
        std::fs::write(&path, &text).map_err(|source| RepoError::Io {
            context: format!("could not write {path}"),
            source,
        })?;
        for (id, target, before, after) in moved {
            report.push(Diagnostic::pass(
                "pins.refreshed",
                format!(
                    "{name}: {id} — {target} sha256:{} → sha256:{}",
                    &before[..12],
                    &after[..12]
                ),
            ));
        }
    }
    if report.diagnostics.is_empty() {
        report.push(Diagnostic::pass(
            "pins.current",
            "every draft Warrant's pins match their bytes".to_owned(),
        ));
    }
    report.note(
        "A refreshed pin records what a file IS, not that anyone approved it. Signing is what \
         makes a pin a promise.",
    );
    Ok(report)
}
