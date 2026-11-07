// SPDX-License-Identifier: Apache-2.0
//! What the app shows and where the cursor is. Nothing here reads a file: a
//! [`Snapshot`] arrives whole from `reader.rs`, and the rest is the cursor.

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8PathBuf;

/// The two modes. `Tab` flips, `1` and `2` go direct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Watch,
    Act,
}

/// One list in Records. `3` calls it up over either mode; `[` and `]` step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum View {
    Tickets,
    Frontier,
    Corpus,
    Obligations,
    Evidence,
    Journal,
    Roadmap,
    Setup,
    Help,
}

impl View {
    pub const ALL: [Self; 9] = [
        Self::Tickets,
        Self::Frontier,
        Self::Corpus,
        Self::Obligations,
        Self::Evidence,
        Self::Journal,
        Self::Roadmap,
        Self::Setup,
        Self::Help,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Tickets => "Tickets",
            Self::Frontier => "Frontier",
            Self::Corpus => "Corpus",
            Self::Obligations => "Obligations",
            Self::Evidence => "Evidence",
            Self::Journal => "Journal",
            Self::Roadmap => "Roadmap",
            Self::Setup => "Setup",
            Self::Help => "Help",
        }
    }

    /// The next (`step` 1) or previous (`step` -1) view, wrapping.
    #[must_use]
    pub fn stepped(self, step: isize) -> Self {
        let n = Self::ALL.len().cast_signed();
        let at = Self::ALL.iter().position(|v| *v == self).unwrap_or(0);
        Self::ALL[(at.cast_signed() + step).rem_euclid(n).cast_unsigned()]
    }
}

/// What is laid over the current mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    None,
    Records(View),
    Repos,
}

/// The panels of a mode, each with a cursor of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Panel {
    Work,
    Claims,
    Live,
    Questions,
    Notices,
    Create,
    Dispatch,
    Answer,
    Sign,
}

impl Panel {
    pub const WATCH: [Self; 5] = [
        Self::Work,
        Self::Claims,
        Self::Live,
        Self::Questions,
        Self::Notices,
    ];
    pub const ACT: [Self; 4] = [Self::Create, Self::Dispatch, Self::Answer, Self::Sign];
}

/// One row of a list: what is shown, the command behind it, and what `Enter`,
/// `v` or `x` does with it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Row {
    pub text: String,
    /// The exact command behind the row, for the bottom line.
    pub command: String,
    /// `war sign <target>`: the act `s`/`Enter` runs in a child.
    pub sign_target: Option<String>,
    /// A remedy `x` may run unasked.
    pub auto: Option<Vec<String>>,
    /// Detail `Enter` opens, when the row has more to say.
    pub detail: Option<String>,
}

/// One document Help can show.
#[derive(Debug, Clone)]
pub struct Doc {
    pub name: String,
    pub text: String,
}

/// A count that was recorded, or was not.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Count {
    #[default]
    Unknown,
    Known(u64),
}

/// One open line of the Work panel: an item of any encoding, or one of its
/// remaining checklist items at depth 1.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkRow {
    pub id: String,
    pub title: String,
    /// The record's own word: `open`, `in_progress`, `authorized`, `ready`...
    pub state: String,
    /// `Ticket`, `Warrant`, `Order` or `UNKNOWN`; derived in `work.rs`.
    pub level: String,
    pub depth: u8,
    pub critical: bool,
    pub holder: Option<String>,
    pub tokens: Count,
}

/// A roadmap phase, as counts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PhaseRow {
    pub id: String,
    pub title: String,
    /// `war plan roadmap`'s word for it.
    pub achieved: String,
    pub done: usize,
    pub total: usize,
    /// Members no record answers for.
    pub unknown: usize,
}

/// One live claim, read from its lock file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClaimRow {
    pub target: String,
    /// `None`: a lock that could not be read. It still holds its item.
    pub actor: Option<String>,
    pub since_unix: u64,
    /// When its lease runs out; `None` for a lock from before leases.
    pub lease_until_unix: Option<u64>,
    /// The lock file's modification time: when a `war` command of the holder
    /// last ran. That is all it says; a silent agent can be working, and a
    /// renewed lock does not prove one is.
    pub renewed_unix: Option<u64>,
    pub workspace: Option<String>,
    /// Declared write paths; empty when none were.
    pub scope: Vec<String>,
}

/// Two claims whose declared paths cannot be proved disjoint.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Collision {
    pub a: String,
    pub b: String,
    pub why: String,
}

/// A W3 notice and how far it got.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NoticeRow {
    pub ticket: String,
    pub actor: String,
    pub line: String,
    pub at: String,
    /// `war` printed its line to the holder.
    pub told: bool,
    /// `war start` showed the holder the change.
    pub seen: bool,
}

