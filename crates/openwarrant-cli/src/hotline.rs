// SPDX-License-Identifier: Apache-2.0
//! Who may answer a question, who is asked it, and how each side hears
//! (OW-ADR-0034, superseding OW-WAR-0069's human-only rule below an Order).
//!
//! # Answer authority follows the ladder
//!
//! A question on a Ticket or a Warrant may be answered by any actor except
//! the one that asked it; the answer records who answered and their kind. A
//! question on an Order, on a Warrant whose text is signed, before any item
//! exists (an intake question asks what a person's input meant), asked with
//! `--needs-human`, or whose level cannot be read, needs a person. A level
//! that cannot be read is never guessed down.
//!
//! # Who is acting
//!
//! With an authority register (`docs/authority/roles.toml`), it decides: the
//! actor must hold an assignment, and its kind is the register's. An agent
//! session cannot answer as a register's person. Without one, a person is
//! whoever is at a terminal, with a git identity, in no agent session; any
//! other caller is an agent. An answer's `answered_by` is an attribution,
//! not a proof: a signature binds an act, and an answer is not one.
//!
//! # Who is asked
//!
//! Asking names a party: the holder of the parent work (`part_of`, else the
//! actor that dispatched this session, `OPENWARRANT_DISPATCHER`), else the
//! work's creator, else anyone; never the asker. A question that needs a
//! person is asked of a person.
//!
//! # Notices
//!
//! Both halves post a notice beside the W3 notices ([`crate::ticket::live`]):
//! the same directory, the same `delivered/` rename for once, and with
//! `[claims] remote`, the same refs for the other clones. A question shows to
//! its party until answered; an answer shows to its asker once. Nothing here
//! signs, and a notice problem never fails a command.

use std::fmt::Write as _;

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_core::authority::ActorKind;
use serde::{Deserialize, Serialize};

use crate::questions::Question;
use crate::repo::Repository;
use crate::ticket::Store;
use crate::work::Level;

pub const SCHEMA: &str = "oh.war/question-notice/v1";
/// Set by `war start --worktree` on the session it launches: the actor that
/// dispatched it, asked when no parent is held.
pub const DISPATCHER_ENV: &str = "OPENWARRANT_DISPATCHER";
/// A question asked of whoever reads it.
pub const ANYONE: &str = "anyone";
/// A question asked of any person.
pub const PERSON: &str = "person";
const PREFIX: &str = "q-";
const DELIVERED: &str = "delivered";
const READ_FILE: &str = ".remote-read-q";
/// A harness that sets one of these is an agent session: the harness
/// session ids [`crate::ticket::scopes::session`] reads.
const AGENT_MARKERS: [&str; 4] = [
    "CLAUDE_CODE_SESSION_ID",
    "CODEX_THREAD_ID",
    "CODEX_SESSION_ID",
    "KATANA_SESSION_ID",
];

// ---- who ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Agent,
    Person,
}

impl Kind {
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Person => "person",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Who {
    pub name: String,
    pub kind: Kind,
}

impl Who {
    #[must_use]
    pub fn uri(&self) -> String {
        match self.kind {
            Kind::Agent => format!("agent://{}", self.name),
            Kind::Person => format!("person://{}", self.name),
        }
    }
}

/// A name without its `agent://` or `person://`.
#[must_use]
pub fn bare(actor: &str) -> &str {
    actor
        .strip_prefix("agent://")
        .or_else(|| actor.strip_prefix("person://"))
        .unwrap_or(actor)
}

fn agent_session() -> bool {
    AGENT_MARKERS
        .iter()
        .any(|m| std::env::var_os(m).is_some_and(|v| !v.is_empty()))
}

fn git_name(root: &Utf8Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["-C", root.as_str(), "config", "user.name"])
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    let name = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    (out.status.success() && !name.is_empty()).then_some(name)
}

/// A person at a terminal: both ends a terminal, no agent session, a git identity.
fn terminal_person(root: &Utf8Path) -> Option<String> {
    (!agent_session() && crate::sign::at_a_terminal())
        .then(|| git_name(root))
        .flatten()
}

