// SPDX-License-Identifier: Apache-2.0
//! Answer-aware interview projection over retained planning/question records.
//! Counts describe recorded questions, never unobserved external conversations.
use crate::{
    diagnostic::{Diagnostic, Report, Severity},
    plan, questions,
    repo::{RepoError, Repository},
};
use camino::Utf8Path;
use openwarrant_core::drafting_v2::DraftProposalV2;
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
};

pub const SCHEMA: &str = "oh.war/interview-view/v1";

#[derive(Debug, Serialize)]
pub struct PendingQuestion {
    pub id: String,
    pub question: String,
    pub stage: String,
    pub expected_information_gain: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct InterviewView {
    pub schema: &'static str,
    pub target: String,
    pub recorded_question_count: Option<usize>,
    pub questions: Vec<PendingQuestion>,
    pub coverage: &'static str,
}

fn optional_bytes(path: &Utf8Path, report: &mut Report) -> Option<Vec<u8>> {
    match std::fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            report.push(Diagnostic::warn("interview.metadata-unreadable", path.as_str(),
                format!("optional interview metadata is unreadable: {error}; retained questions remain visible")));
            None
        }
    }
}

pub fn view(repo: &Repository, target: &str) -> Result<(Report, InterviewView), RepoError> {
    let metadata = match plan::intake::existing_dir(repo, target) {
        Some(dir) => dir,
        None => repo.warrant_dir(target)?.join("plan"),
    };
    let (mut report, list) = questions::list(repo, Some(target), false)?;
    let complete = !report
        .diagnostics
        .iter()
        .any(|d| matches!(d.severity, Severity::Error | Severity::Unknown));
    let count = complete.then_some(list.questions.len());
    let mut supplied = BTreeSet::new();
    if let Some(bytes) = optional_bytes(&metadata.join("request.json"), &mut report) {
        match serde_json::from_slice::<plan::Request>(&bytes) {
            Ok(request) if request.api_version == plan::REQUEST_API => supplied.extend(request.answers.into_keys()),
            _ => report.push(Diagnostic::warn("interview.request-invalid", repo.relative(&metadata.join("request.json")),
                "supplied-answer metadata is invalid; only explicit question-record answers are used")),
        }
    }
    let mut priorities = BTreeMap::new();
    if let Some(bytes) = optional_bytes(&metadata.join("proposal.json"), &mut report) {
        match serde_json::from_slice::<DraftProposalV2>(&bytes) {
            Ok(proposal) if proposal.validate().is_ok() => {
                for q in proposal.unresolved_questions { priorities.insert(q.id.clone(), q); }
            }
            _ => report.push(Diagnostic::warn("interview.proposal-invalid", repo.relative(&metadata.join("proposal.json")),
                "priority metadata is invalid; questions without known priorities are ordered by id")),
        }
    }
    let mut pending: Vec<_> = list
        .questions
        .into_iter()
        .filter(|q| q.is_open() && q.blocking && !supplied.contains(&q.id))
        .map(|q| {
            let gain = priorities
                .get(&q.id)
                .filter(|p| p.question == q.question && p.removes_blocker == q.blocking)
                .map(|p| p.expected_information_gain);
            PendingQuestion {
                id: q.id,
                question: q.question,
                stage: q.stage,
                expected_information_gain: gain,
            }
        })
        .collect();
    pending.sort_by(|a, b| {
        b.expected_information_gain
            .cmp(&a.expected_information_gain)
            .then_with(|| a.id.cmp(&b.id))
    });
    report.note("Only retained question records are counted; external conversations and contract completeness are not measured. This projection authorizes nothing.");
    Ok((
        report,
        InterviewView {
            schema: SCHEMA,
            target: target.to_owned(),
            recorded_question_count: count,
            questions: pending,
            coverage: "retained question records only; external clarifications unobserved",
        },
    ))
}

#[must_use]
pub fn render(view: &InterviewView) -> String {
    let mut out = format!(
        "# Interview: {}\n\n{} unanswered blocker(s). Recorded questions: {}.\n",
        view.target,
        view.questions.len(),
        view.recorded_question_count
            .map_or_else(|| "UNKNOWN".to_owned(), |n| n.to_string())
    );
    for q in &view.questions {
        let _ = writeln!(
            out,
            "\n{}: {}\nAnswer: `war plan answer {} {} \"<answer>\" --as <actor>`",
            q.id, q.question, view.target, q.id
        );
    }
    out
}
