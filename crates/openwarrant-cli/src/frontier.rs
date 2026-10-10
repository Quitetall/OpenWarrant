// SPDX-License-Identifier: Apache-2.0

//! `war plan frontier` (OW-WAR-0068): the stages that can start now.
//!
//! After mattpocock/skills `to-tickets` and `wayfinder` (MIT): a ticket is
//! unblocked when every ticket blocking it is closed, and the frontier is the
//! open, unblocked, unclaimed set. Here the ticket is a Stage, the blocking
//! edge is its milestone's `depends_on`, "closed" is a milestone whose every
//! obligation an admissible verification established, "claimed" is a
//! `dispatch.compiled` journal event naming the stage, and "done" is a
//! `submission.recorded` event naming it. Nothing here is a status claim: it
//! is derived from the same records `war sign resolve --dry-run` reads.
//!
//! Three choices, stated: `done` wins over `blocked` (a recorded submission
//! is history, whatever the graph says now); a milestone with no obligations
//! never completes, so everything behind it stays blocked until someone
//! writes the obligation (fail closed, and reported); a stage cited by more
//! than one milestone is one row, blocked if any of them waits.
//!
//! A second blocking edge (OW-WAR-0132): a blocking question on the stage that
//! no human has answered. The stage is `blocked` and its `waiting_on` names the
//! question beside any milestone; every other stage is untouched, and an
//! answered question blocks nothing. When the authority register holds no
//! actor who is not an agent, nobody can answer, and each such stage also
//! carries UNKNOWN `question.no-responder` naming `roles.toml`: neither
//! waiting normally nor answered (Law 15).

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};

use serde::Serialize;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StageState {
    /// Unblocked, undispatched: on the frontier.
    Open,
    /// Dispatched (a `dispatch.compiled` event), no submission yet.
    Claimed,
    /// A submission is recorded for it.
    Done,
    /// Its milestone waits on another milestone that is not complete, or a
    /// blocking question on it is unanswered.
    Blocked,
}

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub warrant: String,
    pub stage: String,
    pub title: String,
    pub milestone: String,
    pub executor_kind: String,
    pub state: StageState,
    /// Milestones this one's milestone waits on and that are not complete,
    /// then the blocking questions on it that are unanswered (`Q-nnn`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waiting_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Frontier {
    pub schema: String,
    pub rows: Vec<Row>,
    pub open: usize,
    pub claimed: usize,
    pub done: usize,
    pub blocked: usize,
}

pub const SCHEMA: &str = "oh.war/frontier/v1";

fn stage_events(dir: &camino::Utf8Path) -> Result<(BTreeSet<String>, BTreeSet<String>), RepoError> {
    let mut claimed = BTreeSet::new();
    let mut done = BTreeSet::new();
    let path = dir.join(crate::journal_cmd::FILE);
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((claimed, done)),
        Err(source) => {
            return Err(RepoError::Io {
                context: format!("could not inspect {path}"),
                source,
            });
        }
        Ok(_) => {}
    }
    let metadata = std::fs::metadata(&path).map_err(|source| RepoError::Io {
        context: format!("could not inspect {path}"),
        source,
    })?;
    if !metadata.is_file() {
        return Err(RepoError::Message(format!(
            "{path}: journal must be a regular file"
        )));
    }
    // The legacy loader treats non-files as absent. Admission must distinguish
    // missing history from damaged containers and dangling links.
    let text = std::fs::read_to_string(&path).map_err(|source| RepoError::Io {
        context: format!("could not read {path}"),
        source,
    })?;
    let j =
        crate::journal_cmd::parse(&text).map_err(|e| RepoError::Message(format!("{path}: {e}")))?;
    for e in &j.events {
        if !matches!(
            e.event_type.as_str(),
            "dispatch.compiled" | "submission.recorded"
        ) {
            continue;
        }
        let payload: serde_json::Value = serde_json::from_str(&e.payload).map_err(|_| {
            RepoError::Message(format!(
                "{path}: {} event {} has invalid JSON payload",
                e.event_type, e.id
            ))
        })?;
        let stage = payload
            .get("stage")
            .and_then(serde_json::Value::as_str)
            .filter(|stage| !stage.trim().is_empty())
            .ok_or_else(|| {
                RepoError::Message(format!(
                    "{path}: {} event {} requires a nonempty string stage",
                    e.event_type, e.id
                ))
            })?;
        if e.event_type == "dispatch.compiled" {
            claimed.insert(stage.to_owned());
        } else {
            done.insert(stage.to_owned());
        }
    }
    Ok((claimed, done))
}

/// The frontier of one Warrant, or of every unresolved Warrant.
pub fn run(repo: &Repository, alias: Option<&str>) -> Result<(Report, Frontier), RepoError> {
    match alias {
        // One Warrant: read that one, not the corpus.
        Some(a) => {
            let dir = repo.warrant_dir(a)?;
            frontier_of(
                repo,
                vec![dir],
                &|dir| repo.load_warrant(dir).map(Cow::Owned),
                &|one| crate::resolve::assess(repo, one).map(|a| a.established),
            )
        }
        None => run_with(&crate::corpus::Corpus::new(repo), None),
    }
}