fn store_actor(repo: &Repository, actor: Option<&str>) -> String {
    Store::open(repo, actor).map_or_else(|_| repo.performer(), |s| s.actor)
}

/// Who is asking: never refused, since anyone may ask.
#[must_use]
pub fn asker(repo: &Repository, actor: Option<&str>) -> Who {
    let name = store_actor(repo, actor);
    let kind = if !agent_session() && crate::sign::at_a_terminal() {
        Kind::Person
    } else {
        Kind::Agent
    };
    Who { name, kind }
}

/// Who is answering, or the refusal: `(rule, message)`. See the module docs.
pub fn answerer(repo: &Repository, actor: Option<&str>) -> Result<Who, (String, String)> {
    let register = repo
        .load_authority_register()
        .map_err(|e| ("question.register-unreadable".to_owned(), e.to_string()))?;
    if register.assignments.is_empty() {
        if let Some(git) = terminal_person(&repo.root) {
            return Ok(Who {
                name: actor.map_or(git, str::to_owned),
                kind: Kind::Person,
            });
        }
        return Ok(Who {
            name: store_actor(repo, actor),
            kind: Kind::Agent,
        });
    }
    let name = actor.map_or_else(|| store_actor(repo, None), str::to_owned);
    let Some(assignment) = register.actor(&name) else {
        return Err((
            "question.unknown-actor".to_owned(),
            format!("{name:?} holds no role assignment in docs/authority/roles.toml"),
        ));
    };
    if assignment.actor_kind == ActorKind::Agent {
        return Ok(Who {
            name,
            kind: Kind::Agent,
        });
    }
    if agent_session() {
        return Err((
            "question.agent".to_owned(),
            format!(
                "{name:?} is a person in roles.toml, but this is an agent session: a person \
                 answers at their own terminal. Nothing was written"
            ),
        ));
    }
    Ok(Who {
        name,
        kind: Kind::Person,
    })
}

// ---- what the question needs ---------------------------------------------

/// Why a person must answer `q`, or `None` when any other actor may.
#[must_use]
pub fn needs_person(repo: &Repository, q: &Question) -> Option<String> {
    if q.needs_human {
        return Some("it was asked with --needs-human".to_owned());
    }
    if !q.redraft.is_empty() {
        return Some("an intake question asks what a person's input meant".to_owned());
    }
    let alias = &q.warrant;
    let (level, signed) = level_of(repo, alias);
    match level {
        Level::Order => Some(format!("{alias} is an Order")),
        Level::Unknown => Some(format!("{alias}'s level is UNKNOWN")),
        _ if signed => Some(format!("{alias}'s text is signed")),
        _ => None,
    }
}

/// The level of `alias`, and whether its text is under a signature.
fn level_of(repo: &Repository, alias: &str) -> (Level, bool) {
    if crate::ticket::is_ticket_ref(alias) {
        let Ok(store) = Store::open(repo, None) else {
            return (Level::Unknown, false);
        };
        let Some(t) = ticket_of(&store, alias) else {
            return (Level::Unknown, false);
        };
        return (crate::work::ticket_level(&store, &t), false);
    }
    let Ok(dir) = repo.warrant_dir(alias) else {
        return (Level::Unknown, false);
    };
    match crate::warrants::directory_row(repo, &dir).0 {
        Some(row) => {
            let signed = !matches!(row.state.as_str(), "draft");
            (row.level, signed)
        }
        None => (Level::Unknown, false),
    }
}

fn ticket_of(store: &Store, id: &str) -> Option<crate::ticket::Ticket> {
    let (tickets, _) = store.load_all().ok()?;
    tickets.into_iter().find(|t| t.id() == id)
}

