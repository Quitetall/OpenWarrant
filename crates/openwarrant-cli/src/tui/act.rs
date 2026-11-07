// SPDX-License-Identifier: Apache-2.0
//! Act: the things a person starts here. Each is a real `war` command run in
//! a child; the app holds no key and writes no record. Forms gather what a
//! command needs and hand back its argv: none of them writes anything itself.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use super::model::{
    Dispatch, DispatchStep, Effect, ExecutorRow, Form, Launcher, Model, Panel, ReadyRow, Row,
};
use super::{argv, watch};

fn step(panel: Panel, by: isize) -> Panel {
    let all = Panel::ACT;
    let n = all.len().cast_signed();
    let at = all.iter().position(|p| *p == panel).unwrap_or(0);
    all[(at.cast_signed() + by).rem_euclid(n).cast_unsigned()]
}

pub fn panel_len(m: &Model, panel: Panel) -> usize {
    match panel {
        Panel::Dispatch => visible_ready(m).len(),
        Panel::Answer => watch::visible_questions(m).len(),
        Panel::Sign => visible_queue(m).len(),
        _ => 0,
    }
}

/// The ready tasks the filter keeps.
#[must_use]
pub fn visible_ready(m: &Model) -> Vec<&ReadyRow> {
    m.snap
        .ready
        .iter()
        .filter(|r| m.passes(&format!("{} {}", r.id, r.title)))
        .collect()
}

/// The queue rows the filter keeps.
#[must_use]
pub fn visible_queue(m: &Model) -> Vec<&Row> {
    m.snap.queue.iter().filter(|r| m.passes(&r.text)).collect()
}

fn sign_target(m: &Model) -> Option<String> {
    visible_queue(m)
        .get(m.selected_in(Panel::Sign))
        .and_then(|r| r.sign_target.clone())
}

/// The acts checked, in queue order: what `s` signs.
#[must_use]
pub fn checked_targets(m: &Model) -> Vec<String> {
    m.snap
        .queue
        .iter()
        .filter_map(|r| r.sign_target.clone())
        .filter(|t| m.checked.contains(t))
        .collect()
}

