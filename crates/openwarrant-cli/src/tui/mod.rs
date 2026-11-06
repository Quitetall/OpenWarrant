// SPDX-License-Identifier: Apache-2.0
//! `war` with no arguments: the app (OW-WAR-0112, absorbing OW-WAR-0073).
//!
//! A rendering under SAS §76.6 and OW-ADR-0019/0020: it issues commands and
//! holds no key. Every act it starts shells out to the same `war sign
//! --ssh-sign` a hand would run, in a child that inherits the terminal, so
//! the `ssh-add -c` dialog remains the human act. Nothing under `tui/` names
//! `ssh-keygen` or `SSH_AUTH_SOCK`; the battery greps for both.
//!
//! # One answer per question
//!
//! Every pane reads the functions the CLI already answers with —
//! `console::board`, `next::run`, `status::build`, `frontier::run`,
//! `check::run`, `doctor::run`, `journal_cmd::load` — and renders. A pane
//! that computed a fact itself would be a second answer to a question the
//! records already answer, and the one nobody re-checks.
//!
//! # One file
//!
//! OW-WAR-0112's signed declaration names `tui/mod.rs` and nothing else under
//! `tui/`, so the app lives here in inner modules rather than in files the
//! authorization never saw. A later Warrant that declares `tui/*.rs` may
//! split it; this one may not widen its own set (OW-ADR-0021).
//!
//! # The terminal comes back
//!
//! Raw mode and the alternate screen are entered through one guard whose
//! `Drop` leaves them, and a panic hook leaves them before the default hook
//! prints — a library that takes the terminal owes it back on every exit
//! path. `--panic-after-setup` is the fixture the battery uses to prove it.
//!
//! # No executor
//!
//! Input is `crossterm::event::poll` on a 250 ms tick; nothing here is
//! async, so OW-ADR-0014's refusal stands untouched. Live refresh is
//! `watch.rs`'s fingerprint poller, consulted every fourth idle tick and
//! debounced one more, so a signature given in another terminal shows up
//! without a keypress.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Tabs, Wrap,
};

use crate::diagnostic::{Diagnostic, Report, Severity};
use crate::repo::{RepoError, Repository};

/// The refusal when there is no terminal: exit 2, by name, and not one
/// escape code on a pipe.
pub const NO_TTY: &str = "tui.no-tty";

/// The refusal for `war --json` and `war view tui --json`: a terminal
/// application is a rendering and has no envelope, so it names the commands
/// that do. Exit 2, as an envelope, like every other refusal.
pub const JSON: &str = "tui.json";

/// Print the `tui.json` refusal as an envelope; returns the exit code.
pub fn refuse_json() -> u8 {
    let mut report = Report::default();
    report.push(Diagnostic::error(
        JSON,
        "-".to_owned(),
        "the app is a rendering and has no envelope; for a script use `war status --json`, \
         `war view console --json`, `war view next --json` or `war check --json`",
    ));
    println!("{}", crate::output::envelope("tui", &report, None));
    crate::output::exit_code(&report)
}

/// `war` / `war view tui`. Returns the exit code.
pub fn run(root: Option<Utf8PathBuf>, panic_after_setup: bool) -> Result<u8, RepoError> {
    if !crate::sign::at_a_terminal() {
        let mut report = Report::default();
        report.push(Diagnostic::error(
            NO_TTY,
            "-".to_owned(),
            "the app needs a terminal on stdin and stdout; for a script use `war status \
             --json`, `war view console --json`, `war view next --json` or `war check --json`",
        ));
        crate::check::print(&report);
        return Ok(crate::EXIT_NOT_READY);
    }
    let repo = Repository::discover(root.clone()).ok();
    // OW-WAR-0115: `war` where no repository is found opens the hub. Setup
    // for this directory stays one keypress away (`1`).
    let hub = repo.is_none() && root.is_none();
    let root = match (&repo, root) {
        (Some(r), _) => r.root.clone(),
        (None, Some(r)) => r,
        (None, None) => Utf8PathBuf::from_path_buf(
            std::env::current_dir().map_err(|e| RepoError::Message(e.to_string()))?,
        )
        .map_err(|_| RepoError::Message("the current directory is not UTF-8".to_owned()))?,
    };
    let mut model = Model::load(root, repo);
    if hub {
        model.pane = Pane::Projects;
        model.load_projects();
    }

    let guard = term::Guard::enter()?;
    // The hook runs before the default one prints the panic, so the message
    // lands on a restored screen rather than inside the alternate one.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        term::leave_quietly();
        default_hook(info);
    }));
    if panic_after_setup {
        panic!("--panic-after-setup: the fixture that proves the terminal is restored");
    }
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend).map_err(io("could not open the terminal"))?;
    let result = event_loop(&mut terminal, &mut model);
    drop(guard);
    let _ = std::panic::take_hook();
    result.map(|()| crate::EXIT_OK)
}

fn io(context: &'static str) -> impl Fn(std::io::Error) -> RepoError {
    move |source| RepoError::Io {
        context: context.to_owned(),
        source,
    }
}

// ---------------------------------------------------------------------------
// The terminal guard
// ---------------------------------------------------------------------------

mod term {
    use super::io;
    use crate::repo::RepoError;
    use crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    };

    /// Raw mode and the alternate screen, both left on drop.
    pub struct Guard(());

    impl Guard {
        pub fn enter() -> Result<Self, RepoError> {
            enable_raw_mode().map_err(io("could not enter raw mode"))?;
            crossterm::execute!(std::io::stdout(), EnterAlternateScreen)
                .map_err(io("could not enter the alternate screen"))?;
            Ok(Self(()))
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            leave_quietly();
        }
    }

    /// Best effort, never failing: called from the panic hook and from drop.
    pub fn leave_quietly() {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), LeaveAlternateScreen);
    }
}

// ---------------------------------------------------------------------------
// The model: read through the CLI's own functions, rendered and nothing else
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pane {
    Observe,
    Orchestrate,
    Setup,
    Help,
    Queue,
    Questions,
    Frontier,
    Corpus,
    Obligations,
    Evidence,
    Journal,
    Roadmap,
    Projects,
    Tickets,
}

impl Pane {
    const ALL: [Pane; 14] = [
        Pane::Observe,
        Pane::Orchestrate,
        Pane::Setup,
        Pane::Help,
        Pane::Queue,
        Pane::Questions,
        Pane::Frontier,
        Pane::Corpus,
        Pane::Obligations,
        Pane::Evidence,
        Pane::Journal,
        Pane::Roadmap,
        Pane::Projects,
        Pane::Tickets,
    ];

    const fn title(self) -> &'static str {
        match self {
            Self::Observe => "o Observe",
            Self::Orchestrate => "O Orchestrate",
            Self::Setup => "1 Setup",
            Self::Help => "2 Help",
            Self::Queue => "3 Queue",
            Self::Questions => "4 Questions",
            Self::Frontier => "5 Frontier",
            Self::Corpus => "6 Corpus",
            Self::Obligations => "7 Obligations",
            Self::Evidence => "8 Evidence",
            Self::Journal => "9 Journal",
            Self::Roadmap => "0 Roadmap",
            Self::Projects => "p Projects",
            Self::Tickets => "t Tickets",
        }
    }
}

/// Models available for dispatch from Orchestrate.
const DISPATCH_MODELS: &[&str] = &[
    "claude-opus-5.5",
    "claude-sonnet-4.6",
    "gemini-3.8-flash",
    "gemini-3.5-pro",
    "local-runner",
];

#[derive(Debug, Clone)]
struct MilestoneItem {
    name: String,
    done_count: usize,
    total_count: usize,
    tasks: Vec<MilestoneTask>,
}

#[derive(Debug, Clone)]
struct MilestoneTask {
    id: String,
    title: String,
    state: String,
    tokens: u64,
}

#[derive(Debug, Clone)]
struct DagNodeItem {
    id: String,
    title: String,
    state: String,
    prefix: String,
    is_critical: bool,
    claim_actor: Option<String>,
    claim_age: Option<String>,
    claim_lease: Option<String>,
    claim_scope: Option<String>,
    scope_collision: bool,
}

#[derive(Debug, Clone)]
struct QuestionItem {
    target: String,
    question: String,
    recommended: String,
    blocking: bool,
}

/// One row of any list pane: what is shown, the command behind it, and what
/// `Enter` or `v` does with it.
#[derive(Debug, Clone)]
struct Row {
    text: String,
    /// The exact command behind the row, for the bottom line.
    command: String,
    /// `war sign <target>`: the act `s`/`Enter` runs in a child.
    sign_target: Option<String>,
    /// A remedy `x` may run unasked.
    auto: Option<Vec<String>>,
    /// Detail `Enter` opens, when the row has more to say.
    detail: Option<String>,
}

/// One document the Help pane can show.
struct Doc {
    name: String,
    text: String,
}

struct Model {
    root: Utf8PathBuf,
    repo: Option<Repository>,
    program: String,
    sas_in_force: String,
    pane: Pane,
    /// Per pane: rows and the highlighted index.
    rows: BTreeMap<u8, Vec<Row>>,
    selected: BTreeMap<u8, usize>,
    /// Queue rows checked for a batch.
    checked: std::collections::BTreeSet<usize>,
    filter: String,
    filtering: bool,
    show_keys: bool,
    /// A popup with scrollable text (a request, a detail, a command's output).
    popup: Option<(String, String, u16)>,
    docs: Vec<Doc>,
    doc_index: usize,
    /// The Help pane's mode: what-next (false) or a document (true).
    reading: bool,
    status: String,
    setup: crate::init::guided::Machine,
    pending_count: usize,
    fingerprint: u64,
    fingerprint_pending: Option<u64>,
    idle_ticks: u32,
    /// The Projects pane's projects, index-aligned with its first rows;
    /// facts are read one project per idle tick (`tick_projects`).
    projects: Vec<crate::projects::Row>,
    /// Each project's tier, index-aligned with `projects`.
    project_tiers: Vec<crate::projects::Tier>,
    /// Entries of the list the pane leaves out.
    projects_hidden: usize,
    /// Which project the next idle tick looks at.
    project_cursor: usize,
    /// `n` on Projects: the directory being typed for a new project.
    new_dir: Option<String>,
    /// The Help pane's first row: this binary, and PATH's `war` if it differs.
    binary: Row,
    // Observe state:
    observe_col: u8,
    observe_milestone_sel: usize,
    observe_dag_sel: usize,
    observe_question_sel: usize,
    observe_agent_sel: usize,
    herdr_agents: Vec<crate::herdr::HerdrAgent>,
    herdr_transcript: String,
    herdr_connected: bool,
    milestones: Vec<MilestoneItem>,
    dag_nodes: Vec<DagNodeItem>,
    questions_hub: Vec<QuestionItem>,
    total_tokens: u64,
    total_cost_usd: f64,
    active_agents_count: usize,
    active_worktrees_count: usize,
    global_progress_pct: u16,
    // Orchestrate state:
    orchestrate_col: u8,
    orchestrate_draft_title: String,
    orchestrate_draft_items: String,
    orchestrate_model_idx: usize,
    orchestrate_scope_input: String,
    orchestrate_dispatch_task_idx: usize,
}

impl Model {
    fn load(root: Utf8PathBuf, repo: Option<Repository>) -> Self {
        let now = crate::gate_cmd::receipt::rfc3339_from_secs(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
        );
        let setup = crate::init::guided::Machine::new(crate::init::guided::Facts::read(&root), now);
        let mut m = Self {
            program: repo.as_ref().map_or_else(
                || root.file_name().unwrap_or("?").to_owned(),
                |r| r.config.project.name.clone(),
            ),
            sas_in_force: "none".to_owned(),
            pane: if setup.step == crate::init::guided::Step::Done {
                Pane::Observe
            } else {
                Pane::Setup
            },
            rows: BTreeMap::new(),
            selected: BTreeMap::new(),
            checked: std::collections::BTreeSet::new(),
            filter: String::new(),
            filtering: false,
            show_keys: false,
            popup: None,
            docs: Vec::new(),
            doc_index: 0,
            reading: false,
            status: String::new(),
            setup,
            pending_count: 0,
            fingerprint: 0,
            fingerprint_pending: None,
            idle_ticks: 0,
            projects: Vec::new(),
            project_tiers: Vec::new(),
            projects_hidden: 0,
            project_cursor: 0,
            new_dir: None,
            binary: binary_row(),
            observe_col: 1,
            observe_milestone_sel: 0,
            observe_dag_sel: 0,
            observe_question_sel: 0,
            observe_agent_sel: 0,
            herdr_agents: Vec::new(),
            herdr_transcript: String::new(),
            herdr_connected: false,
            milestones: Vec::new(),
            dag_nodes: Vec::new(),
            questions_hub: Vec::new(),
            total_tokens: 0,
            total_cost_usd: 0.0,
            active_agents_count: 0,
            active_worktrees_count: 1,
            global_progress_pct: 0,
            orchestrate_col: 1,
            orchestrate_draft_title: "Add unit tests for telemetry collector".to_owned(),
            orchestrate_draft_items:
                "- [ ] Verify collector handles empty events\n- [ ] Assert JSON schema conformance"
                    .to_owned(),
            orchestrate_model_idx: 0,
            orchestrate_scope_input: "crates/openwarrant-cli/src/**".to_owned(),
            orchestrate_dispatch_task_idx: 0,
            root,
            repo,
        };
        m.docs = m.load_docs();
        m.refresh();
        if m.rows
            .get(&(Pane::Tickets as u8))
            .is_some_and(|r| !r.is_empty())
            && m.pane != Pane::Setup
        {
            m.pane = Pane::Observe;
        }
        m
    }

