// SPDX-License-Identifier: Apache-2.0
//! Watch: where the work stands, who is on it, and what they are doing. All
//! read from the records; what was not recorded says `UNKNOWN`. Lives in
//! `tui/` because it draws. No row here runs anything.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use super::model::{
    ClaimRow, Count, Effect, Herdr, Model, NoticeRow, Panel, QuestionItem, WorkRow, span,
};

fn dim() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub(super) fn focused(on: bool) -> Style {
    if on {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

pub(super) fn state_badge(state: &str) -> (&'static str, Style) {
    match state {
        "done" | "resolved" => ("[DONE] ", Style::default().fg(Color::Green)),
        "in_progress" | "running" => (
            "[ACTV] ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        "open" | "ready" => ("[OPEN] ", Style::default().fg(Color::White)),
        "waiting" => ("[WAIT] ", dim()),
        "blocked" => ("[BLCK] ", Style::default().fg(Color::Yellow)),
        "draft" => ("[DRFT] ", Style::default().fg(Color::White)),
        "authorized" => ("[AUTH] ", Style::default().fg(Color::White)),
        "UNKNOWN" | "unknown" => ("[ ?? ] ", Style::default().fg(Color::Red)),
        _ => ("[ .. ] ", dim()),
    }
}

/// The rows of a panel, as many as its cursor can reach.
#[must_use]
pub fn panel_len(m: &Model, panel: Panel) -> usize {
    match panel {
        Panel::Work => visible_work(m).len(),
        Panel::Claims => visible_claims(m).len(),
        Panel::Questions => visible_questions(m).len(),
        Panel::Notices => visible_notices(m).len(),
        _ => 0,
    }
}

/// The Work rows the filter keeps.
#[must_use]
pub fn visible_work(m: &Model) -> Vec<&WorkRow> {
    m.snap
        .work
        .iter()
        .filter(|w| m.passes(&format!("{} {} {}", w.id, w.title, w.state)))
        .collect()
}

/// The claims the filter keeps.
#[must_use]
pub fn visible_claims(m: &Model) -> Vec<&ClaimRow> {
    m.snap
        .claims
        .iter()
        .filter(|c| m.passes(&claim_text(c)))
        .collect()
}

/// The questions the filter keeps.
#[must_use]
pub fn visible_questions(m: &Model) -> Vec<&QuestionItem> {
    m.snap
        .questions
        .iter()
        .filter(|q| m.passes(&format!("{} {}", q.target(), q.question)))
        .collect()
}

/// The notices the filter keeps.
#[must_use]
pub fn visible_notices(m: &Model) -> Vec<&NoticeRow> {
    m.snap
        .notices
        .iter()
        .filter(|n| m.passes(&format!("{} {} {}", n.ticket, n.actor, n.line)))
        .collect()
}

fn claim_text(c: &ClaimRow) -> String {
    format!(
        "{} {} {}",
        c.target,
        c.actor.as_deref().unwrap_or("UNKNOWN"),
        c.scope.join(" ")
    )
}

/// `h`/`l`, `j`/`k`, `Enter`, and the keys of the highlighted panel.
pub fn key(m: &mut Model, k: KeyEvent) -> Option<Effect> {
    let panel = m.watch_panel;
    match k.code {
        KeyCode::Char('h') | KeyCode::Left => m.watch_panel = step(panel, -1),
        KeyCode::Char('l') | KeyCode::Right => m.watch_panel = step(panel, 1),
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
        KeyCode::Char('[') => {
            let n = m.live.agents.len().max(1);
            m.agent_sel = (m.agent_sel + n - 1) % n;
        }
        KeyCode::Char(']') => m.agent_sel = (m.agent_sel + 1) % m.live.agents.len().max(1),
        KeyCode::Enter => detail(m),
        KeyCode::Char('d') => dispatch(m),
        KeyCode::Char('a') => answer(m),
        KeyCode::Char('c') => {
            return Some(Effect::Capture {
                title: "commit message (`war admin commit --write`)".to_owned(),
                argv: super::argv::commit_message(),
            });
        }
        _ => {}
    }
    None
}

/// `d`: the highlighted work, if it can start, in Act's dispatch form.
fn dispatch(m: &mut Model) {
    let id = visible_work(m)
        .get(m.selected_in(Panel::Work))
        .map(|w| w.id.clone());
    let at = id
        .as_ref()
        .and_then(|id| m.snap.ready.iter().position(|r| &r.id == id));
    match (m.watch_panel == Panel::Work, at) {
        (true, Some(at)) => {
            m.mode = super::model::Mode::Act;
            m.select_in(Panel::Dispatch, at);
            super::act::open_dispatch(m);
        }
        (true, None) => {
            m.message = format!(
                "{} cannot start now; Act lists what can",
                id.unwrap_or_else(|| "this".to_owned())
            );
        }
        _ => {
            m.mode = super::model::Mode::Act;
            m.act_panel = Panel::Dispatch;
        }
    }
}

/// `a`: answer a question, or leave a note on work or a claim.
fn answer(m: &mut Model) {
    let at = m.selected_in(m.watch_panel);
    match m.watch_panel {
        Panel::Work => {
            if let Some(id) = visible_work(m).get(at).map(|w| w.id.clone()) {
                super::act::open_note(m, id);
            }
        }
        Panel::Claims => {
            if let Some(target) = visible_claims(m).get(at).map(|c| c.target.clone()) {
                super::act::open_note(m, target);
            }
        }
        _ => {
            m.mode = super::model::Mode::Act;
            if m.watch_panel == Panel::Questions {
                m.select_in(Panel::Answer, at);
            }
            super::act::open_answer(m);
        }
    }
}

fn step(panel: Panel, by: isize) -> Panel {
    let all = Panel::WATCH;
    let n = all.len().cast_signed();
    let at = all.iter().position(|p| *p == panel).unwrap_or(0);
    all[(at.cast_signed() + by).rem_euclid(n).cast_unsigned()]
}

/// `Enter`: what the highlighted row knows.
fn detail(m: &mut Model) {
    let at = m.selected_in(m.watch_panel);
    let popup = match m.watch_panel {
        Panel::Work => visible_work(m).get(at).map(|w| {
            (
                format!("{} ({})", w.id, w.state),
                format!(
                    "{}\n\nlevel: {}\nstate: {}\ncritical path: {}\nheld by: {}\ntokens: {}",
                    w.title,
                    if w.level.is_empty() { "item" } else { &w.level },
                    w.state,
                    if w.critical { "yes" } else { "no" },
                    w.holder.as_deref().unwrap_or("nobody"),
                    tokens(w.tokens),
                ),
            )
        }),
        Panel::Claims => visible_claims(m).get(at).map(|c| {
            (
                format!("claim on {}", c.target),
                format!(
                    "holder: {}\nheld for: {}\nlease: {}\nlock renewed: {}\nworkspace: {}\nscope: {}\n\
                     tokens recorded: {}\n{}\n\
                     A lock is renewed by the holder's `war` commands. It says when one last ran, \
                     not that an agent is alive.",
                    c.actor.as_deref().unwrap_or("UNKNOWN (the lock could not be read)"),
                    span(m.now_unix.saturating_sub(c.since_unix)),
                    lease(c, m.now_unix),
                    renewed(c, m.now_unix),
                    c.workspace.as_deref().unwrap_or("UNKNOWN"),
                    if c.scope.is_empty() {
                        "none declared".to_owned()
                    } else {
                        c.scope.join(", ")
                    },
                    tokens(m.snap.usage.of(&c.target)),
                    m.snap
                        .collisions
                        .iter()
                        .filter(|x| x.a == c.target || x.b == c.target)
                        .map(|x| format!("collides with {} and {}: {}\n", x.a, x.b, x.why))
                        .collect::<String>(),
                ),
            )
        }),
        Panel::Questions => visible_questions(m).get(at).map(|q| {
            (
                format!("question {}", q.target()),
                format!(
                    "{}\n\nblocking: {}\nrecommended: {}\n\n{}",
                    q.question,
                    if q.blocking { "yes" } else { "no" },
                    if q.recommended.is_empty() {
                        "none"
                    } else {
                        &q.recommended
                    },
                    q.command,
                ),
            )
        }),
        Panel::Notices => visible_notices(m).get(at).map(|n| {
            (
                format!("notice to {} on {}", n.actor, n.ticket),
                format!(
                    "{}\n\nposted: {}\nstate: {}",
                    n.line,
                    n.at,
                    notice_state(n.told, n.seen)
                ),
            )
        }),
        _ => None,
    };
    if let Some((title, text)) = popup {
        m.popup = Some((title, text, 0));
    }
}

/// The command behind the highlighted row, for the bottom line.
#[must_use]
pub fn command(m: &Model) -> String {
    let at = m.selected_in(m.watch_panel);
    match m.watch_panel {
        Panel::Work => visible_work(m)
            .get(at)
            .map(|w| format!("war view show {}", w.id))
            .unwrap_or_default(),
        Panel::Claims => visible_claims(m)
            .get(at)
            .map(|c| format!("war view show {}", c.target))
            .unwrap_or_default(),
        Panel::Questions => visible_questions(m)
            .get(at)
            .map(|q| q.command.clone())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn tokens(t: Count) -> String {
    match t {
        Count::Known(n) => n.to_string(),
        Count::Unknown => "UNKNOWN".to_owned(),
    }
}

/// The total, and what it leaves out, so a number never reads as complete
/// when it is not. Dollars only as the table's dated estimate.
pub(super) fn usage_line(u: &super::usage::Usage) -> String {
    let mut out = tokens(u.total);
    if u.unknown_runs > 0 {
        out.push_str(&format!(" ({} run(s) UNKNOWN)", u.unknown_runs));
    }
    if u.estimated > 0 {
        out.push_str(&format!(" + {} est.", u.estimated));
    }
    if let Some(d) = &u.dollars {
        out.push_str(&format!(
            " ~${} est., {}",
            super::usage::money(d.estimate),
            prices_label(d)
        ));
        if d.unpriced_tokens > 0 {
            out.push_str(&format!(", {} tok unpriced", d.unpriced_tokens));
        }
    }
    out
}

/// `prices of 2026-11-01`, and `STALE` when the table is old.
pub(super) fn prices_label(d: &super::usage::Dollars) -> String {
    match d.stale_days {
        Some(days) => format!("prices of {} (STALE, {days} days old)", d.as_of),
        None => format!("prices of {}", d.as_of),
    }
}

fn lease(c: &ClaimRow, now: u64) -> String {
    match c.lease_until_unix {
        None => "none (a lock from before leases)".to_owned(),
        Some(end) if now >= end => format!("expired {} ago", span(now - end)),
        Some(end) => format!("{} left", span(end - now)),
    }
}

fn renewed(c: &ClaimRow, now: u64) -> String {
    c.renewed_unix.map_or_else(
        || "UNKNOWN".to_owned(),
        |at| format!("{} ago", span(now.saturating_sub(at))),
    )
}

fn notice_state(told: bool, seen: bool) -> &'static str {
    match (told, seen) {
        (_, true) => "seen",
        (true, false) => "told",
        (false, false) => "posted",
    }
}

/// `3 live, 1 expired`, from the leases themselves.
fn lease_counts(m: &Model) -> String {
    let expired = m
        .snap
        .claims
        .iter()
        .filter(|c| c.lease_until_unix.is_some_and(|end| m.now_unix >= end))
        .count();
    let unread = m.snap.claims.iter().filter(|c| c.actor.is_none()).count();
    let live = m.snap.claims.len() - expired - unread;
    let mut out = format!("{live} live");
    if expired > 0 {
        out.push_str(&format!(", {expired} expired"));
    }
    if unread > 0 {
        out.push_str(&format!(", {unread} UNREADABLE"));
    }
    out
}

fn fraction(of: Option<(usize, usize)>) -> String {
    of.map_or_else(|| "UNKNOWN".to_owned(), |(a, b)| format!("{a}/{b}"))
}

fn vitals(m: &Model) -> Vec<Line<'static>> {
    let s = &m.snap;
    let label = |t: &str| Span::styled(t.to_owned(), dim());
    let value = |t: String| Span::styled(t, Style::default().add_modifier(Modifier::BOLD));
    let herdr = match &m.live.herdr {
        Herdr::Unasked => Span::styled("herdr: asking", dim()),
        Herdr::Absent { .. } => Span::styled("herdr: not running", dim()),
        Herdr::Connected { version } => Span::styled(
            format!(
                "herdr: {version}, {} here of {} agent(s)",
                m.agents_here,
                m.live.agents.len()
            ),
            Style::default().fg(Color::Green),
        ),
    };
    let collisions = if s.collisions.is_empty() {
        Span::raw("0")
    } else {
        Span::styled(
            s.collisions.len().to_string(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )
    };
    let top = Line::from(vec![
        label("TOKENS "),
        value(usage_line(&s.usage)),
        label("  CLAIMS "),
        value(s.claims.len().to_string()),
        label(" ("),
        Span::raw(lease_counts(m)),
        label(")  COLLISIONS "),
        collisions,
        label("  WORKTREES "),
        value(
            s.worktrees
                .as_ref()
                .map_or_else(|| "UNKNOWN".to_owned(), |w| w.len().to_string()),
        ),
        label("  SIGN "),
        value(s.queue.len().to_string()),
        label("  "),
        herdr,
    ]);
    let items = s.progress.items.map_or_else(
        || "UNKNOWN".to_owned(),
        |(open, active, done)| format!("{open} open, {active} active, {done} done"),
    );
    let bottom = Line::from(vec![
        label("WARRANTS "),
        value(format!("{} resolved", fraction(s.progress.warrants))),
        label("  PHASES "),
        value(format!("{} achieved", fraction(s.progress.phases))),
        label("  ITEMS "),
        value(items),
        label("  CRITICAL PATH "),
        value(s.critical_length.as_ref().map_or_else(
            || "none open".to_owned(),
            |l| format!("{} node(s), est. {l}", s.critical_path.len()),
        )),
    ]);
    vec![top, bottom]
}

fn work_items<'a>(m: &'a Model) -> Vec<ListItem<'a>> {
    let mut items: Vec<ListItem> = m
        .snap
        .faults
        .iter()
        .map(|f| {
            ListItem::new(Line::from(Span::styled(
                format!("UNKNOWN {f}"),
                Style::default().fg(Color::Red),
            )))
        })
        .collect();
    for w in visible_work(m) {
        let (badge, style) = state_badge(&w.state);
        let mut spans = vec![
            Span::raw(if w.depth > 0 { "  " } else { "" }),
            Span::styled(badge, style),
            Span::styled(
                format!("{} ", w.id.rsplit('/').next().unwrap_or(&w.id)),
                if w.critical {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Cyan)
                },
            ),
        ];
        if w.critical {
            spans.push(Span::styled("* ", Style::default().fg(Color::Yellow)));
        }
        if !w.level.is_empty() && w.level != "Ticket" {
            spans.push(Span::styled(format!("{} ", w.level), dim()));
        }
        spans.push(Span::raw(w.title.clone()));
        if let Some(holder) = &w.holder {
            spans.push(Span::styled(
                format!(" [{holder}]"),
                Style::default().fg(Color::Cyan),
            ));
        }
        if let Count::Known(n) = w.tokens {
            spans.push(Span::styled(format!(" · {n} tok"), dim()));
        }
        items.push(ListItem::new(Line::from(spans)));
    }
    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            if m.snap.loaded {
                "no open work"
            } else {
                "reading the records…"
            },
            dim(),
        ))));
    }
    items
}

