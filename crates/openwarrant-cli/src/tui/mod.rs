// SPDX-License-Identifier: Apache-2.0
//! `war` with no arguments: Mission Control (OW-WAR-0112, absorbing
//! OW-WAR-0073; t-3c16c W5).
//!
//! A rendering under SAS §76.6 and OW-ADR-0019/0020: it issues commands and
//! holds no key. Every act it starts shells out to the same `war sign
//! --ssh-sign` a hand would run, in a child that inherits the terminal, so
//! the `ssh-add -c` dialog remains the human act. Nothing under `tui/` names
//! `ssh-keygen` or `SSH_AUTH_SOCK`; the battery greps for both.
//!
//! # The shape
//!
//! Two modes, **Watch** and **Act**. `Tab` flips, `1` and `2` go direct, `3`
//! calls up **Records** over either, and `p` calls up **Repos** from anywhere.
//!
//! - `model.rs`: the cursor, and the plain data a read produces.
//! - `reader.rs`: background threads; a read is finished before it is shown.
//! - `herdr_adapter.rs`: Herdr, optional, and the working tree without it.
//! - `watch.rs`, `act.rs`, `records.rs`: each mode's keys and drawing.
//! - `argv.rs`: every command line the app builds.
//!
//! A key handler runs nothing. It returns an [`Effect`], so a test can read
//! every action, and only [`event_loop`] touches the terminal.
//!
//! # One answer per question
//!
//! Every list reads the functions the CLI already answers with. A list that
//! computed a fact itself would be a second answer to a question the records
//! already answer, and the one nobody re-checks. What was not recorded says
//! `UNKNOWN`; it is never estimated into a real-looking number.
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
//! Input is `crossterm::event::poll`; nothing here is async, so OW-ADR-0014's
//! refusal stands untouched. Live refresh is `watch.rs`'s fingerprint poller
//! on a background thread, debounced one look, so a signature given in another
//! terminal shows up without a keypress.

mod act;
mod argv;
mod herdr_adapter;
mod md;
mod model;
mod reader;
mod records;
pub(crate) mod usage;
mod watch;

#[cfg(test)]
mod tests;

use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};
use model::{Doc, Effect, Mode, Model, Overlay, View};
use reader::{Reader, Update};

/// The refusal when there is no terminal: exit 2, by name, and not one
/// escape code on a pipe.
pub const NO_TTY: &str = "tui.no-tty";

/// The refusal for `war --json` and `war view tui --json`: a terminal
/// application is a rendering and has no envelope, so it names the commands
/// that do. Exit 2, as an envelope, like every other refusal.
pub const JSON: &str = "tui.json";

/// A command line the app would run that the real CLI does not accept.
pub const BAD_ARGV: &str = "tui.bad-argv";

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

fn model_for(root: Utf8PathBuf) -> Model {
    let now = crate::gate_cmd::receipt::rfc3339_from_secs(crate::ticket::now_secs());
    let setup = crate::init::guided::Machine::new(crate::init::guided::Facts::read(&root), now);
    let setup_done = setup.step == crate::init::guided::Step::Done;
    let mut m = Model::new(root, setup_done, records::binary_row(), Vec::new());
    m.docs = load_docs(&m.root);
    m.now_unix = crate::ticket::now_secs();
    m
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
    // OW-WAR-0115: `war` where no repository is found opens Repos. Setup for
    // this directory stays one row away.
    let hub = repo.is_none() && root.is_none();
    let root = match (&repo, root) {
        (Some(r), _) => r.root.clone(),
        (None, Some(r)) => r,
        (None, None) => Utf8PathBuf::from_path_buf(
            std::env::current_dir().map_err(|e| RepoError::Message(e.to_string()))?,
        )
        .map_err(|_| RepoError::Message("the current directory is not UTF-8".to_owned()))?,
    };
    let mut model = model_for(root);
    let mut reader = Reader::spawn(model.root.clone(), model.binary.clone());
    if hub {
        model.overlay = Overlay::Repos;
        reader.read_repos();
    }

    let guard = term::Guard::enter()?;
    // The hook runs before the default one prints the panic, so the message
    // lands on a restored screen rather than inside the alternate one. A
    // background thread's panic is caught where it runs and leaves the app
    // drawing, so it neither takes the terminal back nor prints over it.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if std::thread::current()
            .name()
            .is_some_and(|n| n.starts_with(reader::THREAD_PREFIX))
        {
            return;
        }
        term::leave_quietly();
        default_hook(info);
    }));
    if panic_after_setup {
        panic!("--panic-after-setup: the fixture that proves the terminal is restored");
    }
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend).map_err(io("could not open the terminal"))?;
    let result = event_loop(&mut terminal, &mut model, &mut reader);
    drop(guard);
    let _ = std::panic::take_hook();
    result.map(|()| crate::EXIT_OK)
}

