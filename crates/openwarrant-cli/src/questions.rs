// SPDX-License-Identifier: Apache-2.0

//! The hotline (OW-WAR-0069): an agent asks, a human answers, both on record.
//!
//! A performing agent that needs a decision has two bad options today: guess,
//! or stop and hope someone reads its transcript. This gives it a third: ask,
//! in a file the owner can answer from anywhere, so one person can clear the
//! blocking questions of many concurrent agents in one pass.
//!
//! Asking is anyone's act and costs nothing. Who may answer follows the
//! ladder (OW-ADR-0034, [`crate::hotline`]): on a Ticket or a Warrant any
//! actor but the asker, on an Order (or `--needs-human`) a person, as §27.2
//! draws it everywhere else. The asker answering its own question is refused
//! by name and writes nothing.
//!
//! A question is `questions/Q-nnn.toml` beside the Warrant's other records,
//! with a `question.asked` journal event; an answer appends to the same file
//! and records `question.answered`. Neither is a disposition, a judgment, or
//! an authorization: an answer informs work, it never accepts it.
//!
//! A question can also exist before its Warrant does (OW-WAR-0141, U-002).
//! When a ticket or a sentence is too thin to draft, the drafter's blocker
//! waits under `docs/intake/<key>/questions/`, where `<key>` is `github-<n>`
//! or `sentence-<digest>` and stands where an alias would. No alias is
//! allocated for it. Such a question names the command that drafts its input
//! again (`redraft`), and it is listed, answered and shown in `war view next`
//! beside every Warrant's questions.

use std::fmt::Write as _;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

pub const SCHEMA: &str = "oh.war/question/v1";
pub const LIST_SCHEMA: &str = "oh.war/questions/v1";
pub const DIR: &str = "questions";
pub const EVENT_ASKED: &str = "question.asked";
pub const EVENT_ANSWERED: &str = "question.answered";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub schema: String,
    pub id: String,
    pub warrant: String,
    /// The stage whose work the answer unblocks. Must exist in the milestones
    /// graph: a question attached to nothing cannot be found by the performer.
    pub stage: String,
    pub asked_by: String,
    pub asked_at: String,
    pub question: String,
    /// Asking for a decision the agent cannot take, versus asking for
    /// confirmation it could proceed without. Only a blocking question stops
    /// a performer.
    #[serde(default)]
    pub blocking: bool,
    /// The asker's own recommended answer, after the grilling discipline: a
    /// question with a recommendation can be answered in one word.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub recommended: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<Answer>,
    /// On an intake question only: the command that drafts its input again
    /// once it is answered. Empty on a Warrant's question.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub redraft: String,
    /// Asked with `--needs-human`: only a person answers it, at any level.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub needs_human: bool,
    /// Whom asking reached ([`crate::hotline::party`]): an actor's name,
    /// `person` or `anyone`. Empty on a question asked before it existed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ask_of: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    pub answered_by: String,
    pub answered_at: String,
    pub answer: String,
    /// `agent` or `person`: what the answerer was taken to be
    /// ([`crate::hotline::answerer`]). Empty on an answer from before.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kind: String,
}

impl Question {
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.answer.is_none()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct QuestionList {
    pub schema: String,
    pub questions: Vec<Question>,
    pub open: usize,
    pub blocking_open: usize,
    pub answered: usize,
}

/// The stage an intake question is asked against: there are no stages
/// before there is a Warrant.
pub const INTAKE_STAGE: &str = "intake";

/// Where a ticket's (or a ticket item's) questions live: one directory per
/// ticket, outside the ticket store, so no ticket encoding has to hold them.
pub const ITEM_DIR: &str = "docs/questions";

/// A ticket id that is safe as one path component.
fn item_key(alias: &str) -> bool {
    crate::ticket::is_ticket_ref(alias)
        && alias.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Where `alias`'s questions live: under the Warrant; for an intake key with
/// a directory under `docs/intake/`, there; for a ticket, under [`ITEM_DIR`].
fn dir_of(repo: &Repository, alias: &str) -> Result<Utf8PathBuf, RepoError> {
    if let Some(dir) = crate::plan::intake::existing_dir(repo, alias) {
        return Ok(dir.join(DIR));
    }
    if item_key(alias) {
        return Ok(repo.root.join(ITEM_DIR).join(alias));
    }
    Ok(repo.warrant_dir(alias)?.join(DIR))
}

/// The Warrant directory whose journal records `alias`'s questions; `None`
/// for an intake key or a ticket, whose question records stand alone.
fn journal_dir(repo: &Repository, alias: &str) -> Option<Utf8PathBuf> {
    if crate::plan::intake::existing_dir(repo, alias).is_some() || item_key(alias) {
        return None;
    }
    repo.warrant_dir(alias).ok()
}

/// Every ticket that has a question directory.
fn item_keys(repo: &Repository) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(repo.root.join(ITEM_DIR)) else {
        return Vec::new();
    };
    let mut keys: Vec<String> = entries
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|k| item_key(k))
        .collect();
    keys.sort();
    keys
}

