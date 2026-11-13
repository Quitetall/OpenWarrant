// SPDX-License-Identifier: Apache-2.0
//! One list of all work, whatever its encoding (t-3c16c W1.1;
//! `docs/design/one-view.md`).
//!
//! An item is one record. It is stored as a directory ticket, a single-file
//! ticket, a directory Warrant, a single-file Warrant, or an OpenSpec or Spec
//! Kit folder read in place; [`Work`] hides which. A reader asks for
//! [`Work::items`], and each [`Item`] says its id, title, state, level and
//! where it lives. A reader that renders a ticket's checklist still has it
//! ([`Work::tickets`]): the list adds no second store.
//!
//! # A level is derived, never stored
//!
//! [`level`] is a pure function of two facts about the item: whether something
//! other than the agent's word decides its completion (a trigger), and whether
//! a person must sign it. No record holds a level, so no record can disagree
//! with one, and an item changes level by changing the field that makes it
//! one. Each encoding states those facts its own way; [`Fields`] is what they
//! come to. Signed records are read here and never written.
//!
//! # What cannot be read stays unknown
//!
//! A source that cannot be read is said, never listed as empty: a fact that
//! could not be read makes the level [`Level::Unknown`] unless what was read
//! already settles it, and [`Work::read`] always returns, naming each source
//! it could not read. Which of those a reader treats as fatal is the reader's
//! call: `prime` and the ticket list refuse without their tickets, as they
//! always did, and the terse status prints `?`.

use serde::Serialize;

use crate::repo::{RepoError, Repository};
use crate::ticket::{Claims, Store, Ticket, TicketState, ladder, state_of};
use crate::warrants::Row;

/// How an item ends. The default is [`Level::Ticket`]: nothing extra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Done when the agent says so.
    Ticket,
    /// Done when a trigger passed.
    Warrant,
    /// Done when a person signed, after any trigger passed.
    Order,
    /// A fact the level depends on could not be read. Never a guess.
    Unknown,
}

impl Level {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ticket => "Ticket",
            Self::Warrant => "Warrant",
            Self::Order => "Order",
            Self::Unknown => "UNKNOWN",
        }
    }
}

/// The facts a level is derived from; `None` is one that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fields {
    /// Something other than the agent's word decides completion.
    pub trigger: Option<bool>,
    /// A person must sign.
    pub signature: Option<bool>,
}

impl Fields {
    /// Nothing could be read.
    pub const UNREAD: Self = Self {
        trigger: None,
        signature: None,
    };
    /// Nothing extra: a title and whatever the agent says.
    pub const NONE: Self = Self {
        trigger: Some(false),
        signature: Some(false),
    };
}

/// The level `fields` make. Only a required signature makes an Order, so a
/// trigger alone never reads as one, however it is written; and a signature
/// that could not be read leaves it unknown, because it could still be one.
#[must_use]
pub const fn level(fields: Fields) -> Level {
    match (fields.signature, fields.trigger) {
        (Some(true), _) => Level::Order,
        (Some(false), Some(true)) => Level::Warrant,
        (Some(false), Some(false)) => Level::Ticket,
        (None, _) | (Some(false), None) => Level::Unknown,
    }
}

/// The level of a Warrant kept as a directory: what its profile selects. An
/// unknown profile selects nothing that can be read.
#[must_use]
pub fn directory_level(repo: &Repository, profile: &str) -> Level {
    use openwarrant_core::Capability as C;
    let Ok(profile) = repo.profiles.resolve(profile) else {
        return Level::Unknown;
    };
    let caps = repo.profiles.capabilities(&profile);
    level(Fields {
        trigger: Some(caps.has(C::Evidence) || caps.has(C::Verification)),
        signature: Some(
            caps.has(C::Authorization) || caps.has(C::Resolution) || caps.has(C::Acceptance),
        ),
    })
}