/// Whom asking `alias` reaches. See the module docs.
#[must_use]
pub fn party(repo: &Repository, alias: &str, asker: &str, person: bool) -> (String, String) {
    if person {
        return (PERSON.to_owned(), "it needs a person".to_owned());
    }
    let other =
        |name: &str| (!name.is_empty() && bare(name) != asker).then(|| bare(name).to_owned());
    if crate::ticket::is_ticket_ref(alias)
        && let Ok(store) = Store::open(repo, None)
        && let Some(t) = ticket_of(&store, alias)
    {
        if let Some(parent) = t.manifest.part_of.as_deref()
            && let Ok(claims) = store.claims()
            && let Some(holder) = claims
                .values()
                .flatten()
                .filter(|c| c.ticket == parent)
                .find_map(|c| other(&c.actor))
        {
            return (holder, format!("holds {parent}"));
        }
        if let Some(d) = dispatcher().and_then(|d| other(&d)) {
            return (d, "dispatched this".to_owned());
        }
        if let Some(c) = creator(&t.dir).and_then(|c| other(&c)) {
            return (c, "created it".to_owned());
        }
        return (ANYONE.to_owned(), "nobody holds it".to_owned());
    }
    if let Some(d) = dispatcher().and_then(|d| other(&d)) {
        return (d, "dispatched this".to_owned());
    }
    if let Ok(dir) = repo.warrant_dir(alias)
        && let Some(c) = creator(&dir).and_then(|c| other(&c))
    {
        return (c, "created it".to_owned());
    }
    (ANYONE.to_owned(), "nobody holds it".to_owned())
}

fn dispatcher() -> Option<String> {
    std::env::var(DISPATCHER_ENV)
        .ok()
        .filter(|d| !d.trim().is_empty())
}

fn creator(dir: &Utf8Path) -> Option<String> {
    let journal = crate::journal_cmd::load(dir).ok()?;
    journal.events.first().map(|e| e.actor_ref.clone())
}

// ---- notices --------------------------------------------------------------

/// One question or answer, for one party.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Posted {
    pub schema: String,
    pub id: String,
    /// `asked` or `answered`.
    pub kind: String,
    /// An actor's name, [`PERSON`] or [`ANYONE`].
    pub to: String,
    pub from: String,
    pub item: String,
    pub question: String,
    pub text: String,
    /// On an answer: what was answered.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub answer: String,
    /// The checkout whose record holds the question, absolute.
    pub root: String,
    pub at: String,
}

impl Posted {
    /// What the party reads, in one line.
    #[must_use]
    pub fn line(&self) -> String {
        if self.kind == "answered" {
            format!(
                "answer: {} {} \"{}\" → {} ({})",
                self.item,
                self.question,
                clip(&self.text),
                clip(&self.answer),
                self.from
            )
        } else {
            format!(
                "question for you: {} {} \"{}\" → {}",
                self.item,
                self.question,
                clip(&self.text),
                answer_command(&self.item, &self.question)
            )
        }
    }
}

fn clip(s: &str) -> String {
    let one = s.lines().next().unwrap_or_default();
    if one.chars().count() > 80 {
        format!("{}…", one.chars().take(79).collect::<String>())
    } else {
        one.to_owned()
    }
}

/// The command that answers `id` on `item`.
#[must_use]
pub fn answer_command(item: &str, id: &str) -> String {
    format!("war plan note {item} --answer {id} \"<answer>\"")
}

fn enabled() -> bool {
    crate::vfs::is_disk()
        && std::env::var_os(crate::ticket::live::DISABLE_ENV).is_none_or(|v| v.is_empty())
}

fn dir_of(store: &Store) -> Option<Utf8PathBuf> {
    enabled()
        .then(|| crate::ticket::live::directory(store))
        .flatten()
}

fn file_id(kind: &str, root: &str, item: &str, question: &str) -> String {
    let digest =
        openwarrant_compiler::sha256_hex(format!("{kind}\n{root}\n{item}\n{question}").as_bytes());
    format!("{PREFIX}{}", &digest[..16])
}

fn files(dir: &Utf8Path) -> Vec<Utf8PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<Utf8PathBuf> = entries
        .flatten()
        .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.starts_with(PREFIX) && n.ends_with(".json"))
        })
        .collect();
    out.sort();
    out
}

/// Whether `path` is a question's or an answer's notice, by its name.
#[must_use]
pub fn is_notice(path: &Utf8Path) -> bool {
    path.file_name().is_some_and(|n| n.starts_with(PREFIX))
}