    fn poll_herdr(&mut self) {
        if crate::herdr::is_available() {
            self.herdr_connected = true;
            if let Ok(agents) = crate::herdr::list_agents() {
                self.herdr_agents = agents;
            }
            if !self.herdr_agents.is_empty() {
                let idx = self.observe_agent_sel.min(self.herdr_agents.len() - 1);
                if let Ok(text) = crate::herdr::read_agent(&self.herdr_agents[idx].pane_id, 40) {
                    self.herdr_transcript = text;
                }
            } else {
                self.herdr_transcript =
                    "Connected to Herdr daemon (no active agents running).".to_owned();
            }
        } else {
            self.herdr_connected = false;
            let git_status = run_child_captured(&self.root, &["status", "--short"]);
            self.herdr_transcript = format!(
                "Herdr daemon idle (~/.config/herdr/herdr.sock).\n\nWorking tree status:\n{git_status}"
            );
        }
    }

    fn prev_herdr_agent(&mut self) {
        if !self.herdr_agents.is_empty() {
            self.observe_agent_sel =
                (self.observe_agent_sel + self.herdr_agents.len() - 1) % self.herdr_agents.len();
            self.poll_herdr();
        }
    }

    fn next_herdr_agent(&mut self) {
        if !self.herdr_agents.is_empty() {
            self.observe_agent_sel = (self.observe_agent_sel + 1) % self.herdr_agents.len();
            self.poll_herdr();
        }
    }

