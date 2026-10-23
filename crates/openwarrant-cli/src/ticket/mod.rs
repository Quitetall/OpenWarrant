// SPDX-License-Identifier: Apache-2.0
//! The ticket loop (OW-WAR-0147): `war create`, `ready`, `claim`, `done`,
//! `add`, `note`, `prime`, `show`, `tickets`, `release` and `promote`.
//!
//! # What a ticket is
//!
//! The working form of a delivery Warrant (`profiles/ticket.toml`,
//! `form = "working"`): a directory under `docs/tickets/<id>/` holding a
//! `manifest.toml`, an intent atom (the sentence, context, decisions and dated
//! notes) and a checklist atom (a Markdown task list), plus the same
//! append-only `journal.jsonl` a Warrant keeps. Plain Markdown a human reads on
//! GitHub; the files ARE the state, and a hand edit is honoured as written
//! (`openwarrant_core::ticket`).
//!
//! # What it is not
//!
//! Not a Warrant of the contract corpus. A ticket needs no signature, no
//! evidence and no verification to be created, worked or finished, and no
//! command here asks a human for anything. `war check` validates a ticket's
//! structure and nothing else about it. Someone who wants sign-off runs
//! `war plan promote <ticket>`, which drafts a delivery Warrant through `war plan new`;
//! from there the authority layer applies exactly as it always has.
//!
//! # Fast by construction
//!
//! Every command here reads `openwarrant.toml`, the profile definitions, the
//! ticket directories and the claims directory, and nothing else: no Warrant is
//! loaded, nothing is compiled, no ownership index is built and no signature is
//! verified. `conformance/plants.d/45-tickets.sh` measures each command on this
//! repository.

pub mod acts;
pub mod cache;
pub mod checks;
pub mod claim;
pub mod format;
pub mod ladder;
pub mod merge;
pub mod parts;
pub mod remote;
mod render;

use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_core::WarUuid;
use openwarrant_core::role::{ProfileDefinition, ProfileRegistry};
use openwarrant_core::ticket::{
    self, Blocker, CHECKLIST_ROLE, Checklist, DEFAULT_PRIORITY, Item, TICKET_PROFILE,
    TICKET_SCHEMA, TicketAtom, TicketManifest,
};
use openwarrant_core::ticks::{self, Level};
use serde::{Deserialize, Serialize};

use crate::compile::atomic;
use crate::diagnostic::{Diagnostic, Report, Severity};
use crate::repo::{RepoError, Repository};

pub use render::{
    Filter, Others, description, list, notes, prime, show, tickets, tickets_filtered,
};

/// Where tickets live unless `[tickets] dir` says otherwise.
pub const DEFAULT_DIR: &str = "docs/tickets";
/// Where claims lived before they were shared across worktrees (M11), and
/// where they still live outside a git checkout: inside `.openwarrant/state/`,
/// which `.gitignore` already names disposable. In a git checkout new claims
/// go under git's common directory ([`claim::SHARED_SUBDIR`]); a claim found
/// here is still honoured.
pub const DEFAULT_CLAIMS_DIR: &str = ".openwarrant/state/claims";
const DEFAULT_TTL_MINUTES: u64 = 120;
/// M11: how long a claim's lease runs unless its holder renews it.
pub const DEFAULT_LEASE_MINUTES: f64 = 30.0;
const DEFAULT_COMPACT_DAYS: u64 = 7;

/// The ticket profile this build ships, used when a repository has no
/// `profiles/ticket.toml` of its own (every repository `war init` made before
/// tickets existed). A repository's own definition wins.
const BUILTIN_PROFILE: &str = include_str!("../../../../profiles/ticket.toml");

/// Where `war create` writes a ticket's intent atom, relative to its directory.
pub(crate) const INTENT_FILE: &str = "atoms/10-intent.md";

/// The files the ticket loop writes while a ticket is worked (t-5d82), for
/// the evidence tree rule to skip: a ticket is a record about the work, not
/// the source a gate ran over. docs/RESOLVING.md, "Working a ticket does not
/// move the tree".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bookkeeping {
    /// `[tickets] dir`, relative to the root; `None` outside it.
    pub dir: Option<String>,
    /// `[tickets] claims_dir`, relative to the root; `None` outside it.
    pub claims_dir: Option<String>,
    /// The files inside one ticket's directory the loop writes, named one by
    /// one: its manifest, journal, intent and checklist. Anything else a
    /// person keeps in a ticket's directory is not named.
    pub files: Vec<String>,
}

/// The journal event types a ticket records, by their Format 1.0 names
/// (FORMAT-1.0 §11.2). A legacy directory ticket's journal is appended in
/// `v: 1` under the `ticket.*` name of each (`journal_cmd::legacy_type`),
/// so that journal keeps one shape; a reader reads either as these.
pub mod event {
    pub const CREATED: &str = "work.created";
    pub const ITEM_ADDED: &str = "work.item_added";
    pub const CLAIMED: &str = "work.claimed";
    pub const CLAIM_STOLEN: &str = "work.claim_stolen";
    /// M11: a claim whose lease ran out, taken by a plain `war claim`; the
    /// payload names whom it was taken from and when their lease ended.
    pub const CLAIM_RECLAIMED: &str = "work.claim_reclaimed";
    pub const RELEASED: &str = "work.released";
    pub const ITEM_DONE: &str = "work.item_done";
    /// FORMAT-1.0 §11.2: a done item's box opened again (`war admin
    /// reopen`), `{item, target, reason}`.
    pub const ITEM_REOPENED: &str = "work.item_reopened";
    /// FORMAT-1.0 §12: a check's run, `{check, run, verdict, revision,
    /// commit?, output_sha256?, value?}`. Read here (a check's `failing`
    /// state); the checks runner that records it is a later step.
    pub const CHECK_RUN: &str = "check.run";
    /// FORMAT-1.0 §12: a trigger fired, `{check, trigger, subject}`. Read
    /// and carried; recorded by the checks runner, a later step.
    pub const TRIGGER_FIRED: &str = "trigger.fired";
    pub const NOTE_ADDED: &str = "work.note_added";
    pub const PROMOTED: &str = "work.promoted";
    /// OW-WAR-0148 M5: `war edit` changed the type, labels, epic or priority.
    pub const EDITED: &str = "work.edited";
    /// OW-WAR-0148 M5: the linked issue was written back to (or could not
    /// be): `outcome` is `written` or `unknown`.
    pub const ISSUE_WRITEBACK: &str = "work.issue_writeback";
}

/// `[tickets]` in `openwarrant.toml`. Every key is optional.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Where ticket directories live, relative to the root.
    #[serde(default)]
    pub dir: Option<String>,
    /// Where claim locks live, relative to the root or absolute. Unset, a git
    /// checkout keeps them under git's common directory, which every worktree
    /// of the clone shares (M11); set, this one directory is used instead.
    #[serde(default)]
    pub claims_dir: Option<String>,
    /// How old a claim must be, counted from when it was taken and whatever
    /// its renewals, before `--steal` may take it while its lease is live.
    #[serde(default)]
    pub claim_ttl_minutes: Option<u64>,
    /// M11: how long a claim's lease runs from its last renewal (default 30;
    /// a fraction is allowed). The holder's `war admin heartbeat` and every `war`
    /// command the holder runs renew it; once it runs out, a plain
    /// `war claim` takes the claim.
    #[serde(default)]
    pub claim_lease_minutes: Option<f64>,
    /// How many days a done ticket keeps its full block in `war view prime`
    /// before it collapses to one line.
    #[serde(default)]
    pub compact_after_days: Option<u64>,
}

/// `[tickets] claim_lease_minutes` in whole seconds; a negative or
/// non-finite value is the default.
fn lease_secs(minutes: Option<f64>) -> u64 {
    let m = minutes
        .filter(|m| m.is_finite() && *m >= 0.0)
        .unwrap_or(DEFAULT_LEASE_MINUTES);
    // Whole seconds of a non-negative, finite number of minutes.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let secs = (m * 60.0).round() as u64;
    secs
}

fn policy_of(
    root: &Utf8Path,
) -> Result<(Policy, remote::Policy, crate::preset::Policy), RepoError> {
    let path = root.join(crate::init::CONFIG_FILE);
    let text = crate::vfs::read_to_string(&path).map_err(|source| RepoError::Io {
        context: format!("could not read {path}"),
        source,
    })?;
    // M14: a preset that does not read is `war check`'s to report
    // (`preset.config`); the loop goes on as if there were none.
    let preset = crate::preset::Policy::from_text(&text).unwrap_or_default();
    #[derive(Deserialize)]
    struct File {
        #[serde(default)]
        tickets: Option<Policy>,
    }
    let file: File = toml::from_str(&text).map_err(|e| {
        RepoError::Message(format!("tickets.config: {path}: the [tickets] table: {e}"))
    })?;
    // M11: `[claims]`, read on its own so its refusal names it.
    #[derive(Deserialize)]
    struct Claims {
        #[serde(default)]
        claims: Option<remote::Policy>,
    }
    let claims: Claims = toml::from_str(&text).map_err(|e| {
        RepoError::Message(format!("tickets.config: {path}: the [claims] table: {e}"))
    })?;
    Ok((
        file.tickets.unwrap_or_default(),
        claims.claims.unwrap_or_default(),
        preset,
    ))
}

/// What a command answers: the report (refusals are error diagnostics in it),
/// the human rendering, and the `result` of the `--json` envelope.
#[derive(Debug)]
pub struct Outcome {
    pub report: Report,
    pub human: String,
    pub result: serde_json::Value,
}

impl Outcome {
    pub(crate) fn ok(human: impl Into<String>, result: serde_json::Value) -> Self {
        Self {
            report: Report::default(),
            human: human.into(),
            result,
        }
    }

    pub(crate) fn refused(rule: &str, file: impl Into<String>, message: impl Into<String>) -> Self {
        let message = message.into();
        let mut report = Report::default();
        report.push(Diagnostic::error(rule, file, message.clone()));
        Self {
            report,
            human: message,
            result: serde_json::Value::Null,
        }
    }

    /// What could not be established, said as `UNKNOWN` (Law 15): never
    /// a granted act and never a plain refusal. It did not happen, so
    /// [`Self::is_refused`] holds.
    pub(crate) fn unknown(rule: &str, file: impl Into<String>, message: impl Into<String>) -> Self {
        let message = message.into();
        let mut report = Report::default();
        report.push(Diagnostic::unknown(rule, file, message.clone()));
        Self {
            report,
            human: message,
            result: serde_json::Value::Null,
        }
    }

    pub(crate) fn from_diagnostic(d: Diagnostic) -> Self {
        let mut report = Report::default();
        let human = d.message.clone();
        report.push(d);
        Self {
            report,
            human,
            result: serde_json::Value::Null,
        }
    }

    /// Whether the command was refused, or could not be established
    /// (`UNKNOWN`): either way, it did not happen.
    #[must_use]
    pub fn is_refused(&self) -> bool {
        self.report.count(Severity::Error) > 0 || self.report.count(Severity::Unknown) > 0
    }
}

/// One ticket as read from disk.
#[derive(Debug, Clone)]
pub struct Ticket {
    /// The repository that supplied this reading, not a root reconstructed
    /// from a directory that may have been replaced since the load.
    pub(crate) repository_root: Utf8PathBuf,
    pub dir: Utf8PathBuf,
    pub manifest: TicketManifest,
    pub intent_path: Utf8PathBuf,
    pub checklist_path: Utf8PathBuf,
    pub intent: String,
    pub checklist_text: String,
    pub checklist: Checklist,
    /// M11: the ticket's revision as `war plan model` reports it (M3): the sha256
    /// of its manifest's bytes. What `--if-rev` compares on a ticket.
    pub revision: String,
    /// The nodes a run set aside ([`crate::go::blocked_nodes`]), when the
    /// ticket cache already read them from this ticket's journal while
    /// loading it; `None` (always, without the cache): read them from the
    /// journal when asked.
    pub(crate) blocked: Option<Vec<crate::go::BlockedNode>>,
    /// A Format 1.0 ticket (FORMAT-1.0 §4.1): one document, `intent_path`
    /// and `checklist_path` both naming it, its frontmatter standing for the
    /// manifest, which is never written ([`format`]).
    pub format: bool,
    /// FORMAT-1.0 §11.1: a Format 1.0 ticket with no items is done when a
    /// `work.item_done` names the document itself and no later
    /// `work.item_reopened` does (§11.2's later-wins rule).
    pub done_whole: bool,
    /// FORMAT-1.0 §11.1: what the document itself `depends_on` (its
    /// frontmatter relation), as far as it names tickets or their items:
    /// each of its items, and the document, waits on these.
    pub depends_on: Vec<Blocker>,
}

/// `sha256:<hex>` of `bytes`, as `war plan model` writes a revision.
fn revision_of(bytes: &[u8]) -> String {
    format!("sha256:{}", openwarrant_compiler::sha256_hex(bytes))
}

/// M11: an item's revision as `war plan model` reports it (M3): the sha256 of its
/// checklist line, newline included. What `--if-rev` compares on an item.
#[must_use]
pub fn item_revision(checklist_text: &str, item: &Item) -> String {
    let line = checklist_text
        .split_inclusive('\n')
        .nth(item.line)
        .unwrap_or_default();
    revision_of(line.as_bytes())
}

impl Ticket {
    /// The revision `--if-rev` compares for the ticket (`None`) or one of
    /// its items; `None` when the item is not in the checklist.
    #[must_use]
    pub fn revision_for(&self, item: Option<&str>) -> Option<String> {
        match item {
            None => Some(self.revision.clone()),
            Some(id) => self
                .item(id)
                .map(|i| item_revision(&self.checklist_text, i)),
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.manifest.id
    }

    /// Whether it is done (FORMAT-1.0 §11.1): it has items and every one is
    /// done, or, with none, a `work.item_done` names the document itself.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.checklist.is_done() || (self.checklist.items.is_empty() && self.done_whole)
    }

    /// The item with `id`, exactly.
    #[must_use]
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.checklist.item(id)
    }
}

/// Open, in progress or done — derived from the checklist and the claims,
/// never written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TicketState {
    Open,
    InProgress,
    Done,
}

impl TicketState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in progress",
            Self::Done => "done",
        }
    }
}

/// The repository's tickets, their claims, and who is acting.
pub struct Store {
    pub root: Utf8PathBuf,
    pub dir: Utf8PathBuf,
    /// Where new claims are taken: `[tickets] claims_dir`, else the clone's
    /// shared directory under git's common directory, else (outside git)
    /// [`DEFAULT_CLAIMS_DIR`].
    pub claims_dir: Utf8PathBuf,
    /// `[tickets] claims_dir`, else [`DEFAULT_CLAIMS_DIR`], under the root:
    /// what the evidence tree rule skips, as before claims were shared.
    tree_claims_dir: Utf8PathBuf,
    /// The checkout's git layout when claims are shared through it.
    layout: Option<claim::GitLayout>,
    /// Every worktree's own claims directory, from before M11: read lazily.
    legacy_dirs: std::sync::OnceLock<Vec<Utf8PathBuf>>,
    pub ttl_secs: u64,
    /// M11: a claim's lease, in seconds.
    pub lease_secs: u64,
    /// M11: `[claims] remote`, the git remote claims are published to.
    pub remote: Option<String>,
    pub compact_days: u64,
    pub definition: ProfileDefinition,
    /// Who this invocation acts as: `--as`, else `OPENWARRANT_ACTOR`, else
    /// `[project] performer`. A name for coordination; it authorizes nothing.
    pub actor: String,
    pub project: String,
    /// OW-WAR-0148 M14: `[preset] name`, when set. Under `vibe`, `war done`
    /// claims an unclaimed item itself.
    pub preset: Option<crate::preset::Preset>,
    /// OW-WAR-0148 M14: `[preset] ticks`, the least every tick shows
    /// (`claimed` without a preset).
    pub tick_floor: Level,
    /// OW-WAR-0148 M15: where `war done --check` runs the tests and KPIs,
    /// when that is not the root: `war evidence go` runs them in the
    /// worktree that holds the node's work, while the tick is written here.
    /// `None` everywhere else.
    pub check_root: Option<Utf8PathBuf>,
}

