// SPDX-License-Identifier: Apache-2.0
//! `war evidence go`: the native graph executor (OW-WAR-0148 M15; decisions
//! 9, 10 and 11).
//!
//! The work graph ([`crate::graph`]) says what can start; this runs it. Each
//! ready node, critical path first ([`crate::estimate`]), is claimed through
//! M11's shared claims, given a worktree on a branch of its own
//! (`war-go/<node>`), and handed to an executor: a local harness (Claude Code
//! or any argv) or an external tool through a thin adapter (GitHub Agent HQ,
//! Linear, Gas Town, any argv). What comes back is a Stage Submission, and it
//! passes exactly the refusals `war evidence submit` applies, so no executor
//! can declare work done. A node whose answer asks to be checked is merged
//! with the integration branch in its worktree, its tests run there, and it
//! lands, merged into the integration branch (or opened as a pull request);
//! then it is ticked at the level it earned (`observed` when its tests ran,
//! `claimed` otherwise) and what waited on it becomes ready.
//!
//! A crash, a timeout, a merge conflict or a failed check leaves the node
//! open and unclaimed, with a note saying what happened, and it goes back to
//! the frontier; after `[go] max_attempts` in a row it is blocked for a
//! person, and `war next` lists it. The run stops when nothing is ready, or
//! a cap or a budget is reached, and says which.
//!
//! Parallelism is by isolation. Several performers at once write several
//! worktrees, never one tree: `[go] max_parallel` above 1 is admitted only
//! with `isolation = "worktree"`, and `war evidence perform`, which runs in
//! the shared tree, still refuses `[perform] max_concurrent` above 1.
//!
//! docs/GO.md is the whole picture.

pub mod git;
pub mod ledger;
pub mod packet;
pub mod policy;
pub mod proc;
pub mod start;

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_core::WarUuid;
use openwarrant_core::execution::{RequestedNextAction, StageSubmission};
use serde::Serialize;

use crate::diagnostic::{Diagnostic, Report};
use crate::graph::{Graph, Kind, State};
use crate::repo::{RepoError, Repository};
use crate::ticket::{Store, Ticket};
use policy::{Isolation, Land, Policy};

pub const SCHEMA: &str = "oh.war/go-run/v1";

/// The journal events a run writes in a light Warrant's journal.
pub mod event {
    /// A node handed to an executor: the bindings its answer must carry.
    pub const DISPATCHED: &str = "go.dispatched";
    /// An attempt ended: `landed`, or why it did not.
    pub const ENDED: &str = "go.ended";
    /// A node set aside for a person.
    pub const BLOCKED: &str = "go.blocked";
    /// A person put it back on the frontier (`war evidence go --retry`).
    pub const RETRY: &str = "go.retry";
}

/// What `war evidence go` was asked.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Who the run claims as; default `<actor>@go-<run>`.
    pub actor: Option<String>,
    /// Start at most this many nodes.
    pub max_nodes: Option<usize>,
    /// Say what would start, in order, and start nothing.
    pub dry_run: bool,
    /// Put a node blocked for a person back on the frontier, and stop.
    pub retry: Option<String>,
    /// Run agent stages of Warrants nobody has signed (`war evidence perform
    /// --prototype`'s meaning).
    pub prototype: bool,
}

/// One node's outcome in a run.
#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub node: String,
    /// `landed`, or why not: `crashed`, `timeout`, `conflict`,
    /// `check-failed`, `refused`, `continue`, `blocked`, ...
    pub outcome: String,
    pub attempt: u32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub detail: String,
    pub seconds: u64,
    /// The level a landed node was ticked at.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// Tokens the harness reported, or `"unknown"`.
    pub tokens: serde_json::Value,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub executor: String,
}

/// Why a run stopped.
#[derive(Debug, Clone, Serialize)]
pub struct Stop {
    /// `done`, `nothing-ready`, `max-nodes`, `time-budget`, `cancelled`.
    pub reason: &'static str,
    pub detail: String,
}

/// `oh.war/go-run/v1`.
#[derive(Debug, Clone, Serialize)]
pub struct RunResult {
    pub schema: &'static str,
    pub run: String,
    pub actor: String,
    pub isolation: &'static str,
    pub integration: String,
    pub max_parallel: u32,
    /// The most nodes that ran at once.
    pub peak_parallel: usize,
    pub landed: Vec<Row>,
    pub requeued: Vec<Row>,
    pub blocked: Vec<Row>,
    /// Nodes this run did not start, and why (held elsewhere, a person's).
    pub skipped: Vec<Row>,
    /// With `--dry-run`: the ready nodes in the order they would start.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub order: Vec<String>,
    pub stop: Stop,
}

/// A node set aside for a person, as `war next` lists it.
#[derive(Debug, Clone, Serialize)]
pub struct BlockedNode {
    pub node: String,
    pub reason: String,
    pub attempts: u32,
    pub at: String,
    pub command: String,
}

// ---------------------------------------------------------------------------
// The journal: attempts, blocks and retries.
// ---------------------------------------------------------------------------

fn payload(e: &openwarrant_core::journal::JournalEvent) -> serde_json::Value {
    serde_json::from_str(&e.payload).unwrap_or(serde_json::Value::Null)
}

fn node_of(v: &serde_json::Value) -> &str {
    v.get("node")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
}

/// Unlanded attempts in a row for `node`, since it last landed or a person
/// put it back.
fn attempts_in_row(journal: &openwarrant_core::journal::Journal, node: &str) -> u32 {
    let mut n = 0;
    for e in &journal.events {
        let p = payload(e);
        if node_of(&p) != node {
            continue;
        }
        match e.event_type.as_str() {
            event::ENDED => {
                if p.get("outcome").and_then(serde_json::Value::as_str) == Some("landed") {
                    n = 0;
                } else {
                    n += 1;
                }
            }
            event::RETRY => n = 0,
            _ => {}
        }
    }
    n
}

/// Why `node` is set aside for a person, when it is: the last `go.blocked`
/// with no `go.retry` or landing after it.
fn blocked_in(
    journal: &openwarrant_core::journal::Journal,
    node: &str,
) -> Option<(String, u32, String)> {
    let mut out = None;
    for e in &journal.events {
        let p = payload(e);
        if node_of(&p) != node {
            continue;
        }
        match e.event_type.as_str() {
            event::BLOCKED => {
                out = Some((
                    p.get("reason")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("set aside")
                        .to_owned(),
                    p.get("attempts")
                        .and_then(serde_json::Value::as_u64)
                        .and_then(|n| u32::try_from(n).ok())
                        .unwrap_or(0),
                    e.occurred_at.clone(),
                ));
            }
            event::RETRY => out = None,
            event::ENDED
                if p.get("outcome").and_then(serde_json::Value::as_str) == Some("landed") =>
            {
                out = None;
            }
            _ => {}
        }
    }
    out
}

