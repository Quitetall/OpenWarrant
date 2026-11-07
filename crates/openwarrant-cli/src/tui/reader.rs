// SPDX-License-Identifier: Apache-2.0
//! The background readers. The main thread draws and takes keys and never
//! opens a corpus: a thread reads the records into a finished [`Snapshot`],
//! another asks Herdr or git, and both hand the result over a channel, so a
//! reload or a slow socket never stalls a keypress. Nothing here is async
//! (OW-ADR-0014): plain threads and `std::sync::mpsc`.
//!
//! # One answer per question
//!
//! [`read`] calls the functions the CLI answers with and computes nothing of
//! its own: `console::board_with`, `corpus::Corpus`, `status`,
//! `roadmap_cmd::view_with`, `work::Work`, `graph::build`,
//! `estimate::schedule`, `ticket::scopes::conflict`, `ticket::live::board`,
//! `next::run_with`, `doctor::run`, `check::run_with`, `journal_cmd::load`.
//! What cannot be read is said, in `Snapshot::faults`, never shown as empty.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};

use super::herdr_adapter::{self, Focus};
use super::model::{
    ClaimRow, Collision, Dispatch, ExecutorRow, Herdr, Launcher, Live, NoticeRow, PhaseRow,
    Progress, QuestionItem, ReadyRow, Row, Snapshot, View, WorkRow,
};
use super::records;
use crate::repo::Repository;
use crate::ticket::{claim_on, scopes};
use crate::work::{Source, Work};

/// The name every thread here carries; the panic hook leaves the terminal
/// alone for a thread so named, since the app is still drawing.
pub const THREAD_PREFIX: &str = "war-tui-";

/// How often the tree's fingerprint is looked at.
const FINGERPRINT_EVERY: Duration = Duration::from_secs(1);
/// How often an agent at work is read: Herdr has no event for new output.
const LIVE_WORKING: Duration = Duration::from_millis(1000);
/// How often an idle fleet is re-read in case an event was missed.
const LIVE_IDLE: Duration = Duration::from_secs(30);
/// How often the working tree is looked at without Herdr.
const LIVE_TREE: Duration = Duration::from_secs(3);
/// How long before asking Herdr for a subscription again.
const EVENTS_RETRY_SECS: u32 = 5;
/// The newest events kept for the panel without Herdr.
const PROGRESS_LINES: usize = 8;

/// What the threads send the main thread.
#[derive(Debug)]
pub enum Update {
    Snapshot(Box<Snapshot>),
    /// A read that panicked or could not start; the last snapshot stands.
    ReadFailed(String),
    Live(Box<Live>),
    /// Everyone on the Repos list, before any one's facts are read.
    RepoList {
        generation: u64,
        rows: Vec<(crate::projects::Tier, crate::projects::Row)>,
        hidden: usize,
    },
    /// One repository's facts for the Repos list.
    Repo {
        generation: u64,
        index: usize,
        row: Box<crate::projects::Row>,
    },
    /// A finished child, for a popup.
    Child {
        title: String,
        text: String,
    },
}

/// What wakes the Live thread.
enum Wake {
    /// Look somewhere else.
    Focus(Focus),
    /// Herdr said the fleet changed.
    Event,
}

enum Ask {
    /// Read again now.
    Reload,
}

/// The app's end of the channels. Dropping it ends the threads.
pub struct Reader {
    tx: Sender<Update>,
    rx: Receiver<Update>,
    ask: Sender<Ask>,
    focus: Sender<Wake>,
    /// Set when the app leaves, or moves to another repository.
    stop: Arc<AtomicBool>,
    /// The Repos read in flight; an older one's rows are dropped.
    pub repo_generation: u64,
    last_focus: Focus,
}

impl Reader {
    /// Start reading `root`.
    #[must_use]
    pub fn spawn(root: Utf8PathBuf, binary: Row) -> Self {
        let (tx, rx) = channel();
        let (ask, ask_rx) = channel();
        let (focus, focus_rx) = channel();
        let records_tx = tx.clone();
        let records_root = root.clone();
        spawn_named("records", move || {
            records_thread(&records_root, &binary, &ask_rx, &records_tx);
        });
        let live_tx = tx.clone();
        let first = Focus {
            pane: None,
            workdir: root,
        };
        let last_focus = first.clone();
        spawn_named("live", move || live_thread(first, &focus_rx, &live_tx));
        let stop = Arc::new(AtomicBool::new(false));
        let events = focus.clone();
        let stopped = Arc::clone(&stop);
        spawn_named("herdr-events", move || events_thread(&events, &stopped));
        Self {
            tx,
            rx,
            ask,
            focus,
            stop,
            repo_generation: 0,
            last_focus,
        }
    }