fn phase_lines(m: &Model) -> Vec<Line<'static>> {
    m.snap
        .phases
        .iter()
        .map(|p| {
            let unknown = if p.unknown > 0 {
                format!(", {} UNKNOWN", p.unknown)
            } else {
                String::new()
            };
            Line::from(vec![
                Span::styled(format!("{} ", p.id), Style::default().fg(Color::Cyan)),
                Span::raw(format!("{}/{}{unknown} ", p.done, p.total)),
                Span::styled(p.achieved.clone(), dim()),
            ])
        })
        .collect()
}

fn claim_items(m: &Model) -> Vec<ListItem<'static>> {
    let mut items = Vec::new();
    for c in visible_claims(m) {
        let collided = m
            .snap
            .collisions
            .iter()
            .any(|x| x.a == c.target || x.b == c.target);
        let Some(actor) = c.actor.clone() else {
            items.push(ListItem::new(Line::from(Span::styled(
                format!("UNREADABLE lock {}", c.target),
                Style::default().fg(Color::Red),
            ))));
            continue;
        };
        let expired = c.lease_until_unix.is_some_and(|end| m.now_unix >= end);
        let workspace = c
            .workspace
            .as_deref()
            .map_or("UNKNOWN", |w| w.rsplit('/').next().unwrap_or(w))
            .to_owned();
        let mut first = vec![
            Span::styled(format!("{actor} "), Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} ", c.target)),
            Span::styled(span(m.now_unix.saturating_sub(c.since_unix)), dim()),
        ];
        // Recorded tokens, when there are any; the detail says UNKNOWN.
        if let Count::Known(n) = m.snap.usage.of(&c.target) {
            first.push(Span::styled(format!(" · {n} tok"), dim()));
        }
        if collided {
            first.push(Span::styled(
                " COLLISION",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ));
        }
        let second = vec![
            Span::styled(
                format!("  lease {} · ", lease(c, m.now_unix)),
                if expired {
                    Style::default().fg(Color::Yellow)
                } else {
                    dim()
                },
            ),
            Span::styled(format!("lock renewed {} · ", renewed(c, m.now_unix)), dim()),
            Span::raw(workspace),
        ];
        let mut lines = vec![Line::from(first), Line::from(second)];
        if !c.scope.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("  writes {}", c.scope.join(", ")),
                dim(),
            )));
        }
        items.push(ListItem::new(lines));
    }
    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "no live claims",
            dim(),
        ))));
    }
    for (target, n, at) in recent_usage(m, 3) {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("recent {target} · {n} tok recorded {at}"),
            dim(),
        ))));
    }
    items
}