/// `war view tui --render <watch|act|records|repos>`: one frame as text, for
/// the battery, which cannot drive a pipe. A read and a draw, with no
/// terminal: the screen a person would see, not a second report.
pub fn render(root: Option<Utf8PathBuf>, what: &str) -> Result<u8, RepoError> {
    let root = match Repository::discover(root.clone()).ok() {
        Some(r) => r.root,
        None => root.ok_or_else(|| RepoError::Message("no repository here".to_owned()))?,
    };
    let mut m = model_for(root);
    m.overlay = Overlay::None;
    match what {
        "watch" => {}
        "act" => m.mode = Mode::Act,
        "records" => m.overlay = Overlay::Records(View::Tickets),
        "usage" => m.overlay = Overlay::Records(View::Usage),
        "repos" => {
            m.overlay = Overlay::Repos;
            let (rows, hidden) = crate::projects::hub();
            records::set_repos(&mut m, rows, hidden);
        }
        other => {
            return Err(RepoError::Message(format!(
                "--render takes watch, act, records, usage or repos, not {other}"
            )));
        }
    }
    let snap = reader::read(&m.root, &m.binary);
    m.apply(snap);
    m.live = herdr_adapter::look(&herdr_adapter::Focus {
        pane: None,
        workdir: m.root.clone(),
    });
    print!("{}", frame(&m, 160, 48));
    Ok(crate::EXIT_OK)
}

/// A frame on this repository may take this long, Warrants and all.
const BUDGET_FRAME_MS: u128 = 50;
/// A key may take this long: it runs nothing, so it is the cursor's cost.
const BUDGET_KEY_MS: u128 = 10;
/// A background read may take this long on a debug build under load.
const BUDGET_READ_MS: u128 = 600_000;

const fn over_budget(read: u128, frame: u128, key: u128) -> bool {
    read > BUDGET_READ_MS || frame > BUDGET_FRAME_MS || key > BUDGET_KEY_MS
}