/// The level of a single-file Warrant: what its profile selects, what the
/// document declares, and the highest `min` of its milestones.
#[must_use]
pub fn document_level(
    alias: &str,
    loaded: &crate::format_doc::Loaded,
    profiles: &openwarrant_core::format_profile::Profiles,
) -> Level {
    use openwarrant_core::format_profile::Lookup;
    use openwarrant_core::ticks;
    let Lookup::Found(profile) = profiles.lookup(&loaded.doc.profile) else {
        return Level::Unknown;
    };
    let Ok(text) = std::str::from_utf8(&loaded.bytes) else {
        return Level::Unknown;
    };
    let checks = ladder::document_checks(alias, text);
    let floor = checks
        .milestones
        .iter()
        .filter_map(|m| m.min)
        .fold(ticks::Level::Claimed, ticks::Level::max);
    let declared = ladder::fields_from(&checks, floor);
    let selects = |any: &[&str]| any.iter().any(|c| profile.has(c));
    level(Fields {
        trigger: declared
            .trigger
            .map(|t| t || selects(&["evidence", "verification"])),
        signature: declared
            .signature
            .map(|s| s || selects(&["authorization", "resolution", "acceptance"])),
    })
}

/// The repository-relative document of the single-file Warrant `id`, whether
/// or not it reads well enough to be listed: a check of one that does not is
/// what says why. `None` for a directory Warrant and for any other id.
#[must_use]
pub fn document_of(repo: &Repository, id: &str) -> Option<String> {
    if id.is_empty() || id.contains(['/', '\\']) || repo.warrant_dir(id).is_ok() {
        return None;
    }
    let dir = repo.warrants_dir();
    [
        dir.join(format!("{id}.md")),
        dir.join(id).join("warrant.md"),
    ]
    .into_iter()
    .find(|p| crate::vfs::is_file(p))
    .map(|p| repo.relative(&p))
}

/// Where and how an item is stored. A reader needs this only to say so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Encoding {
    /// A directory ticket: a manifest and its atoms.
    Light,
    /// One Format 1.0 document, a ticket's or a Warrant's.
    SingleFile,
    /// A directory Warrant.
    Directory,
    /// A folder another tool owns, read where it is.
    ReadInPlace,
}

/// One item of work, the same whatever stores it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Item {
    pub id: String,
    pub title: String,
    /// `open`, `in_progress` or `done`; or the phase a Warrant's journal
    /// records (`draft`, `authorized`, `resolved`, `unknown`).
    pub state: String,
    pub level: Level,
    pub encoding: Encoding,
    /// Repository-relative: its document, or the directory that holds it.
    pub place: String,
}

impl Item {
    /// Whether it is finished. A state that cannot be read is not.
    #[must_use]
    pub fn is_done(&self) -> bool {
        matches!(self.state.as_str(), "done" | "resolved")
    }
}

/// How many open items stand at each level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct LevelCounts {
    pub tickets: usize,
    pub warrants: usize,
    pub orders: usize,
    pub unknown: usize,
}

impl LevelCounts {
    /// The open items among `items`.
    pub fn open<'a>(items: impl IntoIterator<Item = &'a Item>) -> Self {
        let mut counts = Self::default();
        for item in items.into_iter().filter(|i| !i.is_done()) {
            match item.level {
                Level::Ticket => counts.tickets += 1,
                Level::Warrant => counts.warrants += 1,
                Level::Order => counts.orders += 1,
                Level::Unknown => counts.unknown += 1,
            }
        }
        counts
    }

    /// `12 Tickets, 1 Warrant`: what is there, or `none`.
    #[must_use]
    pub fn describe(&self) -> String {
        let plural = |n: usize| if n == 1 { "" } else { "s" };
        let mut parts = Vec::new();
        for (n, word) in [
            (self.tickets, "Ticket"),
            (self.warrants, "Warrant"),
            (self.orders, "Order"),
        ] {
            if n > 0 {
                parts.push(format!("{n} {word}{}", plural(n)));
            }
        }
        if self.unknown > 0 {
            parts.push(format!("{} of unknown level", self.unknown));
        }
        if parts.is_empty() {
            "none".to_owned()
        } else {
            parts.join(", ")
        }
    }
}

/// What a read draws on, each of which can fail alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Tickets,
    /// Without these a ticket's `in_progress` may read `open`; whether it is
    /// done, or listed, does not change.
    Claims,
    /// Every Warrant outside the ticket store.
    Others,
}

/// Every item of a repository, as one read. See the module docs.
#[derive(Debug, Default)]
pub struct Work {
    items: Vec<Item>,
    tickets: Vec<Ticket>,
    claims: Claims,
    rows: Vec<Row>,
    ticket_faults: Vec<crate::diagnostic::Diagnostic>,
    other_faults: Vec<crate::diagnostic::Diagnostic>,
    types: Vec<String>,
    tickets_unread: Option<String>,
    claims_unread: Option<String>,
    others_unread: Option<String>,
}