/// Every open node of a light Warrant that a run set aside for a person.
/// Done items are left out: a person who finished it by hand finished it.
#[must_use]
pub fn blocked_nodes(tickets: &[Ticket]) -> Vec<BlockedNode> {
    let mut out = Vec::new();
    for t in tickets {
        if t.checklist.is_done() {
            continue;
        }
        let Ok(journal) = crate::journal_cmd::load(&t.dir) else {
            continue;
        };
        if !journal
            .events
            .iter()
            .any(|e| e.event_type == event::BLOCKED)
        {
            continue;
        }
        let mut nodes: Vec<String> = t
            .checklist
            .items
            .iter()
            .filter(|i| !i.done)
            .filter_map(|i| i.id.as_ref().map(|id| format!("{}/{id}", t.id())))
            .collect();
        if t.checklist.items.is_empty() {
            nodes.push(t.id().to_owned());
        }
        for node in nodes {
            if let Some((reason, attempts, at)) = blocked_in(&journal, &node) {
                out.push(BlockedNode {
                    command: format!("war evidence go --retry {node}"),
                    node,
                    reason,
                    attempts,
                    at,
                });
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The run.
// ---------------------------------------------------------------------------

/// What is running for one node.
struct Live {
    node: String,
    kind: Kind,
    /// The light Warrant, or the directory Warrant's alias.
    warrant: String,
    /// The item, or the stage.
    part: Option<String>,
    /// The worktree's top level (git runs here).
    worktree: Utf8PathBuf,
    /// The program's root inside it (the performer and the checks run here).
    program: Utf8PathBuf,
    branch: Option<String>,
    dispatch_id: String,
    attempt: u32,
    lock: Utf8PathBuf,
    last_renew: Instant,
    started: Instant,
    executor: String,
    via: Via,
}

enum Via {
    Native(Option<proc::Running>),
    External {
        exec: policy::Executor,
        reference: String,
        next_poll: Instant,
    },
    Stage {
        running: Option<proc::Running>,
        kind: crate::perform::Attempt,
        dir: Utf8PathBuf,
        uuid: Option<String>,
        writer: Utf8PathBuf,
    },
}

/// How one attempt ended, before it is judged.
enum Ending {
    Answer { text: String, tokens: Option<u64> },
    Crashed { how: String, stderr: Option<String> },
    Timeout,
    Flooded,
    AdapterFailed(String),
    Cancelled,
}

struct Run<'a> {
    repo: &'a Repository,
    /// Where the program's root is below the checkout's top level (empty
    /// for most repositories): the same path inside each worktree.
    below: Utf8PathBuf,
    policy: Policy,
    store: Store,
    graph: Graph,
    common: Option<Utf8PathBuf>,
    run_id: String,
    actor: String,
    prototype: bool,
    report: Report,
    human: String,
    landed: Vec<Row>,
    requeued: Vec<Row>,
    blocked: Vec<Row>,
    skipped: Vec<Row>,
    /// Nodes this run set aside: not started again by it.
    passed_over: BTreeSet<String>,
    tokens_unknown: BTreeSet<String>,
}

fn now_rfc3339() -> String {
    crate::gate_cmd::receipt::rfc3339_from_secs(crate::ticket::now_secs())
}

fn tokens_value(t: Option<u64>) -> serde_json::Value {
    t.map_or_else(|| serde_json::json!("unknown"), |n| serde_json::json!(n))
}

/// `war evidence go`.
pub fn run(
    repo: &Repository,
    opts: &Options,
) -> Result<(Report, serde_json::Value, String), RepoError> {
    let mut report = Report::default();
    let policy = match policy::load(&repo.root) {
        Ok(p) => p,
        Err(d) => {
            report.push(d);
            return Ok((report, serde_json::Value::Null, String::new()));
        }
    };
    let run_id = WarUuid::mint().to_string();
    let base_actor = Store::open(repo, None)?.actor;
    let actor = opts.actor.clone().unwrap_or_else(|| {
        format!(
            "{base_actor}@go-{}",
            &run_id[run_id.len().saturating_sub(4)..]
        )
    });
    let store = Store::open(repo, Some(&actor))?;

    if let Some(node) = &opts.retry {
        return retry(&store, node, report);
    }

    let common = git::common_dir(&repo.root);
    if policy.isolation == Isolation::Worktree && common.is_none() {
        report.push(Diagnostic::error(
            "go.no-git",
            repo.root.to_string(),
            "`isolation = \"worktree\"` gives each node a git worktree, and this is not a git \
             checkout. Run it in one, or set `[go] isolation = \"shared\"` to run one node at a \
             time in this tree",
        ));
        return Ok((report, serde_json::Value::Null, String::new()));
    }
    if policy.isolation == Isolation::Shared && policy.max_parallel > 1 {
        report.push(Diagnostic::error(
            "go.no-containment",
            crate::init::CONFIG_FILE.to_owned(),
            format!(
                "[go] max_parallel is {} with `isolation = \"shared\"`: several performers would \
                 write this one tree at once. Several at once run in worktrees (`isolation = \
                 \"worktree\"`, the default), or set max_parallel = 1",
                policy.max_parallel
            ),
        ));
        return Ok((report, serde_json::Value::Null, String::new()));
    }

    let corpus = crate::corpus::Corpus::new(repo);
    let (graph_report, graph) = crate::graph::build(&corpus, &store)?;
    let cycles = !graph.cycles.is_empty();
    for d in graph_report.diagnostics {
        report.push(d);
    }
    if cycles {
        report.note(
            "nothing was started: a cycle means no node in it can ever be ready, so the run \
             waits until it is broken"
                .to_owned(),
        );
        return Ok((report, serde_json::Value::Null, String::new()));
    }
    let tickets: Vec<Ticket> = corpus.tickets()?.0.clone();
    let below = git::toplevel(&repo.root)
        .and_then(|top| repo.root.strip_prefix(&top).ok().map(Utf8Path::to_owned))
        .unwrap_or_default();
    let mut run = Run {
        repo,
        below,
        policy,
        store,
        graph,
        common,
        run_id,
        actor,
        prototype: opts.prototype,
        report,
        human: String::new(),
        landed: Vec::new(),
        requeued: Vec::new(),
        blocked: Vec::new(),
        skipped: Vec::new(),
        passed_over: BTreeSet::new(),
        tokens_unknown: BTreeSet::new(),
    };
    run.overlay(&tickets);
    let mut history_report = Report::default();
    let history = crate::estimate::learn(&tickets, run.policy.prior_secs, &mut history_report);
    for d in history_report.diagnostics {
        run.report.push(d);
    }

    if opts.dry_run {
        let schedule = crate::estimate::schedule(&run.graph, &history);
        let mut human = String::new();
        if schedule.order.is_empty() {
            human.push_str("nothing is ready to start\n");
        }
        for (n, id) in schedule.order.iter().enumerate() {
            let row = schedule.rows.iter().find(|r| r.id == *id);
            human.push_str(&format!(
                "{:>2}. {id}  {}{}  ({})\n",
                n + 1,
                row.map(|r| crate::estimate::human(r.estimate.seconds))
                    .unwrap_or_default(),
                row.and_then(|r| r.due.as_ref())
                    .map(|d| format!(", due {d}"))
                    .unwrap_or_default(),
                run.executor_name(id),
            ));
        }
        let result = RunResult {
            schema: SCHEMA,
            run: run.run_id.clone(),
            actor: run.actor.clone(),
            isolation: run.isolation_word(),
            integration: run.policy.integration.clone(),
            max_parallel: run.policy.max_parallel,
            peak_parallel: 0,
            landed: Vec::new(),
            requeued: Vec::new(),
            blocked: Vec::new(),
            skipped: run.skipped.clone(),
            order: schedule.order.clone(),
            stop: Stop {
                reason: "dry-run",
                detail: format!("{} node(s) would start in this order", schedule.order.len()),
            },
        };
        run.report.note(format!(
            "dry run: {} ready node(s); nothing was claimed or started",
            schedule.order.len()
        ));
        return Ok((run.report, output_value(&result), human));
    }

    run.preflight();
    if run.report.count(crate::diagnostic::Severity::Error) > 0 {
        return Ok((run.report, serde_json::Value::Null, run.human));
    }
    let (stop, peak) = run.drive(&history, opts.max_nodes)?;
    let result = RunResult {
        schema: SCHEMA,
        run: run.run_id.clone(),
        actor: run.actor.clone(),
        isolation: run.isolation_word(),
        integration: run.policy.integration.clone(),
        max_parallel: run.policy.max_parallel,
        peak_parallel: peak,
        landed: run.landed.clone(),
        requeued: run.requeued.clone(),
        blocked: run.blocked.clone(),
        skipped: run.skipped.clone(),
        order: Vec::new(),
        stop: stop.clone(),
    };
    run.human.push_str(&format!(
        "stopped: {} ({} landed, {} sent back, {} blocked for a person, {} not started)\n",
        stop.detail,
        run.landed.len(),
        run.requeued.len(),
        run.blocked.len(),
        run.skipped.len()
    ));
    run.report.note(format!("war evidence go: {}", stop.detail));
    Ok((run.report, output_value(&result), run.human))
}

fn output_value<T: Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
}

/// `war evidence go --retry <node>`: a person puts a blocked node back.
fn retry(
    store: &Store,
    node: &str,
    mut report: Report,
) -> Result<(Report, serde_json::Value, String), RepoError> {
    let (tickets, _) = store.load_all()?;
    let (tid, _) = openwarrant_core::ticket::split_record_id(node);
    let Some(t) = tickets.iter().find(|t| t.id() == tid) else {
        report.push(Diagnostic::error(
            "go.unknown-node",
            String::new(),
            format!(
                "{node} is not a node of a light Warrant; `war plan frontier --all` lists them"
            ),
        ));
        return Ok((report, serde_json::Value::Null, String::new()));
    };
    let journal = crate::journal_cmd::load(&t.dir)?;
    if blocked_in(&journal, node).is_none() {
        report.push(Diagnostic::error(
            "go.not-blocked",
            store.rel(&t.dir.join(crate::journal_cmd::FILE)),
            format!("{node} is not set aside; nothing to put back"),
        ));
        return Ok((report, serde_json::Value::Null, String::new()));
    }
    store.journal(
        t,
        event::RETRY,
        &serde_json::json!({"node": node, "retry": WarUuid::mint().to_string()}),
    )?;
    let human = format!("{node} is back on the frontier; `war evidence go` starts it again\n");
    report.push(Diagnostic::pass("go.retry", human.trim_end().to_owned()));
    Ok((
        report,
        serde_json::json!({"schema": SCHEMA, "retry": node}),
        human,
    ))
}

impl Run<'_> {
    fn isolation_word(&self) -> &'static str {
        match self.policy.isolation {
            Isolation::Worktree => "worktree",
            Isolation::Shared => "shared",
        }
    }

    fn executor_name(&self, id: &str) -> String {
        let Some(n) = self.graph.node(id) else {
            return "native".to_owned();
        };
        if n.kind == Kind::Stage {
            return "native ([perform] performer_argv)".to_owned();
        }
        self.policy.executor_for(&n.labels).map_or_else(
            || "native".to_owned(),
            |e| format!("{} ({})", e.name, e.adapter),
        )
    }

    fn rel(&self, p: &Utf8Path) -> String {
        self.repo.relative(p)
    }

    /// What the graph alone does not know: nodes landed by another run,
    /// nodes set aside for a person, nodes that do not run unattended.
    fn overlay(&mut self, tickets: &[Ticket]) {
        if let Some(common) = &self.common {
            for l in ledger::all(common) {
                if self
                    .graph
                    .node(&l.node)
                    .is_some_and(|n| n.state != State::Done)
                {
                    self.graph.mark_done(&l.node);
                }
            }
        }
        for b in blocked_nodes(tickets) {
            self.graph.block(
                &b.node,
                format!("set aside for a person: {} (`{}`)", b.reason, b.command),
            );
            self.skipped.push(Row {
                node: b.node.clone(),
                outcome: "blocked".to_owned(),
                attempt: b.attempts,
                detail: b.reason.clone(),
                seconds: 0,
                level: None,
                commit: None,
                tokens: tokens_value(None),
                executor: String::new(),
            });
        }
        let by_id: BTreeMap<&str, &Ticket> = tickets.iter().map(|t| (t.id(), t)).collect();
        let mut held_back: Vec<(String, String)> = Vec::new();
        for n in &self.graph.nodes {
            if !n.runnable || matches!(n.state, State::Done | State::Blocked) {
                continue;
            }
            if !self.policy.unattended_admits(n.work_type.as_deref()) {
                held_back.push((
                    n.id.clone(),
                    format!(
                        "its type ({}) is not in [go] unattended; a person runs it with `war \
                         start {}`",
                        n.work_type.as_deref().unwrap_or("untyped"),
                        n.id
                    ),
                ));
                continue;
            }
            // A tick `war evidence go` cannot earn is a person's to give.
            if n.kind != Kind::Stage
                && let Some(t) = n.warrant.as_deref().and_then(|w| by_id.get(w))
            {
                let item = openwarrant_core::ticket::split_record_id(&n.id).1;
                let checks = crate::ticket::ladder::checks_of(t);
                let (tests, kpis) = checks.for_item(item);
                let deciding = tests.len() + kpis.iter().filter(|k| k.gates()).count();
                let reach = if deciding > 0 {
                    openwarrant_core::ticks::Level::Observed
                } else {
                    openwarrant_core::ticks::Level::Claimed
                };
                let (minimum, source) =
                    crate::ticket::ladder::minimum(&self.store, t, &checks, item);
                if reach < minimum {
                    held_back.push((
                        n.id.clone(),
                        format!(
                            "it ticks at {} or above ({}), and a run reaches {}; {}",
                            minimum.as_str(),
                            source.describe(),
                            reach.as_str(),
                            crate::ticket::ladder::command_for(minimum, &n.id)
                        ),
                    ));
                }
            }
        }
        for (id, why) in held_back {
            self.graph.block(&id, why.clone());
            self.passed_over.insert(id.clone());
            self.skipped.push(Row {
                node: id,
                outcome: "for-a-person".to_owned(),
                attempt: 0,
                detail: why,
                seconds: 0,
                level: None,
                commit: None,
                tokens: tokens_value(None),
                executor: String::new(),
            });
        }
        self.graph.settle();
    }

    /// Refusals that apply to the whole run, before anything is claimed.
    fn preflight(&mut self) {
        let ready: Vec<(String, Kind, Vec<String>)> = self
            .graph
            .ready()
            .iter()
            .map(|n| (n.id.clone(), n.kind, n.labels.clone()))
            .collect();
        let native_items = ready
            .iter()
            .any(|(_, k, labels)| *k != Kind::Stage && self.policy.executor_for(labels).is_none());
        if native_items && self.policy.harness_argv.is_empty() {
            self.report.push(Diagnostic::error(
                "go.no-harness",
                crate::init::CONFIG_FILE.to_owned(),
                "no harness is configured for work run here: set `[go] harness = \"claude\"`, or \
                 `harness = \"generic\"` with `harness_argv` (the packet on stdin, a submission \
                 on stdout). Nothing was started"
                    .to_owned(),
            ));
        }
        if self.policy.isolation == Isolation::Worktree {
            if let Err(e) = git::ensure_branch(&self.repo.root, &self.policy.integration) {
                self.report.push(Diagnostic::error(
                    "go.worktree",
                    self.repo.root.to_string(),
                    format!("the integration branch {}: {e}", self.policy.integration),
                ));
            }
            git::exclude(
                &self.repo.root,
                &format!("/{}/", self.policy.worktrees.trim_end_matches('/')),
            );
            for f in packet::SESSION_FILES {
                git::exclude(&self.repo.root, f);
            }
        }
    }

    /// The loop: start what is ready up to the cap, watch what runs, land or
    /// send back what ends, until nothing more can start.
    fn drive(
        &mut self,
        history: &crate::estimate::History,
        max_nodes: Option<usize>,
    ) -> Result<(Stop, usize), RepoError> {
        let _armed = crate::perform::cancel::arm().map_err(|e| {
            RepoError::Message(format!("could not install the signal handler: {e}"))
        })?;
        let began = Instant::now();
        let lease = Duration::from_secs(self.store.lease_secs.max(3));
        let mut live: Vec<Live> = Vec::new();
        let mut started = 0usize;
        let mut peak = 0usize;
        let mut stop: Option<Stop> = None;
        let mut last_sync = Instant::now();
        loop {
            if stop.is_none()
                && let Some(signal) = crate::perform::cancel::requested()
            {
                stop = Some(Stop {
                    reason: "cancelled",
                    detail: format!("{signal} received; what was running was stopped and released"),
                });
            }
            if matches!(&stop, Some(s) if s.reason == "cancelled") {
                for l in std::mem::take(&mut live) {
                    self.cancel(l);
                }
                break;
            }
            if last_sync.elapsed() > Duration::from_secs(1) {
                self.sync_foreign(&live);
                last_sync = Instant::now();
            }
            while stop.is_none() && live.len() < self.policy.max_parallel as usize {
                if max_nodes.is_some_and(|m| started >= m) {
                    stop = Some(Stop {
                        reason: "max-nodes",
                        detail: format!("--max-nodes {} reached", max_nodes.unwrap_or_default()),
                    });
                    break;
                }
                if self
                    .policy
                    .max_total_secs
                    .is_some_and(|t| began.elapsed().as_secs() >= t)
                {
                    stop = Some(Stop {
                        reason: "time-budget",
                        detail: format!(
                            "[go] max_total_secs ({}) reached",
                            self.policy.max_total_secs.unwrap_or_default()
                        ),
                    });
                    break;
                }
                let schedule = crate::estimate::schedule(&self.graph, history);
                let Some(id) = schedule
                    .order
                    .iter()
                    .find(|id| !self.passed_over.contains(*id))
                    .cloned()
                else {
                    break;
                };
                match self.launch(&id)? {
                    Some(l) => {
                        self.human.push_str(&format!(
                            "started {} (attempt {}, {})\n",
                            l.node, l.attempt, l.executor
                        ));
                        self.graph.mark_running(&l.node, &self.actor);
                        live.push(l);
                        started += 1;
                        peak = peak.max(live.len());
                    }
                    None => {
                        self.passed_over.insert(id);
                    }
                }
            }
            if live.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
            let mut still = Vec::new();
            for mut l in std::mem::take(&mut live) {
                if l.last_renew.elapsed() > lease / 3 {
                    self.renew(&l);
                    l.last_renew = Instant::now();
                }
                match self.poll(&mut l) {
                    Some(ending) => self.conclude(l, ending)?,
                    None => still.push(l),
                }
            }
            live = still;
        }
        let stop = stop.unwrap_or_else(|| self.why_nothing_ready());
        Ok((stop, peak))
    }

    /// The stop when nothing more could start.
    fn why_nothing_ready(&self) -> Stop {
        let open: Vec<&crate::graph::Node> = self
            .graph
            .nodes
            .iter()
            .filter(|n| n.runnable && n.state != State::Done)
            .collect();
        if open.is_empty() {
            return Stop {
                reason: "done",
                detail: "nothing is left to run; every node is done".to_owned(),
            };
        }
        let count = |s: State| open.iter().filter(|n| n.state == s).count();
        Stop {
            reason: "nothing-ready",
            detail: format!(
                "nothing is ready: {} waiting on other work, {} blocked, {} held by someone else, \
                 {} not started by this run",
                count(State::Waiting),
                count(State::Blocked),
                count(State::Running),
                open.iter()
                    .filter(|n| n.state == State::Ready && self.passed_over.contains(&n.id))
                    .count()
            ),
        }
    }

    /// Nodes another run holds: landed since (its marker), or given back.
    fn sync_foreign(&mut self, live: &[Live]) {
        let mine: BTreeSet<&str> = live.iter().map(|l| l.node.as_str()).collect();
        let foreign: Vec<String> = self
            .graph
            .nodes
            .iter()
            .filter(|n| n.state == State::Running && !mine.contains(n.id.as_str()))
            .map(|n| n.id.clone())
            .collect();
        if foreign.is_empty() {
            return;
        }
        let claims = self.store.claims().unwrap_or_default();
        let now = crate::ticket::now_secs();
        for id in foreign {
            if let Some(common) = &self.common
                && ledger::landed(common, &id).is_some()
            {
                self.graph.mark_done(&id);
                continue;
            }
            let (t, item) = openwarrant_core::ticket::split_record_id(&id);
            let held = |item: Option<&str>| {
                crate::ticket::claim_on(&claims, t, item)
                    .is_some_and(|(_, c)| c.is_none_or(|c| !c.lease_expired(now)))
            };
            let kind = self.graph.node(&id).map(|n| n.kind);
            if kind != Some(Kind::Stage) && !held(item) && !held(None) {
                self.graph.release(&id);
            }
        }
    }

    fn renew(&self, l: &Live) {
        if let Some(name) = l.lock.file_name() {
            self.store.renew_held(Some(name));
        }
    }

    // ---- start ------------------------------------------------------------

    fn skip(&mut self, node: &str, outcome: &str, detail: String) {
        self.human
            .push_str(&format!("not started {node}: {detail}\n"));
        self.skipped.push(Row {
            node: node.to_owned(),
            outcome: outcome.to_owned(),
            attempt: 0,
            detail,
            seconds: 0,
            level: None,
            commit: None,
            tokens: tokens_value(None),
            executor: String::new(),
        });
    }

    /// Claim `id`, give it a worktree, hand it over. `None` when it could not
    /// start; the run has recorded why and does not try it again.
    fn launch(&mut self, id: &str) -> Result<Option<Live>, RepoError> {
        let Some(node) = self.graph.node(id).cloned() else {
            return Ok(None);
        };
        if node.kind == Kind::Stage {
            return self.launch_stage(&node);
        }
        let Some(tid) = node.warrant.clone() else {
            return Ok(None);
        };
        let item = openwarrant_core::ticket::split_record_id(id)
            .1
            .map(str::to_owned);
        let Some(dir) = self.ticket_dir(&tid) else {
            self.skip(id, "unknown", format!("{tid} could not be read"));
            return Ok(None);
        };
        let t = match self.store.load(&dir) {
            Ok(t) => t,
            Err(d) => {
                self.skip(id, "unknown", d.message);
                return Ok(None);
            }
        };
        let journal = crate::journal_cmd::load(&t.dir)?;
        let before = attempts_in_row(&journal, id);
        if before >= self.policy.max_attempts {
            self.set_aside(
                &t,
                id,
                format!("{before} attempt(s) in a row ended without landing"),
                before,
            )?;
            return Ok(None);
        }
        // Claim, through M11's shared store: a second run (another worktree,
        // or another clone with `[claims] remote`) is refused here by name.
        let claimed = crate::ticket::claim_cmd(&self.store, id, false)?;
        if claimed.is_refused() {
            let d = claimed.report.diagnostics.first();
            let rule = d.map_or("ticket.claim", |d| d.rule.as_str()).to_owned();
            let message = d.map_or_else(String::new, |d| d.message.clone());
            self.report.push(Diagnostic::warn(
                rule.clone(),
                self.rel(&t.checklist_path),
                format!("not started: {message}"),
            ));
            self.skip(id, "held", format!("{rule}: {message}"));
            return Ok(None);
        }
        let t = self.store.load(&dir).unwrap_or(t);
        let revision = match &item {
            Some(i) => t.revision_for(Some(i)).unwrap_or_default(),
            None => t.revision.clone(),
        };
        let lock = self
            .store
            .claims_dir
            .join(crate::ticket::claim::lock_name(&tid, item.as_deref()));
        let (worktree, branch, base) = match self.workspace(id, &node.warrant) {
            Ok(w) => w,
            Err(e) => {
                let _ = crate::ticket::release(&self.store, id, None);
                self.report.push(Diagnostic::error(
                    "go.worktree",
                    self.rel(&self.repo.root.join(&self.policy.worktrees)),
                    format!("{id}: {e}"),
                ));
                self.skip(id, "worktree", e);
                return Ok(None);
            }
        };
        let program = worktree.join(&self.below);
        let checks = crate::ticket::ladder::checks_of(&t);
        let (tests, kpis) = checks.for_item(item.as_deref());
        let mut deciding: Vec<packet::Check> = tests
            .iter()
            .map(|c| packet::Check {
                name: c.name.clone(),
                cmd: c.cmd.clone(),
            })
            .collect();
        deciding.extend(kpis.iter().filter(|k| k.gates()).map(|k| packet::Check {
            name: k.name.clone(),
            cmd: k.cmd.clone(),
        }));
        let dispatch_id = WarUuid::mint().to_string();
        let attempt_id = WarUuid::mint().to_string();
        let exec = self.policy.executor_for(&node.labels).cloned();
        let executor = exec
            .as_ref()
            .map_or_else(|| "native".to_owned(), |e| e.name.clone());
        let mut p = packet::Packet {
            schema: packet::SCHEMA,
            dispatch_id: dispatch_id.clone(),
            attempt_id: attempt_id.clone(),
            attempt: before + 1,
            node: id.to_owned(),
            kind: node.kind.as_str(),
            warrant: tid.clone(),
            title: t.manifest.title.clone(),
            text: node.text.clone(),
            description: crate::ticket::description(&t.intent),
            notes: crate::ticket::notes(&t.intent),
            depends_on: self.graph.deps(id).map(str::to_owned).collect(),
            checks: deciding,
            allowed: packet::allowed_of_ticket(&t),
            revision: revision.clone(),
            worktree: program.to_string(),
            branch: branch.clone().unwrap_or_default(),
            base: base.clone(),
            root: self.repo.root.to_string(),
            answer: packet::Answer {
                dispatch_id: dispatch_id.clone(),
                attempt_id: attempt_id.clone(),
                contract_digest: revision.clone(),
                stage_id: id.to_owned(),
                requested_next_action: packet::ACTIONS,
            },
            brief: String::new(),
        };
        p.brief = packet::brief(&p);
        let packet_json = serde_json::to_string_pretty(&p).unwrap_or_default();
        let packet_path = program.join(".openwarrant").join("go-packet.json");
        let _ = std::fs::create_dir_all(program.join(".openwarrant"));
        let _ = std::fs::write(&packet_path, &packet_json);
        // The bindings its answer must carry, before anything is handed over.
        self.store.journal(
            &t,
            event::DISPATCHED,
            &serde_json::json!({
                "node": id,
                "dispatch_id": dispatch_id,
                "attempt_id": attempt_id,
                "contract_digest": revision,
                "stage": id,
                "attempt": before + 1,
                "branch": branch,
                "base": base,
                "executor": executor,
                "run": self.run_id,
            }),
        )?;
        let env: Vec<(String, String)> = vec![
            ("WAR_GO_NODE".into(), id.to_owned()),
            ("WAR_GO_PACKET".into(), packet_path.to_string()),
            ("WAR_GO_ROOT".into(), self.repo.root.to_string()),
            ("WAR_GO_WORKTREE".into(), program.to_string()),
            (
                "OPENWARRANT_ACTOR".into(),
                format!("{}.performer", self.actor),
            ),
        ];
        let mut live = Live {
            node: id.to_owned(),
            kind: node.kind,
            warrant: tid.clone(),
            part: item,
            worktree: worktree.clone(),
            program: program.clone(),
            branch,
            dispatch_id,
            attempt: before + 1,
            lock,
            last_renew: Instant::now(),
            started: Instant::now(),
            executor,
            via: Via::External {
                exec: policy::Executor {
                    name: String::new(),
                    adapter: String::new(),
                    dispatch_argv: Vec::new(),
                    poll_argv: Vec::new(),
                    stdin: policy::Input::Packet,
                    poll_secs: 1,
                },
                reference: String::new(),
                next_poll: Instant::now(),
            },
        };
        match exec {
            None => {
                let values = [
                    ("node", id),
                    ("worktree", program.as_str()),
                    ("packet", packet_path.as_str()),
                    ("title", p.title.as_str()),
                    ("brief", p.brief.as_str()),
                ];
                let argv = proc::fill(&self.policy.harness_argv, &values);
                let stdin = match self.policy.harness {
                    policy::Harness::Claude => p.brief.clone(),
                    policy::Harness::Generic => packet_json.clone(),
                };
                match proc::spawn(&argv, &program, stdin.as_bytes(), &env) {
                    Ok(r) => live.via = Via::Native(Some(r)),
                    Err(e) => {
                        self.conclude(
                            live,
                            Ending::Crashed {
                                how: e,
                                stderr: None,
                            },
                        )?;
                        return Ok(None);
                    }
                }
            }
            Some(exec) => {
                let input = match exec.stdin {
                    policy::Input::Packet => packet_json.clone(),
                    policy::Input::Brief => p.brief.clone(),
                    policy::Input::Beads => self.beads_line(&tid, id),
                };
                let argv = proc::fill(
                    &exec.dispatch_argv,
                    &[
                        ("node", id),
                        ("title", &format!("{}: {}", p.title, p.text)),
                        ("packet", packet_path.as_str()),
                        ("branch", &p.branch),
                    ],
                );
                let ended = proc::run_bounded(
                    &argv,
                    &program,
                    input.as_bytes(),
                    &env,
                    Duration::from_secs(120),
                );
                let reference = match &ended {
                    Ok(e) if e.status.is_some_and(|s| s.success()) => e
                        .stdout
                        .lines()
                        .rfind(|l| !l.trim().is_empty())
                        .map(|l| l.trim().to_owned()),
                    _ => None,
                };
                match reference {
                    Some(reference) => {
                        self.human.push_str(&format!(
                            "handed {id} to {} ({}): {reference}\n",
                            exec.name, exec.adapter
                        ));
                        live.via = Via::External {
                            next_poll: Instant::now() + Duration::from_secs(exec.poll_secs),
                            exec,
                            reference,
                        };
                    }
                    None => {
                        let why = match ended {
                            Ok(e) => format!(
                                "the dispatch {} and printed no reference{}",
                                e.how(),
                                e.last_stderr()
                                    .map(|l| format!(" (stderr: {l})"))
                                    .unwrap_or_default()
                            ),
                            Err(e) => e,
                        };
                        self.conclude(live, Ending::AdapterFailed(why))?;
                        return Ok(None);
                    }
                }
            }
        }
        Ok(Some(live))
    }

    fn ticket_dir(&self, tid: &str) -> Option<Utf8PathBuf> {
        let d = self.store.dir.join(tid);
        crate::vfs::is_file(d.join("manifest.toml")).then_some(d)
    }

    /// The node's Beads issue line, as `war admin export beads` writes it.
    fn beads_line(&self, tid: &str, node: &str) -> String {
        let Ok((jsonl, _)) = crate::interop::beads::export(&self.store) else {
            return String::new();
        };
        let item_index = self
            .store
            .load(&self.store.dir.join(tid))
            .ok()
            .and_then(|t| {
                let (_, item) = openwarrant_core::ticket::split_record_id(node);
                item.and_then(|i| {
                    t.checklist
                        .items
                        .iter()
                        .position(|x| x.id.as_deref() == Some(i))
                })
            });
        let wanted: Vec<String> = match item_index {
            Some(n) => vec![format!(".{}", n + 1)],
            None => Vec::new(),
        };
        jsonl
            .lines()
            .find(|l| {
                let v: serde_json::Value = serde_json::from_str(l).unwrap_or_default();
                let id = v
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let ext = v
                    .get("external_ref")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                (id.starts_with(tid) || ext.contains(tid) || l.contains(tid))
                    && (wanted.is_empty() || wanted.iter().any(|w| id.ends_with(w.as_str())))
            })
            .or_else(|| jsonl.lines().find(|l| l.contains(tid)))
            .map(|l| format!("{l}\n"))
            .unwrap_or_default()
    }

    /// The worktree a node runs in: its own, on `war-go/<node>` from the
    /// integration branch (worktree isolation), or this checkout (shared).
    fn workspace(
        &self,
        id: &str,
        _warrant: &Option<String>,
    ) -> Result<(Utf8PathBuf, Option<String>, String), String> {
        if self.policy.isolation == Isolation::Shared {
            let head = git::rev(&self.repo.root, "HEAD").unwrap_or_default();
            return Ok((self.repo.root.clone(), None, head));
        }
        let root = &self.repo.root;
        let base = git::ensure_branch(root, &self.policy.integration)?;
        let path = root.join(&self.policy.worktrees).join(git::slug(id));
        let branch = git::node_branch(id);
        git::add_worktree(root, &path, &branch, &base)?;
        // Stacked work (`land = "pr"`): what this node waits on may be on its
        // own branch rather than the integration branch yet.
        for dep in self.graph.deps(id) {
            let dep_branch = git::node_branch(dep);
            if let Some(tip) = git::branch_tip(root, &dep_branch)
                && !git::is_ancestor(root, &tip, &base)
                && let Err(files) =
                    git::merge(&path, &dep_branch, &format!("war go: {dep} under {id}"))
            {
                git::remove_worktree(root, &path, Some(&branch));
                return Err(format!(
                    "the branch of {dep}, which it waits on, does not merge cleanly: {}",
                    files.join(", ")
                ));
            }
        }
        Ok((path, Some(branch), base))
    }

    /// An agent stage: `war evidence perform`'s admissions, its Dispatch, its
    /// performer, in a worktree.
    fn launch_stage(&mut self, node: &crate::graph::Node) -> Result<Option<Live>, RepoError> {
        let id = node.id.clone();
        let Some((alias, stage)) = id.split_once('/') else {
            return Ok(None);
        };
        let (alias, stage) = (alias.to_owned(), stage.to_owned());
        let dir = self.repo.warrant_dir(&alias)?;
        let mut admit = Report::default();
        if self.repo.config.perform.performer_argv.is_empty() {
            admit.push(Diagnostic::error(
                "perform.no-performer",
                crate::init::CONFIG_FILE.to_owned(),
                format!(
                    "{id}: no stage performer is configured; set [perform] performer_argv (the \
                     Dispatch on stdin, a Stage Submission on stdout)"
                ),
            ));
        }
        let kind = crate::perform::hotline_admitted(self.repo, &dir, &alias, &stage, &mut admit)?;
        let clear = kind.is_some()
            && admit.count(crate::diagnostic::Severity::Error) == 0
            && crate::perform::handoff(self.repo, &dir, &alias, &stage, &mut admit);
        if !clear {
            let why: Vec<String> = admit
                .diagnostics
                .iter()
                .filter(|d| d.severity != crate::diagnostic::Severity::Pass)
                .map(|d| format!("{}: {}", d.rule, d.message))
                .collect();
            for d in admit.diagnostics {
                if d.severity != crate::diagnostic::Severity::Pass {
                    self.report.push(Diagnostic::warn(
                        d.rule,
                        d.file.unwrap_or_default(),
                        format!("not started: {}", d.message),
                    ));
                }
            }
            self.graph.block(&id, why.join("; "));
            self.skip(&id, "for-a-person", why.join("; "));
            return Ok(None);
        }
        let kind = kind.unwrap_or(crate::perform::Attempt::Initial);
        let now = crate::ticket::now_secs();
        let lock = self
            .store
            .claims_dir
            .join(crate::ticket::claim::lock_name(&alias, Some(&stage)));
        let claim = crate::ticket::claim::Claim {
            schema: crate::ticket::claim::CLAIM_SCHEMA.to_owned(),
            claim: WarUuid::mint().to_string(),
            ticket: alias.clone(),
            item: Some(stage.clone()),
            actor: self.actor.clone(),
            since: crate::gate_cmd::receipt::rfc3339_from_secs(now),
            since_unix: now,
            lease_until: Some(crate::gate_cmd::receipt::rfc3339_from_secs(
                now + self.store.lease_secs,
            )),
            lease_until_unix: Some(now + self.store.lease_secs),
        };
        match crate::ticket::claim::take(&lock, &claim) {
            Ok(crate::ticket::claim::Taken::Won) => {}
            Ok(crate::ticket::claim::Taken::Held(c)) => {
                let who = c.map_or_else(|| "somebody".to_owned(), |c| c.actor);
                self.skip(&id, "held", format!("{id} is claimed by {who}"));
                return Ok(None);
            }
            Err(e) => {
                self.skip(&id, "held", format!("could not claim {id}: {e}"));
                return Ok(None);
            }
        }
        let (worktree, branch, _base) = match self.workspace(&id, &node.warrant) {
            Ok(w) => w,
            Err(e) => {
                let _ = crate::ticket::claim::release(&lock, &self.actor);
                self.skip(&id, "worktree", e);
                return Ok(None);
            }
        };
        let program = worktree.join(&self.below);
        let mut compiled = Report::default();
        let Some(dispatch) = crate::perform::compile(
            self.repo,
            &alias,
            &stage,
            &dir,
            &mut compiled,
            self.prototype,
        )?
        else {
            let why: Vec<String> = compiled
                .diagnostics
                .iter()
                .filter(|d| d.severity != crate::diagnostic::Severity::Pass)
                .map(|d| format!("{}: {}", d.rule, d.message))
                .collect();
            let _ = crate::ticket::claim::release(&lock, &self.actor);
            if let Some(b) = &branch {
                git::remove_worktree(&self.repo.root, &worktree, Some(b));
            }
            self.graph.block(&id, why.join("; "));
            self.skip(&id, "for-a-person", why.join("; "));
            return Ok(None);
        };
        let body = serde_json::to_string_pretty(&dispatch).unwrap_or_default();
        let env: Vec<(String, String)> = vec![
            ("WAR_GO_NODE".into(), id.clone()),
            ("WAR_GO_ROOT".into(), self.repo.root.to_string()),
            ("WAR_GO_WORKTREE".into(), program.to_string()),
        ];
        let argv = self.repo.config.perform.performer_argv.clone();
        let running = match proc::spawn(&argv, &program, body.as_bytes(), &env) {
            Ok(r) => r,
            Err(e) => {
                let _ = crate::ticket::claim::release(&lock, &self.actor);
                if let Some(b) = &branch {
                    git::remove_worktree(&self.repo.root, &worktree, Some(b));
                }
                self.skip(&id, "crashed", e);
                return Ok(None);
            }
        };
        // The writer record, as `war evidence perform` keeps one.
        let writer = crate::perform::writer_path(&dir, &dispatch.dispatch_id);
        let record = crate::perform::WriterRecord {
            pid: running.pid,
            pgid: running.pid,
            leader_started: crate::perform::leader_started(running.pid).unwrap_or_default(),
            stage_id: stage.clone(),
            dispatch_id: dispatch.dispatch_id.clone(),
        };
        let _ = serde_json::to_string_pretty(&record).map(|t| std::fs::write(&writer, t));
        let uuid = self
            .repo
            .load_warrant(&dir)
            .ok()
            .and_then(|l| l.validated.map(|v| v.uuid.to_string()));
        Ok(Some(Live {
            node: id.clone(),
            kind: Kind::Stage,
            warrant: alias,
            part: Some(stage),
            worktree,
            program,
            branch,
            dispatch_id: dispatch.dispatch_id.clone(),
            attempt: 1,
            lock,
            last_renew: Instant::now(),
            started: Instant::now(),
            executor: "native ([perform] performer_argv)".to_owned(),
            via: Via::Stage {
                running: Some(running),
                kind,
                dir,
                uuid,
                writer,
            },
        }))
    }

    // ---- watch ------------------------------------------------------------

    /// `Some` once the attempt has ended.
    fn poll(&mut self, l: &mut Live) -> Option<Ending> {
        let bound = Duration::from_secs(self.policy.node_timeout_secs);
        let timed_out = l.started.elapsed() > bound;
        match &mut l.via {
            Via::Native(slot) | Via::Stage { running: slot, .. } => {
                let Some(r) = slot.as_mut() else {
                    return Some(Ending::Crashed {
                        how: "is gone".to_owned(),
                        stderr: None,
                    });
                };
                if let Some(status) = r.exited() {
                    return slot
                        .take()
                        .map(|r| Self::ended_native(r.finish(Some(status))));
                }
                if r.flooded() {
                    r.kill();
                    let _ = slot.take().map(|r| r.finish(None));
                    return Some(Ending::Flooded);
                }
                if timed_out {
                    r.kill();
                    let _ = slot.take().map(|r| r.finish(None));
                    return Some(Ending::Timeout);
                }
                None
            }
            Via::External {
                exec,
                reference,
                next_poll,
            } => {
                if timed_out {
                    return Some(Ending::Timeout);
                }
                if Instant::now() < *next_poll {
                    return None;
                }
                *next_poll = Instant::now() + Duration::from_secs(exec.poll_secs);
                let argv = proc::fill(
                    &exec.poll_argv,
                    &[("ref", reference.as_str()), ("node", &l.node)],
                );
                match proc::run_bounded(&argv, &l.program, b"", &[], Duration::from_secs(60)) {
                    Err(e) => Some(Ending::AdapterFailed(e)),
                    Ok(e) if !e.status.is_some_and(|s| s.success()) => {
                        Some(Ending::AdapterFailed(format!(
                            "the poll {}{}",
                            e.how(),
                            e.last_stderr()
                                .map(|l| format!(" (stderr: {l})"))
                                .unwrap_or_default()
                        )))
                    }
                    Ok(e) => match adapter_answer(&e.stdout) {
                        AdapterSays::Pending => None,
                        AdapterSays::Submitted(text) => Some(Ending::Answer { text, tokens: None }),
                        AdapterSays::Other(d) => Some(Ending::AdapterFailed(d)),
                    },
                }
            }
        }
    }

    fn ended_native(e: proc::Ended) -> Ending {
        if e.oversized {
            return Ending::Flooded;
        }
        match e.status {
            Some(s) if s.success() && !e.stdout.trim().is_empty() => {
                let (text, tokens) = packet::answer_in(&e.stdout);
                Ending::Answer { text, tokens }
            }
            _ => Ending::Crashed {
                how: if e.status.is_some_and(|s| s.success()) {
                    "exited 0 and printed no answer".to_owned()
                } else {
                    e.how()
                },
                stderr: e.last_stderr().map(str::to_owned),
            },
        }
    }

    fn cancel(&mut self, mut l: Live) {
        if let Via::Native(slot) | Via::Stage { running: slot, .. } = &mut l.via
            && let Some(r) = slot.take()
        {
            let mut r = r;
            r.kill();
            let _ = r.finish(None);
        }
        let _ = self.conclude(l, Ending::Cancelled);
    }

    // ---- judge ------------------------------------------------------------

    /// What happens to an attempt that ended.
    fn conclude(&mut self, l: Live, ending: Ending) -> Result<(), RepoError> {
        if l.kind == Kind::Stage {
            return self.conclude_stage(l, ending);
        }
        let seconds = l.started.elapsed().as_secs();
        let (text, tokens) = match ending {
            Ending::Answer { text, tokens } => (text, tokens),
            Ending::Crashed { how, stderr } => {
                let detail = format!(
                    "the performer {how} without an answer{}",
                    stderr
                        .map(|s| format!("; its last stderr line: {s}"))
                        .unwrap_or_default()
                );
                self.report.push(Diagnostic::warn(
                    "go.performer-crashed",
                    l.node.clone(),
                    detail.clone(),
                ));
                return self.send_back(&l, "crashed", detail, seconds, None);
            }
            Ending::Timeout => {
                let detail = format!(
                    "it ran past its time budget ([go] node_timeout_secs = {}) and was stopped",
                    self.policy.node_timeout_secs
                );
                self.report.push(Diagnostic::warn(
                    "go.timeout",
                    l.node.clone(),
                    detail.clone(),
                ));
                return self.send_back(&l, "timeout", detail, seconds, None);
            }
            Ending::Flooded => {
                let detail = "the performer wrote more than an answer can be (8 MiB) and was \
                              stopped"
                    .to_owned();
                self.report.push(Diagnostic::warn(
                    "go.performer-crashed",
                    l.node.clone(),
                    detail.clone(),
                ));
                return self.send_back(&l, "crashed", detail, seconds, None);
            }
            Ending::AdapterFailed(why) => {
                let detail = format!("the {} executor: {why}", l.executor);
                self.report.push(Diagnostic::warn(
                    "go.adapter-failed",
                    l.node.clone(),
                    detail.clone(),
                ));
                return self.send_back(&l, "adapter-failed", detail, seconds, None);
            }
            Ending::Cancelled => {
                let detail = "the run was cancelled; it is open and unclaimed".to_owned();
                return self.send_back(&l, "cancelled", detail, seconds, None);
            }
        };
        // The same refusals `war evidence submit` applies, whoever answered.
        let Some(dir) = self.ticket_dir(&l.warrant) else {
            return Ok(());
        };
        let journal_rel = self.rel(&dir.join(crate::journal_cmd::FILE));
        let file = format!("{} answer for {}", l.executor, l.node);
        let bindings = || -> Result<Vec<serde_json::Value>, RepoError> {
            Ok(crate::journal_cmd::load(&dir)?
                .events
                .iter()
                .filter(|e| e.event_type == event::DISPATCHED)
                .map(payload)
                .collect())
        };
        let submission =
            match crate::run_cmd::admit_answer(&l.node, &file, &journal_rel, &text, bindings)? {
                Ok(s) => s,
                Err(d) => {
                    let detail = format!("{}: {}", d.rule, d.message);
                    self.report.push(d);
                    return self.send_back(&l, "refused", detail, seconds, tokens);
                }
            };
        if submission.dispatch_id != l.dispatch_id {
            let d = Diagnostic::error(
                "submission.dispatch-mismatch",
                file.clone(),
                format!(
                    "{}: the answer is for dispatch {}, an earlier attempt; this attempt is {}. \
                     Nothing was written",
                    l.node, submission.dispatch_id, l.dispatch_id
                ),
            );
            let detail = format!("{}: {}", d.rule, d.message);
            self.report.push(d);
            return self.send_back(&l, "refused", detail, seconds, tokens);
        }
        // Tokens: enforced where the harness reports them, UNKNOWN otherwise.
        if let Some(budget) = self.policy.node_tokens {
            match tokens {
                Some(n) if n > budget => {
                    let d = Diagnostic::error(
                        "go.token-budget",
                        l.node.clone(),
                        format!(
                            "{}: the harness reported {n} tokens, over [go] node_tokens = {budget}; \
                             the work was not landed",
                            l.node
                        ),
                    );
                    let detail = d.message.clone();
                    self.report.push(d);
                    return self.send_back(&l, "token-budget", detail, seconds, tokens);
                }
                Some(_) => {}
                None => {
                    if self.tokens_unknown.insert(l.node.clone()) {
                        self.report.push(Diagnostic::unknown(
                            "go.tokens-unknown",
                            l.node.clone(),
                            format!(
                                "{}: [go] node_tokens is {budget}, and the {} harness reported no \
                                 usage, so whether the budget held is not known. Unknown is not \
                                 zero",
                                l.node, l.executor
                            ),
                        ));
                    }
                }
            }
        }
        match submission.requested_next_action {
            Some(RequestedNextAction::Verify) => self.land(&l, &submission, seconds, tokens),
            Some(
                action @ (RequestedNextAction::Block
                | RequestedNextAction::Amend
                | RequestedNextAction::Cancel),
            ) => {
                let why = submission
                    .blockers
                    .iter()
                    .map(|b| b.reason.clone())
                    .chain(submission.unresolved_items.iter().cloned())
                    .collect::<Vec<_>>()
                    .join("; ");
                let reason = format!(
                    "the performer asked to {action}{}",
                    if why.is_empty() {
                        String::new()
                    } else {
                        format!(": {why}")
                    }
                );
                self.ended_event(&l, &action.to_string(), &reason, seconds, tokens)?;
                self.release_node(&l, true);
                if let Some(t) = self.load_ticket(&l.warrant) {
                    self.set_aside(&t, &l.node, reason.clone(), l.attempt)?;
                }
                Ok(())
            }
            Some(RequestedNextAction::Continue) | None => self.send_back(
                &l,
                "continue",
                "the performer asked to continue; the work was not landed".to_owned(),
                seconds,
                tokens,
            ),
        }
    }

    fn load_ticket(&self, tid: &str) -> Option<Ticket> {
        self.ticket_dir(tid).and_then(|d| self.store.load(&d).ok())
    }

    /// Journal `go.ended`.
    fn ended_event(
        &self,
        l: &Live,
        outcome: &str,
        detail: &str,
        seconds: u64,
        tokens: Option<u64>,
    ) -> Result<(), RepoError> {
        let Some(t) = self.load_ticket(&l.warrant) else {
            return Ok(());
        };
        self.store.journal(
            &t,
            event::ENDED,
            &serde_json::json!({
                "node": l.node,
                "dispatch_id": l.dispatch_id,
                "outcome": outcome,
                "attempt": l.attempt,
                "seconds": seconds,
                "tokens": tokens_value(tokens),
                "detail": detail,
                "run": self.run_id,
            }),
        )?;
        Ok(())
    }

    /// Give the claim back and clear the worktree. `keep_branch` keeps the
    /// node's branch for a person to read.
    fn release_node(&self, l: &Live, keep_branch: bool) {
        let _ = crate::ticket::release(&self.store, &l.node, None);
        if let Some(b) = &l.branch {
            git::remove_worktree(
                &self.repo.root,
                &l.worktree,
                (!keep_branch).then_some(b.as_str()),
            );
        }
    }

    /// An attempt that did not land: journalled, noted on the Warrant,
    /// released, and back on the frontier, or set aside for a person once
    /// `[go] max_attempts` in a row have not landed.
    fn send_back(
        &mut self,
        l: &Live,
        outcome: &str,
        detail: String,
        seconds: u64,
        tokens: Option<u64>,
    ) -> Result<(), RepoError> {
        self.ended_event(l, outcome, &detail, seconds, tokens)?;
        self.release_node(l, false);
        let row = Row {
            node: l.node.clone(),
            outcome: outcome.to_owned(),
            attempt: l.attempt,
            detail: detail.clone(),
            seconds,
            level: None,
            commit: None,
            tokens: tokens_value(tokens),
            executor: l.executor.clone(),
        };
        if outcome == "cancelled" {
            self.graph.release(&l.node);
            self.requeued.push(row);
            return Ok(());
        }
        let max = self.policy.max_attempts;
        if l.attempt >= max {
            if let Some(t) = self.load_ticket(&l.warrant) {
                self.set_aside(
                    &t,
                    &l.node,
                    format!(
                        "{} attempt(s) in a row ended without landing; the last: {detail}",
                        l.attempt
                    ),
                    l.attempt,
                )?;
            }
            return Ok(());
        }
        if let Some(t) = self.load_ticket(&l.warrant) {
            let _ = crate::ticket::note(
                &self.store,
                t.id(),
                &format!(
                    "war go: {} attempt {} of {} did not land ({outcome}): {detail}. It is back \
                     on the frontier.",
                    l.node, l.attempt, max
                ),
                None,
            );
        }
        self.human.push_str(&format!(
            "sent back {} (attempt {} of {max}, {outcome}): {detail}\n",
            l.node, l.attempt
        ));
        self.graph.release(&l.node);
        self.requeued.push(row);
        Ok(())
    }

    /// Set a node aside for a person: journalled, noted, blocked in the
    /// graph, and listed by `war next`.
    fn set_aside(
        &mut self,
        t: &Ticket,
        node: &str,
        reason: String,
        attempts: u32,
    ) -> Result<(), RepoError> {
        self.store.journal(
            t,
            event::BLOCKED,
            &serde_json::json!({
                "node": node,
                "reason": reason,
                "attempts": attempts,
                "run": self.run_id,
                "at": now_rfc3339(),
            }),
        )?;
        let _ = crate::ticket::note(
            &self.store,
            t.id(),
            &format!(
                "war go: {node} is set aside for a person: {reason}. `war evidence go --retry \
                 {node}` puts it back on the frontier."
            ),
            None,
        );
        self.report.push(Diagnostic::warn(
            "go.blocked",
            node.to_owned(),
            format!("{node} is set aside for a person: {reason}"),
        ));
        self.human
            .push_str(&format!("set aside {node} for a person: {reason}\n"));
        self.graph
            .block(node, format!("set aside for a person: {reason}"));
        self.passed_over.insert(node.to_owned());
        self.blocked.push(Row {
            node: node.to_owned(),
            outcome: "blocked".to_owned(),
            attempt: attempts,
            detail: reason,
            seconds: 0,
            level: None,
            commit: None,
            tokens: tokens_value(None),
            executor: String::new(),
        });
        Ok(())
    }

    // ---- land -------------------------------------------------------------

    /// Commit the worktree, merge the integration branch into it, run the
    /// checks there, move the integration branch (or open a pull request),
    /// and tick: all under the landing lock.
    fn land(
        &mut self,
        l: &Live,
        sub: &StageSubmission,
        seconds: u64,
        tokens: Option<u64>,
    ) -> Result<(), RepoError> {
        let shared = self.policy.isolation == Isolation::Shared;
        let _lock = match self.common.as_deref().map(ledger::land_lock) {
            Some(Err(e)) => {
                return self.send_back(l, "land-failed", e, seconds, tokens);
            }
            Some(Ok(f)) => Some(f),
            None => None,
        };
        let integration = self.policy.integration.clone();
        let root = self.repo.root.clone();
        let mut head = git::rev(&l.worktree, "HEAD").unwrap_or_default();
        let mut old_tip = String::new();
        if !shared {
            let message = format!("war go: {} ({})", l.node, l.dispatch_id);
            match git::commit_all(&l.worktree, &message, packet::SESSION_FILES) {
                Ok(Some(c)) => head = c,
                Ok(None) => {}
                Err(e) => return self.send_back(l, "land-failed", e, seconds, tokens),
            }
            // An external executor's work, named as a git ref in its answer.
            for a in sub
                .artifact_refs
                .iter()
                .filter_map(|a| a.strip_prefix("git:"))
            {
                let fetched = git::git(&l.worktree, &["fetch", "-q", &self.policy.remote, a]);
                if !fetched.ok {
                    let detail = format!(
                        "the answer names {a}, and it could not be fetched from {}: {}",
                        self.policy.remote,
                        fetched.why()
                    );
                    return self.send_back(l, "land-failed", detail, seconds, tokens);
                }
                if let Err(files) = git::merge(
                    &l.worktree,
                    "FETCH_HEAD",
                    &format!("war go: {} from {a}", l.node),
                ) {
                    let detail = format!("the work at {a} conflicts in {}", files.join(", "));
                    self.report.push(Diagnostic::warn(
                        "go.conflict",
                        l.node.clone(),
                        detail.clone(),
                    ));
                    return self.send_back(l, "conflict", detail, seconds, tokens);
                }
            }
            old_tip = git::branch_tip(&root, &integration).unwrap_or_default();
            if !old_tip.is_empty() && !git::is_ancestor(&l.worktree, &old_tip, "HEAD") {
                if let Err(files) = git::merge(
                    &l.worktree,
                    &old_tip,
                    &format!("war go: {} onto {integration}", l.node),
                ) {
                    let detail = format!(
                        "merging {integration} into its branch conflicted in {}",
                        files.join(", ")
                    );
                    self.report.push(Diagnostic::warn(
                        "go.conflict",
                        l.node.clone(),
                        detail.clone(),
                    ));
                    return self.send_back(l, "conflict", detail, seconds, tokens);
                }
            }
            head = git::rev(&l.worktree, "HEAD").unwrap_or(head);
        }
        // The checks, in the worktree that holds the work, merged.
        let Some(t) = self.load_ticket(&l.warrant) else {
            return self.send_back(
                l,
                "land-failed",
                "the Warrant could not be read".into(),
                seconds,
                tokens,
            );
        };
        let checks = crate::ticket::ladder::checks_of(&t);
        let (tests, kpis) = checks.for_item(l.part.as_deref());
        let deciding = tests.len() + kpis.iter().filter(|k| k.gates()).count();
        let round = if deciding > 0 {
            self.store.check_root = (!shared).then(|| l.program.clone());
            let r = crate::ticket::ladder::check_for_tick(
                &self.store,
                &t,
                l.part.as_deref(),
                &checks,
                &l.node,
            );
            self.store.check_root = None;
            match r? {
                Ok(round) => Some(round),
                Err(refusal) => {
                    let rule = refusal
                        .report
                        .diagnostics
                        .iter()
                        .find(|d| d.severity != crate::diagnostic::Severity::Pass)
                        .map_or("ticket.check-failed", |d| d.rule.as_str())
                        .to_owned();
                    let detail = format!("{rule}: {}", refusal.human);
                    self.report.push(Diagnostic::warn(
                        "go.check-failed",
                        l.node.clone(),
                        detail.clone(),
                    ));
                    return self.send_back(l, "check-failed", detail, seconds, tokens);
                }
            }
        } else {
            None
        };
        // Land.
        let mut pr = None;
        if !shared {
            match self.policy.land {
                Land::Merge => {
                    let moved = if old_tip.is_empty() {
                        git::git(&root, &["branch", "-f", &integration, &head])
                            .ok
                            .then_some(())
                            .ok_or_else(|| "could not create it".to_owned())
                    } else {
                        git::update_branch(&root, &integration, &head, &old_tip)
                    };
                    if let Err(e) = moved {
                        let detail = format!("{integration} could not be moved to {head}: {e}");
                        return self.send_back(l, "land-failed", detail, seconds, tokens);
                    }
                }
                Land::Pr => match self.open_pr(l, &t) {
                    Ok(url) => pr = Some(url),
                    Err(e) => return self.send_back(l, "land-failed", e, seconds, tokens),
                },
            }
        }
        let level = if round.is_some() {
            "observed"
        } else {
            "claimed"
        };
        if let Some(common) = &self.common {
            let _ = ledger::mark(
                common,
                &ledger::Landed {
                    node: l.node.clone(),
                    run: self.run_id.clone(),
                    actor: self.actor.clone(),
                    commit: head.clone(),
                    integration: integration.clone(),
                    level: level.to_owned(),
                    at: now_rfc3339(),
                },
            );
        }
        let short = &head[..head.len().min(12)];
        let note = match (&pr, shared) {
            (Some(url), _) => format!("war go: pull request {url} ({short})"),
            (None, true) => "war go: done in this checkout".to_owned(),
            (None, false) => format!("war go: landed {short} on {integration}"),
        };
        let ticked = crate::ticket::done_ran(&self.store, &l.node, Some(&note), round)?;
        if ticked.is_refused() {
            if let Some(common) = &self.common {
                ledger::unmark(common, &l.node);
            }
            let why = ticked
                .report
                .diagnostics
                .first()
                .map(|d| format!("{}: {}", d.rule, d.message))
                .unwrap_or_default();
            for d in ticked.report.diagnostics {
                self.report.push(d);
            }
            self.ended_event(l, "tick-refused", &why, seconds, tokens)?;
            self.release_node(l, true);
            self.set_aside(
                &t,
                &l.node,
                format!("its work landed at {short}, and the tick was refused ({why})"),
                l.attempt,
            )?;
            return Ok(());
        }
        for d in ticked.report.diagnostics {
            self.report.push(d);
        }
        self.ended_event(l, "landed", &note, seconds, tokens)?;
        if let Some(b) = &l.branch {
            // Merged work needs its branch no longer; a pull request does.
            let keep = self.policy.land == Land::Pr;
            git::remove_worktree(&root, &l.worktree, (!keep).then_some(b.as_str()));
        }
        self.human.push_str(&format!(
            "landed {} at {level}: {}\n",
            l.node,
            note.trim_start_matches("war go: ")
        ));
        self.graph.mark_done(&l.node);
        self.landed.push(Row {
            node: l.node.clone(),
            outcome: "landed".to_owned(),
            attempt: l.attempt,
            detail: note,
            seconds,
            level: Some(level.to_owned()),
            commit: Some(head),
            tokens: tokens_value(tokens),
            executor: l.executor.clone(),
        });
        Ok(())
    }

    /// `land = "pr"`: push the node's branch and run `[go] pr_argv`. The
    /// pull request's URL, or why there is none.
    fn open_pr(&self, l: &Live, t: &Ticket) -> Result<String, String> {
        let Some(branch) = &l.branch else {
            return Err("the node has no branch".to_owned());
        };
        let pushed = git::git(
            &l.worktree,
            &[
                "push",
                "-q",
                "--no-verify",
                &self.policy.remote,
                &format!("{branch}:{branch}"),
            ],
        );
        if !pushed.ok {
            return Err(format!(
                "could not push {branch} to {}: {}",
                self.policy.remote,
                pushed.why()
            ));
        }
        let base = self.policy.pr_base.clone().unwrap_or_else(|| {
            git::git(&self.repo.root, &["symbolic-ref", "--short", "HEAD"])
                .stdout
                .trim()
                .to_owned()
        });
        let title = format!("{}: {}", t.manifest.title, l.node);
        let body = format!(
            "Work for `{}`, from `war evidence go` (run {}). It waited on: {}.",
            l.node,
            self.run_id,
            {
                let d: Vec<&str> = self.graph.deps(&l.node).collect();
                if d.is_empty() {
                    "nothing".to_owned()
                } else {
                    d.join(", ")
                }
            }
        );
        let argv = proc::fill(
            &self.policy.pr_argv,
            &[
                ("branch", branch.as_str()),
                ("base", base.as_str()),
                ("title", title.as_str()),
                ("body", body.as_str()),
                ("node", l.node.as_str()),
            ],
        );
        let ended = proc::run_bounded(&argv, &l.worktree, b"", &[], Duration::from_secs(120))?;
        if !ended.status.is_some_and(|s| s.success()) {
            return Err(format!(
                "the pull request argv {}{}",
                ended.how(),
                ended
                    .last_stderr()
                    .map(|s| format!(": {s}"))
                    .unwrap_or_default()
            ));
        }
        Ok(ended
            .stdout
            .lines()
            .rfind(|l| !l.trim().is_empty())
            .unwrap_or("(no URL printed)")
            .trim()
            .to_owned())
    }

    /// A stage's attempt: ingested through `war evidence submit` itself.
    fn conclude_stage(&mut self, l: Live, ending: Ending) -> Result<(), RepoError> {
        let seconds = l.started.elapsed().as_secs();
        let Via::Stage {
            kind,
            dir,
            uuid,
            writer,
            ..
        } = &l.via
        else {
            return Ok(());
        };
        let (kind, dir, uuid, writer) = (*kind, dir.clone(), uuid.clone(), writer.clone());
        let _ = std::fs::remove_file(&writer);
        let stage = l.part.clone().unwrap_or_default();
        let alias = l.warrant.clone();
        let ended = |run: &mut Self, how: crate::perform::Ending| {
            crate::perform::journal_ended(
                run.repo,
                &dir,
                uuid.as_deref(),
                &stage,
                (&l.dispatch_id, seconds),
                kind,
                how,
                &mut run.report,
            );
        };
        let give_back = |run: &mut Self, outcome: &str, detail: String| {
            let _ = crate::ticket::claim::release(&l.lock, &run.actor);
            if let Some(b) = &l.branch {
                git::remove_worktree(&run.repo.root, &l.worktree, Some(b));
            }
            run.human
                .push_str(&format!("sent back {} ({outcome}): {detail}\n", l.node));
            run.graph.release(&l.node);
            // `war evidence perform`'s recovery limit decides the next start.
            run.requeued.push(Row {
                node: l.node.clone(),
                outcome: outcome.to_owned(),
                attempt: 1,
                detail,
                seconds,
                level: None,
                commit: None,
                tokens: tokens_value(None),
                executor: l.executor.clone(),
            });
        };
        let text = match ending {
            Ending::Answer { text, .. } => text,
            Ending::Timeout => {
                ended(self, crate::perform::Ending::Timeout);
                give_back(self, "timeout", "it ran past its time budget".into());
                return Ok(());
            }
            Ending::Cancelled => {
                ended(self, crate::perform::Ending::Cancelled);
                give_back(self, "cancelled", "the run was cancelled".into());
                return Ok(());
            }
            Ending::Crashed { how, .. } => {
                ended(self, crate::perform::Ending::Failed);
                give_back(
                    self,
                    "crashed",
                    format!("the performer {how} without an answer"),
                );
                return Ok(());
            }
            Ending::Flooded | Ending::AdapterFailed(_) => {
                ended(self, crate::perform::Ending::Failed);
                give_back(
                    self,
                    "crashed",
                    "the performer gave no usable answer".into(),
                );
                return Ok(());
            }
        };
        let _lock = self.common.as_deref().map(ledger::land_lock).transpose();
        let integration = self.policy.integration.clone();
        let root = self.repo.root.clone();
        let mut head = String::new();
        let mut old_tip = String::new();
        if l.branch.is_some() {
            if let Err(e) = git::commit_all(
                &l.worktree,
                &format!("war go: {}", l.node),
                packet::SESSION_FILES,
            ) {
                ended(self, crate::perform::Ending::Failed);
                give_back(self, "land-failed", e);
                return Ok(());
            }
            old_tip = git::branch_tip(&root, &integration).unwrap_or_default();
            if !old_tip.is_empty()
                && !git::is_ancestor(&l.worktree, &old_tip, "HEAD")
                && let Err(files) =
                    git::merge(&l.worktree, &old_tip, &format!("war go: {}", l.node))
            {
                ended(self, crate::perform::Ending::Failed);
                give_back(
                    self,
                    "conflict",
                    format!("merging {integration} conflicted in {}", files.join(", ")),
                );
                return Ok(());
            }
            head = git::rev(&l.worktree, "HEAD").unwrap_or_default();
        }
        let answer = dir
            .join(crate::perform::DISPATCHES_DIR)
            .join(format!("answer-{}.json", l.dispatch_id));
        let _ = std::fs::write(&answer, text.as_bytes());
        let ingest = crate::run_cmd::submit(self.repo, &alias, &answer)?;
        let _ = std::fs::remove_file(&answer);
        let accepted = ingest.is_ready();
        for d in ingest.diagnostics {
            self.report.push(d);
        }
        if !accepted {
            ended(self, crate::perform::Ending::Refused);
            give_back(self, "refused", "the submission was refused".into());
            return Ok(());
        }
        if l.branch.is_some() {
            let moved = if old_tip.is_empty() {
                Ok(())
            } else {
                git::update_branch(&root, &integration, &head, &old_tip)
            };
            if let Err(e) = moved {
                self.report.push(Diagnostic::unknown(
                    "go.land-unknown",
                    l.node.clone(),
                    format!(
                        "{}: the submission is recorded, and {integration} could not be moved to \
                         {head}: {e}. The work is on {}",
                        l.node,
                        l.branch.clone().unwrap_or_default()
                    ),
                ));
            }
        }
        ended(self, crate::perform::Ending::Answered);
        if let Some(common) = &self.common {
            let _ = ledger::mark(
                common,
                &ledger::Landed {
                    node: l.node.clone(),
                    run: self.run_id.clone(),
                    actor: self.actor.clone(),
                    commit: head.clone(),
                    integration: integration.clone(),
                    level: "claimed".to_owned(),
                    at: now_rfc3339(),
                },
            );
        }
        let _ = crate::ticket::claim::release(&l.lock, &self.actor);
        if let Some(b) = &l.branch {
            git::remove_worktree(&root, &l.worktree, Some(b));
        }
        self.human
            .push_str(&format!("landed {}: submission recorded\n", l.node));
        self.graph.mark_done(&l.node);
        self.landed.push(Row {
            node: l.node.clone(),
            outcome: "landed".to_owned(),
            attempt: 1,
            detail: "submission recorded".to_owned(),
            seconds,
            level: Some("claimed".to_owned()),
            commit: (!head.is_empty()).then_some(head),
            tokens: tokens_value(None),
            executor: l.executor.clone(),
        });
        Ok(())
    }
}