/// Now, in Unix seconds.
#[must_use]
pub fn now_secs() -> u64 {
    if let Some(at) = AT.get() {
        return *at;
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The time a command answers or acts at (`--at`), when given.
static AT: std::sync::OnceLock<u64> = std::sync::OnceLock::new();

/// Answer as of `secs` (Unix seconds) for the rest of this process: what
/// `war next --at <rfc3339>` sets, so its answer is a pure function of the
/// documents, journals, claim store and that time (FORMAT-1.0 §11.5). A
/// write given `--at` (`claim`, `done`, `note`) acts then: what it records
/// is dated then, for replaying a recorded history. The first setting
/// stands.
pub fn answer_at(secs: u64) {
    let _ = AT.set(secs);
}

fn rfc3339(secs: u64) -> String {
    crate::gate_cmd::receipt::rfc3339_from_secs(secs)
}

fn date_of(secs: u64) -> String {
    rfc3339(secs)[..10].to_owned()
}

fn io(context: String) -> impl FnOnce(std::io::Error) -> RepoError {
    move |source| RepoError::Io { context, source }
}

impl Store {
    /// Open the ticket store of `repo`, acting as `actor` when given.
    pub fn open(repo: &Repository, actor: Option<&str>) -> Result<Self, RepoError> {
        let (policy, claims_policy, preset) = policy_of(&repo.root)?;
        let definition = ticket_definition(&repo.profiles)?;
        let resolve = |p: &str| {
            let p = Utf8PathBuf::from(p);
            if p.is_absolute() {
                p
            } else {
                repo.root.join(p)
            }
        };
        let actor = actor
            .map(str::to_owned)
            .or_else(|| std::env::var("OPENWARRANT_ACTOR").ok())
            .map(|a| ticket::one_line(&a))
            .filter(|a| !a.is_empty())
            .unwrap_or_else(|| repo.performer());
        let tree_claims_dir = resolve(policy.claims_dir.as_deref().unwrap_or(DEFAULT_CLAIMS_DIR));
        let layout = match policy.claims_dir {
            Some(_) => None,
            None => claim::git_layout(&repo.root),
        };
        Ok(Self {
            root: repo.root.clone(),
            dir: resolve(policy.dir.as_deref().unwrap_or(DEFAULT_DIR)),
            claims_dir: layout.as_ref().map_or_else(
                || tree_claims_dir.clone(),
                claim::GitLayout::shared_claims_dir,
            ),
            tree_claims_dir,
            layout,
            legacy_dirs: std::sync::OnceLock::new(),
            // FORMAT-1.0 §11.3: `[claims]` holds both; `[tickets]`, where
            // they were before, is read when `[claims]` says nothing.
            ttl_secs: claims_policy
                .claim_ttl_minutes
                .or(policy.claim_ttl_minutes)
                .unwrap_or(DEFAULT_TTL_MINUTES)
                * 60,
            lease_secs: lease_secs(
                claims_policy
                    .claim_lease_minutes
                    .or(policy.claim_lease_minutes),
            ),
            remote: claims_policy.remote.filter(|r| !r.trim().is_empty()),
            compact_days: policy.compact_after_days.unwrap_or(DEFAULT_COMPACT_DAYS),
            definition,
            actor,
            project: repo.config.project.name.clone(),
            preset: preset.preset,
            tick_floor: if preset.preset.is_some() {
                preset.ticks
            } else {
                Level::Claimed
            },
            check_root: None,
        })
    }

    /// `path` relative to the root, for messages.
    #[must_use]
    pub fn rel(&self, path: &Utf8Path) -> String {
        path.strip_prefix(&self.root)
            .map_or_else(|_| path.to_string(), ToString::to_string)
    }

    /// What the loop writes as tickets are worked, relative to the root: the
    /// evidence tree rule skips it (t-5d82, `gate_cmd::source::is_ticket_record`).
    #[must_use]
    pub fn bookkeeping(&self) -> Bookkeeping {
        let rel = |p: &Utf8Path| {
            p.strip_prefix(&self.root)
                .ok()
                .map(|r| r.as_str().trim_end_matches('/').to_owned())
                .filter(|r| !r.is_empty() && r != ".")
        };
        Bookkeeping {
            dir: rel(&self.dir),
            claims_dir: rel(&self.tree_claims_dir),
            files: vec![
                "manifest.toml".to_owned(),
                crate::journal_cmd::FILE.to_owned(),
                INTENT_FILE.to_owned(),
                format!("atoms/{}", self.checklist_file()),
                // OW-WAR-0148 M13: the optional parts `war add` writes.
                ticks::CHECKS_FILE.to_owned(),
            ],
        }
    }

    pub(crate) fn checklist_file(&self) -> &str {
        self.definition
            .required_extension_roles
            .iter()
            .find(|r| r.role == CHECKLIST_ROLE)
            .map_or("15-checklist.md", |r| r.file.as_str())
    }

    pub(crate) fn checklist_ordinal(&self) -> u32 {
        self.definition
            .required_extension_roles
            .iter()
            .find(|r| r.role == CHECKLIST_ROLE)
            .map_or(15, |r| r.ordinal)
    }

    pub(crate) fn checklist_stub(&self) -> String {
        self.definition
            .required_extension_roles
            .iter()
            .find(|r| r.role == CHECKLIST_ROLE)
            .map_or_else(|| "# Checklist\n\n".to_owned(), |r| r.stub.clone())
    }

    /// Every ticket directory (one holding a `manifest.toml`), sorted.
    pub(crate) fn ticket_dirs(&self) -> Result<Vec<Utf8PathBuf>, RepoError> {
        Ok(self
            .entries()?
            .into_iter()
            .filter(|path| {
                crate::vfs::is_file(path.join("manifest.toml"))
                    && !crate::format_doc::is_document_dir(path)
            })
            .collect())
    }

    /// Every entry of the ticket directory, sorted, before
    /// [`Self::ticket_dirs`] keeps those holding a `manifest.toml`.
    pub(crate) fn entries(&self) -> Result<Vec<Utf8PathBuf>, RepoError> {
        let entries = match crate::vfs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => {
                return Err(RepoError::Io {
                    context: format!("could not read {}", self.dir),
                    source,
                });
            }
        };
        let mut out = Vec::new();
        for entry in entries {
            let entry = entry.map_err(io(format!("could not read {}", self.dir)))?;
            let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
                continue;
            };
            out.push(path);
        }
        // Every entry is a child of one directory, so path order is the
        // order of the last component's bytes: sorted on that, read once per
        // entry rather than walked out of both paths per comparison.
        out.sort_by_cached_key(|p| p.file_name().map(str::to_owned));
        Ok(out)
    }

    /// Read one ticket. `Err` names what is wrong with it. `dir` is a
    /// directory ticket's directory or a Format 1.0 ticket's evidence
    /// directory (`<tickets>/<alias>`, which need not exist).
    pub fn load(&self, dir: &Utf8Path) -> Result<Ticket, Diagnostic> {
        if let Some(doc) = format::document_of(dir) {
            return self.load_format(&doc, dir.file_name().unwrap_or_default());
        }
        self.load_observed(dir, &mut |_| {})
    }

    /// [`Self::load`], calling `before_read` with each file's path just
    /// before it is read: the manifest, then the intent atom, then the
    /// checklist atom (the ticket cache stats each there).
    pub(crate) fn load_observed(
        &self,
        dir: &Utf8Path,
        before_read: &mut dyn FnMut(&Utf8Path),
    ) -> Result<Ticket, Diagnostic> {
        let manifest_path = dir.join("manifest.toml");
        let rel = self.rel(&manifest_path);
        let bad = |message: String| Diagnostic::error("ticket.manifest", rel.clone(), message);
        before_read(&manifest_path);
        let text = crate::vfs::read_regular_to_string(
            self.root.as_std_path(),
            manifest_path.as_std_path(),
        )
        .map_err(|e| bad(format!("could not read it: {e}")))?;
        let manifest: TicketManifest =
            toml::from_str(&text).map_err(|e| bad(format!("does not parse: {e}")))?;
        let revision = revision_of(text.as_bytes());
        manifest
            .validate(&self.definition.working_roles())
            .map_err(bad)?;
        if dir.file_name() != Some(manifest.id.as_str()) {
            return Err(Diagnostic::error(
                "ticket.id-mismatch",
                rel,
                format!(
                    "the directory is {} and the manifest's id is {}; a light Warrant's \
                     directory is its id",
                    dir.file_name().unwrap_or_default(),
                    manifest.id
                ),
            ));
        }
        let mut atom = |role: &str| -> Result<(Utf8PathBuf, String), Diagnostic> {
            let path = dir.join(manifest.atom_path(role).unwrap_or_default());
            before_read(&path);
            crate::vfs::read_regular_to_string(self.root.as_std_path(), path.as_std_path())
                .map(|t| (path.clone(), t))
                .map_err(|e| {
                    Diagnostic::error(
                        "ticket.atom-missing",
                        self.rel(&path),
                        format!("the {role} atom could not be read: {e}"),
                    )
                })
        };
        let (intent_path, intent) = atom("intent")?;
        let (checklist_path, checklist_text) = atom(CHECKLIST_ROLE)?;
        let checklist = ticket::parse(&checklist_text);
        Ok(Ticket {
            repository_root: self.root.clone(),
            dir: dir.to_owned(),
            manifest,
            intent_path,
            checklist_path,
            intent,
            checklist_text,
            checklist,
            revision,
            blocked: None,
            format: false,
            done_whole: false,
            depends_on: Vec::new(),
        })
    }

    /// Every readable ticket, and a diagnostic per unreadable one.
    pub fn load_all(&self) -> Result<(Vec<Ticket>, Vec<Diagnostic>), RepoError> {
        self.load_every(true)
    }

    /// [`Self::load_all`], always from the files, never the ticket cache:
    /// for what judges an approval (`official`), which a cache must never
    /// answer.
    pub fn load_all_uncached(&self) -> Result<(Vec<Ticket>, Vec<Diagnostic>), RepoError> {
        self.load_every(false)
    }

    fn load_every(&self, cached: bool) -> Result<(Vec<Ticket>, Vec<Diagnostic>), RepoError> {
        let mut tickets = Vec::new();
        let mut faults = Vec::new();
        // Each ticket is a few small files read on their own, so the disk
        // reads fan out across threads; the order is the sorted directory
        // order either way (`vfs::map_ordered`). Where this run may use the
        // ticket cache, it answers what is unchanged since it was read
        // (`cache`): the same tickets, faster.
        let (entries, loaded) = match cache::Cache::open(self).filter(|_| cached) {
            Some(cache) => cache.load(self)?,
            None => {
                let entries = self.entries()?;
                let loaded = crate::vfs::map_ordered(&entries, 256, |dir| {
                    crate::vfs::is_file(dir.join("manifest.toml"))
                        .then(|| self.load_observed(dir, &mut |_| {}))
                });
                (entries, loaded)
            }
        };
        // A directory no directory ticket was read from may hold a Format
        // 1.0 `ticket.md` (FORMAT-1.0 §4.1); a manifest beside one holds
        // tool settings only (§4.2), so it is read as no directory ticket.
        let mut look: BTreeSet<&Utf8Path> = BTreeSet::new();
        for (dir, one) in entries.iter().zip(loaded) {
            match one {
                Some(Ok(t)) => tickets.push(t),
                Some(Err(_)) if crate::vfs::exists(dir.join(format::INNER)) => {
                    look.insert(dir);
                }
                Some(Err(d)) => faults.push(d),
                None => {
                    look.insert(dir);
                }
            }
        }
        let mut any_format = false;
        for one in self.format_tickets(&entries, &|d| look.contains(d)) {
            match one {
                Ok(t) => {
                    any_format = true;
                    tickets.push(t);
                }
                Err(d) => faults.push(d),
            }
        }
        if any_format {
            // One order for both encodings: the directory's, which is the
            // ids'.
            tickets.sort_by(|a, b| a.id().cmp(b.id()));
        }
        Ok((tickets, faults))
    }

    /// What `war claim <query>` reads: every readable ticket [`resolve`]
    /// could name for `query`, and every ticket their items wait on, which
    /// is all [`open_blockers`] looks up. For `t-x` or `t-x/i-y` those are
    /// the tickets whose directory name starts with `t-x` (a ticket's
    /// directory is its id) and the blockers' own; for any other query
    /// (`i-y` searches every ticket's items) every ticket, as
    /// [`Self::load_all`]. A ticket outside the set can neither match the
    /// query nor be one a match waits on, so resolving over the set gives
    /// what resolving over every ticket gives.
    pub fn load_for(&self, query: &str) -> Result<(Vec<Ticket>, Vec<Diagnostic>), RepoError> {
        let query = query.trim();
        let prefix = match query.split_once('/') {
            Some((t, _)) => t,
            None if query.starts_with("t-") => query,
            None => return self.load_all(),
        };
        let entries = self.entries()?;
        // A directory ticket's directory is its id; a single-file Format
        // 1.0 ticket's file is `<id>.md`.
        let named = |dir: &Utf8PathBuf, name: &str| {
            dir.file_name()
                .is_some_and(|n| n == name || format::single_file(n) == Some(name))
        };
        let mut tickets: Vec<Ticket> = Vec::new();
        let mut faults = Vec::new();
        let mut take = |path: &Utf8PathBuf, tickets: &mut Vec<Ticket>| {
            let name = path.file_name().unwrap_or_default();
            let one = if let Some(alias) = format::single_file(name) {
                Some(self.load_format(path, alias))
            } else if crate::vfs::exists(path.join(format::INNER)) {
                Some(self.load_format(&path.join(format::INNER), name))
            } else if crate::vfs::is_file(path.join("manifest.toml")) {
                Some(self.load_observed(path, &mut |_| {}))
            } else {
                None
            };
            match one {
                Some(Ok(t)) => tickets.push(t),
                Some(Err(d)) => faults.push(d),
                None => {}
            }
        };
        for dir in entries
            .iter()
            .filter(|d| d.file_name().is_some_and(|n| n.starts_with(prefix)))
        {
            take(dir, &mut tickets);
        }
        let waited_on: BTreeSet<String> = tickets
            .iter()
            .flat_map(|t| {
                t.checklist
                    .items
                    .iter()
                    .flat_map(|i| i.after.iter())
                    .chain(&t.depends_on)
                    .map(move |b| ticket::split_record_id(&b.record_id(t.id())).0.to_owned())
            })
            .filter(|id| !tickets.iter().any(|t| t.id() == id))
            .collect();
        for id in &waited_on {
            for dir in entries.iter().filter(|d| named(d, id)) {
                take(dir, &mut tickets);
            }
        }
        Ok((tickets, faults))
    }

    /// Where claims taken before M11 may still lie: each worktree's own
    /// [`DEFAULT_CLAIMS_DIR`], this worktree's first. Empty when
    /// `[tickets] claims_dir` names the directory or there is no git
    /// checkout (the one directory is then [`Self::claims_dir`] itself).
    pub fn legacy_claims_dirs(&self) -> &[Utf8PathBuf] {
        self.legacy_dirs.get_or_init(|| {
            let Some(layout) = &self.layout else {
                return Vec::new();
            };
            let below = self
                .root
                .strip_prefix(&layout.toplevel)
                .map(Utf8Path::to_owned)
                .unwrap_or_default();
            layout
                .worktrees()
                .into_iter()
                .map(|top| top.join(&below).join(DEFAULT_CLAIMS_DIR))
                .collect()
        })
    }

    /// Every claim now held, by lock file name: the shared directory's, then
    /// any from before M11 in a worktree's own directory that the shared one
    /// does not hold.
    pub fn claims(&self) -> Result<BTreeMap<String, Option<claim::Claim>>, RepoError> {
        let mut out = claim::all(&self.claims_dir, self.lease_secs)
            .map_err(io(format!("could not read {}", self.claims_dir)))?;
        for dir in self.legacy_claims_dirs() {
            // Another worktree's directory may be gone or unreadable; what
            // cannot be read holds nothing.
            for (name, c) in claim::all(dir, self.lease_secs).unwrap_or_default() {
                out.entry(name).or_insert(c);
            }
        }
        Ok(out)
    }

    /// Where a new claim on the target is taken.
    fn lock_path(&self, ticket: &str, item: Option<&str>) -> Utf8PathBuf {
        self.claims_dir.join(claim::lock_name(ticket, item))
    }

    /// Where the claim on the target lies now: the shared directory's lock,
    /// else one from before M11 in a worktree's own directory, else (no claim)
    /// where a new one would be taken.
    fn lock_of(&self, ticket: &str, item: Option<&str>) -> Utf8PathBuf {
        let shared = self.lock_path(ticket, item);
        if crate::vfs::exists(&shared) {
            return shared;
        }
        let name = claim::lock_name(ticket, item);
        self.legacy_claims_dirs()
            .iter()
            .map(|d| d.join(&name))
            .find(|p| crate::vfs::exists(p))
            .unwrap_or(shared)
    }

    /// Read the claim on the target where it lies.
    fn read_claim(&self, path: &Utf8Path) -> std::io::Result<Option<Option<claim::Claim>>> {
        claim::read(path, self.lease_secs)
    }

    /// M11: renew the lease of every claim this actor holds, or of the one
    /// lock named `only`: a stat-sized touch per lock, no ticket is read.
    /// Returns the claims renewed. Nothing is renewed in a hosted run, which
    /// writes nothing.
    pub fn renew_held(&self, only: Option<&str>) -> Vec<claim::Claim> {
        if crate::vfs::is_hosted() {
            return Vec::new();
        }
        let mut renewed = Vec::new();
        let mut seen = BTreeSet::new();
        let dirs = std::iter::once(&self.claims_dir).chain(self.legacy_claims_dirs());
        for dir in dirs {
            for path in claim::lock_files(dir) {
                let name = path.file_name().unwrap_or_default().to_owned();
                if only.is_some_and(|o| o != name) || seen.contains(&name) {
                    continue;
                }
                if let Ok(Some(c)) = claim::renew(&path, &self.actor, self.lease_secs) {
                    seen.insert(name);
                    renewed.push(c);
                }
            }
        }
        renewed
    }

    pub(crate) fn journal(
        &self,
        t: &Ticket,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<(), RepoError> {
        let mut payload = payload.clone();
        // FORMAT-1.0 §11.2: so that a fact that really happens again (done,
        // reopened, done again) is a new event, a Format 1.0 document's
        // claim, done, reopen and release carry `episode`.
        if t.format
            && EPISODIC.contains(&event_type)
            && payload.get("episode").is_none()
            && let Some(target) = payload.get("target").and_then(|v| v.as_str())
        {
            let n = episode_of(&t.dir, event_type, target)?;
            payload["episode"] = serde_json::json!(n);
        }
        crate::journal_cmd::record(
            &t.dir,
            &t.manifest.uuid,
            event_type,
            &self.actor,
            &payload.to_string(),
        )
        .map(|_| ())
    }
}

/// The event types whose payload carries `episode` (FORMAT-1.0 §11.2):
/// `work.claimed` (and `work.claim_reclaimed`, `work.claim_stolen`, "as
/// `work.claimed`"), `work.item_done`, `work.item_reopened`,
/// `work.released`.
pub const EPISODIC: [&str; 6] = [
    event::CLAIMED,
    event::CLAIM_RECLAIMED,
    event::CLAIM_STOLEN,
    event::ITEM_DONE,
    event::ITEM_REOPENED,
    event::RELEASED,
];

/// `episode` (FORMAT-1.0 §11.2): the number of events of `event_type` for
/// `target` already in the journal in `dir`, its `ticket.*` spelling read as
/// the `work.*` one.
pub fn episode_of(dir: &Utf8Path, event_type: &str, target: &str) -> Result<u64, RepoError> {
    let journal = crate::journal_cmd::load(dir)?;
    let kind = crate::journal_cmd::work_type(event_type);
    let n = journal
        .events
        .iter()
        .filter(|e| crate::journal_cmd::work_type(&e.event_type) == kind)
        .filter(|e| {
            serde_json::from_str::<serde_json::Value>(&e.payload)
                .ok()
                .and_then(|p| {
                    p.get("target")
                        .and_then(|v| v.as_str())
                        .map(|t| t == target)
                })
                .unwrap_or(false)
        })
        .count();
    Ok(u64::try_from(n).unwrap_or(u64::MAX))
}

/// M11: renew the claims of the agent a command runs as when it names none
/// (`$OPENWARRANT_ACTOR`, else `[project] performer`). Best effort and
/// silent: a repository without tickets, or a claim that cannot be touched,
/// changes nothing about the command.
pub fn renew_ambient(repo: &Repository) {
    if let Ok(store) = Store::open(repo, None) {
        store.renew_held(None);
    }
}

/// The `ticket` profile: the repository's own `profiles/ticket.toml`, else the
/// one this build ships. Refused when the repository defines `ticket` as
/// something other than a working form.
fn ticket_definition(registry: &ProfileRegistry) -> Result<ProfileDefinition, RepoError> {
    let from = |r: &ProfileRegistry| {
        r.resolve(TICKET_PROFILE)
            .ok()
            .and_then(|p| r.definition(&p).cloned())
    };
    let definition = match from(registry) {
        Some(d) => d,
        None => {
            let builtin = ProfileRegistry::with_definitions([(
                "profiles/ticket.toml",
                BUILTIN_PROFILE.as_bytes(),
            )])
            .map_err(|e| RepoError::Message(format!("the built-in ticket profile: {e}")))?;
            from(&builtin).ok_or_else(|| {
                RepoError::Message("the built-in ticket profile did not resolve".to_owned())
            })?
        }
    };
    if !definition.is_working_form() {
        return Err(RepoError::Message(
            "tickets.profile: profiles/ticket.toml does not declare `form = \"working\"`; a \
             ticket is the working form of a Warrant, and this definition would make it a \
             contract"
                .to_owned(),
        ));
    }
    if !definition
        .required_extension_roles
        .iter()
        .any(|r| r.role == CHECKLIST_ROLE)
    {
        return Err(RepoError::Message(format!(
            "tickets.profile: profiles/ticket.toml requires no {CHECKLIST_ROLE} role"
        )));
    }
    Ok(definition)
}

// ---- lookup ----------------------------------------------------------------

/// Whether `s` names a ticket or an item (`t-...`, `i-...`, `t-.../i-...`)
/// rather than a Warrant alias, which is `NS-WAR-NNNN` and never lowercase.
#[must_use]
pub fn is_ticket_ref(s: &str) -> bool {
    s.starts_with("t-") || s.starts_with("i-")
}

/// What a command acts on.
#[derive(Debug, Clone)]
pub enum Target {
    Ticket(usize),
    Item(usize, String),
}

fn pick<'a>(candidates: &[&'a str], query: &str) -> Result<&'a str, Vec<&'a str>> {
    if let Some(exact) = candidates.iter().find(|c| **c == query) {
        return Ok(exact);
    }
    let matches: Vec<&str> = candidates
        .iter()
        .copied()
        .filter(|c| c.starts_with(query))
        .collect();
    match matches.as_slice() {
        [one] => Ok(one),
        _ => Err(matches),
    }
}

/// Resolve `t-x`, `t-x/i-y` or `i-y` (each a unique prefix) against `tickets`.
pub fn resolve(tickets: &[Ticket], query: &str) -> Result<Target, Diagnostic> {
    let query = query.trim();
    let unknown = |what: &str, found: &[&str]| {
        Diagnostic::error(
            if found.is_empty() {
                "ticket.unknown"
            } else {
                "ticket.ambiguous"
            },
            String::new(),
            if found.is_empty() {
                format!("no {what} is {query:?}; `war next` and `war view warrants` list them")
            } else {
                format!(
                    "{query:?} names more than one {what}: {}. Give more of the id",
                    found.join(", ")
                )
            },
        )
    };
    let find_ticket = |q: &str| -> Result<usize, Diagnostic> {
        let ids: Vec<&str> = tickets.iter().map(Ticket::id).collect();
        let id = pick(&ids, q).map_err(|found| unknown("Warrant", &found))?;
        Ok(tickets
            .iter()
            .position(|t| t.id() == id)
            .unwrap_or_default())
    };
    if let Some((t, i)) = query.split_once('/') {
        let index = find_ticket(t)?;
        let ids: Vec<String> = tickets[index].checklist.ids().into_iter().collect();
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        let item = pick(&refs, i).map_err(|found| unknown("item", &found))?;
        return Ok(Target::Item(index, item.to_owned()));
    }
    if query.starts_with("t-") {
        return find_ticket(query).map(Target::Ticket);
    }
    if query.starts_with("i-") {
        let all: Vec<(usize, String)> = tickets
            .iter()
            .enumerate()
            .flat_map(|(n, t)| t.checklist.ids().into_iter().map(move |i| (n, i)))
            .collect();
        let exact: Vec<&(usize, String)> = all.iter().filter(|(_, i)| i == query).collect();
        let found: Vec<&(usize, String)> = if exact.is_empty() {
            all.iter().filter(|(_, i)| i.starts_with(query)).collect()
        } else {
            exact
        };
        return match found.as_slice() {
            [(n, id)] => Ok(Target::Item(*n, id.clone())),
            [] => Err(unknown("item", &[])),
            many => {
                let names: Vec<String> = many
                    .iter()
                    .map(|(n, i)| format!("{}/{i}", tickets[*n].id()))
                    .collect();
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                Err(unknown("item", &refs))
            }
        };
    }
    Err(Diagnostic::error(
        "ticket.unknown",
        String::new(),
        format!(
            "{query:?} is not a light Warrant (t-...), an item (i-...) or an item of one (t-.../i-...)"
        ),
    ))
}

// ---- derived state ---------------------------------------------------------

/// The claim on a ticket's item, or on the whole ticket, if any.
pub(crate) fn claim_on<'a>(
    claims: &'a BTreeMap<String, Option<claim::Claim>>,
    ticket: &str,
    item: Option<&str>,
) -> Option<(&'a String, Option<&'a claim::Claim>)> {
    claims
        .get_key_value(&claim::lock_name(ticket, item))
        .map(|(k, v)| (k, v.as_ref()))
}

/// `work.claimed-by-other` (FORMAT-1.0 §11.3): a claim, or a write, on a
/// target someone else holds, naming the holder.
pub(crate) fn claimed_by_other(file: impl Into<String>, message: impl Into<String>) -> Outcome {
    let rule = crate::work_rules::CLAIMED_BY_OTHER;
    Outcome::refused(
        rule,
        file,
        format!("{}{}", message.into(), crate::work_rules::formerly(rule)),
    )
}

/// Who holds `name` (for a message), or `somebody` for a lock that names nobody.
fn holder(c: Option<&claim::Claim>, now: u64) -> String {
    c.map_or_else(
        || "somebody (the lock file does not parse)".to_owned(),
        |c| {
            format!(
                "{} since {} ({} ago)",
                c.actor,
                c.since,
                render::ago(c.age(now))
            )
        },
    )
}

/// The blockers of `item` in `t` that are not done yet, by name; an unknown
/// one is named as such and blocks (fail closed). The one definition of
/// "blocked": `ready_rows` and `claim_cmd` both call it over a fresh
/// `Store::load_all` of the checklist files, with no cache between them
/// (t-9d3e).
///
/// OW-WAR-0148 M5: a blocker is read as the kernel reads it, a `depends_on`
/// relation to a global record id (`t-x/i-y` or `t-x`,
/// [`ticket::Blocker::record_id`]); `war plan model` emits the same relation.
fn open_blockers(tickets: &[Ticket], t: &Ticket, item: &Item) -> Vec<String> {
    open_blockers_by(&|id| tickets.iter().find(|x| x.id() == id), t, item)
}

/// [`open_blockers`], finding a ticket by id with `find`: what a scan of
/// the tickets for the first with that id finds ([`first_by_id`]).
fn open_blockers_by<'a>(
    find: &dyn Fn(&str) -> Option<&'a Ticket>,
    t: &Ticket,
    item: &Item,
) -> Vec<String> {
    // FORMAT-1.0 §11.1: an item also waits on what its document
    // `depends_on`.
    item.after
        .iter()
        .chain(&t.depends_on)
        .filter_map(|b| match record_done(find, t, &b.record_id(t.id())) {
            Some(true) => None,
            Some(false) => Some(b.to_string()),
            None => Some(format!("{b} (unknown)")),
        })
        .collect()
}

/// The blockers of `item` in `t` that are not done, by name, looked up in
/// `tickets`: FORMAT-1.0 §11.1's `blocked`, read as `war claim` reads it.
#[must_use]
pub fn waiting_on(tickets: &[Ticket], t: &Ticket, item: &Item) -> Vec<String> {
    open_blockers(tickets, t, item)
}

/// What a work document itself waits on that is not done (its
/// `depends_on`, FORMAT-1.0 §11.1): a document with no items is `blocked`
/// by these as an item would be.
fn document_blockers_by<'a>(find: &dyn Fn(&str) -> Option<&'a Ticket>, t: &Ticket) -> Vec<String> {
    t.depends_on
        .iter()
        .filter_map(|b| match record_done(find, t, &b.record_id(t.id())) {
            Some(true) => None,
            Some(false) => Some(b.to_string()),
            None => Some(format!("{b} (unknown)")),
        })
        .collect()
}

/// Each id's first ticket in `tickets`, by id: the one
/// `tickets.iter().find(|x| x.id() == id)` finds, without the scan.
fn first_by_id(tickets: &[Ticket]) -> std::collections::HashMap<&str, &Ticket> {
    let mut by = std::collections::HashMap::with_capacity(tickets.len());
    for t in tickets {
        by.entry(t.id()).or_insert(t);
    }
    by
}

