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
//!
//! t-67ed: ready ticket items come first. They are read from the ticket
//! store with the same computation `war view ready` uses, carry `war claim`, and
//! are never a signature; the Warrant acts follow them, still judged.

use serde::Serialize;

use crate::repo::{RepoError, Repository};
use crate::sign::{self, Pending};

pub const SCHEMA: &str = "oh.war/next/v1";

/// The first line `war next` prints when nothing is tracked (M9): ordinary
/// work needs no ticket and no Warrant, so an empty list is permission to
/// carry on, said as such.
pub const IDLE_UNTRACKED: &str = "nothing tracked; work freely";
/// The first line when tickets or Warrants exist and none is ready now.
pub const IDLE_TRACKED: &str = "nothing tracked is ready; work freely";
/// The one-line hint under either: tracking is a choice, never a step.
pub const IDLE_HINT: &str = "to track work (optional): war create \"what this work does\"";

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

/// A ticket item that can start now (`war view ready`'s row), offered before any
/// Warrant act. Its command is always `war claim`: never a signature.
#[derive(Debug, Clone, Serialize)]
pub struct ReadyItem {
    pub actor: Actor,
    pub ticket: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    pub title: String,
    pub text: String,
    pub priority: u8,
    pub command: String,
}

impl ReadyItem {
    fn of(r: &crate::ticket::ReadyRow) -> Self {
        Self {
            actor: Actor::Agent,
            ticket: r.ticket.clone(),
            item: r.item.clone(),
            title: r.title.clone(),
            text: r.text.clone(),
            priority: r.priority,
            command: format!("war claim {}", r.target()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Next {
    pub schema: &'static str,
    /// Ticket items that can start now, most urgent first (t-67ed). Listed
    /// before `actions`; nothing here needs anyone's signature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ready: Vec<ReadyItem>,
    pub actions: Vec<Action>,
    /// Why `actions` is empty, when it is. Never silently empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nothing: Option<String>,
    /// M9: when nothing at all is ready (no ticket item, no act), the line
    /// that says so in plain words: [`IDLE_UNTRACKED`] when nothing is
    /// tracked, [`IDLE_TRACKED`] when tracked work exists and none of it is
    /// ready. Either way ordinary work goes on; neither is a stop.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle: Option<String>,
    /// What stands in the way that no action here can clear (OW-WAR-0132):
    /// today `question.no-responder`, a blocking question nobody in the
    /// authority register may answer. UNKNOWN, never silently "waiting".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<Finding>,
    /// OW-WAR-0148 M14: under a preset that signs at release, the one
    /// command that signs the queue above in one batch. Absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release: Option<String>,
    /// OW-WAR-0148 M15: work `war evidence go` set aside for a person (its
    /// attempts ran out, or its performer asked for a decision), each with
    /// the reason and the command that puts it back. Absent when there is
    /// none, so the envelope is the bytes it always was.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked: Vec<crate::go::BlockedNode>,
}

/// A finding carried into `war next`, in the envelope diagnostic's shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub rule: String,
    /// `unknown` / `warn` / `error`, as the envelope spells a severity.
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub message: String,
}

/// The pure part: given the pending human acts and the corpus projection,
/// derive the table. Separated from I/O so the "an agent never signs"
/// invariant can be asserted on synthetic inputs.
///
/// `profiles` answers what each Warrant's kind selects (OW-ADR-0031): no act
/// is offered for a capability its kind lacks. A human act arrives here only
/// when its request could be built, and `war sign authorize`/`war sign resolve` refuse
/// a kind without the capability, so the gate for those is upstream.
#[must_use]
pub fn derive(
    pending: &[Pending],
    status: &openwarrant_core::status::CorpusStatus,
    profiles: &openwarrant_core::role::ProfileRegistry,
) -> Next {
    use openwarrant_core::Capability as C;
    let capabilities = |alias: &str| {
        status
            .warrants
            .iter()
            .find(|w| w.alias == alias)
            .and_then(|w| w.profile.as_deref())
            .and_then(|p| profiles.resolve(p).ok())
            .map_or(openwarrant_core::Capabilities::ALL, |p| {
                profiles.capabilities(&p)
            })
    };
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
            Pending::AcceptStanding { request } => (
                request.reference.clone(),
                "accept",
                crate::standing_cmd::target(&request.id, request.revision),
                format!(
                    "standing authorization {} awaits its one signature",
                    request.reference
                ),
            ),
            Pending::RevokeStanding { request } => (
                request.reference.clone(),
                "revoke",
                crate::standing_cmd::target(&request.id, request.revision),
                "a signed class, offered for revocation".to_owned(),
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
            Pending::Invalidate { gate, request } => (
                gate.clone(),
                "invalidate",
                gate.clone(),
                format!(
                    "invalidating {gate} would dispute {} resolution(s)",
                    request.disputes.len()
                ),
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
        // A kind without `resolution` is never delivered against §56.1, so
        // nothing stands between it and a human act an agent could clear.
        let resolvable = capabilities(&w.alias).has(C::Resolution);
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
            R::Draft if !already_human && resolvable => {
                let unmet = w.unmet.join("; ");
                actions.push(Action {
                    actor: Actor::Agent,
                    warrant: w.alias.clone(),
                    action: "deliver".to_owned(),
                    command: format!(
                        "war evidence record {a} && war evidence verify {a}",
                        a = w.alias
                    ),
                    why: if unmet.is_empty() {
                        "not every §56.1 requirement is met".to_owned()
                    } else {
                        format!("unmet: {unmet}")
                    },
                    judged: None,
                });
            }
            R::ReadyToResolve if w.would_resolve_satisfied != Some(true) && resolvable => {
                actions.push(Action {
                    actor: Actor::Agent,
                    warrant: w.alias.clone(),
                    action: "verify".to_owned(),
                    command: format!("war evidence verify {}", w.alias),
                    why: "obligations remain unestablished; a blind verifier must dispose of them"
                        .to_owned(),
                    judged: None,
                });
            }
            _ => {}
        }
    }
    for s in &status.next_actionable {
        if !capabilities(&s.warrant).has(C::Stages) {
            continue;
        }
        actions.push(Action {
            actor: Actor::Agent,
            warrant: s.warrant.clone(),
            action: "execute".to_owned(),
            command: format!("war admin dispatch {} {}", s.warrant, s.stage),
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
                .unwrap_or_else(|| "no signature is waiting and no stage is ready".to_owned()),
        )
    } else {
        None
    };
    Next {
        schema: SCHEMA,
        ready: Vec::new(),
        actions,
        nothing,
        idle: None,
        findings: Vec::new(),
        release: None,
        blocked: Vec::new(),
    }
}

/// OW-WAR-0132: the frontier's question edge, applied to the table. A stage
/// a blocking question holds is not offered to an agent (`war admin dispatch` would
/// start work the question is meant to stop), and the frontier's
/// `question.no-responder` is carried as a finding. Pure over the frontier,
/// so the rule is testable without a repository.
pub fn apply_questions(
    next: &mut Next,
    frontier: &crate::frontier::Frontier,
    report: &crate::diagnostic::Report,
) {
    let held: std::collections::BTreeSet<(&str, &str)> = frontier
        .rows
        .iter()
        .filter(|r| {
            r.state == crate::frontier::StageState::Blocked
                && r.waiting_on.iter().any(|w| is_question(w))
        })
        .map(|r| (r.warrant.as_str(), r.stage.as_str()))
        .collect();
    next.actions.retain(|a| {
        a.action != "execute" || {
            let stage = a.command.rsplit(' ').next().unwrap_or_default();
            !held.contains(&(a.warrant.as_str(), stage))
        }
    });
    for d in &report.diagnostics {
        if d.rule == "question.no-responder" {
            next.findings.push(Finding {
                rule: d.rule.clone(),
                severity: d.severity.label().to_ascii_lowercase(),
                file: d.file.clone(),
                message: d.message.clone(),
            });
        }
    }
    if next.actions.is_empty() && next.nothing.is_none() {
        next.nothing = Some(if held.is_empty() {
            "no signature is waiting and no stage is ready".to_owned()
        } else {
            format!(
                "every actionable stage waits on a blocking question: {}",
                held.iter()
                    .map(|(w, s)| format!("{w}/{s}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        });
    }
}

/// `Q-nnn`, as the frontier names a question in `waiting_on`; a milestone is
/// `M-…` / `MS-…` and never matches.
fn is_question(id: &str) -> bool {
    id.strip_prefix("Q-")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

pub fn run(repo: &Repository) -> Result<Next, RepoError> {
    run_with(&crate::corpus::Corpus::new(repo))
}

/// [`run`] over a corpus already loaded, so a caller that also needs the
/// status, the sign queue or the frontier (`war admin compile`'s master document,
/// the web UI) builds each once.
pub fn run_with(corpus: &crate::corpus::Corpus) -> Result<Next, RepoError> {
    let repo = corpus.repo();
    let status = corpus.status()?;
    let pending = corpus.pending()?;
    let mut next = derive(pending, status, &repo.profiles);
    let (report, frontier) = corpus.frontier()?;
    apply_questions(&mut next, frontier, report);
    // OW-WAR-0141: a question waiting before its Warrant exists is a human's
    // act too (`answer`), read from `docs/intake/` by questions.rs.
    next.actions.extend(crate::questions::intake_actions(repo));
    if !next.actions.is_empty() {
        next.nothing = None;
    }
    judge(repo, pending, &mut next);
    preset_acts(repo, &mut next);
    ready_tickets_with(corpus, &mut next);
    // OW-WAR-0148 M15: what a run set aside is a person's to look at, and is
    // not offered as ready meanwhile.
    if let Ok((tickets, _)) = corpus.tickets() {
        next.blocked = crate::go::blocked_nodes(tickets);
        next.ready.retain(|r| {
            let id = match &r.item {
                Some(i) => format!("{}/{i}", r.ticket),
                None => r.ticket.clone(),
            };
            !next.blocked.iter().any(|b| b.node == id)
        });
    }
    if next.actions.is_empty() && next.ready.is_empty() {
        // M9: an empty list is said as what it means for ordinary work. A
        // ticket store that could not be read counts as tracked: "nothing
        // tracked" is a claim, and the finding above already says UNKNOWN.
        let open_ticket = corpus.tickets().map_or(true, |(tickets, _)| {
            tickets.iter().any(|t| !t.checklist.is_done())
        });
        let open_warrant = status
            .warrants
            .iter()
            .any(|w| w.rung != openwarrant_core::status::WarrantRung::Resolved);
        next.idle = Some(
            if open_ticket || open_warrant {
                IDLE_TRACKED
            } else {
                IDLE_UNTRACKED
            }
            .to_owned(),
        );
    }
    Ok(next)
}

/// The lines `war next` (and the console) print when nothing is ready: the
/// plain first line, the reason, and the one-line hint. `None` when there is
/// something to list.
#[must_use]
pub fn idle_lines(n: &Next) -> Option<Vec<String>> {
    let first = n.idle.as_deref()?;
    let mut lines = vec![first.to_owned()];
    if let Some(why) = &n.nothing {
        lines.push(format!("  {why}"));
    }
    lines.push(format!("  {IDLE_HINT}"));
    Some(lines)
}

/// t-67ed: the ticket store's ready set, ahead of every Warrant act. The
/// same `ticket::ready_rows` `war view ready` answers from, so the two never
/// disagree. A store that cannot be read is an UNKNOWN finding, never an
/// empty list presented as "nothing ready".
pub fn ready_tickets(repo: &Repository, next: &mut Next) {
    let rows = crate::ticket::Store::open(repo, None).and_then(|store| {
        let (tickets, _faults) = store.load_all()?;
        crate::ticket::ready_rows(&store, &tickets)
    });
    ready_rows_into(rows, next);
}

/// [`ready_tickets`] over the corpus's tickets.
pub fn ready_tickets_with(corpus: &crate::corpus::Corpus, next: &mut Next) {
    let rows = crate::ticket::Store::open(corpus.repo(), None).and_then(|store| {
        let (tickets, _faults) = corpus.tickets()?;
        crate::ticket::ready_rows(&store, tickets)
    });
    ready_rows_into(rows, next);
}

fn ready_rows_into(rows: Result<Vec<crate::ticket::ReadyRow>, RepoError>, next: &mut Next) {
    match rows {
        Ok(rows) => next.ready = rows.iter().map(ReadyItem::of).collect(),
        Err(e) => next.findings.push(Finding {
            rule: "next.tickets-unreadable".to_owned(),
            severity: "unknown".to_owned(),
            file: None,
            message: format!("the ticket store could not be read, so ready items are UNKNOWN: {e}"),
        }),
    }
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
/// dozen reads of it. The sweep is over `pending` itself — the queue the
/// actions were derived from, read by the caller in this process — not a
/// second read of it (`sign::dry_run_all`, t-280c).
pub fn judge(repo: &Repository, pending: &[Pending], next: &mut Next) {
    let verdicts = match sign::dry_run_all(repo, pending) {
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

/// OW-WAR-0148 M14: the human acts a preset adds, only where one (or a
/// `[roles]` table) is configured: each approval someone asked for (`war
/// sign approve <id> --ssh-sign`, judged by its own dry run), and, when the
/// preset signs at release and two or more acts wait, the one command that
/// signs them in a batch. Without a preset the list is what it always was.
pub fn preset_acts(repo: &Repository, next: &mut Next) {
    let policy = crate::preset::Policy::read_or_default(&repo.root);
    if !policy.configured() {
        return;
    }
    let signing_acts = next
        .actions
        .iter()
        .filter(|a| a.actor == Actor::Human && a.command.starts_with("war sign "))
        .count();
    if policy.signing == crate::preset::Signing::Release && signing_acts >= 2 {
        next.release = Some(format!(
            "{signing_acts} acts above can be signed in one sitting at release: `war sign \
             release <tag>` lists them and drafts the one batch"
        ));
    }
    let requested = crate::official::requested(repo);
    if requested.is_empty() {
        return;
    }
    let store = crate::ticket::Store::open(repo, None).ok();
    for r in requested {
        let judged = store.as_ref().map_or(
            Judged::WouldRefuse {
                rule: "sign.dry-run-failed".to_owned(),
            },
            |st| {
                let args = crate::official::ApproveArgs {
                    dry_run: true,
                    ..crate::official::ApproveArgs::default()
                };
                verdict(crate::official::approve(repo, st, &r.warrant, &args).map(|o| o.report))
            },
        );
        next.actions.push(Action {
            actor: Actor::Human,
            warrant: r.warrant.clone(),
            action: "approve".to_owned(),
            command: r.command.clone(),
            why: format!(
                "{} asked for an approval of this {} Warrant (\"{}\"); approved, it is official",
                r.requested_by, r.kind, r.title
            ),
            judged: Some(judged),
        });
    }
    next.nothing = None;
    next.idle = None;
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
        Pending::Invalidate { gate, .. } => gate.clone(),
        Pending::AcceptStanding { .. } | Pending::RevokeStanding { .. } => {
            crate::sign::target_of(p)
        }
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

/// Workable items first (M9): an agent's acts, then the human acts, and
/// among those what would record before what would refuse. A signature row
/// at the top read as "wait for this", in a repository where nothing waits
/// on one. A stable sort, so the order within each group is `derive`'s.
fn order(next: &mut Next) {
    next.actions.sort_by_key(|a| match (a.actor, &a.judged) {
        (Actor::Agent, _) => 0,
        (Actor::Human, Some(Judged::WouldRecord) | None) => 1,
        (Actor::Human, Some(Judged::WouldRefuse { .. })) => 2,
    });
}

#[must_use]
pub fn render(n: &Next) -> String {
    let mut s = String::new();
    for r in &n.ready {
        s.push_str(&format!(
            "{:<6} {:<12} {:<10} {}\n{:>6} p{} {}{}\n",
            "ready",
            r.ticket,
            "claim",
            r.command,
            "",
            r.priority,
            r.text,
            if r.item.is_some() {
                format!(" — {}", r.title)
            } else {
                String::new()
            },
        ));
    }
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
    if let Some(r) = &n.release {
        s.push_str(&format!("{:>6} {r}\n", ""));
    }
    if let Some(lines) = idle_lines(n) {
        for l in lines {
            s.push_str(&l);
            s.push('\n');
        }
    } else if let Some(why) = &n.nothing {
        s.push_str(&format!("no Warrant act waits: {why}\n"));
    }
    // OW-WAR-0148 M15: after the plain first line, what a run set aside.
    for b in &n.blocked {
        s.push_str(&format!(
            "{:<6} {:<12} {:<10} {}\n{:>6} set aside by `war evidence go` after {} attempt(s): {}\n",
            "HUMAN", b.node, "look", b.command, "", b.attempts, b.reason
        ));
    }
    for f in &n.findings {
        s.push_str(&format!(
            "{} {}{}: {}\n",
            f.severity.to_ascii_uppercase(),
            f.rule,
            f.file
                .as_deref()
                .map(|p| format!(" ({p})"))
                .unwrap_or_default(),
            f.message
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(
        stage: &str,
        state: crate::frontier::StageState,
        waiting: &[&str],
    ) -> crate::frontier::Row {
        crate::frontier::Row {
            warrant: "X-WAR-0001".to_owned(),
            stage: stage.to_owned(),
            title: String::new(),
            milestone: "MS-001".to_owned(),
            executor_kind: "agent".to_owned(),
            state,
            waiting_on: waiting.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    fn execute(stage: &str) -> Action {
        Action {
            actor: Actor::Agent,
            warrant: "X-WAR-0001".to_owned(),
            action: "execute".to_owned(),
            command: format!("war admin dispatch X-WAR-0001 {stage}"),
            why: String::new(),
            judged: None,
        }
    }

    #[test]
    fn a_question_held_stage_is_not_offered_and_no_responder_is_carried() {
        use crate::frontier::StageState::{Blocked, Open};
        let frontier = crate::frontier::Frontier {
            schema: crate::frontier::SCHEMA.to_owned(),
            rows: vec![
                row("STAGE-001", Blocked, &["Q-001"]),
                row("STAGE-002", Open, &[]),
                row("STAGE-003", Blocked, &["MS-000"]),
            ],
            open: 1,
            claimed: 0,
            done: 0,
            blocked: 2,
        };
        let mut report = crate::diagnostic::Report::default();
        report.push(crate::diagnostic::Diagnostic::unknown(
            "question.no-responder",
            "docs/authority/roles.toml".to_owned(),
            "nobody can answer",
        ));
        let mut next = Next {
            schema: SCHEMA,
            ready: Vec::new(),
            actions: vec![
                execute("STAGE-001"),
                execute("STAGE-002"),
                execute("STAGE-003"),
            ],
            nothing: None,
            idle: None,
            findings: Vec::new(),
            release: None,
            blocked: Vec::new(),
        };
        apply_questions(&mut next, &frontier, &report);
        let offered: Vec<&str> = next.actions.iter().map(|a| a.command.as_str()).collect();
        // The question-held stage is gone; a milestone wait is not this rule's.
        assert_eq!(
            offered,
            [
                "war admin dispatch X-WAR-0001 STAGE-002",
                "war admin dispatch X-WAR-0001 STAGE-003"
            ]
        );
        assert_eq!(next.findings.len(), 1);
        assert_eq!(next.findings[0].severity, "unknown");
        assert_eq!(
            next.findings[0].file.as_deref(),
            Some("docs/authority/roles.toml")
        );
        assert!(render(&next).contains("UNKNOWN question.no-responder"));

        // Refusal side: with only the held stage, the list says why it is
        // empty instead of going silent, and nothing is offered.
        let mut next = Next {
            schema: SCHEMA,
            ready: Vec::new(),
            actions: vec![execute("STAGE-001")],
            nothing: None,
            idle: None,
            findings: Vec::new(),
            release: None,
            blocked: Vec::new(),
        };
        apply_questions(&mut next, &frontier, &crate::diagnostic::Report::default());
        assert!(next.actions.is_empty());
        assert!(next.findings.is_empty());
        assert!(
            next.nothing
                .as_deref()
                .unwrap_or_default()
                .contains("X-WAR-0001/STAGE-001")
        );
    }

    #[test]
    fn an_agent_is_never_handed_a_signing_act_and_workable_acts_come_first() {
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
        // t-67ed: a ready ticket item is an agent's, and is a claim — never
        // a signature, never an authority verb.
        for r in &n.ready {
            assert_eq!(r.actor, Actor::Agent);
            assert!(r.command.starts_with("war claim "), "{r:?}");
            assert!(!r.command.contains("war sign"), "{r:?}");
        }
        // M9: an agent's workable acts come before every human act, so a
        // signature row never heads the list of what to do.
        let last_agent = n.actions.iter().rposition(|a| a.actor == Actor::Agent);
        let first_human = n.actions.iter().position(|a| a.actor == Actor::Human);
        if let (Some(la), Some(fh)) = (last_agent, first_human) {
            assert!(la < fh, "agent acts are listed before human acts");
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

    fn ready_item() -> ReadyItem {
        ReadyItem {
            actor: Actor::Agent,
            ticket: "t-0001".to_owned(),
            item: Some("i-0001".to_owned()),
            title: "A ticket".to_owned(),
            text: "The first item".to_owned(),
            priority: 1,
            command: "war claim t-0001/i-0001".to_owned(),
        }
    }

    fn human_act() -> Action {
        Action {
            actor: Actor::Human,
            warrant: "X-WAR-0001".to_owned(),
            action: "authorize".to_owned(),
            command: "war sign X-WAR-0001".to_owned(),
            why: "revision 1 awaits authorization".to_owned(),
            judged: Some(Judged::WouldRecord),
        }
    }

    #[test]
    fn a_ready_ticket_item_is_listed_before_a_human_act() {
        let next = Next {
            schema: SCHEMA,
            ready: vec![ready_item()],
            actions: vec![human_act()],
            nothing: None,
            idle: None,
            findings: Vec::new(),
            release: None,
            blocked: Vec::new(),
        };
        let out = render(&next);
        let claim = out
            .find("war claim t-0001/i-0001")
            .expect("the ready item is listed");
        let sign = out
            .find("war sign X-WAR-0001")
            .expect("the human act stays listed");
        assert!(claim < sign, "ready items come first:\n{out}");
        assert!(
            out.contains("[would record]"),
            "the human act is still judged"
        );
        // JSON: `ready` sits beside `actions`, and names an agent's claim.
        let v = serde_json::to_value(&next).unwrap();
        assert_eq!(v["ready"][0]["command"], "war claim t-0001/i-0001");
        assert_eq!(v["ready"][0]["actor"], "agent");
        assert_eq!(v["actions"][0]["actor"], "human");
    }

    #[test]
    fn with_nothing_ready_it_says_work_freely_and_ready_is_omitted() {
        let next = Next {
            schema: SCHEMA,
            ready: Vec::new(),
            actions: Vec::new(),
            nothing: Some("no signature is waiting".to_owned()),
            idle: Some(IDLE_UNTRACKED.to_owned()),
            findings: Vec::new(),
            release: None,
            blocked: Vec::new(),
        };
        let out = render(&next);
        // M9: the first line is permission, the second the reason, the third
        // the optional hint; nothing reads as "nothing to do".
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "nothing tracked; work freely", "{out}");
        assert_eq!(lines[1], "  no signature is waiting");
        assert!(lines[2].contains("war create"), "{out}");
        assert!(!out.contains("nothing to do"), "{out}");
        let v = serde_json::to_value(&next).unwrap();
        assert!(v.get("ready").is_none());
        assert_eq!(v["idle"], IDLE_UNTRACKED);
        // Tracked work that is not ready says so, and still frees the reader.
        let tracked = Next {
            idle: Some(IDLE_TRACKED.to_owned()),
            ..next.clone()
        };
        assert!(render(&tracked).starts_with("nothing tracked is ready; work freely\n"));
        // Refusal side: with a ready item there is no idle line at all.
        let next = Next {
            ready: vec![ready_item()],
            idle: None,
            ..next
        };
        let out = render(&next);
        assert!(!out.contains("work freely"), "{out}");
        assert!(out.contains("no Warrant act waits: no signature is waiting"));
    }

    #[test]
    fn an_agent_act_is_ordered_before_a_human_act() {
        let mut next = Next {
            schema: SCHEMA,
            ready: Vec::new(),
            actions: vec![human_act(), execute("STAGE-001")],
            nothing: None,
            idle: None,
            findings: Vec::new(),
            release: None,
            blocked: Vec::new(),
        };
        order(&mut next);
        assert_eq!(next.actions[0].actor, Actor::Agent);
        assert_eq!(next.actions[1].actor, Actor::Human);
        // And the reverse input gives the same order: the sort decides it.
        next.actions.reverse();
        order(&mut next);
        assert_eq!(next.actions[0].actor, Actor::Agent);
    }
}