/// What an external executor's poll printed.
enum AdapterSays {
    Pending,
    Submitted(String),
    Other(String),
}

/// The adapter contract (docs/GO.md): `{"status": "pending"}`, or
/// `{"status": "submitted", "submission": {...}}`. A status that claims the
/// work is finished (`done`, `closed`, `resolved`, ...) without a
/// submission is no answer: the work is decided by the refusals and the
/// checks, never by an adapter's word.
fn adapter_answer(stdout: &str) -> AdapterSays {
    let v: serde_json::Value = match serde_json::from_str(stdout.trim()) {
        Ok(v) => v,
        Err(e) => {
            return AdapterSays::Other(format!(
                "go.adapter-answer: the poll printed no JSON ({e}); the contract is \
                 {{\"status\": \"pending\"}} or {{\"status\": \"submitted\", \"submission\": {{...}}}}"
            ));
        }
    };
    match v.get("status").and_then(serde_json::Value::as_str) {
        Some("pending") => AdapterSays::Pending,
        Some("submitted") => match v.get("submission") {
            Some(s) if s.is_object() => AdapterSays::Submitted(s.to_string()),
            _ => AdapterSays::Other(
                "go.adapter-answer: status \"submitted\" with no submission object".to_owned(),
            ),
        },
        Some(other) => AdapterSays::Other(format!(
            "go.adapter-answer: status {other:?} is not an answer; an executor answers with a \
             submission, and the refusals and the checks decide whether the work is done"
        )),
        None => {
            AdapterSays::Other("go.adapter-answer: the poll's JSON has no \"status\"".to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(kind: &str, payload: serde_json::Value) -> openwarrant_core::journal::JournalEvent {
        crate::journal_cmd::event("u", kind, "a", "2026-10-07T00:00:00Z", &payload.to_string())
    }

    /// Attempts count unlanded endings in a row, reset by a landing or a
    /// person's retry; a block holds until a retry.
    #[test]
    fn attempts_and_blocks_are_read_from_the_journal() {
        let mut j = openwarrant_core::journal::Journal::default();
        let n = "t-a/i-1";
        for e in [
            ev(
                event::ENDED,
                serde_json::json!({"node": n, "outcome": "crashed", "d": 1}),
            ),
            ev(
                event::ENDED,
                serde_json::json!({"node": n, "outcome": "timeout", "d": 2}),
            ),
            ev(
                event::ENDED,
                serde_json::json!({"node": "t-a/i-2", "outcome": "crashed"}),
            ),
        ] {
            j.append(e).unwrap();
        }
        assert_eq!(attempts_in_row(&j, n), 2);
        j.append(ev(
            event::BLOCKED,
            serde_json::json!({"node": n, "reason": "two crashes", "attempts": 2}),
        ))
        .unwrap();
        assert_eq!(
            blocked_in(&j, n).map(|b| b.0),
            Some("two crashes".to_owned())
        );
        j.append(ev(event::RETRY, serde_json::json!({"node": n, "retry": 1})))
            .unwrap();
        assert!(blocked_in(&j, n).is_none());
        assert_eq!(attempts_in_row(&j, n), 0);
    }

    /// The adapter contract: pending, submitted, and every claim of being
    /// finished that is not a submission refused.
    #[test]
    fn an_adapter_saying_done_is_no_answer() {
        assert!(matches!(
            adapter_answer(r#"{"status":"pending"}"#),
            AdapterSays::Pending
        ));
        assert!(matches!(
            adapter_answer(r#"{"status":"submitted","submission":{"dispatch_id":"d"}}"#),
            AdapterSays::Submitted(_)
        ));
        for done in [
            r#"{"status":"done"}"#,
            r#"{"status":"closed"}"#,
            r#"{"status":"submitted"}"#,
            r#"{"state":"merged"}"#,
            "closed",
        ] {
            assert!(
                matches!(adapter_answer(done), AdapterSays::Other(_)),
                "{done}"
            );
        }
    }
}