/// The repository-relative documents the corpus compiles.
/// Where the documents a formal act takes are, among the roots
/// [`crate::warrants::document_rows`] reads (`docs/work/` and the Warrants'
/// directory): the rows [`Work::elsewhere`] drops are those rows, so a
/// subject in the ticket store, which holds none of them, is never asked
/// for, and its ten thousand documents are not opened to find out.
fn subject_places(repo: &Repository) -> std::collections::BTreeSet<String> {
    crate::subject::all_in(&[crate::subject::work_root(repo), repo.warrants_dir()])
        .iter()
        .map(|f| repo.relative(&f.file))
        .collect()
}

impl Work {
    /// Every item of `repo`, in every encoding. Never fails: what could not
    /// be read is named by [`Self::unread`].
    #[must_use]
    pub fn read(repo: &Repository) -> Self {
        match Store::open(repo, None) {
            Ok(store) => Self::read_with(repo, &store),
            Err(e) => {
                let mut work = Self {
                    tickets_unread: Some(e.to_string()),
                    ..Self::default()
                };
                work.read_others(repo);
                work
            }
        }
    }

    /// [`Self::read`] through a store already open, whose actor and policy a
    /// command chose.
    #[must_use]
    pub fn read_with(repo: &Repository, store: &Store) -> Self {
        let mut work = Self::tickets_of(store);
        work.read_others(repo);
        work
    }

    /// The ticket store's items alone: what a reader that only ever shows
    /// tickets pays for. `prime <id>` and the board read this.
    #[must_use]
    pub fn tickets_of(store: &Store) -> Self {
        let (loaded, claims) = crate::ticket::with_claims(store, || store.load_all());
        let mut work = Self::default();
        match claims {
            Ok(claims) => work.claims = claims,
            Err(e) => work.claims_unread = Some(e.to_string()),
        }
        match loaded {
            Ok((tickets, faults)) => {
                work.ticket_faults = faults;
                work.items = tickets
                    .iter()
                    .map(|t| ticket_item(store, t, &work.claims))
                    .collect();
                work.tickets = tickets;
            }
            Err(e) => work.tickets_unread = Some(e.to_string()),
        }
        work
    }

    /// Single-file Warrants and work read in place alone: what no reader of
    /// the ticket store or the directory corpus already has. `next` reads
    /// this, and pays for neither.
    #[must_use]
    pub fn elsewhere(repo: &Repository) -> Self {
        let mut work = Self::default();
        let (mut rows, mut faults) = crate::warrants::document_rows(repo);
        let subjects = subject_places(repo);
        rows.retain(|r| !subjects.contains(&r.dir));
        let adapted = crate::interop::adapters::load(repo);
        rows.extend(adapted.rows());
        faults.extend(adapted.diagnostics());
        work.items = rows.iter().map(row_item).collect();
        work.rows = rows;
        work.other_faults = faults;
        work
    }

