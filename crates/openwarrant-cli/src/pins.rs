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
}

#[derive(Debug, Clone, Serialize)]
pub struct Pins {
    pub schema: &'static str,
    pub pins: Vec<Pin>,
}

/// Every pin in the corpus, sorted by path then Warrant.
pub fn list(repo: &Repository, resolved_only: bool) -> Result<Pins, RepoError> {
    let mut pins = Vec::new();
    for dir in repo.warrant_dirs()? {
        let Some(alias) = dir.file_name().map(str::to_owned) else {
            continue;
        };
        let deliverables = repo.load_deliverables(&dir)?;
        if deliverables.records.is_empty() {
            continue;
        }
        let state = if repo.load_resolution(&dir)?.is_some() {
            PinState::Resolved
        } else if repo.load_authorization(&dir)?.is_some() {
            PinState::Authorized
        } else {
            PinState::Draft
        };
        if resolved_only && state != PinState::Resolved {
            continue;
        }
        let corrections = repo.load_corrections(&dir)?;
        for d in &deliverables.records {
            pins.push(Pin {
                path: d.target_ref.clone(),
                warrant: alias.clone(),
                deliverable_id: d.id.clone(),
                state,
                content_addressed: d.content_addressed,
                content_digest: d.provenance.as_ref().map(|p| p.content_digest.clone()),
                corrections: u32::try_from(corrections.for_deliverable(&d.id).len())
                    .unwrap_or(u32::MAX),
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
            "{:<10} {:<12} {:<6} {}{}\n",
            match pin.state {
                PinState::Resolved => "resolved",
                PinState::Authorized => "authorized",
                PinState::Draft => "draft",
            },
            pin.warrant,
            pin.deliverable_id,
            pin.path,
            if pin.corrections > 0 {
                format!("  (corrected {}×)", pin.corrections)
            } else {
                String::new()
            }
        ));
    }
    if s.is_empty() {
        s.push_str("no deliverables are pinned\n");
    }
    s
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
pub fn refresh(repo: &Repository, alias: Option<&str>) -> Result<crate::diagnostic::Report, RepoError> {
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
            moved.push((d.id.clone(), d.target_ref.clone(), recorded.to_owned(), actual));
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