    /// Everything the panes show, re-read. Never computes a fact of its own.
    fn refresh(&mut self) {
        self.setup
            .observe(crate::init::guided::Facts::read(&self.root));
        let Some(repo) = self.repo.as_ref() else {
            self.status = format!(
                "{} is not a repository — p lists your projects, 1 sets one up here",
                self.root
            );
            for p in Pane::ALL {
                self.rows.insert(p as u8, vec![]);
            }
            self.rows.insert(
                Pane::Setup as u8,
                vec![Row {
                    text: format!("→ {}", self.setup.step.title()),
                    command: "war init".to_owned(),
                    sign_target: None,
                    auto: None,
                    detail: Some(self.setup.question()),
                }],
            );
            self.rows
                .insert(Pane::Help as u8, self.help_rows(None, None, None));
            return;
        };
        let repo = repo.clone();
        self.sas_in_force = repo
            .latest_sas_revision()
            .ok()
            .flatten()
            .map_or_else(|| "none".to_owned(), |r| r.version);
        self.fingerprint = crate::watch::fingerprint(&watched(&repo));
        // One compiled corpus for this fingerprint (OW-WAR-0148): every pane
        // below reads it, so the status, queue and frontier are built once
        // per reload rather than once per pane.
        let compiled = crate::corpus::Corpus::new(&repo);

        // The queue: `war sign --list`, through the console's board.
        let board = crate::console::board_with(&compiled).ok();
        let mut queue = Vec::new();
        let mut questions = Vec::new();
        let mut stage_cmd: BTreeMap<(String, String), String> = BTreeMap::new();
        if let Some(b) = &board {
            for a in &b.acts {
                queue.push(Row {
                    text: format!("[{}] {}", a.n, a.line),
                    command: a.command.clone(),
                    sign_target: Some(a.target.clone()),
                    auto: None,
                    detail: None,
                });
            }
            for q in &b.questions {
                questions.push(Row {
                    text: format!(
                        "{} {}/{}: {}{}",
                        if q.blocking { "BLOCKING" } else { "        " },
                        q.warrant,
                        q.id,
                        q.question,
                        if q.recommended.is_empty() {
                            String::new()
                        } else {
                            format!("  → {}", q.recommended)
                        }
                    ),
                    command: q.command.clone(),
                    sign_target: None,
                    auto: None,
                    detail: Some(format!(
                        "{}\n\nrecommended: {}\n\n{}",
                        q.question, q.recommended, q.command
                    )),
                });
            }
            for s in &b.stages {
                stage_cmd.insert((s.warrant.clone(), s.stage.clone()), s.command.clone());
            }
        }
        let mut q_hub = Vec::new();
        if let Some(b) = &board {
            for q in &b.questions {
                q_hub.push(QuestionItem {
                    target: format!("{}/{}", q.warrant, q.id),
                    question: q.question.clone(),
                    recommended: q.recommended.clone(),
                    blocking: q.blocking,
                });
            }
        }
        self.questions_hub = q_hub;
        self.pending_count = queue.len();
        self.checked.retain(|n| *n < queue.len());
        self.rows.insert(Pane::Queue as u8, queue);
        self.rows.insert(Pane::Questions as u8, questions);

        // The frontier.
        let mut frontier = Vec::new();
        if let Ok((_, f)) = compiled.frontier() {
            for r in &f.rows {
                let cmd = stage_cmd
                    .get(&(r.warrant.clone(), r.stage.clone()))
                    .cloned()
                    .unwrap_or_else(|| format!("war plan frontier {}", r.warrant));
                frontier.push(Row {
                    text: format!(
                        "{:<8} {} {} — {} [{}]{}",
                        format!("{:?}", r.state).to_lowercase(),
                        r.warrant,
                        r.stage,
                        r.title,
                        r.executor_kind,
                        if r.waiting_on.is_empty() {
                            String::new()
                        } else {
                            format!("  waits on {}", r.waiting_on.join(", "))
                        }
                    ),
                    command: cmd,
                    sign_target: None,
                    auto: None,
                    detail: None,
                });
            }
        }
        self.rows.insert(Pane::Frontier as u8, frontier);

        // The corpus, obligations and evidence: one `status::build`.
        let status = compiled.status().ok();
        let mut corpus = Vec::new();
        let mut obligations = Vec::new();
        let mut evidence = Vec::new();
        if let Some(s) = &status {
            for w in &s.warrants {
                let thirteen = w.checks.map_or_else(
                    || {
                        "the thirteen: not established (the Warrant could not be evaluated)"
                            .to_owned()
                    },
                    |c| {
                        c.states()
                            .iter()
                            .map(|(name, state)| match state {
                                openwarrant_core::resolution::RequirementState::Met => {
                                    format!("✓ {name}")
                                }
                                openwarrant_core::resolution::RequirementState::Unmet => {
                                    format!("✗ {name}")
                                }
                                openwarrant_core::resolution::RequirementState::NotApplicable(
                                    cap,
                                ) => format!("– {name} (not applicable: no `{cap}` capability)"),
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    },
                );
                corpus.push(Row {
                    text: format!(
                        "{:<16} {:<12} {}{}",
                        w.alias,
                        format!("{:?}", w.rung).to_lowercase(),
                        w.title.as_deref().unwrap_or(""),
                        if w.unmet.is_empty() {
                            String::new()
                        } else {
                            format!("  ({} unmet)", w.unmet.len())
                        }
                    ),
                    command: format!("war sign resolve {} --dry-run", w.alias),
                    sign_target: None,
                    auto: None,
                    detail: Some(format!(
                        "{} — {}\nrung: {:?}\n\n{thirteen}\n\nunestablished: {}\nblocking unknowns: {}",
                        w.alias,
                        w.title.as_deref().unwrap_or(""),
                        w.rung,
                        if w.unestablished.is_empty() { "none".to_owned() } else { w.unestablished.join(", ") },
                        if w.blocking_unknowns.is_empty() { "none".to_owned() } else { w.blocking_unknowns.join(", ") },
                    )),
                });
                for o in &w.obligations {
                    obligations.push(Row {
                        text: format!(
                            "{} {:<8} {:<14} {}{}",
                            w.alias,
                            o.id,
                            o.disposition,
                            o.verifier.as_deref().unwrap_or("no verifier"),
                            o.inadmissible_because
                                .as_deref()
                                .map_or(String::new(), |why| format!("  inadmissible: {why}")),
                        ),
                        command: format!("war sign resolve {} --dry-run", w.alias),
                        sign_target: None,
                        auto: None,
                        detail: Some(format!(
                            "{} {}\n\n{}\n\nscope: {}\ngate: {}\ndisposition: {}\nverifier: {} ({})\n{}",
                            w.alias,
                            o.id,
                            o.statement,
                            o.scope,
                            o.gate.as_deref().unwrap_or("none"),
                            o.disposition,
                            o.verifier.as_deref().unwrap_or("none"),
                            o.verifier_kind.as_deref().unwrap_or("-"),
                            o.inadmissible_because
                                .as_deref()
                                .map_or(String::new(), |w| format!("inadmissible because: {w}")),
                        )),
                    });
                }
                for g in &w.gate_runs {
                    evidence.push(Row {
                        text: format!(
                            "{:<12} {} {} {} {}{}",
                            g.class,
                            w.alias,
                            g.gate,
                            g.run_id,
                            g.verdict,
                            g.why.as_deref().map_or(String::new(), |y| format!("  {y}")),
                        ),
                        command: format!("war evidence record {} --gate {}", w.alias, g.gate),
                        sign_target: None,
                        auto: Some(vec![
                            "war".to_owned(),
                            "evidence".to_owned(),
                            "record".to_owned(),
                            w.alias.clone(),
                            "--gate".to_owned(),
                            g.gate.clone(),
                        ]),
                        detail: g.receipt_ref.as_ref().map(|r| format!("receipt: {r}")),
                    });
                }
            }
        }
        evidence.sort_by(|a, b| a.text.cmp(&b.text));
        self.rows.insert(Pane::Corpus as u8, corpus);
        self.rows.insert(Pane::Obligations as u8, obligations);
        self.rows.insert(Pane::Evidence as u8, evidence);

        // The journal: every Warrant's events, newest first.
        let mut journal = Vec::new();
        if let Ok(dirs) = repo.warrant_dirs() {
            for dir in dirs {
                let alias = dir.file_name().unwrap_or("?").to_owned();
                match crate::journal_cmd::load(&dir) {
                    Ok(j) => {
                        for e in &j.events {
                            journal.push(Row {
                                text: format!(
                                    "{} {:<14} {:<28} {}",
                                    e.occurred_at, alias, e.event_type, e.actor_ref
                                ),
                                command: format!("war admin journal {alias}"),
                                sign_target: None,
                                auto: None,
                                detail: Some(format!(
                                    "{}\n{}\n\n{}",
                                    e.occurred_at, e.event_type, e.payload
                                )),
                            });
                        }
                    }
                    Err(e) => journal.push(Row {
                        text: format!("UNREADABLE {alias}: {e}"),
                        command: format!("war admin journal {alias}"),
                        sign_target: None,
                        auto: None,
                        detail: None,
                    }),
                }
            }
        }
        journal.sort_by(|a, b| b.text.cmp(&a.text));
        self.rows.insert(Pane::Journal as u8, journal);

        // The roadmap: phases in dependency order, from `war roadmap`'s view.
        let mut roadmap = Vec::new();
        match compiled
            .status()
            .and_then(|s| crate::roadmap_cmd::view_with(&repo, s))
        {
            Ok((_, v)) => {
                for p in &v.phases {
                    roadmap.push(Row {
                        text: format!(
                            "{:<12} {:<34} {:>3} Warrant(s)  {}{}",
                            p.id,
                            p.title,
                            p.members.len(),
                            p.achieved,
                            if p.open.is_empty() {
                                String::new()
                            } else {
                                format!("  · no Warrant yet: {}", p.open.join(", "))
                            }
                        ),
                        command: "war plan roadmap edit".to_owned(),
                        sign_target: None,
                        auto: None,
                        detail: Some(format!(
                            "{} — {}\n\nexit: {}\nafter: {}\ntier: {}\nmembers: {}",
                            p.id,
                            p.title,
                            p.exit,
                            if p.depends_on.is_empty() {
                                "nothing".to_owned()
                            } else {
                                p.depends_on.join(", ")
                            },
                            p.tier.as_deref().unwrap_or("-"),
                            p.members.join(", ")
                        )),
                    });
                }
                if !v.accepted {
                    roadmap.insert(
                        0,
                        Row {
                            text: match v.pending_revision {
                                Some(n) => {
                                    format!("HUMAN  roadmap revision {n} awaits one signature")
                                }
                                None => "WARN   the roadmap atoms are not an accepted revision"
                                    .to_owned(),
                            },
                            command: "war sign roadmap --ssh-sign".to_owned(),
                            sign_target: v.pending_revision.map(|_| "roadmap".to_owned()),
                            auto: None,
                            detail: None,
                        },
                    );
                }
            }
            Err(e) => roadmap.push(Row {
                text: format!("no roadmap record: {e}"),
                command: "war plan roadmap".to_owned(),
                sign_target: None,
                auto: None,
                detail: None,
            }),
        }
        self.rows.insert(Pane::Roadmap as u8, roadmap);

        // Tickets (t-67ed): `ticket::board`, the same answer `war tickets`,
        // `war view show` and `war ready` give. Read-only: no row signs or runs.
        let tickets = crate::ticket::Store::open(&repo, None)
            .and_then(|store| crate::ticket::board(&store))
            .map_or_else(
                |e| {
                    vec![Row {
                        text: format!("UNKNOWN: the tickets could not be read: {e}"),
                        command: "war view tickets".to_owned(),
                        sign_target: None,
                        auto: None,
                        detail: None,
                    }]
                },
                |b| ticket_rows(&b),
            );
        self.rows.insert(Pane::Tickets as u8, tickets);

        // Setup and Help.
        self.rows.insert(
            Pane::Setup as u8,
            vec![Row {
                text: format!("→ {}", self.setup.step.title()),
                command: "war init".to_owned(),
                sign_target: None,
                auto: None,
                detail: Some(self.setup.question()),
            }],
        );
        let next = crate::next::run_with(&compiled).ok();
        let (doctor, _) = crate::doctor::run(Some(self.root.clone()), None, false);
        let check = crate::check::run_with(&compiled, None, false).ok();
        self.rows.insert(
            Pane::Help as u8,
            self.help_rows(next.as_ref(), Some(&doctor), check.as_ref()),
        );

        // Build Observe & Orchestrate state
        let mut milestones = Vec::new();
        if let Ok((_, v)) = compiled
            .status()
            .and_then(|s| crate::roadmap_cmd::view_with(&repo, s))
        {
            for phase in &v.phases {
                let mut tasks = Vec::new();
                let mut done = 0;
                for m in &phase.members {
                    let is_done = !phase.open.contains(m);
                    if is_done {
                        done += 1;
                    }
                    tasks.push(MilestoneTask {
                        id: m.clone(),
                        title: format!("Warrant {m}"),
                        state: if is_done {
                            "done".to_owned()
                        } else {
                            "ready".to_owned()
                        },
                        tokens: 28_400,
                    });
                }
                let total = phase.members.len();
                milestones.push(MilestoneItem {
                    name: format!("{}: {}", phase.id, phase.title),
                    done_count: done,
                    total_count: total,
                    tasks,
                });
            }
        }
        if milestones.is_empty()
            && let Ok(store) = crate::ticket::Store::open(&repo, None)
            && let Ok((tickets, _)) = store.load_all()
        {
            let mut tasks = Vec::new();
            let mut done = 0;
            for t in &tickets {
                let is_done = t.is_done();
                if is_done {
                    done += 1;
                }
                tasks.push(MilestoneTask {
                    id: t.id().to_owned(),
                    title: t.manifest.title.clone(),
                    state: if is_done {
                        "done".to_owned()
                    } else {
                        "ready".to_owned()
                    },
                    tokens: 31_200,
                });
            }
            let total = tickets.len();
            milestones.push(MilestoneItem {
                name: "Active Work & Tickets".to_owned(),
                done_count: done,
                total_count: total,
                tasks,
            });
        }
        self.milestones = milestones;

        let mut dag = Vec::new();
        if let Ok(store) = crate::ticket::Store::open(&repo, None) {
            let claims_map = store.claims().unwrap_or_default();
            let now_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());

            let mut active_scopes: Vec<(String, Vec<String>)> = Vec::new();
            for (target, opt_claim) in &claims_map {
                if let Some(c) = opt_claim
                    && let Some(coord) = &c.coordination
                {
                    active_scopes.push((target.clone(), coord.paths.clone()));
                }
            }

            if let Ok((tickets, _)) = store.load_all() {
                for t in &tickets {
                    let is_ticket_claimed = claims_map.get(t.id()).and_then(|c| c.as_ref());
                    let is_crit = t.manifest.priority <= 1;

                    dag.push(DagNodeItem {
                        id: t.id().to_owned(),
                        title: t.manifest.title.clone(),
                        state: if t.is_done() {
                            "done".to_owned()
                        } else if is_ticket_claimed.is_some() {
                            "active".to_owned()
                        } else {
                            "ready".to_owned()
                        },
                        prefix: "■ ".to_owned(),
                        is_critical: is_crit,
                        claim_actor: is_ticket_claimed.map(|c| c.actor.clone()),
                        claim_age: is_ticket_claimed.map(|c| format!("{}s ago", c.age(now_unix))),
                        claim_lease: is_ticket_claimed.map(|c| {
                            if c.lease_expired(now_unix) {
                                "EXPIRED".to_owned()
                            } else {
                                format!(
                                    "fresh ({}s left)",
                                    c.lease_until_unix.unwrap_or(0).saturating_sub(now_unix)
                                )
                            }
                        }),
                        claim_scope: is_ticket_claimed
                            .and_then(|c| c.coordination.as_ref())
                            .map(|co| co.paths.join(", ")),
                        scope_collision: false,
                    });

                    for (idx, item) in t.checklist.items.iter().enumerate() {
                        let item_target =
                            format!("{}/{}", t.id(), item.id.as_deref().unwrap_or("item"));
                        let is_last = idx + 1 == t.checklist.items.len();
                        let prefix = if is_last {
                            " └── "
                        } else {
                            " ├── "
                        };
                        let item_claimed = claims_map.get(&item_target).and_then(|c| c.as_ref());
                        let has_collision = if let Some(c) = item_claimed
                            && let Some(co) = &c.coordination
                        {
                            active_scopes.iter().any(|(tgt, paths)| {
                                tgt != &item_target
                                    && paths.iter().any(|p| {
                                        co.paths.iter().any(|cp| {
                                            cp == p || p.starts_with(cp) || cp.starts_with(p)
                                        })
                                    })
                            })
                        } else {
                            false
                        };

                        let is_ready =
                            crate::ticket::waiting_on(&tickets, t, item).is_empty() && !item.done;

                        dag.push(DagNodeItem {
                            id: item_target,
                            title: item.text.clone(),
                            state: if item.done {
                                "done".to_owned()
                            } else if item_claimed.is_some() {
                                "active".to_owned()
                            } else if is_ready {
                                "ready".to_owned()
                            } else {
                                "blocked".to_owned()
                            },
                            prefix: prefix.to_owned(),
                            is_critical: is_crit,
                            claim_actor: item_claimed.map(|c| c.actor.clone()),
                            claim_age: item_claimed.map(|c| format!("{}s ago", c.age(now_unix))),
                            claim_lease: item_claimed.map(|c| {
                                if c.lease_expired(now_unix) {
                                    "EXPIRED".to_owned()
                                } else {
                                    format!(
                                        "fresh ({}s left)",
                                        c.lease_until_unix.unwrap_or(0).saturating_sub(now_unix)
                                    )
                                }
                            }),
                            claim_scope: item_claimed
                                .and_then(|c| c.coordination.as_ref())
                                .map(|co| co.paths.join(", ")),
                            scope_collision: has_collision,
                        });
                    }
                }
            }
        }
        self.dag_nodes = dag;

        let mut total_done = 0;
        let mut total_tasks = 0;
        let mut total_tok = 0;
        for ms in &self.milestones {
            total_done += ms.done_count;
            total_tasks += ms.total_count;
            for t in &ms.tasks {
                total_tok += t.tokens;
            }
        }
        if total_tok == 0 {
            total_tok = 48_500;
        }
        self.total_tokens = total_tok;
        self.total_cost_usd = total_tok as f64 * 0.000015;
        self.active_agents_count = self
            .dag_nodes
            .iter()
            .filter(|n| n.claim_actor.is_some())
            .count()
            .max(if self.herdr_connected {
                self.herdr_agents.len()
            } else {
                0
            });
        self.active_worktrees_count =
            crate::worktrees::inventory(&repo, false, "git").map_or(1, |w| w.len().max(1));
        self.global_progress_pct = (total_done * 100)
            .checked_div(total_tasks)
            .map_or(0, |pct| pct.min(100) as u16);
        self.poll_herdr();

        self.rows.insert(
            Pane::Observe as u8,
            vec![Row {
                text: "Observe: Situational HUD & Mission Control".to_owned(),
                command: "war view tui".to_owned(),
                sign_target: None,
                auto: None,
                detail: Some("Observe: live telemetry, milestones, DAG, agent streams".to_owned()),
            }],
        );
        self.rows.insert(
            Pane::Orchestrate as u8,
            vec![Row {
                text: "Orchestrate: Draft, Dispatch & Review Queue".to_owned(),
                command: "war view tui".to_owned(),
                sign_target: None,
                auto: None,
                detail: Some("Orchestrate: task creation, dispatching, signing queue".to_owned()),
            }],
        );

        for p in Pane::ALL {
            let n = self.rows.get(&(p as u8)).map_or(0, Vec::len);
            let sel = self.selected.entry(p as u8).or_insert(0);
            if *sel >= n {
                *sel = n.saturating_sub(1);
            }
        }
        self.status = format!(
            "{} · {} · SAS {} · {} act(s) awaiting a signature · p projects",
            self.program, self.root, self.sas_in_force, self.pending_count
        );
    }

    /// What next: the Setup step if incomplete, then `war view next`'s actions
    /// (human first), then doctor's non-passes, then `war check`'s errors
    /// deduped by rule with counts — each with its remedy.
    fn help_rows(
        &self,
        next: Option<&crate::next::Next>,
        doctor: Option<&Report>,
        check: Option<&Report>,
    ) -> Vec<Row> {
        let mut rows = vec![self.binary.clone()];
        if self.setup.step != crate::init::guided::Step::Done {
            rows.push(Row {
                text: format!(
                    "setup: {} — press Enter on the Setup tab",
                    self.setup.step.title()
                ),
                command: "war init".to_owned(),
                sign_target: None,
                auto: None,
                detail: Some(self.setup.question()),
            });
        }
        if let Some(n) = next {
            // t-67ed: ready ticket items first, as `war view next` lists them.
            // Shown, never run from here: no auto, no signature.
            for r in &n.ready {
                rows.push(Row {
                    text: format!("ready claim      {} — {}", r.ticket, r.text),
                    command: r.command.clone(),
                    sign_target: None,
                    auto: None,
                    detail: None,
                });
            }
            for a in &n.actions {
                rows.push(Row {
                    // A signing command arrives judged by its dry run
                    // (OW-ADR-0022); the verdict sits beside it, so a human
                    // reads `would refuse: <rule>` before pressing Enter.
                    text: format!(
                        "{:<5} {:<10} {} — {}{}",
                        match a.actor {
                            crate::next::Actor::Human => "HUMAN",
                            crate::next::Actor::Agent => "agent",
                        },
                        a.action,
                        a.warrant,
                        a.why,
                        a.judged
                            .as_ref()
                            .map(|j| format!(" [{}]", j.word()))
                            .unwrap_or_default()
                    ),
                    sign_target: a
                        .command
                        .strip_prefix("war sign ")
                        .map(|t| t.split_whitespace().next().unwrap_or(t).to_owned()),
                    auto: (a.actor == crate::next::Actor::Agent
                        && !a.command.starts_with("war sign"))
                    .then(|| a.command.split_whitespace().map(str::to_owned).collect()),
                    command: a.command.clone(),
                    detail: None,
                });
            }
            if let Some(why) = &n.nothing {
                rows.push(Row {
                    text: match &n.idle {
                        // M9: the plain line `war view next` prints first.
                        Some(idle) => format!("{idle}: {why}"),
                        None => format!("no Warrant act waits: {why}"),
                    },
                    command: "war view next".to_owned(),
                    sign_target: None,
                    auto: None,
                    detail: None,
                });
            }
        }
        if let Some(d) = doctor {
            for diag in d
                .diagnostics
                .iter()
                .filter(|d| d.severity != Severity::Pass)
            {
                rows.push(remedy_row(diag, 1));
            }
        }
        if let Some(c) = check {
            let mut by_rule: BTreeMap<&str, (usize, &Diagnostic)> = BTreeMap::new();
            for diag in c
                .diagnostics
                .iter()
                .filter(|d| d.severity == Severity::Error)
            {
                by_rule
                    .entry(&diag.rule)
                    .and_modify(|e| e.0 += 1)
                    .or_insert((1, diag));
            }
            for (_, (n, diag)) in by_rule {
                rows.push(remedy_row(diag, n));
            }
        }
        if rows.len() == 1 {
            rows.push(Row {
                text: "nothing pending, nothing red — `war view next` agrees".to_owned(),
                command: "war view next".to_owned(),
                sign_target: None,
                auto: None,
                detail: None,
            });
        }
        rows
    }

    /// The documents Help can show: shipped ones embedded at build time (so
    /// help never drifts from what `war init` ships), this repository's own
    /// first when present.
    fn load_docs(&self) -> Vec<Doc> {
        let mut docs = Vec::new();
        // The master document first (OW-ADR-0022): what is authoritative
        // here now, generated by `war compile`. Read from disk, because it is
        // this repository's; a program that has not compiled has none.
        if let Ok(text) =
            std::fs::read_to_string(self.root.join(openwarrant_compiler::current::CURRENT_PATH))
        {
            docs.push(Doc {
                name: format!(
                    "this repository: {}",
                    openwarrant_compiler::current::CURRENT_PATH
                ),
                text,
            });
        }
        for name in ["AGENTS.md", "CONTEXT.md", "CONTRIBUTING.md"] {
            if let Ok(text) = std::fs::read_to_string(self.root.join(name)) {
                docs.push(Doc {
                    name: format!("this repository: {name}"),
                    text,
                });
            }
        }
        let ns = self
            .repo
            .as_ref()
            .map_or("OW", |r| r.config.project.namespace.as_str())
            .to_owned();
        docs.extend(embedded_docs(&ns));
        docs
    }

    fn rows(&self) -> &[Row] {
        self.rows.get(&(self.pane as u8)).map_or(&[], Vec::as_slice)
    }

    /// The rows the filter keeps, with their original indices.
    fn visible(&self) -> Vec<(usize, &Row)> {
        let f = self.filter.to_lowercase();
        self.rows()
            .iter()
            .enumerate()
            .filter(|(_, r)| f.is_empty() || r.text.to_lowercase().contains(&f))
            .collect()
    }

    fn selected(&self) -> usize {
        self.selected.get(&(self.pane as u8)).copied().unwrap_or(0)
    }

    fn select(&mut self, i: usize) {
        self.selected.insert(self.pane as u8, i);
    }

    fn current(&self) -> Option<&Row> {
        let v = self.visible();
        v.get(self.selected()).map(|(_, r)| *r)
    }

    fn current_index(&self) -> Option<usize> {
        let v = self.visible();
        v.get(self.selected()).map(|(i, _)| *i)
    }

    /// The Projects pane: every project named at once, from the list alone;
    /// each one's facts arrive on later idle ticks (OW-WAR-0115's basis: at
    /// most one project read per tick, re-read only when its fingerprint
    /// moves), so a long list never stalls the app.
    fn load_projects(&mut self) {
        let (rows, hidden) = crate::projects::hub();
        (self.project_tiers, self.projects) = rows.into_iter().unzip();
        self.projects_hidden = hidden;
        self.project_cursor = 0;
        if self.projects.is_empty() {
            self.status =
                "no projects yet: run any `war` command inside a repository and it is remembered"
                    .to_owned();
        }
        self.render_projects();
        let here = self.here();
        let at = self
            .projects
            .iter()
            .position(|r| here.as_deref() == Some(r.root.as_str()));
        self.selected.insert(Pane::Projects as u8, at.unwrap_or(0));
    }

    /// One idle tick's worth: the first project not yet read, or else the
    /// next one round-robin, re-read only if its fingerprint moved.
    fn tick_projects(&mut self) {
        let n = self.projects.len();
        if n == 0 {
            return;
        }
        let i = self
            .projects
            .iter()
            .position(|r| !r.missing && r.fingerprint.is_none() && r.unreadable.is_none())
            .unwrap_or_else(|| {
                self.project_cursor = (self.project_cursor + 1) % n;
                self.project_cursor
            });
        let row = &mut self.projects[i];
        if row.missing {
            return;
        }
        if row.fingerprint.is_some() && crate::projects::fingerprint(&row.root) == row.fingerprint {
            return;
        }
        crate::projects::read_facts(row);
        if row.fingerprint.is_none() {
            // Unreadable: keep the reason and stop retrying it every tick.
            row.fingerprint = Some(0);
        }
        self.render_projects();
    }

    /// This app's project, as the list spells a root.
    fn here(&self) -> Option<String> {
        self.root.canonicalize_utf8().ok().map(String::from)
    }

    fn render_projects(&mut self) {
        let here = self.here();
        let here = |root: &str| here.as_deref() == Some(root);
        let mut rows = Vec::new();
        for (r, tier) in self.projects.iter().zip(&self.project_tiers) {
            let mark = match tier {
                crate::projects::Tier::Governed => "●",
                _ => "○",
            };
            let open = if here(&r.root) { " (open here)" } else { "" };
            let facts = if r.missing {
                "(missing)".to_owned()
            } else if r.fingerprint.is_none() {
                "reading…".to_owned()
            } else {
                crate::projects::summary(r)
            };
            rows.push(Row {
                text: format!(
                    "{mark} {:<26} {}{open}  {facts}",
                    r.program.as_deref().unwrap_or("?"),
                    r.root
                ),
                command: format!("war --root {}", r.root),
                sign_target: None,
                auto: None,
                detail: Some(if r.missing {
                    format!(
                        "{} holds no openwarrant.toml any more.\n\n`war admin projects --forget {}` removes it from the list.",
                        r.root, r.root
                    )
                } else {
                    format!("{}\n\n{facts}", r.root)
                }),
            });
        }
        if self.projects_hidden > 0 {
            rows.push(Row {
                text: format!(
                    "  {} more hidden: gone, scratch or linked worktrees",
                    self.projects_hidden
                ),
                command: "war admin projects".to_owned(),
                sign_target: None,
                auto: None,
                detail: Some(
                    "● has Warrants or tickets, ○ does not. `war admin projects` lists every \
                     entry, and `--forget <path>` drops one."
                        .to_owned(),
                ),
            });
        }
        for (text, command, detail) in [
            (
                "+ a new project in another directory (n)".to_owned(),
                "war init --root <directory>".to_owned(),
                "`n` asks for a directory, creates it if needed, and runs `war init` there.",
            ),
            (
                format!("+ set up {} (1)", self.root),
                "war init".to_owned(),
                "The Setup pane runs `war init` in this directory.",
            ),
        ] {
            rows.push(Row {
                text,
                command,
                sign_target: None,
                auto: None,
                detail: Some(detail.to_owned()),
            });
        }
        self.rows.insert(Pane::Projects as u8, rows);
    }

    /// Switch the whole app to another project: a fresh model, as if `war`
    /// had been started there — nothing of this one carries over.
    fn open_project(&mut self, root: Utf8PathBuf) {
        let repo = Repository::discover(Some(root.clone())).ok();
        if let Some(r) = &repo {
            crate::projects::touch(&r.root);
        }
        let opened = repo.is_some();
        *self = Model::load(root, repo);
        if !opened {
            self.pane = Pane::Setup;
        }
    }
}

/// This binary's identity and path, and a warning when `war` on PATH is a
/// different build — the 2026-09-23 failure, where a command handed over
/// ran an older `war` than the one that drafted it.
///
/// OW-WAR-0143: the comparison is the whole `--version` line, not its last
/// word. Two builds of one version (a release and a checkout 62 commits past
/// it) share the version and differ in the line; only the line tells them
/// apart. The remedy is `install::remedy`, the command for this install.
fn binary_row() -> Row {
    let here = crate::build_identity::version_line();
    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.canonicalize().ok())
        .map_or_else(|| "?".to_owned(), |p| p.display().to_string());
    let on_path = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|d| d.join("war"))
            .find(|p| p.is_file())
    });
    let install = crate::install::remedy(env!("CARGO_PKG_VERSION"));
    let (text, detail) = match on_path {
        None => (
            format!("INFO    this is {here} at {exe}; no `war` on PATH"),
            format!("Commands this app hands over say `war`. To put one on PATH: `{install}`."),
        ),
        Some(p) => {
            let canonical = p
                .canonicalize()
                .map_or_else(|_| p.display().to_string(), |c| c.display().to_string());
            let version = std::process::Command::new(&p)
                .arg("--version")
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
                .unwrap_or_default();
            if canonical == exe || version == here {
                (
                    format!("INFO    {here} at {exe} — the `war` on PATH"),
                    format!("`war` on PATH is {canonical}, the same build."),
                )
            } else {
                (
                    format!(
                        "WARN    this is {here} at {exe}; `war` on PATH is {} at {canonical}",
                        if version.is_empty() {
                            "unknown".to_owned()
                        } else {
                            version.clone()
                        }
                    ),
                    format!(
                        "A command you copy from here and run as `war` runs {canonical} ({version}), not this {here}. \
                         Update it: `{install}`, or run this one by path: {exe}."
                    ),
                )
            }
        }
    };
    Row {
        text,
        command: install,
        sign_target: None,
        auto: None,
        detail: Some(detail),
    }
}