/// `war view tui --budget`: a read of this repository, then the frames and
/// keys it costs the main thread, against the budget. Never run on the main
/// thread in the app: the read's time is reported so it can be watched, and a
/// keypress does not wait for it.
pub fn budget(root: Option<Utf8PathBuf>) -> Result<u8, RepoError> {
    use std::time::Instant;
    let repo = Repository::discover(root)?;
    let mut m = model_for(repo.root.clone());
    m.overlay = Overlay::None;
    let at = Instant::now();
    let snap = reader::read(&m.root, &m.binary);
    let read = at.elapsed().as_millis();
    let at = Instant::now();
    let _ = crate::watch::fingerprint(&crate::watch::watched_dirs(&repo));
    let fingerprint = at.elapsed().as_millis();
    let warrants = snap.progress.warrants.map_or(0, |(_, n)| n);
    let laps = snap.laps.clone();
    m.apply(snap);
    let mut worst_frame = 0;
    for (mode, overlay) in [
        (Mode::Watch, Overlay::None),
        (Mode::Act, Overlay::None),
        (Mode::Watch, Overlay::Records(View::Journal)),
        (Mode::Watch, Overlay::Records(View::Obligations)),
    ] {
        m.mode = mode;
        m.overlay = overlay;
        // The median of five: one frame's cost, not one scheduler's stall on
        // a loaded machine.
        let mut frames = [0; 5].map(|_: u128| {
            let at = Instant::now();
            let _ = frame(&m, 200, 60);
            at.elapsed().as_millis()
        });
        frames.sort_unstable();
        worst_frame = worst_frame.max(frames[2]);
    }
    m.mode = Mode::Watch;
    m.overlay = Overlay::None;
    let mut worst_key = 0;
    for code in [KeyCode::Char('j'), KeyCode::Char('k'), KeyCode::Char('l')].repeat(40) {
        let at = Instant::now();
        let _ = handle_key(&mut m, KeyEvent::new(code, KeyModifiers::NONE));
        worst_key = worst_key.max(at.elapsed().as_millis());
    }
    println!("{warrants} Warrants");
    println!("read {read} ms (budget {BUDGET_READ_MS}), on a background thread");
    for (what, ms) in &laps {
        println!("  {what}: {ms} ms");
    }
    println!("fingerprint {fingerprint} ms, on a background thread");
    println!("frame {worst_frame} ms, the slowest screen's median (budget {BUDGET_FRAME_MS})");
    println!("key {worst_key} ms at worst (budget {BUDGET_KEY_MS})");
    if over_budget(read, worst_frame, worst_key) {
        println!("OVER BUDGET");
        return Ok(crate::EXIT_NOT_READY);
    }
    Ok(crate::EXIT_OK)
}