fn read(path: &Utf8Path) -> Option<Posted> {
    serde_json::from_slice::<Posted>(&std::fs::read(path).ok()?)
        .ok()
        .filter(|p| p.schema == SCHEMA)
}

fn write(dir: &Utf8Path, p: &Posted) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let body = serde_json::to_vec(p).map_err(std::io::Error::other)?;
    let path = dir.join(format!("{}.json", p.id));
    let tmp = Utf8PathBuf::from(format!("{path}.{}.tmp", std::process::id()));
    std::fs::write(&tmp, body)?;
    std::fs::rename(&tmp, &path)
}

/// Post a question or an answer. Best effort: the warning is the caller's.
pub fn post(store: &Store, mut p: Posted) -> Result<(), String> {
    let Some(dir) = dir_of(store) else {
        return Ok(());
    };
    p.schema = SCHEMA.to_owned();
    p.id = file_id(&p.kind, &p.root, &p.item, &p.question);
    write(&dir, &p).map_err(|e| e.to_string())?;
    if p.kind == "answered" {
        // The question is no news to anyone once it is answered.
        let asked = file_id("asked", &p.root, &p.item, &p.question);
        for d in [dir.clone(), dir.join(DELIVERED)] {
            let _ = std::fs::remove_file(d.join(format!("{asked}.json")));
        }
    }
    if let Some(remote) = store.remote.as_deref()
        && let Ok(body) = serde_json::to_string(&p)
    {
        crate::ticket::remote::publish_notice(&store.root, remote, &p.id, &body)?;
    }
    Ok(())
}

/// With `[claims] remote`: what the other clones posted, once per `remote_ttl`.
fn import(store: &Store, dir: &Utf8Path) {
    let Some(remote) = store.remote.as_deref() else {
        return;
    };
    let now = crate::ticket::now_secs();
    let stamp = dir.join(READ_FILE);
    let last = std::fs::read_to_string(&stamp)
        .ok()
        .and_then(|t| t.trim().parse::<u64>().ok());
    if last.is_some_and(|l| now.saturating_sub(l) < store.remote_ttl_secs) {
        return;
    }
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(&stamp, now.to_string());
    let Ok(bodies) = crate::ticket::remote::read_notices(&store.root, remote) else {
        return;
    };
    for body in bodies {
        let Ok(p) = serde_json::from_str::<Posted>(&body) else {
            continue;
        };
        if p.schema != SCHEMA || !p.id.starts_with(PREFIX) || p.id.contains(['/', '\\', '.']) {
            continue;
        }
        let name = format!("{}.json", p.id);
        if dir.join(&name).exists() || dir.join(DELIVERED).join(&name).exists() {
            continue;
        }
        // An answer heard here retires the question it answers.
        if p.kind == "answered" {
            let asked = file_id("asked", &p.root, &p.item, &p.question);
            if dir.join(DELIVERED).join(format!("{asked}.json")).exists()
                || dir.join(format!("{asked}.json")).exists()
            {
                let _ = std::fs::remove_file(dir.join(format!("{asked}.json")));
                let _ = std::fs::remove_file(dir.join(DELIVERED).join(format!("{asked}.json")));
            }
        }
        let _ = write(dir, &p);
    }
}

/// Whether `p` is for whoever `store` acts as.
fn for_me(store: &Store, p: &Posted, person: bool) -> bool {
    if p.kind == "answered" {
        return p.to == store.actor;
    }
    p.from != store.actor && (p.to == store.actor || p.to == ANYONE || (p.to == PERSON && person))
}

fn viewer_is_person(store: &Store) -> bool {
    !agent_session() && crate::sign::at_a_terminal() && git_name(&store.root).is_some()
}

/// Whether the question `p` names is still open where it was asked. A record
/// that cannot be read keeps the notice: unknown is not answered.
fn still_open(p: &Posted) -> bool {
    let root = Utf8PathBuf::from(&p.root);
    // Its checkout is gone (a removed worktree): nobody can answer it there.
    if !root.is_dir() {
        return false;
    }
    let Ok(repo) = Repository::discover(Some(root)) else {
        return true;
    };
    match crate::questions::load(&repo, &p.item) {
        Ok(qs) => qs
            .iter()
            .find(|q| q.id == p.question)
            .is_none_or(Question::is_open),
        Err(_) => true,
    }
}