    /// Everything that has arrived.
    pub fn drain(&self) -> Vec<Update> {
        self.rx.try_iter().collect()
    }

    pub fn reload(&self) {
        let _ = self.ask.send(Ask::Reload);
    }

    /// Tell the Live thread what to look at, when that changed.
    pub fn focus(&mut self, focus: Focus) {
        if focus != self.last_focus {
            self.last_focus = focus.clone();
            let _ = self.focus.send(Wake::Focus(focus));
        }
    }

    /// A child off the main thread; its output arrives as [`Update::Child`].
    pub fn capture(&self, root: Utf8PathBuf, title: String, argv: Vec<String>) {
        let tx = self.tx.clone();
        spawn_named("child", move || {
            let args: Vec<&str> = argv.iter().map(String::as_str).collect();
            let text = super::run_child_captured(&root, &args);
            let _ = tx.send(Update::Child { title, text });
        });
    }

    /// A child off the main thread, then `after`, then `prompt` to a pane if
    /// the child succeeded.
    pub fn say(
        &self,
        root: Utf8PathBuf,
        title: String,
        argv: Vec<String>,
        after: String,
        prompt: Option<(String, String)>,
    ) {
        let tx = self.tx.clone();
        spawn_named("child", move || {
            let args: Vec<&str> = argv.iter().map(String::as_str).collect();
            let mut text = super::run_child_captured(&root, &args);
            if text.trim_end().ends_with("[exit 0]") {
                if let Some((pane, line)) = prompt {
                    match crate::herdr::prompt_agent(&pane, &line) {
                        Ok(()) => text.push_str(&format!("\n\nDelivered to {pane} now.")),
                        Err(why) => {
                            text.push_str(&format!("\n\nNot delivered to {pane}: {why}\n\n{after}"))
                        }
                    }
                } else {
                    text.push_str(&format!("\n\n{after}"));
                }
            }
            let _ = tx.send(Update::Child { title, text });
        });
    }

    /// Claim, worktree and launch, off the main thread: `war start
    /// --worktree --print` does the first two and says what to run, and the
    /// launcher runs it. A refusal is the command's own.
    pub fn dispatch(&self, root: Utf8PathBuf, plan: Dispatch) {
        let tx = self.tx.clone();
        spawn_named("dispatch", move || {
            let title = format!("dispatch {}", plan.node);
            let text = launch(&root, &plan).unwrap_or_else(|why| why);
            let _ = tx.send(Update::Child { title, text });
        });
    }

    /// Read the Repos list, then each repository's facts, one after another,
    /// off the main thread: the list alone stats every entry.
    pub fn read_repos(&mut self) {
        self.repo_generation += 1;
        let generation = self.repo_generation;
        let tx = self.tx.clone();
        spawn_named("repos", move || {
            let (rows, hidden) = crate::projects::hub();
            let projects: Vec<_> = rows.iter().map(|(_, r)| r.clone()).collect();
            let listed = Update::RepoList {
                generation,
                rows,
                hidden,
            };
            if tx.send(listed).is_err() {
                return;
            }
            for (index, mut row) in projects.into_iter().enumerate() {
                if row.missing {
                    continue;
                }
                crate::projects::read_facts(&mut row);
                if row.fingerprint.is_none() {
                    // Unreadable: the reason is on the row, and it has been read.
                    row.fingerprint = Some(0);
                }
                let update = Update::Repo {
                    generation,
                    index,
                    row: Box::new(row),
                };
                if tx.send(update).is_err() {
                    return;
                }
            }
        });
    }
}

/// What a printed `war start` answered with.
#[derive(Debug, PartialEq, Eq)]
struct Started {
    worktree: String,
    command: Vec<String>,
    actor: String,
    claim: Option<String>,
}

fn started_in(envelope: &str) -> Result<Started, String> {
    let v: serde_json::Value =
        serde_json::from_str(envelope).map_err(|_| envelope.trim().to_owned())?;
    let r = &v["result"];
    let refused = || {
        v["diagnostics"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|d| d["severity"] == "error")
            .map(|d| format!("{}: {}", d["rule"].as_str().unwrap_or("?"), d["message"]))
            .collect::<Vec<_>>()
            .join("\n")
    };
    Ok(Started {
        worktree: r["worktree"].as_str().ok_or_else(refused)?.to_owned(),
        command: r["command"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| c.as_str().map(str::to_owned))
            .collect(),
        actor: r["actor"].as_str().unwrap_or_default().to_owned(),
        claim: r["claim_id"].as_str().map(str::to_owned),
    })
}