/// `m` drawn, as lines of text.
fn frame(m: &Model, width: u16, height: u16) -> String {
    let backend = ratatui::backend::TestBackend::new(width, height);
    let Ok(mut terminal) = Terminal::new(backend);
    let _ = terminal.draw(|f| draw(f, m));
    let buffer = terminal.backend().buffer();
    let mut out = String::new();
    for y in 0..buffer.area.height {
        let line: String = (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect();
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

/// `war view tui --check-argv [FILE]`: every command line the app can build,
/// and every one a read of this repository put on a row, through the real
/// parser. With a file, its lines instead: the battery plants a bad one and
/// watches this refuse it.
pub fn check_argv(root: Option<Utf8PathBuf>, file: Option<&Utf8Path>) -> Result<u8, RepoError> {
    let mut lines = match file {
        Some(path) => argv::read_table(path.as_std_path()).map_err(RepoError::Message)?,
        None => argv::table(),
    };
    if file.is_none()
        && let Ok(repo) = Repository::discover(root)
    {
        let snap = reader::read(&repo.root, &model::Row::default());
        lines.extend(snap_argv(&snap));
    }
    let mut report = Report::default();
    for line in &lines {
        if let Err(why) = argv::parses(line) {
            report.push(Diagnostic::error(
                BAD_ARGV,
                "-".to_owned(),
                format!(
                    "`war {}`: {}",
                    line.join(" "),
                    why.lines().next().unwrap_or("refused")
                ),
            ));
        }
    }
    if report.diagnostics.is_empty() {
        println!("{} command line(s) parse", lines.len());
    }
    crate::check::print(&report);
    Ok(crate::output::exit_code(&report))
}

/// Every argv a snapshot's rows carry: remedies, and displayed commands that
/// are whole.
fn snap_argv(snap: &model::Snapshot) -> Vec<Vec<String>> {
    let rows = snap.views.values().flatten().chain(&snap.queue);
    let mut out = Vec::new();
    for r in rows {
        if let Some(auto) = r
            .auto
            .as_ref()
            .filter(|a| a.first().is_some_and(|w| w == "war"))
        {
            out.push(auto[1..].to_vec());
        }
        out.extend(argv::of_display(&r.command));
    }
    out
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
// The event loop
// ---------------------------------------------------------------------------

/// How long the loop waits for a key before it looks for news. Short, since a
/// finished snapshot should show at once, and waiting costs nothing.
const POLL: Duration = Duration::from_millis(100);

fn take(m: &mut Model, reader: &Reader, update: Update) {
    match update {
        Update::Snapshot(s) => m.apply(*s),
        Update::ReadFailed(why) => m.message = format!("UNKNOWN: the read failed: {why}"),
        Update::Live(l) => m.apply_live(*l),
        Update::RepoList {
            generation,
            rows,
            hidden,
        } if generation == reader.repo_generation => records::set_repos(m, rows, hidden),
        Update::RepoList { .. } => {}
        Update::Repo {
            generation,
            index,
            row,
        } if generation == reader.repo_generation => {
            if let Some(slot) = m.repos.projects.get_mut(index) {
                *slot = *row;
                records::render_repos(m);
            }
        }
        Update::Repo { .. } => {}
        Update::Child { title, text } => {
            m.popup = Some((title, text, 0));
            reader.reload();
        }
    }
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    m: &mut Model,
    reader: &mut Reader,
) -> Result<(), RepoError> {
    loop {
        m.now_unix = crate::ticket::now_secs();
        for update in reader.drain() {
            take(m, reader, update);
        }
        reader.focus(herdr_adapter::Focus {
            pane: m.focused_agent().map(|a| a.pane_id.clone()),
            workdir: m.live_workdir(),
        });
        terminal
            .draw(|f| draw(f, m))
            .map_err(io("could not draw"))?;
        if !event::poll(POLL).map_err(io("could not poll the terminal"))? {
            continue;
        }
        if let Event::Key(k) = event::read().map_err(io("could not read the terminal"))?
            && k.kind == KeyEventKind::Press
            && let Some(effect) = handle_key(m, k)
            && perform(terminal, m, reader, effect)? == Flow::Quit
        {
            return Ok(());
        }
    }
}

#[derive(PartialEq, Eq)]
enum Flow {
    Continue,
    Quit,
}

/// Do what a key asked. The one place an action runs.
fn perform(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    m: &mut Model,
    reader: &mut Reader,
    effect: Effect,
) -> Result<Flow, RepoError> {
    match effect {
        Effect::Quit => return Ok(Flow::Quit),
        Effect::Reload => reader.reload(),
        Effect::Terminal { argv, root } => {
            let at = root.unwrap_or_else(|| m.root.clone());
            let args: Vec<&str> = argv.iter().map(String::as_str).collect();
            let code = run_child_inherit(terminal, &at, &args)?;
            m.message = format!("war {} exited {code}", argv.join(" "));
            reader.reload();
        }
        Effect::Capture { title, argv } => {
            m.message = format!("running war {}…", argv.join(" "));
            reader.capture(m.root.clone(), title, argv);
        }
        Effect::Say {
            title,
            argv,
            at,
            after,
            prompt,
        } => {
            m.message = format!("running war {}…", argv.join(" "));
            let root = at
                .filter(|dir| dir.is_dir())
                .unwrap_or_else(|| m.root.clone());
            reader.say(root, title, argv, after, prompt);
        }
        Effect::Dispatch(plan) => {
            if plan.launcher == model::Launcher::Terminal {
                let argv = argv::start(&plan.node, &plan.actor, &plan.paths, false);
                let args: Vec<&str> = argv.iter().map(String::as_str).collect();
                let code = run_child_inherit(terminal, &m.root, &args)?;
                m.message = format!("war {} exited {code}", argv.join(" "));
                reader.reload();
            } else {
                m.message = format!("dispatching {}…", plan.node);
                reader.dispatch(m.root.clone(), plan);
            }
        }
        Effect::Open(root) => open_repo(m, reader, root),
        Effect::NewRepo(dir) => {
            if let Err(e) = std::fs::create_dir_all(&dir) {
                m.message = format!("could not create {dir}: {e}");
                return Ok(Flow::Continue);
            }
            run_child_inherit(terminal, &dir, &["init"])?;
            open_repo(m, reader, dir);
        }
        Effect::ReadRepos => reader.read_repos(),
    }
    Ok(Flow::Continue)
}

/// Switch the whole app to another repository: a fresh model and fresh
/// readers, as if `war` had been started there. Nothing of this one carries
/// over.
fn open_repo(m: &mut Model, reader: &mut Reader, root: Utf8PathBuf) {
    let repo = Repository::discover(Some(root.clone())).ok();
    if let Some(r) = &repo {
        crate::projects::touch(&r.root);
    }
    let opened = repo.is_some();
    let root = repo.map_or(root, |r| r.root);
    *m = model_for(root);
    if !opened {
        m.overlay = Overlay::Records(View::Setup);
    }
    *reader = Reader::spawn(m.root.clone(), m.binary.clone());
}

/// One key. Runs nothing: what it asks for comes back as an [`Effect`].
fn handle_key(m: &mut Model, k: KeyEvent) -> Option<Effect> {
    if !m.message.is_empty() && !m.filtering {
        m.message.clear();
    }
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
        return None;
    }
    if m.form.is_some() {
        return act::form_key(m, k);
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
        reset_cursor(m);
        return None;
    }
    if m.show_keys {
        m.show_keys = false;
        return None;
    }
    // `n` in Repos: a directory is being typed.
    if let Some(buf) = m.new_dir.as_mut() {
        match k.code {
            KeyCode::Esc => m.new_dir = None,
            KeyCode::Backspace => {
                buf.pop();
            }
            KeyCode::Char(c) => buf.push(c),
            KeyCode::Enter => {
                let dir = expand_home(buf.trim());
                m.new_dir = None;
                if !dir.is_empty() {
                    return Some(Effect::NewRepo(Utf8PathBuf::from(dir)));
                }
            }
            _ => {}
        }
        return None;
    }
    clamp_cursors(m);
    match (k.code, k.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            return Some(Effect::Quit);
        }
        (KeyCode::Char('?'), _) => m.show_keys = true,
        (KeyCode::Tab | KeyCode::BackTab, _) => {
            m.overlay = Overlay::None;
            m.mode = match m.mode {
                Mode::Watch => Mode::Act,
                Mode::Act => Mode::Watch,
            };
        }
        (KeyCode::Char('1'), _) => {
            m.overlay = Overlay::None;
            m.mode = Mode::Watch;
        }
        (KeyCode::Char('2'), _) => {
            m.overlay = Overlay::None;
            m.mode = Mode::Act;
        }
        (KeyCode::Char('3'), _) => {
            m.overlay = Overlay::Records(m.last_view);
        }
        (KeyCode::Char('t'), _) => {
            m.last_view = View::Tickets;
            m.overlay = Overlay::Records(View::Tickets);
        }
        (KeyCode::Char('p'), _) => {
            m.overlay = Overlay::Repos;
            return Some(Effect::ReadRepos);
        }
        (KeyCode::Esc, _) => {
            if m.filter.is_empty() {
                m.overlay = Overlay::None;
            } else {
                m.filter.clear();
                reset_cursor(m);
            }
        }
        (KeyCode::Char('/'), _) => {
            m.filtering = true;
            m.filter.clear();
            reset_cursor(m);
        }
        (KeyCode::Char('r'), _) if m.overlay == Overlay::Repos => {
            return Some(Effect::ReadRepos);
        }
        (KeyCode::Char('r'), _) => return Some(Effect::Reload),
        _ => {
            return match (m.overlay, m.mode) {
                (Overlay::None, Mode::Watch) => watch::key(m, k),
                (Overlay::None, Mode::Act) => act::key(m, k),
                _ => records::key(m, k),
            };
        }
    }
    None
}

/// A snapshot can arrive with a shorter list than the cursor stood on; a key
/// then acts on the row the frame shows, the last one.
fn clamp_cursors(m: &mut Model) {
    for panel in model::Panel::WATCH {
        let at = m
            .selected_in(panel)
            .min(watch::panel_len(m, panel).saturating_sub(1));
        m.select_in(panel, at);
    }
    for panel in model::Panel::ACT {
        let at = m
            .selected_in(panel)
            .min(act::panel_len(m, panel).saturating_sub(1));
        m.select_in(panel, at);
    }
}

/// A filter that changed moves the cursor to the top of what it left.
fn reset_cursor(m: &mut Model) {
    if m.overlay == Overlay::None {
        let panel = m.panel();
        m.select_in(panel, 0);
    } else {
        records::select(m, 0);
    }
}

/// `war sign <target> --ssh-sign`: what every signature from here runs, in a
/// child that owns the terminal.
fn sign_argv(target: &str) -> Vec<String> {
    argv::words(&["sign", target, "--ssh-sign"])
}

/// The app steps out of the alternate screen and raw mode, waits for the
/// child, and steps back. A child that succeeded returns at once; one that
/// failed leaves its output up until Enter, so it can be read. Returns the
/// child's exit code.
fn run_child_inherit(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    root: &Utf8Path,
    args: &[&str],
) -> Result<i32, RepoError> {
    use std::io::Write as _;
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
    let code = status.as_ref().map_or(-1, |s| s.code().unwrap_or(-1));
    if code != 0 {
        let _ = writeln!(
            stdout,
            "\n[war {} exited {code}] — press Enter to return",
            args.join(" ")
        );
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
    }
    let _ = term::Guard::enter().map(std::mem::forget);
    terminal
        .clear()
        .map_err(io("could not clear the terminal"))?;
    Ok(code)
}

/// `~/x` → `$HOME/x`: a directory typed at the prompt reads as a shell would.
fn expand_home(s: &str) -> String {
    match (s.strip_prefix("~/"), std::env::var("HOME")) {
        (Some(rest), Ok(home)) => format!("{home}/{rest}"),
        _ if s == "~" => std::env::var("HOME").unwrap_or_else(|_| s.to_owned()),
        _ => s.to_owned(),
    }
}

/// A child's stdout, on a background thread, for a caller that reads it.
fn run_child_output(root: &Utf8Path, args: &[&str]) -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::process::Command::new(exe)
        .arg("--root")
        .arg(root.as_str())
        .args(args)
        .current_dir(root)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .map_err(|e| format!("could not run war {}: {e}", args.join(" ")))
}

/// A child that outlives the app: its own process group, no terminal, its
/// output in `log`. Returns its pid.
fn run_child_detached(root: &Utf8Path, args: &[&str], log: &Utf8Path) -> Result<u32, String> {
    use std::process::Stdio;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    // `[go] allowed_acts = "declared"` writes no session files, so nothing
    // has made its directory.
    if let Some(dir) = log.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{dir}: {e}"))?;
    }
    let file = std::fs::File::create(log).map_err(|e| format!("{log}: {e}"))?;
    let err = file.try_clone().map_err(|e| format!("{log}: {e}"))?;
    std::process::Command::new(exe)
        .arg("--root")
        .arg(root.as_str())
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(file)
        .stderr(err)
        .own_group()
        .spawn()
        .map(|c| c.id())
        .map_err(|e| format!("could not run war {}: {e}", args.join(" ")))
}