/// The most recently recorded session usage of work no live claim holds:
/// its target, tokens and day, newest first.
fn recent_usage(m: &Model, most: usize) -> Vec<(String, u64, String)> {
    let mut latest: std::collections::BTreeMap<&str, (u64, &str)> =
        std::collections::BTreeMap::new();
    for c in &m.snap.usage.counted {
        if c.run.at.is_empty() || m.snap.claims.iter().any(|x| x.target == c.run.item) {
            continue;
        }
        let e = latest.entry(c.run.item.as_str()).or_insert((0, ""));
        e.0 = e.0.saturating_add(c.run.tokens.unwrap_or(0));
        if c.run.at.as_str() > e.1 {
            e.1 = c.run.at.as_str();
        }
    }
    let mut rows: Vec<(String, u64, String)> = latest
        .into_iter()
        .map(|(t, (n, at))| (t.to_owned(), n, at.to_owned()))
        .collect();
    rows.sort_by(|a, b| b.2.cmp(&a.2));
    rows.truncate(most);
    rows.into_iter()
        .map(|(t, n, at)| (t, n, at.get(..10).unwrap_or("UNKNOWN").to_owned()))
        .collect()
}

fn live_title(m: &Model) -> String {
    let title = live_source(m);
    if searching(m) {
        let n = m.live.text.lines().filter(|l| m.passes(l)).count();
        return format!("{title}· {n} line(s) match {} ", m.filter);
    }
    title
}