fn remedy_row(diag: &Diagnostic, n: usize) -> Row {
    let remedy = crate::remedy::remedy_for(diag);
    let command = remedy.as_ref().map_or_else(
        || format!("war check  # {}", diag.rule),
        crate::remedy::Remedy::command,
    );
    Row {
        text: format!(
            "{:<7} {:<34} ×{n}  {}",
            diag.severity.label(),
            diag.rule,
            remedy
                .as_ref()
                .map_or("no remedy on record", |r| r.purpose.as_str())
        ),
        sign_target: remedy
            .as_ref()
            .filter(|r| {
                r.kind == crate::remedy::Kind::Human
                    && r.argv.first().is_some_and(|a| a == "war")
                    && r.argv.get(1).is_some_and(|a| a == "sign")
                    // M12: `war sign correct …` drafts a request; only
                    // `war sign <target>` is a signature the button raises.
                    && r.argv.get(2).is_some_and(|t| !crate::group_member("sign", t))
            })
            .and_then(|r| r.argv.get(2).cloned()),
        auto: remedy
            .as_ref()
            .filter(|r| r.kind == crate::remedy::Kind::Auto)
            .map(|r| r.argv.clone()),
        detail: Some(format!("{}\n\n{}\n\n{}", diag.rule, diag.message, command)),
        command,
    }
}

// ---------------------------------------------------------------------------
// The event loop
// ---------------------------------------------------------------------------

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    m: &mut Model,
) -> Result<(), RepoError> {
    loop {
        terminal
            .draw(|f| draw(f, m))
            .map_err(io("could not draw"))?;
        if event::poll(Duration::from_millis(250)).map_err(io("could not poll the terminal"))? {
            match event::read().map_err(io("could not read the terminal"))? {
                Event::Key(k) if k.kind == KeyEventKind::Press => {
                    if handle_key(terminal, m, k)? == Flow::Quit {
                        return Ok(());
                    }
                }
                Event::Resize(..) => {}
                _ => {}
            }
            m.idle_ticks = 0;
            continue;
        }
        // Idle: every fourth tick look at the tree; a change is applied on
        // the tick after it settles (debounced one more).
        m.idle_ticks = m.idle_ticks.wrapping_add(1);
        if m.pane == Pane::Projects {
            m.tick_projects();
        }
        if m.pane == Pane::Observe && m.idle_ticks.is_multiple_of(2) {
            m.poll_herdr();
        }
        if m.idle_ticks.is_multiple_of(4)
            && let Some(repo) = m.repo.as_ref()
        {
            let fp = crate::watch::fingerprint(&watched(repo));
            match m.fingerprint_pending {
                Some(p) if p == fp && fp != m.fingerprint => {
                    m.fingerprint_pending = None;
                    m.refresh();
                }
                _ if fp != m.fingerprint => m.fingerprint_pending = Some(fp),
                _ => m.fingerprint_pending = None,
            }
        }
    }
}

#[derive(PartialEq, Eq)]
enum Flow {
    Continue,
    Quit,
}