    /// The item with id `id` among [`Self::items`].
    #[must_use]
    pub fn find(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    fn read_others(&mut self, repo: &Repository) {
        self.types = crate::warrants::type_words(repo);
        match crate::warrants::other_rows(repo) {
            Ok((rows, faults)) => {
                self.other_faults = faults;
                self.items.extend(rows.iter().map(row_item));
                self.rows = rows;
            }
            Err(e) => self.others_unread = Some(e.to_string()),
        }
    }

    /// Every item: the ticket store's in its order, then the rest in id order
    /// within each encoding.
    #[must_use]
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// The Warrants and Orders kept as one document, wherever it is: what the
    /// corpus projection, which reads Warrant directories, leaves out. An item
    /// whose level could not be read may be one, so it is listed.
    #[must_use]
    pub fn documents(&self) -> Vec<Item> {
        self.items
            .iter()
            .filter(|i| i.encoding == Encoding::SingleFile && i.level != Level::Ticket)
            .cloned()
            .collect()
    }

    /// The items outside the ticket store.
    pub fn others(&self) -> impl Iterator<Item = &Item> {
        self.items.iter().skip(self.tickets.len())
    }

    /// What the ticket store could not supply, tickets before claims, as
    /// those two reads were always taken.
    pub fn ticket_store(&self) -> Result<(), RepoError> {
        match self.tickets_unread.as_ref().or(self.claims_unread.as_ref()) {
            Some(why) => Err(RepoError::Message(why.clone())),
            None => Ok(()),
        }
    }

    /// Why `source` could not be read, when it could not.
    #[must_use]
    pub fn unread(&self, source: Source) -> Option<&str> {
        match source {
            Source::Tickets => &self.tickets_unread,
            Source::Claims => &self.claims_unread,
            Source::Others => &self.others_unread,
        }
        .as_deref()
    }

    /// Whether every item is here. Claims change a state, never whether an
    /// item is listed.
    #[must_use]
    pub const fn complete(&self) -> bool {
        self.tickets_unread.is_none() && self.others_unread.is_none()
    }

    /// The tickets themselves, for what only a ticket has.
    #[must_use]
    pub fn tickets(&self) -> &[Ticket] {
        &self.tickets
    }

    /// The claims read with them.
    #[must_use]
    pub const fn claims(&self) -> &Claims {
        &self.claims
    }

    /// The row of each item outside the ticket store.
    #[must_use]
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Why a ticket that could not be read is not listed.
    #[must_use]
    pub fn ticket_faults(&self) -> &[crate::diagnostic::Diagnostic] {
        &self.ticket_faults
    }

    /// Warnings from reading everything else.
    #[must_use]
    pub fn other_faults(&self) -> &[crate::diagnostic::Diagnostic] {
        &self.other_faults
    }

    /// The words `--type` accepts beyond a ticket's own.
    #[must_use]
    pub fn types(&self) -> &[String] {
        &self.types
    }
}

/// The block `war status --verbose` and `war plan frontier` print for
/// [`Work::documents`]; nothing when there are none.
#[must_use]
pub fn render_documents(items: &[Item]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    if !items.is_empty() {
        out.push_str("\nSingle-file Warrants and Orders\n");
    }
    for i in items {
        let _ = writeln!(
            out,
            "  {:<12} {:<8} {:<12} {}",
            i.id,
            i.level.as_str(),
            i.state,
            i.title
        );
    }
    out
}

fn ticket_item(store: &Store, t: &Ticket, claims: &Claims) -> Item {
    Item {
        id: t.id().to_owned(),
        title: t.manifest.title.clone(),
        state: match state_of(t, claims) {
            TicketState::Open => "open",
            TicketState::InProgress => "in_progress",
            TicketState::Done => "done",
        }
        .to_owned(),
        level: ticket_level(store, t),
        encoding: if t.format {
            Encoding::SingleFile
        } else {
            Encoding::Light
        },
        place: store.rel(if t.format { &t.intent_path } else { &t.dir }),
    }
}

fn row_item(row: &Row) -> Item {
    Item {
        id: row.id.clone(),
        title: row.title.clone(),
        state: row.state.clone(),
        level: row.level,
        encoding: match row.encoding {
            crate::warrants::DIRECTORY => Encoding::Directory,
            crate::warrants::READ_IN_PLACE => Encoding::ReadInPlace,
            _ => Encoding::SingleFile,
        },
        place: row.dir.clone(),
    }
}

/// A ticket's level, from what it declares and the minimum its repository
/// sets for it.
#[must_use]
pub fn ticket_level(store: &Store, t: &Ticket) -> Level {
    level(ladder::level_fields(store, t))
}

/// Validate records through the same routes for the CLI and MCP. This checks
/// record integrity, while executable completion checks run through `done`.
pub fn check_records(
    shared: &crate::corpus::Corpus,
    alias: Option<&str>,
    generated: bool,
) -> Result<crate::diagnostic::Report, RepoError> {
    use crate::{
        check, diagnostic, format_doc, gate_cmd, init, interfaces, interop, ledger, preset, score,
        ticket, warrants,
    };
    let repository = shared.repo();
    // A ticket is checked for its structure only (OW-WAR-0147): never
    // for a signature, evidence or verification it does not need.
    if let Some(a) = alias.filter(|a| ticket::is_ticket_ref(a)) {
        let mut report = diagnostic::Report::default();
        match ticket::Store::open(repository, None) {
            Ok(store) => {
                // A Format 1.0 ticket is checked as the document it
                // is. One the loop refuses is no ticket to go on to.
                if let Some(document) = store.document_of(a)? {
                    format_doc::check_one(repository, &document, &mut report);
                }
                if report.is_ready() {
                    ticket::check(&store, Some(a), &mut report)?;
                }
            }
            Err(e) => report.push(diagnostic::Diagnostic::error(
                "tickets.config",
                init::CONFIG_FILE.to_owned(),
                e.to_string(),
            )),
        }
        return Ok(report);
    }
    // M10: a Warrant read in place is checked for what its folder
    // says, read where it is; nothing is written.
    if let Some(a) = alias.filter(|a| warrants::kind_of(a) == warrants::IdKind::ReadInPlace) {
        let mut report = diagnostic::Report::default();
        interop::adapters::check(repository, Some(a), &mut report);
        return Ok(report);
    }
    // W1.1: a single-file Warrant is checked as the document it is,
    // with or without a directory corpus around it.
    if let Some(source) = alias.and_then(|a| document_of(repository, a)) {
        let mut report = diagnostic::Report::default();
        format_doc::check_one(repository, &source, &mut report);
        return Ok(report);
    }
    // One-shot and read-only: the tree does not move under it (t-eca6).
    gate_cmd::source::remember_tree_reads();
    let mut report = check::run_with(shared, alias, generated)?;
    if alias.is_none() {
        match ticket::Store::open(repository, None) {
            Ok(store) => ticket::check(&store, None, &mut report)?,
            Err(e) => report.push(diagnostic::Diagnostic::error(
                "tickets.config",
                init::CONFIG_FILE.to_owned(),
                e.to_string(),
            )),
        }
        interop::adapters::check(repository, None, &mut report);
        // W6: open clauses counted, late ones flagged, [repos] read.
        interfaces::check(repository, chrono::Utc::now().date_naive(), &mut report);
        // OW-WAR-0148 M14: a [preset], [roles] or [notify] table that
        // does not read is refused by key; an absent one says nothing.
        if let Err(e) = preset::Policy::read(&repository.root) {
            report.push(diagnostic::Diagnostic::error(
                preset::CONFIG_RULE,
                init::CONFIG_FILE.to_owned(),
                e,
            ));
        }
        // OW-WAR-0148 M17: the ledger's atoms and table, the JSONL
        // never committed, and the [score] table.
        ledger::check(repository, &mut report);
        if let Err(e) = score::Config::read(&repository.root) {
            report.push(diagnostic::Diagnostic::error(
                "score.config",
                init::CONFIG_FILE.to_owned(),
                e,
            ));
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn fields(trigger: Option<bool>, signature: Option<bool>) -> Fields {
        Fields { trigger, signature }
    }

    #[test]
    fn nothing_extra_is_a_ticket() {
        assert_eq!(level(Fields::NONE), Level::Ticket);
    }

    #[test]
    fn a_trigger_makes_a_warrant_and_nothing_less_does() {
        assert_eq!(level(fields(Some(true), Some(false))), Level::Warrant);
        assert_ne!(level(fields(Some(false), Some(false))), Level::Warrant);
    }

    #[test]
    fn a_required_signature_makes_an_order_with_or_without_a_trigger() {
        assert_eq!(level(fields(Some(true), Some(true))), Level::Order);
        assert_eq!(level(fields(Some(false), Some(true))), Level::Order);
    }

    #[test]
    fn no_trigger_ever_reads_as_an_order_without_a_signature() {
        for trigger in [Some(true), Some(false), None] {
            for signature in [Some(false), None] {
                assert_ne!(level(fields(trigger, signature)), Level::Order);
            }
        }
    }

    #[test]
    fn what_could_not_be_read_is_unknown_until_something_settles_it() {
        assert_eq!(level(Fields::UNREAD), Level::Unknown);
        assert_eq!(level(fields(Some(true), None)), Level::Unknown);
        assert_eq!(level(fields(None, Some(false))), Level::Unknown);
        // A signature already read settles it: nothing unread could lower it.
        assert_eq!(level(fields(None, Some(true))), Level::Order);
    }

    #[test]
    fn counts_read_open_items_and_say_only_what_is_there() {
        let item = |state: &str, level: Level| Item {
            id: "x".into(),
            title: String::new(),
            state: state.into(),
            level,
            encoding: Encoding::Directory,
            place: String::new(),
        };
        let items = [
            item("open", Level::Ticket),
            item("in_progress", Level::Warrant),
            item("authorized", Level::Order),
            item("draft", Level::Order),
            item("resolved", Level::Order),
            item("done", Level::Ticket),
        ];
        let counts = LevelCounts::open(&items);
        assert_eq!(counts.describe(), "1 Ticket, 1 Warrant, 2 Orders");
        assert_eq!(LevelCounts::default().describe(), "none");
        let unknown = [item("unknown", Level::Unknown)];
        assert_eq!(LevelCounts::open(&unknown).describe(), "1 of unknown level");
    }
}