fn path_of(dir: &Utf8Path, id: &str) -> Utf8PathBuf {
    dir.join(format!("{id}.toml"))
}

/// Every question of one Warrant, by id, tolerating a file that does not
/// parse: one hand-edited record must not hide every other question from the
/// human who has to answer them. The unreadable paths come back beside the
/// questions so a caller can report them.
pub fn load_tolerant(repo: &Repository, alias: &str) -> (Vec<Question>, Vec<String>) {
    let dir = match dir_of(repo, alias) {
        Ok(dir) => dir,
        Err(e) => return (Vec::new(), vec![e.to_string()]),
    };
    let paths = match question_paths(&dir) {
        Ok(paths) => paths,
        Err(_) => return (Vec::new(), vec![dir.to_string()]),
    };
    let mut out = Vec::new();
    let mut bad = Vec::new();
    for path in paths {
        match read_question(&path) {
            Ok(q) => out.push(q),
            Err(_) => bad.push(path.to_string()),
        }
    }
    (out, bad)
}

/// Missing storage means no questions. Every other enumeration error stays visible.
fn question_paths(dir: &Utf8Path) -> Result<Vec<Utf8PathBuf>, RepoError> {
    let metadata = match std::fs::symlink_metadata(dir) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(RepoError::Io {
                context: format!("could not inspect {dir}"),
                source,
            });
        }
    };
    if !metadata.is_dir() {
        return Err(RepoError::Message(format!(
            "{dir}: question store must be a real directory"
        )));
    }
    let entries = std::fs::read_dir(dir).map_err(|source| RepoError::Io {
        context: format!("could not list {dir}"),
        source,
    })?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| RepoError::Io {
            context: format!("could not read entry in {dir}"),
            source,
        })?;
        let path = Utf8PathBuf::from_path_buf(entry.path()).map_err(|_| RepoError::NonUtf8Path)?;
        if path.extension() == Some("toml") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn read_question(path: &Utf8Path) -> Result<Question, RepoError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| RepoError::Io {
        context: format!("could not inspect {path}"),
        source,
    })?;
    if !metadata.is_file() {
        return Err(RepoError::Message(format!(
            "{path}: question record must be a regular file"
        )));
    }
    let text = std::fs::read_to_string(path).map_err(|source| RepoError::Io {
        context: format!("could not read {path}"),
        source,
    })?;
    let q: Question = toml::from_str(&text)
        .map_err(|e| RepoError::Message(format!("{path}: not a question record: {e}")))?;
    if q.schema != SCHEMA {
        return Err(RepoError::Message(format!(
            "{path}: schema is {}, not {SCHEMA}",
            q.schema
        )));
    }
    Ok(q)
}

/// Complete answer context and mutation callers must not skip damaged records.
pub fn load(repo: &Repository, alias: &str) -> Result<Vec<Question>, RepoError> {
    question_paths(&dir_of(repo, alias)?)?
        .iter()
        .map(|p| read_question(p))
        .collect()
}

