// SPDX-License-Identifier: Apache-2.0
//! One id space and one view over every encoding of a Warrant
//! (OW-WAR-0148 M10; docs/TYPES.md, "One Warrant, three encodings").
//!
//! A Warrant is one record. Its minimum is a title; everything else is
//! optional, chosen by its type. It is written down in one of three ways,
//! and every one of them reads forever, byte for byte (decision 14):
//!
//! - **light** — `docs/tickets/<t-id>/`: a manifest, an intent and a
//!   checklist. The working form (`profiles/ticket.toml`): workable the
//!   moment it exists, never signed. `war create` writes this one.
//! - **directory** — `docs/warrants/<alias>/`: a manifest and the atoms its
//!   type requires (delivery, decision, ...), compiled, and signed where
//!   its type says so. `war new` writes this one.
//! - **read in place** — an OpenSpec change or a Spec Kit feature named by
//!   an `[[adapters]]` entry in `openwarrant.toml`, read where it is and
//!   never written ([`crate::interop::adapters`]).
//!
//! One id names each, whatever its encoding: `t-3f2a` (and its items
//! `t-3f2a/i-9c01`), `OW-WAR-0148`, `openspec:add-2fa`,
//! `speckit:001-photo-albums`. [`kind_of`] tells them apart by shape alone,
//! so a command routes an id without reading anything.
//!
//! The list here is built for speed: a directory Warrant's row reads its
//! manifest and its journal, never the compiler or the status builder, so
//! `war warrants` answers in milliseconds on a corpus of any size. Its
//! state is the phase the journal records (`provenance: recorded`);
//! `war status <alias>` is the full, computed answer.

use serde::Serialize;

use crate::diagnostic::Diagnostic;
use crate::repo::{RepoError, Repository};

/// What an id names, by its shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdKind {
    /// `t-…`, `i-…`, `t-…/i-…`: a Warrant in the light encoding, or one of
    /// its items.
    Light,
    /// `openspec:…`, `speckit:…`: a Warrant read in place, or one of its
    /// records.
    ReadInPlace,
    /// Anything else: a Warrant alias (`NS-WAR-NNNN`), which a directory
    /// holds.
    Directory,
}

/// The formats a Warrant can be read in place from: an id of one begins
/// `<kind>:`.
pub const READ_IN_PLACE_KINDS: &[&str] = &["openspec", "speckit"];

/// Route an id by its shape. Nothing is read.
#[must_use]
pub fn kind_of(id: &str) -> IdKind {
    if crate::ticket::is_ticket_ref(id) {
        IdKind::Light
    } else if READ_IN_PLACE_KINDS
        .iter()
        .any(|k| id.strip_prefix(k).is_some_and(|r| r.starts_with(':')))
    {
        IdKind::ReadInPlace
    } else {
        IdKind::Directory
    }
}

/// One Warrant that is not in the light encoding, as `war warrants` lists
/// it beside the light ones (`result.warrants`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Row {
    /// Its alias, or `openspec:<change>` / `speckit:<feature>`.
    pub id: String,
    pub title: String,
    /// Its type: the profile a directory Warrant's manifest names
    /// (`delivery`, `decision`, ...), or `openspec` / `speckit`.
    pub profile: String,
    /// `directory`, or `read_in_place`.
    pub encoding: &'static str,
    /// A directory Warrant: the phase its journal records (`draft`,
    /// `authorized`, `resolved`, ...), or `unknown` when it has no readable
    /// journal. One read in place: `open`, `in_progress` or `done`, from its
    /// tasks.
    pub state: String,
    /// `recorded` (the journal), `computed` (the tasks) or `unknown`.
    pub provenance: &'static str,
    /// Tasks done and in all, for one read in place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
    /// Repository-relative directory that holds it.
    pub dir: String,
}

impl Row {
    /// The fixed state a `--state` filter reads this row as: a recorded
    /// phase maps onto the light encoding's three (draft → open, a phase
    /// between authorized and verifying → in_progress, resolved → done).
    #[must_use]
    pub fn fixed_state(&self) -> Option<&'static str> {
        match self.state.as_str() {
            "open" | "draft" | "proposed" => Some("open"),
            "in_progress" | "authorized" | "ready" | "executing" | "verifying" => {
                Some("in_progress")
            }
            "done" | "resolved" => Some("done"),
            _ => None,
        }
    }
}

/// The phase words a directory Warrant's journal can record, accepted by
/// `--state` beside the fixed three.
pub const PHASES: &[&str] = &[
    "draft",
    "proposed",
    "authorized",
    "ready",
    "executing",
    "verifying",
    "resolved",
];