/// An open question.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestionItem {
    pub warrant: String,
    pub id: String,
    pub question: String,
    pub recommended: String,
    pub blocking: bool,
    pub command: String,
}

impl QuestionItem {
    pub fn target(&self) -> String {
        format!("{}/{}", self.warrant, self.id)
    }
}

/// A node that can start now, in the order a scheduler takes them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReadyRow {
    pub id: String,
    pub title: String,
    pub critical: bool,
}

/// Something that can run a task, from `[go]`. No list is built in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExecutorRow {
    pub name: String,
    /// Why it cannot run one chosen task, when it cannot.
    pub unavailable: Option<String>,
    /// Herdr's `--kind` for it, when Herdr knows the agent.
    pub herdr_kind: Option<String>,
    /// The harness needs a terminal, so a detached start would die.
    pub needs_terminal: bool,
}

/// Progress as counts, each `None` when its source could not be read. Never
/// a percentage: a count is what the records hold, and a percentage of
/// unequal Warrants would be a number nobody recorded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Progress {
    /// Resolved, of all directory and single-file Warrants `status` lists.
    pub warrants: Option<(usize, usize)>,
    /// Achieved, of all roadmap phases.
    pub phases: Option<(usize, usize)>,
    /// Open, in progress and done, over every item of every encoding.
    pub items: Option<(usize, usize, usize)>,
}

/// Everything `reader::read` found, finished, so nothing is half-drawn.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// False until the first read arrives.
    pub loaded: bool,
    pub program: String,
    pub sas_in_force: String,
    pub repository: bool,
    pub setup_done: bool,
    pub setup_step: String,
    pub views: BTreeMap<View, Vec<Row>>,
    pub queue: Vec<Row>,
    pub questions: Vec<QuestionItem>,
    pub phases: Vec<PhaseRow>,
    pub work: Vec<WorkRow>,
    pub ready: Vec<ReadyRow>,
    pub claims: Vec<ClaimRow>,
    pub collisions: Vec<Collision>,
    pub notices: Vec<NoticeRow>,
    pub progress: Progress,
    /// The critical path's nodes, first to start first.
    pub critical_path: Vec<String>,
    /// Its remaining length, as `estimate::human` says it; an estimate.
    pub critical_length: Option<String>,
    /// Every checkout of this clone, for telling which agents work here.
    pub worktrees: Option<Vec<String>>,
    pub usage: super::usage::Usage,
    /// `[go]`'s executors.
    pub executors: Vec<ExecutorRow>,
    /// What `[plan] drafter_argv` runs, when it is set.
    pub drafter: Option<String>,
    /// The one human the register names, for `war plan answer --as`.
    pub human: Option<String>,
    /// Sources that could not be read, each said once.
    pub faults: Vec<String>,
    /// The newest journal events, for the panel without Herdr.
    pub progress_lines: Vec<String>,
}

/// Herdr, seen from here. Its absence is a normal state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Herdr {
    /// Nothing asked yet.
    Unasked,
    Absent {
        why: String,
    },
    Connected {
        version: String,
    },
}

/// What the Live panel shows.
#[derive(Debug, Clone)]
pub struct Live {
    pub herdr: Herdr,
    pub agents: Vec<crate::herdr::HerdrAgent>,
    /// The focused agent's terminal, or `git status` and diff without Herdr.
    pub text: String,
}

impl Default for Live {
    fn default() -> Self {
        Self {
            herdr: Herdr::Unasked,
            agents: Vec::new(),
            text: String::new(),
        }
    }
}

/// How a dispatched task starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launcher {
    /// A Herdr tab and `agent.start`.
    Herdr { kind: String },
    /// `war start --worktree` in this terminal, until it exits.
    Terminal,
    /// `war start --worktree` with no terminal, logging to a file.
    Detached,
}

impl Launcher {
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Herdr { .. } => "a Herdr tab",
            Self::Terminal => "this terminal, until the session ends",
            Self::Detached => "detached, logging to a file",
        }
    }
}

/// A task on its way to an agent: claim, worktree, launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dispatch {
    pub node: String,
    pub actor: String,
    pub paths: Vec<String>,
    pub launcher: Launcher,
}

/// A form over Act. It takes every key until it is done or `Esc`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Form {
    Create {
        title: String,
        /// Separated by `;`.
        items: String,
        /// On the items line, not the title.
        on_items: bool,
        draft: bool,
    },
    Dispatch {
        node: String,
        title: String,
        step: DispatchStep,
        executor: usize,
        scope: String,
        worktree: bool,
        launcher: usize,
    },
    Answer {
        warrant: String,
        id: String,
        question: String,
        recommended: String,
        text: String,
        who: String,
        on_who: bool,
    },
    Note {
        target: String,
        text: String,
        /// A Herdr agent in the holder's workspace, which can be told now.
        pane: Option<String>,
        now: bool,
    },
}