fn launch(root: &Utf8Path, plan: &Dispatch) -> Result<String, String> {
    let mut argv = vec!["--json".to_owned()];
    argv.extend(super::argv::start(
        &plan.node,
        &plan.actor,
        &plan.paths,
        true,
    ));
    let args: Vec<&str> = argv.iter().map(String::as_str).collect();
    let out = super::run_child_output(root, &args)?;
    let started = started_in(&out)?;
    let worktree = &started.worktree;
    let done = format!(
        "claimed {} as {}\nworktree {worktree}",
        plan.node, started.actor
    );
    match &plan.launcher {
        Launcher::Herdr { kind } => {
            let Some(args) = started.command.get(1..) else {
                return Err(format!("{done}\n\nno harness is configured in [go]"));
            };
            let mut env = vec![
                ("OPENWARRANT_ACTOR", started.actor.as_str()),
                ("WAR_GO_NODE", plan.node.as_str()),
            ];
            if let Some(id) = &started.claim {
                env.push(("OPENWARRANT_CLAIM_ID", id));
            }
            let pane = crate::herdr::create_tab(worktree, &plan.node, &env)
                .map_err(|why| format!("{done}\n\nHerdr could not open a tab: {why}"))?;
            let name = crate::go::git::slug(&plan.node);
            crate::herdr::start_agent(&name, kind, &pane, args)
                .map_err(|why| format!("{done}\n\nHerdr could not start it in {pane}: {why}"))?;
            Ok(format!("{done}\nstarted {kind} in Herdr pane {pane}"))
        }
        Launcher::Detached => {
            let log = Utf8Path::new(worktree).join(".openwarrant/start.log");
            let argv = super::argv::start(&plan.node, &plan.actor, &plan.paths, false);
            let args: Vec<&str> = argv.iter().map(String::as_str).collect();
            let pid = super::run_child_detached(root, &args, &log)?;
            Ok(format!(
                "{done}\nstarted detached, pid {pid}\nlog {log}\n\nIf the harness needs a \
                 terminal it stops at once; the log says so, and the claim stands."
            ))
        }
        // The event loop hands this one the terminal; it never comes here.
        Launcher::Terminal => Ok(done),
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn spawn_named(name: &str, f: impl FnOnce() + Send + 'static) {
    let _ = std::thread::Builder::new()
        .name(format!("{THREAD_PREFIX}{name}"))
        .spawn(f);
}

/// The panic's message, whatever it carried.
fn panic_message(p: &(dyn std::any::Any + Send)) -> String {
    p.downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| p.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a read panicked".to_owned())
}

fn send_read(root: &Utf8Path, binary: &Row, tx: &Sender<Update>) -> bool {
    let update = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        read_in_stages(root, binary, |early| {
            let _ = tx.send(Update::Snapshot(Box::new(early.clone())));
        })
    }))
    .map_or_else(
        |p| Update::ReadFailed(panic_message(p.as_ref())),
        |s| Update::Snapshot(Box::new(s)),
    );
    tx.send(update).is_ok()
}

/// Read once, then again whenever the tree's fingerprint moved and settled
/// (seen the same on two looks a second apart), or when asked.
fn records_thread(root: &Utf8Path, binary: &Row, ask: &Receiver<Ask>, tx: &Sender<Update>) {
    let fingerprint = || {
        Repository::discover(Some(root.to_owned()))
            .ok()
            .map_or(0, |r| {
                crate::watch::fingerprint(&crate::watch::watched_dirs(&r))
            })
    };
    let mut read_at = fingerprint();
    if !send_read(root, binary, tx) {
        return;
    }
    let mut pending = None;
    loop {
        match ask.recv_timeout(FINGERPRINT_EVERY) {
            Ok(Ask::Reload) => {
                read_at = fingerprint();
                pending = None;
                if !send_read(root, binary, tx) {
                    return;
                }
            }
            Err(RecvTimeoutError::Disconnected) => return,
            Err(RecvTimeoutError::Timeout) => {
                let now = fingerprint();
                if pending == Some(now) && now != read_at {
                    read_at = now;
                    pending = None;
                    if !send_read(root, binary, tx) {
                        return;
                    }
                } else {
                    pending = (now != read_at).then_some(now);
                }
            }
        }
    }
}