/// Whether the ticket record `id` (`t-x` or `t-x/i-y`) is done; `None` when
/// it names nothing. `t` is consulted first, as the ticket being read now.
fn record_done<'a>(
    find: &dyn Fn(&str) -> Option<&'a Ticket>,
    t: &Ticket,
    id: &str,
) -> Option<bool> {
    let (tid, iid) = ticket::split_record_id(id);
    let owner: &Ticket = if tid == t.id() { t } else { find(tid)? };
    match iid {
        Some(i) => owner.item(i).map(|x| x.done),
        None => Some(owner.is_done()),
    }
}

/// The tickets `part_of` `id`, in id order: an epic's tickets.
pub(crate) fn children_of<'a>(tickets: &'a [Ticket], id: &str) -> Vec<&'a Ticket> {
    tickets
        .iter()
        .filter(|x| x.manifest.part_of.as_deref() == Some(id))
        .collect()
}

/// A ticket's state given the claims now held.
pub(crate) fn state_of(t: &Ticket, claims: &BTreeMap<String, Option<claim::Claim>>) -> TicketState {
    if t.is_done() {
        return TicketState::Done;
    }
    let claimed = claims
        .keys()
        .any(|k| k == &claim::lock_name(t.id(), None) || k.starts_with(&format!("{}--", t.id())));
    if claimed || t.checklist.items.iter().any(|i| i.done) {
        TicketState::InProgress
    } else {
        TicketState::Open
    }
}

/// A work document's phase (FORMAT-1.0 §11.1) with the documents `part_of`
/// it counted as its items, when `tickets` is given: done when it has items
/// or children and every one is done; in progress when one is done or
/// claimed (or it is claimed itself); else open. Without `tickets`, or with
/// no child, [`state_of`].
pub fn phase_of(t: &Ticket, tickets: Option<&[Ticket]>, claims: &Claims) -> TicketState {
    phase_within(t, tickets, claims, 0)
}

fn phase_within(
    t: &Ticket,
    tickets: Option<&[Ticket]>,
    claims: &Claims,
    depth: usize,
) -> TicketState {
    let own = state_of(t, claims);
    let Some(all) = tickets else { return own };
    let children = children_of(all, t.id());
    // A `part_of` cycle is refused (`ticket.part-of-cycle`); the depth only
    // keeps a refused one from recursing for ever.
    if children.is_empty() || depth > 64 {
        return own;
    }
    let kids: Vec<TicketState> = children
        .iter()
        .map(|c| phase_within(c, tickets, claims, depth + 1))
        .collect();
    let own_items_done = t.checklist.items.iter().all(|i| i.done);
    if own_items_done && kids.iter().all(|k| *k == TicketState::Done) {
        TicketState::Done
    } else if own != TicketState::Open || kids.iter().any(|k| *k != TicketState::Open) {
        TicketState::InProgress
    } else {
        TicketState::Open
    }
}

// ---- compare-and-set (M11) ---------------------------------------------------

/// `--if-rev`: whether the revision the caller read is still the record's.
/// `sha256:` is optional on the caller's side.
fn same_revision(given: &str, current: &str) -> bool {
    let bare = |s: &str| s.trim().trim_start_matches("sha256:").to_ascii_lowercase();
    bare(given) == bare(current)
}

/// The refusal of a write whose `--if-rev` is not the record's revision now:
/// `work.stale-revision` (FORMAT-1.0 §11.3), naming both, and how to read
/// the current one.
fn stale_revision(
    store: &Store,
    t: &Ticket,
    what: &str,
    file: &Utf8Path,
    given: &str,
    current: &str,
) -> Outcome {
    let rule = crate::work_rules::STALE_REVISION;
    Outcome::refused(
        rule,
        store.rel(file),
        format!(
            "{what} changed since you read it: you passed revision {given}, and it is now \
             {current}. Nothing was written. Read it again (`war show {} --json`) and retry \
             with the revision it gives{}",
            t.id(),
            crate::work_rules::formerly(rule)
        ),
    )
}

/// `--if-rev` against the target as loaded: `Some(refusal)` when stale.
fn check_if_rev(
    store: &Store,
    t: &Ticket,
    item: Option<&str>,
    what: &str,
    if_rev: Option<&str>,
) -> Option<Outcome> {
    let given = if_rev?;
    let current = t.revision_for(item).unwrap_or_default();
    if same_revision(given, &current) {
        return None;
    }
    // FORMAT-1.0 §13.1: a stale write is reported at the document and, for
    // an item, the item's line.
    let (file, line) = match item.and_then(|i| t.item(i)) {
        Some(it) => (t.checklist_path.clone(), Some(it.line + 1)),
        None => (record_file(t), None),
    };
    let mut refusal = stale_revision(store, t, what, &file, given, &current);
    if let Some(n) = line {
        for d in &mut refusal.report.diagnostics {
            d.file = d.file.take().map(|f| format!("{f}:{n}"));
        }
    }
    Some(refusal)
}

// ---- writes ----------------------------------------------------------------

/// One writer at a time in `dir`: an advisory lock (flock) on the directory
/// itself, held until the returned file is dropped and released by the
/// kernel however the process ends. `None` where the filesystem has none;
/// the prestate check below still stands then.
fn dir_lock(dir: &Utf8Path) -> Option<std::fs::File> {
    let f = std::fs::File::open(dir).ok()?;
    f.lock().ok()?;
    Some(f)
}

/// Rewrite a file from its current bytes, retrying when another writer moved
/// it between the read and the rename (`storage.prestate-moved`). `edit`
/// returns the new text and a value, or `Err` to write nothing.
///
/// M11: the read, the edit and the rename run under a lock on the file's
/// directory. `write_if` compares the prestate and then renames, two steps:
/// two writers could both pass the compare and the second rename would drop
/// the first one's line (found by plant 100's compare-and-set race). Under
/// the lock no write lands between another's read and its rename.
fn rewrite<T>(
    path: &Utf8Path,
    mut edit: impl FnMut(&str) -> Result<(String, T), Box<Outcome>>,
) -> Result<Result<T, Box<Outcome>>, RepoError> {
    let _held = path.parent().and_then(dir_lock);
    for _ in 0..8 {
        let bytes = crate::vfs::read(path).map_err(io(format!("could not read {path}")))?;
        let text = String::from_utf8(bytes.clone())
            .map_err(|e| RepoError::Message(format!("{path}: not UTF-8 ({e})")))?;
        let (next, value) = match edit(&text) {
            Ok(v) => v,
            Err(refusal) => return Ok(Err(refusal)),
        };
        if next == text {
            return Ok(Ok(value));
        }
        match atomic::write_if(path, next.as_bytes(), &atomic::Prestate::of(&bytes)) {
            Ok(()) => return Ok(Ok(value)),
            Err(r) if r.rule == "storage.prestate-moved" => {}
            Err(r) => return Err(r.into()),
        }
    }
    Err(RepoError::Message(format!(
        "storage.prestate-moved: {path} kept changing under this write; nothing was written"
    )))
}

/// [`rewrite`], for a file that may not exist yet: absent, it is `stub`
/// edited, written only if it is still absent (OW-WAR-0148 M13).
fn rewrite_or_create<T>(
    path: &Utf8Path,
    stub: &str,
    mut edit: impl FnMut(&str) -> Result<(String, T), Box<Outcome>>,
) -> Result<Result<T, Box<Outcome>>, RepoError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io(format!("could not create {parent}")))?;
    }
    // One writer at a time, as `rewrite` (M11).
    let _held = path.parent().and_then(dir_lock);
    for _ in 0..8 {
        let (current, prestate) = match crate::vfs::read(path) {
            Ok(bytes) => {
                let text = String::from_utf8(bytes.clone())
                    .map_err(|e| RepoError::Message(format!("{path}: not UTF-8 ({e})")))?;
                (text, atomic::Prestate::of(&bytes))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                (stub.to_owned(), atomic::Prestate::Absent)
            }
            Err(source) => {
                return Err(RepoError::Io {
                    context: format!("could not read {path}"),
                    source,
                });
            }
        };
        let (next, value) = match edit(&current) {
            Ok(v) => v,
            Err(refusal) => return Ok(Err(refusal)),
        };
        if next == current && prestate != atomic::Prestate::Absent {
            return Ok(Ok(value));
        }
        match atomic::write_if(path, next.as_bytes(), &prestate) {
            Ok(()) => return Ok(Ok(value)),
            Err(r) if r.rule == "storage.prestate-moved" => {}
            Err(r) => return Err(r.into()),
        }
    }
    Err(RepoError::Message(format!(
        "storage.prestate-moved: {path} kept changing under this write; nothing was written"
    )))
}

fn fresh_item_id(taken: &BTreeSet<String>) -> String {
    ticket::item_id(&WarUuid::mint().to_string(), taken)
}

/// The checklist text with every unnamed item given an id.
fn named(text: &str) -> (String, Vec<String>) {
    ticket::name_unnamed(text, &mut fresh_item_id)
}

/// [`named`] for `t`'s checklist file: for a Format 1.0 ticket, only the
/// box lines that are items (§3.8) are named.
fn named_in(t: &Ticket, text: &str) -> (String, Vec<String>) {
    if t.format {
        format::name_unnamed(text, &mut fresh_item_id)
    } else {
        named(text)
    }
}

/// `t`'s checklist as `text` holds it now.
fn checklist_in(t: &Ticket, text: &str) -> Checklist {
    if t.format {
        format::checklist_of(text)
    } else {
        ticket::parse(text)
    }
}

/// `text` with the item `line` added where `t`'s next item goes.
fn with_item(t: &Ticket, text: &str, c: &Checklist, line: &str) -> String {
    if t.format {
        format::append_item(text, line)
    } else {
        ticket::append_item(text, c, line)
    }
}

/// The file a write to the whole of `t` is based on: its manifest, or a
/// Format 1.0 ticket's document.
fn record_file(t: &Ticket) -> Utf8PathBuf {
    if t.format {
        t.checklist_path.clone()
    } else {
        t.dir.join("manifest.toml")
    }
}

/// The refusal of a command that writes what a Format 1.0 ticket does not
/// have: a manifest, or an atom beside its document.
pub(crate) fn format_unsupported(store: &Store, t: &Ticket, what: &str) -> Option<Outcome> {
    t.format.then(|| {
        Outcome::refused(
            format::UNSUPPORTED,
            store.rel(&t.checklist_path),
            format!(
                "{} is a Format 1.0 ticket: one document, no manifest and no atoms beside \
                 it, so {what} is not written by war. Edit the document itself (its \
                 frontmatter is plain `key: value` lines)",
                t.id()
            ),
        )
    })
}

// ---- create ----------------------------------------------------------------

/// What `war create` is asked for.
#[derive(Debug, Clone, Default)]
pub struct CreateArgs {
    pub title: String,
    /// Format 1.0 section 10.1: one line, at most 200 bytes, saying what the
    /// work is for. Written to the manifest's `summary` when given.
    pub summary: Option<String>,
    pub items: Vec<String>,
    pub body: Option<String>,
    pub priority: Option<u8>,
    /// OW-WAR-0148 M5: one of the profile's `[fields] types`.
    pub kind: Option<String>,
    /// Labels; refused outside a closed set.
    pub labels: Vec<String>,
    /// The ticket (epic) this one is part of: an id or a unique prefix.
    pub part_of: Option<String>,
    /// The GitHub issue it was made from, as intake read it.
    pub issue: Option<crate::plan::intake::Issue>,
}

/// Why a type or a label set is refused by the profile, as a refusal.
fn refuse_fields(store: &Store, kind: Option<&str>, labels: &[String]) -> Option<Outcome> {
    let fields = &store.definition.fields;
    if let Some(k) = kind {
        if !ticket::is_field_word(k) {
            return Some(Outcome::refused(
                "ticket.type-unknown",
                String::new(),
                format!("type {k:?} is not a word (lowercase, [a-z0-9_-])"),
            ));
        }
        if let Some(why) = fields.refuse_type(k) {
            return Some(Outcome::refused(
                "ticket.type-unknown",
                "profiles/ticket.toml",
                why,
            ));
        }
    }
    for l in labels {
        if !ticket::is_field_word(l) {
            return Some(Outcome::refused(
                "ticket.label-malformed",
                String::new(),
                format!("label {l:?} is not a word (lowercase, [a-z0-9_-], 1 to 40)"),
            ));
        }
        if let Some(why) = fields.refuse_label(l) {
            return Some(Outcome::refused(
                "ticket.label-unknown",
                "profiles/ticket.toml",
                why,
            ));
        }
    }
    None
}

