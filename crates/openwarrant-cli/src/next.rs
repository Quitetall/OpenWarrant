// SPDX-License-Identifier: Apache-2.0
//! `war next` — what should happen next in this repository, and whose act it
//! is.
//!
//! Two lists exist already: `war sign --list` (the human's acts) and the
//! corpus projection's `next_actionable` (stages an agent could execute).
//! Neither answers the question an agent asks first: "of everything pending,
//! which is MINE?" This merges them into one ordered list where every action
//! names its actor, and it is asserted over the whole table that no action
//! whose actor is `agent` is a signing act. An agent that reads this cannot be
//! told to authorize, resolve, accept or correct; it can only be told that a
//! human must, and how.

use serde::Serialize;

use crate::repo::{RepoError, Repository};
use crate::sign::{self, Pending};

pub const SCHEMA: &str = "oh.war/next/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    Human,
    Agent,
}

/// What the act's dry run said about a signing command (OW-ADR-0022): the
/// ingest ran with the write withheld, and this is its verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum Judged {
    /// Every refusal passed; the signature is the only thing missing.
    WouldRecord,
    /// The ingest would refuse, and `rule` is the first rule that refused.
    WouldRefuse { rule: String },
}

impl Judged {
    #[must_use]
    pub fn word(&self) -> String {
        match self {
            Self::WouldRecord => "would record".to_owned(),
            Self::WouldRefuse { rule } => format!("would refuse: {rule}"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Action {
    pub actor: Actor,
    pub warrant: String,
    /// A short verb phrase: `authorize`, `resolve`, `correct`, `accept`,
    /// `check`, `deliver`, `verify`, `execute`.
    pub action: String,
    /// The command that does it, verbatim.
    pub command: String,
    pub why: String,
    /// On every action whose command begins `war sign`: the dry run's
    /// verdict on exactly that command. Absent on an agent's action, which
    /// has no signature to judge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub judged: Option<Judged>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Next {
    pub schema: &'static str,
    pub actions: Vec<Action>,
    /// Why the list is empty, when it is. Never silently empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nothing: Option<String>,
}

/// The pure part: given the pending human acts and the corpus projection,
/// derive the table. Separated from I/O so the "an agent never signs"
/// invariant can be asserted on synthetic inputs.
#[must_use]
pub fn derive(pending: &[Pending], status: &openwarrant_core::status::CorpusStatus) -> Next {
    let mut actions = Vec::new();
    // Human acts first: they unblock the most.
    for p in pending {
        let (warrant, action, target, why) = match p {
            Pending::Authorize {
                alias,
                revision,
                amendment,
                ..
            } => (
                alias.clone(),
                "authorize",
                alias.clone(),
                format!(
                    "revision {revision} awaits authorization{}",
                    amendment
                        .as_ref()
                        .map(|a| format!(" under {}", a.id))
                        .unwrap_or_default()
                ),
            ),
            Pending::Resolve { alias, request, .. } => (
                alias.clone(),
                "resolve",
                alias.clone(),
                if request.would_resolve_satisfied == Some(true) {
                    "the thirteen are met and §38.6 would resolve satisfied".to_owned()
                } else {
                    format!(
                        "the thirteen are met; {} obligation(s) unestablished, so the outcome must be named",
                        request.unestablished.len()
                    )
                },
            ),
            Pending::Accept { version, .. } => (
                format!("SAS {version}"),
                "accept",
                version.clone(),
                "a proposed SAS revision awaits acceptance".to_owned(),
            ),
            Pending::AcceptRoadmap { revision, request } => (
                "roadmap".to_owned(),
                "accept",
                "roadmap".to_owned(),
                format!(
                    "roadmap revision {revision} awaits acceptance: {}",
                    request.diff.summary()
                ),
            ),
            Pending::Correct {
                alias,
                deliverable_id,
                request,
                ..
            } => (
                alias.clone(),
                "correct",
                format!("{alias}/{deliverable_id}"),
                format!("{} drifted after resolution", request.target_ref),
            ),
        };
        actions.push(Action {
            actor: Actor::Human,
            warrant,
            action: action.to_owned(),
            command: format!("war sign {target}"),
            why,
            judged: None,
        });
    }
    // Agent acts: whatever stands between a Warrant and the human's next act.
    for w in &status.warrants {
        use openwarrant_core::status::WarrantRung as R;
        let already_human = actions
            .iter()
            .any(|a| a.actor == Actor::Human && a.warrant == w.alias);
        match w.rung {
            R::Invalid => actions.push(Action {
                actor: Actor::Agent,
                warrant: w.alias.clone(),
                action: "check".to_owned(),
                command: format!("war check {}", w.alias),
                why: "the manifest does not validate".to_owned(),
                judged: None,
            }),
            // A pending human act on this Warrant is the unblocker; an agent
            // action beside it would be noise, so Draft is only an agent's when
            // no human act is pending.
            R::Draft if !already_human => {
                let unmet = w.unmet.join("; ");
                actions.push(Action {
                    actor: Actor::Agent,
                    warrant: w.alias.clone(),
                    action: "deliver".to_owned(),
                    command: format!("war evidence record {a} && war verify {a}", a = w.alias),
                    why: if unmet.is_empty() {
                        "not every §56.1 requirement is met".to_owned()
                    } else {
                        format!("unmet: {unmet}")
                    },
                    judged: None,
                });
            }
            R::ReadyToResolve if w.would_resolve_satisfied != Some(true) => {
                actions.push(Action {
                    actor: Actor::Agent,
                    warrant: w.alias.clone(),
                    action: "verify".to_owned(),
                    command: format!("war verify {}", w.alias),
                    why: "obligations remain unestablished; a blind verifier must dispose of them"
                        .to_owned(),
                    judged: None,
                });
            }
            _ => {}
        }
    }
    for s in &status.next_actionable {
        actions.push(Action {
            actor: Actor::Agent,
            warrant: s.warrant.clone(),
            action: "execute".to_owned(),
            command: format!("war dispatch {} {}", s.warrant, s.stage),
            why: format!("{} / {}: {}", s.milestone, s.stage, s.why),
            judged: None,
        });
    }
    // `assert!`, not `debug_assert!`: this invariant is the point of the
    // command and must hold in a release-built test binary too.
    assert!(
        actions
            .iter()
            .all(|a| a.actor != Actor::Agent || !a.command.starts_with("war sign")),
        "an agent is never handed a signing act"
    );
    let nothing = if actions.is_empty() {
        Some(
            status
                .nothing_actionable
                .as_ref()
                .map(|n| {
                    // `n.why` is already the sentence. Debug-printing the
                    // struct put `NothingActionable { objective: None,
                    // blocked_by: [], why: "..." }` on the terminal of anyone
                    // whose corpus had nothing to do — on the first command
                    // QUICKSTART tells a new reader to trust.
                    let mut s = n.why.clone();
                    if let Some(o) = &n.objective {
                        s.push_str(&format!(" ({o})"));
                    }
                    if !n.blocked_by.is_empty() {
                        s.push_str(&format!("; blocked by {}", n.blocked_by.join(", ")));
                    }
                    s
                })
                .unwrap_or_else(|| {
                    "nothing awaits a signature and no stage is actionable".to_owned()
                }),
        )
    } else {
        None
    };
    Next {
        schema: SCHEMA,
        actions,
        nothing,
    }
}

pub fn run(repo: &Repository) -> Result<Next, RepoError> {
    let status = crate::status::build(repo)?;
    run_with(repo, &status)
}

/// [`run`] over a corpus status already built, so a caller that also needs
/// the status (`war compile`'s master document) builds it once.
pub fn run_with(
    repo: &Repository,
    status: &openwarrant_core::status::CorpusStatus,
) -> Result<Next, RepoError> {
    let pending = sign::pending(repo)?;
    let mut next = derive(&pending, status);
    judge(repo, &pending, &mut next);
    Ok(next)
}

/// The dry run in front of every handed-over command (OW-ADR-0022). The
/// whole queue is judged by one `war sign --all --dry-run` — each act
/// drafted, its ingest run with the write withheld — and each verdict is
/// carried beside the `war sign <target>` that act is. Acts the dry run would
/// refuse are listed after the ones it would record, so the first command a
/// human reads is one that will work. Writes nothing and holds no key: the
/// dry run's own guarantees (`sign::Options::dry_run`).
///
/// One sweep rather than one dry run per target: each `sign::run` re-reads
/// the whole queue, and a queue of a dozen acts judged one by one cost a
/// dozen reads of it.
pub fn judge(repo: &Repository, pending: &[Pending], next: &mut Next) {
    let opts = sign::Options {
        dry_run: true,
        all: true,
        ..sign::Options::default()
    };
    let verdicts = match sign::run(repo, None, &opts) {
        Ok(report) => verdicts(pending, &report),
        Err(_) => std::collections::BTreeMap::new(),
    };
    for a in &mut next.actions {
        let Some(target) = a.command.strip_prefix("war sign ") else {
            continue;
        };
        let target = target.split_whitespace().next().unwrap_or(target);
        a.judged = Some(
            verdicts
                .get(target)
                .cloned()
                .unwrap_or_else(|| Judged::WouldRefuse {
                    rule: "sign.dry-run-failed".to_owned(),
                }),
        );
    }
    order(next);
}

/// The `war sign` target each pending act is handed over as — the same
/// words `derive` writes after `war sign `.
fn target_of(p: &Pending) -> String {
    match p {
        Pending::Authorize { alias, .. } | Pending::Resolve { alias, .. } => alias.clone(),
        Pending::Accept { version, .. } => version.clone(),
        Pending::AcceptRoadmap { .. } => "roadmap".to_owned(),
        Pending::Correct {
            alias,
            deliverable_id,
            ..
        } => format!("{alias}/{deliverable_id}"),
    }
}

/// Split a whole-queue dry run into one verdict per act. The sweep reports
/// each act as its ingest's diagnostics followed by one closing diagnostic
/// that names the act (`sign::line`): `sign.would-record`, or a refusal —
/// `sign.would-refuse`, `sign.who`, `sign.not-draftable`,
/// `sign.needs-decision`, `sign.dry-run-failed`.
fn verdicts(
    pending: &[Pending],
    report: &crate::diagnostic::Report,
) -> std::collections::BTreeMap<String, Judged> {
    const CLOSING: [&str; 6] = [
        "sign.would-record",
        "sign.would-refuse",
        "sign.who",
        "sign.not-draftable",
        "sign.needs-decision",
        "sign.dry-run-failed",
    ];
    let lines: Vec<(String, String)> = pending
        .iter()
        .map(|p| (sign::line(p), target_of(p)))
        .collect();
    let mut out = std::collections::BTreeMap::new();
    let mut segment = crate::diagnostic::Report::default();
    for d in &report.diagnostics {
        if !CLOSING.contains(&d.rule.as_str()) {
            segment.push(d.clone());
            continue;
        }
        let named = |l: &str| d.file.as_deref() == Some(l) || d.message.starts_with(l);
        if let Some((_, target)) = lines.iter().find(|(l, _)| named(l)) {
            segment.push(d.clone());
            out.insert(target.clone(), verdict(Ok(std::mem::take(&mut segment))));
        }
        segment = crate::diagnostic::Report::default();
    }
    out
}

/// Read a dry run's report as a verdict: `sign.would-record` is the only
/// recording word; otherwise the first error that is not the summary
/// `sign.would-refuse` names why.
fn verdict(report: Result<crate::diagnostic::Report, RepoError>) -> Judged {
    let report = match report {
        Ok(r) => r,
        Err(_) => {
            return Judged::WouldRefuse {
                rule: "sign.dry-run-failed".to_owned(),
            };
        }
    };
    if report
        .diagnostics
        .iter()
        .any(|d| d.rule == "sign.would-record")
    {
        return Judged::WouldRecord;
    }
    let rule = report
        .diagnostics
        .iter()
        .filter(|d| {
            d.severity == crate::diagnostic::Severity::Error || d.rule == "sign.needs-decision"
        })
        .map(|d| d.rule.clone())
        .find(|r| r != "sign.would-refuse")
        .unwrap_or_else(|| "sign.would-refuse".to_owned());
    Judged::WouldRefuse { rule }
}

/// Humans first; among them, what would record before what would refuse.
/// A stable sort, so the order within each group is `derive`'s.
fn order(next: &mut Next) {
    next.actions.sort_by_key(|a| match (a.actor, &a.judged) {
        (Actor::Human, Some(Judged::WouldRecord) | None) => 0,
        (Actor::Human, Some(Judged::WouldRefuse { .. })) => 1,
        (Actor::Agent, _) => 2,
    });
}

#[must_use]
pub fn render(n: &Next) -> String {
    let mut s = String::new();
    for a in &n.actions {
        s.push_str(&format!(
            "{:<6} {:<12} {:<10} {}{}\n{:>6} {}\n",
            match a.actor {
                Actor::Human => "HUMAN",
                Actor::Agent => "agent",
            },
            a.warrant,
            a.action,
            a.command,
            a.judged
                .as_ref()
                .map(|j| format!("  [{}]", j.word()))
                .unwrap_or_default(),
            "",
            a.why
        ));
    }
    if let Some(why) = &n.nothing {
        s.push_str(&format!("nothing to do: {why}\n"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_agent_is_never_handed_a_signing_act_and_humans_come_first() {
        // The real corpus is the fixture: every kind of pending act exists in
        // it or in its history, and the invariant must hold over all of it.
        let root = camino::Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize_utf8()
            .unwrap();
        let repo = Repository::open(root).unwrap();
        let n = run(&repo).unwrap();
        assert!(!n.actions.is_empty());
        for a in &n.actions {
            if a.actor == Actor::Agent {
                assert!(
                    !a.command.starts_with("war sign"),
                    "agent handed a signature: {a:?}"
                );
                assert!(
                    !["authorize", "resolve", "accept", "correct"].contains(&a.action.as_str())
                );
            } else {
                assert!(a.command.starts_with("war sign"), "{a:?}");
                assert!(
                    a.judged.is_some(),
                    "a signing command handed over unjudged: {a:?}"
                );
            }
        }
        let first_agent = n.actions.iter().position(|a| a.actor == Actor::Agent);
        let last_human = n.actions.iter().rposition(|a| a.actor == Actor::Human);
        if let (Some(fa), Some(lh)) = (first_agent, last_human) {
            assert!(lh < fa, "human acts are listed before agent acts");
        }
        let first_refused = n
            .actions
            .iter()
            .position(|a| matches!(a.judged, Some(Judged::WouldRefuse { .. })));
        let last_recordable = n
            .actions
            .iter()
            .rposition(|a| matches!(a.judged, Some(Judged::WouldRecord)));
        if let (Some(fr), Some(lr)) = (first_refused, last_recordable) {
            assert!(
                lr < fr,
                "a refusable act is listed after every recordable one"
            );
        }
    }

    #[test]
    fn a_verdict_names_the_ingest_rule_not_the_summary() {
        use crate::diagnostic::{Diagnostic, Report};
        let mut r = Report::default();
        r.push(Diagnostic::error("authorize.no-amendment", "x", "why"));
        r.push(Diagnostic::error("sign.would-refuse", "x", "summary"));
        assert_eq!(
            verdict(Ok(r)),
            Judged::WouldRefuse {
                rule: "authorize.no-amendment".into()
            }
        );
        let mut ok = Report::default();
        ok.push(Diagnostic::pass("sign.would-record", "fine"));
        assert_eq!(verdict(Ok(ok)), Judged::WouldRecord);
    }
}