fn stage_exists(repo: &Repository, alias: &str, stage: &str) -> Result<bool, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let Some(basis) = one.basis else {
        return Ok(false);
    };
    for atom in basis.atoms.iter().filter(|a| a.role == "milestones") {
        let text = String::from_utf8_lossy(&atom.bytes);
        if let Ok(graph) = openwarrant_core::milestones::parse(&text)
            && graph.stages.iter().any(|s| s.id == stage)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn next_id(existing: &[Question]) -> String {
    let highest = existing
        .iter()
        .filter_map(|q| q.id.strip_prefix("Q-").and_then(|n| n.parse::<u32>().ok()))
        .max()
        .unwrap_or(0);
    // Zero-padded to three, and past 999 simply wider: the id stays
    // parseable, which is what the sequence depends on.
    format!("Q-{:03}", highest + 1)
}

fn write_question(path: &Utf8Path, q: &Question, create_new: bool) -> Result<(), RepoError> {
    let body = toml::to_string_pretty(q)
        .map_err(|e| RepoError::Message(format!("could not render {}: {e}", q.id)))?;
    let header = format!(
        "# {SCHEMA}. Who may answer follows the ladder (OW-ADR-0034). An answer\n\
         # informs the work; it is never a disposition, a judgment, or an\n\
         # authorization, and `answered_by` is an attribution, not a proof: a\n\
         # signature binds an ACT, and an answer is not one. `war questions\n\
         # --open` lists what awaits an answer. Regenerated when answered, so\n\
         # a comment added below this header does not survive.\n\n"
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| RepoError::Io {
            context: format!("could not create {parent}"),
            source,
        })?;
    }
    let text = format!("{header}{body}");
    if create_new {
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|source| RepoError::Io {
                context: format!("could not create {path}"),
                source,
            })?;
        f.write_all(text.as_bytes())
            .map_err(|source| RepoError::Io {
                context: format!("could not write {path}"),
                source,
            })?;
    } else {
        std::fs::write(path, text).map_err(|source| RepoError::Io {
            context: format!("could not write {path}"),
            source,
        })?;
    }
    Ok(())
}

/// The `question.asked` payload: what its journal key is made of.
fn asked_payload(stage: &str, id: &str, blocking: bool) -> String {
    serde_json::json!({ "stage": stage, "question": id, "blocking": blocking }).to_string()
}

/// What `war note <id> --ask` and `war plan ask` were given.
#[derive(Debug, Clone, Copy, Default)]
pub struct Asking<'a> {
    /// A Warrant's stage, a ticket's item, or empty for the whole ticket.
    pub stage: &'a str,
    pub question: &'a str,
    pub recommended: &'a str,
    pub blocking: bool,
    pub needs_human: bool,
    /// `--as`: who asks.
    pub actor: Option<&'a str>,
}

/// Whether a ticket `alias` exists and holds `stage` (empty, or one of its
/// items); `None` when the ticket store cannot be read.
fn ticket_holds(repo: &Repository, alias: &str, stage: &str) -> Option<bool> {
    let store = crate::ticket::Store::open(repo, None).ok()?;
    let (tickets, _) = store.load_all().ok()?;
    Some(tickets.iter().any(|t| {
        t.id() == alias && (stage.is_empty() || stage == alias || t.checklist.ids().contains(stage))
    }))
}

