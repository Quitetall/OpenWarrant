// SPDX-License-Identifier: Apache-2.0

//! The hotline (OW-WAR-0069): an agent asks, a human answers, both on record.
//!
//! A performing agent that needs a decision has two bad options today: guess,
//! or stop and hope someone reads its transcript. This gives it a third: ask,
//! in a file the owner can answer from anywhere, so one person can clear the
//! blocking questions of many concurrent agents in one pass.
//!
//! The asymmetry is the point, and it is the same one §27.2 draws everywhere
//! else. Asking is an agent's act and costs nothing. Answering is the human's,
//! checked through `docs/authority/roles.toml` by actor kind, so an agent that
//! answers its own question is refused by name and writes nothing.
//!
//! A question is `questions/Q-nnn.toml` beside the Warrant's other records,
//! with a `question.asked` journal event; an answer appends to the same file
//! and records `question.answered`. Neither is a disposition, a judgment, or
//! an authorization: an answer informs work, it never accepts it.

use std::fmt::Write as _;

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_core::authority::ActorKind;
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    pub answered_by: String,
    pub answered_at: String,
    pub answer: String,
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

fn dir_of(repo: &Repository, alias: &str) -> Result<Utf8PathBuf, RepoError> {
    Ok(repo.warrant_dir(alias)?.join(DIR))
}

fn path_of(dir: &Utf8Path, id: &str) -> Utf8PathBuf {
    dir.join(format!("{id}.toml"))
}

/// Every question of one Warrant, by id, tolerating a file that does not
/// parse: one hand-edited record must not hide every other question from the
/// human who has to answer them. The unreadable paths come back beside the
/// questions so a caller can report them.
pub fn load_tolerant(repo: &Repository, alias: &str) -> (Vec<Question>, Vec<String>) {
    match load(repo, alias) {
        Ok(qs) => (qs, Vec::new()),
        Err(_) => {
            let Ok(dir) = dir_of(repo, alias) else {
                return (Vec::new(), Vec::new());
            };
            let Ok(entries) = std::fs::read_dir(&dir) else {
                return (Vec::new(), Vec::new());
            };
            let mut paths: Vec<Utf8PathBuf> = entries
                .filter_map(Result::ok)
                .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
                .filter(|p| p.extension() == Some("toml"))
                .collect();
            paths.sort();
            let (mut out, mut bad) = (Vec::new(), Vec::new());
            for path in paths {
                match std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|t| toml::from_str::<Question>(&t).ok())
                    .filter(|q| q.schema == SCHEMA)
                {
                    Some(q) => out.push(q),
                    None => bad.push(path.to_string()),
                }
            }
            (out, bad)
        }
    }
}

