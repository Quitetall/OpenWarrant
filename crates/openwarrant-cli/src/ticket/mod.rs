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
//! `war promote <ticket>`, which drafts a delivery Warrant through `war new`;
//! from there the authority layer applies exactly as it always has.
//!
//! # Fast by construction
//!
//! Every command here reads `openwarrant.toml`, the profile definitions, the
//! ticket directories and the claims directory, and nothing else: no Warrant is
//! loaded, nothing is compiled, no ownership index is built and no signature is
//! verified. `conformance/plants.d/45-tickets.sh` measures each command on this
//! repository.

pub mod claim;
mod render;

use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_core::WarUuid;
use openwarrant_core::role::{ProfileDefinition, ProfileRegistry};
use openwarrant_core::ticket::{
    self, Blocker, CHECKLIST_ROLE, Checklist, DEFAULT_PRIORITY, Item, TICKET_PROFILE,
    TICKET_SCHEMA, TicketAtom, TicketManifest,
};
use serde::{Deserialize, Serialize};

use crate::compile::atomic;
use crate::diagnostic::{Diagnostic, Report, Severity};
use crate::repo::{RepoError, Repository};

pub use render::{prime, show, tickets};

/// Where tickets live unless `[tickets] dir` says otherwise.
pub const DEFAULT_DIR: &str = "docs/tickets";
/// Where claims live unless `[tickets] claims_dir` says otherwise: inside
/// `.openwarrant/state/`, which `.gitignore` already names disposable.
pub const DEFAULT_CLAIMS_DIR: &str = ".openwarrant/state/claims";
const DEFAULT_TTL_MINUTES: u64 = 120;
const DEFAULT_COMPACT_DAYS: u64 = 7;

/// The ticket profile this build ships, used when a repository has no
/// `profiles/ticket.toml` of its own (every repository `war init` made before
/// tickets existed). A repository's own definition wins.
const BUILTIN_PROFILE: &str = include_str!("../../../../profiles/ticket.toml");

/// Where `war create` writes a ticket's intent atom, relative to its directory.
const INTENT_FILE: &str = "atoms/10-intent.md";

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

/// The journal event types a ticket records.
pub mod event {
    pub const CREATED: &str = "ticket.created";
    pub const ITEM_ADDED: &str = "ticket.item_added";
    pub const CLAIMED: &str = "ticket.claimed";
    pub const CLAIM_STOLEN: &str = "ticket.claim_stolen";
    pub const RELEASED: &str = "ticket.released";
    pub const ITEM_DONE: &str = "ticket.item_done";
    pub const NOTE_ADDED: &str = "ticket.note_added";
    pub const PROMOTED: &str = "ticket.promoted";
}

/// `[tickets]` in `openwarrant.toml`. Every key is optional.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Where ticket directories live, relative to the root.
    #[serde(default)]
    pub dir: Option<String>,
    /// Where claim locks live, relative to the root or absolute. Point it at
    /// a directory several worktrees share to make claims visible across them.
    #[serde(default)]
    pub claims_dir: Option<String>,
    /// How long a claim holds before `--steal` may take it.
    #[serde(default)]
    pub claim_ttl_minutes: Option<u64>,
    /// How many days a done ticket keeps its full block in `war prime`
    /// before it collapses to one line.
    #[serde(default)]
    pub compact_after_days: Option<u64>,
}

fn policy_of(root: &Utf8Path) -> Result<Policy, RepoError> {
    let path = root.join(crate::init::CONFIG_FILE);
    let text = std::fs::read_to_string(&path).map_err(|source| RepoError::Io {
        context: format!("could not read {path}"),
        source,
    })?;
    #[derive(Deserialize)]
    struct File {
        #[serde(default)]
        tickets: Option<Policy>,
    }
    let file: File = toml::from_str(&text).map_err(|e| {
        RepoError::Message(format!("tickets.config: {path}: the [tickets] table: {e}"))
    })?;
    Ok(file.tickets.unwrap_or_default())
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
    fn ok(human: impl Into<String>, result: serde_json::Value) -> Self {
        Self {
            report: Report::default(),
            human: human.into(),
            result,
        }
    }

    fn refused(rule: &str, file: impl Into<String>, message: impl Into<String>) -> Self {
        let message = message.into();
        let mut report = Report::default();
        report.push(Diagnostic::error(rule, file, message.clone()));
        Self {
            report,
            human: message,
            result: serde_json::Value::Null,
        }
    }

    fn from_diagnostic(d: Diagnostic) -> Self {
        let mut report = Report::default();
        let human = d.message.clone();
        report.push(d);
        Self {
            report,
            human,
            result: serde_json::Value::Null,
        }
    }

    /// Whether the command was refused.
    #[must_use]
    pub fn is_refused(&self) -> bool {
        self.report.count(Severity::Error) > 0
    }
}