/// The questions asked of this actor that still wait, newest first: for
/// `war status`, `war status --next`, `war view prime` and Mission Control.
/// Reading delivers nothing.
#[must_use]
pub fn waiting(store: &Store) -> Vec<Posted> {
    let Some(dir) = dir_of(store) else {
        return Vec::new();
    };
    import(store, &dir);
    let person = viewer_is_person(store);
    let mut out: Vec<Posted> = [dir.clone(), dir.join(DELIVERED)]
        .iter()
        .flat_map(|d| files(d))
        .filter_map(|p| read(&p))
        .filter(|p| p.kind == "asked" && for_me(store, p, person) && still_open(p))
        .collect();
    out.sort_by(|a, b| b.at.cmp(&a.at));
    out
}

/// The question `id` on `item` as some notice of this clone carries it, for
/// an answerer whose checkout does not hold the record.
#[must_use]
pub fn find(store: &Store, item: &str, id: &str) -> Option<Posted> {
    let dir = dir_of(store)?;
    import(store, &dir);
    [dir.clone(), dir.join(DELIVERED)]
        .iter()
        .flat_map(|d| files(d))
        .filter_map(|p| read(&p))
        .find(|p| p.kind == "asked" && p.item == item && p.question == id)
}

/// What this actor is owed, taken: one line, once. A question addressed to
/// it by name, or an answer to its own; renamed into `delivered/`.
fn take(store: &Store, only_item: Option<&str>) -> Option<String> {
    let dir = dir_of(store)?;
    import(store, &dir);
    let delivered = dir.join(DELIVERED);
    let mut won = Vec::new();
    for path in files(&dir) {
        let Some(p) = read(&path) else {
            continue;
        };
        let named = p.to == store.actor && p.from != store.actor;
        if !named || only_item.is_some_and(|i| i != p.item) {
            continue;
        }
        let _ = std::fs::create_dir_all(&delivered);
        if let Some(name) = path.file_name()
            && std::fs::rename(&path, delivered.join(name)).is_ok()
        {
            won.push(p);
        }
    }
    match won.as_slice() {
        [] => None,
        [one] => Some(one.line()),
        many => {
            let answers = many.iter().filter(|p| p.kind == "answered").count();
            Some(format!(
                "{} answer(s) and {} question(s) for you: {}",
                answers,
                many.len() - answers,
                many.iter()
                    .map(|p| format!("{} {}", p.item, p.question))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
    }
}

/// After any command: what this actor is owed, once. Never fails.
#[must_use]
pub fn take_line(root: Option<&Utf8Path>, actor: Option<&str>) -> Option<String> {
    if !enabled() {
        return None;
    }
    let repo = Repository::discover(root.map(Utf8Path::to_owned)).ok()?;
    let store = Store::open(&repo, actor).ok()?;
    take(&store, None)
}

/// `war start <id>`'s one line, after a newline, or nothing: answers to this
/// actor's questions on `id`, taken once, and a blocking question still waiting.
#[must_use]
pub fn start_line(store: &Store, id: &str) -> String {
    one_start_line(store, id).map_or_else(String::new, |l| format!("\n{l}"))
}

fn one_start_line(store: &Store, id: &str) -> Option<String> {
    let repo = Repository::discover(Some(store.root.clone())).ok()?;
    let mut parts: Vec<String> = take(store, Some(id)).into_iter().collect();
    if let Ok(qs) = crate::questions::load(&repo, id) {
        let open: Vec<&Question> = qs.iter().filter(|q| q.is_open() && q.blocking).collect();
        if !open.is_empty() {
            parts.push(format!(
                "waits on {} (asked of {}): stop here, or work around it; `war status` shows the \
                 answer when it comes",
                open.iter()
                    .map(|q| q.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                open.first()
                    .map(|q| q.ask_of.as_str())
                    .filter(|a| !a.is_empty())
                    .unwrap_or(ANYONE)
            ));
        }
    }
    (!parts.is_empty()).then(|| parts.join("; "))
}

/// The lines `war status` and `war view prime` lead with.
#[must_use]
pub fn status_lines(store: &Store) -> String {
    let mut out = String::new();
    for p in waiting(store) {
        let _ = writeln!(out, "{}", p.line());
    }
    out
}

/// `war status --next`'s rows: each waiting question is its party's answer act.
#[must_use]
pub fn next_actions(store: &Store) -> Vec<crate::next::Action> {
    waiting(store)
        .into_iter()
        .map(|p| crate::next::Action {
            actor: if p.to == PERSON {
                crate::next::Actor::Human
            } else {
                crate::next::Actor::Agent
            },
            warrant: p.item.clone(),
            action: "answer".to_owned(),
            command: answer_command(&p.item, &p.question),
            why: format!("{}: {} (asked by {})", p.question, clip(&p.text), p.from),
            judged: None,
        })
        .collect()
}

// ---- `war note --ask` / `--answer` ----------------------------------------

/// What `war note` (and MCP `war_note`) was asked to do with a question.
#[derive(Debug, Clone, Copy)]
pub enum NoteAct<'a> {
    Ask(crate::questions::Asking<'a>),
    Answer { id: &'a str, text: &'a str },
}

/// `war note <id> --ask "..."` or `war note <id> --answer Q-nnn "..."`: one
/// spelling for a person and an agent. `<id>` is a ticket, `t-x/i-y`, a
/// Warrant's `<alias>/<stage>`, or an intake key.
pub fn note(
    repo: &Repository,
    target: &str,
    act: NoteAct,
    actor: Option<&str>,
) -> Result<crate::ticket::Outcome, crate::repo::RepoError> {
    let (alias, stage) = target.split_once('/').unwrap_or((target, ""));
    let report = match act {
        NoteAct::Ask(mut a) => {
            a.stage = stage;
            a.actor = actor;
            crate::questions::ask(repo, alias, &a)?
        }
        NoteAct::Answer { id, text } => crate::questions::answer(repo, alias, id, text, actor)?,
    };
    let human = report
        .diagnostics
        .iter()
        .filter(|d| d.severity == crate::diagnostic::Severity::Pass)
        .map(|d| d.message.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let result = if report.is_ready() {
        serde_json::json!({"schema": "oh.war/question-act/v1", "target": alias, "said": human})
    } else {
        serde_json::Value::Null
    };
    Ok(crate::ticket::Outcome {
        report,
        human,
        result,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn posted(kind: &str, to: &str, from: &str) -> Posted {
        Posted {
            schema: SCHEMA.to_owned(),
            id: String::new(),
            kind: kind.to_owned(),
            to: to.to_owned(),
            from: from.to_owned(),
            item: "t-1".to_owned(),
            question: "Q-001".to_owned(),
            text: "which way?".to_owned(),
            answer: "this way".to_owned(),
            root: "/r".to_owned(),
            at: "2026-11-07T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn a_name_loses_its_scheme() {
        assert_eq!(bare("agent://claude"), "claude");
        assert_eq!(bare("person://Brian Lam"), "Brian Lam");
        assert_eq!(bare("orch"), "orch");
    }

    #[test]
    fn lines_carry_the_command_or_the_answer() {
        assert!(
            posted("asked", "orch", "w")
                .line()
                .contains("war plan note t-1 --answer Q-001")
        );
        assert!(
            posted("answered", "w", "orch")
                .line()
                .contains("→ this way (orch)")
        );
    }

    #[test]
    fn notice_ids_differ_by_kind_and_place() {
        assert_ne!(
            file_id("asked", "/r", "t-1", "Q-001"),
            file_id("answered", "/r", "t-1", "Q-001")
        );
        assert_ne!(
            file_id("asked", "/r", "t-1", "Q-001"),
            file_id("asked", "/s", "t-1", "Q-001")
        );
    }
}