/// `labels`, sorted and unique.
fn label_set(labels: &[String]) -> Vec<String> {
    labels
        .iter()
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Resolve `--part-of` to a ticket id, refusing one that is unknown or that
/// would make `child` (when it exists) part of itself through the chain.
fn parent_of(tickets: &[Ticket], query: &str, child: Option<&str>) -> Result<String, Box<Outcome>> {
    let parent = match resolve(tickets, query) {
        Ok(Target::Ticket(n)) => tickets[n].id().to_owned(),
        Ok(Target::Item(..)) => {
            return Err(Box::new(Outcome::refused(
                "ticket.part-of",
                String::new(),
                format!(
                    "`--part-of {query}` names an item; a Warrant is part of a Warrant (an epic)"
                ),
            )));
        }
        Err(d) => return Err(Box::new(Outcome::from_diagnostic(d))),
    };
    if child == Some(parent.as_str()) {
        return Err(Box::new(Outcome::refused(
            "ticket.part-of-cycle",
            String::new(),
            format!("{parent} cannot be part of itself"),
        )));
    }
    if let Some(child) = child {
        let mut at = Some(parent.clone());
        let mut seen = BTreeSet::new();
        while let Some(id) = at {
            if id == child {
                return Err(Box::new(Outcome::refused(
                    "ticket.part-of-cycle",
                    String::new(),
                    format!(
                        "{child} cannot be part of {parent}: {parent} is already part of {child}, \
                         through `part_of`"
                    ),
                )));
            }
            if !seen.insert(id.clone()) {
                break;
            }
            at = tickets
                .iter()
                .find(|t| t.id() == id)
                .and_then(|t| t.manifest.part_of.clone());
        }
    }
    Ok(parent)
}

pub(crate) fn toml_of(m: &TicketManifest) -> Result<String, RepoError> {
    let body = toml::to_string(m)
        .map_err(|e| RepoError::Message(format!("could not render the ticket manifest: {e}")))?;
    Ok(format!(
        "# A Warrant in its light encoding (a ticket; OW-WAR-0147): the working form. The atoms\n\
         # beside this file are the Warrant; `war show {}` renders them. Nothing here is signed.\n{body}",
        m.id
    ))
}

/// `war create`: a ticket, workable at once.
pub fn create(store: &Store, args: &CreateArgs) -> Result<Outcome, RepoError> {
    let title = ticket::one_line(&args.title);
    if title.is_empty() {
        return Ok(Outcome::refused(
            "ticket.title-empty",
            String::new(),
            "a Warrant needs a title: `war create \"what this work accomplishes\"`",
        ));
    }
    let summary = args
        .summary
        .as_deref()
        .map(ticket::one_line)
        .filter(|s| !s.is_empty());
    if let Some(s) = &summary
        && s.len() > openwarrant_core::format::SUMMARY_MAX
    {
        return Ok(Outcome::refused(
            "ticket.summary",
            String::new(),
            format!(
                "a summary is one line of at most {} bytes; this one is {}. Say what the work is for, and put the rest in --body",
                openwarrant_core::format::SUMMARY_MAX,
                s.len()
            ),
        ));
    }
    let priority = args.priority.unwrap_or(DEFAULT_PRIORITY);
    if priority > 4 {
        return Ok(Outcome::refused(
            "ticket.priority",
            String::new(),
            format!("priority {priority} is outside 0 (most urgent) ..= 4"),
        ));
    }
    let items: Vec<String> = args
        .items
        .iter()
        .map(|i| ticket::one_line(i))
        .filter(|i| !i.is_empty())
        .collect();
    let labels = label_set(&args.labels);
    if let Some(refusal) = refuse_fields(store, args.kind.as_deref(), &labels) {
        return Ok(refusal);
    }
    let needs_tickets = args.part_of.is_some() || args.issue.is_some();
    let all = if needs_tickets {
        store.load_all()?.0
    } else {
        Vec::new()
    };
    let part_of = match args.part_of.as_deref() {
        None => None,
        Some(q) => match parent_of(&all, q, None) {
            Ok(p) => Some(p),
            Err(refusal) => return Ok(*refusal),
        },
    };
    if let Some(issue) = &args.issue
        && let Some(linked) = all.iter().find(|t| t.manifest.issue == Some(issue.number))
    {
        return Ok(Outcome::refused(
            "ticket.issue-linked",
            store.rel(&linked.dir.join("manifest.toml")),
            format!(
                "issue #{} is already Warrant {} ({}); one issue, one Warrant",
                issue.number,
                linked.id(),
                linked.manifest.title
            ),
        ));
    }
    std::fs::create_dir_all(&store.dir).map_err(io(format!("could not create {}", store.dir)))?;
    let existing = store.ticket_dirs()?.len();
    let uuid = WarUuid::mint().to_string();
    // The id lengthens past a collision on this branch; across branches the
    // length rule keeps the chance of one under 1%.
    let mut dir = None;
    for len in ticket::ticket_id_len(existing)..=16 {
        let candidate = store.dir.join(ticket::ticket_id(&uuid, len));
        match std::fs::create_dir(&candidate) {
            Ok(()) => {
                dir = Some(candidate);
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(source) => {
                return Err(RepoError::Io {
                    context: format!("could not create {candidate}"),
                    source,
                });
            }
        }
    }
    let Some(dir) = dir else {
        return Err(RepoError::Message(
            "ticket.id: every length of this Warrant's id is taken".to_owned(),
        ));
    };
    let id = dir.file_name().unwrap_or_default().to_owned();
    let now = now_secs();
    let checklist_file = format!("atoms/{}", store.checklist_file());
    let manifest = TicketManifest {
        schema: TICKET_SCHEMA.to_owned(),
        id: id.clone(),
        uuid: uuid.clone(),
        title: title.clone(),
        summary,
        profile: TICKET_PROFILE.to_owned(),
        priority,
        created_at: rfc3339(now),
        created_by: store.actor.clone(),
        promoted_to: None,
        kind: args.kind.clone(),
        labels: labels.clone(),
        part_of: part_of.clone(),
        issue: args.issue.as_ref().map(|i| i.number),
        issue_url: args
            .issue
            .as_ref()
            .map(|i| i.url.clone())
            .filter(|u| !u.is_empty()),
        imported_from: None,
        due: None,
        atoms: vec![
            TicketAtom {
                ordinal: 10,
                role: "intent".to_owned(),
                path: INTENT_FILE.to_owned(),
            },
            TicketAtom {
                ordinal: store.checklist_ordinal(),
                role: CHECKLIST_ROLE.to_owned(),
                path: checklist_file.clone(),
            },
        ],
    };
    let mut intent = format!("# {title}\n");
    if let Some(body) = args
        .body
        .as_deref()
        .map(str::trim)
        .filter(|b| !b.is_empty())
    {
        intent.push('\n');
        intent.push_str(body);
        intent.push('\n');
    }
    let mut checklist = store.checklist_stub();
    if !checklist.ends_with('\n') {
        checklist.push('\n');
    }
    let mut taken = BTreeSet::new();
    let mut made = Vec::new();
    for text in &items {
        let item_id = fresh_item_id(&taken);
        taken.insert(item_id.clone());
        checklist.push_str(&Item::new(&item_id, text, Vec::new()).render());
        checklist.push('\n');
        made.push(serde_json::json!({"id": item_id, "text": text}));
    }
    std::fs::create_dir_all(dir.join("atoms"))
        .map_err(io(format!("could not create {dir}/atoms")))?;
    atomic::write(&dir.join(INTENT_FILE), intent)?;
    atomic::write(&dir.join(&checklist_file), checklist)?;
    atomic::write(&dir.join("manifest.toml"), toml_of(&manifest)?)?;
    let t = store
        .load(&dir)
        .map_err(|d| RepoError::Message(format!("{}: {}", d.rule, d.message)))?;
    let mut payload = serde_json::json!({"ticket": id, "title": title, "items": made.len()});
    if let Some(issue) = &args.issue {
        // Where it came from: the issue's number, address and a digest of
        // its body, never the body as evidence of anything (§74.8).
        payload["intake"] = serde_json::to_value(issue.record()).unwrap_or_default();
    }
    store.journal(&t, event::CREATED, &payload)?;
    let rel = store.rel(&dir);
    let mut human = format!("{id}  {title}\ncreated {rel}/");
    if let Some(issue) = &args.issue {
        human.push_str(&format!(
            "\nfrom GitHub issue #{}{}",
            issue.number,
            if issue.url.is_empty() {
                String::new()
            } else {
                format!(" ({})", issue.url)
            }
        ));
    }
    if let Some(p) = &part_of {
        human.push_str(&format!("\npart of {p}"));
    }
    if made.is_empty() {
        human.push_str(&format!(
            "\nno items yet: `war add {id} \"...\"`, or `war claim {id}` and work it whole"
        ));
    } else {
        human.push_str(&format!("\n{} item(s); `war next` lists them", made.len()));
    }
    // OW-WAR-0148 M13 (decision 2): one line, never a refusal, suggesting
    // the test a new Warrant does not have yet. `[warrants] hints = false`
    // turns it off.
    let hint = hints_enabled(&store.root).then(|| {
        format!(
            "hint (optional): `war add {id} --test \"<command>\"` gives it a test, and `war \
             done <item> --check` then ticks only when it passes"
        )
    });
    if let Some(h) = &hint {
        human.push('\n');
        human.push_str(h);
    }
    let mut result = serde_json::json!({
        "schema": "oh.war/ticket-created/v1",
        "id": id,
        "uuid": uuid,
        "title": title,
        "dir": rel,
        "priority": priority,
        "items": made,
    });
    if let Some(k) = &args.kind {
        result["type"] = serde_json::json!(k);
    }
    if !labels.is_empty() {
        result["labels"] = serde_json::json!(labels);
    }
    if let Some(p) = &part_of {
        result["part_of"] = serde_json::json!(p);
    }
    if let Some(issue) = &args.issue {
        result["issue"] = serde_json::json!({"number": issue.number, "url": issue.url});
    }
    if let Some(h) = hint {
        result["hint"] = serde_json::json!(h);
    }
    Ok(Outcome::ok(human, result))
}

/// `[warrants] hints` in `openwarrant.toml` (OW-WAR-0148 M13): whether
/// `war create` prints its one-line hint. On unless set to `false`; a file
/// that cannot be read leaves it on.
#[must_use]
pub fn hints_enabled(root: &Utf8Path) -> bool {
    #[derive(Deserialize)]
    struct Warrants {
        #[serde(default)]
        hints: Option<bool>,
    }
    #[derive(Deserialize)]
    struct File {
        #[serde(default)]
        warrants: Option<Warrants>,
    }
    crate::vfs::read_to_string(root.join(crate::init::CONFIG_FILE))
        .ok()
        .and_then(|text| toml::from_str::<File>(&text).ok())
        .and_then(|f| f.warrants)
        .and_then(|w| w.hints)
        .unwrap_or(true)
}

/// A refusal for `--issue <n>` when a ticket already holds that issue, read
/// before anything is fetched.
pub fn issue_already_linked(store: &Store, n: &str) -> Result<Option<Outcome>, RepoError> {
    let Ok(number) = n.trim().trim_start_matches('#').parse::<u64>() else {
        return Ok(None);
    };
    let (tickets, _) = store.load_all()?;
    Ok(tickets
        .iter()
        .find(|t| t.manifest.issue == Some(number))
        .map(|linked| {
            Outcome::refused(
                "ticket.issue-linked",
                store.rel(&linked.dir.join("manifest.toml")),
                format!(
                    "issue #{number} is already Warrant {} ({}); one issue, one Warrant. Nothing \
                     was fetched",
                    linked.id(),
                    linked.manifest.title
                ),
            )
        }))
}

/// A command-level failure as a refusal by rule: intake's messages open
/// with their rule (`intake.fetch-not-a-read: ...`).
#[must_use]
pub fn refusal_of(e: RepoError) -> Outcome {
    let message = e.to_string();
    let rule = message
        .split_once(": ")
        .map(|(r, _)| r)
        .filter(|r| r.contains('.') && !r.contains(' '))
        .unwrap_or("ticket.create")
        .to_owned();
    Outcome::refused(&rule, crate::init::CONFIG_FILE, message)
}

/// One item per record id: the record's first sentence, then
/// `(implements <id>)`, so the ticket profile's `implements` relation names
/// the record (`war plan impact` finds the item). Refused, by name, for an id no
/// record atom declares. Reads the record atoms under `docs/records/` and
/// nothing else.
pub fn implementing_items(
    repo: &Repository,
    ids: &[String],
) -> Result<Result<Vec<String>, Outcome>, RepoError> {
    if ids.is_empty() {
        return Ok(Ok(Vec::new()));
    }
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for file in crate::records::files(repo) {
        let Ok(text) = crate::vfs::read_to_string(&file) else {
            continue;
        };
        let Ok(atom) = openwarrant_core::record::parse(&text) else {
            continue;
        };
        for r in atom.records {
            if ids.contains(&r.id) && !found.contains_key(&r.id) {
                let span = text.get(r.start..r.end).unwrap_or_default();
                found.insert(r.id.clone(), first_sentence(span));
            }
        }
    }
    let mut items = Vec::new();
    for id in ids {
        let Some(summary) = found.get(id) else {
            return Ok(Err(Outcome::refused(
                "ticket.record-unknown",
                crate::records::DIR,
                format!(
                    "`--implements {id}`: no record atom under {} declares {id}; nothing was \
                     created",
                    crate::records::DIR
                ),
            )));
        };
        let text = if summary.is_empty() {
            format!("Implement {id}")
        } else {
            summary.trim_end_matches('.').to_owned()
        };
        items.push(format!("{text} (implements {id})"));
    }
    Ok(Ok(items))
}

/// The first sentence of a record's prose: its span without the heading,
/// relation lines and blank lines, up to the first `. `.
fn first_sentence(span: &str) -> String {
    let mut prose = Vec::new();
    let mut started = false;
    for line in span.lines().skip(1) {
        let t = line.trim();
        if t.is_empty() {
            if started {
                break;
            }
            continue;
        }
        let first = t.split_whitespace().next().unwrap_or_default();
        let is_relation = !started
            && t.split_whitespace().count() >= 2
            && first
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_' || c == '.')
            && t.split_whitespace().nth(1).is_some_and(|w| {
                w.chars().next().is_some_and(|c| c.is_ascii_uppercase()) && w.contains('-')
            });
        if is_relation {
            continue;
        }
        started = true;
        prose.push(t);
    }
    let joined = ticket::one_line(&prose.join(" "));
    match joined.find(". ") {
        Some(at) => joined[..=at].to_owned(),
        None => joined,
    }
}

/// The items a configured drafter proposes for `sentence`, or a refusal. The
/// drafter answers `war plan`'s request; its proposal's deliverables (or its
/// stages) become the items. Nothing is invented when it proposes none, or
/// when it asks a question instead.
pub fn drafted_items(
    repo: &Repository,
    sentence: &str,
) -> Result<Result<Vec<String>, Outcome>, RepoError> {
    if repo.config.plan.drafter_argv.is_empty() {
        return Ok(Err(Outcome::refused(
            "ticket.no-drafter",
            crate::init::CONFIG_FILE,
            "`--draft` asks the configured drafter, and `[plan] drafter_argv` is not set; \
             nothing was invented. Give the items with `--item`, or add them later with \
             `war add`",
        )));
    }
    let request = crate::plan::request(
        repo,
        sentence,
        "delivery",
        "basic",
        &std::collections::BTreeMap::new(),
    )?;
    let (out, _run) = crate::plan::run_drafter(repo, &request)?;
    let proposal: serde_json::Value = serde_json::from_str(out.trim()).map_err(|e| {
        RepoError::Message(format!(
            "ticket.draft: the drafter's answer is not JSON: {e}"
        ))
    })?;
    if let Some(questions) = proposal["unresolved_questions"].as_array()
        && !questions.is_empty()
    {
        let asked: Vec<String> = questions
            .iter()
            .filter_map(|q| q["question"].as_str().map(str::to_owned))
            .collect();
        return Ok(Err(Outcome::refused(
            "ticket.draft-question",
            String::new(),
            format!(
                "the drafter asked instead of proposing: {}. Nothing was created; answer it in \
                 the title or the items",
                asked.join(" / ")
            ),
        )));
    }
    let bodies = |role: &str| -> Vec<String> {
        proposal["operations"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|op| op["role"].as_str() == Some(role))
            .filter_map(|op| op["body"].as_str().map(str::to_owned))
            .collect()
    };
    let mut items = Vec::new();
    for body in bodies("work_order") {
        let mut inside = false;
        for line in body.lines() {
            if line.starts_with("## ") {
                inside = line.trim() == "## Deliverables";
                continue;
            }
            if !inside {
                continue;
            }
            let t = line.trim_start();
            let rest = t
                .split_once(". ")
                .filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
                .map(|(_, r)| r)
                .or_else(|| t.strip_prefix("- "));
            if let Some(rest) = rest.map(ticket::one_line).filter(|r| !r.is_empty()) {
                items.push(rest);
            }
        }
    }
    if items.is_empty() {
        for body in bodies("milestones") {
            let mut in_stages = false;
            for line in body.lines() {
                if !line.starts_with(' ') && !line.starts_with('-') {
                    in_stages = line.trim_end() == "stages:";
                    continue;
                }
                if in_stages
                    && let Some(title) = line
                        .trim_start()
                        .trim_start_matches("- ")
                        .strip_prefix("title:")
                {
                    let title = ticket::one_line(title.trim().trim_matches('"'));
                    if !title.is_empty() {
                        items.push(title);
                    }
                }
            }
        }
    }
    if items.is_empty() {
        return Ok(Err(Outcome::refused(
            "ticket.draft-empty",
            String::new(),
            "the drafter proposed no items; nothing was created and nothing was invented",
        )));
    }
    Ok(Ok(items))
}

// ---- ready -----------------------------------------------------------------

/// One row of `war view ready`.
#[derive(Debug, Clone, Serialize)]
pub struct ReadyRow {
    pub ticket: String,
    pub title: String,
    pub priority: u8,
    /// `None`: the ticket has no items yet and is itself the work, or the
    /// item is a line a human added without an id (see `line`).
    pub item: Option<String>,
    pub text: String,
    /// 1-based line of an unnamed item in the checklist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// A claim past its TTL: `war claim --steal` may take it. Boxed, as is
    /// `expired_claim`: a ready set holds a row per open item, and almost
    /// none carries a claim.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_claim: Option<Box<claim::Claim>>,
    /// M11: a claim whose lease ran out: a plain `war claim` takes it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expired_claim: Option<Box<claim::Claim>>,
    /// FORMAT-1.0 §11.5: the earliest `due` of the row and of everything
    /// that waits on it, by which it is ordered first.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    /// FORMAT-1.0 §11.5: its estimate and the longest remaining path of
    /// what waits on it, in seconds: the critical path is first. Absent
    /// when there is one candidate, which needs no ranking (and no journal
    /// is read for it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_secs: Option<u64>,
    #[serde(skip)]
    created_at: String,
    #[serde(skip)]
    order: usize,
}

impl ReadyRow {
    /// `t-x/i-y`, or `t-x` for a whole ticket or an unnamed item.
    #[must_use]
    pub fn target(&self) -> String {
        match &self.item {
            Some(i) => format!("{}/{i}", self.ticket),
            None => self.ticket.clone(),
        }
    }
}

/// The ready set, FORMAT-1.0 §11.5's candidates: open, unblocked items no
/// live claim holds (a claim whose lease ran out holds nothing), and tickets
/// with no items that nobody holds, in §11.5's order
/// ([`crate::estimate::OrderKey`]).
pub fn ready_rows(store: &Store, tickets: &[Ticket]) -> Result<Vec<ReadyRow>, RepoError> {
    ready_rows_with(store, tickets, store.claims()?)
}

/// Every claim now held, by lock file name ([`Store::claims`]).
pub type Claims = BTreeMap<String, Option<claim::Claim>>;

/// `f`, with [`Store::claims`] read meanwhile: on the disk on a thread of
/// its own, elsewhere after `f`. The two are separate reads, and a caller
/// takes `f`'s error before the claims', as when they were read in turn.
pub fn with_claims<T>(store: &Store, f: impl FnOnce() -> T) -> (T, Result<Claims, RepoError>) {
    if !crate::vfs::is_disk() {
        let t = f();
        return (t, store.claims());
    }
    std::thread::scope(|scope| {
        let reading = std::thread::Builder::new()
            .spawn_scoped(scope, || store.claims())
            .ok();
        let t = f();
        let claims = match reading.map(|h| h.join()) {
            Some(Ok(claims)) => claims,
            Some(Err(panic)) => std::panic::resume_unwind(panic),
            None => store.claims(),
        };
        (t, claims)
    })
}

/// [`ready_rows`] over claims already read.
pub fn ready_rows_with(
    store: &Store,
    tickets: &[Ticket],
    claims: Claims,
) -> Result<Vec<ReadyRow>, RepoError> {
    ready_rows_at(store, tickets, claims, now_secs())
}

/// [`ready_rows`] over claims already read, as of `now`: a pure function of
/// the tickets (documents and journals), the claims and the time
/// (FORMAT-1.0 §11.5), so callers asking at one time get one answer.
pub fn ready_rows_at(
    store: &Store,
    tickets: &[Ticket],
    claims: Claims,
    now: u64,
) -> Result<Vec<ReadyRow>, RepoError> {
    // A claim leaves its item in the ready set only when its lease ran out
    // (a plain claim takes it): a live claim holds its item out (§11.5),
    // however old (a claim past the TTL is taken with `war claim --steal`,
    // which `war show` and the refusal name).
    type Takeable = (Option<claim::Claim>, Option<claim::Claim>);
    let stale = |c: Option<&claim::Claim>| -> Result<Takeable, ()> {
        match c {
            Some(c) if c.lease_expired(now) => Ok((None, Some(c.clone()))),
            _ => Err(()),
        }
    };
    // The epics: every ticket some other names `part_of`, read once rather
    // than one scan of every ticket per ticket without items.
    let parents: BTreeSet<&str> = tickets
        .iter()
        .filter_map(|x| x.manifest.part_of.as_deref())
        .collect();
    let by_id = first_by_id(tickets);
    let find = |id: &str| by_id.get(id).copied();
    // Each ticket's rows depend on it, the claims and the blockers alone:
    // on the disk they are made across threads, kept in ticket order.
    let rows_of = |t: &Ticket| -> Vec<ReadyRow> {
        let mut rows = Vec::new();
        if t.is_done() {
            return rows;
        }
        let whole = claim_on(&claims, t.id(), None);
        let whole_stale: Takeable = match whole {
            None => (None, None),
            Some((_, c)) => match stale(c) {
                Ok(s) => s,
                Err(()) => return rows,
            },
        };
        let row = |item: Option<String>,
                   text: String,
                   line: Option<usize>,
                   order: usize,
                   (stale_claim, expired_claim): Takeable| ReadyRow {
            ticket: t.id().to_owned(),
            title: t.manifest.title.clone(),
            priority: t.manifest.priority,
            item,
            text,
            line,
            stale_claim: stale_claim.map(Box::new),
            expired_claim: expired_claim.map(Box::new),
            due: None,
            remaining_secs: None,
            created_at: t.manifest.created_at.clone(),
            order,
        };
        if t.checklist.items.is_empty() {
            // An epic with tickets of its own and no items is worked through
            // its tickets, never whole (OW-WAR-0148 M5); a document whose
            // `depends_on` is not done waits (FORMAT-1.0 §11.1).
            if !parents.contains(t.id()) && document_blockers_by(&find, t).is_empty() {
                rows.push(row(None, t.manifest.title.clone(), None, 0, whole_stale));
            }
            return rows;
        }
        for (order, item) in t.checklist.items.iter().enumerate() {
            if item.done || !open_blockers_by(&find, t, item).is_empty() {
                continue;
            }
            let mut stale_claim = whole_stale.clone();
            if let Some(id) = &item.id
                && let Some((_, c)) = claim_on(&claims, t.id(), Some(id))
            {
                match stale(c) {
                    Ok(s) => stale_claim = s,
                    Err(()) => continue,
                }
            }
            let line = item.id.is_none().then_some(item.line + 1);
            rows.push(row(
                item.id.clone(),
                item.text.clone(),
                line,
                order,
                stale_claim,
            ));
        }
        rows
    };
    let mut rows: Vec<ReadyRow> = crate::vfs::map_ordered(tickets, 512, rows_of)
        .into_iter()
        .flatten()
        .collect();
    rank_rows(store, tickets, &claims, now, &mut rows);
    // FORMAT-1.0 §11.5's order. The keys are sorted, not the rows, and each
    // row then moves once. A row's index breaks every tie, so this unstable
    // sort orders the rows exactly as a stable sort of the rows on the keys
    // alone.
    let mut by: Vec<(crate::estimate::OrderKey<'_>, usize)> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            (
                crate::estimate::order_key_of(
                    r.due.as_deref(),
                    r.remaining_secs.unwrap_or(0),
                    r.priority,
                    r.created_at.as_str(),
                    r.ticket.as_str(),
                    r.order,
                ),
                i,
            )
        })
        .collect();
    by.sort_unstable();
    let by: Vec<usize> = by.into_iter().map(|k| k.1).collect();
    let mut slots: Vec<Option<ReadyRow>> = rows.into_iter().map(Some).collect();
    Ok(by
        .into_iter()
        .filter_map(|i| slots.get_mut(i).and_then(Option::take))
        .collect())
}

/// Each candidate's effective `due` and remaining path (FORMAT-1.0 §11.5),
/// over the graph of the tickets ([`crate::graph::of_tickets`]) with the
/// estimates their journals teach ([`crate::estimate::learn`]): the same
/// computation `war evidence go` orders its nodes by. One candidate needs
/// no ranking, and no journal is read for it.
fn rank_rows(store: &Store, tickets: &[Ticket], claims: &Claims, now: u64, rows: &mut [ReadyRow]) {
    if rows.is_empty() {
        return;
    }
    let graph = crate::graph::of_tickets(tickets, claims, now);
    // The due dates need no history; the remaining paths do, and only when
    // there is more than one candidate to order. Only a ticket with
    // something done can hold a sample, so only those journals are read.
    let ranked = rows.len() > 1;
    let prior = crate::estimate::prior_secs(&store.root, None);
    let history = if ranked {
        // An unreadable journal teaches nothing; the estimate falls back as
        // §11.5 says, and `war plan estimate` names the journal.
        crate::estimate::learn_from(
            tickets
                .iter()
                .filter(|t| t.checklist.items.iter().any(|i| i.done)),
            prior,
            &mut Report::default(),
        )
    } else {
        crate::estimate::History {
            prior_secs: prior,
            ..crate::estimate::History::default()
        }
    };
    let ranks = crate::estimate::ranks(&graph, &history);
    let by_id = first_by_id(tickets);
    for r in rows.iter_mut() {
        let node = match &r.item {
            Some(i) => format!("{}/{i}", r.ticket),
            None => r.ticket.clone(),
        };
        let (remaining, due) = if let Some(rem) = ranks.remaining.get(node.as_str()) {
            (*rem, ranks.due.get(node.as_str()).cloned().flatten())
        } else {
            // An item with no id yet is no node: its own estimate, and what
            // waits on its Warrant (which waits on it).
            let t = by_id.get(r.ticket.as_str());
            let own = history
                .estimate(
                    t.and_then(|t| t.manifest.kind.as_deref()),
                    t.map_or(&[][..], |t| t.manifest.labels.as_slice()),
                )
                .seconds;
            (
                own + ranks.remaining.get(r.ticket.as_str()).copied().unwrap_or(0),
                ranks
                    .due
                    .get(r.ticket.as_str())
                    .cloned()
                    .flatten()
                    .or_else(|| t.and_then(|t| t.manifest.due.clone())),
            )
        };
        r.due = due;
        r.remaining_secs = ranked.then_some(remaining);
    }
}

/// The ready set as text: one line per Warrant, in the order its first ready
/// row has, with how many of its items can start and the command that takes
/// it. Item text is `war start`'s to print, for the one Warrant taken.
#[must_use]
pub fn ready_lines(rows: &[ReadyRow]) -> String {
    let mut order: Vec<&str> = Vec::new();
    let mut by: BTreeMap<&str, Vec<&ReadyRow>> = BTreeMap::new();
    for r in rows {
        if !by.contains_key(r.ticket.as_str()) {
            order.push(&r.ticket);
        }
        by.entry(&r.ticket).or_default().push(r);
    }
    let mut out = String::new();
    for id in order {
        let rs = &by[id];
        let first = rs[0];
        let what = if rs.len() == 1 && first.item.is_none() && first.line.is_none() {
            "the Warrant is the work".to_owned()
        } else {
            format!("{} ready", rs.len())
        };
        out.push_str(&format!(
            "{id:<10} p{}  {}  ({what})  `war start {id}`",
            first.priority, first.title
        ));
        if let Some(c) = rs.iter().find_map(|r| r.expired_claim.as_ref()) {
            out.push_str(&format!("  [lease ran out: {} held it]", c.actor));
        } else if let Some(c) = rs.iter().find_map(|r| r.stale_claim.as_ref()) {
            out.push_str(&format!(
                "  [stale claim: {}; `war claim --steal`]",
                c.actor
            ));
        }
        out.push('\n');
    }
    out
}

/// `war view ready`.
pub fn ready(store: &Store) -> Result<Outcome, RepoError> {
    ready_for(store, None)
}

/// [`ready`] for the one output `mode` that will print it: only that form
/// is built, and the other is left empty (an empty text, or JSON `null`).
/// `None` builds both, as [`ready`].
pub fn ready_for(store: &Store, mode: Option<crate::output::Mode>) -> Result<Outcome, RepoError> {
    let human_too = !matches!(mode, Some(crate::output::Mode::Json));
    let json_too = !matches!(mode, Some(crate::output::Mode::Human));
    // The claims are read while the tickets are (`with_claims`).
    let (loaded, claims) = with_claims(store, || store.load_all());
    let (tickets, faults) = loaded?;
    let rows = ready_rows_with(store, &tickets, claims?)?;
    let mut human = String::new();
    if human_too && rows.is_empty() {
        // M9: the same plain first line `war next` prints; ordinary work
        // goes on either way.
        if tickets.iter().any(|t| !t.is_done()) {
            human.push_str(
                "nothing tracked is ready; work freely. Every open item is claimed or \
                 waiting (`war view warrants`)",
            );
        } else {
            human.push_str(
                "nothing tracked; work freely. To track work (optional): `war create \
                 \"what this work does\"`",
            );
        }
    }
    if human_too {
        human.push_str(&ready_lines(&rows));
    }
    let result = if json_too {
        serde_json::json!({"schema": "oh.war/ticket-ready/v1", "ready": &rows})
    } else {
        serde_json::Value::Null
    };
    let mut out = Outcome::ok(human.trim_end().to_owned(), result);
    for f in faults {
        out.report.push(Diagnostic::warn(
            f.rule.clone(),
            f.file.clone().unwrap_or_default(),
            format!("skipped: {}", f.message),
        ));
    }
    crate::vfs::drop_later((tickets, rows));
    Ok(out)
}

// ---- claim / release -------------------------------------------------------

fn new_claim(store: &Store, ticket: &str, item: Option<&str>, now: u64) -> claim::Claim {
    claim::Claim {
        schema: claim::CLAIM_SCHEMA.to_owned(),
        claim: WarUuid::mint().to_string(),
        ticket: ticket.to_owned(),
        item: item.map(str::to_owned),
        actor: store.actor.clone(),
        since: rfc3339(now),
        since_unix: now,
        lease_until: Some(rfc3339(now + store.lease_secs)),
        lease_until_unix: Some(now + store.lease_secs),
    }
}