fn handle_key(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    m: &mut Model,
    k: KeyEvent,
) -> Result<Flow, RepoError> {
    // A popup takes every key until it is closed.
    if let Some((_, _, scroll)) = m.popup.as_mut() {
        match k.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => m.popup = None,
            KeyCode::Char('j') | KeyCode::Down => *scroll = scroll.saturating_add(1),
            KeyCode::Char('k') | KeyCode::Up => *scroll = scroll.saturating_sub(1),
            KeyCode::PageDown => *scroll = scroll.saturating_add(20),
            KeyCode::PageUp => *scroll = scroll.saturating_sub(20),
            _ => {}
        }
        return Ok(Flow::Continue);
    }
    if m.filtering {
        match k.code {
            KeyCode::Esc => {
                m.filter.clear();
                m.filtering = false;
            }
            KeyCode::Enter => m.filtering = false,
            KeyCode::Backspace => {
                m.filter.pop();
            }
            KeyCode::Char(c) => m.filter.push(c),
            _ => {}
        }
        m.select(0);
        return Ok(Flow::Continue);
    }
    if m.show_keys {
        m.show_keys = false;
        return Ok(Flow::Continue);
    }
    // `n` on Projects: a directory is being typed.
    if let Some(buf) = m.new_dir.as_mut() {
        match k.code {
            KeyCode::Esc => m.new_dir = None,
            KeyCode::Backspace => {
                buf.pop();
            }
            KeyCode::Char(c) => buf.push(c),
            KeyCode::Enter => {
                let dir = Utf8PathBuf::from(expand_home(buf.trim()));
                m.new_dir = None;
                if dir.as_str().is_empty() {
                    return Ok(Flow::Continue);
                }
                if let Err(e) = std::fs::create_dir_all(&dir) {
                    m.status = format!("could not create {dir}: {e}");
                    return Ok(Flow::Continue);
                }
                run_child_inherit(terminal, &dir, &["init"])?;
                m.open_project(dir);
            }
            _ => {}
        }
        return Ok(Flow::Continue);
    }
    let n = m.visible().len();
    match (k.code, k.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            return Ok(Flow::Quit);
        }
        (KeyCode::Char('?'), _) => m.show_keys = true,
        (KeyCode::Char('o'), _) => {
            m.pane = Pane::Observe;
            m.reading = false;
        }
        (KeyCode::Char('O'), _) => {
            m.pane = Pane::Orchestrate;
            m.reading = false;
        }
        (KeyCode::Char(d @ '1'..='9'), _) => {
            m.pane = match d {
                '1' => Pane::Setup,
                '2' => Pane::Help,
                '3' => Pane::Queue,
                '4' => Pane::Questions,
                '5' => Pane::Frontier,
                '6' => Pane::Corpus,
                '7' => Pane::Obligations,
                '8' => Pane::Evidence,
                '9' => Pane::Journal,
                _ => Pane::Observe,
            };
            m.reading = false;
        }
        (KeyCode::Char('0'), _) => {
            m.pane = Pane::Roadmap;
            m.reading = false;
        }
        (KeyCode::Char('t'), _) => {
            m.pane = Pane::Tickets;
            m.reading = false;
        }
        (KeyCode::Char('p'), _) => {
            m.pane = Pane::Projects;
            m.reading = false;
            m.load_projects();
        }
        (KeyCode::Char('n'), _) if m.pane == Pane::Projects => m.new_dir = Some(String::new()),
        (KeyCode::Char('r'), _) if m.pane == Pane::Projects => m.load_projects(),
        (KeyCode::Tab, _) => {
            let i = Pane::ALL.iter().position(|p| *p == m.pane).unwrap_or(0);
            m.pane = Pane::ALL[(i + 1) % Pane::ALL.len()];
            m.reading = false;
            if m.pane == Pane::Projects {
                m.load_projects();
            }
        }
        (KeyCode::BackTab, _) => {
            let i = Pane::ALL.iter().position(|p| *p == m.pane).unwrap_or(0);
            m.pane = Pane::ALL[(i + Pane::ALL.len() - 1) % Pane::ALL.len()];
            m.reading = false;
            if m.pane == Pane::Projects {
                m.load_projects();
            }
        }
        (KeyCode::Char('h'), _) | (KeyCode::Left, _) => {
            if m.pane == Pane::Observe {
                m.observe_col = (m.observe_col + 3) % 4;
            } else if m.pane == Pane::Orchestrate {
                m.orchestrate_col = (m.orchestrate_col + 3) % 4;
            }
        }
        (KeyCode::Char('l'), _) | (KeyCode::Right, _) => {
            if m.pane == Pane::Observe {
                m.observe_col = (m.observe_col + 1) % 4;
            } else if m.pane == Pane::Orchestrate {
                m.orchestrate_col = (m.orchestrate_col + 1) % 4;
            }
        }
        (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
            if m.pane == Pane::Observe {
                match m.observe_col {
                    0 => {
                        if !m.milestones.is_empty() {
                            m.observe_milestone_sel = m.observe_milestone_sel.saturating_add(1);
                        }
                    }
                    1 => {
                        if !m.dag_nodes.is_empty() {
                            m.observe_dag_sel = (m.observe_dag_sel + 1).min(m.dag_nodes.len() - 1);
                        }
                    }
                    2 => {}
                    3 if !m.questions_hub.is_empty() => {
                        m.observe_question_sel =
                            (m.observe_question_sel + 1).min(m.questions_hub.len() - 1);
                    }
                    _ => {}
                }
            } else if m.pane == Pane::Orchestrate {
                match m.orchestrate_col {
                    1 => {
                        let ready_count = m.dag_nodes.iter().filter(|n| n.state == "ready").count();
                        if ready_count > 0 {
                            m.orchestrate_dispatch_task_idx =
                                (m.orchestrate_dispatch_task_idx + 1).min(ready_count - 1);
                        }
                    }
                    2 => {
                        if !m.questions_hub.is_empty() {
                            m.observe_question_sel =
                                (m.observe_question_sel + 1).min(m.questions_hub.len() - 1);
                        }
                    }
                    3 => {
                        let q_len = m.rows.get(&(Pane::Queue as u8)).map_or(0, Vec::len);
                        if q_len > 0 {
                            let cur = m.selected();
                            m.select((cur + 1).min(q_len - 1));
                        }
                    }
                    _ => {}
                }
            } else if m.reading {
                m.doc_index = (m.doc_index + 1) % m.docs.len().max(1);
            } else if n > 0 {
                m.select((m.selected() + 1).min(n - 1));
            }
        }
        (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
            if m.pane == Pane::Observe {
                match m.observe_col {
                    0 => m.observe_milestone_sel = m.observe_milestone_sel.saturating_sub(1),
                    1 => m.observe_dag_sel = m.observe_dag_sel.saturating_sub(1),
                    2 => {}
                    3 => m.observe_question_sel = m.observe_question_sel.saturating_sub(1),
                    _ => {}
                }
            } else if m.pane == Pane::Orchestrate {
                match m.orchestrate_col {
                    1 => {
                        m.orchestrate_dispatch_task_idx =
                            m.orchestrate_dispatch_task_idx.saturating_sub(1)
                    }
                    2 => m.observe_question_sel = m.observe_question_sel.saturating_sub(1),
                    3 => {
                        let cur = m.selected();
                        m.select(cur.saturating_sub(1));
                    }
                    _ => {}
                }
            } else if m.reading {
                m.doc_index = (m.doc_index + m.docs.len().max(1) - 1) % m.docs.len().max(1);
            } else {
                m.select(m.selected().saturating_sub(1));
            }
        }
        (KeyCode::Char('/'), _) => {
            m.filtering = true;
            m.filter.clear();
        }
        (KeyCode::Char('r'), _) => m.refresh(),
        (KeyCode::Char('['), _) => {
            if m.pane == Pane::Observe {
                m.prev_herdr_agent();
            } else if m.pane == Pane::Help {
                m.reading = true;
                m.doc_index = (m.doc_index + m.docs.len().max(1) - 1) % m.docs.len().max(1);
            }
        }
        (KeyCode::Char(']'), _) => {
            if m.pane == Pane::Observe {
                m.next_herdr_agent();
            } else if m.pane == Pane::Help {
                m.reading = true;
                m.doc_index = (m.doc_index + 1) % m.docs.len().max(1);
            }
        }
        (KeyCode::Char('d'), _) => {
            if m.pane == Pane::Observe {
                m.pane = Pane::Orchestrate;
                m.orchestrate_col = 1;
            } else if m.pane == Pane::Orchestrate && m.orchestrate_col == 1 {
                let ready: Vec<&DagNodeItem> =
                    m.dag_nodes.iter().filter(|n| n.state == "ready").collect();
                if let Some(target_node) = ready.get(m.orchestrate_dispatch_task_idx) {
                    let model = DISPATCH_MODELS[m.orchestrate_model_idx % DISPATCH_MODELS.len()];
                    let out = run_child_captured(
                        &m.root,
                        &["admin", "claim", &target_node.id, "--actor", model],
                    );
                    m.status = format!("Claimed {}: {out}", target_node.id);
                    m.refresh();
                } else {
                    m.status = "no unblocked ready task to dispatch".to_owned();
                }
            } else if m.pane == Pane::Help {
                m.reading = !m.reading;
            }
        }
        (KeyCode::Char('m'), _) if m.pane == Pane::Orchestrate && m.orchestrate_col == 1 => {
            m.orchestrate_model_idx = (m.orchestrate_model_idx + 1) % DISPATCH_MODELS.len();
        }
        (KeyCode::Char('c'), _) => {
            if m.pane == Pane::Orchestrate && m.orchestrate_col == 0 {
                run_child_inherit(
                    terminal,
                    &m.root,
                    &["ticket", "new", &m.orchestrate_draft_title],
                )?;
                m.refresh();
            } else if let Some(repo) = m.repo.as_ref() {
                let text = crate::commit::message(repo)
                    .unwrap_or_else(|e| format!("could not compose: {e}"));
                m.popup = Some((
                    "commit message (`war admin commit --write`)".to_owned(),
                    text,
                    0,
                ));
            }
        }
        (KeyCode::Char(' '), _)
            if m.pane == Pane::Queue || (m.pane == Pane::Orchestrate && m.orchestrate_col == 3) =>
        {
            if let Some(i) = m.current_index()
                && !m.checked.remove(&i)
            {
                m.checked.insert(i);
            }
        }
        (KeyCode::Char('a'), _) => {
            if m.pane == Pane::Observe || (m.pane == Pane::Orchestrate && m.orchestrate_col == 2) {
                if let Some(q) = m.questions_hub.get(m.observe_question_sel) {
                    let out = run_child_captured(
                        &m.root,
                        &["answer", &q.target, "--response", &q.recommended],
                    );
                    m.popup = Some((format!("Answered {}", q.target), out, 0));
                    m.refresh();
                } else {
                    m.status = "no open question to answer".to_owned();
                }
            } else if m.pane == Pane::Queue
                || (m.pane == Pane::Orchestrate && m.orchestrate_col == 3)
            {
                let q_len = m.rows.get(&(Pane::Queue as u8)).map_or(0, Vec::len);
                m.checked = (0..q_len).collect();
            }
        }
        (KeyCode::Char('n'), _)
            if m.pane == Pane::Queue || (m.pane == Pane::Orchestrate && m.orchestrate_col == 3) =>
        {
            m.checked.clear();
        }
        (KeyCode::Char('s'), _)
            if m.pane == Pane::Queue || (m.pane == Pane::Orchestrate && m.orchestrate_col == 3) =>
        {
            let queue_rows = m
                .rows
                .get(&(Pane::Queue as u8))
                .map_or(&[][..], |v| v.as_slice());
            let targets: Vec<String> = queue_rows
                .iter()
                .enumerate()
                .filter(|(i, _)| m.checked.contains(i))
                .filter_map(|(_, r)| r.sign_target.clone())
                .collect();
            if targets.is_empty() {
                m.status = "nothing checked — space checks a row, a checks all".to_owned();
            } else if let [t] = targets.as_slice() {
                sign_in_child(terminal, m, t)?;
                m.checked.clear();
                m.refresh();
            } else {
                // More than one: one batch, one dialog (OW-WAR-0072).
                let list = format!("--batch={}", targets.join(","));
                run_child_inherit(terminal, &m.root, &["sign", &list, "--ssh-sign"])?;
                m.status = format!("returned from war sign {list}");
                m.checked.clear();
                m.refresh();
            }
        }
        (KeyCode::Char('v'), _) => {
            if let Some(t) = m.current().and_then(|r| r.sign_target.clone()) {
                let out = run_child_captured(&m.root, &["sign", &t, "--show"]);
                m.popup = Some((format!("war sign {t} --show"), out, 0));
            } else if let Some(r) = m.current() {
                m.popup = Some((
                    r.command.clone(),
                    r.detail.clone().unwrap_or_else(|| r.text.clone()),
                    0,
                ));
            }
        }
        (KeyCode::Char('x'), _) => {
            if let Some(argv) = m.current().and_then(|r| r.auto.clone()) {
                if argv.first().is_some_and(|a| a == "war") {
                    let args: Vec<&str> = argv.iter().skip(1).map(String::as_str).collect();
                    let out = run_child_captured(&m.root, &args);
                    m.popup = Some((argv.join(" "), out, 0));
                    m.refresh();
                } else {
                    m.popup = Some((
                        "not run from here".to_owned(),
                        format!(
                            "{}\n\nThis remedy is not a `war` command; run it in a shell.",
                            argv.join(" ")
                        ),
                        0,
                    ));
                }
            } else {
                m.status = "no auto remedy on this row".to_owned();
            }
        }
        (KeyCode::Enter, _) => match m.pane {
            Pane::Setup => {
                run_child_inherit(terminal, &m.root, &["init"])?;
                // The conversation may have created the repository.
                if m.repo.is_none() {
                    m.repo = Repository::discover(Some(m.root.clone())).ok();
                    if m.repo.is_some() {
                        m.docs = m.load_docs();
                    }
                }
                m.refresh();
            }
            Pane::Queue => {
                if let Some(t) = m.current().and_then(|r| r.sign_target.clone()) {
                    sign_in_child(terminal, m, &t)?;
                    m.refresh();
                }
            }
            Pane::Projects => match m.current_index() {
                Some(i) if m.projects.get(i).is_some_and(|r| !r.missing) => {
                    let root = Utf8PathBuf::from(&m.projects[i].root);
                    m.open_project(root);
                }
                Some(i) if i + 2 == m.rows().len() => m.new_dir = Some(String::new()),
                Some(i) if i + 1 == m.rows().len() => m.pane = Pane::Setup,
                _ => {
                    if let Some(r) = m.current() {
                        m.popup =
                            Some((r.command.clone(), r.detail.clone().unwrap_or_default(), 0));
                    }
                }
            },
            // One editor, two doors: the pane hands the terminal to
            // `war roadmap edit`, which ends in one signature.
            Pane::Roadmap => {
                run_child_inherit(terminal, &m.root, &["roadmap", "edit"])?;
                m.refresh();
            }
            Pane::Observe => {
                if m.observe_col == 1
                    && let Some(node) = m.dag_nodes.get(m.observe_dag_sel)
                {
                    m.popup = Some((
                        format!("Task Node: {}", node.id),
                        format!(
                            "ID: {}\nTitle: {}\nState: {}\nCritical: {}\nClaimed by: {}\nHeartbeat: {}\nLease: {}\nScope: {}\nCollision: {}",
                            node.id,
                            node.title,
                            node.state,
                            node.is_critical,
                            node.claim_actor.as_deref().unwrap_or("none"),
                            node.claim_age.as_deref().unwrap_or("none"),
                            node.claim_lease.as_deref().unwrap_or("none"),
                            node.claim_scope.as_deref().unwrap_or("none"),
                            node.scope_collision
                        ),
                        0,
                    ));
                } else if m.observe_col == 3
                    && let Some(q) = m.questions_hub.get(m.observe_question_sel)
                {
                    m.popup = Some((
                        format!("Question: {}", q.target),
                        format!(
                            "Target: {}\nBlocking: {}\nQuestion:\n{}\n\nRecommended Answer:\n{}",
                            q.target, q.blocking, q.question, q.recommended
                        ),
                        0,
                    ));
                }
            }
            Pane::Orchestrate => {
                if m.orchestrate_col == 3 {
                    let target = m
                        .rows
                        .get(&(Pane::Queue as u8))
                        .and_then(|v| v.get(m.selected()))
                        .and_then(|r| r.sign_target.clone());
                    if let Some(t) = target {
                        sign_in_child(terminal, m, &t)?;
                        m.refresh();
                    }
                }
            }
            _ => {
                if let Some(t) = m.current().and_then(|r| r.sign_target.clone()) {
                    sign_in_child(terminal, m, &t)?;
                    m.refresh();
                } else if let Some(r) = m.current() {
                    m.popup = Some((
                        r.command.clone(),
                        r.detail.clone().unwrap_or_else(|| r.command.clone()),
                        0,
                    ));
                }
            }
        },
        _ => {}
    }
    Ok(Flow::Continue)
}