/// `war note <id> --ask "<question>"` (and `war plan ask`): anyone's half.
/// The question is asked of a party ([`crate::hotline::party`]), who hears it.
pub fn ask(repo: &Repository, alias: &str, a: &Asking) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let stage = if a.stage == alias { "" } else { a.stage };
    if a.question.trim().is_empty() {
        return Err(RepoError::Message(
            "question.empty: a question with no text cannot be answered".to_owned(),
        ));
    }
    if item_key(alias) {
        match ticket_holds(repo, alias, stage) {
            Some(true) => {}
            Some(false) => {
                return Err(RepoError::Message(format!(
                    "question.unknown-stage: no ticket {alias}{}, so nothing would find this \
                     question",
                    if stage.is_empty() {
                        String::new()
                    } else {
                        format!(" with item {stage}")
                    }
                )));
            }
            None => {
                return Err(RepoError::Message(
                    "question.unknown-stage: the ticket store cannot be read".to_owned(),
                ));
            }
        }
    } else if !stage_exists(repo, alias, stage)? {
        return Err(RepoError::Message(format!(
            "question.unknown-stage: {alias} declares no stage {stage:?}, so nothing would find \
             this question. Ask against a stage in its milestones atom"
        )));
    }
    let journal = journal_dir(repo, alias);
    let dir = dir_of(repo, alias)?;
    let existing = load(repo, alias)?;
    let asker = crate::hotline::asker(repo, a.actor);
    let asked_by = asker.uri();
    let (question, recommended) = (a.question.trim(), a.recommended.trim());
    // OW-WAR-0130 (§67.4), before the first write: the same question, asked
    // by the same actor against the same stage, is a retry. Its journal key
    // names the question already on file; held by the same actor it replays
    // and writes nothing, held by another it is a conflict and writes nothing.
    if let Some(prior) = existing.iter().find(|q| {
        q.stage == stage
            && q.question == question
            && q.blocking == a.blocking
            && q.recommended == recommended
            && q.asked_by == asked_by
    }) {
        if let Some(jdir) = &journal {
            let payload = asked_payload(stage, &prior.id, a.blocking);
            if let crate::journal_cmd::Prior::Conflict { recorded_by } =
                crate::journal_cmd::already_recorded(jdir, EVENT_ASKED, &payload, &asked_by)?
            {
                report.push(crate::journal_cmd::conflict(
                    repo.relative(&jdir.join(crate::journal_cmd::FILE)),
                    EVENT_ASKED,
                    &recorded_by,
                    &asked_by,
                ));
                return Ok(report);
            }
        }
        report.push(Diagnostic::pass(
            "question.replayed",
            format!(
                "{alias}/{} already asks this{} → {}; an equivalent retry replays and writes \
                 nothing",
                prior.id,
                match &prior.answer {
                    Some(a) => format!(", answered by {}", a.answered_by),
                    None => " and awaits an answer".to_owned(),
                },
                repo.relative(&path_of(&dir, &prior.id))
            ),
        ));
        return Ok(report);
    }
    let mut q = Question {
        schema: SCHEMA.to_owned(),
        id: next_id(&existing),
        warrant: alias.to_owned(),
        stage: stage.to_owned(),
        asked_by,
        asked_at: crate::gate_cmd::receipt::now_rfc3339_public(),
        question: question.to_owned(),
        blocking: a.blocking,
        recommended: recommended.to_owned(),
        answer: None,
        redraft: String::new(),
        needs_human: a.needs_human,
        ask_of: String::new(),
    };
    let person = crate::hotline::needs_person(repo, &q);
    let (party, why) = crate::hotline::party(repo, alias, &asker.name, person.is_some());
    q.ask_of.clone_from(&party);
    let path = path_of(&dir, &q.id);
    write_question(&path, &q, true)?;
    if let Some(jdir) = &journal
        && let Some(uuid) = repo
            .load_warrant(jdir)?
            .validated
            .as_ref()
            .map(|v| v.uuid.to_string())
    {
        crate::journal_cmd::record(
            jdir,
            &uuid,
            EVENT_ASKED,
            &q.asked_by,
            &asked_payload(stage, &q.id, a.blocking),
        )?;
    }
    if let Ok(store) = crate::ticket::Store::open(repo, a.actor)
        && let Err(e) = crate::hotline::post(
            &store,
            crate::hotline::Posted {
                schema: String::new(),
                id: String::new(),
                kind: "asked".to_owned(),
                to: party.clone(),
                from: asker.name.clone(),
                item: alias.to_owned(),
                question: q.id.clone(),
                text: q.question.clone(),
                answer: String::new(),
                root: repo.root.to_string(),
                at: q.asked_at.clone(),
            },
        )
    {
        report.push(Diagnostic::warn(
            "notices.unavailable",
            alias.to_owned(),
            format!("the question is recorded, but its notice was not posted: {e}"),
        ));
    }
    report.push(Diagnostic::pass(
        "question.asked",
        format!(
            "{alias} {}{} asked of {party} ({why}{}) → {}",
            q.id,
            if a.blocking { " (blocking)" } else { "" },
            person
                .map(|p| format!("; a person answers: {p}"))
                .unwrap_or_default(),
            repo.relative(&path)
        ),
    ));
    report.note(format!(
        "Answer: `{}`. Nothing here authorizes anything.",
        crate::hotline::answer_command(alias, &q.id)
    ));
    Ok(report)
}