/// `war claim <item|ticket> [--steal]`.
pub fn claim_cmd(store: &Store, query: &str, steal: bool) -> Result<Outcome, RepoError> {
    let (mut tickets, _) = store.load_for(query)?;
    let target = match resolve(&tickets, query) {
        Ok(t) => t,
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let index = match &target {
        Target::Ticket(n) | Target::Item(n, _) => *n,
    };
    // Name any line a human added without an id, so every item can be
    // claimed by name from here on. Only those lines change.
    let t = &tickets[index];
    let mut named_now = Vec::new();
    if t.checklist.items.iter().any(|i| i.id.is_none()) {
        named_now = match rewrite(&t.checklist_path, |text| Ok(named_in(t, text)))? {
            Ok(n) => n,
            Err(refusal) => return Ok(*refusal),
        };
        let reloaded = store
            .load(&t.dir)
            .map_err(|d| RepoError::Message(format!("{}: {}", d.rule, d.message)))?;
        tickets[index] = reloaded;
    }
    let t = &tickets[index];
    let now = now_secs();
    let claims = store.claims()?;
    let item = match &target {
        Target::Item(_, id) => Some(id.clone()),
        Target::Ticket(_) => None,
    };
    let what = match &item {
        Some(i) => format!("{}/{i}", t.id()),
        None => t.id().to_owned(),
    };
    if let Some(id) = &item {
        let it = t.item(id).expect("resolved");
        if it.done {
            return Ok(Outcome::refused(
                "ticket.already-done",
                store.rel(&t.checklist_path),
                format!(
                    "{what} is already done{}; nothing to claim",
                    it.done_by
                        .as_ref()
                        .map(|b| format!(" (by {b})"))
                        .unwrap_or_default()
                ),
            ));
        }
        let waiting = open_blockers(&tickets, t, it);
        if !waiting.is_empty() {
            return Ok(Outcome::refused(
                "ticket.blocked",
                store.rel(&t.checklist_path),
                format!(
                    "{what} waits on {}; `war next` lists what can start now",
                    waiting.join(", ")
                ),
            ));
        }
        // The whole ticket held by someone else holds this item too, while
        // its lease is live (an expired claim holds nothing, §11.3).
        if let Some((_, c)) = claim_on(&claims, t.id(), None)
            && c.is_none_or(|c| c.actor != store.actor && !c.lease_expired(now))
        {
            return Ok(claimed_by_other(
                store.rel(&store.lock_path(t.id(), None)),
                format!(
                    "{what}: the whole Warrant {} is claimed by {}",
                    t.id(),
                    holder(c, now)
                ),
            ));
        }
    } else {
        if t.is_done() {
            return Ok(Outcome::refused(
                "ticket.already-done",
                store.rel(&t.checklist_path),
                format!("{what} is done: every item is ticked"),
            ));
        }
        let by_id = first_by_id(&tickets);
        let waiting = document_blockers_by(&|id| by_id.get(id).copied(), t);
        if !waiting.is_empty() {
            return Ok(Outcome::refused(
                "ticket.blocked",
                store.rel(&t.checklist_path),
                format!(
                    "{what} waits on {}; `war next` lists what can start now",
                    waiting.join(", ")
                ),
            ));
        }
        // An item of it held by someone else, under a live lease: the ticket
        // is not free whole (§11.3). The refusal names the first holder's
        // claim file.
        let others: Vec<(&String, String)> = claims
            .iter()
            .filter(|(k, _)| k.starts_with(&format!("{}--", t.id())))
            .filter(|(_, c)| {
                c.as_ref()
                    .is_none_or(|c| c.actor != store.actor && !c.lease_expired(now))
            })
            .map(|(k, c)| {
                let who = c.as_ref().map_or_else(
                    || "an item (unreadable lock)".to_owned(),
                    |c| format!("{} by {}", c.target(), holder(Some(c), now)),
                );
                (k, who)
            })
            .collect();
        if let Some((first, _)) = others.first() {
            let file = store.lock_of(
                t.id(),
                first
                    .strip_prefix(&format!("{}--", t.id()))
                    .and_then(|i| i.strip_suffix(".lock")),
            );
            let who: Vec<&str> = others.iter().map(|(_, w)| w.as_str()).collect();
            return Ok(claimed_by_other(
                store.rel(&file),
                format!("{what} cannot be claimed whole: {}", who.join("; ")),
            ));
        }
    }
    let path = store.lock_path(t.id(), item.as_deref());
    let mine = new_claim(store, t.id(), item.as_deref(), now);
    // A claim from before claims were shared (M11) holds where it lies.
    let held_at = store.lock_of(t.id(), item.as_deref());
    let taken = if held_at == path {
        claim::take(&path, &mine).map_err(io(format!("could not claim {path}")))?
    } else {
        claim::Taken::Held(
            store
                .read_claim(&held_at)
                .map_err(io(format!("could not read {held_at}")))?
                .flatten(),
        )
    };
    // A claim whose lease ran out is reclaimed by a plain claim (M11); one
    // past the TTL with its lease live is taken only with --steal.
    let mut reclaimed_from: Option<claim::Claim> = None;
    let stolen_from = match taken {
        claim::Taken::Won => None,
        claim::Taken::Held(Some(c)) if c.actor == store.actor => {
            let c = store
                .renew_held(Some(&claim::lock_name(t.id(), item.as_deref())))
                .pop()
                .unwrap_or(c);
            return Ok(Outcome::ok(
                format!("{what} is already yours (since {})", c.since),
                serde_json::json!({"schema": "oh.war/ticket-claim/v1", "target": what, "claim": c, "already_held": true}),
            ));
        }
        claim::Taken::Held(Some(c)) if c.lease_expired(now) => {
            match claim::steal_into(&held_at, Some(&c), &path, &mine, store.lease_secs, &|m| {
                m.lease_expired(now)
            })
            .map_err(io(format!("could not reclaim {held_at}")))?
            {
                claim::Stolen::Won { .. } => {
                    reclaimed_from = Some(c);
                    None
                }
                claim::Stolen::Lost(other) => {
                    return Ok(claimed_by_other(
                        store.rel(&held_at),
                        format!(
                            "{what} was taken first by {}; pick another (`war next`)",
                            holder(other.as_ref(), now)
                        ),
                    ));
                }
            }
        }
        claim::Taken::Held(c) => {
            let is_stale = c.as_ref().is_some_and(|c| c.age(now) > store.ttl_secs);
            if !steal || !is_stale {
                let hint = if is_stale {
                    format!(
                        ". The claim is past its {} TTL: `war claim {what} --steal` takes it",
                        render::ago(store.ttl_secs)
                    )
                } else if steal {
                    format!(
                        ". It is not stale: a claim can be stolen after {} ({} left)",
                        render::ago(store.ttl_secs),
                        render::ago(
                            store
                                .ttl_secs
                                .saturating_sub(c.as_ref().map_or(0, |c| c.age(now)))
                        )
                    )
                } else {
                    c.as_ref().map_or_else(String::new, |c| {
                        format!(
                            ". Its lease runs out in {} unless {} renews it; then `war claim \
                             {what}` takes it",
                            render::ago(c.lease_left(now)),
                            c.actor
                        )
                    })
                };
                return Ok(claimed_by_other(
                    store.rel(&held_at),
                    format!("{what} is claimed by {}{hint}", holder(c.as_ref(), now)),
                ));
            }
            let judged = c.clone();
            match claim::steal_into(&held_at, c.as_ref(), &path, &mine, store.lease_secs, &|m| {
                judged.as_ref().is_some_and(|j| j.claim == m.claim)
            })
            .map_err(io(format!("could not steal {held_at}")))?
            {
                claim::Stolen::Won { from } => Some(from),
                claim::Stolen::Lost(other) => {
                    return Ok(claimed_by_other(
                        store.rel(&path),
                        format!("{what} was taken first by {}", holder(other.as_ref(), now)),
                    ));
                }
            }
        }
    };
    // Taken. Re-check the other level: a whole-ticket claim and an item claim
    // made at the same instant must not both stand.
    let after = store.claims()?;
    let conflict = match &item {
        Some(_) => claim_on(&after, t.id(), None)
            .filter(|(_, c)| c.is_none_or(|c| c.actor != store.actor && !c.lease_expired(now)))
            .map(|(_, c)| holder(c, now)),
        None => after
            .iter()
            .filter(|(k, _)| k.starts_with(&format!("{}--", t.id())))
            .find(|(_, c)| {
                c.as_ref()
                    .is_none_or(|c| c.actor != store.actor && !c.lease_expired(now))
            })
            .map(|(_, c)| holder(c.as_ref(), now)),
    };
    if let Some(other) = conflict {
        let _ = claim::release(&path, &store.actor);
        return Ok(claimed_by_other(
            store.rel(&path),
            format!("{what}: {other} claimed it at the same moment; released, try another"),
        ));
    }
    // M11: with a claims remote, the claim holds only once the remote has it.
    let mut remote_from = None;
    if let Some(remote_name) = store.remote.as_deref() {
        match store.publish_claim(remote_name, &path, &mine, &what, steal, now) {
            Ok(from) => remote_from = from,
            Err(refusal) => {
                let _ = claim::release(&path, &store.actor);
                return Ok(*refusal);
            }
        }
    }
    // A takeover at the remote is journalled as the local one would be.
    let mut stolen_from = stolen_from;
    match remote_from {
        Some((c, Takeover::Reclaimed)) if reclaimed_from.is_none() && stolen_from.is_none() => {
            reclaimed_from = Some(c);
        }
        Some((c, Takeover::Stolen)) if reclaimed_from.is_none() && stolen_from.is_none() => {
            stolen_from = Some(Some(c));
        }
        _ => {}
    }
    let mut payload = serde_json::json!({
        "claim": mine.claim,
        "target": what,
        "since": mine.since,
    });
    // FORMAT-1.0 §11.2: `work.claimed` carries the lease it was taken with.
    if t.format
        && let Some(until) = &mine.lease_until
    {
        payload["lease_until"] = serde_json::json!(until);
    }
    if !named_now.is_empty() {
        payload["named"] = serde_json::json!(named_now);
    }
    if let Some(remote_name) = store.remote.as_deref() {
        payload["remote"] = serde_json::json!(format!(
            "{remote_name}:{}",
            remote::ref_name(path.file_name().unwrap_or_default())
        ));
    }
    let event_type = match (&stolen_from, &reclaimed_from) {
        (Some(from), _) => {
            payload["from"] = serde_json::json!(from.as_ref().map(|c| &c.actor));
            payload["from_since"] = serde_json::json!(from.as_ref().map(|c| &c.since));
            event::CLAIM_STOLEN
        }
        (None, Some(from)) => {
            payload["from"] = serde_json::json!(from.actor);
            payload["from_since"] = serde_json::json!(from.since);
            payload["from_lease_until"] = serde_json::json!(from.lease_until);
            event::CLAIM_RECLAIMED
        }
        (None, None) => event::CLAIMED,
    };
    store.journal(t, event_type, &payload)?;
    let text = match &item {
        Some(id) => t.item(id).map(|i| i.text.clone()).unwrap_or_default(),
        None => t.manifest.title.clone(),
    };
    let mut human = format!("claimed {what}: {text}");
    if !named_now.is_empty() {
        human.push_str(&format!(
            "\nnamed {} line(s) added without an id: {}",
            named_now.len(),
            named_now.join(", ")
        ));
    }
    if let Some(Some(from)) = &stolen_from {
        human.push_str(&format!(
            " (stolen from {}, idle since {})",
            from.actor, from.since
        ));
    }
    if let Some(from) = &reclaimed_from {
        human.push_str(&format!(
            " (reclaimed from {}, whose lease ran out at {})",
            from.actor,
            from.lease_until.as_deref().unwrap_or("?")
        ));
    }
    // A whole ticket is closed in one call, every open item at once.
    human.push_str(&format!(
        "\nwhen it is done: `war done {what} --note \"...\"`"
    ));
    let mut result = serde_json::json!({
        "schema": "oh.war/ticket-claim/v1",
        "target": what,
        "claim": mine,
        "stolen_from": stolen_from.flatten(),
        "named": named_now,
    });
    if let Some(from) = &reclaimed_from {
        result["reclaimed_from"] = serde_json::json!(from);
    }
    Ok(Outcome::ok(human, result))
}

/// The Warrants `store.actor` holds a live claim on (whole or on an item),
/// by id, in id order.
fn held_by_me(store: &Store) -> Result<Vec<String>, RepoError> {
    let now = now_secs();
    let mut ids: Vec<String> = store
        .claims()?
        .values()
        .flatten()
        .filter(|c| c.actor == store.actor && !c.lease_expired(now))
        .map(|c| c.ticket.clone())
        .collect();
    ids.sort();
    ids.dedup();
    Ok(ids)
}

/// What `war done` with no id closes: the one Warrant this actor holds.
/// `Err` is the refusal when it holds none, or more than one.
pub fn held_target(store: &Store) -> Result<Result<String, Outcome>, RepoError> {
    let held = held_by_me(store)?;
    Ok(match held.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(Outcome::refused(
            "ticket.nothing-held",
            String::new(),
            format!(
                "{} holds no Warrant: `war start` takes the next ready one",
                store.actor
            ),
        )),
        many => Err(Outcome::refused(
            "ticket.several-held",
            String::new(),
            format!(
                "{} holds {}; name one: `war done <id>`",
                store.actor,
                many.join(", ")
            ),
        )),
    })
}

/// `war start [<id>]`: claim work and print only what it needs, in one call.
/// With an id, that Warrant (or item). Without one: the Warrant this actor
/// already holds, so an interrupted session resumes where it was; else the
/// first ready Warrant in FORMAT-1.0 §11.5's order that it can claim whole.
/// The answer is [`render::brief`]: no context packet, no other Warrant.
pub fn start_cmd(store: &Store, query: Option<&str>) -> Result<Outcome, RepoError> {
    let chosen = if let Some(q) = query {
        let claimed = claim_cmd(store, q, false)?;
        if claimed.is_refused() {
            return Ok(claimed);
        }
        q.split('/').next().unwrap_or(q).to_owned()
    } else {
        let held = held_by_me(store)?;
        if let Some(first) = held.first() {
            // Renew it, and say it is a resumption.
            let _ = store.renew_held(None);
            first.clone()
        } else {
            let (tickets, _) = store.load_all()?;
            let rows = ready_rows(store, &tickets)?;
            let mut order: Vec<String> = Vec::new();
            for r in &rows {
                if !order.contains(&r.ticket) {
                    order.push(r.ticket.clone());
                }
            }
            let mut taken = None;
            for id in &order {
                let claimed = claim_cmd(store, id, false)?;
                if !claimed.is_refused() {
                    taken = Some(id.clone());
                    break;
                }
            }
            let Some(id) = taken else {
                let idle = if tickets.iter().any(|t| !t.is_done()) {
                    "nothing tracked is ready; work freely. Every open item is claimed or waiting \
                     (`war view prime`)"
                } else {
                    "nothing tracked; work freely. To track work (optional): `war create \"what \
                     this work does\"`"
                };
                return Ok(Outcome::ok(
                    idle,
                    serde_json::json!({"schema": "oh.war/ticket-start/v1", "ticket": null}),
                ));
            };
            id
        }
    };
    let (tickets, _) = store.load_all()?;
    let index = match resolve(&tickets, &chosen) {
        Ok(Target::Ticket(n) | Target::Item(n, _)) => n,
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    Ok(render::brief(
        store,
        &tickets,
        &tickets[index],
        &store.actor,
    ))
}

/// `war admin release <item|ticket>`: give a claim back without finishing.
pub fn release(store: &Store, query: &str, if_rev: Option<&str>) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let (index, item) = match resolve(&tickets, query) {
        Ok(Target::Ticket(n)) => (n, None),
        Ok(Target::Item(n, i)) => (n, Some(i)),
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let t = &tickets[index];
    let path = store.lock_of(t.id(), item.as_deref());
    let what = item
        .as_ref()
        .map_or_else(|| t.id().to_owned(), |i| format!("{}/{i}", t.id()));
    if let Some(refusal) = check_if_rev(store, t, item.as_deref(), &what, if_rev) {
        return Ok(refusal);
    }
    match store
        .read_claim(&path)
        .map_err(io(format!("could not read {path}")))?
    {
        None => Ok(Outcome::refused(
            "ticket.not-claimed",
            store.rel(&path),
            format!("{what} is not claimed; nothing to release"),
        )),
        Some(Some(c)) if c.actor == store.actor => {
            // M11: the remote's ref goes too, if it is still the one this
            // machine published (a lease that ran out there may be someone
            // else's claim now, and stays).
            let warning = store.retire_published(&path);
            claim::release(&path, &store.actor).map_err(io(format!("could not release {path}")))?;
            store.journal(
                t,
                event::RELEASED,
                &serde_json::json!({"claim": c.claim, "target": what}),
            )?;
            let mut out = Outcome::ok(
                format!("released {what}"),
                serde_json::json!({"schema": "oh.war/ticket-release/v1", "target": what, "claim": c}),
            );
            if let Some(w) = warning {
                out.report.push(w);
            }
            Ok(out)
        }
        Some(c) => Ok(claimed_by_other(
            store.rel(&path),
            format!(
                "{what} is claimed by {}; only its holder releases it (a stale claim is taken \
                 with `war claim --steal`)",
                holder(c.as_ref(), now_secs())
            ),
        )),
    }
}

/// [`reopen`] of a Format 1.0 ticket with no items: no line changes; the
/// journal's `work.item_reopened` names the document, and, being later
/// than its `work.item_done`, makes it open again (§11.2).
fn reopen_whole(store: &Store, t: &Ticket, reason: &str) -> Result<Outcome, RepoError> {
    let what = t.id().to_owned();
    let reason = ticket::one_line(reason);
    if reason.is_empty() {
        return Ok(Outcome::refused(
            "ticket.reason-empty",
            String::new(),
            format!("{what}: a reopening says why (`--reason \"...\"`)"),
        ));
    }
    if !t.is_done() {
        return Ok(Outcome::refused(
            "ticket.not-done",
            store.rel(&t.checklist_path),
            format!("{what} is open; nothing to reopen"),
        ));
    }
    let now = now_secs();
    let claims = store.claims()?;
    if let Some((_, c)) = claim_on(&claims, t.id(), None)
        && c.is_none_or(|c| c.actor != store.actor && !c.lease_expired(now))
    {
        return Ok(claimed_by_other(
            store.rel(&store.lock_of(t.id(), None)),
            format!(
                "{what} is claimed by {}; only its holder reopens it",
                holder(c, now)
            ),
        ));
    }
    // The done it reopens, by event id: a second reopening of a later done
    // is a new fact, not a replay of the first (§11.2's key has no time).
    let journal = crate::journal_cmd::load(&t.dir)?;
    let reopens = journal
        .events
        .iter()
        .filter(|e| {
            e.event_type == event::ITEM_DONE
                && serde_json::from_str::<serde_json::Value>(&e.payload)
                    .ok()
                    .is_some_and(|p| p.get("target").and_then(|v| v.as_str()) == Some(&what))
        })
        .reduce(|a, b| crate::journal_cmd::later(a, b))
        .map(|e| e.id.clone());
    store.journal(
        t,
        event::ITEM_REOPENED,
        &serde_json::json!({"item": what, "target": what, "reason": reason, "reopens": reopens}),
    )?;
    Ok(Outcome::ok(
        format!("reopened {what}: {reason}"),
        serde_json::json!({"schema": "oh.war/ticket-reopen/v1", "ticket": what, "item": what, "target": what, "reason": reason}),
    ))
}

/// `war admin reopen <item> --reason`: untick a done item (FORMAT-1.0
/// §11.2, `work.item_reopened`). Its box opens again and its done suffix
/// goes; every other byte of the file stays. The write names the revision
/// it read (`--if-rev`, §11.3): a stale one writes nothing. An item someone
/// else holds a claim on is theirs to reopen.
pub fn reopen(
    store: &Store,
    query: &str,
    reason: &str,
    if_rev: Option<&str>,
) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let (index, id) = match resolve(&tickets, query) {
        Ok(Target::Item(n, i)) => (n, i),
        // FORMAT-1.0 §11.1: a Format 1.0 ticket with no items was done as
        // the document itself, and is reopened so.
        Ok(Target::Ticket(n)) if tickets[n].format && tickets[n].checklist.items.is_empty() => {
            return reopen_whole(store, &tickets[n], reason);
        }
        Ok(Target::Ticket(_)) => {
            return Ok(Outcome::refused(
                "ticket.reopen-item",
                String::new(),
                format!(
                    "`war admin reopen {query}` names a Warrant; reopen one of its items \
                     (`<ticket>/<item>`)"
                ),
            ));
        }
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let t = &tickets[index];
    let what = format!("{}/{id}", t.id());
    let reason = ticket::one_line(reason);
    if reason.is_empty() {
        return Ok(Outcome::refused(
            "ticket.reason-empty",
            String::new(),
            format!("{what}: a reopening says why (`--reason \"...\"`)"),
        ));
    }
    if let Some(refusal) = check_if_rev(store, t, Some(&id), &what, if_rev) {
        return Ok(refusal);
    }
    let Some(it) = t.item(&id) else {
        return Ok(Outcome::from_diagnostic(Diagnostic::error(
            "ticket.unknown",
            String::new(),
            format!("no item is {what}"),
        )));
    };
    if !it.done {
        return Ok(Outcome::refused(
            "ticket.not-done",
            store.rel(&t.checklist_path),
            format!("{what} is open; nothing to reopen"),
        ));
    }
    let now = now_secs();
    let claims = store.claims()?;
    for level in [Some(id.as_str()), None] {
        if let Some((_, c)) = claim_on(&claims, t.id(), level)
            && c.is_none_or(|c| c.actor != store.actor && !c.lease_expired(now))
        {
            let whose = if level.is_some() {
                "it"
            } else {
                "the whole Warrant"
            };
            return Ok(claimed_by_other(
                store.rel(&store.lock_of(t.id(), level)),
                format!(
                    "{what}: {whose} is claimed by {}; only its holder reopens it",
                    holder(c, now)
                ),
            ));
        }
    }
    let written = rewrite(&t.checklist_path, |text| {
        let c = checklist_in(t, text);
        let Some(it) = c.item(&id) else {
            return Err(Box::new(Outcome::refused(
                "ticket.unknown",
                String::new(),
                format!(
                    "{id} is no longer in the checklist (edited meanwhile); nothing was written"
                ),
            )));
        };
        if let Some(given) = if_rev {
            let now = item_revision(text, it);
            if !same_revision(given, &now) {
                return Err(Box::new(stale_revision(
                    store,
                    t,
                    &what,
                    &t.checklist_path,
                    given,
                    &now,
                )));
            }
        }
        if !it.done {
            return Err(Box::new(Outcome::refused(
                "ticket.not-done",
                store.rel(&t.checklist_path),
                format!("{what} was reopened meanwhile; nothing was written"),
            )));
        }
        let line = text
            .split_inclusive('\n')
            .nth(it.line)
            .unwrap_or_default()
            .trim_end_matches(['\n', '\r']);
        let open = format::reopened_line(line);
        Ok((ticket::replace_line(text, it.line, &open), open))
    })?;
    let line = match written {
        Ok(line) => line,
        Err(refusal) => return Ok(*refusal),
    };
    store.journal(
        t,
        event::ITEM_REOPENED,
        &serde_json::json!({"item": id, "target": what, "reason": reason}),
    )?;
    Ok(Outcome::ok(
        format!("reopened {what}: {reason}\n{line}"),
        serde_json::json!({"schema": "oh.war/ticket-reopen/v1", "ticket": t.id(), "item": id, "target": what, "reason": reason}),
    ))
}

// ---- claims across machines (M11) -------------------------------------------

/// How a claim published to the remote came to be this agent's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Takeover {
    /// The remote held a claim whose lease had run out.
    Reclaimed,
    /// The remote held a claim past the TTL, and `--steal` was given.
    Stolen,
    /// The remote held this agent's own earlier claim.
    Own,
}