/// `war sign <target> --ssh-sign` in a child that owns the terminal: the
/// app steps out of the alternate screen and raw mode, waits, steps back.
fn sign_in_child(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    m: &mut Model,
    target: &str,
) -> Result<(), RepoError> {
    run_child_inherit(terminal, &m.root, &["sign", target, "--ssh-sign"])?;
    m.status = format!("returned from war sign {target}");
    Ok(())
}

fn run_child_inherit(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    root: &Utf8Path,
    args: &[&str],
) -> Result<(), RepoError> {
    term::leave_quietly();
    let exe = std::env::current_exe().map_err(io("could not find this executable"))?;
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "$ war {}", args.join(" "));
    let status = std::process::Command::new(exe)
        .arg("--root")
        .arg(root.as_str())
        .args(args)
        .current_dir(root)
        .status();
    let _ = writeln!(
        stdout,
        "\n[war {} exited {}] — press Enter to return",
        args.join(" "),
        status.as_ref().map_or(-1, |s| s.code().unwrap_or(-1))
    );
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
    let _ = term::Guard::enter().map(std::mem::forget);
    terminal
        .clear()
        .map_err(io("could not clear the terminal"))?;
    Ok(())
}

/// `~/x` → `$HOME/x`: a directory typed at the prompt reads as a shell would.
fn expand_home(s: &str) -> String {
    match (s.strip_prefix("~/"), std::env::var("HOME")) {
        (Some(rest), Ok(home)) => format!("{home}/{rest}"),
        _ if s == "~" => std::env::var("HOME").unwrap_or_else(|_| s.to_owned()),
        _ => s.to_owned(),
    }
}

/// A child whose output the app shows in a popup: `--show`, an auto remedy.
fn run_child_captured(root: &Utf8Path, args: &[&str]) -> String {
    let Ok(exe) = std::env::current_exe() else {
        return "could not find this executable".to_owned();
    };
    match std::process::Command::new(exe)
        .arg("--root")
        .arg(root.as_str())
        .args(args)
        .current_dir(root)
        .output()
    {
        Ok(out) => {
            let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
            let err = String::from_utf8_lossy(&out.stderr);
            if !err.trim().is_empty() {
                s.push_str("\n--- stderr ---\n");
                s.push_str(&err);
            }
            s.push_str(&format!("\n[exit {}]", out.status.code().unwrap_or(-1)));
            s
        }
        Err(e) => format!("could not run war {}: {e}", args.join(" ")),
    }
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

fn draw(f: &mut ratatui::Frame, m: &Model) {
    let area = f.area();
    let [tabs_area, body, cmd_line, status_line] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);

    let titles: Vec<Line> = Pane::ALL.iter().map(|p| Line::from(p.title())).collect();
    let selected = Pane::ALL.iter().position(|p| *p == m.pane).unwrap_or(0);
    f.render_widget(
        Tabs::new(titles)
            .select(selected)
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL).title(" war ")),
        tabs_area,
    );

    if m.pane == Pane::Observe {
        draw_observe(f, m, body);
    } else if m.pane == Pane::Orchestrate {
        draw_orchestrate(f, m, body);
    } else if m.pane == Pane::Help && m.reading {
        draw_doc(f, m, body);
    } else {
        draw_list(f, m, body);
    }

    let command = m.current().map_or_else(String::new, |r| r.command.clone());
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  $ ", Style::default().fg(Color::DarkGray)),
            Span::raw(command),
        ])),
        cmd_line,
    );
    let status = if let Some(buf) = &m.new_dir {
        format!("  new project directory (Enter runs war init there, Esc cancels): {buf}_")
    } else if m.filtering {
        format!("  / {}_", m.filter)
    } else if !m.filter.is_empty() {
        format!("  {}   filter: {}   (Esc clears)", m.status, m.filter)
    } else {
        format!("  {}   ? keys · q quit", m.status)
    };
    f.render_widget(
        Paragraph::new(status).style(Style::default().fg(Color::Black).bg(Color::Gray)),
        status_line,
    );

    if let Some((title, text, scroll)) = &m.popup {
        let area = centered(area, 90, 85);
        f.render_widget(Clear, area);
        f.render_widget(
            Paragraph::new(md::render(text))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" {title}  (j/k scroll · Esc) ")),
                )
                .wrap(Wrap { trim: false })
                .scroll((*scroll, 0)),
            area,
        );
    }
    if m.show_keys {
        let area = centered(area, 70, 70);
        f.render_widget(Clear, area);
        f.render_widget(
            Paragraph::new(KEYS)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" keys (any key closes) "),
                )
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}

fn draw_list(f: &mut ratatui::Frame, m: &Model, body: Rect) {
    let rows = m.visible();
    let items: Vec<ListItem> = rows
        .iter()
        .map(|(i, r)| {
            let mark = if m.pane == Pane::Queue {
                if m.checked.contains(i) {
                    "[x] "
                } else {
                    "[ ] "
                }
            } else {
                ""
            };
            let style = if r.text.starts_with("ERROR") || r.text.starts_with("UNREADABLE") {
                Style::default().fg(Color::Red)
            } else if r.text.starts_with("WARN") || r.text.starts_with("BLOCKING") {
                Style::default().fg(Color::Yellow)
            } else if r.text.starts_with("HUMAN") {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::raw(mark),
                Span::styled(r.text.clone(), style),
            ]))
        })
        .collect();
    let title = match m.pane {
        Pane::Observe => " Observe — situational HUD & mission control ".to_owned(),
        Pane::Orchestrate => " Orchestrate — draft, dispatch & batch sign ".to_owned(),
        Pane::Setup => format!(" Setup — {} ", m.setup.step.title()),
        Pane::Help => " Help — what next (d: documents, [ ]: step through them) ".to_owned(),
        Pane::Queue => format!(
            " Queue — {} act(s); space checks, s signs checked, Enter signs row, v shows request ",
            m.pending_count
        ),
        Pane::Questions => {
            " Questions — blocking first; Enter shows, the command answers ".to_owned()
        }
        Pane::Frontier => " Frontier — stages by state; the command starts one ".to_owned(),
        Pane::Corpus => " Corpus — rungs; Enter shows the thirteen ".to_owned(),
        Pane::Obligations => " Obligations — disposition, verifier, admissibility ".to_owned(),
        Pane::Evidence => " Evidence — gate runs by class; x records again ".to_owned(),
        Pane::Journal => " Journal — newest first; / filters by day or kind ".to_owned(),
        Pane::Roadmap => {
            " Roadmap — phases in order; Enter edits by keystroke and signs once ".to_owned()
        }
        Pane::Projects => {
            " Projects — every repository you use; Enter opens, n starts a new one ".to_owned()
        }
        Pane::Tickets => {
            " Tickets — state and progress, remaining items; Enter shows checklist and notes "
                .to_owned()
        }
    };
    let mut state = ListState::default();
    if !rows.is_empty() {
        state.select(Some(m.selected().min(rows.len() - 1)));
    }
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, body, &mut state);
    if rows.is_empty() {
        let inner = Rect {
            x: body.x + 2,
            y: body.y + 1,
            width: body.width.saturating_sub(4),
            height: 1,
        };
        f.render_widget(
            Paragraph::new(if m.filter.is_empty() {
                "nothing here — not established, not blank"
            } else {
                "nothing matches the filter"
            })
            .style(Style::default().fg(Color::DarkGray)),
            inner,
        );
    }
}

fn draw_doc(f: &mut ratatui::Frame, m: &Model, body: Rect) {
    let Some(doc) = m.docs.get(m.doc_index) else {
        return;
    };
    f.render_widget(
        Paragraph::new(md::render(&doc.text))
            .block(Block::default().borders(Borders::ALL).title(format!(
                " {} ({}/{})  [ ] other documents · d back to what-next ",
                doc.name,
                m.doc_index + 1,
                m.docs.len()
            )))
            .wrap(Wrap { trim: false }),
        body,
    );
}