fn live_source(m: &Model) -> String {
    match (&m.live.herdr, m.focused_agent()) {
        (_, Some(a)) => format!(
            " {} {} ({}/{})  [ ] agents ",
            a.status_badge(),
            a.display_label(),
            m.agent_sel.min(m.live.agents.len() - 1) + 1,
            m.live.agents.len()
        ),
        (Herdr::Connected { .. }, None) => " Live: no agent in Herdr; working tree ".to_owned(),
        (Herdr::Unasked, None) => " Live ".to_owned(),
        (Herdr::Absent { .. }, None) => " Live: working tree and journal ".to_owned(),
    }
}

/// Whether `/` is searching the transcript rather than filtering lists.
fn searching(m: &Model) -> bool {
    m.panel() == Panel::Live && !m.filter.is_empty()
}

fn live_lines(m: &Model, height: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<String> = m.live.text.lines().map(str::to_owned).collect();
    if searching(m) {
        lines.retain(|l| m.passes(l));
        if lines.is_empty() {
            lines.push(format!("nothing in the transcript matches {}", m.filter));
        }
    }
    if m.focused_agent().is_none() && !m.snap.progress_lines.is_empty() && !searching(m) {
        lines.push("journal".to_owned());
        lines.extend(m.snap.progress_lines.iter().cloned());
    }
    let tail = if m.focused_agent().is_some() {
        lines.len().saturating_sub(height)
    } else {
        0
    };
    lines[tail..]
        .iter()
        .map(|l| Line::from(l.clone()))
        .collect()
}