/// Every question of one Warrant, by id. Strict: `ask` allocates the next id
/// from this, so a file it cannot read is an error there rather than a silent
/// gap in the sequence.
pub fn load(repo: &Repository, alias: &str) -> Result<Vec<Question>, RepoError> {
    let dir = dir_of(repo, alias)?;
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(Vec::new());
    };
    let mut paths: Vec<Utf8PathBuf> = entries
        .filter_map(Result::ok)
        .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
        .filter(|p| p.extension() == Some("toml"))
        .collect();
    paths.sort();
    let mut out = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(&path).map_err(|source| RepoError::Io {
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
        out.push(q);
    }
    Ok(out)
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
        "# {SCHEMA}. Asked by an agent, answered by a human (§27.2). An answer\n\
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

/// `war ask <alias> <stage> "<question>"`: the agent's half.
pub fn ask(
    repo: &Repository,
    alias: &str,
    stage: &str,
    question: &str,
    recommended: &str,
    blocking: bool,
) -> Result<Report, RepoError> {
    let mut report = Report::default();
    if question.trim().is_empty() {
        return Err(RepoError::Message(
            "question.empty: a question with no text cannot be answered".to_owned(),
        ));
    }
    if !stage_exists(repo, alias, stage)? {
        return Err(RepoError::Message(format!(
            "question.unknown-stage: {alias} declares no stage {stage:?}, so nothing would find \
             this question. Ask against a stage in its milestones atom"
        )));
    }
    let dir = repo.warrant_dir(alias)?;
    let existing = load(repo, alias)?;
    let q = Question {
        schema: SCHEMA.to_owned(),
        id: next_id(&existing),
        warrant: alias.to_owned(),
        stage: stage.to_owned(),
        asked_by: format!("agent://{}", repo.performer()),
        asked_at: crate::gate_cmd::receipt::now_rfc3339_public(),
        question: question.trim().to_owned(),
        blocking,
        recommended: recommended.trim().to_owned(),
        answer: None,
    };
    let path = path_of(&dir.join(DIR), &q.id);
    write_question(&path, &q, true)?;
    if let Some(uuid) = repo
        .load_warrant(&dir)?
        .validated
        .as_ref()
        .map(|v| v.uuid.to_string())
    {
        crate::journal_cmd::record(
            &dir,
            &uuid,
            EVENT_ASKED,
            &q.asked_by,
            &serde_json::json!({ "stage": stage, "question": q.id, "blocking": blocking })
                .to_string(),
        )?;
    }
    report.push(Diagnostic::pass(
        "question.asked",
        format!(
            "{alias}/{} asked against {stage}{} → {}",
            q.id,
            if blocking { " (blocking)" } else { "" },
            repo.relative(&path)
        ),
    ));
    report.note(format!(
        "A human answers it: `war answer {alias} {} \"<answer>\" --as <actor>`. \
         Nothing here authorizes anything.",
        q.id
    ));
    Ok(report)
}

/// `war answer <alias> <id> "<answer>" --as <actor>`: the human's half.
pub fn answer(
    repo: &Repository,
    alias: &str,
    id: &str,
    answer_text: &str,
    actor: &str,
) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let mut refuse = |rule: &str, message: String| {
        report.push(Diagnostic::error(rule, format!("{alias}/{id}"), message));
    };
    if answer_text.trim().is_empty() {
        refuse(
            "question.empty-answer",
            "an answer with no text is not an answer".to_owned(),
        );
        return Ok(report);
    }
    let register = repo.load_authority_register()?;
    let Some(assignment) = register.actor(actor) else {
        refuse(
            "question.unknown-actor",
            format!("{actor:?} holds no role assignment in docs/authority/roles.toml"),
        );
        return Ok(report);
    };
    // The whole asymmetry, in one check. An agent may ask anything and answer
    // nothing: an agent that answers its own question has closed the loop it
    // opened, which is the same defect as verifying its own work (§46).
    if assignment.actor_kind == ActorKind::Agent {
        refuse(
            "question.agent",
            format!("{actor:?} is an agent. An agent asks; a human answers. Nothing was written"),
        );
        return Ok(report);
    }
    let dir = repo.warrant_dir(alias)?;
    let existing = load(repo, alias)?;
    let Some(mut q) = existing.iter().find(|q| q.id == id).cloned() else {
        refuse(
            "question.unknown",
            format!("no question {id} under {}", repo.relative(&dir.join(DIR))),
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
    q.answer = Some(Answer {
        answered_by: format!("person://{actor}"),
        answered_at: crate::gate_cmd::receipt::now_rfc3339_public(),
        answer: answer_text.trim().to_owned(),
    });
    let path = path_of(&dir.join(DIR), &q.id);
    write_question(&path, &q, false)?;
    if let Some(uuid) = repo
        .load_warrant(&dir)?
        .validated
        .as_ref()
        .map(|v| v.uuid.to_string())
    {
        crate::journal_cmd::record(
            &dir,
            &uuid,
            EVENT_ANSWERED,
            &format!("person://{actor}"),
            &serde_json::json!({ "stage": q.stage, "question": q.id }).to_string(),
        )?;
    }
    report.push(Diagnostic::pass(
        "question.answered",
        format!("{alias}/{id} answered by {actor}; {} knows", q.stage),
    ));
    Ok(report)
}

/// `war questions [--open]`: one queue across every Warrant, blocking first.
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
            .collect(),
    };
    let mut questions = Vec::new();
    for a in &aliases {
        let (qs, unreadable) = load_tolerant(repo, a);
        for path in unreadable {
            report.push(Diagnostic::error(
                "question.malformed",
                path.clone(),
                format!("{a}: this question record does not parse, so nobody can answer it; the rest are listed"),
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

/// What the performer of a stage reads before it starts.
pub fn answers_for(
    repo: &Repository,
    alias: &str,
    stage: Option<&str>,
) -> Result<Vec<Question>, RepoError> {
    Ok(load_tolerant(repo, alias)
        .0
        .into_iter()
        .filter(|q| stage.is_none_or(|s| q.stage == s))
        .filter(|q| !q.is_open())
        .collect())
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
                    "            war answer {} {} \"<answer>\" --as <actor>",
                    q.warrant, q.id
                );
            }
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
            }),
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