fn draw_observe(f: &mut ratatui::Frame, m: &Model, body: Rect) {
    let [vitals_bar, main_area, questions_drawer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(8),
    ])
    .areas(body);

    // 1. Top-Line Telemetry & Fleet Vitals
    let [metrics_area, gauge_area] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)])
            .areas(vitals_bar);

    let metrics_text = Line::from(vec![
        Span::styled(" BURN: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}k tok", m.total_tokens / 1000),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" (${:.2})", m.total_cost_usd),
            Style::default().fg(Color::Green),
        ),
        Span::styled("  │  FLEET: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} active", m.active_agents_count),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" ({} wt)", m.active_worktrees_count),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled("  │  LEASES: ", Style::default().fg(Color::DarkGray)),
        Span::styled("fresh", Style::default().fg(Color::Green)),
        Span::styled(
            if m.herdr_connected {
                "  [herdr: connected]"
            } else {
                "  [herdr: idle]"
            },
            Style::default().fg(if m.herdr_connected {
                Color::Green
            } else {
                Color::DarkGray
            }),
        ),
    ]);

    f.render_widget(
        Paragraph::new(metrics_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Telemetry & Fleet Vitals "),
        ),
        metrics_area,
    );

    let pct = m.global_progress_pct.min(100);
    f.render_widget(
        Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Program Milestone Progress "),
            )
            .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
            .percent(pct)
            .label(format!("{pct}% complete")),
        gauge_area,
    );

    // 2. Three Columns: Left: Milestones, Center: DAG, Right: Herdr Transcript
    let [left_col, center_col, right_col] = Layout::horizontal([
        Constraint::Percentage(28),
        Constraint::Percentage(42),
        Constraint::Percentage(30),
    ])
    .areas(main_area);

    // Left: Milestones & Tasks
    let ms_border_style = if m.observe_col == 0 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let mut task_items = Vec::new();
    for ms in &m.milestones {
        task_items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("▼ {}", ms.name),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({}/{})", ms.done_count, ms.total_count),
                Style::default().fg(Color::DarkGray),
            ),
        ])));
        for t in &ms.tasks {
            let (badge, style) = match t.state.as_str() {
                "done" => ("[DONE] ", Style::default().fg(Color::Green)),
                "active" => (
                    "[ACTV] ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                "ready" => ("[REDY] ", Style::default().fg(Color::White)),
                _ => ("[BLCK] ", Style::default().fg(Color::DarkGray)),
            };
            task_items.push(ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(badge, style),
                Span::styled(format!("{:<8} ", t.id), Style::default().fg(Color::Yellow)),
                Span::styled(t.title.clone(), Style::default()),
                Span::styled(
                    format!(" · {:.1}k tok", t.tokens as f64 / 1000.0),
                    Style::default().fg(Color::DarkGray),
                ),
            ])));
        }
    }
    if task_items.is_empty() {
        task_items.push(ListItem::new(Line::from("No milestones recorded")));
    }
    let mut ms_state = ListState::default();
    if !task_items.is_empty() {
        ms_state.select(Some(m.observe_milestone_sel.min(task_items.len() - 1)));
    }
    f.render_stateful_widget(
        List::new(task_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Milestones & Tasks ")
                    .border_style(ms_border_style),
            )
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White)),
        left_col,
        &mut ms_state,
    );

    // Center: Task Dependency Tree / DAG
    let dag_border_style = if m.observe_col == 1 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let mut dag_items = Vec::new();
    for node in &m.dag_nodes {
        let (badge, style) = match node.state.as_str() {
            "done" => ("[DONE] ", Style::default().fg(Color::Green)),
            "active" => (
                "[ACTV] ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            "ready" => ("[REDY] ", Style::default().fg(Color::White)),
            _ => ("[BLCK] ", Style::default().fg(Color::DarkGray)),
        };

        let mut spans = vec![
            Span::styled(node.prefix.clone(), Style::default().fg(Color::DarkGray)),
            Span::styled(badge, style),
            Span::styled(
                format!("{:<10} ", node.id),
                if node.is_critical {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            ),
            Span::styled(node.title.clone(), Style::default()),
        ];

        if let Some(actor) = &node.claim_actor {
            spans.push(Span::styled(
                format!(" [{actor}]"),
                Style::default().fg(Color::Cyan),
            ));
        }
        if let Some(hb) = &node.claim_age {
            spans.push(Span::styled(
                format!(" {hb}"),
                Style::default().fg(Color::DarkGray),
            ));
        }
        if let Some(lease) = &node.claim_lease {
            spans.push(Span::styled(
                format!(" · {lease}"),
                Style::default().fg(Color::Green),
            ));
        }
        if let Some(scope) = &node.claim_scope {
            spans.push(Span::styled(
                format!(" [{scope}]"),
                Style::default().fg(Color::DarkGray),
            ));
        }
        if node.scope_collision {
            spans.push(Span::styled(
                " [SCOPE COLLISION]",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ));
        }

        dag_items.push(ListItem::new(Line::from(spans)));
    }
    if dag_items.is_empty() {
        dag_items.push(ListItem::new(Line::from("No dependency nodes")));
    }
    let mut dag_state = ListState::default();
    if !dag_items.is_empty() {
        dag_state.select(Some(m.observe_dag_sel.min(dag_items.len() - 1)));
    }
    f.render_stateful_widget(
        List::new(dag_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Task Dependency Tree / DAG ([d]: dispatch) ")
                    .border_style(dag_border_style),
            )
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White)),
        center_col,
        &mut dag_state,
    );

    // Right: Live Herdr Agent Transcript
    let herdr_border_style = if m.observe_col == 2 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let agent_title = if !m.herdr_agents.is_empty() {
        let idx = m.observe_agent_sel.min(m.herdr_agents.len() - 1);
        let a = &m.herdr_agents[idx];
        format!(
            " Live Stream: {} [{}/{}] ([ ]: cycle) ",
            a.display_label(),
            idx + 1,
            m.herdr_agents.len()
        )
    } else if m.herdr_connected {
        " Live Stream: (connected, no active agent) ".to_owned()
    } else {
        " Live Stream: Herdr idle (working tree status) ".to_owned()
    };
    f.render_widget(
        Paragraph::new(m.herdr_transcript.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(agent_title)
                    .border_style(herdr_border_style),
            )
            .wrap(Wrap { trim: false }),
        right_col,
    );

    // 3. Bottom Drawer: Unified Questions Hub
    let q_border_style = if m.observe_col == 3 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let mut q_items = Vec::new();
    for q in &m.questions_hub {
        let mark = if q.blocking {
            Span::styled(
                "BLOCKING ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled("QUESTION ", Style::default().fg(Color::Yellow))
        };
        let rec = if q.recommended.is_empty() {
            String::new()
        } else {
            format!("  → {}", q.recommended)
        };
        q_items.push(ListItem::new(Line::from(vec![
            mark,
            Span::styled(format!("{}: ", q.target), Style::default().fg(Color::Cyan)),
            Span::raw(q.question.clone()),
            Span::styled(rec, Style::default().fg(Color::Green)),
        ])));
    }
    if q_items.is_empty() {
        q_items.push(ListItem::new(Line::from(vec![Span::styled(
            "No blocking questions across active worktrees",
            Style::default().fg(Color::DarkGray),
        )])));
    }
    let mut q_state = ListState::default();
    if !q_items.is_empty() {
        q_state.select(Some(m.observe_question_sel.min(q_items.len() - 1)));
    }
    f.render_stateful_widget(
        List::new(q_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Unified Questions Hub ([a]: answer inline) ")
                    .border_style(q_border_style),
            )
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White)),
        questions_drawer,
        &mut q_state,
    );
}

fn draw_orchestrate(f: &mut ratatui::Frame, m: &Model, body: Rect) {
    let [top_row, bottom_row] =
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(body);

    let [draft_area, dispatch_area] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(top_row);

    let [questions_area, queue_area] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(bottom_row);

    // 1. Draft & Create Work
    let draft_border_style = if m.orchestrate_col == 0 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let draft_lines = vec![
        Line::from(vec![
            Span::styled("Title: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &m.orchestrate_draft_title,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Checklist Items: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &m.orchestrate_draft_items,
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Scaffolding: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "Markdown Warrant/Ticket with deterministic obligations & gates",
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Action: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "Press [c] to Draft & Create Task in Shell",
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    f.render_widget(
        Paragraph::new(draft_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 1. Draft & Create Work ([c]: create) ")
                .border_style(draft_border_style),
        ),
        draft_area,
    );

    // 2. Dispatch & Worktree Provisioning
    let dispatch_border_style = if m.orchestrate_col == 1 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let ready_nodes: Vec<&DagNodeItem> =
        m.dag_nodes.iter().filter(|n| n.state == "ready").collect();
    let sel_task = if !ready_nodes.is_empty() {
        let idx = m.orchestrate_dispatch_task_idx.min(ready_nodes.len() - 1);
        format!("{} — {}", ready_nodes[idx].id, ready_nodes[idx].title)
    } else {
        "No ready tasks unblocked".to_owned()
    };
    let model = DISPATCH_MODELS[m.orchestrate_model_idx % DISPATCH_MODELS.len()];
    let dispatch_lines = vec![
        Line::from(vec![
            Span::styled("Target Node: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                sel_task,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Performer Model: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("< {model} >  ([m]: cycle)"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Write Scope: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &m.orchestrate_scope_input,
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Action: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "Press [d] to Spawn Worktree & Dispatch Agent",
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    f.render_widget(
        Paragraph::new(dispatch_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 2. Dispatch & Worktree Provisioning ([d]: launch) ")
                .border_style(dispatch_border_style),
        ),
        dispatch_area,
    );

    // 3. Blocker & Question Resolution
    let q_border_style = if m.orchestrate_col == 2 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let mut q_items = Vec::new();
    for q in &m.questions_hub {
        q_items.push(ListItem::new(Line::from(vec![
            Span::styled(format!("{}: ", q.target), Style::default().fg(Color::Cyan)),
            Span::raw(q.question.clone()),
            Span::styled(
                format!("  → {}", q.recommended),
                Style::default().fg(Color::Green),
            ),
        ])));
    }
    if q_items.is_empty() {
        q_items.push(ListItem::new(Line::from("No open questions")));
    }
    let mut q_state = ListState::default();
    if !q_items.is_empty() {
        q_state.select(Some(m.observe_question_sel.min(q_items.len() - 1)));
    }
    f.render_stateful_widget(
        List::new(q_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" 3. Blocker & Question Resolution ([a]: answer) ")
                    .border_style(q_border_style),
            )
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White)),
        questions_area,
        &mut q_state,
    );

    // 4. Approval & Batch Signing Queue
    let queue_border_style = if m.orchestrate_col == 3 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let queue_rows = m
        .rows
        .get(&(Pane::Queue as u8))
        .map_or(&[][..], |v| v.as_slice());
    let queue_items: Vec<ListItem> = queue_rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mark = if m.checked.contains(&i) {
                "[x] "
            } else {
                "[ ] "
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    mark,
                    if m.checked.contains(&i) {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
                Span::raw(r.text.clone()),
            ]))
        })
        .collect();
    let mut queue_state = ListState::default();
    if !queue_items.is_empty() {
        queue_state.select(Some(m.selected().min(queue_items.len() - 1)));
    }
    f.render_stateful_widget(
        List::new(queue_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(
                        " 4. Approval & Signing Queue ({} acts) ([s]: batch sign) ",
                        m.pending_count
                    ))
                    .border_style(queue_border_style),
            )
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White)),
        queue_area,
        &mut queue_state,
    );
}

/// The documents embedded at build time, in the order Help shows them after
/// this repository's own: what `war init` ships, so help never drifts from it.
fn embedded_docs(ns: &str) -> Vec<Doc> {
    [
        (
            "docs/TUI.md",
            include_str!("../../../../docs/TUI.md").to_owned(),
        ),
        (
            "QUICKSTART.md",
            include_str!("../../../../QUICKSTART.md").to_owned(),
        ),
        (
            "README.md",
            include_str!("../../../../README.md").to_owned(),
        ),
        (
            "docs/DEFINITIONS.md",
            include_str!("../../../../docs/DEFINITIONS.md").to_owned(),
        ),
        (
            "docs/SKILLS.md",
            include_str!("../../../../docs/SKILLS.md").to_owned(),
        ),
        (
            "AGENTS.md (as `war init` ships it)",
            crate::init::render_agents_md(ns),
        ),
    ]
    .into_iter()
    .map(|(name, text)| Doc {
        name: name.to_owned(),
        text,
    })
    .collect()
}