fn question_items(m: &Model) -> Vec<ListItem<'static>> {
    let mut items: Vec<ListItem> = visible_questions(m)
        .iter()
        .map(|q| {
            ListItem::new(Line::from(vec![
                if q.blocking {
                    Span::styled(
                        "BLOCKING ",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled("question ", Style::default().fg(Color::Yellow))
                },
                Span::styled(
                    format!("{}: ", q.target()),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(q.question.clone()),
            ]))
        })
        .collect();
    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "no open questions",
            dim(),
        ))));
    }
    items
}

fn notice_items(m: &Model) -> Vec<ListItem<'static>> {
    let mut items: Vec<ListItem> = visible_notices(m)
        .iter()
        .map(|n| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<6} ", notice_state(n.told, n.seen)),
                    Style::default().fg(if n.seen { Color::Green } else { Color::Yellow }),
                ),
                Span::styled(format!("{} ", n.actor), Style::default().fg(Color::Cyan)),
                Span::raw(n.line.clone()),
            ]))
        })
        .collect();
    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled("no notices", dim()))));
    }
    items
}

fn panel_list(
    f: &mut ratatui::Frame,
    m: &Model,
    area: Rect,
    panel: Panel,
    title: String,
    items: Vec<ListItem<'_>>,
    offset: usize,
) {
    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some((m.selected_in(panel) + offset).min(items.len() - 1)));
    }
    f.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(focused(m.panel() == panel)),
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
    let [vitals_area, main, drawer] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(6),
        Constraint::Length(7),
    ])
    .areas(body);
    f.render_widget(
        Paragraph::new(vitals(m)).block(Block::default().borders(Borders::ALL).title(" Watch ")),
        vitals_area,
    );
    let [left, center, right] = Layout::horizontal([
        Constraint::Percentage(36),
        Constraint::Percentage(34),
        Constraint::Percentage(30),
    ])
    .areas(main);

    let phases = phase_lines(m);
    let phase_height = u16::try_from(phases.len().min(6)).unwrap_or(6);
    let [phase_area, work_area] = if phases.is_empty() {
        [Rect::default(), left]
    } else {
        Layout::vertical([Constraint::Length(phase_height + 2), Constraint::Min(3)]).areas(left)
    };
    if !phases.is_empty() {
        f.render_widget(
            Paragraph::new(phases).block(Block::default().borders(Borders::ALL).title(" Phases ")),
            phase_area,
        );
    }
    let faults = m.snap.faults.len();
    panel_list(
        f,
        m,
        work_area,
        Panel::Work,
        " Work  (* critical path) ".to_owned(),
        work_items(m),
        faults,
    );
    panel_list(
        f,
        m,
        center,
        Panel::Claims,
        " Claims ".to_owned(),
        claim_items(m),
        0,
    );
    f.render_widget(
        Paragraph::new(live_lines(m, usize::from(right.height.saturating_sub(2)))).block(
            Block::default()
                .borders(Borders::ALL)
                .title(live_title(m))
                .border_style(focused(m.panel() == Panel::Live)),
        ),
        right,
    );
    let [questions, notices] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(drawer);
    panel_list(
        f,
        m,
        questions,
        Panel::Questions,
        " Questions ".to_owned(),
        question_items(m),
        0,
    );
    panel_list(
        f,
        m,
        notices,
        Panel::Notices,
        " Notices ".to_owned(),
        notice_items(m),
        0,
    );
}