/// [`run`] over a corpus already loaded: each Warrant's assessment is the
/// corpus's, the same one `war status` reads.
pub fn run_with(
    corpus: &crate::corpus::Corpus,
    alias: Option<&str>,
) -> Result<(Report, Frontier), RepoError> {
    let repo = corpus.repo();
    let dirs: Vec<Utf8PathBuf> = match alias {
        Some(a) => vec![repo.warrant_dir(a)?],
        None => corpus.entries()?.iter().map(|e| e.dir.clone()).collect(),
    };
    frontier_of(
        repo,
        dirs,
        &|dir| match corpus.entry_at(dir) {
            Some(e) => e.loaded().map(Cow::Borrowed),
            None => repo.load_warrant(dir).map(Cow::Owned),
        },
        &|one| match corpus.entry_at(&one.dir) {
            Some(e) => e.assessment(repo).map(|a| a.established.clone()),
            None => crate::resolve::assess(repo, one).map(|a| a.established),
        },
    )
}

type LoadFn<'a> = dyn Fn(&Utf8Path) -> Result<Cow<'a, crate::repo::Loaded>, RepoError> + 'a;
type EstablishedFn<'a> = dyn Fn(&crate::repo::Loaded) -> Result<Vec<String>, RepoError> + 'a;

fn frontier_of<'a>(
    repo: &Repository,
    dirs: Vec<Utf8PathBuf>,
    load: &LoadFn<'a>,
    established_of: &EstablishedFn<'a>,
) -> Result<(Report, Frontier), RepoError> {
    let mut report = Report::default();
    let mut rows = Vec::new();
    // Read once, and only if a blocking question needs it.
    let mut responder: Option<Result<bool, String>> = None;
    for dir in dirs {
        let one = load(&dir)?;
        let one: &crate::repo::Loaded = &one;
        let alias = dir.file_name().unwrap_or_default().to_owned();
        if repo.load_resolution(&dir)?.is_some() {
            continue;
        }
        let basis = one.basis.as_ref().ok_or_else(|| {
            RepoError::Message(format!("{alias}: no readable workspace basis for frontier"))
        })?;
        let Some(text) = basis
            .atoms
            .iter()
            .find(|a| a.role == "milestones")
            .and_then(|a| String::from_utf8(a.bytes.clone()).ok())
        else {
            if one
                .validated
                .as_ref()
                .is_some_and(|v| !v.raw.atoms.iter().any(|a| a.role == "milestones"))
            {
                continue;
            }
            return Err(RepoError::Message(format!(
                "{alias}: no readable milestones atom for frontier"
            )));
        };
        let graph = openwarrant_core::milestones::parse(&text)
            .map_err(|e| RepoError::Message(format!("{alias}: invalid milestones: {e}")))?;
        let established: BTreeSet<String> = established_of(one)?.into_iter().collect();
        let complete: BTreeMap<&str, bool> = graph
            .milestones
            .iter()
            .map(|m| {
                (
                    m.id.as_str(),
                    !m.obligation_refs.is_empty()
                        && m.obligation_refs.iter().all(|o| established.contains(o)),
                )
            })
            .collect();
        for m in graph
            .milestones
            .iter()
            .filter(|m| m.obligation_refs.is_empty())
        {
            if graph
                .milestones
                .iter()
                .any(|o| o.depends_on.contains(&m.id))
            {
                report.push(Diagnostic::warn(
                    "frontier.milestone-without-obligations",
                    repo.relative(&dir.join("atoms/45-milestones.yaml")),
                    format!(
                        "{alias}: {} has no obligation_refs, so it never completes and every milestone depending on it stays blocked",
                        m.id
                    ),
                ));
            }
        }
        let (claimed, done) = stage_events(&dir)?;
        let asked = open_blocking(repo, &alias, &mut report);
        // One row per stage: its milestones' waits are unioned.
        let mut per_stage: BTreeMap<String, (Vec<String>, BTreeSet<String>)> = BTreeMap::new();
        for m in &graph.milestones {
            let waiting: Vec<String> = m
                .depends_on
                .iter()
                .filter(|d| !complete.get(d.as_str()).copied().unwrap_or(false))
                .cloned()
                .collect();
            for sid in &m.stage_refs {
                let entry = per_stage.entry(sid.clone()).or_default();
                entry.0.push(m.id.clone());
                entry.1.extend(waiting.iter().cloned());
            }
        }
        for (sid, (milestones, waiting)) in per_stage {
            let Some(stage) = graph.stages.iter().find(|s| s.id == sid) else {
                continue;
            };
            let questions = asked.get(&sid).cloned().unwrap_or_default();
            let mut waiting: Vec<String> = waiting.into_iter().collect();
            waiting.extend(questions.iter().cloned());
            let state = if done.contains(&sid) {
                StageState::Done
            } else if !waiting.is_empty() {
                StageState::Blocked
            } else if claimed.contains(&sid) {
                StageState::Claimed
            } else {
                StageState::Open
            };
            if state == StageState::Blocked && !questions.is_empty() {
                let answerable = responder.get_or_insert_with(|| someone_can_answer(repo));
                if let Some(why) = no_responder(answerable) {
                    report.push(Diagnostic::unknown(
                        "question.no-responder",
                        "docs/authority/roles.toml".to_owned(),
                        format!(
                            "{alias}/{sid} waits on {} and {why}, so nobody can answer:                              `war plan answer` refuses an agent by kind (§27.2). The stage stays                              blocked; it is not waiting normally and it is not answered. A human                              added to docs/authority/roles.toml can answer",
                            questions.join(", ")
                        ),
                    ));
                }
            }
            rows.push(Row {
                warrant: alias.clone(),
                stage: sid.clone(),
                title: stage.title.clone().unwrap_or_default(),
                milestone: milestones.join("+"),
                executor_kind: stage.executor_kind.to_string(),
                state,
                waiting_on: if state == StageState::Blocked {
                    waiting
                } else {
                    vec![]
                },
            });
        }
    }
    rows.sort_by(|a, b| (&a.warrant, &a.stage).cmp(&(&b.warrant, &b.stage)));
    let count = |s: StageState| rows.iter().filter(|r| r.state == s).count();
    let f = Frontier {
        schema: SCHEMA.to_owned(),
        open: count(StageState::Open),
        claimed: count(StageState::Claimed),
        done: count(StageState::Done),
        blocked: count(StageState::Blocked),
        rows,
    };
    report.note(format!(
        "{} open, {} claimed, {} done, {} blocked across {} stage(s)",
        f.open,
        f.claimed,
        f.done,
        f.blocked,
        f.rows.len()
    ));
    Ok((report, f))
}

