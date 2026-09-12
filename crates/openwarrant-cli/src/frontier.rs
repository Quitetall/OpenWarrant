// SPDX-License-Identifier: AGPL-3.0-or-later

//! `war frontier` (OW-WAR-0068): the stages that can start now.
//!
//! After mattpocock/skills `to-tickets` and `wayfinder` (MIT): a ticket is
//! unblocked when every ticket blocking it is closed, and the frontier is the
//! open, unblocked, unclaimed set. Here the ticket is a Stage, the blocking
//! edge is its milestone's `depends_on`, "closed" is a milestone whose every
//! obligation an admissible verification established, "claimed" is a
//! `dispatch.compiled` journal event naming the stage, and "done" is a
//! `submission.recorded` event naming it. Nothing here is a status claim: it
//! is derived from the same records `war resolve --dry-run` reads.
//!
//! Three choices, stated: `done` wins over `blocked` (a recorded submission
//! is history, whatever the graph says now); a milestone with no obligations
//! never completes, so everything behind it stays blocked until someone
//! writes the obligation (fail closed, and reported); a stage cited by more
//! than one milestone is one row, blocked if any of them waits.

use std::collections::{BTreeMap, BTreeSet};

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
    /// Its milestone waits on another milestone that is not complete.
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
    /// Milestones this one's milestone waits on and that are not complete.
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

fn stage_events(dir: &camino::Utf8Path) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut claimed = BTreeSet::new();
    let mut done = BTreeSet::new();
    if let Ok(j) = crate::journal_cmd::load(dir) {
        for e in &j.events {
            let stage = serde_json::from_str::<serde_json::Value>(&e.payload)
                .ok()
                .and_then(|v| v.get("stage").and_then(|s| s.as_str()).map(str::to_owned));
            let Some(stage) = stage else { continue };
            match e.event_type.as_str() {
                "dispatch.compiled" => {
                    claimed.insert(stage);
                }
                "submission.recorded" => {
                    done.insert(stage);
                }
                _ => {}
            }
        }
    }
    (claimed, done)
}

/// The frontier of one Warrant, or of every unresolved Warrant.
pub fn run(repo: &Repository, alias: Option<&str>) -> Result<(Report, Frontier), RepoError> {
    let mut report = Report::default();
    let dirs = match alias {
        Some(a) => vec![repo.warrant_dir(a)?],
        None => repo.warrant_dirs()?,
    };
    let mut rows = Vec::new();
    for dir in dirs {
        let Ok(one) = repo.load_warrant(&dir) else {
            continue;
        };
        let alias = dir.file_name().unwrap_or_default().to_owned();
        if repo.load_resolution(&dir)?.is_some() {
            continue;
        }
        let Some(basis) = &one.basis else { continue };
        let Some(text) = basis
            .atoms
            .iter()
            .find(|a| a.role == "milestones")
            .and_then(|a| String::from_utf8(a.bytes.clone()).ok())
        else {
            continue;
        };
        let Ok(graph) = openwarrant_core::milestones::parse(&text) else {
            report.push(Diagnostic::warn(
                "frontier.milestones",
                repo.relative(&dir),
                format!(
                    "{alias}: the milestones atom does not parse; `war check {alias}` names why"
                ),
            ));
            continue;
        };
        let established: BTreeSet<String> = crate::resolve::assess(repo, &one)
            .map(|a| a.established.into_iter().collect())
            .unwrap_or_default();
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
        let (claimed, done) = stage_events(&dir);
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
            let state = if done.contains(&sid) {
                StageState::Done
            } else if !waiting.is_empty() {
                StageState::Blocked
            } else if claimed.contains(&sid) {
                StageState::Claimed
            } else {
                StageState::Open
            };
            rows.push(Row {
                warrant: alias.clone(),
                stage: sid.clone(),
                title: stage.title.clone().unwrap_or_default(),
                milestone: milestones.join("+"),
                executor_kind: stage.executor_kind.to_string(),
                state,
                waiting_on: if state == StageState::Blocked {
                    waiting.into_iter().collect()
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