/// `war note <id> --answer Q-nnn "<answer>"` (and `war plan answer`): who may
/// answer follows the ladder ([`crate::hotline`]).
pub fn answer(
    repo: &Repository,
    alias: &str,
    id: &str,
    answer_text: &str,
    actor: Option<&str>,
) -> Result<Report, RepoError> {
    let mut report = Report::default();
    if answer_text.trim().is_empty() {
        report.push(Diagnostic::error(
            "question.empty-answer",
            format!("{alias}/{id}"),
            "an answer with no text is not an answer".to_owned(),
        ));
        return Ok(report);
    }
    let who = match crate::hotline::answerer(repo, actor) {
        Ok(who) => who,
        Err((rule, message)) => {
            report.push(Diagnostic::error(&rule, format!("{alias}/{id}"), message));
            return Ok(report);
        }
    };
    // A question asked in another worktree of this clone is answered where
    // it was asked: its notice names the checkout that holds the record.
    let here = load(repo, alias)?.iter().any(|q| q.id == id);
    if !here
        && let Ok(store) = crate::ticket::Store::open(repo, actor)
        && let Some(p) = crate::hotline::find(&store, alias, id)
        && p.root != repo.root.as_str()
        && let Ok(there) = Repository::discover(Some(Utf8PathBuf::from(&p.root)))
    {
        return answer_as(&there, alias, id, answer_text, &who, actor);
    }
    answer_as(repo, alias, id, answer_text, &who, actor)
}

fn answer_as(
    repo: &Repository,
    alias: &str,
    id: &str,
    answer_text: &str,
    who: &crate::hotline::Who,
    actor: Option<&str>,
) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let mut refuse = |rule: &str, message: String| {
        report.push(Diagnostic::error(rule, format!("{alias}/{id}"), message));
    };
    let dir = dir_of(repo, alias)?;
    let existing = load(repo, alias)?;
    let Some(mut q) = existing.iter().find(|q| q.id == id).cloned() else {
        refuse(
            "question.unknown",
            format!("no question {id} under {}", repo.relative(&dir)),
        );
        return Ok(report);
    };
    if let Some(prior) = &q.answer {
        refuse(
            "question.answered",
            format!(
                "already answered by {} at {}: {:?}. An answer is not edited; ask a new question",
                prior.answered_by, prior.answered_at, prior.answer
            ),
        );
        return Ok(report);
    }
    // The ladder: an Order, signed text, an intake question or
    // `--needs-human` waits on a person (§27.2); below that, any actor.
    if who.kind == crate::hotline::Kind::Agent
        && let Some(why) = crate::hotline::needs_person(repo, &q)
    {
        refuse(
            "question.agent",
            format!(
                "{:?} is an agent and {why}: a person answers it. Nothing was written",
                who.name
            ),
        );
        return Ok(report);
    }
    // Whoever asked has closed the loop it opened if it answers: the same
    // defect as verifying its own work (§46).
    let asker = crate::hotline::bare(&q.asked_by).to_owned();
    if asker == who.name {
        refuse(
            "question.own",
            format!(
                "{:?} asked {id}; another actor answers it. Nothing was written",
                who.name
            ),
        );
        return Ok(report);
    }
    q.answer = Some(Answer {
        answered_by: who.uri(),
        answered_at: crate::gate_cmd::receipt::now_rfc3339_public(),
        answer: answer_text.trim().to_owned(),
        kind: who.kind.word().to_owned(),
    });
    let path = path_of(&dir, &q.id);
    write_question(&path, &q, false)?;
    if !q.redraft.is_empty() {
        report.push(Diagnostic::pass(
            "question.answered",
            format!(
                "{alias}/{id} answered by {}; nothing is drafted until the input is",
                who.name
            ),
        ));
        report.note(format!("Draft it again with its answers: `{}`", q.redraft));
        return Ok(report);
    }
    if let Some(jdir) = journal_dir(repo, alias)
        && let Some(uuid) = repo
            .load_warrant(&jdir)?
            .validated
            .as_ref()
            .map(|v| v.uuid.to_string())
    {
        crate::journal_cmd::record(
            &jdir,
            &uuid,
            EVENT_ANSWERED,
            &who.uri(),
            &serde_json::json!({ "stage": q.stage, "question": q.id }).to_string(),
        )?;
    }
    if let Ok(store) = crate::ticket::Store::open(repo, actor)
        && let Err(e) = crate::hotline::post(
            &store,
            crate::hotline::Posted {
                schema: String::new(),
                id: String::new(),
                kind: "answered".to_owned(),
                to: asker.clone(),
                from: who.name.clone(),
                item: alias.to_owned(),
                question: q.id.clone(),
                text: q.question.clone(),
                answer: answer_text.trim().to_owned(),
                root: repo.root.to_string(),
                at: crate::gate_cmd::receipt::now_rfc3339_public(),
            },
        )
    {
        report.push(Diagnostic::warn(
            "notices.unavailable",
            alias.to_owned(),
            format!("the answer is recorded, but its notice was not posted: {e}"),
        ));
    }
    report.push(Diagnostic::pass(
        "question.answered",
        format!(
            "{alias}/{id} answered by {} ({}); {asker} is told",
            who.name,
            who.kind.word()
        ),
    ));
    Ok(report)
}