/// The refusal when the claims remote cannot be read or written.
/// The claims remote could not be reached: `UNKNOWN`
/// (`work.claim-remote-unreachable`, FORMAT-1.0 §11.3), never a granted
/// claim.
fn remote_unreachable(store: &Store, remote: &str, what: &str, why: &str, doing: &str) -> Outcome {
    let rule = crate::work_rules::CLAIM_REMOTE_UNREACHABLE;
    Outcome::unknown(
        rule,
        store.rel(&store.root.join(crate::init::CONFIG_FILE)),
        format!(
            "{what}: the claims remote `{remote}` could not be reached ({why}), so {doing}. \
             With [claims] remote set in {} a claim holds only once that remote has it; retry \
             when `git push {remote}` works, or remove [claims] remote to claim on this \
             machine alone{}",
            crate::init::CONFIG_FILE,
            crate::work_rules::formerly(rule)
        ),
    )
}

impl Store {
    /// Publish `mine`, just taken locally at `lock`, to the claims remote.
    /// `Ok(None)`: published. `Ok(Some((from, how)))`: published over `from`'s
    /// claim there. `Err`: not published, refused by name; the caller gives
    /// its local lock back.
    fn publish_claim(
        &self,
        remote_name: &str,
        lock: &Utf8Path,
        mine: &claim::Claim,
        what: &str,
        steal: bool,
        now: u64,
    ) -> Result<Option<(claim::Claim, Takeover)>, Box<Outcome>> {
        let name = lock.file_name().unwrap_or_default();
        let refname = remote::ref_name(name);
        let commit = remote::commit_for(&self.root, mine).map_err(|e| {
            Box::new(remote_unreachable(
                self,
                remote_name,
                what,
                &e,
                "nothing was claimed",
            ))
        })?;
        let mut expect: Option<String> = None;
        let mut takeover = None;
        let mut last_holder = None;
        for _ in 0..4 {
            match remote::push(
                &self.root,
                remote_name,
                &refname,
                Some(&commit),
                expect.as_deref(),
            ) {
                remote::Push::Won => {
                    remote::write_sidecar(
                        lock,
                        &remote::Published {
                            remote: remote_name.to_owned(),
                            refname,
                            commit,
                            lease_until_unix: mine.lease_until_unix.unwrap_or(now),
                        },
                    );
                    return Ok(takeover);
                }
                remote::Push::Failed(e) => {
                    return Err(Box::new(remote_unreachable(
                        self,
                        remote_name,
                        what,
                        &e,
                        "nothing was claimed",
                    )));
                }
                remote::Push::Lost => {}
            }
            match remote::read(&self.root, remote_name, &refname) {
                Err(e) => {
                    return Err(Box::new(remote_unreachable(
                        self,
                        remote_name,
                        what,
                        &e,
                        "nothing was claimed",
                    )));
                }
                Ok(remote::Held::Absent) => {
                    expect = None;
                    takeover = None;
                }
                Ok(remote::Held::At(sha, c)) => {
                    let c = c.map(|c| *c);
                    let how = match &c {
                        Some(c) if c.actor == self.actor => Some(Takeover::Own),
                        Some(c) if c.lease_expired(now) => Some(Takeover::Reclaimed),
                        Some(c) if steal && c.age(now) > self.ttl_secs => Some(Takeover::Stolen),
                        _ => None,
                    };
                    match (how, c) {
                        (Some(how), Some(c)) => {
                            expect = Some(sha);
                            takeover = Some((c, how));
                        }
                        (_, c) => {
                            last_holder = c;
                            break;
                        }
                    }
                }
            }
        }
        let hint = last_holder.as_ref().map_or_else(String::new, |c| {
            format!(
                ". Its lease runs out in {} unless {} renews it; then `war claim {what}` takes it",
                render::ago(c.lease_left(now)),
                c.actor
            )
        });
        Err(Box::new(claimed_by_other(
            format!("{remote_name}:{refname}"),
            format!(
                "{what} is claimed on the remote `{remote_name}` by {}{hint}",
                holder(last_holder.as_ref(), now)
            ),
        )))
    }

    /// With a claims remote: whether the remote still has `lock`'s claim as
    /// this agent's. `Ok(Some(commit))`: yes, at that commit; `Ok(None)`:
    /// the remote holds nothing for it; `Err`: someone else holds it there,
    /// or the remote could not be read (refused by name either way).
    fn remote_holds_mine(
        &self,
        remote_name: &str,
        lock: &Utf8Path,
        what: &str,
        doing: &str,
    ) -> Result<Option<String>, Box<Outcome>> {
        let name = lock.file_name().unwrap_or_default();
        let refname = remote::ref_name(name);
        let now = now_secs();
        match remote::read(&self.root, remote_name, &refname) {
            Err(e) => Err(Box::new(remote_unreachable(
                self,
                remote_name,
                what,
                &e,
                doing,
            ))),
            Ok(remote::Held::Absent) => Ok(None),
            Ok(remote::Held::At(sha, c)) => {
                let c = c.map(|c| *c);
                let published = remote::read_sidecar(lock).is_some_and(|p| p.commit == sha);
                if published || c.as_ref().is_some_and(|c| c.actor == self.actor) {
                    Ok(Some(sha))
                } else {
                    Err(Box::new(claimed_by_other(
                        format!("{remote_name}:{refname}"),
                        format!(
                            "{what} is claimed on the remote `{remote_name}` by {}: your lease \
                             there ran out and it was taken; {doing}",
                            holder(c.as_ref(), now)
                        ),
                    )))
                }
            }
        }
    }

    /// [`Self::retire_remote`] at the commit this machine last published for
    /// `lock`, if it published one.
    fn retire_published(&self, lock: &Utf8Path) -> Option<Diagnostic> {
        let remote_name = self.remote.as_deref()?;
        let published = remote::read_sidecar(lock)?;
        self.retire_remote(remote_name, lock, &published.commit)
    }

    /// Delete the remote's ref for `lock` if it is still at `commit`; a
    /// warning when it could not be.
    fn retire_remote(
        &self,
        remote_name: &str,
        lock: &Utf8Path,
        commit: &str,
    ) -> Option<Diagnostic> {
        let name = lock.file_name().unwrap_or_default();
        let refname = remote::ref_name(name);
        remote::remove_sidecar(lock);
        match remote::push(&self.root, remote_name, &refname, None, Some(commit)) {
            remote::Push::Won => None,
            remote::Push::Lost => None,
            remote::Push::Failed(e) => Some(Diagnostic::warn(
                crate::work_rules::CLAIM_REMOTE_UNREACHABLE,
                format!("{remote_name}:{refname}"),
                format!(
                    "the claim's ref on `{remote_name}` was not deleted ({e}); it reads expired \
                     once its lease runs out, and a plain `war claim` takes it then"
                ),
            )),
        }
    }

    /// Republish the leases of `renewed` claims that have a published ref,
    /// when `force` or when less than half their lease is left there. Returns
    /// a line per claim the remote no longer gives this agent, and per claim
    /// whose ref could not be renewed.
    fn renew_remote(&self, renewed: &[claim::Claim], force: bool) -> Vec<String> {
        let Some(remote_name) = self.remote.as_deref() else {
            return Vec::new();
        };
        let now = now_secs();
        let mut problems = Vec::new();
        for c in renewed {
            let lock = self.lock_of(&c.ticket, c.item.as_deref());
            let Some(published) = remote::read_sidecar(&lock) else {
                continue;
            };
            let half = self.lease_secs / 2;
            if !force && now.saturating_add(half) < published.lease_until_unix {
                continue;
            }
            let commit = match remote::commit_for(&self.root, c) {
                Ok(commit) => commit,
                Err(e) => {
                    problems.push(format!(
                        "{}: not renewed on `{remote_name}` ({e})",
                        c.target()
                    ));
                    continue;
                }
            };
            match remote::push(
                &self.root,
                remote_name,
                &published.refname,
                Some(&commit),
                Some(&published.commit),
            ) {
                remote::Push::Won => remote::write_sidecar(
                    &lock,
                    &remote::Published {
                        commit,
                        lease_until_unix: c.lease_until_unix.unwrap_or(now),
                        ..published
                    },
                ),
                remote::Push::Lost => problems.push(format!(
                    "{}: the remote `{remote_name}` no longer has your claim (it was taken there)",
                    c.target()
                )),
                remote::Push::Failed(e) => {
                    problems.push(format!(
                        "{}: not renewed on `{remote_name}` ({e})",
                        c.target()
                    ));
                }
            }
        }
        problems
    }

    /// Renew this actor's leases, here and (past half their lease) on the
    /// claims remote: what every ticket command does first.
    pub fn renew_all(&self) {
        let renewed = self.renew_held(None);
        let _ = self.renew_remote(&renewed, false);
    }
}

// ---- merge driver (M11) ----------------------------------------------------

/// `war admin merge-ticket <base> <ours> <theirs> [<path>]`: git's merge driver
/// for ticket files ([`merge`]). The result goes to `ours`, as git expects;
/// a merge this cannot make is git's text merge, conflict markers and all,
/// refused `ticket.merge-conflict` so git stops and asks.
pub fn merge_ticket(
    base: &Utf8Path,
    ours: &Utf8Path,
    theirs: &Utf8Path,
    path: Option<&str>,
) -> Result<Outcome, RepoError> {
    let read = |p: &Utf8Path| {
        std::fs::read_to_string(p).map_err(|source| RepoError::Io {
            context: format!("could not read {p}"),
            source,
        })
    };
    let (o, a, b) = (read(base)?, read(ours)?, read(theirs)?);
    let shown = path.unwrap_or(ours.as_str()).to_owned();
    let (how, text) = match merge::merge(&shown, &o, &a, &b) {
        merge::Merged::Items(t) => ("items", t),
        merge::Merged::Appends(t) => ("appends", t),
        merge::Merged::Text(why) => {
            let ran = std::process::Command::new("git")
                .args(["merge-file", "-L", "ours", "-L", "base", "-L", "theirs"])
                .args([ours.as_str(), base.as_str(), theirs.as_str()])
                .stdin(std::process::Stdio::null())
                .output()
                .map_err(|source| RepoError::Io {
                    context: "could not run git merge-file".to_owned(),
                    source,
                })?;
            if ran.status.success() {
                return Ok(Outcome::ok(
                    format!("{shown}: merged as text ({why}; git found no conflict)"),
                    serde_json::json!({"schema": "oh.war/ticket-merge/v1", "path": shown, "merged": "text"}),
                ));
            }
            return Ok(Outcome::refused(
                "ticket.merge-conflict",
                shown.clone(),
                format!(
                    "{shown}: {why}, so it was merged as text and the conflict markers are in \
                     the file. Keep the line each item should have, then `git add` it"
                ),
            ));
        }
    };
    std::fs::write(ours, &text).map_err(|source| RepoError::Io {
        context: format!("could not write {ours}"),
        source,
    })?;
    Ok(Outcome::ok(
        String::new(),
        serde_json::json!({"schema": "oh.war/ticket-merge/v1", "path": shown, "merged": how}),
    ))
}

/// `war admin merge-ticket --install`.
pub fn merge_install(root: &Utf8Path) -> Outcome {
    match merge::install(root) {
        Ok(did) => Outcome::ok(
            did.join("\n"),
            serde_json::json!({"schema": "oh.war/ticket-merge-install/v1", "did": did}),
        ),
        Err(why) => Outcome::refused("ticket.merge-install", ".gitattributes", why),
    }
}

// ---- heartbeat (M11) -------------------------------------------------------

/// `war admin heartbeat [<item|ticket>]`: renew the lease on the caller's claims,
/// or on the one named. Every `war` command the holder runs renews them too;
/// this is for an agent that is working and running nothing else.
pub fn heartbeat(store: &Store, query: Option<&str>) -> Result<Outcome, RepoError> {
    let now = now_secs();
    let only = match query {
        None => None,
        Some(q) => {
            let (tickets, _) = store.load_all()?;
            let (index, item) = match resolve(&tickets, q) {
                Ok(Target::Ticket(n)) => (n, None),
                Ok(Target::Item(n, i)) => (n, Some(i)),
                Err(d) => return Ok(Outcome::from_diagnostic(d)),
            };
            let t = &tickets[index];
            let what = item
                .as_ref()
                .map_or_else(|| t.id().to_owned(), |i| format!("{}/{i}", t.id()));
            let path = store.lock_of(t.id(), item.as_deref());
            match store
                .read_claim(&path)
                .map_err(io(format!("could not read {path}")))?
            {
                None => {
                    return Ok(Outcome::refused(
                        "ticket.not-claimed",
                        store.rel(&path),
                        format!(
                            "{what} is not claimed, so there is no lease to renew; `war claim \
                             {what}` takes it"
                        ),
                    ));
                }
                Some(c) if c.as_ref().is_none_or(|c| c.actor != store.actor) => {
                    return Ok(claimed_by_other(
                        store.rel(&path),
                        format!(
                            "{what} is claimed by {}; only its holder renews the lease",
                            holder(c.as_ref(), now)
                        ),
                    ));
                }
                Some(_) => Some(claim::lock_name(t.id(), item.as_deref())),
            }
        }
    };
    let renewed = store.renew_held(only.as_deref());
    let remote_problems = store.renew_remote(&renewed, true);
    let human = if renewed.is_empty() {
        format!(
            "{} holds no claim, so there is no lease to renew",
            store.actor
        )
    } else {
        let mut h = format!("renewed {} claim(s) for {}:", renewed.len(), store.actor);
        for c in &renewed {
            h.push_str(&format!(
                "\n  {}  lease until {}",
                c.target(),
                c.lease_until.as_deref().unwrap_or("?")
            ));
        }
        h
    };
    let mut out = Outcome::ok(
        human,
        serde_json::json!({
            "schema": "oh.war/ticket-heartbeat/v1",
            "actor": store.actor,
            "renewed": renewed,
        }),
    );
    for p in remote_problems {
        out.report.push(Diagnostic::warn(
            "ticket.claim-remote-lease",
            store.remote.clone().unwrap_or_default(),
            p,
        ));
    }
    Ok(out)
}

// ---- done ------------------------------------------------------------------

/// Whether `actor` may finish `item` of `t`: it holds the item or the whole
/// ticket. `Err` is the refusal.
fn may_finish(
    store: &Store,
    t: &Ticket,
    item: Option<&str>,
    what: &str,
) -> Result<(), Box<Outcome>> {
    let now = now_secs();
    let read = |item: Option<&str>| {
        let path = store.lock_of(t.id(), item);
        (store.read_claim(&path).ok().flatten(), path)
    };
    let (own, own_path) = read(item);
    let (whole, whole_path) = if item.is_some() {
        read(None)
    } else {
        (None, own_path.clone())
    };
    let mine = |c: &Option<Option<claim::Claim>>| {
        c.as_ref()
            .and_then(Option::as_ref)
            .is_some_and(|c| c.actor == store.actor)
    };
    if mine(&own) || mine(&whole) {
        return Ok(());
    }
    if let Some(c) = own {
        return Err(Box::new(claimed_by_other(
            store.rel(&own_path),
            format!(
                "{what} is claimed by {}; only its holder finishes it",
                holder(c.as_ref(), now)
            ),
        )));
    }
    if let Some(c) = whole {
        return Err(Box::new(claimed_by_other(
            store.rel(&whole_path),
            format!(
                "{what}: the whole Warrant is claimed by {}",
                holder(c.as_ref(), now)
            ),
        )));
    }
    Err(Box::new(Outcome::refused(
        "ticket.not-claimed",
        store.rel(&own_path),
        format!(
            "{what} is not claimed by {}. Claim it first (`war claim {what}`), so two agents \
             never finish the same item",
            store.actor
        ),
    )))
}

/// `war done <item|ticket> [--note]`: a claimed tick.
pub fn done(
    store: &Store,
    query: &str,
    note: Option<&str>,
    if_rev: Option<&str>,
) -> Result<Outcome, RepoError> {
    done_with(store, query, note, if_rev, false)
}

/// `war done <item|ticket> [--note] [--if-rev] [--check]`. With `check`
/// (OW-WAR-0148 M13) the item's tests and KPIs run first and the tick is
/// written at `observed` only when every one that decides passes; a done
/// item's tick is raised to `observed` the same way. Either way a tick below
/// the minimum its item must reach is refused, naming the command that
/// reaches it.
pub fn done_with(
    store: &Store,
    query: &str,
    note: Option<&str>,
    if_rev: Option<&str>,
    check: bool,
) -> Result<Outcome, RepoError> {
    finish(
        store,
        query,
        note,
        if_rev,
        if check { How::Check } else { How::Claimed },
    )
}

/// OW-WAR-0148 M15: tick `query` with checks that already ran and passed
/// (`war evidence go` runs them in the node's worktree before its work lands,
/// and ticks only after it has), at `observed`; with `None`, as `claimed`.
/// Every other refusal of `war done` applies, in its order.
pub fn done_ran(
    store: &Store,
    query: &str,
    note: Option<&str>,
    round: Option<ladder::CheckRound>,
) -> Result<Outcome, RepoError> {
    finish(
        store,
        query,
        note,
        None,
        round.map_or(How::Claimed, How::Ran),
    )
}

/// How a tick is earned.
enum How {
    /// No check: `claimed`.
    Claimed,
    /// Run the checks now: `observed` when they pass.
    Check,
    /// The checks already ran and passed: `observed`.
    Ran(ladder::CheckRound),
}

