// SPDX-License-Identifier: Apache-2.0
//! Act: the things a person starts here. Each is a real `war` command run in
//! a child; the app holds no key and writes no record. What does not work yet
//! says so and names the command that does.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use super::model::{Effect, Model, Panel};
use super::{argv, watch};

fn step(panel: Panel, by: isize) -> Panel {
    let all = Panel::ACT;
    let n = all.len().cast_signed();
    let at = all.iter().position(|p| *p == panel).unwrap_or(0);
    all[(at.cast_signed() + by).rem_euclid(n).cast_unsigned()]
}

fn panel_len(m: &Model, panel: Panel) -> usize {
    match panel {
        Panel::Dispatch => m.snap.ready.len(),
        Panel::Answer => m.snap.questions.len(),
        Panel::Sign => m.snap.queue.len(),
        _ => 0,
    }
}

fn sign_target(m: &Model) -> Option<String> {
    m.snap
        .queue
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
            m.checked = m
                .snap
                .queue
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
        KeyCode::Char('c') | KeyCode::Enter if panel == Panel::Create => {
            m.message = "creating here is unavailable in this build: run `war create`".to_owned();
        }
        KeyCode::Char('d') | KeyCode::Enter if panel == Panel::Dispatch => {
            m.message =
                "dispatch is unavailable in this build: run `war start --worktree <id>`".to_owned();
        }
        KeyCode::Char('a') | KeyCode::Enter if panel == Panel::Answer => {
            m.message =
                "answering here is unavailable in this build: the bottom line is the command"
                    .to_owned();
        }
        _ => {}
    }
    None
}

/// The command behind the highlighted row, for the bottom line.
#[must_use]
pub fn command(m: &Model) -> String {
    let at = m.selected_in(m.act_panel);
    match m.act_panel {
        Panel::Create => "war create \"what this work accomplishes\"".to_owned(),
        Panel::Dispatch => m
            .snap
            .ready
            .get(at)
            .map(|r| format!("war start --worktree {}", r.id))
            .unwrap_or_default(),
        Panel::Answer => m
            .snap
            .questions
            .get(at)
            .map(|q| q.command.clone())
            .unwrap_or_default(),
        _ => watch::queue(m)
            .get(at)
            .map(|r| r.command.clone())
            .unwrap_or_default(),
    }
}

fn unavailable(f: &mut ratatui::Frame, m: &Model, area: Rect, panel: Panel, title: &str, why: &str) {
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "unavailable in this build",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(why.to_owned()),
        ])
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title.to_owned())
                .border_style(watch::focused(m.panel() == panel)),
        ),
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

    unavailable(
        f,
        m,
        create,
        Panel::Create,
        " Create ",
        "Run `war create \"what this work accomplishes\"` in a shell.",
    );
    let mut ready: Vec<ListItem> = m
        .snap
        .ready
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
        " Ready (dispatch unavailable in this build: the bottom line is the command) ".to_owned(),
        ready,
    );
    let mut questions: Vec<ListItem> = m
        .snap
        .questions
        .iter()
        .map(|q| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{}: ", q.target()), Style::default().fg(Color::Cyan)),
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
        " Answer (unavailable in this build: the bottom line is the command) ".to_owned(),
        questions,
    );
    let queue_items: Vec<ListItem> = m
        .snap
        .queue
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
            " Sign — {} act(s); space checks, s signs checked, Enter signs row, v shows request ",
            m.snap.queue.len()
        ),
        queue_items,
    );
}