/// The unanswered blocking questions of one Warrant, by stage. A question
/// store that cannot be read is reported, not taken to hold nothing.
fn open_blocking(
    repo: &Repository,
    alias: &str,
    report: &mut Report,
) -> BTreeMap<String, Vec<String>> {
    let (questions, unreadable) = crate::questions::load_tolerant(repo, alias);
    for path in unreadable {
        report.push(Diagnostic::unknown(
            "frontier.question-unreadable",
            path,
            format!(
                "{alias}: a question record cannot be read, so whether it blocks a stage cannot                  be established; the rows below count only the readable ones"
            ),
        ));
    }
    let mut by_stage: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for q in questions.into_iter().filter(|q| q.blocking && q.is_open()) {
        by_stage.entry(q.stage).or_default().push(q.id);
    }
    by_stage
}

/// Whether the register names an actor `war plan answer` would accept: any
/// assignment whose kind is not `agent` (A-002). `Err` when the register
/// cannot be read at all.
fn someone_can_answer(repo: &Repository) -> Result<bool, String> {
    repo.load_authority_register()
        .map(|r| {
            r.assignments
                .iter()
                .any(|a| a.actor_kind != openwarrant_core::authority::ActorKind::Agent)
        })
        .map_err(|e| e.to_string())
}

/// The clause of the no-responder finding, or `None` when someone can answer.
fn no_responder(answerable: &Result<bool, String>) -> Option<String> {
    match answerable {
        Ok(true) => None,
        Ok(false) => Some(
            "docs/authority/roles.toml holds no actor whose actor_kind is not `agent`".to_owned(),
        ),
        Err(e) => Some(format!("docs/authority/roles.toml cannot be read ({e})")),
    }
}

#[must_use]
pub fn render(f: &Frontier) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "frontier: {} open · {} claimed · {} done · {} blocked\n\n",
        f.open, f.claimed, f.done, f.blocked
    ));
    for r in &f.rows {
        let state = match r.state {
            StageState::Open => "OPEN   ",
            StageState::Claimed => "CLAIMED",
            StageState::Done => "done   ",
            StageState::Blocked => "blocked",
        };
        out.push_str(&format!(
            "  {state}  {:<12} {:<10} {:<4} {:<8} {}{}\n",
            r.warrant,
            r.stage,
            r.milestone,
            r.executor_kind,
            r.title,
            if r.waiting_on.is_empty() {
                String::new()
            } else {
                format!("  (waits on {})", r.waiting_on.join(", "))
            }
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_states_are_distinct_words() {
        for s in [
            StageState::Open,
            StageState::Claimed,
            StageState::Done,
            StageState::Blocked,
        ] {
            let j = serde_json::to_string(&s).unwrap();
            assert!(["\"open\"", "\"claimed\"", "\"done\"", "\"blocked\""].contains(&j.as_str()));
        }
    }
}
