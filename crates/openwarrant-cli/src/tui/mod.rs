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
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap};

use crate::diagnostic::{Diagnostic, Report, Severity};
use crate::repo::{RepoError, Repository};

/// The refusal when there is no terminal: exit 2, by name, and not one
/// escape code on a pipe.
pub const NO_TTY: &str = "tui.no-tty";

/// `war` / `war tui`. Returns the exit code.
pub fn run(root: Option<Utf8PathBuf>, panic_after_setup: bool) -> Result<u8, RepoError> {
    if !crate::sign::at_a_terminal() {
        let mut report = Report::default();
        report.push(Diagnostic::error(
            NO_TTY,
            "-".to_owned(),
            "the app needs a terminal on stdin and stdout; for a script use `war status \
             --json`, `war console --json`, `war next --json` or `war check --json`",
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
}

impl Pane {
    const ALL: [Pane; 11] = [
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
    ];

    const fn title(self) -> &'static str {
        match self {
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
        }
    }
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
    /// Which project the next idle tick looks at.
    project_cursor: usize,
    /// `n` on Projects: the directory being typed for a new project.
    new_dir: Option<String>,
    /// The Help pane's first row: this binary, and PATH's `war` if it differs.
    binary: Row,
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
                Pane::Queue
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
            project_cursor: 0,
            new_dir: None,
            binary: binary_row(),
            root,
            repo,
        };
        m.docs = m.load_docs();
        m.refresh();
        m
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
        self.fingerprint = crate::watch::fingerprint(&crate::watch::watched_dirs(&repo));

        // The queue: `war sign --list`, through the console's board.
        let board = crate::console::board(&repo).ok();
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
        self.pending_count = queue.len();
        self.checked.retain(|n| *n < queue.len());
        self.rows.insert(Pane::Queue as u8, queue);
        self.rows.insert(Pane::Questions as u8, questions);

        // The frontier.
        let mut frontier = Vec::new();
        if let Ok((_, f)) = crate::frontier::run(&repo, None) {
            for r in &f.rows {
                let cmd = stage_cmd
                    .get(&(r.warrant.clone(), r.stage.clone()))
                    .cloned()
                    .unwrap_or_else(|| format!("war frontier {}", r.warrant));
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
        let status = crate::status::build(&repo).ok();
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
                        c.as_pairs()
                            .iter()
                            .map(|(name, met)| format!("{} {name}", if *met { "✓" } else { "✗" }))
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
                    command: format!("war resolve {} --dry-run", w.alias),
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
                        command: format!("war resolve {} --dry-run", w.alias),
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
                                command: format!("war journal {alias}"),
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
                        command: format!("war journal {alias}"),
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
        match crate::roadmap_cmd::view(&repo) {
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
                        command: "war roadmap edit".to_owned(),
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
                command: "war roadmap".to_owned(),
                sign_target: None,
                auto: None,
                detail: None,
            }),
        }
        self.rows.insert(Pane::Roadmap as u8, roadmap);

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
        let next = crate::next::run(&repo).ok();
        let (doctor, _) = crate::doctor::run(Some(self.root.clone()), None, false);
        let check = crate::check::run(&repo, None, false).ok();
        self.rows.insert(
            Pane::Help as u8,
            self.help_rows(next.as_ref(), Some(&doctor), check.as_ref()),
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

    /// What next: the Setup step if incomplete, then `war next`'s actions
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
            for a in &n.actions {
                rows.push(Row {
                    text: format!(
                        "{:<5} {:<10} {} — {}",
                        match a.actor {
                            crate::next::Actor::Human => "HUMAN",
                            crate::next::Actor::Agent => "agent",
                        },
                        a.action,
                        a.warrant,
                        a.why
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
                    text: format!("nothing to do: {why}"),
                    command: "war next".to_owned(),
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
                text: "nothing pending, nothing red — `war next` agrees".to_owned(),
                command: "war next".to_owned(),
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
        for (name, text) in [
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
                crate::init::render_agents_md(&ns),
            ),
        ] {
            docs.push(Doc {
                name: name.to_owned(),
                text,
            });
        }
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
        self.projects = crate::projects::listed();
        self.project_cursor = 0;
        if self.projects.is_empty() {
            self.status =
                "no projects yet: run any `war` command inside a repository and it is remembered"
                    .to_owned();
        }
        self.render_projects();
        self.selected.insert(Pane::Projects as u8, 0);
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

    fn render_projects(&mut self) {
        let mut rows = Vec::new();
        for r in &self.projects {
            let facts = if r.missing {
                "(missing)".to_owned()
            } else if r.fingerprint.is_none() {
                "reading…".to_owned()
            } else {
                crate::projects::summary(r)
            };
            rows.push(Row {
                text: format!(
                    "{:<26} {}  {facts}",
                    r.program.as_deref().unwrap_or("?"),
                    r.root
                ),
                command: format!("war --root {}", r.root),
                sign_target: None,
                auto: None,
                detail: Some(if r.missing {
                    format!(
                        "{} holds no openwarrant.toml any more.\n\n`war projects --forget {}` removes it from the list.",
                        r.root, r.root
                    )
                } else {
                    format!("{}\n\n{facts}", r.root)
                }),
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

/// This binary's version and path, and a warning when `war` on PATH is a
/// different version — the 2026-09-23 failure, where a command handed over
/// ran an older `war` than the one that drafted it.
fn binary_row() -> Row {
    let here = env!("CARGO_PKG_VERSION");
    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.canonicalize().ok())
        .map_or_else(|| "?".to_owned(), |p| p.display().to_string());
    let on_path = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|d| d.join("war"))
            .find(|p| p.is_file())
    });
    let install = "cargo install --path crates/openwarrant-cli";
    let (text, detail) = match on_path {
        None => (
            format!("INFO    this is war {here} at {exe}; no `war` on PATH"),
            format!(
                "Commands this app hands over say `war`. To put this one on PATH: `{install}`."
            ),
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
            if canonical == exe || version.split_whitespace().last() == Some(here) {
                (
                    format!("INFO    war {here} at {exe} — the `war` on PATH"),
                    format!("`war` on PATH is {canonical}, the same version."),
                )
            } else {
                (
                    format!(
                        "WARN    this is war {here} at {exe}; `war` on PATH is {} at {canonical}",
                        if version.is_empty() {
                            "unknown".to_owned()
                        } else {
                            version.clone()
                        }
                    ),
                    format!(
                        "A command you copy from here and run as `war` runs {canonical} ({version}), not this {here}. \
                         Install this one: `{install}` from its checkout, or run it by path: {exe}."
                    ),
                )
            }
        }
    };
    Row {
        text,
        command: install.to_owned(),
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
        if m.idle_ticks.is_multiple_of(4)
            && let Some(repo) = m.repo.as_ref()
        {
            let fp = crate::watch::fingerprint(&crate::watch::watched_dirs(repo));
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
        (KeyCode::Char(d @ '1'..='9'), _) => {
            m.pane = Pane::ALL[(d as u8 - b'1') as usize];
            m.reading = false;
        }
        (KeyCode::Char('0'), _) => {
            m.pane = Pane::Roadmap;
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
        (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
            if m.reading {
                m.doc_index = (m.doc_index + 1) % m.docs.len().max(1);
            } else if n > 0 {
                m.select((m.selected() + 1).min(n - 1));
            }
        }
        (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
            if m.reading {
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
        (KeyCode::Char('['), _) | (KeyCode::Char(']'), _) if m.pane == Pane::Help => {
            m.reading = true;
            if k.code == KeyCode::Char(']') {
                m.doc_index = (m.doc_index + 1) % m.docs.len().max(1);
            } else {
                m.doc_index = (m.doc_index + m.docs.len().max(1) - 1) % m.docs.len().max(1);
            }
        }
        (KeyCode::Char('d'), _) if m.pane == Pane::Help => m.reading = !m.reading,
        (KeyCode::Char('c'), _) => {
            if let Some(repo) = m.repo.as_ref() {
                let text = crate::commit::message(repo)
                    .unwrap_or_else(|e| format!("could not compose: {e}"));
                m.popup = Some(("commit message (`war commit --write`)".to_owned(), text, 0));
            }
        }
        (KeyCode::Char(' '), _) if m.pane == Pane::Queue => {
            if let Some(i) = m.current_index()
                && !m.checked.remove(&i)
            {
                m.checked.insert(i);
            }
        }
        (KeyCode::Char('a'), _) if m.pane == Pane::Queue => {
            m.checked = (0..m.rows().len()).collect();
        }
        (KeyCode::Char('n'), _) if m.pane == Pane::Queue => m.checked.clear(),
        (KeyCode::Char('s'), _) if m.pane == Pane::Queue => {
            let targets: Vec<String> = m
                .rows()
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

    if m.pane == Pane::Help && m.reading {
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
pub const KEYS: &str = "\
0-9 / Tab / Shift-Tab   panes
p                       projects: every repository you use (Enter opens one, n starts one)
j k                     move (in a document: next/previous document)
/                       filter this pane (Esc clears)
Enter                   act on the row: sign (queue, help), open the detail (others), run `war init` (setup)
v                       show the request behind a signing row (`war sign <target> --show`)
space  a  n             queue: check row / check all / check none
s                       queue: sign the checked acts — one, or a batch in one dialog
x                       run the row's auto remedy (never a signing act)
c                       show the commit message `war commit --write` would use
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
        let stale = Diagnostic::warn(
            "deliverable.pin-stale",
            "docs/warrants/OW-WAR-0003/deliverables.toml".to_owned(),
            "OW-WAR-0003: D-001 records sha256:a and the file is now sha256:b",
        );
        let r = remedy_row(&stale, 2);
        assert_eq!(
            r.auto.as_deref(),
            Some(&["war", "pins", "--refresh", "--alias", "OW-WAR-0003"].map(String::from)[..])
        );
        assert!(r.sign_target.is_none());
    }
}
