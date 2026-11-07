// SPDX-License-Identifier: Apache-2.0
//! One id space and the rows of every Warrant outside the ticket store
//! (OW-WAR-0148 M10; docs/TYPES.md, "One Warrant, three encodings").
//! [`crate::work`] is the one list; it reads these rows beside the tickets.
//!
//! A Warrant is one record. Its minimum is a title; everything else is
//! optional, chosen by its type. It is written down in one of four ways,
//! and every one of them reads forever, byte for byte (decision 14):
//!
//! - **light** — `docs/tickets/<t-id>/`: a manifest, an intent and a
//!   checklist. The working form (`profiles/ticket.toml`): workable the
//!   moment it exists, never signed. `war create` writes this one.
//! - **directory** — `docs/warrants/<alias>/`: a manifest and the atoms its
//!   type requires (delivery, decision, ...), compiled, and signed where
//!   its type says so. `war plan new` writes this one.
//! - **single file** — `docs/warrants/<alias>.md` or
//!   `docs/warrants/<alias>/warrant.md` (FORMAT-1.0 §4.1): read here, never
//!   written here.
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
//! `war view warrants` answers in milliseconds on a corpus of any size. Its
//! state is the phase the journal records (`provenance: recorded`);
//! `war status <alias>` is the full, computed answer.

use camino::Utf8Path;
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

/// [`Row::encoding`] of a Warrant kept as a directory.
pub const DIRECTORY: &str = "directory";
/// [`Row::encoding`] of a Warrant kept as one Format 1.0 document.
pub const SINGLE_FILE: &str = "single_file";
/// [`Row::encoding`] of a Warrant another tool owns.
pub const READ_IN_PLACE: &str = "read_in_place";

/// One Warrant that is not in the light encoding, as `war view warrants` lists
/// it beside the light ones (`result.warrants`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Row {
    /// Its alias, or `openspec:<change>` / `speckit:<feature>`.
    pub id: String,
    pub title: String,
    /// Its type: the profile a directory Warrant's manifest names
    /// (`delivery`, `decision`, ...), or `openspec` / `speckit`.
    pub profile: String,
    /// [`DIRECTORY`], [`SINGLE_FILE`] or [`READ_IN_PLACE`].
    pub encoding: &'static str,
    /// Derived from its fields ([`crate::work::level`]), never stored.
    pub level: crate::work::Level,
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
    /// Repository-relative directory that holds it; a single-file Warrant's
    /// document.
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
    let dirs = repo.warrant_dirs()?;
    // Each row reads its own two files, so the reads fan out across threads
    // on the disk; the reader that pays is `war view prime`, at session start.
    let read = crate::vfs::map_ordered(&dirs, 8, |dir| directory_row(repo, dir));
    let mut rows = Vec::new();
    let mut faults = Vec::new();
    for (row, found) in read {
        rows.extend(row);
        faults.extend(found);
    }
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    Ok((rows, faults))
}

fn directory_row(repo: &Repository, dir: &Utf8Path) -> (Option<Row>, Vec<Diagnostic>) {
    let mut faults = Vec::new();
    let manifest = dir.join("manifest.toml");
    // A Format 1.0 document's state directory: its row is the document's
    // ([`document_rows`]), not a manifest's.
    if !crate::vfs::is_file(&manifest) && crate::subject::of_state(dir).is_some() {
        return (None, faults);
    }
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
            return (None, faults);
        }
    };
    let text = |key: &str| value.get(key).and_then(|v| v.as_str()).map(str::to_owned);
    let id = text("local_alias").unwrap_or_else(|| dir.file_name().unwrap_or_default().to_owned());
    let (state, provenance) = match recorded_phase(dir) {
        Ok(Some(phase)) => (phase, "recorded"),
        Ok(None) => ("unknown".to_owned(), "unknown"),
        Err(e) => {
            faults.push(Diagnostic::warn(
                "warrants.journal-unreadable",
                repo.relative(&dir.join(crate::journal_cmd::FILE)),
                format!("{id}: its state reads unknown: {e}"),
            ));
            ("unknown".to_owned(), "unknown")
        }
    };
    let profile = text("profile").unwrap_or_else(|| "delivery".to_owned());
    let row = Row {
        title: text("title").unwrap_or_default(),
        level: crate::work::directory_level(repo, &profile),
        profile,
        encoding: DIRECTORY,
        state,
        provenance,
        done: None,
        total: None,
        dir: repo.relative(dir),
        id,
    };
    (Some(row), faults)
}

/// The phase the journal in `dir` records, `None` when it records nothing.
fn recorded_phase(dir: &Utf8Path) -> Result<Option<String>, RepoError> {
    let mut journal = crate::journal_cmd::load(dir)?;
    journal
        .events
        .retain(|e| e.event_type != crate::states::ENTERED);
    Ok(crate::journal_cmd::recorded_state(&journal, None).map(|s| s.phase.as_str().to_owned()))
}

/// Every single-file Warrant (FORMAT-1.0 §4.1), in alias order. A document
/// nothing can be read from is skipped with a warning, as a manifest is.
/// With no journal yet it is a `draft`: nothing has happened to it.
#[must_use]
pub fn document_rows(repo: &Repository) -> (Vec<Row>, Vec<Diagnostic>) {
    let docs = crate::format_doc::warrant_documents(repo);
    let mut rows = Vec::new();
    let mut faults: Vec<Diagnostic> = docs
        .faults
        .iter()
        .map(|f| {
            Diagnostic::warn(
                "warrants.document-unreadable",
                f.file.clone(),
                format!("skipped: {}", f.message),
            )
        })
        .collect();
    for loaded in &docs.docs {
        let Some(id) = loaded.doc.alias.clone().filter(|_| loaded.admitted()) else {
            // The same as an unworkable ticket document: not worked, and
            // said, with what is wrong with it.
            let why = loaded
                .doc
                .findings
                .iter()
                .find(|f| f.outcome != openwarrant_core::format::Outcome::Warning)
                .map_or("it could not be read as a Warrant", |f| f.message.as_str());
            faults.push(Diagnostic::warn(
                "warrants.document-unreadable",
                loaded.source.clone(),
                format!("skipped: {why}"),
            ));
            continue;
        };
        let dir = repo.warrants_dir().join(&id);
        let (state, provenance) = match recorded_phase(&dir) {
            Ok(Some(phase)) => (phase, "recorded"),
            Ok(None) => ("draft".to_owned(), "computed"),
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
            title: loaded.doc.title.clone().unwrap_or_default(),
            level: crate::work::document_level(&id, loaded, &docs.profiles),
            profile: loaded.doc.profile.clone(),
            encoding: SINGLE_FILE,
            state,
            provenance,
            done: None,
            total: None,
            dir: loaded.source.clone(),
            id,
        });
    }
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    (rows, faults)
}

/// Every Warrant not in the light encoding: the directory ones, the
/// single-file ones, then the ones read in place, with a warning per one that
/// could not be read.
pub fn other_rows(repo: &Repository) -> Result<(Vec<Row>, Vec<Diagnostic>), RepoError> {
    let (mut rows, mut faults) = directory_rows(repo)?;
    let (documents, found) = document_rows(repo);
    rows.extend(documents);
    faults.extend(found);
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
            encoding: DIRECTORY,
            level: crate::work::Level::Order,
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