/// `war plan questions [--open]`: one queue across every Warrant, blocking first.
pub fn list(
    repo: &Repository,
    alias: Option<&str>,
    open_only: bool,
) -> Result<(Report, QuestionList), RepoError> {
    let mut report = Report::default();
    let aliases: Vec<String> = match alias {
        Some(a) => vec![a.to_owned()],
        None => repo
            .warrant_dirs()?
            .iter()
            .filter_map(|d| d.file_name().map(ToOwned::to_owned))
            .chain(crate::plan::intake::keys(repo))
            .chain(item_keys(repo))
            .collect(),
    };
    let mut questions = Vec::new();
    for a in &aliases {
        let (qs, unreadable) = load_tolerant(repo, a);
        for path in unreadable {
            report.push(Diagnostic::error(
                "question.malformed",
                path.clone(),
                format!("{a}: question storage is unreadable or invalid; displayed counts cover only readable records"),
            ));
        }
        for q in qs {
            if open_only && !q.is_open() {
                continue;
            }
            questions.push(q);
        }
    }
    // Blocking and open first: that is the order a human should answer in.
    questions.sort_by(|a, b| {
        let key = |q: &Question| (!(q.is_open() && q.blocking), !q.is_open());
        key(a)
            .cmp(&key(b))
            .then_with(|| (&a.warrant, &a.id).cmp(&(&b.warrant, &b.id)))
    });
    let open = questions.iter().filter(|q| q.is_open()).count();
    let blocking_open = questions
        .iter()
        .filter(|q| q.is_open() && q.blocking)
        .count();
    let list = QuestionList {
        schema: LIST_SCHEMA.to_owned(),
        answered: questions.len() - open,
        open,
        blocking_open,
        questions,
    };
    report.note(format!(
        "{} open ({} blocking), {} answered",
        list.open, list.blocking_open, list.answered
    ));
    Ok((report, list))
}

/// Queue projections have no diagnostic field; do not project an incomplete queue.
pub fn complete_list(repo: &Repository, open_only: bool) -> Result<QuestionList, RepoError> {
    let (report, list) = list(repo, None, open_only)?;
    if !report.is_ready() {
        return Err(RepoError::Message(
            "question store is incomplete; run war plan questions for diagnostics".to_owned(),
        ));
    }
    Ok(list)
}

/// What the performer of a stage reads before it starts.
pub fn answers_for(
    repo: &Repository,
    alias: &str,
    stage: Option<&str>,
) -> Result<Vec<Question>, RepoError> {
    Ok(load(repo, alias)?
        .into_iter()
        .filter(|q| stage.is_none_or(|s| q.stage == s))
        .filter(|q| !q.is_open())
        .collect())
}

/// Record a drafter's blocker for an input that has no Warrant yet
/// (OW-WAR-0141). The agent's half, like `ask`: an existing record with the
/// same id is left as it is — asked once, answered once.
pub fn record_intake(
    repo: &Repository,
    key: &str,
    id: &str,
    question: &str,
    asked_by: &str,
    redraft: &str,
) -> Result<Utf8PathBuf, RepoError> {
    let dir = crate::plan::intake::store_dir(repo, key).join(DIR);
    let path = path_of(&dir, id);
    if path.exists() {
        return Ok(path);
    }
    let q = Question {
        schema: SCHEMA.to_owned(),
        id: id.to_owned(),
        warrant: key.to_owned(),
        stage: INTAKE_STAGE.to_owned(),
        asked_by: asked_by.to_owned(),
        asked_at: crate::gate_cmd::receipt::now_rfc3339_public(),
        question: question.trim().to_owned(),
        blocking: true,
        recommended: String::new(),
        answer: None,
        redraft: redraft.to_owned(),
        needs_human: false,
        ask_of: crate::hotline::PERSON.to_owned(),
    };
    write_question(&path, &q, true)?;
    Ok(path)
}