/// A process group of its own, so the app's terminal signals miss a child
/// that is meant to outlive it.
trait OwnGroup {
    fn own_group(&mut self) -> &mut Self;
}

impl OwnGroup for std::process::Command {
    #[cfg(unix)]
    fn own_group(&mut self) -> &mut Self {
        std::os::unix::process::CommandExt::process_group(self, 0)
    }
    #[cfg(not(unix))]
    fn own_group(&mut self) -> &mut Self {
        self
    }
}

/// A child whose output the app shows in a popup: `--show`, an auto remedy.
/// Called on a background thread, never the one that draws.
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

fn header(m: &Model) -> Line<'static> {
    let chosen = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let tab = |label: &'static str, on: bool| {
        Span::styled(label, if on { chosen } else { Style::default() })
    };
    let bare = m.overlay == Overlay::None;
    Line::from(vec![
        Span::raw(" "),
        tab("1 Watch", bare && m.mode == Mode::Watch),
        Span::raw("  "),
        tab("2 Act", bare && m.mode == Mode::Act),
        Span::raw("  "),
        tab("3 Records", matches!(m.overlay, Overlay::Records(_))),
        Span::raw("  "),
        tab("p Repos", m.overlay == Overlay::Repos),
    ])
}

fn draw(f: &mut ratatui::Frame, m: &Model) {
    let area = f.area();
    let [head, body, cmd_line, status_line] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);
    f.render_widget(Paragraph::new(header(m)), head);

    match (m.overlay, m.mode) {
        (Overlay::None, Mode::Watch) => watch::draw(f, m, body),
        (Overlay::None, Mode::Act) => act::draw(f, m, body),
        _ => records::draw(f, m, body),
    }

    let command = match (m.overlay, m.mode) {
        (Overlay::None, Mode::Watch) => watch::command(m),
        (Overlay::None, Mode::Act) => act::command(m),
        _ => records::current(m).map_or_else(String::new, |r| r.command.clone()),
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  $ ", Style::default().fg(Color::DarkGray)),
            Span::raw(command),
        ])),
        cmd_line,
    );
    let status = if let Some(buf) = &m.new_dir {
        format!("  new repository directory (Enter runs war init there, Esc cancels): {buf}_")
    } else if m.filtering {
        format!("  / {}_", m.filter)
    } else if !m.filter.is_empty() {
        format!(
            "  {}   filter: {}   (Esc clears)",
            m.status_line(),
            m.filter
        )
    } else {
        format!("  {}   ? keys · q quit", m.status_line())
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
    if m.form.is_some() {
        act::draw_form(f, m, centered(area, 76, 60));
    }
    if m.show_keys {
        let area = centered(area, 80, 80);
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

/// The documents Help can show: shipped ones embedded at build time (so
/// help never drifts from what `war init` ships), this repository's own
/// first when present.
fn load_docs(root: &Utf8Path) -> Vec<Doc> {
    let mut docs = Vec::new();
    // The master document first (OW-ADR-0022): what is authoritative here
    // now, generated by `war compile`. Read from disk, because it is this
    // repository's; a program that has not compiled has none.
    if let Ok(text) =
        std::fs::read_to_string(root.join(openwarrant_compiler::current::CURRENT_PATH))
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
        if let Ok(text) = std::fs::read_to_string(root.join(name)) {
            docs.push(Doc {
                name: format!("this repository: {name}"),
                text,
            });
        }
    }
    let ns = Repository::discover(Some(root.to_owned())).map_or_else(
        |_| "OW".to_owned(),
        |r| r.config.project.namespace.as_str().to_owned(),
    );
    docs.extend(embedded_docs(&ns));
    docs
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
pub const KEYS: &str = "\
Tab                     flip between Watch and Act
1                       Watch: where the work stands, who is on it
2                       Act: create, dispatch, answer, sign
3                       Records over either mode: tickets, frontier, corpus, obligations, evidence, journal, roadmap, setup, help
p                       Repos from anywhere: Enter opens one, n starts one, r re-reads
t                       Records, on Tickets
Esc                     close Records or Repos; clear a filter
h l                     move between panels
j k                     move (in a document: next/previous document)
/                       filter the lists showing, or search the transcript from Live (Esc clears)
Enter                   open the row's detail; sign (Act, Help); run war init (Setup); open a repository
v                       show the request behind a signing row
space  a  n             Sign panel: check row / check all / check none (these win over a above)
s                       Sign panel: sign the checked acts, one or a batch in one dialog
x                       run the row's auto remedy (never a signing act)
c                       Act: create work; elsewhere: the commit message `war admin commit --write` would use
d                       Watch, Act: dispatch the highlighted task; Help: documents on/off
a                       Watch, Act: answer the highlighted question, or note on the highlighted work
i                       Act, Answer panel: `war plan interview`, read-only
e                       Act, Sign panel: the obligations and evidence of the highlighted act
f                       Act, Sign panel: `war admin diff` of the highlighted act
[  ]                    Records: previous/next view; a document: previous/next; Watch: previous/next agent
r                       re-read the records now (they also refresh on their own)
?                       this table
q / Ctrl-C              quit
";
