// SPDX-License-Identifier: AGPL-3.0-or-later
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