pub fn key(m: &mut Model, k: KeyEvent) -> Option<Effect> {
    let panel = m.act_panel;
    match k.code {
        KeyCode::Char('h') | KeyCode::Left => m.act_panel = step(panel, -1),
        KeyCode::Char('l') | KeyCode::Right => m.act_panel = step(panel, 1),
        KeyCode::Char('j') | KeyCode::Down => {
            let n = panel_len(m, panel);
            if n > 0 {
                let at = (m.selected_in(panel) + 1).min(n - 1);
                m.select_in(panel, at);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let at = m.selected_in(panel).saturating_sub(1);
            m.select_in(panel, at);
        }
        KeyCode::Char(' ') if panel == Panel::Sign => {
            if let Some(t) = sign_target(m)
                && !m.checked.remove(&t)
            {
                m.checked.insert(t);
            }
        }
        KeyCode::Char('a') if panel == Panel::Sign => {
            // All of what shows: a filter narrows what `a` means.
            m.checked = visible_queue(m)
                .iter()
                .filter_map(|r| r.sign_target.clone())
                .collect();
        }
        KeyCode::Char('n') if panel == Panel::Sign => m.checked.clear(),
        KeyCode::Char('s') if panel == Panel::Sign => {
            let targets = checked_targets(m);
            match targets.as_slice() {
                [] => m.message = "nothing checked: space checks a row, a checks all".to_owned(),
                [t] => {
                    let argv = super::sign_argv(t);
                    m.checked.clear();
                    return Some(Effect::Terminal { argv, root: None });
                }
                // More than one: one batch, one dialog (OW-WAR-0072).
                _ => {
                    m.checked.clear();
                    return Some(Effect::Terminal {
                        argv: argv::sign_batch(&targets),
                        root: None,
                    });
                }
            }
        }
        KeyCode::Enter if panel == Panel::Sign => {
            return sign_target(m).map(|t| Effect::Terminal {
                argv: super::sign_argv(&t),
                root: None,
            });
        }
        KeyCode::Char('v') if panel == Panel::Sign => {
            return sign_target(m).map(|t| Effect::Capture {
                title: format!("war sign {t} --show"),
                argv: argv::sign_show(&t),
            });
        }
        KeyCode::Char('e') if panel == Panel::Sign => evidence(m),
        KeyCode::Char('f') if panel == Panel::Sign => {
            return warrant_of(m).map(|alias| Effect::Capture {
                title: format!("war admin diff {alias}"),
                argv: argv::diff(&alias),
            });
        }
        KeyCode::Char('c') => open_create(m),
        KeyCode::Enter if panel == Panel::Create => open_create(m),
        KeyCode::Char('d') => open_dispatch(m),
        KeyCode::Enter if panel == Panel::Dispatch => open_dispatch(m),
        KeyCode::Char('a') => open_answer(m),
        KeyCode::Enter if panel == Panel::Answer => open_answer(m),
        KeyCode::Char('i') if panel == Panel::Answer => {
            let warrant = watch::visible_questions(m)
                .get(m.selected_in(Panel::Answer))?
                .warrant
                .clone();
            return Some(Effect::Capture {
                title: format!("war plan interview {warrant} (read-only)"),
                argv: argv::interview(&warrant),
            });
        }
        _ => {}
    }
    None
}

/// The Warrant behind a signing target: `OW-WAR-0001` or `OW-WAR-0001/D-002`.
/// A SAS version or a gate names none.
fn warrant_of(m: &mut Model) -> Option<String> {
    let target = sign_target(m)?;
    let alias = target.split('/').next().unwrap_or(&target).to_owned();
    let known = m
        .snap
        .views
        .get(&super::model::View::Corpus)
        .is_some_and(|rows| {
            rows.iter()
                .any(|r| r.text.starts_with(&format!("{alias} ")))
        });
    if known {
        Some(alias)
    } else {
        m.message = format!("{target} is not a Warrant: `v` shows its request");
        None
    }
}

/// `e`: what is being signed for, from the rows already read.
fn evidence(m: &mut Model) {
    let Some(alias) = warrant_of(m) else {
        return;
    };
    let lines = |view| {
        let rows = m.snap.views.get(&view).map_or(&[][..], Vec::as_slice);
        let found: Vec<&str> = rows
            .iter()
            .map(|r| r.text.as_str())
            .filter(|t| t.split_whitespace().any(|w| w == alias))
            .collect();
        if found.is_empty() {
            "none recorded".to_owned()
        } else {
            found.join("\n")
        }
    };
    let text = format!(
        "Obligations\n{}\n\nEvidence\n{}\n\n`f` shows the diff, `v` the request.",
        lines(super::model::View::Obligations),
        lines(super::model::View::Evidence),
    );
    m.popup = Some((format!("{alias}: what this signature rests on"), text, 0));
}

fn open_create(m: &mut Model) {
    m.act_panel = Panel::Create;
    m.form = Some(Form::Create {
        title: String::new(),
        items: String::new(),
        on_items: false,
        draft: false,
    });
}

/// `d`: the highlighted ready task, on to its executor.
pub fn open_dispatch(m: &mut Model) {
    let Some(task) = visible_ready(m)
        .get(m.selected_in(Panel::Dispatch))
        .map(|r| (*r).clone())
    else {
        m.message = "nothing can start now".to_owned();
        return;
    };
    if m.snap.executors.is_empty() {
        m.message = "no executor: `[go]` could not be read".to_owned();
        return;
    }
    m.act_panel = Panel::Dispatch;
    m.form = Some(Form::Dispatch {
        node: task.id,
        title: task.title,
        step: DispatchStep::Executor,
        executor: 0,
        scope: String::new(),
        worktree: true,
        launcher: 0,
    });
}

/// `a`: the highlighted question.
pub fn open_answer(m: &mut Model) {
    let Some(q) = watch::visible_questions(m)
        .get(m.selected_in(Panel::Answer))
        .map(|q| (*q).clone())
    else {
        m.message = "no open question".to_owned();
        return;
    };
    m.act_panel = Panel::Answer;
    m.form = Some(Form::Answer {
        warrant: q.warrant,
        id: q.id,
        question: q.question,
        recommended: q.recommended,
        text: String::new(),
        who: m.snap.human.clone().unwrap_or_default(),
        on_who: false,
    });
}

/// `a` on work: a note on `target`, told to an agent in the holder's
/// workspace if Herdr has one there.
pub fn open_note(m: &mut Model, target: String) {
    let workspace = m
        .snap
        .claims
        .iter()
        .find(|c| c.target == target || target.starts_with(&format!("{}/", c.target)))
        .and_then(|c| c.workspace.clone());
    let pane = workspace.as_ref().and_then(|w| {
        m.live
            .agents
            .iter()
            .find(|a| a.workdir() == w)
            .map(|a| a.pane_id.clone())
    });
    m.form = Some(Form::Note {
        target,
        text: String::new(),
        workspace: workspace.map(Into::into),
        pane,
        now: false,
    });
}

/// The launchers an executor can use: Herdr when it is there and knows the
/// agent; a terminal; detached, last when the harness needs a terminal.
#[must_use]
pub fn launchers(m: &Model, executor: &ExecutorRow) -> Vec<Launcher> {
    let mut out = Vec::new();
    if let (super::model::Herdr::Connected { .. }, Some(kind)) =
        (&m.live.herdr, &executor.herdr_kind)
    {
        out.push(Launcher::Herdr { kind: kind.clone() });
    }
    if executor.needs_terminal {
        out.extend([Launcher::Terminal, Launcher::Detached]);
    } else {
        out.extend([Launcher::Detached, Launcher::Terminal]);
    }
    out
}

fn split_scope(scope: &str) -> Vec<String> {
    scope.split_whitespace().map(str::to_owned).collect()
}

fn dispatch_effect(
    node: &str,
    executor: &ExecutorRow,
    scope: &str,
    worktree: bool,
    launcher: Option<Launcher>,
) -> Effect {
    let paths = split_scope(scope);
    match launcher {
        Some(launcher) if worktree => Effect::Dispatch(Dispatch {
            node: node.to_owned(),
            actor: executor.name.clone(),
            paths,
            launcher,
        }),
        _ => Effect::Say {
            title: format!("claim {node}"),
            argv: argv::claim_here(node, &executor.name, &paths),
            at: None,
            after: "Claimed in this checkout. Nothing was started: the holder works here."
                .to_owned(),
            prompt: None,
        },
    }
}

fn after_answer() -> String {
    "Recorded. A question answers no running agent by itself: the agent reads this on its next \
     `war start` or `war view show`."
        .to_owned()
}

fn after_note(held: bool) -> String {
    if held {
        "Recorded in the holder's checkout. Nothing interrupts it: it reads this at its next `war \
         view show`, `war view prime <id>` or `war start`."
            .to_owned()
    } else {
        "Recorded. Nobody holds this now: whoever starts it reads the note in its brief.".to_owned()
    }
}

/// A key in the open form. `Esc` leaves it with nothing done.
pub fn form_key(m: &mut Model, k: KeyEvent) -> Option<Effect> {
    if k.code == KeyCode::Esc {
        m.form = None;
        return None;
    }
    let mut form = m.form.take()?;
    let effect = match &mut form {
        Form::Create {
            title,
            items,
            on_items,
            draft,
        } => match k.code {
            KeyCode::Enter if !*on_items => {
                if title.trim().is_empty() {
                    m.message = "a title is the one thing `war create` needs".to_owned();
                } else {
                    *on_items = true;
                }
                None
            }
            KeyCode::Enter => {
                let list: Vec<String> = items
                    .split(';')
                    .map(str::trim)
                    .filter(|i| !i.is_empty())
                    .map(str::to_owned)
                    .collect();
                let argv = argv::create(title.trim(), &list, *draft);
                m.form = None;
                return Some(Effect::Capture {
                    title: format!("war create: {}", title.trim()),
                    argv,
                });
            }
            KeyCode::Tab if m.snap.drafter.is_some() => {
                *draft = !*draft;
                None
            }
            code => {
                edit(if *on_items { items } else { title }, code);
                None
            }
        },
        Form::Dispatch {
            node,
            step,
            executor,
            scope,
            worktree,
            launcher,
            ..
        } => {
            let chosen = m.snap.executors.get(*executor).cloned().unwrap_or_default();
            match (*step, k.code) {
                (DispatchStep::Executor, KeyCode::Char('j') | KeyCode::Down) => {
                    *executor = (*executor + 1).min(m.snap.executors.len().saturating_sub(1));
                    None
                }
                (DispatchStep::Executor, KeyCode::Char('k') | KeyCode::Up) => {
                    *executor = executor.saturating_sub(1);
                    None
                }
                (DispatchStep::Executor, KeyCode::Enter) => {
                    if let Some(why) = &chosen.unavailable {
                        m.message = format!("{} is unavailable here: {why}", chosen.name);
                    } else {
                        *step = DispatchStep::Scope;
                    }
                    None
                }
                (DispatchStep::Scope, KeyCode::Enter) => {
                    *step = DispatchStep::Worktree;
                    None
                }
                (DispatchStep::Scope, code) => {
                    edit(scope, code);
                    None
                }
                (
                    DispatchStep::Worktree,
                    KeyCode::Char(' ' | 'j' | 'k') | KeyCode::Up | KeyCode::Down,
                ) => {
                    *worktree = !*worktree;
                    None
                }
                (DispatchStep::Worktree, KeyCode::Enter) if *worktree => {
                    *step = DispatchStep::Launch;
                    None
                }
                (DispatchStep::Worktree, KeyCode::Enter) => {
                    Some(dispatch_effect(node, &chosen, scope, false, None))
                }
                (DispatchStep::Launch, KeyCode::Char('j') | KeyCode::Down) => {
                    *launcher = (*launcher + 1).min(launchers(m, &chosen).len() - 1);
                    None
                }
                (DispatchStep::Launch, KeyCode::Char('k') | KeyCode::Up) => {
                    *launcher = launcher.saturating_sub(1);
                    None
                }
                (DispatchStep::Launch, KeyCode::Enter) => {
                    let how = launchers(m, &chosen).get(*launcher).cloned();
                    Some(dispatch_effect(node, &chosen, scope, true, how))
                }
                _ => None,
            }
        }
        Form::Answer {
            warrant,
            id,
            recommended,
            text,
            who,
            on_who,
            ..
        } => match k.code {
            KeyCode::Enter if !*on_who => {
                if text.trim().is_empty() {
                    text.clone_from(recommended);
                }
                if text.trim().is_empty() {
                    m.message = "an answer needs words: there is no recommendation".to_owned();
                } else {
                    *on_who = true;
                }
                None
            }
            KeyCode::Enter => {
                if who.trim().is_empty() {
                    m.message = "who is answering? It must hold a human role".to_owned();
                    None
                } else {
                    Some(Effect::Say {
                        title: format!("answer {warrant}/{id}"),
                        argv: argv::answer(warrant, id, text.trim(), who.trim()),
                        at: None,
                        after: after_answer(),
                        prompt: None,
                    })
                }
            }
            code => {
                edit(if *on_who { who } else { text }, code);
                None
            }
        },
        Form::Note {
            target,
            text,
            workspace,
            pane,
            now,
        } => match k.code {
            KeyCode::Tab if pane.is_some() => {
                *now = !*now;
                None
            }
            KeyCode::Enter if text.trim().is_empty() => {
                m.message = "an empty note says nothing".to_owned();
                None
            }
            KeyCode::Enter => {
                let held =
                    m.snap.claims.iter().any(|c| {
                        c.target == *target || target.starts_with(&format!("{}/", c.target))
                    });
                Some(Effect::Say {
                    title: format!("note on {target}"),
                    argv: argv::note(target, text.trim()),
                    at: workspace.clone(),
                    after: after_note(held),
                    prompt: pane
                        .clone()
                        .filter(|_| *now)
                        .map(|p| (p, format!("A note on {target}: {}", text.trim()))),
                })
            }
            code => {
                edit(text, code);
                None
            }
        },
    };
    if effect.is_none() {
        m.form = Some(form);
    }
    effect
}

fn edit(field: &mut String, code: KeyCode) {
    match code {
        KeyCode::Backspace => {
            field.pop();
        }
        KeyCode::Char(c) => field.push(c),
        _ => {}
    }
}

/// The command behind the highlighted row, for the bottom line.
#[must_use]
pub fn command(m: &Model) -> String {
    let at = m.selected_in(m.act_panel);
    match m.act_panel {
        Panel::Create => "war create \"what this work accomplishes\"".to_owned(),
        Panel::Dispatch => visible_ready(m)
            .get(at)
            .map(|r| format!("war start {} --worktree", r.id))
            .unwrap_or_default(),
        Panel::Answer => watch::visible_questions(m)
            .get(at)
            .map(|q| q.command.clone())
            .unwrap_or_default(),
        _ => visible_queue(m)
            .get(at)
            .map(|r| r.command.clone())
            .unwrap_or_default(),
    }
}

fn text_panel(
    f: &mut ratatui::Frame,
    m: &Model,
    area: Rect,
    panel: Panel,
    title: &str,
    lines: Vec<Line<'static>>,
) {
    f.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title.to_owned())
                .border_style(watch::focused(m.panel() == panel)),
        ),
        area,
    );
}