/// Where a dispatch form stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchStep {
    Executor,
    Scope,
    Worktree,
    Launch,
}

/// What a keypress asks of the terminal or the background threads. A key
/// handler never runs anything, so every action is a value a test can read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Quit,
    /// Read the tree again now.
    Reload,
    /// A child that owns the terminal, with the app stepped aside: a
    /// signature, `war init`, the roadmap editor.
    Terminal {
        argv: Vec<String>,
        /// Where it runs, when not this project: `war init` in a new one.
        root: Option<Utf8PathBuf>,
    },
    /// A child off the main thread; its output lands in a popup.
    Capture { title: String, argv: Vec<String> },
    /// A child off the main thread, its output and `after` in a popup, and
    /// `prompt`, a pane and text, sent there when it succeeded.
    Say {
        title: String,
        argv: Vec<String>,
        after: String,
        prompt: Option<(String, String)>,
    },
    /// Claim, worktree, launch.
    Dispatch(Dispatch),
    /// Switch the whole app to another repository.
    Open(Utf8PathBuf),
    /// Make a directory and run `war init` in it.
    NewRepo(Utf8PathBuf),
    /// Read the repository list again.
    ReadRepos,
}

/// The Repos list: who is on it, and each one's facts as they arrive.
#[derive(Debug, Default)]
pub struct Repos {
    pub projects: Vec<crate::projects::Row>,
    pub tiers: Vec<crate::projects::Tier>,
    pub hidden: usize,
    pub rows: Vec<Row>,
}

pub struct Model {
    pub root: Utf8PathBuf,
    pub mode: Mode,
    pub overlay: Overlay,
    /// The Records view `3` calls up.
    pub last_view: View,
    pub snap: Snapshot,
    pub live: Live,
    pub watch_panel: Panel,
    pub act_panel: Panel,
    /// Each panel's and each view's highlighted row.
    pub panel_sel: BTreeMap<Panel, usize>,
    pub view_sel: BTreeMap<View, usize>,
    pub repos_sel: usize,
    /// Which focused agent the Live panel follows.
    pub agent_sel: usize,
    /// How many of `live.agents`, which sorts them first, work in this clone.
    pub agents_here: usize,
    /// Signing targets checked for a batch. Targets, not row numbers: the
    /// queue is re-read behind the cursor, and a row number could then name
    /// a different act from the one the human checked.
    pub checked: BTreeSet<String>,
    pub filter: String,
    pub filtering: bool,
    pub show_keys: bool,
    /// A popup with scrollable text: a request, a detail, a command's output.
    pub popup: Option<(String, String, u16)>,
    pub docs: Vec<Doc>,
    pub doc_index: usize,
    /// In Help: a document (true) or what-next (false).
    pub reading: bool,
    /// The last thing the app has to say; cleared by the next key.
    pub message: String,
    /// The form open over Act.
    pub form: Option<Form>,
    /// `n` in Repos: the directory being typed.
    pub new_dir: Option<String>,
    pub repos: Repos,
    /// Help's first row: this binary, and PATH's `war` if it differs.
    pub binary: Row,
    /// Seconds since the epoch, kept by the event loop so a countdown runs
    /// between snapshots, and fixed in a test.
    pub now_unix: u64,
}

impl Model {
    /// A model with nothing read yet: the app draws at once, and the first
    /// snapshot fills it in.
    #[must_use]
    pub fn new(root: Utf8PathBuf, setup_done: bool, binary: Row, docs: Vec<Doc>) -> Self {
        Self {
            mode: Mode::Watch,
            overlay: if setup_done {
                Overlay::None
            } else {
                Overlay::Records(View::Setup)
            },
            last_view: View::Tickets,
            snap: Snapshot {
                setup_done,
                ..Snapshot::default()
            },
            live: Live::default(),
            watch_panel: Panel::Work,
            act_panel: Panel::Sign,
            panel_sel: BTreeMap::new(),
            view_sel: BTreeMap::new(),
            repos_sel: 0,
            agent_sel: 0,
            agents_here: 0,
            checked: BTreeSet::new(),
            filter: String::new(),
            filtering: false,
            show_keys: false,
            popup: None,
            docs,
            doc_index: 0,
            reading: false,
            message: String::new(),
            form: None,
            new_dir: None,
            repos: Repos::default(),
            binary,
            now_unix: 0,
            root,
        }
    }