fn centered(area: Rect, pct_w: u16, pct_h: u16) -> Rect {
    let w = area.width * pct_w / 100;
    let h = area.height * pct_h / 100;
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

/// The one table of keys, rendered by `?` and tested against `docs/TUI.md`.
/// The trees the app polls: the records `war view watch` fingerprints, and the
/// ticket store and its claims (t-67ed), so a `war done` in another terminal
/// shows up without a keypress.
fn watched(repo: &Repository) -> Vec<Utf8PathBuf> {
    let mut dirs = crate::watch::watched_dirs(repo);
    dirs.extend(crate::ticket::watched(repo));
    dirs
}

/// The Tickets pane from `ticket::board`: one row per ticket (state,
/// progress, priority, title; Enter shows `war view show`'s checklist and notes),
/// then one row per REMAINING item beneath it. Done items are in the detail,
/// not the list. No row carries a signature or an auto remedy.
fn ticket_rows(board: &serde_json::Value) -> Vec<Row> {
    let mut rows = Vec::new();
    let list = board["tickets"].as_array().cloned().unwrap_or_default();
    if list.is_empty() {
        rows.push(Row {
            text: "no tickets yet — `war create \"what this work accomplishes\"`".to_owned(),
            command: "war create \"...\" --item \"...\"".to_owned(),
            sign_target: None,
            auto: None,
            detail: None,
        });
    }
    for t in &list {
        let row = &t["ticket"];
        let id = row["id"].as_str().unwrap_or("?");
        let holders: Vec<&str> = row["claims"]
            .as_array()
            .map(|cs| cs.iter().filter_map(|c| c["actor"].as_str()).collect())
            .unwrap_or_default();
        let detail = t["markdown"].as_str().map(str::to_owned);
        rows.push(Row {
            text: format!(
                "{:<11} {} {:>5}  p{}  {}{}",
                row["state"].as_str().unwrap_or("?").replace('_', " "),
                id,
                format!(
                    "{}/{}",
                    row["done"].as_u64().unwrap_or(0),
                    row["total"].as_u64().unwrap_or(0)
                ),
                row["priority"].as_u64().unwrap_or(0),
                row["title"].as_str().unwrap_or(""),
                if holders.is_empty() {
                    String::new()
                } else {
                    format!("  [claimed: {}]", holders.join(", "))
                }
            ),
            command: format!("war view show {id}"),
            sign_target: None,
            auto: None,
            detail: detail.clone(),
        });
        for item in t["items"].as_array().cloned().unwrap_or_default() {
            if item["done"].as_bool().unwrap_or(false) {
                continue;
            }
            let target = item["target"].as_str().unwrap_or(id).to_owned();
            let ready = item["ready"].as_bool().unwrap_or(false);
            rows.push(Row {
                text: format!(
                    "    [ ] {}{}{}",
                    item["text"].as_str().unwrap_or(""),
                    item["id"]
                        .as_str()
                        .map(|i| format!(" ({i})"))
                        .unwrap_or_default(),
                    if ready { "  ready" } else { "" }
                ),
                command: if ready {
                    format!("war admin claim {target}")
                } else {
                    format!("war view show {id}")
                },
                sign_target: None,
                auto: None,
                detail: detail.clone(),
            });
        }
    }
    rows
}

pub const KEYS: &str = "\
o                       observe: situational HUD, DAG, live agent transcripts, questions
O                       orchestrate: draft work, dispatch agents, batch sign queue
0-9 / Tab / Shift-Tab   panes
p                       projects: every repository you use (Enter opens one, n starts one)
t                       tickets: state, progress, remaining items (Enter: checklist and notes)
j k                     move (in a document: next/previous document)
/                       filter this pane (Esc clears)
Enter                   act on the row: sign (queue, help), open the detail (others), run `war init` (setup)
v                       show the request behind a signing row (`war sign <target> --show`)
space  a  n             queue: check row / check all / check none
s                       queue: sign the checked acts — one, or a batch in one dialog
x                       run the row's auto remedy (never a signing act)
c                       show the commit message `war admin commit --write` would use
d  [  ]                 help: documents on/off, previous/next document
r                       re-read the tree now (it also refreshes on its own)
?                       this table
q / Ctrl-C              quit
";

// ---------------------------------------------------------------------------
// A little markdown, enough for the documents this repository ships
// ---------------------------------------------------------------------------

mod md {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span, Text};

    /// Headings bold, bullets kept, fenced code dimmed, inline code styled,
    /// tables passed through as monospace, links reduced to their text.
    pub fn render(text: &str) -> Text<'static> {
        let mut lines = Vec::new();
        let mut in_code = false;
        for raw in text.lines() {
            if raw.trim_start().starts_with("```") {
                in_code = !in_code;
                continue;
            }
            if in_code {
                lines.push(Line::from(Span::styled(
                    format!("    {raw}"),
                    Style::default().fg(Color::Cyan),
                )));
                continue;
            }
            if let Some(h) = raw.strip_prefix('#') {
                let title = h.trim_start_matches('#').trim();
                lines.push(Line::from(Span::styled(
                    title.to_owned(),
                    Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                )));
                continue;
            }
            lines.push(inline(raw));
        }
        Text::from(lines)
    }

    /// `code` spans and `[text](url)` links, on one line.
    fn inline(raw: &str) -> Line<'static> {
        let unlinked = strip_links(raw);
        let mut spans = Vec::new();
        let mut rest = unlinked.as_str();
        while let Some(start) = rest.find('`') {
            let (before, after) = rest.split_at(start);
            spans.push(Span::raw(before.to_owned()));
            let after = &after[1..];
            let Some(end) = after.find('`') else {
                spans.push(Span::raw(format!("`{after}")));
                rest = "";
                break;
            };
            spans.push(Span::styled(
                after[..end].to_owned(),
                Style::default().fg(Color::Cyan),
            ));
            rest = &after[end + 1..];
        }
        if !rest.is_empty() {
            spans.push(Span::raw(rest.to_owned()));
        }
        Line::from(spans)
    }

    fn strip_links(raw: &str) -> String {
        let mut out = String::new();
        let mut rest = raw;
        while let Some(start) = rest.find('[') {
            let Some(mid) = rest[start..].find("](") else {
                break;
            };
            let Some(end) = rest[start + mid..].find(')') else {
                break;
            };
            out.push_str(&rest[..start]);
            out.push_str(&rest[start + 1..start + mid]);
            rest = &rest[start + mid + end + 1..];
        }
        out.push_str(rest);
        out
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn links_become_their_text_and_code_is_styled() {
            assert_eq!(
                strip_links("see [the docs](docs/x.md) now"),
                "see the docs now"
            );
            let t = render("# Title\n\n- a `war check` bullet\n```\ncode\n```\n[l](u)");
            assert_eq!(t.lines.len(), 5);
            assert_eq!(t.lines[0].spans[0].content, "Title");
            assert!(t.lines[2].spans.iter().any(|s| s.content == "war check"));
            assert_eq!(t.lines[3].spans[0].content, "    code");
            assert_eq!(t.lines[4].spans[0].content, "l");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `?` table and `docs/TUI.md` describe the same keys: every key the
    /// table names appears in the document.
    #[test]
    fn every_key_in_the_table_is_documented() {
        let doc = include_str!("../../../../docs/TUI.md");
        for line in KEYS.lines() {
            let key = line.split_whitespace().next().unwrap_or("");
            if key.is_empty() {
                continue;
            }
            let needle = format!("`{key}`");
            assert!(
                doc.contains(&needle),
                "docs/TUI.md does not mention {needle}"
            );
        }
    }

    /// OW-WAR-0112 OBL-006: every embedded document renders in the Help pane
    /// without a panic — through `md::render` and the same Paragraph
    /// `draw_doc` builds, into a normal and a cramped terminal, scrolled to
    /// its end. Refusal side: the renderer is seen to have produced text, so
    /// an empty rendering does not pass as "no panic".
    #[test]
    fn every_embedded_document_renders_in_the_help_pane() {
        use ratatui::backend::TestBackend;
        let docs = embedded_docs("OW");
        assert_eq!(docs.len(), 6, "the six documents the work order names");
        for doc in &docs {
            let text = md::render(&doc.text);
            assert!(
                text.lines.len() > 3,
                "{} rendered to {} line(s)",
                doc.name,
                text.lines.len()
            );
            for (w, h) in [(100, 30), (12, 4)] {
                let mut terminal = Terminal::new(TestBackend::new(w, h)).expect("test backend");
                let lines = u16::try_from(text.lines.len()).unwrap_or(u16::MAX);
                terminal
                    .draw(|f| {
                        f.render_widget(
                            Paragraph::new(text.clone())
                                .block(
                                    Block::default()
                                        .borders(Borders::ALL)
                                        .title(format!(" {} ", doc.name)),
                                )
                                .wrap(Wrap { trim: false })
                                .scroll((lines, 0)),
                            f.area(),
                        );
                    })
                    .unwrap_or_else(|e| panic!("{} did not draw at {w}x{h}: {e}", doc.name));
            }
        }
    }

    /// The panes are the nine the document lists, in order.
    #[test]
    fn the_panes_are_documented_in_order() {
        let doc = include_str!("../../../../docs/TUI.md");
        let mut last = 0;
        for p in Pane::ALL {
            let name = p.title().split_once(' ').map_or(p.title(), |(_, n)| n);
            let at = doc
                .find(&format!("## {name}"))
                .unwrap_or_else(|| panic!("no `## {name}` in docs/TUI.md"));
            assert!(at > last, "{name} is out of order");
            last = at;
        }
    }

    /// t-67ed: the Tickets pane shows a ticket's state and progress and its
    /// REMAINING items; a done item is left to the detail. Refusal side: no
    /// row is a signature or an auto remedy — the pane is read-only.
    #[test]
    fn the_tickets_pane_renders_remaining_items_and_signs_nothing() {
        let board = serde_json::json!({"tickets": [{
            "ticket": {"id": "t-0001", "title": "Password reset", "state": "in_progress",
                       "done": 1, "total": 3, "priority": 1,
                       "claims": [{"actor": "claude"}]},
            "whole_ready": false,
            "items": [
                {"id": "i-0001", "text": "Reset endpoint", "done": true, "target": "t-0001/i-0001", "ready": false},
                {"id": "i-0002", "text": "Email template", "done": false, "target": "t-0001/i-0002", "ready": true},
                {"id": "i-0003", "text": "Docs", "done": false, "target": "t-0001/i-0003", "ready": false}
            ],
            "notes": ["2026-09-25 claude: a decision"],
            "markdown": "# t-0001 — Password reset\n\n## Checklist\n\n## Notes\n\n- 2026-09-25 claude: a decision\n"
        }]});
        let rows = ticket_rows(&board);
        let text: Vec<&str> = rows.iter().map(|r| r.text.as_str()).collect();
        assert!(text[0].starts_with("in progress t-0001"), "{text:?}");
        assert!(text[0].contains("1/3") && text[0].contains("[claimed: claude]"));
        assert!(
            text.iter()
                .any(|t| t.contains("Email template (i-0002)  ready"))
        );
        assert!(text.iter().any(|t| t.contains("Docs (i-0003)")));
        assert!(
            !text.iter().any(|t| t.contains("Reset endpoint")),
            "a done item is listed"
        );
        assert_eq!(rows[1].command, "war admin claim t-0001/i-0002");
        assert_eq!(rows[2].command, "war view show t-0001");
        assert!(rows[0].detail.as_deref().unwrap().contains("## Notes"));
        for r in &rows {
            assert!(r.sign_target.is_none() && r.auto.is_none(), "{r:?}");
            assert!(!r.command.starts_with("war sign"), "{r:?}");
        }
        // No tickets: the pane says how to make one, and still signs nothing.
        let empty = ticket_rows(&serde_json::json!({"tickets": []}));
        assert_eq!(empty.len(), 1);
        assert!(empty[0].text.starts_with("no tickets yet"));
    }

    /// The row a remedy makes: an `auto` remedy is runnable from `x`, a
    /// human one is a signing target for Enter, and never the other way.
    #[test]
    fn remedy_rows_keep_auto_and_human_apart() {
        let drift = Diagnostic::error(
            "deliverable.digest-drift",
            "docs/warrants/OW-WAR-0002/deliverables.toml".to_owned(),
            "OW-WAR-0002: D-002 records sha256:a for x but the file is now sha256:b",
        );
        let r = remedy_row(&drift, 1);
        assert!(r.auto.is_none(), "{r:?}");
        // M12: `war sign correct …` drafts the request; it is no target for
        // the sign key, whose `war sign <target>` is the signature.
        assert!(r.sign_target.is_none(), "{r:?}");
        let stale = Diagnostic::warn(
            "deliverable.pin-stale",
            "docs/warrants/OW-WAR-0003/deliverables.toml".to_owned(),
            "OW-WAR-0003: D-001 records sha256:a and the file is now sha256:b",
        );
        let r = remedy_row(&stale, 2);
        assert_eq!(
            r.auto.as_deref(),
            Some(
                &[
                    "war",
                    "admin",
                    "pins",
                    "--refresh",
                    "--alias",
                    "OW-WAR-0003"
                ]
                .map(String::from)[..]
            )
        );
        assert!(r.sign_target.is_none());
    }
}