fn field(label: &str, value: &str, on: bool) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(Color::DarkGray)),
        Span::raw(value.to_owned()),
        Span::raw(if on { "_" } else { "" }),
    ])
}

fn choice(label: String, on: bool, note: Option<&str>) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!("{} {label}", if on { ">" } else { " " }),
        if on {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        },
    )];
    if let Some(n) = note {
        spans.push(Span::styled(
            format!("  unavailable: {n}"),
            Style::default().fg(Color::DarkGray),
        ));
    }
    Line::from(spans)
}

fn form_lines(m: &Model, form: &Form) -> (String, Vec<Line<'static>>, String) {
    match form {
        Form::Create {
            title,
            items,
            on_items,
            draft,
        } => {
            let mut lines = vec![
                field("Title", title, !on_items),
                field("Items, separated by ;", items, *on_items),
            ];
            let mut hint = "Enter next · Esc cancels".to_owned();
            if let Some(d) = &m.snap.drafter {
                lines.push(field(
                    &format!("Draft items with the configured process `{d}`"),
                    if *draft { "yes" } else { "no" },
                    false,
                ));
                hint.push_str(" · Tab toggles the drafter");
            }
            (" Create: war create ".to_owned(), lines, hint)
        }
        Form::Dispatch {
            node,
            title,
            step,
            executor,
            scope,
            worktree,
            launcher,
        } => {
            let chosen = m.snap.executors.get(*executor).cloned().unwrap_or_default();
            let mut lines = vec![field("Task", &format!("{node} {title}"), false)];
            lines.push(Line::from(""));
            let hint = match step {
                DispatchStep::Executor => {
                    lines.push(Line::from("Executor, from [go]"));
                    for (i, e) in m.snap.executors.iter().enumerate() {
                        lines.push(choice(
                            e.name.clone(),
                            i == *executor,
                            e.unavailable.as_deref(),
                        ));
                    }
                    "j k choose · Enter next · Esc cancels"
                }
                DispatchStep::Scope => {
                    lines.push(field("Executor", &chosen.name, false));
                    lines.push(field(
                        "Write scope, space separated; empty is what the task declares",
                        scope,
                        true,
                    ));
                    "Enter next · Esc cancels"
                }
                DispatchStep::Worktree => {
                    lines.push(field("Executor", &chosen.name, false));
                    lines.push(choice("a new worktree".to_owned(), *worktree, None));
                    lines.push(choice(
                        "this checkout: claim only, start nothing".to_owned(),
                        !*worktree,
                        None,
                    ));
                    "space toggles · Enter next · Esc cancels"
                }
                DispatchStep::Launch => {
                    lines.push(field("Executor", &chosen.name, false));
                    lines.push(Line::from("Launch in"));
                    for (i, l) in launchers(m, &chosen).iter().enumerate() {
                        lines.push(choice(l.label().to_owned(), i == *launcher, None));
                    }
                    "j k choose · Enter claims, makes the worktree and launches · Esc cancels"
                }
            };
            (" Dispatch ".to_owned(), lines, hint.to_owned())
        }
        Form::Answer {
            warrant,
            id,
            question,
            recommended,
            text,
            who,
            on_who,
        } => (
            format!(" Answer {warrant}/{id}: war plan answer "),
            vec![
                Line::from(question.clone()),
                Line::from(""),
                field("Recommended", recommended, false),
                field("Answer", text, !on_who),
                field("As (a human role in roles.toml)", who, *on_who),
                Line::from(""),
                Line::from("An agent reads this on its next `war start` or `war view show`."),
            ],
            "Enter next, an empty answer takes the recommendation · Esc cancels".to_owned(),
        ),
        Form::Note {
            target,
            text,
            pane,
            now,
            ..
        } => {
            let mut lines = vec![field("Note", text, true), Line::from("")];
            let mut hint = "Enter records it · Esc cancels".to_owned();
            lines.push(Line::from(match (pane, now) {
                (Some(p), true) => format!("Herdr delivers it to {p} as a prompt now."),
                _ => "The holder reads it at its next `war view show`, `war view prime <id>` or `war \
                      start`, not before."
                    .to_owned(),
            }));
            if pane.is_some() {
                hint.push_str(" · Tab: deliver through Herdr now");
            }
            (format!(" Note on {target}: war note "), lines, hint)
        }
    }
}