/// Hold a subscription whenever Herdr is there, and try again when it is not.
fn events_thread(wake: &Sender<Wake>, stop: &AtomicBool) {
    while !stop.load(Ordering::Relaxed) {
        let _ = crate::herdr::subscribe(
            || {
                let _ = wake.send(Wake::Event);
            },
            || !stop.load(Ordering::Relaxed),
        );
        for _ in 0..EVENTS_RETRY_SECS {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}

/// How long until the next look when nothing wakes the thread. Herdr pushes
/// fleet changes, and has no event for new output, so only an agent at work
/// is read on a clock. Without Herdr the working tree is looked at instead.
fn next_look(live: &Live, focus: &Focus) -> Duration {
    let working = live
        .agents
        .iter()
        .find(|a| focus.pane.as_ref() == Some(&a.pane_id))
        .or_else(|| live.agents.first())
        .is_some_and(|a| a.agent_status == "working");
    match live.herdr {
        Herdr::Connected { .. } if working => LIVE_WORKING,
        Herdr::Connected { .. } => LIVE_IDLE,
        _ => LIVE_TREE,
    }
}

fn live_thread(mut focus: Focus, rx: &Receiver<Wake>, tx: &Sender<Update>) {
    loop {
        let look =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| herdr_adapter::look(&focus)));
        let wait = look.as_ref().map_or(LIVE_TREE, |l| next_look(l, &focus));
        if let Ok(live) = look
            && tx.send(Update::Live(Box::new(live))).is_err()
        {
            return;
        }
        match rx.recv_timeout(wait) {
            Ok(Wake::Focus(next)) => focus = next,
            Ok(Wake::Event) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

// ---- one read -------------------------------------------------------------

/// Where a read's time goes, for the budget.
struct Lap {
    at: std::time::Instant,
    laps: Vec<(&'static str, u128)>,
}

impl Lap {
    fn new() -> Self {
        Self {
            at: std::time::Instant::now(),
            laps: Vec::new(),
        }
    }

    fn mark(&mut self, what: &'static str) {
        self.laps.push((what, self.at.elapsed().as_millis()));
        self.at = std::time::Instant::now();
    }
}

/// Everything the app shows, read once. `binary` is Help's first row.
#[must_use]
pub fn read(root: &Utf8Path, binary: &Row) -> Snapshot {
    read_in_stages(root, binary, |_| {})
}

/// [`read`], handing `early` everything but Help's what-next list, which
/// costs more than the rest together: Watch and Act should not wait for it.
fn read_in_stages(root: &Utf8Path, binary: &Row, mut early: impl FnMut(&Snapshot)) -> Snapshot {
    let mut lap = Lap::new();
    let now = crate::gate_cmd::receipt::rfc3339_from_secs(crate::ticket::now_secs());
    let setup = crate::init::guided::Machine::new(crate::init::guided::Facts::read(root), now);
    let setup_done = setup.step == crate::init::guided::Step::Done;
    let setup_step = setup.step.title().to_owned();
    let setup_row = Row {
        text: format!("→ {setup_step}"),
        command: "war init".to_owned(),
        detail: Some(setup.question()),
        ..Row::default()
    };
    let setup_pair = (!setup_done).then(|| (setup_step.clone(), setup.question()));
    let setup_ref = setup_pair.as_ref().map(|(a, b)| (a.as_str(), b.as_str()));
    let mut snap = Snapshot {
        loaded: true,
        program: root.file_name().unwrap_or("?").to_owned(),
        sas_in_force: "none".to_owned(),
        setup_done,
        setup_step,
        ..Snapshot::default()
    };
    snap.views.insert(View::Setup, vec![setup_row]);
    let with_binary = |rows: Vec<Row>| {
        let mut all = vec![binary.clone()];
        all.extend(rows);
        all
    };
    let Ok(repo) = Repository::discover(Some(root.to_owned())) else {
        snap.views.insert(
            View::Help,
            with_binary(records::help_rows(setup_ref, None, None, None)),
        );
        return snap;
    };
    snap.repository = true;
    snap.program.clone_from(&repo.config.project.name);
    snap.sas_in_force = repo
        .latest_sas_revision()
        .ok()
        .flatten()
        .map_or_else(|| "none".to_owned(), |r| r.version);
    let mut fault = |what: &str, why: String| {
        snap.faults.push(format!("{what}: {why}"));
    };

    // One compiled corpus for this read (OW-WAR-0148): every list reads it,
    // so the status, queue and frontier are built once.
    let compiled = crate::corpus::Corpus::new(&repo);
    let board = match crate::console::board_with(&compiled) {
        Ok(b) => Some(b),
        Err(e) => {
            fault("the signing queue", e.to_string());
            None
        }
    };
    let status = match compiled.status() {
        Ok(s) => Some(s),
        Err(e) => {
            fault("the corpus status", e.to_string());
            None
        }
    };
    lap.mark("corpus, board, status");
    let work = Work::read(&repo);
    for (source, what) in [
        (Source::Tickets, "the tickets"),
        (Source::Claims, "the claims"),
        (Source::Others, "the Warrants"),
    ] {
        if let Some(why) = work.unread(source) {
            fault(what, why.to_owned());
        }
    }

    // The queue, the questions, the frontier.
    let queue = board.as_ref().map(records::queue_rows).unwrap_or_default();
    let questions: Vec<QuestionItem> = board
        .iter()
        .flat_map(|b| &b.questions)
        .map(|q| QuestionItem {
            warrant: q.warrant.clone(),
            id: q.id.clone(),
            question: q.question.clone(),
            recommended: q.recommended.clone(),
            blocking: q.blocking,
            command: q.command.clone(),
        })
        .collect();
    let frontier = match compiled.frontier() {
        Ok((_, f)) => records::frontier_rows(f, board.as_ref()),
        Err(e) => vec![Row {
            text: format!("UNKNOWN: the frontier could not be read: {e}"),
            command: "war plan frontier".to_owned(),
            ..Row::default()
        }],
    };
    let [corpus, obligations, evidence] = status.map_or_else(
        || [Vec::new(), Vec::new(), Vec::new()],
        records::status_rows,
    );
    lap.mark("frontier and status rows");
    let journal = records::journal_rows(&repo);

    // The roadmap, and the phases as counts.
    let (roadmap, phases, phase_counts) = match status
        .ok_or_else(|| "the corpus status could not be read".to_owned())
        .and_then(|s| crate::roadmap_cmd::view_with(&repo, s).map_err(|e| e.to_string()))
    {
        Ok((_, v)) => {
            let phases = phase_rows(&v, status, &work);
            let achieved = v.phases.iter().filter(|p| p.achieved == "achieved").count();
            let counts = (!v.phases.is_empty()).then_some((achieved, v.phases.len()));
            (records::roadmap_rows(&v), phases, counts)
        }
        Err(e) => (
            vec![Row {
                text: format!("no roadmap record: {e}"),
                command: "war plan roadmap".to_owned(),
                ..Row::default()
            }],
            Vec::new(),
            None,
        ),
    };

    // Tickets (t-67ed): `ticket::board`, the same answer `war view tickets`,
    // `war view show` and `war view ready` give. No row signs or runs.
    let store = crate::ticket::Store::open(&repo, None);
    let tickets = store
        .as_ref()
        .map_err(ToString::to_string)
        .and_then(|s| crate::ticket::board(s).map_err(|e| e.to_string()))
        .map_or_else(
            |e| {
                vec![Row {
                    text: format!("UNKNOWN: the tickets could not be read: {e}"),
                    command: "war view tickets".to_owned(),
                    ..Row::default()
                }]
            },
            |b| records::ticket_rows(&b),
        );

    lap.mark("roadmap and tickets");
    // The graph: what can start, and the critical path.
    let scheduled = store.as_ref().ok().map(|s| {
        let prior = crate::go::policy::load(&repo.root)
            .ok()
            .map(|p| p.prior_secs);
        crate::estimate::run(&compiled, s, crate::estimate::prior_secs(&repo.root, prior)).and_then(
            |(_, schedule, _)| crate::graph::build(&compiled, s).map(|(_, g)| (g, schedule)),
        )
    });
    let (graph, schedule) = match scheduled {
        Some(Ok((g, s))) => (Some(g), Some(s)),
        Some(Err(e)) => {
            fault("the work graph", e.to_string());
            (None, None)
        }
        None => (None, None),
    };

    lap.mark("graph and schedule");
    let claims = match &store {
        Ok(s) => claim_rows(s, work.claims()),
        Err(_) => Vec::new(),
    };
    let notices = store.as_ref().map(notice_rows).unwrap_or_default();
    let worktrees = crate::worktrees::inventory(&repo, false, "git")
        .map(|w| w.into_iter().map(|r| r.path).collect::<Vec<_>>())
        .map_err(|e| fault("the worktrees", e.to_string()))
        .ok();

    let (usage, usage_faults) = super::usage::read(&repo, work.tickets());
    for why in usage_faults {
        fault("usage", why);
    }

    lap.mark("claims, notices, worktrees, usage");
    let critical: BTreeSet<&str> = schedule
        .iter()
        .flat_map(|s| &s.critical_path)
        .map(String::as_str)
        .collect();
    let work_rows = work_rows(&work, graph.as_ref(), &critical, &usage);
    let ready = schedule
        .iter()
        .flat_map(|s| &s.order)
        .filter_map(|id| graph.as_ref()?.node(id))
        .map(|n| ReadyRow {
            id: n.id.clone(),
            title: if n.text.is_empty() {
                n.title.clone()
            } else {
                n.text.clone()
            },
            critical: critical.contains(n.id.as_str()),
        })
        .collect();
    let (resolved, total) = status.map_or((0, 0), |s| {
        (
            s.warrants
                .iter()
                .filter(|w| w.rung == openwarrant_core::status::WarrantRung::Resolved)
                .count(),
            s.warrants.len(),
        )
    });
    let items = work.complete().then(|| {
        let n = |word: &str| work.items().iter().filter(|i| i.state == word).count();
        let done = work.items().iter().filter(|i| i.is_done()).count();
        let active = n("in_progress");
        (work.items().len() - done - active, active, done)
    });

    snap.progress = Progress {
        warrants: status.map(|_| (resolved, total)),
        phases: phase_counts,
        items,
    };
    snap.critical_length = schedule
        .as_ref()
        .filter(|s| !s.critical_path.is_empty())
        .map(|s| crate::estimate::human(s.critical_secs));
    snap.critical_path = schedule.map(|s| s.critical_path).unwrap_or_default();
    snap.collisions = collisions(&claims, work.claims());
    snap.progress_lines = journal
        .iter()
        .take(PROGRESS_LINES)
        .map(|r| r.text.clone())
        .collect();
    snap.queue = queue;
    snap.questions = questions;
    snap.phases = phases;
    snap.work = work_rows;
    snap.ready = ready;
    snap.claims = claims;
    snap.notices = notices;
    snap.worktrees = worktrees;
    snap.usage = usage;
    snap.drafter = repo.config.plan.drafter_argv.first().cloned();
    snap.human = only_human(&repo.root);
    snap.executors = match crate::go::policy::load(&repo.root) {
        Ok(policy) => executor_rows(&policy),
        Err(d) => {
            snap.faults.push(format!("[go]: {}", d.message));
            Vec::new()
        }
    };
    for (view, rows) in [
        (View::Tickets, tickets),
        (View::Frontier, frontier),
        (View::Corpus, corpus),
        (View::Obligations, obligations),
        (View::Evidence, evidence),
        (View::Journal, journal),
        (View::Roadmap, roadmap),
    ] {
        snap.views.insert(view, rows);
    }
    snap.views.insert(
        View::Help,
        vec![
            binary.clone(),
            Row {
                text: "reading what next…".to_owned(),
                command: "war view next".to_owned(),
                ..Row::default()
            },
        ],
    );
    lap.mark("everything but Help");
    early(&snap);

    // Help: what next.
    let next = crate::next::run_with(&compiled).ok();
    let (doctor, _) = crate::doctor::run(Some(root.to_owned()), None, false);
    let check = crate::check::run_with(&compiled, None, false).ok();
    let help = with_binary(records::help_rows(
        setup_ref,
        next.as_ref(),
        Some(&doctor),
        check.as_ref(),
    ));

    snap.views.insert(View::Help, help);
    lap.mark("next, doctor, check");
    snap.laps = lap.laps;
    snap
}

/// `[go]`'s executors. `war start` runs the local harness; an external
/// executor is run by `war evidence go`, for what its route sends it.
fn executor_rows(policy: &crate::go::policy::Policy) -> Vec<ExecutorRow> {
    use crate::go::policy::Harness;
    let program = policy.start_argv.first();
    let word = match policy.harness {
        Harness::Claude => "claude".to_owned(),
        // The program's own name: it becomes the claim's actor, and a path
        // is no actor.
        Harness::Generic => program.map_or_else(
            || "harness".to_owned(),
            |p| Utf8Path::new(p).file_name().unwrap_or(p).to_owned(),
        ),
    };
    let mut rows = vec![ExecutorRow {
        unavailable: program
            .is_none()
            .then(|| "no harness for a session: set [go] harness or start_argv".to_owned()),
        herdr_kind: program
            .and_then(|p| Utf8Path::new(p).file_name())
            .filter(|n| HERDR_KINDS.contains(n))
            .map(str::to_owned),
        needs_terminal: policy.harness == Harness::Claude,
        name: word,
    }];
    rows.extend(policy.executors.keys().map(|name| ExecutorRow {
        name: name.clone(),
        unavailable: Some("runs under `war evidence go`, by its [go.route]".to_owned()),
        ..ExecutorRow::default()
    }));
    rows
}

/// The agents `herdr agent start --kind` takes.
const HERDR_KINDS: &[&str] = &[
    "pi",
    "claude",
    "codex",
    "gemini",
    "cursor",
    "devin",
    "agy",
    "cline",
    "omp",
    "mastracode",
    "opencode",
    "copilot",
    "kimi",
    "kiro",
    "droid",
    "amp",
    "grok",
    "hermes",
    "kilo",
    "qodercli",
    "qwen",
    "letta",
    "maki",
    "muse",
];

/// The human the register names, when there is exactly one, so an answer's
/// `--as` starts filled in. The register, not this, decides who may answer.
fn only_human(root: &Utf8Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("docs/authority/roles.toml")).ok()?;
    let doc: toml::Table = toml::from_str(&text).ok()?;
    let humans: BTreeSet<&str> = doc
        .get("assignment")?
        .as_array()?
        .iter()
        .filter(|a| a.get("actor_kind").and_then(toml::Value::as_str) == Some("human"))
        .filter_map(|a| a.get("actor").and_then(toml::Value::as_str))
        .collect();
    let mut humans = humans.into_iter();
    let first = humans.next()?;
    humans.next().is_none().then(|| first.to_owned())
}

/// Each phase's members as counts. A member is done when its record says so:
/// a resolved Warrant, or a finished item. One that no record answers for is
/// counted as unknown, never as done or as open.
fn phase_rows(
    view: &crate::roadmap_cmd::View,
    status: Option<&openwarrant_core::status::CorpusStatus>,
    work: &Work,
) -> Vec<PhaseRow> {
    use openwarrant_core::status::WarrantRung;
    view.phases
        .iter()
        .map(|p| {
            let mut row = PhaseRow {
                id: p.id.clone(),
                title: p.title.clone(),
                achieved: p.achieved.clone(),
                total: p.members.len(),
                ..PhaseRow::default()
            };
            (row.done, row.unknown) = member_counts(&p.members, |m| {
                let rung = status
                    .and_then(|s| s.warrants.iter().find(|w| w.alias == m))
                    .map(|w| w.rung == WarrantRung::Resolved);
                rung.or_else(|| work.items().iter().find(|i| i.id == m).map(|i| i.is_done()))
            });
            row
        })
        .collect()
}

/// How many of `members` are done, and how many no record answers for.
fn member_counts(members: &[String], done: impl Fn(&str) -> Option<bool>) -> (usize, usize) {
    let mut counts = (0, 0);
    for m in members {
        match done(m) {
            Some(true) => counts.0 += 1,
            Some(false) => {}
            None => counts.1 += 1,
        }
    }
    counts
}

/// Every open item, the ones in progress first, then the critical path, each
/// followed by its remaining checklist items.
fn work_rows(
    work: &Work,
    graph: Option<&crate::graph::Graph>,
    critical: &BTreeSet<&str>,
    usage: &super::usage::Usage,
) -> Vec<WorkRow> {
    let tickets = work.tickets();
    let mut open: Vec<&crate::work::Item> = work.items().iter().filter(|i| !i.is_done()).collect();
    open.sort_by_key(|i| {
        (
            i.state != "in_progress",
            !critical.contains(i.id.as_str()),
            i.id.clone(),
        )
    });
    let mut rows = Vec::new();
    for item in open {
        let ticket = tickets.iter().find(|t| t.id() == item.id);
        rows.push(WorkRow {
            id: item.id.clone(),
            title: item.title.clone(),
            state: item.state.clone(),
            level: item.level.as_str().to_owned(),
            depth: 0,
            critical: critical.contains(item.id.as_str()),
            holder: claim_on(work.claims(), &item.id, None)
                .and_then(|(_, c)| c.map(|c| c.actor.clone())),
            tokens: usage.of(&item.id),
        });
        for line in ticket.iter().flat_map(|t| &t.checklist.items) {
            let (false, Some(id)) = (line.done, line.id.as_deref()) else {
                continue;
            };
            let target = format!("{}/{id}", item.id);
            let tokens = usage.of(&target);
            rows.push(WorkRow {
                state: graph
                    .and_then(|g| g.node(&target))
                    .map_or("UNKNOWN", |n| n.state.as_str())
                    .to_owned(),
                critical: critical.contains(target.as_str()),
                id: target,
                title: line.text.clone(),
                level: String::new(),
                depth: 1,
                holder: claim_on(work.claims(), &item.id, Some(id))
                    .and_then(|(_, c)| c.map(|c| c.actor.clone())),
                tokens,
            });
        }
    }
    rows
}

/// Every lock, one row each, a lock that could not be read included.
fn claim_rows(store: &crate::ticket::Store, claims: &crate::ticket::Claims) -> Vec<ClaimRow> {
    claims
        .iter()
        .map(|(name, claim)| {
            let target = name.strip_suffix(".lock").unwrap_or(name).to_owned();
            let Some(c) = claim else {
                return ClaimRow {
                    target,
                    ..ClaimRow::default()
                };
            };
            let target = c
                .item
                .as_ref()
                .map_or_else(|| c.ticket.clone(), |i| format!("{}/{i}", c.ticket));
            let coordination = c.coordination.as_deref();
            ClaimRow {
                target,
                actor: Some(c.actor.clone()),
                since_unix: c.since_unix,
                lease_until_unix: c.lease_until_unix,
                renewed_unix: std::fs::metadata(store.claims_dir.join(name))
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs()),
                workspace: coordination.map(|co| co.workspace.clone()),
                scope: coordination
                    .filter(|co| !scopes::undeclared(co))
                    .map(|co| co.paths.clone())
                    .unwrap_or_default(),
            }
        })
        .collect()
}

/// Each pair of claims whose declared paths `ticket::scopes` will not call
/// disjoint.
fn collisions(rows: &[ClaimRow], claims: &crate::ticket::Claims) -> Vec<Collision> {
    let held: Vec<(&str, &scopes::Coordination)> = claims
        .values()
        .flatten()
        .filter_map(|c| {
            let name = rows
                .iter()
                .find(|r| {
                    r.actor.as_deref() == Some(c.actor.as_str()) && r.since_unix == c.since_unix
                })
                .map_or(c.ticket.as_str(), |r| r.target.as_str());
            c.coordination.as_deref().map(|co| (name, co))
        })
        .collect();
    let mut out = Vec::new();
    for (i, (a, ca)) in held.iter().enumerate() {
        for (b, cb) in &held[i + 1..] {
            if let Some(why) = scopes::conflict(ca, cb) {
                out.push(Collision {
                    a: (*a).to_owned(),
                    b: (*b).to_owned(),
                    why,
                });
            }
        }
    }
    out
}

fn notice_rows(store: &crate::ticket::Store) -> Vec<NoticeRow> {
    crate::ticket::live::board(store)
        .into_iter()
        .map(|(n, told, seen)| NoticeRow {
            ticket: n.ticket,
            actor: n.actor,
            line: n.line,
            at: n.at,
            told,
            seen,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The old app counted a member done when it was not in a phase's `open`
    /// list, which holds unplaced work and never a member: every phase read
    /// as finished. A member is done only when a record says so.
    #[test]
    fn a_phase_member_is_done_only_when_a_record_says_so() {
        let members = ["a", "b", "c"].map(String::from);
        let answers = |m: &str| match m {
            "a" => Some(true),
            "b" => Some(false),
            _ => None,
        };
        assert_eq!(member_counts(&members, answers), (1, 1));
    }

    #[test]
    fn a_printed_start_says_where_and_what_to_run() {
        let ok = r#"{"result":{"worktree":"/w/t","command":["claude","brief"],"actor":"a","claim_id":"c"}}"#;
        let s = started_in(ok).expect("started");
        assert_eq!(s.worktree, "/w/t");
        assert_eq!(s.command, ["claude", "brief"]);
        assert_eq!(s.claim.as_deref(), Some("c"));
    }

    /// Refusal: a refused start is the command's own words, and nothing
    /// launches.
    #[test]
    fn a_refused_start_launches_nothing() {
        let refused = r#"{"result":null,"diagnostics":[{"severity":"error","rule":"work.scope-conflict","message":"src overlaps"}]}"#;
        let why = started_in(refused).expect_err("refused");
        assert!(why.contains("work.scope-conflict"), "{why}");
        assert!(started_in("not json").is_err());
    }

    fn claim(paths: &[&str], actor: &str, since: u64) -> crate::ticket::claim::Claim {
        crate::ticket::claim::Claim {
            schema: crate::ticket::claim::CLAIM_SCHEMA.to_owned(),
            claim: format!("c-{actor}"),
            ticket: format!("t-{actor}"),
            item: None,
            actor: actor.to_owned(),
            since: String::new(),
            since_unix: since,
            lease_until: None,
            lease_until_unix: None,
            coordination: Some(Box::new(scopes::Coordination {
                workspace: "/w".to_owned(),
                session: "s".to_owned(),
                base_revision: None,
                paths: paths.iter().map(|p| (*p).to_owned()).collect(),
                display_name: actor.to_owned(),
            })),
        }
    }

    fn collide(a: &[&str], b: &[&str]) -> Vec<Collision> {
        let mut claims = crate::ticket::Claims::new();
        claims.insert("a.lock".to_owned(), Some(claim(a, "a", 1)));
        claims.insert("b.lock".to_owned(), Some(claim(b, "b", 2)));
        let rows: Vec<ClaimRow> = claims
            .values()
            .flatten()
            .map(|c| ClaimRow {
                target: c.ticket.clone(),
                actor: Some(c.actor.clone()),
                since_unix: c.since_unix,
                ..ClaimRow::default()
            })
            .collect();
        collisions(&rows, &claims)
    }

    #[test]
    fn overlapping_declared_paths_collide() {
        let found = collide(&["src/a/**"], &["src/a/b.rs"]);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].a.as_str(), found[0].b.as_str()), ("t-a", "t-b"));
    }

    /// Refusal: disjoint paths do not collide, and a claim that declared
    /// nothing collides with nothing.
    #[test]
    fn disjoint_or_undeclared_paths_do_not_collide() {
        assert!(collide(&["src/a/**"], &["src/b/**"]).is_empty());
        assert!(collide(&["**"], &["src/b/**"]).is_empty());
    }
}