fn finish(
    store: &Store,
    query: &str,
    note: Option<&str>,
    if_rev: Option<&str>,
    mut how: How,
) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let target = match resolve(&tickets, query) {
        Ok(t) => t,
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let now = now_secs();
    let date = date_of(now);
    let note = note.map(ticket::one_line).filter(|n| !n.is_empty());
    let (index, item_id) = match target {
        Target::Item(n, id) => (n, Some(id)),
        Target::Ticket(n) => (n, None),
    };
    let t = &tickets[index];
    let what = item_id
        .as_ref()
        .map_or_else(|| t.id().to_owned(), |i| format!("{}/{i}", t.id()));
    if let Some(refusal) = check_if_rev(store, t, item_id.as_deref(), &what, if_rev) {
        return Ok(refusal);
    }

    // A whole ticket: done in one call. With items open, every one is judged
    // as `war done <item>` judges it and all are ticked in one write, or none
    // is (`finish_ticket`). With no items, the ticket is the one item.
    if item_id.is_none() {
        if t.is_done() {
            return Ok(Outcome::ok(
                format!("{what} is already done"),
                serde_json::json!({"schema": "oh.war/ticket-done/v1", "target": what, "already_done": true, "ticket_state": "done"}),
            ));
        }
        if t.checklist.items.iter().any(|i| !i.done) {
            return finish_ticket(store, &tickets, index, note, if_rev, how);
        }
    }
    if let Some(id) = &item_id {
        let it = t.item(id).expect("resolved");
        if it.done && matches!(how, How::Check) {
            return ladder::raise_observed(store, t, id, &what, if_rev);
        }
        if it.done {
            let by = it.done_by.clone().unwrap_or_else(|| "hand".to_owned());
            return Ok(if it.done_by.as_deref() == Some(store.actor.as_str()) {
                Outcome::ok(
                    format!(
                        "{what} is already done (by you, {})",
                        it.done_on.clone().unwrap_or_default()
                    ),
                    serde_json::json!({"schema": "oh.war/ticket-done/v1", "target": what, "already_done": true}),
                )
            } else {
                Outcome::refused(
                    "ticket.already-done",
                    store.rel(&t.checklist_path),
                    format!("{what} is already done, by {by}"),
                )
            });
        }
    }
    // OW-WAR-0148 M14: under the vibe preset, a done on an item nobody
    // holds claims it first (init, create, done). So does a done that names
    // the revision it was based on (`--if-rev`, FORMAT-1.0 §11.3's
    // compare-and-set): of two finishing one item from one revision, the
    // second's base is stale and it writes nothing, so two agents still never
    // finish one item. Anywhere else the refusal stands.
    let mut auto_claim = false;
    if let Err(refusal) = may_finish(store, t, item_id.as_deref(), &what) {
        let unclaimed = refusal
            .report
            .diagnostics
            .iter()
            .any(|d| d.rule == "ticket.not-claimed");
        let based = if_rev.is_some() && matches!(how, How::Claimed);
        if !(unclaimed && (store.preset == Some(crate::preset::Preset::Vibe) || based)) {
            return Ok(*refusal);
        }
        auto_claim = true;
    }
    // FORMAT-1.0 §12: a Format 1.0 item a check evaluates with `on: done`
    // (the default) is ticked only if the run passes, at `observed`; a fail
    // or an UNKNOWN run ticks nothing. `--check` runs every check that
    // evaluates it.
    if t.format
        && let Some(id) = &item_id
        && matches!(how, How::Claimed | How::Check)
    {
        match checks::on_done(store, t, id, matches!(how, How::Check))? {
            None => {}
            Some(Ok(round)) => how = How::Ran(round),
            Some(Err(refusal)) => return Ok(*refusal),
        }
    }
    let check = !matches!(how, How::Claimed);
    // OW-WAR-0148 M13: the level this tick reaches, and the least it must.
    let checks = ladder::checks_of(t);
    let reach = if check {
        Level::Observed
    } else {
        Level::Claimed
    };
    if let Some(refusal) =
        ladder::below_minimum(store, t, &checks, item_id.as_deref(), &what, reach)
    {
        return Ok(refusal);
    }
    // The claim, as `war claim` takes it: refused when the item is blocked
    // or someone took it meanwhile, journalled when it is taken.
    let mut auto_claimed = None;
    if auto_claim {
        let claimed = claim_cmd(store, &what, false)?;
        if claimed.is_refused() {
            return Ok(claimed);
        }
        auto_claimed = Some(claimed.result.get("claim").cloned().unwrap_or_default());
    }
    let round = match how {
        How::Claimed => None,
        How::Ran(round) => Some(round),
        How::Check => match ladder::check_for_tick(store, t, item_id.as_deref(), &checks, &what)? {
            Ok(round) => Some(round),
            Err(refusal) => return Ok(*refusal),
        },
    };
    // The checks may have run for a while: the claim must still be this
    // agent's (its lease was renewed after each run) before anything is
    // ticked.
    if round.is_some()
        && let Err(refusal) = may_finish(store, t, item_id.as_deref(), &what)
    {
        return Ok(*refusal);
    }
    let date = ticks::with_level(&date, reach);
    // M11: with a claims remote, the remote must still give the claim to this
    // agent: a lease that ran out there may have been taken from another
    // machine.
    let mut remote_held = None;
    if let Some(remote_name) = store.remote.as_deref() {
        let own = store.lock_of(t.id(), item_id.as_deref());
        let lock = match store.read_claim(&own) {
            Ok(Some(Some(c))) if c.actor == store.actor => own,
            _ => store.lock_of(t.id(), None),
        };
        match store.remote_holds_mine(remote_name, &lock, &what, "nothing was ticked") {
            Ok(commit) => remote_held = commit.map(|c| (remote_name.to_owned(), lock, c)),
            Err(refusal) => return Ok(*refusal),
        }
    }
    let actor = store.actor.clone();
    let title = t.manifest.title.clone();
    let wanted = item_id.clone();
    // FORMAT-1.0 §11.1: a Format 1.0 ticket with no items is done as the
    // document itself: no line is added, and the journal's `work.item_done`
    // names the document.
    let whole_doc = t.format && item_id.is_none() && t.checklist.items.is_empty();
    let written = if whole_doc {
        Ok((t.id().to_owned(), Vec::new()))
    } else {
        rewrite(&t.checklist_path, |text| {
            let (text, named_now) = named_in(t, text);
            let c = checklist_in(t, &text);
            match &wanted {
                Some(id) => {
                    let Some(it) = c.item(id) else {
                        return Err(Box::new(Outcome::refused(
                            "ticket.unknown",
                            String::new(),
                            format!(
                                "{id} is no longer in the checklist (edited meanwhile); nothing was written"
                            ),
                        )));
                    };
                    // The compare and the set are one write: the line judged is
                    // the line the rename replaces (`rewrite`'s prestate).
                    if let Some(given) = if_rev {
                        let now = item_revision(&text, it);
                        if !same_revision(given, &now) {
                            return Err(Box::new(stale_revision(
                                store,
                                t,
                                &what,
                                &t.checklist_path,
                                given,
                                &now,
                            )));
                        }
                    }
                    let line = it.ticked(&actor, &date, note.as_deref()).render();
                    Ok((
                        ticket::replace_line(&text, it.line, &line),
                        (id.clone(), named_now),
                    ))
                }
                None => {
                    let id = fresh_item_id(&c.ids());
                    let line = Item::new(&id, &title, Vec::new())
                        .ticked(&actor, &date, note.as_deref())
                        .render();
                    Ok((with_item(t, &text, &c, &line), (id, named_now)))
                }
            }
        })?
    };
    let (done_id, named_now) = match written {
        Ok(v) => v,
        Err(refusal) => return Ok(*refusal),
    };
    let t = store
        .load(&t.dir)
        .map_err(|d| RepoError::Message(format!("{}: {}", d.rule, d.message)))?;
    let on = date_of(now);
    let mut payload = serde_json::json!({"item": done_id, "target": what, "on": on});
    // A whole document's done is its state (§11.1), so a done after a
    // reopening must not read as a replay of the first: it names the claim
    // it ends, a new one each time.
    if whole_doc && let Ok(Some(Some(c))) = store.read_claim(&store.lock_of(t.id(), None)) {
        payload["claim"] = serde_json::json!(c.claim);
    }
    if let Some(n) = &note {
        payload["note"] = serde_json::json!(n);
    }
    if !named_now.is_empty() {
        payload["named"] = serde_json::json!(named_now);
    }
    // A claimed tick's event is the bytes it always was; an observed one
    // carries its level and the receipt of every run.
    if let Some(r) = &round {
        payload["level"] = serde_json::json!(reach.as_str());
        payload["runs"] = serde_json::to_value(&r.runs).unwrap_or_default();
        payload["commit"] = serde_json::json!(r.commit);
        payload["run"] = serde_json::json!(r.run);
    }
    store.journal(&t, event::ITEM_DONE, &payload)?;
    let t = if whole_doc {
        store
            .load(&t.dir)
            .map_err(|d| RepoError::Message(format!("{}: {}", d.rule, d.message)))?
    } else {
        t
    };
    // The claim is spent: release the item's, and the ticket's once it is done.
    let mut remote_warnings = Vec::new();
    if let Some((remote_name, lock, commit)) = &remote_held {
        remote_warnings.extend(store.retire_remote(remote_name, lock, commit));
    }
    let item_lock = store.lock_of(t.id(), Some(&done_id));
    if remote_held.as_ref().is_none_or(|(_, l, _)| *l != item_lock) {
        remote_warnings.extend(store.retire_published(&item_lock));
    }
    let _ = claim::release(&item_lock, &store.actor);
    if t.is_done() {
        let whole = store.lock_of(t.id(), None);
        if remote_held.as_ref().is_none_or(|(_, l, _)| *l != whole) {
            remote_warnings.extend(store.retire_published(&whole));
        }
        let _ = claim::release(&whole, &store.actor);
    }
    let claims = store.claims()?;
    let state = state_of(&t, &claims);
    let (d, n) = t.checklist.progress();
    let mut human = if whole_doc {
        format!("done {}", t.id())
    } else {
        format!("done {}/{done_id}  ({d}/{n})", t.id())
    };
    if auto_claimed.is_some() {
        human = format!(
            "claimed {what} first: under the vibe preset `war done` claims an item nobody \
             holds
{human}"
        );
    }
    // Every tick says how it was earned (OW-WAR-0148 M13).
    match &round {
        Some(r) => human.push_str(&format!(
            "\nticked at observed: {} passed",
            ladder::passed_names(r)
        )),
        None => human.push_str("\nticked as claimed: nothing was checked"),
    }
    // OW-WAR-0148 M5: the ticket just became done. If it was made from an
    // issue, say so there when write-back is configured.
    let mut issue_report = None;
    let mut unknown = None;
    if state == TicketState::Done
        && let Some(number) = t.manifest.issue
    {
        let (report, line, diag) = issue_writeback(store, &t, number)?;
        human.push_str(&format!("\n{} is done: every item is ticked", t.id()));
        human.push('\n');
        human.push_str(&line);
        issue_report = Some(report);
        unknown = diag;
    } else if state == TicketState::Done {
        human.push_str(&format!("\n{} is done: every item is ticked", t.id()));
    } else if let Ok(rows) = ready_rows(store, std::slice::from_ref(&t))
        && let Some(next) = rows.first()
    {
        human.push_str(&format!(
            "\nnext in this Warrant: {}  {}",
            next.target(),
            next.text
        ));
    }
    let mut result = serde_json::json!({
        "schema": "oh.war/ticket-done/v1",
        "ticket": t.id(),
        "item": done_id,
        "note": note,
        "progress": {"done": d, "total": n},
        "ticket_state": state,
        "level": reach.as_str(),
    });
    if let Some(r) = &round {
        result["checks"] = serde_json::to_value(r).unwrap_or_default();
    }
    if let Some(c) = auto_claimed {
        result["auto_claimed"] = c;
    }
    if let Some(r) = issue_report {
        result["issue"] = r;
    }
    let mut out = Outcome::ok(human, result);
    if let Some(d) = unknown {
        out.report.push(d);
    }
    for w in remote_warnings {
        out.report.push(w);
    }
    if let Some(r) = &round {
        for w in ladder::signal_warnings(r) {
            out.report.push(w);
        }
    }
    Ok(out)
}

/// `war done <ticket>` with items open: every open item ticked in one write,
/// or none. Each is judged first as `war done <item>` judges it: held by this
/// actor (or the whole ticket is; under the vibe preset, or with `--if-rev`,
/// an unheld ticket is claimed whole first), not waiting on anything, its
/// checks passing (`on: done`, or every one with `--check`) and its tick at
/// the least its item must reach. Each item gets its own `work.item_done`
/// (estimates learn from them); a note is one ticket note, not a copy per
/// line.
fn finish_ticket(
    store: &Store,
    tickets: &[Ticket],
    index: usize,
    note: Option<String>,
    if_rev: Option<&str>,
    how: How,
) -> Result<Outcome, RepoError> {
    let reload = |t: &Ticket| {
        store
            .load(&t.dir)
            .map_err(|d| RepoError::Message(format!("{}: {}", d.rule, d.message)))
    };
    let mut t = tickets[index].clone();
    // Lines a human added without an id are named first, as `war claim`
    // names them, so every item ticked is named in the journal.
    if t.checklist.items.iter().any(|i| !i.done && i.id.is_none()) {
        if let Err(refusal) = rewrite(&t.checklist_path, |text| Ok(named_in(&t, text)))? {
            return Ok(*refusal);
        }
        t = reload(&t)?;
    }
    let what = t.id().to_owned();
    let open: Vec<String> = t
        .checklist
        .items
        .iter()
        .filter(|i| !i.done)
        .filter_map(|i| i.id.clone())
        .collect();
    let item_what = |id: &str| format!("{what}/{id}");

    // The claim: every open item held by this actor, by its own claim or the
    // whole ticket's.
    let mut auto_claimed = None;
    if let Some(refusal) = open
        .iter()
        .find_map(|id| may_finish(store, &t, Some(id), &item_what(id)).err())
    {
        let unclaimed = refusal
            .report
            .diagnostics
            .iter()
            .any(|d| d.rule == "ticket.not-claimed");
        let based = if_rev.is_some() && matches!(how, How::Claimed);
        if !(unclaimed && (store.preset == Some(crate::preset::Preset::Vibe) || based)) {
            return Ok(*refusal);
        }
        let claimed = claim_cmd(store, &what, false)?;
        if claimed.is_refused() {
            return Ok(claimed);
        }
        auto_claimed = Some(claimed.result.get("claim").cloned().unwrap_or_default());
    }
    // What the items wait on.
    for id in &open {
        let it = t.item(id).expect("open item");
        let waiting = open_blockers(tickets, &t, it);
        if !waiting.is_empty() {
            return Ok(Outcome::refused(
                "ticket.blocked",
                store.rel(&t.checklist_path),
                format!(
                    "{} waits on {}; nothing was ticked",
                    item_what(id),
                    waiting.join(", ")
                ),
            ));
        }
    }
    // Each item's checks and least tick, before anything is written.
    let checks = ladder::checks_of(&t);
    let checking = matches!(how, How::Check);
    let judged_here = matches!(how, How::Claimed | How::Check);
    let mut whole_round = match how {
        How::Ran(round) => Some(round),
        How::Claimed | How::Check => None,
    };
    if checking {
        match ladder::check_for_tick(store, &t, None, &checks, &what)? {
            Ok(round) => whole_round = Some(round),
            Err(refusal) => return Ok(*refusal),
        }
    }
    let mut ticks_of: Vec<(String, Option<ladder::CheckRound>)> = Vec::new();
    for id in &open {
        let mut round = whole_round.clone();
        if t.format && judged_here {
            match checks::on_done(store, &t, id, checking)? {
                None => {}
                Some(Ok(r)) => round = Some(r),
                Some(Err(refusal)) => return Ok(*refusal),
            }
        }
        let reach = if round.is_some() {
            Level::Observed
        } else {
            Level::Claimed
        };
        if let Some(refusal) =
            ladder::below_minimum(store, &t, &checks, Some(id), &item_what(id), reach)
        {
            return Ok(refusal);
        }
        ticks_of.push((id.clone(), round));
    }
    // Checks may have run for a while: the claim must still be this agent's.
    if ticks_of.iter().any(|(_, r)| r.is_some())
        && let Some(refusal) = open
            .iter()
            .find_map(|id| may_finish(store, &t, Some(id), &item_what(id)).err())
    {
        return Ok(*refusal);
    }
    // M11: with a claims remote, the remote must still give the claim to
    // this agent.
    let mut remote_held = None;
    if let Some(remote_name) = store.remote.as_deref() {
        let lock = store.lock_of(t.id(), None);
        match store.remote_holds_mine(remote_name, &lock, &what, "nothing was ticked") {
            Ok(commit) => remote_held = commit.map(|c| (remote_name.to_owned(), lock, c)),
            Err(refusal) => return Ok(*refusal),
        }
    }
    let now = now_secs();
    let date = date_of(now);
    let actor = store.actor.clone();
    let written = rewrite(&t.checklist_path, |text| {
        let c = checklist_in(&t, text);
        let mut next = text.to_owned();
        for (id, round) in &ticks_of {
            let Some(it) = c.item(id).filter(|it| !it.done) else {
                return Err(Box::new(Outcome::refused(
                    "ticket.unknown",
                    String::new(),
                    format!("{id} was edited or ticked meanwhile; nothing was written"),
                )));
            };
            let reach = if round.is_some() {
                Level::Observed
            } else {
                Level::Claimed
            };
            let line = it
                .ticked(&actor, &ticks::with_level(&date, reach), None)
                .render();
            next = ticket::replace_line(&next, it.line, &line);
        }
        Ok((next, ()))
    })?;
    if let Err(refusal) = written {
        return Ok(*refusal);
    }
    let t = reload(&t)?;
    let on = date_of(now);
    for (id, round) in &ticks_of {
        let mut payload = serde_json::json!({"item": id, "target": item_what(id), "on": on});
        if let Some(r) = round {
            payload["level"] = serde_json::json!(Level::Observed.as_str());
            payload["runs"] = serde_json::to_value(&r.runs).unwrap_or_default();
            payload["commit"] = serde_json::json!(r.commit);
            payload["run"] = serde_json::json!(r.run);
        }
        store.journal(&t, event::ITEM_DONE, &payload)?;
    }
    if let Some(n) = &note {
        let noted = self::note(store, &what, n, None)?;
        if noted.is_refused() {
            return Ok(noted);
        }
    }
    // The claims are spent.
    let mut remote_warnings = Vec::new();
    if let Some((remote_name, lock, commit)) = &remote_held {
        remote_warnings.extend(store.retire_remote(remote_name, lock, commit));
    }
    for id in &open {
        let lock = store.lock_of(t.id(), Some(id));
        remote_warnings.extend(store.retire_published(&lock));
        let _ = claim::release(&lock, &store.actor);
    }
    let whole = store.lock_of(t.id(), None);
    if remote_held.as_ref().is_none_or(|(_, l, _)| *l != whole) {
        remote_warnings.extend(store.retire_published(&whole));
    }
    let _ = claim::release(&whole, &store.actor);
    let t = reload(&t)?;
    let claims = store.claims()?;
    let state = state_of(&t, &claims);
    let (d, n) = t.checklist.progress();
    let mut human = format!("done {what}  ({d}/{n})");
    if auto_claimed.is_some() {
        human = format!("claimed {what} first\n{human}");
    }
    let observed: Vec<&str> = ticks_of
        .iter()
        .filter(|(_, r)| r.is_some())
        .map(|(id, _)| id.as_str())
        .collect();
    if observed.is_empty() {
        human.push_str("\nticked as claimed: nothing was checked");
    } else {
        human.push_str(&format!("\nticked at observed: {}", observed.join(", ")));
    }
    let mut issue_report = None;
    let mut unknown = None;
    if state == TicketState::Done
        && let Some(number) = t.manifest.issue
    {
        let (report, line, diag) = issue_writeback(store, &t, number)?;
        human.push('\n');
        human.push_str(&line);
        issue_report = Some(report);
        unknown = diag;
    }
    let mut result = serde_json::json!({
        "schema": "oh.war/ticket-done/v1",
        "ticket": t.id(),
        "item": serde_json::Value::Null,
        "items": open,
        "note": note,
        "progress": {"done": d, "total": n},
        "ticket_state": state,
        "level": if observed.len() == ticks_of.len() { Level::Observed.as_str() } else { Level::Claimed.as_str() },
    });
    if let Some(c) = auto_claimed {
        result["auto_claimed"] = c;
    }
    if let Some(r) = issue_report {
        result["issue"] = r;
    }
    let mut out = Outcome::ok(human, result);
    if let Some(d) = unknown {
        out.report.push(d);
    }
    for w in remote_warnings {
        out.report.push(w);
    }
    for (_, r) in &ticks_of {
        if let Some(r) = r {
            for w in ladder::signal_warnings(r) {
                out.report.push(w);
            }
        }
    }
    Ok(out)
}

/// `[intake.writeback]` from `openwarrant.toml`: `Ok(None)` when absent.
fn writeback_policy(root: &Utf8Path) -> Result<Option<crate::repo::WritebackPolicy>, String> {
    let path = root.join(crate::init::CONFIG_FILE);
    let text =
        crate::vfs::read_to_string(&path).map_err(|e| format!("could not read {path}: {e}"))?;
    #[derive(Deserialize)]
    struct File {
        #[serde(default)]
        intake: Option<crate::repo::IntakePolicy>,
    }
    let file: File =
        toml::from_str(&text).map_err(|e| format!("{path}: the [intake] table: {e}"))?;
    Ok(file.intake.and_then(|i| i.writeback))
}

/// The comment a done ticket leaves on its issue: what was done, item by
/// item with who and the note, then the ticket's notes.
#[must_use]
pub fn writeback_body(t: &Ticket) -> String {
    let mut body = format!("Done in Warrant {}: {}\n\n", t.id(), t.manifest.title);
    for item in &t.checklist.items {
        body.push_str(&format!(
            "- [{}] {}",
            if item.done { 'x' } else { ' ' },
            item.text
        ));
        match (&item.done_by, &item.done_on) {
            (Some(by), Some(on)) => body.push_str(&format!(" — done by {by}, {on}")),
            (Some(by), None) => body.push_str(&format!(" — done by {by}")),
            _ => {}
        }
        if let Some(n) = &item.note {
            body.push_str(&format!(": {n}"));
        }
        body.push('\n');
    }
    let notes = render::notes(&t.intent);
    if !notes.is_empty() {
        body.push_str("\nNotes:\n\n");
        for n in &notes {
            body.push_str(&format!("- {n}\n"));
        }
    }
    body
}

/// Write back to the issue a ticket was made from, now that it is done:
/// one comment, one close, as `[intake.writeback]` configures them. Off
/// unless configured. A write that fails leaves the ticket done (it already
/// is: the checklist was written first) and reads UNKNOWN, by name, never
/// silently; nothing is retried or rolled back. Journalled either way.
fn issue_writeback(
    store: &Store,
    t: &Ticket,
    number: u64,
) -> Result<(serde_json::Value, String, Option<Diagnostic>), RepoError> {
    let url = t.manifest.issue_url.clone().unwrap_or_default();
    let policy = match writeback_policy(&store.root) {
        Ok(None) => {
            return Ok((
                serde_json::json!({"number": number, "url": url, "writeback": "off"}),
                format!(
                    "GitHub issue #{number}: not written to ([intake.writeback] is not set); \
                     close it when you will"
                ),
                None,
            ));
        }
        Ok(Some(p)) => Ok(p),
        Err(why) => Err(why),
    };
    let steps = match policy {
        Ok(p) => crate::plan::intake::write_back(&store.root, &p, number, &writeback_body(t)),
        Err(why) => vec![crate::plan::intake::Written {
            step: "configuration".to_owned(),
            outcome: "unknown".to_owned(),
            detail: format!("{why}; nothing was started"),
        }],
    };
    let written = steps.iter().all(|s| s.outcome == "written");
    let outcome = if written { "written" } else { "unknown" };
    store.journal(
        t,
        event::ISSUE_WRITEBACK,
        &serde_json::json!({"issue": number, "outcome": outcome, "steps": steps}),
    )?;
    let said: Vec<String> = steps
        .iter()
        .map(|s| {
            if s.detail.is_empty() {
                format!("{} {}", s.step, s.outcome)
            } else {
                format!("{} {} ({})", s.step, s.outcome, s.detail)
            }
        })
        .collect();
    let (line, diag) = if written {
        (
            format!(
                "GitHub issue #{number}: {}",
                steps
                    .iter()
                    .map(|s| match s.step.as_str() {
                        "comment" => "commented",
                        "close" => "closed",
                        other => other,
                    })
                    .collect::<Vec<_>>()
                    .join(" and ")
            ),
            None,
        )
    } else {
        let message = format!(
            "GitHub issue #{number} is UNKNOWN: {}. The ticket {} is done and stays done; \
             nothing was rolled back. Look at the issue, and finish what did not land by hand",
            said.join("; "),
            t.id()
        );
        (
            format!("GitHub issue #{number}: UNKNOWN ({})", said.join("; ")),
            Some(Diagnostic::unknown(
                "ticket.issue-unknown",
                store.rel(&t.dir.join("manifest.toml")),
                message,
            )),
        )
    };
    Ok((
        serde_json::json!({"number": number, "url": url, "writeback": outcome, "steps": steps}),
        line,
        diag,
    ))
}

// ---- add / note ------------------------------------------------------------

/// `war add <ticket> "<text>" [--after <item|ticket>]...`.
pub fn add(
    store: &Store,
    query: &str,
    text: &str,
    after: &[String],
    if_rev: Option<&str>,
) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let (index, named_item) = match resolve(&tickets, query) {
        Ok(Target::Ticket(n)) => (n, None),
        Ok(Target::Item(n, i)) => (n, Some(i)),
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let t = &tickets[index];
    let what = named_item
        .as_ref()
        .map_or_else(|| t.id().to_owned(), |i| format!("{}/{i}", t.id()));
    if let Some(refusal) = check_if_rev(store, t, named_item.as_deref(), &what, if_rev) {
        return Ok(refusal);
    }
    let text = ticket::one_line(text);
    if text.is_empty() {
        return Ok(Outcome::refused(
            "ticket.item-empty",
            String::new(),
            "an item needs text",
        ));
    }
    let mut blockers = Vec::new();
    for a in after {
        // `--after i-3f` resolves a prefix within this ticket first.
        let blocker = if a.starts_with("i-") {
            let ids: Vec<String> = t.checklist.ids().into_iter().collect();
            let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
            match pick(&refs, a) {
                Ok(id) => Blocker::Item {
                    item: id.to_owned(),
                },
                Err(_) => {
                    return Ok(Outcome::refused(
                        "ticket.blocker-unknown",
                        store.rel(&t.checklist_path),
                        format!("`--after {a}`: no item of {} is {a}", t.id()),
                    ));
                }
            }
        } else {
            match resolve(&tickets, a) {
                Ok(Target::Ticket(n)) => Blocker::Ticket {
                    ticket: tickets[n].id().to_owned(),
                },
                Ok(Target::Item(n, i)) => Blocker::ItemOf {
                    ticket: tickets[n].id().to_owned(),
                    item: i,
                },
                Err(d) => {
                    return Ok(Outcome::refused(
                        "ticket.blocker-unknown",
                        store.rel(&t.checklist_path),
                        format!("`--after {a}`: {}", d.message),
                    ));
                }
            }
        };
        blockers.push(blocker);
    }
    let written = rewrite(&t.checklist_path, |current| {
        let (current, named_now) = named_in(t, current);
        let c = checklist_in(t, &current);
        let id = fresh_item_id(&c.ids());
        let line = Item::new(&id, &text, blockers.clone()).render();
        Ok((with_item(t, &current, &c, &line), (id, line, named_now)))
    })?;
    let (id, line, named_now) = match written {
        Ok(v) => v,
        Err(refusal) => return Ok(*refusal),
    };
    let mut payload = serde_json::json!({"item": id, "text": text});
    if !blockers.is_empty() {
        payload["after"] =
            serde_json::json!(blockers.iter().map(ToString::to_string).collect::<Vec<_>>());
    }
    if !named_now.is_empty() {
        payload["named"] = serde_json::json!(named_now);
    }
    store.journal(t, event::ITEM_ADDED, &payload)?;
    Ok(Outcome::ok(
        format!("added {}/{id}\n{line}", t.id()),
        serde_json::json!({"schema": "oh.war/ticket-item/v1", "ticket": t.id(), "item": id, "text": text, "after": blockers}),
    ))
}