/// One ticket as read from disk.
#[derive(Debug, Clone)]
pub struct Ticket {
    pub dir: Utf8PathBuf,
    pub manifest: TicketManifest,
    pub intent_path: Utf8PathBuf,
    pub checklist_path: Utf8PathBuf,
    pub intent: String,
    pub checklist_text: String,
    pub checklist: Checklist,
}

impl Ticket {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.manifest.id
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
    pub claims_dir: Utf8PathBuf,
    pub ttl_secs: u64,
    pub compact_days: u64,
    pub definition: ProfileDefinition,
    /// Who this invocation acts as: `--as`, else `OPENWARRANT_ACTOR`, else
    /// `[project] performer`. A name for coordination; it authorizes nothing.
    pub actor: String,
    pub project: String,
}

/// Now, in Unix seconds.
#[must_use]
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
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
        let policy = policy_of(&repo.root)?;
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
        Ok(Self {
            root: repo.root.clone(),
            dir: resolve(policy.dir.as_deref().unwrap_or(DEFAULT_DIR)),
            claims_dir: resolve(policy.claims_dir.as_deref().unwrap_or(DEFAULT_CLAIMS_DIR)),
            ttl_secs: policy.claim_ttl_minutes.unwrap_or(DEFAULT_TTL_MINUTES) * 60,
            compact_days: policy.compact_after_days.unwrap_or(DEFAULT_COMPACT_DAYS),
            definition,
            actor,
            project: repo.config.project.name.clone(),
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
            claims_dir: rel(&self.claims_dir),
            files: vec![
                "manifest.toml".to_owned(),
                crate::journal_cmd::FILE.to_owned(),
                INTENT_FILE.to_owned(),
                format!("atoms/{}", self.checklist_file()),
            ],
        }
    }

    fn checklist_file(&self) -> &str {
        self.definition
            .required_extension_roles
            .iter()
            .find(|r| r.role == CHECKLIST_ROLE)
            .map_or("15-checklist.md", |r| r.file.as_str())
    }

    fn checklist_ordinal(&self) -> u32 {
        self.definition
            .required_extension_roles
            .iter()
            .find(|r| r.role == CHECKLIST_ROLE)
            .map_or(15, |r| r.ordinal)
    }

    fn checklist_stub(&self) -> String {
        self.definition
            .required_extension_roles
            .iter()
            .find(|r| r.role == CHECKLIST_ROLE)
            .map_or_else(|| "# Checklist\n\n".to_owned(), |r| r.stub.clone())
    }

    /// Every ticket directory (one holding a `manifest.toml`), sorted.
    fn ticket_dirs(&self) -> Result<Vec<Utf8PathBuf>, RepoError> {
        let entries = match std::fs::read_dir(&self.dir) {
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
            if path.join("manifest.toml").is_file() {
                out.push(path);
            }
        }
        out.sort();
        Ok(out)
    }

    /// Read one ticket. `Err` names what is wrong with it.
    pub fn load(&self, dir: &Utf8Path) -> Result<Ticket, Diagnostic> {
        let manifest_path = dir.join("manifest.toml");
        let rel = self.rel(&manifest_path);
        let bad = |message: String| Diagnostic::error("ticket.manifest", rel.clone(), message);
        let text = std::fs::read_to_string(&manifest_path)
            .map_err(|e| bad(format!("could not read it: {e}")))?;
        let manifest: TicketManifest =
            toml::from_str(&text).map_err(|e| bad(format!("does not parse: {e}")))?;
        manifest
            .validate(&self.definition.working_roles())
            .map_err(bad)?;
        if dir.file_name() != Some(manifest.id.as_str()) {
            return Err(Diagnostic::error(
                "ticket.id-mismatch",
                rel,
                format!(
                    "the directory is {} and the manifest's id is {}; a ticket's directory is \
                     its id",
                    dir.file_name().unwrap_or_default(),
                    manifest.id
                ),
            ));
        }
        let atom = |role: &str| -> Result<(Utf8PathBuf, String), Diagnostic> {
            let path = dir.join(manifest.atom_path(role).unwrap_or_default());
            std::fs::read_to_string(&path)
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
            dir: dir.to_owned(),
            manifest,
            intent_path,
            checklist_path,
            intent,
            checklist_text,
            checklist,
        })
    }

    /// Every readable ticket, and a diagnostic per unreadable one.
    pub fn load_all(&self) -> Result<(Vec<Ticket>, Vec<Diagnostic>), RepoError> {
        let mut tickets = Vec::new();
        let mut faults = Vec::new();
        for dir in self.ticket_dirs()? {
            match self.load(&dir) {
                Ok(t) => tickets.push(t),
                Err(d) => faults.push(d),
            }
        }
        Ok((tickets, faults))
    }

    /// Every claim now held, by lock file name.
    pub fn claims(&self) -> Result<BTreeMap<String, Option<claim::Claim>>, RepoError> {
        claim::all(&self.claims_dir).map_err(io(format!("could not read {}", self.claims_dir)))
    }

    fn lock_path(&self, ticket: &str, item: Option<&str>) -> Utf8PathBuf {
        self.claims_dir.join(claim::lock_name(ticket, item))
    }

    fn journal(
        &self,
        t: &Ticket,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<(), RepoError> {
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
                format!("no {what} is {query:?}; `war ready` and `war tickets` list them")
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
        let id = pick(&ids, q).map_err(|found| unknown("ticket", &found))?;
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
            "{query:?} is not a ticket (t-...), an item (i-...) or an item of a ticket (t-.../i-...)"
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
fn open_blockers(tickets: &[Ticket], t: &Ticket, item: &Item) -> Vec<String> {
    let find = |id: &str| tickets.iter().find(|x| x.id() == id);
    item.after
        .iter()
        .filter_map(|b| match b {
            Blocker::Item { item: i } => match t.item(i) {
                Some(x) if x.done => None,
                Some(_) => Some(i.clone()),
                None => Some(format!("{i} (unknown)")),
            },
            Blocker::Ticket { ticket: id } => match find(id) {
                Some(x) if x.checklist.is_done() => None,
                Some(_) => Some(id.clone()),
                None => Some(format!("{id} (unknown)")),
            },
            Blocker::ItemOf {
                ticket: id,
                item: i,
            } => match find(id).and_then(|x| x.item(i)) {
                Some(x) if x.done => None,
                Some(_) => Some(format!("{id}/{i}")),
                None => Some(format!("{id}/{i} (unknown)")),
            },
        })
        .collect()
}

/// A ticket's state given the claims now held.
pub(crate) fn state_of(t: &Ticket, claims: &BTreeMap<String, Option<claim::Claim>>) -> TicketState {
    if t.checklist.is_done() {
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

// ---- writes ----------------------------------------------------------------

/// Rewrite a file from its current bytes, retrying when another writer moved
/// it between the read and the rename (`storage.prestate-moved`). `edit`
/// returns the new text and a value, or `Err` to write nothing.
fn rewrite<T>(
    path: &Utf8Path,
    mut edit: impl FnMut(&str) -> Result<(String, T), Box<Outcome>>,
) -> Result<Result<T, Box<Outcome>>, RepoError> {
    for _ in 0..8 {
        let bytes = std::fs::read(path).map_err(io(format!("could not read {path}")))?;
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

fn fresh_item_id(taken: &BTreeSet<String>) -> String {
    ticket::item_id(&WarUuid::mint().to_string(), taken)
}

/// The checklist text with every unnamed item given an id.
fn named(text: &str) -> (String, Vec<String>) {
    ticket::name_unnamed(text, &mut fresh_item_id)
}

// ---- create ----------------------------------------------------------------

/// What `war create` is asked for.
#[derive(Debug, Clone, Default)]
pub struct CreateArgs {
    pub title: String,
    pub items: Vec<String>,
    pub body: Option<String>,
    pub priority: Option<u8>,
}

fn toml_of(m: &TicketManifest) -> Result<String, RepoError> {
    let body = toml::to_string(m)
        .map_err(|e| RepoError::Message(format!("could not render the ticket manifest: {e}")))?;
    Ok(format!(
        "# A ticket (OW-WAR-0147): the working form of a delivery Warrant. The atoms beside\n\
         # this file are the ticket; `war show {}` renders them. Nothing here is signed.\n{body}",
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
            "a ticket needs a title: `war create \"what this work accomplishes\"`",
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
            "ticket.id: every length of this ticket's id is taken".to_owned(),
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
        profile: TICKET_PROFILE.to_owned(),
        priority,
        created_at: rfc3339(now),
        created_by: store.actor.clone(),
        promoted_to: None,
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
    store.journal(
        &t,
        event::CREATED,
        &serde_json::json!({"ticket": id, "title": title, "items": made.len()}),
    )?;
    let rel = store.rel(&dir);
    let mut human = format!("{id}  {title}\ncreated {rel}/");
    if made.is_empty() {
        human.push_str(&format!(
            "\nno items yet: `war add {id} \"...\"`, or `war claim {id}` and work it whole"
        ));
    } else {
        human.push_str(&format!("\n{} item(s); `war ready` lists them", made.len()));
    }
    Ok(Outcome::ok(
        human,
        serde_json::json!({
            "schema": "oh.war/ticket-created/v1",
            "id": id,
            "uuid": uuid,
            "title": title,
            "dir": rel,
            "priority": priority,
            "items": made,
        }),
    ))
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

/// One row of `war ready`.
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
    /// A claim past its TTL: `war claim --steal` may take it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_claim: Option<claim::Claim>,
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

/// The ready set: open, unclaimed (or stale), unblocked items, and tickets
/// with no items that nobody holds. Most urgent, then oldest, first.
pub fn ready_rows(store: &Store, tickets: &[Ticket]) -> Result<Vec<ReadyRow>, RepoError> {
    let claims = store.claims()?;
    let now = now_secs();
    let stale = |c: Option<&claim::Claim>| -> Result<Option<claim::Claim>, ()> {
        match c {
            Some(c) if c.age(now) > store.ttl_secs => Ok(Some(c.clone())),
            _ => Err(()),
        }
    };
    let mut rows = Vec::new();
    for t in tickets {
        if t.checklist.is_done() {
            continue;
        }
        let whole = claim_on(&claims, t.id(), None);
        let whole_stale = match whole {
            None => None,
            Some((_, c)) => match stale(c) {
                Ok(s) => s,
                Err(()) => continue,
            },
        };
        let row =
            |item: Option<String>, text: String, line: Option<usize>, order: usize, stale_claim| {
                ReadyRow {
                    ticket: t.id().to_owned(),
                    title: t.manifest.title.clone(),
                    priority: t.manifest.priority,
                    item,
                    text,
                    line,
                    stale_claim,
                    created_at: t.manifest.created_at.clone(),
                    order,
                }
            };
        if t.checklist.items.is_empty() {
            rows.push(row(None, t.manifest.title.clone(), None, 0, whole_stale));
            continue;
        }
        for (order, item) in t.checklist.items.iter().enumerate() {
            if item.done || !open_blockers(tickets, t, item).is_empty() {
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
    }
    rows.sort_by(|a, b| {
        (a.priority, &a.created_at, &a.ticket, a.order).cmp(&(
            b.priority,
            &b.created_at,
            &b.ticket,
            b.order,
        ))
    });
    Ok(rows)
}

/// `war ready`.
pub fn ready(store: &Store) -> Result<Outcome, RepoError> {
    let (tickets, faults) = store.load_all()?;
    let rows = ready_rows(store, &tickets)?;
    let mut human = String::new();
    if rows.is_empty() {
        human.push_str("nothing is ready: every item is done, claimed or waiting (`war tickets`)");
    }
    for r in &rows {
        let what = match (&r.item, r.line) {
            (Some(_), _) => r.text.clone(),
            (None, Some(line)) => format!(
                "{} (line {line}, no id yet: `war claim {}` names it)",
                r.text, r.ticket
            ),
            (None, None) => format!("{} (no items: the ticket is the work)", r.text),
        };
        human.push_str(&format!("{:<16} p{}  {what}", r.target(), r.priority));
        if r.item.is_some() || r.line.is_some() {
            human.push_str(&format!("  — {}", r.title));
        }
        if let Some(c) = &r.stale_claim {
            human.push_str(&format!(
                "  [stale claim: {} since {}; `war claim --steal`]",
                c.actor, c.since
            ));
        }
        human.push('\n');
    }
    let mut out = Outcome::ok(
        human.trim_end().to_owned(),
        serde_json::json!({"schema": "oh.war/ticket-ready/v1", "ready": rows}),
    );
    for f in faults {
        out.report.push(Diagnostic::warn(
            f.rule.clone(),
            f.file.clone().unwrap_or_default(),
            format!("skipped: {}", f.message),
        ));
    }
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
    }
}

/// `war claim <item|ticket> [--steal]`.
pub fn claim_cmd(store: &Store, query: &str, steal: bool) -> Result<Outcome, RepoError> {
    let (mut tickets, _) = store.load_all()?;
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
        named_now = match rewrite(&t.checklist_path, |text| Ok(named(text)))? {
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
                    "{what} waits on {}; `war ready` lists what can start now",
                    waiting.join(", ")
                ),
            ));
        }
        // The whole ticket held by someone else holds this item too.
        if let Some((_, c)) = claim_on(&claims, t.id(), None)
            && c.is_none_or(|c| c.actor != store.actor)
        {
            return Ok(Outcome::refused(
                "ticket.claimed-by-other",
                store.rel(&store.lock_path(t.id(), None)),
                format!(
                    "{what}: the whole ticket {} is claimed by {}",
                    t.id(),
                    holder(c, now)
                ),
            ));
        }
    } else {
        if t.checklist.is_done() {
            return Ok(Outcome::refused(
                "ticket.already-done",
                store.rel(&t.checklist_path),
                format!("{what} is done: every item is ticked"),
            ));
        }
        // An item of it held by someone else: the ticket is not free whole.
        let others: Vec<String> = claims
            .iter()
            .filter(|(k, _)| k.starts_with(&format!("{}--", t.id())))
            .filter(|(_, c)| c.as_ref().is_none_or(|c| c.actor != store.actor))
            .map(|(_, c)| {
                c.as_ref().map_or_else(
                    || "an item (unreadable lock)".to_owned(),
                    |c| format!("{} by {}", c.target(), holder(Some(c), now)),
                )
            })
            .collect();
        if !others.is_empty() {
            return Ok(Outcome::refused(
                "ticket.claimed-by-other",
                store.rel(&store.claims_dir),
                format!("{what} cannot be claimed whole: {}", others.join("; ")),
            ));
        }
    }
    let path = store.lock_path(t.id(), item.as_deref());
    let mine = new_claim(store, t.id(), item.as_deref(), now);
    let taken = claim::take(&path, &mine).map_err(io(format!("could not claim {path}")))?;
    let stolen_from = match taken {
        claim::Taken::Won => None,
        claim::Taken::Held(Some(c)) if c.actor == store.actor => {
            return Ok(Outcome::ok(
                format!("{what} is already yours (since {})", c.since),
                serde_json::json!({"schema": "oh.war/ticket-claim/v1", "target": what, "claim": c, "already_held": true}),
            ));
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
                    String::new()
                };
                return Ok(Outcome::refused(
                    "ticket.claimed-by-other",
                    store.rel(&path),
                    format!("{what} is claimed by {}{hint}", holder(c.as_ref(), now)),
                ));
            }
            match claim::steal(&path, c.as_ref(), &mine)
                .map_err(io(format!("could not steal {path}")))?
            {
                claim::Stolen::Won { from } => Some(from),
                claim::Stolen::Lost(other) => {
                    return Ok(Outcome::refused(
                        "ticket.claimed-by-other",
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
            .filter(|(_, c)| c.is_none_or(|c| c.actor != store.actor))
            .map(|(_, c)| holder(c, now)),
        None => after
            .iter()
            .filter(|(k, _)| k.starts_with(&format!("{}--", t.id())))
            .find(|(_, c)| c.as_ref().is_none_or(|c| c.actor != store.actor))
            .map(|(_, c)| holder(c.as_ref(), now)),
    };
    if let Some(other) = conflict {
        let _ = claim::release(&path, &store.actor);
        return Ok(Outcome::refused(
            "ticket.claimed-by-other",
            store.rel(&path),
            format!("{what}: {other} claimed it at the same moment; released, try another"),
        ));
    }
    let mut payload = serde_json::json!({
        "claim": mine.claim,
        "target": what,
        "since": mine.since,
    });
    if !named_now.is_empty() {
        payload["named"] = serde_json::json!(named_now);
    }
    let event_type = match &stolen_from {
        Some(from) => {
            payload["from"] = serde_json::json!(from.as_ref().map(|c| &c.actor));
            payload["from_since"] = serde_json::json!(from.as_ref().map(|c| &c.since));
            event::CLAIM_STOLEN
        }
        None => event::CLAIMED,
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
    // A whole ticket with items open is done item by item: `war done <ticket>`
    // is refused (`ticket.items-open`) until the last is ticked, so the hint
    // names the next open item instead (t-87fb, t-8a2c).
    let next_open = if item.is_none() {
        t.checklist
            .items
            .iter()
            .find(|i| !i.done)
            .and_then(|i| i.id.clone())
    } else {
        None
    };
    match &next_open {
        Some(next) => human.push_str(&format!(
            "\nnext: `war done {}/{next} --note \"...\"` (the ticket is done when its last item is)",
            t.id()
        )),
        None => human.push_str(&format!(
            "\nwhen it is done: `war done {what} --note \"...\"`"
        )),
    }
    Ok(Outcome::ok(
        human,
        serde_json::json!({
            "schema": "oh.war/ticket-claim/v1",
            "target": what,
            "claim": mine,
            "stolen_from": stolen_from.flatten(),
            "named": named_now,
        }),
    ))
}

/// `war release <item|ticket>`: give a claim back without finishing.
pub fn release(store: &Store, query: &str) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let (index, item) = match resolve(&tickets, query) {
        Ok(Target::Ticket(n)) => (n, None),
        Ok(Target::Item(n, i)) => (n, Some(i)),
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let t = &tickets[index];
    let path = store.lock_path(t.id(), item.as_deref());
    let what = item
        .as_ref()
        .map_or_else(|| t.id().to_owned(), |i| format!("{}/{i}", t.id()));
    match claim::read(&path).map_err(io(format!("could not read {path}")))? {
        None => Ok(Outcome::refused(
            "ticket.not-claimed",
            store.rel(&path),
            format!("{what} is not claimed; nothing to release"),
        )),
        Some(Some(c)) if c.actor == store.actor => {
            claim::release(&path, &store.actor).map_err(io(format!("could not release {path}")))?;
            store.journal(
                t,
                event::RELEASED,
                &serde_json::json!({"claim": c.claim, "target": what}),
            )?;
            Ok(Outcome::ok(
                format!("released {what}"),
                serde_json::json!({"schema": "oh.war/ticket-release/v1", "target": what, "claim": c}),
            ))
        }
        Some(c) => Ok(Outcome::refused(
            "ticket.claimed-by-other",
            store.rel(&path),
            format!(
                "{what} is claimed by {}; only its holder releases it (a stale claim is taken \
                 with `war claim --steal`)",
                holder(c.as_ref(), now_secs())
            ),
        )),
    }
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
        let path = store.lock_path(t.id(), item);
        (claim::read(&path).ok().flatten(), path)
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
        return Err(Box::new(Outcome::refused(
            "ticket.claimed-by-other",
            store.rel(&own_path),
            format!(
                "{what} is claimed by {}; only its holder finishes it",
                holder(c.as_ref(), now)
            ),
        )));
    }
    if let Some(c) = whole {
        return Err(Box::new(Outcome::refused(
            "ticket.claimed-by-other",
            store.rel(&whole_path),
            format!(
                "{what}: the whole ticket is claimed by {}",
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

/// `war done <item|ticket> [--note]`.
pub fn done(store: &Store, query: &str, note: Option<&str>) -> Result<Outcome, RepoError> {
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

    // A whole ticket: done only when nothing remains, or when it has no items
    // (the ticket was the one item, and is ticked as one).
    if item_id.is_none() {
        let open: Vec<String> = t
            .checklist
            .items
            .iter()
            .filter(|i| !i.done)
            .map(|i| {
                i.id.clone()
                    .unwrap_or_else(|| format!("line {}", i.line + 1))
            })
            .collect();
        if t.checklist.is_done() {
            return Ok(Outcome::ok(
                format!("{what} is already done"),
                serde_json::json!({"schema": "oh.war/ticket-done/v1", "target": what, "already_done": true, "ticket_state": "done"}),
            ));
        }
        if !open.is_empty() {
            return Ok(Outcome::refused(
                "ticket.items-open",
                store.rel(&t.checklist_path),
                format!(
                    "{what} still has {} open item(s): {}. Finish each (`war done <item>`); the \
                     ticket reads done when the last one is",
                    open.len(),
                    open.join(", ")
                ),
            ));
        }
    }
    if let Some(id) = &item_id {
        let it = t.item(id).expect("resolved");
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
    if let Err(refusal) = may_finish(store, t, item_id.as_deref(), &what) {
        return Ok(*refusal);
    }
    let actor = store.actor.clone();
    let title = t.manifest.title.clone();
    let wanted = item_id.clone();
    let written = rewrite(&t.checklist_path, |text| {
        let (text, named_now) = named(text);
        let c = ticket::parse(&text);
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
                Ok((ticket::append_item(&text, &c, &line), (id, named_now)))
            }
        }
    })?;
    let (done_id, named_now) = match written {
        Ok(v) => v,
        Err(refusal) => return Ok(*refusal),
    };
    let t = store
        .load(&t.dir)
        .map_err(|d| RepoError::Message(format!("{}: {}", d.rule, d.message)))?;
    let mut payload = serde_json::json!({"item": done_id, "target": what, "on": date});
    if let Some(n) = &note {
        payload["note"] = serde_json::json!(n);
    }
    if !named_now.is_empty() {
        payload["named"] = serde_json::json!(named_now);
    }
    store.journal(&t, event::ITEM_DONE, &payload)?;
    // The claim is spent: release the item's, and the ticket's once it is done.
    let _ = claim::release(&store.lock_path(t.id(), Some(&done_id)), &store.actor);
    if t.checklist.is_done() {
        let _ = claim::release(&store.lock_path(t.id(), None), &store.actor);
    }
    let claims = store.claims()?;
    let state = state_of(&t, &claims);
    let (d, n) = t.checklist.progress();
    let mut human = format!("done {}/{done_id}  ({d}/{n})", t.id());
    if state == TicketState::Done {
        human.push_str(&format!("\n{} is done: every item is ticked", t.id()));
    } else if let Ok(rows) = ready_rows(store, std::slice::from_ref(&t))
        && let Some(next) = rows.first()
    {
        human.push_str(&format!(
            "\nnext in this ticket: {}  {}",
            next.target(),
            next.text
        ));
    }
    Ok(Outcome::ok(
        human,
        serde_json::json!({
            "schema": "oh.war/ticket-done/v1",
            "ticket": t.id(),
            "item": done_id,
            "note": note,
            "progress": {"done": d, "total": n},
            "ticket_state": state,
        }),
    ))
}

// ---- add / note ------------------------------------------------------------

/// `war add <ticket> "<text>" [--after <item|ticket>]...`.
pub fn add(store: &Store, query: &str, text: &str, after: &[String]) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let index = match resolve(&tickets, query) {
        Ok(Target::Ticket(n) | Target::Item(n, _)) => n,
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let t = &tickets[index];
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
        let (current, named_now) = named(current);
        let c = ticket::parse(&current);
        let id = fresh_item_id(&c.ids());
        let line = Item::new(&id, &text, blockers.clone()).render();
        Ok((
            ticket::append_item(&current, &c, &line),
            (id, line, named_now),
        ))
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

/// The heading `war note` appends under, created at the end of the intent
/// atom the first time.
pub const NOTES_HEADING: &str = "## Notes";

/// `war note <ticket|item> "<text>"`: a dated note in the ticket's intent, the
/// durable context the next agent or human reads.
pub fn note(store: &Store, query: &str, text: &str) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let (index, item) = match resolve(&tickets, query) {
        Ok(Target::Ticket(n)) => (n, None),
        Ok(Target::Item(n, i)) => (n, Some(i)),
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let t = &tickets[index];
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

// ---- promote ---------------------------------------------------------------

/// `war promote <ticket>`: draft a delivery Warrant from a ticket, for when
/// someone wants sign-off. The Warrant starts where `war new` starts it, with
/// the ticket's description and checklist carried into its intent; from there
/// the authority layer applies as it always has. The ticket stays workable.
pub fn promote(repo: &Repository, store: &Store, query: &str) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let index = match resolve(&tickets, query) {
        Ok(Target::Ticket(n) | Target::Item(n, _)) => n,
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let t = &tickets[index];
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
             each atom's questions, `war check {alias}`, `war compile`, then `war authorize \
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
    Store::open(repo, None).map_or_else(|_| Vec::new(), |s| vec![s.dir, s.claims_dir])
}

/// Every ticket as `war tickets` lists it, each with `war show`'s items,
/// notes and Markdown, and which of its items `war ready` offers now. The
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
    if only.is_none() {
        for f in faults {
            report.push(f);
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