/// Every directory Warrant, as a row read from its manifest and journal
/// only, in alias order; a manifest that does not parse is skipped with a
/// warning, never listed under a guessed title.
pub fn directory_rows(repo: &Repository) -> Result<(Vec<Row>, Vec<Diagnostic>), RepoError> {
    let mut rows = Vec::new();
    let mut faults = Vec::new();
    for dir in repo.warrant_dirs()? {
        let manifest = dir.join("manifest.toml");
        let rel = repo.relative(&manifest);
        let parsed = crate::vfs::read_to_string(&manifest)
            .map_err(|e| e.to_string())
            .and_then(|t| toml::from_str::<toml::Value>(&t).map_err(|e| e.to_string()));
        let value = match parsed {
            Ok(v) => v,
            Err(e) => {
                faults.push(Diagnostic::warn(
                    "warrants.manifest-unreadable",
                    rel,
                    format!("skipped: the manifest could not be read: {e}"),
                ));
                continue;
            }
        };
        let text = |key: &str| value.get(key).and_then(|v| v.as_str()).map(str::to_owned);
        let id =
            text("local_alias").unwrap_or_else(|| dir.file_name().unwrap_or_default().to_owned());
        let (state, provenance) = match crate::journal_cmd::load(&dir) {
            Ok(mut journal) => {
                journal
                    .events
                    .retain(|e| e.event_type != crate::states::ENTERED);
                match crate::journal_cmd::recorded_state(&journal, None) {
                    Some(s) => (s.phase.as_str().to_owned(), "recorded"),
                    None => ("unknown".to_owned(), "unknown"),
                }
            }
            Err(e) => {
                faults.push(Diagnostic::warn(
                    "warrants.journal-unreadable",
                    repo.relative(&dir.join(crate::journal_cmd::FILE)),
                    format!("{id}: its state reads unknown: {e}"),
                ));
                ("unknown".to_owned(), "unknown")
            }
        };
        rows.push(Row {
            title: text("title").unwrap_or_default(),
            profile: text("profile").unwrap_or_else(|| "delivery".to_owned()),
            encoding: "directory",
            state,
            provenance,
            done: None,
            total: None,
            dir: repo.relative(&dir),
            id,
        });
    }
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    Ok((rows, faults))
}

/// Every Warrant not in the light encoding: the directory ones, then the
/// ones read in place, with a warning per one that could not be read.
pub fn other_rows(repo: &Repository) -> Result<(Vec<Row>, Vec<Diagnostic>), RepoError> {
    let (mut rows, mut faults) = directory_rows(repo)?;
    let adapted = crate::interop::adapters::load(repo);
    rows.extend(adapted.rows());
    faults.extend(adapted.diagnostics());
    Ok((rows, faults))
}

/// The words `--type` accepts beyond the light encoding's own types: every
/// profile the program declares, and the two read-in-place kinds.
#[must_use]
pub fn type_words(repo: &Repository) -> Vec<String> {
    let mut out: Vec<String> = repo
        .profiles
        .names()
        .into_iter()
        .map(str::to_owned)
        .collect();
    out.extend(READ_IN_PLACE_KINDS.iter().map(|k| (*k).to_owned()));
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_routes_by_its_shape_alone() {
        assert_eq!(kind_of("t-3f2a"), IdKind::Light);
        assert_eq!(kind_of("t-3f2a/i-9c01"), IdKind::Light);
        assert_eq!(kind_of("i-9c01"), IdKind::Light);
        assert_eq!(kind_of("openspec:add-2fa"), IdKind::ReadInPlace);
        assert_eq!(
            kind_of("speckit:001-photo-albums/T001"),
            IdKind::ReadInPlace
        );
        assert_eq!(kind_of("OW-WAR-0148"), IdKind::Directory);
        assert_eq!(kind_of("REQ-pr1"), IdKind::Directory);
    }

    #[test]
    fn a_recorded_phase_reads_as_one_of_the_three() {
        let row = |state: &str| Row {
            id: "X-WAR-0001".into(),
            title: String::new(),
            profile: "delivery".into(),
            encoding: "directory",
            state: state.into(),
            provenance: "recorded",
            done: None,
            total: None,
            dir: String::new(),
        };
        assert_eq!(row("draft").fixed_state(), Some("open"));
        assert_eq!(row("authorized").fixed_state(), Some("in_progress"));
        assert_eq!(row("resolved").fixed_state(), Some("done"));
        assert_eq!(row("unknown").fixed_state(), None);
    }
}