    /// Take a finished snapshot. The cursor stays where it was, clamped; a
    /// check on an act that left the queue goes with it.
    pub fn apply(&mut self, snap: Snapshot) {
        let kept: BTreeSet<&String> = snap
            .queue
            .iter()
            .filter_map(|r| r.sign_target.as_ref())
            .collect();
        self.checked.retain(|t| kept.contains(t));
        // Setup is called up first only over a repository with no open work.
        if !self.snap.loaded
            && self.overlay == Overlay::Records(View::Setup)
            && !snap.work.is_empty()
        {
            self.overlay = Overlay::None;
        }
        self.snap = snap;
    }

    /// Take a look at Herdr or the working tree. Herdr lists every agent of
    /// the session; the ones in a checkout of this clone come first, and the
    /// highlighted one stays highlighted.
    pub fn apply_live(&mut self, mut live: Live) {
        let pane = self.focused_agent().map(|a| a.pane_id.clone());
        let here = |dir: &str| {
            let inside = |root: &str| {
                dir == root
                    || dir
                        .strip_prefix(root)
                        .is_some_and(|rest| rest.starts_with('/'))
            };
            inside(self.root.as_str())
                || self
                    .snap
                    .worktrees
                    .iter()
                    .flatten()
                    .any(|w| inside(w.as_str()))
        };
        live.agents.sort_by_key(|a| !here(a.workdir()));
        self.agents_here = live.agents.iter().filter(|a| here(a.workdir())).count();
        if let Some(pane) = pane
            && let Some(at) = live.agents.iter().position(|a| a.pane_id == pane)
        {
            self.agent_sel = at;
        }
        self.live = live;
    }

    /// Whether a row's text passes the `/` filter.
    #[must_use]
    pub fn passes(&self, text: &str) -> bool {
        self.filter.is_empty() || text.to_lowercase().contains(&self.filter.to_lowercase())
    }

    /// The panel the keys act on.
    #[must_use]
    pub const fn panel(&self) -> Panel {
        match self.mode {
            Mode::Watch => self.watch_panel,
            Mode::Act => self.act_panel,
        }
    }

    #[must_use]
    pub fn selected_in(&self, panel: Panel) -> usize {
        self.panel_sel.get(&panel).copied().unwrap_or(0)
    }

    pub fn select_in(&mut self, panel: Panel, at: usize) {
        self.panel_sel.insert(panel, at);
    }

    /// What is shown of the agents, one line each, in the order `[` and `]`
    /// step them.
    #[must_use]
    pub fn focused_agent(&self) -> Option<&crate::herdr::HerdrAgent> {
        self.live
            .agents
            .get(self.agent_sel.min(self.live.agents.len().saturating_sub(1)))
    }

    /// Where the panel without Herdr looks: the workspace of the highlighted
    /// claim, else this checkout.
    #[must_use]
    pub fn live_workdir(&self) -> Utf8PathBuf {
        self.snap
            .claims
            .get(self.selected_in(Panel::Claims))
            .and_then(|c| c.workspace.as_deref())
            .map_or_else(|| self.root.clone(), Utf8PathBuf::from)
    }

    /// The word for the bar at the top.
    #[must_use]
    pub fn status_line(&self) -> String {
        if !self.message.is_empty() {
            return self.message.clone();
        }
        if !self.snap.loaded {
            return format!("{}: reading the records…", self.root);
        }
        if !self.snap.repository {
            return format!(
                "{} is not a repository: p lists your repositories, 3 sets one up here",
                self.root
            );
        }
        let setup = if self.snap.setup_done {
            String::new()
        } else {
            format!(" · setup: {} (3, then Setup)", self.snap.setup_step)
        };
        format!(
            "{} · {} · SAS {} · {} act(s) awaiting a signature{setup}",
            self.snap.program,
            self.root,
            self.snap.sas_in_force,
            self.snap.queue.len()
        )
    }
}

/// `3m 42s`, `1h 03m`, `4s`: as long as it needs to be, no longer.
#[must_use]
pub fn span(secs: u64) -> String {
    match secs {
        0..=59 => format!("{secs}s"),
        60..=3599 => format!("{}m {:02}s", secs / 60, secs % 60),
        3600..=86_399 => format!("{}h {:02}m", secs / 3600, secs / 60 % 60),
        _ => format!("{}d {:02}h", secs / 86_400, secs / 3600 % 24),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_span_says_as_much_as_it_needs() {
        assert_eq!(span(4), "4s");
        assert_eq!(span(222), "3m 42s");
        assert_eq!(span(3780), "1h 03m");
        assert_eq!(span(90_000), "1d 01h");
    }

    #[test]
    fn views_step_and_wrap() {
        assert_eq!(View::Tickets.stepped(1), View::Frontier);
        assert_eq!(View::Tickets.stepped(-1), View::Help);
        assert_eq!(View::Help.stepped(1), View::Tickets);
    }
}