/// `war view next`'s rows for intake questions: each open one is a human's
/// `answer` act, and its `why` names the command that drafts the input
/// again. Never a signing act, and never an agent's.
pub fn intake_actions(repo: &Repository) -> Vec<crate::next::Action> {
    let mut out = Vec::new();
    for key in crate::plan::intake::keys(repo) {
        let (qs, _) = load_tolerant(repo, &key);
        for q in qs.into_iter().filter(Question::is_open) {
            out.push(crate::next::Action {
                actor: crate::next::Actor::Human,
                warrant: key.clone(),
                action: "answer".to_owned(),
                command: format!("war plan answer {key} {} \"<answer>\" --as <actor>", q.id),
                why: format!(
                    "{}: {} — no Warrant is drafted until it is answered; then `{}`",
                    q.id, q.question, q.redraft
                ),
                judged: None,
            });
        }
    }
    out
}

#[must_use]
pub fn render(list: &QuestionList) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{} open question(s), {} blocking; {} answered\n",
        list.open, list.blocking_open, list.answered
    );
    for q in &list.questions {
        let mark = match (&q.answer, q.blocking) {
            (Some(_), _) => "answered",
            (None, true) => "BLOCKING",
            (None, false) => "open    ",
        };
        let _ = writeln!(out, "  {mark}  {} {}  {}", q.warrant, q.id, q.question);
        if !q.recommended.is_empty() && q.is_open() {
            let _ = writeln!(out, "            recommended: {}", q.recommended);
        }
        match &q.answer {
            Some(a) => {
                let _ = writeln!(out, "            {} → {}", a.answered_by, a.answer);
            }
            None => {
                let _ = writeln!(
                    out,
                    "            {}",
                    crate::hotline::answer_command(&q.warrant, &q.id)
                );
            }
        }
        if !q.redraft.is_empty() {
            let _ = writeln!(out, "            then: {}", q.redraft);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(id: &str, blocking: bool, answered: bool) -> Question {
        Question {
            schema: SCHEMA.to_owned(),
            id: id.to_owned(),
            warrant: "OW-WAR-0001".to_owned(),
            stage: "STAGE-001".to_owned(),
            asked_by: "agent://claude".to_owned(),
            asked_at: "2026-09-12T00:00:00Z".to_owned(),
            question: "x".to_owned(),
            blocking,
            recommended: String::new(),
            answer: answered.then(|| Answer {
                answered_by: "person://brian".to_owned(),
                answered_at: "2026-09-12T00:01:00Z".to_owned(),
                answer: "y".to_owned(),
                kind: String::new(),
            }),
            redraft: String::new(),
            needs_human: false,
            ask_of: String::new(),
        }
    }

    #[test]
    fn ids_continue_from_the_highest_recorded() {
        assert_eq!(next_id(&[]), "Q-001");
        assert_eq!(
            next_id(&[q("Q-001", false, false), q("Q-009", false, false)]),
            "Q-010"
        );
        // A malformed id does not reset the sequence onto an existing file.
        assert_eq!(
            next_id(&[q("Q-003", false, false), q("nonsense", false, false)]),
            "Q-004"
        );
    }

    #[test]
    fn a_blocking_open_question_sorts_above_everything() {
        let mut v = [
            q("Q-001", false, true),
            q("Q-002", false, false),
            q("Q-003", true, false),
        ];
        v.sort_by(|a, b| {
            let key = |q: &Question| (!(q.is_open() && q.blocking), !q.is_open());
            key(a)
                .cmp(&key(b))
                .then_with(|| (&a.warrant, &a.id).cmp(&(&b.warrant, &b.id)))
        });
        assert_eq!(
            v.iter().map(|q| q.id.as_str()).collect::<Vec<_>>(),
            ["Q-003", "Q-002", "Q-001"]
        );
    }
}