// ---- edit (OW-WAR-0148 M5) ---------------------------------------------------

/// What `war edit` changes; `None` leaves a field as it is.
#[derive(Debug, Clone, Default)]
pub struct EditArgs {
    /// `Some(None)`: clear the type.
    pub kind: Option<Option<String>>,
    pub add_labels: Vec<String>,
    pub remove_labels: Vec<String>,
    /// `Some(None)`: no longer part of anything.
    pub part_of: Option<Option<String>>,
    pub priority: Option<u8>,
    /// OW-WAR-0148 M15: `Some(None)` clears the due date.
    pub due: Option<Option<String>>,
    /// M11: the ticket revision the caller read (`--if-rev`); a stale one
    /// is refused, `work.stale-revision`.
    pub if_rev: Option<String>,
}

/// `war edit <ticket>`: set a ticket's type, labels, epic or priority. Each
/// changes one line of `manifest.toml` (added, replaced or removed) and
/// nothing else; journalled as `ticket.edited`.
pub fn edit(store: &Store, query: &str, args: &EditArgs) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let index = match resolve(&tickets, query) {
        Ok(Target::Ticket(n)) => n,
        Ok(Target::Item(..)) => {
            return Ok(Outcome::refused(
                "ticket.edit-item",
                String::new(),
                format!("`war edit {query}` names an item; type, labels and epic are a Warrant's"),
            ));
        }
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let t = &tickets[index];
    if let Some(refusal) = format_unsupported(store, t, "a change to its manifest") {
        return Ok(refusal);
    }
    if let Some(refusal) = check_if_rev(store, t, None, t.id(), args.if_rev.as_deref()) {
        return Ok(refusal);
    }
    let mut m = t.manifest.clone();
    let mut changes = serde_json::Map::new();
    if let Some(kind) = &args.kind {
        if let Some(refusal) = refuse_fields(store, kind.as_deref(), &[]) {
            return Ok(refusal);
        }
        if m.kind != *kind {
            m.kind.clone_from(kind);
            changes.insert("type".into(), serde_json::json!(kind));
        }
    }
    let added = label_set(&args.add_labels);
    if let Some(refusal) = refuse_fields(store, None, &added) {
        return Ok(refusal);
    }
    let removed = label_set(&args.remove_labels);
    let mut labels: BTreeSet<String> = m.labels.iter().cloned().collect();
    labels.extend(added);
    for r in &removed {
        labels.remove(r);
    }
    let labels: Vec<String> = labels.into_iter().collect();
    if labels != m.labels {
        m.labels.clone_from(&labels);
        changes.insert("labels".into(), serde_json::json!(labels));
    }
    if let Some(p) = &args.part_of {
        let parent = match p {
            None => None,
            Some(q) => match parent_of(&tickets, q, Some(t.id())) {
                Ok(id) => Some(id),
                Err(refusal) => return Ok(*refusal),
            },
        };
        if m.part_of != parent {
            m.part_of.clone_from(&parent);
            changes.insert("part_of".into(), serde_json::json!(parent));
        }
    }
    if let Some(p) = args.priority {
        if p > 4 {
            return Ok(Outcome::refused(
                "ticket.priority",
                String::new(),
                format!("priority {p} is outside 0 (most urgent) ..= 4"),
            ));
        }
        if m.priority != p {
            m.priority = p;
            changes.insert("priority".into(), serde_json::json!(p));
        }
    }
    if let Some(due) = &args.due {
        if let Some(d) = due
            && ticket::due_date(d).is_none()
        {
            return Ok(Outcome::refused(
                "ticket.due-invalid",
                store.rel(&t.dir.join("manifest.toml")),
                format!(
                    "due {d:?} is not a calendar date; give it as YYYY-MM-DD (2026-11-01), or \
                     `none` to clear it. Nothing was written"
                ),
            ));
        }
        if m.due != *due {
            m.due.clone_from(due);
            changes.insert("due".into(), serde_json::json!(due));
        }
    }
    if let Err(why) = m.validate(&store.definition.working_roles()) {
        return Ok(Outcome::refused(
            "ticket.manifest",
            store.rel(&t.dir.join("manifest.toml")),
            why,
        ));
    }
    if changes.is_empty() {
        return Ok(Outcome::ok(
            format!("{}: nothing to change", t.id()),
            serde_json::json!({"schema": "oh.war/ticket-edit/v1", "ticket": t.id(), "changed": {}}),
        ));
    }
    let manifest_path = t.dir.join("manifest.toml");
    let edited = m.clone();
    let written = rewrite(&manifest_path, |text| {
        // The compare and the set are one write (`rewrite`'s prestate).
        if let Some(given) = args.if_rev.as_deref() {
            let now = revision_of(text.as_bytes());
            if !same_revision(given, &now) {
                return Err(Box::new(stale_revision(
                    store,
                    t,
                    t.id(),
                    &manifest_path,
                    given,
                    &now,
                )));
            }
        }
        let mut next = text.to_owned();
        let quoted = |s: &str| ticket::toml_string(s);
        next =
            ticket::set_manifest_key(&next, "type", edited.kind.as_deref().map(quoted).as_deref());
        let labels = (!edited.labels.is_empty()).then(|| {
            format!(
                "[{}]",
                edited
                    .labels
                    .iter()
                    .map(|l| quoted(l))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        });
        next = ticket::set_manifest_key(&next, "labels", labels.as_deref());
        next = ticket::set_manifest_key(
            &next,
            "part_of",
            edited.part_of.as_deref().map(quoted).as_deref(),
        );
        if edited.priority != t.manifest.priority {
            next = ticket::set_manifest_key(&next, "priority", Some(&edited.priority.to_string()));
        }
        if edited.due != t.manifest.due {
            next = ticket::set_manifest_key(
                &next,
                "due",
                edited.due.as_deref().map(quoted).as_deref(),
            );
        }
        // What was written must read back as the manifest asked for.
        match toml::from_str::<TicketManifest>(&next) {
            Ok(back) if back == edited => Ok((next, ())),
            _ => Err(Box::new(Outcome::refused(
                "ticket.manifest",
                store.rel(&manifest_path),
                "the manifest is laid out so that a line edit would not read back as asked \
                 (a key inside a table?); nothing was written. Edit it by hand",
            ))),
        }
    })?;
    if let Err(refusal) = written {
        return Ok(*refusal);
    }
    // M11: each edit is its own event. The changes alone repeat (a priority
    // set to 3 again, by someone else), and a repeated payload is a journal
    // idempotency conflict: the manifest was written and the command then
    // failed. The edit's own id keeps every edit's key distinct.
    let mut payload = changes.clone();
    payload.insert(
        "edit".into(),
        serde_json::json!(WarUuid::mint().to_string()),
    );
    store.journal(t, event::EDITED, &serde_json::Value::Object(payload))?;
    let said: Vec<String> = changes
        .iter()
        .map(|(k, v)| match v {
            serde_json::Value::Null => format!("{k} cleared"),
            serde_json::Value::String(s) => format!("{k} {s}"),
            serde_json::Value::Array(a) => format!(
                "{k} [{}]",
                a.iter()
                    .filter_map(|x| x.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            other => format!("{k} {other}"),
        })
        .collect();
    Ok(Outcome::ok(
        format!("{}: {}", t.id(), said.join("; ")),
        serde_json::json!({"schema": "oh.war/ticket-edit/v1", "ticket": t.id(), "changed": changes}),
    ))
}

/// The heading `war note` appends under, created at the end of the intent
/// atom the first time.
pub const NOTES_HEADING: &str = "## Notes";

/// `war note <ticket|item> "<text>"`: a dated note in the ticket's intent, the
/// durable context the next agent or human reads.
pub fn note(
    store: &Store,
    query: &str,
    text: &str,
    if_rev: Option<&str>,
) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let (index, item) = match resolve(&tickets, query) {
        Ok(Target::Ticket(n)) => (n, None),
        Ok(Target::Item(n, i)) => (n, Some(i)),
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let t = &tickets[index];
    let what = item
        .as_ref()
        .map_or_else(|| t.id().to_owned(), |i| format!("{}/{i}", t.id()));
    if let Some(refusal) = check_if_rev(store, t, item.as_deref(), &what, if_rev) {
        return Ok(refusal);
    }
    let body = text.trim();
    if body.is_empty() {
        return Ok(Outcome::refused(
            "ticket.note-empty",
            String::new(),
            "a note needs text",
        ));
    }
    let now = now_secs();
    let stamp = rfc3339(now);
    // FORMAT-1.0 §11: the documents say what the work is and the journal what
    // happened to it, so a Format 1.0 work document's note is a
    // `work.note_added` event, `{target, note}` (§11.2), and the document is
    // not touched. The time is not in the payload, so the same note by the
    // same actor is a replay and by another a conflict (§11.2, exactly once).
    if t.format {
        return note_event(store, t, &what, item.as_deref(), body);
    }
    let mut lines = body.lines();
    let mut entry = format!(
        "- **{} {} UTC, {}{}:** {}\n",
        &stamp[..10],
        &stamp[11..16],
        store.actor,
        item.as_ref()
            .map(|i| format!(", on {i}"))
            .unwrap_or_default(),
        lines.next().unwrap_or_default().trim()
    );
    for more in lines {
        entry.push_str("  ");
        entry.push_str(more.trim_end());
        entry.push('\n');
    }
    let written = rewrite(&t.intent_path, |current| {
        if t.format {
            return Ok((format::append_note(current, &entry, NOTES_HEADING), ()));
        }
        let mut next = current.to_owned();
        let has_heading = current.lines().any(|l| l.trim_end() == NOTES_HEADING);
        if !next.is_empty() && !next.ends_with('\n') {
            next.push('\n');
        }
        if !has_heading {
            next.push('\n');
            next.push_str(NOTES_HEADING);
            next.push_str("\n\n");
        }
        next.push_str(&entry);
        Ok((next, ()))
    })?;
    if let Err(refusal) = written {
        return Ok(*refusal);
    }
    let mut payload = serde_json::json!({"at": stamp, "note": body});
    if let Some(i) = &item {
        payload["item"] = serde_json::json!(i);
    }
    store.journal(t, event::NOTE_ADDED, &payload)?;
    Ok(Outcome::ok(
        format!("noted on {}: {}", t.id(), entry.trim_end()),
        serde_json::json!({"schema": "oh.war/ticket-note/v1", "ticket": t.id(), "item": item, "at": stamp, "note": body}),
    ))
}

/// A Format 1.0 work document's note: one `work.note_added` event, written
/// once (§11.2). The same note from the same actor replays and writes
/// nothing; from another it is refused `work.idempotency-conflict`, at the
/// line that already records it.
fn note_event(
    store: &Store,
    t: &Ticket,
    what: &str,
    item: Option<&str>,
    body: &str,
) -> Result<Outcome, RepoError> {
    let payload = serde_json::json!({"target": what, "note": body});
    let text = payload.to_string();
    let result = |replayed: bool| {
        serde_json::json!({"schema": "oh.war/ticket-note/v1", "ticket": t.id(), "item": item,
            "target": what, "note": body, "replayed": replayed})
    };
    match crate::journal_cmd::already_recorded(&t.dir, event::NOTE_ADDED, &text, &store.actor)? {
        crate::journal_cmd::Prior::Equivalent { occurred_at } => Ok(Outcome::ok(
            format!(
                "{what}: this note is already recorded by {} ({occurred_at}); nothing written",
                store.actor
            ),
            result(true),
        )),
        crate::journal_cmd::Prior::Conflict { recorded_by } => {
            let journal = t.dir.join(crate::journal_cmd::FILE);
            let rel = store.rel(&journal);
            let file = crate::journal_cmd::recorded_line(&t.dir, event::NOTE_ADDED, &text)
                .map_or_else(|| rel.clone(), |n| format!("{rel}:{n}"));
            Ok(Outcome::from_diagnostic(crate::journal_cmd::conflict(
                file,
                event::NOTE_ADDED,
                &recorded_by,
                &store.actor,
            )))
        }
        crate::journal_cmd::Prior::Fresh => {
            store.journal(t, event::NOTE_ADDED, &payload)?;
            Ok(Outcome::ok(
                format!("noted on {what}: {body}"),
                result(false),
            ))
        }
    }
}

// ---- promote ---------------------------------------------------------------

/// `war plan promote <ticket>`: draft a delivery Warrant from a ticket, for when
/// someone wants sign-off. The Warrant starts where `war plan new` starts it, with
/// the ticket's description and checklist carried into its intent; from there
/// the authority layer applies as it always has. The ticket stays workable.
pub fn promote(repo: &Repository, store: &Store, query: &str) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let index = match resolve(&tickets, query) {
        Ok(Target::Ticket(n) | Target::Item(n, _)) => n,
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let t = &tickets[index];
    if let Some(refusal) = format_unsupported(store, t, "`promoted_to` in a manifest") {
        return Ok(refusal);
    }
    if let Some(w) = &t.manifest.promoted_to {
        return Ok(Outcome::refused(
            "ticket.already-promoted",
            store.rel(&t.dir.join("manifest.toml")),
            format!(
                "{} was promoted into {w} already; work the contract there",
                t.id()
            ),
        ));
    }
    // `war new` writes the title into manifest.toml verbatim, quoted: a quote,
    // a backslash or a control character would be a TOML injection.
    let title: String = t
        .manifest
        .title
        .chars()
        .map(|c| match c {
            '"' => '\'',
            '\\' => '/',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    // The promotion target is the profile's data (OW-ADR-0031): the core
    // profile the ticket's working form `extends`, never a name in the code.
    let target = repo
        .profiles
        .resolve(&t.manifest.profile)
        .ok()
        .and_then(|p| repo.profiles.definition(&p).and_then(|d| d.extends))
        .map_or(
            openwarrant_core::Profile::Delivery,
            openwarrant_core::Profile::from_core,
        );
    let dir = crate::new::run(repo, &title, target)?;
    let alias = dir.file_name().unwrap_or_default().to_owned();
    let intent_path = dir.join("atoms/10-intent.md");
    let description = render::description(&t.intent);
    let items: String = t
        .checklist
        .items
        .iter()
        .map(|i| format!("- {}{}\n", if i.done { "(done) " } else { "" }, i.text))
        .collect();
    let rel_ticket = store.rel(&t.dir);
    if intent_path.is_file() {
        let _ = rewrite(&intent_path, |current| {
            let problem = format!(
                "From ticket [{}]({}): {}\n\n",
                t.id(),
                pathdiff(&dir.join("atoms"), &t.dir),
                if description.is_empty() {
                    t.manifest.title.clone()
                } else {
                    description.clone()
                }
            );
            let outcome = if items.is_empty() {
                String::new()
            } else {
                format!("Every item of the ticket's checklist is done:\n\n{items}\n")
            };
            let mut next = current.to_owned();
            if let Some(at) = next.find("## Desired Outcome") {
                next.insert_str(at, &problem);
                if let Some(at) = next.find("## Non-goals") {
                    next.insert_str(at, &outcome);
                } else {
                    next.push_str(&outcome);
                }
            } else {
                next.push_str(&format!(
                    "\n## From ticket {}\n\n{problem}{outcome}",
                    t.id()
                ));
            }
            Ok((next, ()))
        })?;
    }
    let manifest_path = t.dir.join("manifest.toml");
    let mut manifest = t.manifest.clone();
    manifest.promoted_to = Some(alias.clone());
    atomic::write(&manifest_path, toml_of(&manifest)?)?;
    store.journal(
        t,
        event::PROMOTED,
        &serde_json::json!({"warrant": alias, "dir": repo.relative(&dir)}),
    )?;
    Ok(Outcome::ok(
        format!(
            "promoted {} into {alias} ({}), a draft delivery Warrant carrying the ticket's \
             description and checklist ({rel_ticket}).\nThe contract path starts here: answer \
             each atom's questions, `war check {alias}`, `war admin compile`, then `war sign authorize \
             {alias}` asks a human to sign. The ticket stays workable meanwhile.",
            t.id(),
            repo.relative(&dir)
        ),
        serde_json::json!({"schema": "oh.war/ticket-promoted/v1", "ticket": t.id(), "warrant": alias, "dir": repo.relative(&dir)}),
    ))
}

/// A relative path from `from` (a directory) to `to`, for a Markdown link.
fn pathdiff(from: &Utf8Path, to: &Utf8Path) -> String {
    let from: Vec<&str> = from.components().map(|c| c.as_str()).collect();
    let to_parts: Vec<&str> = to.components().map(|c| c.as_str()).collect();
    let common = from
        .iter()
        .zip(&to_parts)
        .take_while(|(a, b)| a == b)
        .count();
    let mut out: Vec<String> = std::iter::repeat_n("..".to_owned(), from.len() - common).collect();
    out.extend(to_parts[common..].iter().map(|s| (*s).to_owned()));
    out.join("/")
}

// ---- board (the app's and the web page's Tickets pane, t-67ed) -------------

/// The directories whose change changes what the Tickets panes show: the
/// ticket store and the claims. Empty when the store cannot be opened.
#[must_use]
pub fn watched(repo: &Repository) -> Vec<Utf8PathBuf> {
    Store::open(repo, None).map_or_else(
        |_| Vec::new(),
        |s| {
            let mut out = vec![s.dir.clone(), s.claims_dir.clone()];
            out.extend(s.legacy_claims_dirs().iter().cloned());
            out
        },
    )
}

/// Every ticket as `war view tickets` lists it, each with `war show`'s items,
/// notes and Markdown, and which of its items `war view ready` offers now. The
/// TUI and the web page render this and compute nothing of their own.
pub fn board(store: &Store) -> Result<serde_json::Value, RepoError> {
    let list = tickets(store)?;
    let (all, _) = store.load_all()?;
    let ready: BTreeSet<String> = ready_rows(store, &all)?
        .iter()
        .map(ReadyRow::target)
        .collect();
    let mut out = Vec::new();
    for row in list.result["tickets"]
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        let id = row["id"].as_str().unwrap_or_default().to_owned();
        let shown = show(store, &id)?;
        let items: Vec<serde_json::Value> = shown.result["items"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|mut item| {
                let target = item["id"]
                    .as_str()
                    .map_or_else(|| id.clone(), |i| format!("{id}/{i}"));
                item["target"] = serde_json::json!(target);
                item["ready"] = serde_json::json!(ready.contains(&target));
                item
            })
            .collect();
        out.push(serde_json::json!({
            "ticket": row,
            "whole_ready": ready.contains(&id),
            "items": items,
            "notes": shown.result["notes"],
            "markdown": shown.result["markdown"],
        }));
    }
    Ok(serde_json::json!({"schema": "oh.war/ticket-board/v1", "tickets": out}))
}

// ---- check -----------------------------------------------------------------

/// `war check`'s ticket rules, into `report`: each ticket's manifest and
/// checklist structure, and blockers that name nothing. Nothing about
/// authorization, evidence or verification: an unsigned ticket is a working
/// state, not a finding. `only` limits it to one ticket.
pub fn check(store: &Store, only: Option<&str>, report: &mut Report) -> Result<(), RepoError> {
    let (tickets, faults) = store.load_all()?;
    let chosen: Vec<&Ticket> = match only {
        None => tickets.iter().collect(),
        Some(q) => match resolve(&tickets, q) {
            Ok(Target::Ticket(n) | Target::Item(n, _)) => vec![&tickets[n]],
            Err(d) => {
                report.push(d);
                return Ok(());
            }
        },
    };
    // A Format 1.0 ticket is checked as the document it is
    // (`format_doc::check`): its faults are reported there, once.
    let named = chosen.clone();
    let chosen: Vec<&Ticket> = chosen.into_iter().filter(|t| !t.format).collect();
    if only.is_none() {
        for f in faults {
            if !f.rule.starts_with("format.")
                && !f.file.as_deref().is_some_and(format::is_document_path)
            {
                report.push(f);
            }
        }
    }
    let mut errors = 0;
    for t in &chosen {
        let file = store.rel(&t.checklist_path);
        for fault in &t.checklist.faults {
            errors += 1;
            report.push(Diagnostic::error(
                fault.rule,
                format!("{file}:{}", fault.line),
                format!("{}: line {}: {}", t.id(), fault.line, fault.message),
            ));
        }
        for item in &t.checklist.items {
            for b in &item.after {
                let missing = match b {
                    Blocker::Item { .. } => false,
                    Blocker::Ticket { ticket } => !tickets.iter().any(|x| x.id() == ticket),
                    Blocker::ItemOf { ticket, item } => !tickets
                        .iter()
                        .any(|x| x.id() == ticket && x.item(item).is_some()),
                };
                if missing {
                    errors += 1;
                    report.push(Diagnostic::error(
                        "ticket.blocker-unknown",
                        format!("{file}:{}", item.line + 1),
                        format!(
                            "{}: line {}: `after {b}` names no ticket or item in {}; the item \
                             waits on nothing that can finish",
                            t.id(),
                            item.line + 1,
                            store.rel(&store.dir)
                        ),
                    ));
                }
            }
        }
        // OW-WAR-0148 M13: the optional parts, when the ticket has any.
        errors += ladder::check_parts(store, t, report);
    }
    // FORMAT-1.0 §11.2: every worked ticket's journal is append-only and
    // reads, a v1 line, a v2 line or both.
    let dirs: Vec<(String, Utf8PathBuf)> = named
        .iter()
        .map(|t| (t.id().to_owned(), t.dir.clone()))
        .collect();
    crate::journal_cmd::check_work(&store.root, &dirs, report);
    // FORMAT-1.0 §11.4: a tick level the journal does not back reads down,
    // and says so (a warning), for every ticket, Format 1.0 ones too.
    let reader = ladder::Reader::new(store);
    for t in named {
        for w in ladder::unbacked(store, t, &ladder::ticks_of(&reader, t)) {
            report.push(w);
        }
    }
    if !chosen.is_empty() && errors == 0 {
        let items: usize = chosen.iter().map(|t| t.checklist.items.len()).sum();
        report.push(Diagnostic::pass(
            "ticket.well-formed",
            format!(
                "{} ticket(s), {items} item(s): every checklist well-formed, every blocker named \
                 (a ticket needs no signature, evidence or verification to be worked)",
                chosen.len()
            ),
        ));
    }
    Ok(())
}