pub fn draw_form(f: &mut ratatui::Frame, m: &Model, area: Rect) {
    let Some(form) = &m.form else {
        return;
    };
    let (title, mut lines, hint) = form_lines(m, form);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(title)),
        area,
    );
}

fn list(
    f: &mut ratatui::Frame,
    m: &Model,
    area: Rect,
    panel: Panel,
    title: String,
    items: Vec<ListItem<'_>>,
) {
    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some(m.selected_in(panel).min(items.len() - 1)));
    }
    f.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(watch::focused(m.panel() == panel)),
            )
            .highlight_style(if m.panel() == panel {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            }),
        area,
        &mut state,
    );
}

pub fn draw(f: &mut ratatui::Frame, m: &Model, body: Rect) {
    let [top, bottom] =
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(body);
    let [create, dispatch] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).areas(top);
    let [answer, sign] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(bottom);

    let mut create_lines = vec![Line::from("c  a new item through `war create`")];
    if let Some(d) = &m.snap.drafter {
        create_lines.push(Line::from(format!(
            "   with the configured drafter `{d}` if you ask"
        )));
    }
    text_panel(f, m, create, Panel::Create, " Create ", create_lines);
    let mut ready: Vec<ListItem> = visible_ready(m)
        .iter()
        .map(|r| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{} ", r.id),
                    if r.critical {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Cyan)
                    },
                ),
                Span::raw(r.title.clone()),
            ]))
        })
        .collect();
    if ready.is_empty() {
        ready.push(ListItem::new("nothing can start now"));
    }
    list(
        f,
        m,
        dispatch,
        Panel::Dispatch,
        " Dispatch — what can start now, critical path first; d dispatches ".to_owned(),
        ready,
    );
    let mut questions: Vec<ListItem> = watch::visible_questions(m)
        .iter()
        .map(|q| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{}: ", q.target()),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(q.question.clone()),
            ]))
        })
        .collect();
    if questions.is_empty() {
        questions.push(ListItem::new("no open questions"));
    }
    list(
        f,
        m,
        answer,
        Panel::Answer,
        " Answer — a answers, i interviews (read-only) ".to_owned(),
        questions,
    );
    let queue_items: Vec<ListItem> = visible_queue(m)
        .iter()
        .map(|r| {
            let on = r
                .sign_target
                .as_ref()
                .is_some_and(|t| m.checked.contains(t));
            ListItem::new(Line::from(vec![
                Span::styled(
                    if on { "[x] " } else { "[ ] " },
                    if on {
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
    list(
        f,
        m,
        sign,
        Panel::Sign,
        format!(
            " Sign — {} act(s); space checks, s signs, Enter signs row, v request, e evidence, f diff ",
            m.snap.queue.len()
        ),
        queue_items,
    );
}
