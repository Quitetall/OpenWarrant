// SPDX-License-Identifier: Apache-2.0
//! Records: the lists `3` calls up over either mode, and Repos, which `p`
//! calls up from anywhere. Each row reads the function the CLI answers with;
//! a list that computed a fact itself would be a second answer.

use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use super::argv;
use super::md;
use super::model::{Effect, Model, Overlay, Row, View};
use crate::diagnostic::{Diagnostic, Report, Severity};

/// The queue: `war sign --list`, through the console's board.
#[must_use]
pub fn queue_rows(board: &crate::console::Board) -> Vec<Row> {
    board
        .acts
        .iter()
        .map(|a| Row {
            text: format!("[{}] {}", a.n, a.line),
            command: a.command.clone(),
            sign_target: Some(a.target.clone()),
            ..Row::default()
        })
        .collect()
}

/// The frontier: every stage with its state, and the command that starts it.
#[must_use]
pub fn frontier_rows(
    frontier: &crate::frontier::Frontier,
    board: Option<&crate::console::Board>,
) -> Vec<Row> {
    let mut stage_cmd: BTreeMap<(&str, &str), &str> = BTreeMap::new();
    for s in board.iter().flat_map(|b| &b.stages) {
        stage_cmd.insert((&s.warrant, &s.stage), &s.command);
    }
    frontier
        .rows
        .iter()
        .map(|r| Row {
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
            command: stage_cmd
                .get(&(r.warrant.as_str(), r.stage.as_str()))
                .map_or_else(
                    || format!("war plan frontier {}", r.warrant),
                    |c| (*c).to_owned(),
                ),
            ..Row::default()
        })
        .collect()
}

/// The corpus, obligations and evidence: one `status::build`.
#[must_use]
pub fn status_rows(status: &openwarrant_core::status::CorpusStatus) -> [Vec<Row>; 3] {
    use openwarrant_core::resolution::RequirementState as R;
    let mut corpus = Vec::new();
    let mut obligations = Vec::new();
    let mut evidence = Vec::new();
    let none_or = |list: &[String]| {
        if list.is_empty() {
            "none".to_owned()
        } else {
            list.join(", ")
        }
    };
    for w in &status.warrants {
        let thirteen = w.checks.map_or_else(
            || "the thirteen: not established (the Warrant could not be evaluated)".to_owned(),
            |c| {
                c.states()
                    .iter()
                    .map(|(name, state)| match state {
                        R::Met => format!("✓ {name}"),
                        R::Unmet => format!("✗ {name}"),
                        R::NotApplicable(cap) => {
                            format!("– {name} (not applicable: no `{cap}` capability)")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            },
        );
        let title = w.title.as_deref().unwrap_or("");
        corpus.push(Row {
            text: format!(
                "{:<16} {:<12} {title}{}",
                w.alias,
                format!("{:?}", w.rung).to_lowercase(),
                if w.unmet.is_empty() {
                    String::new()
                } else {
                    format!("  ({} unmet)", w.unmet.len())
                }
            ),
            command: format!("war sign resolve {} --dry-run", w.alias),
            detail: Some(format!(
                "{} — {title}\nrung: {:?}\n\n{thirteen}\n\nunestablished: {}\nblocking unknowns: {}",
                w.alias,
                w.rung,
                none_or(&w.unestablished),
                none_or(&w.blocking_unknowns),
            )),
            ..Row::default()
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
                ..Row::default()
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
                auto: Some(argv::record_gate(&w.alias, &g.gate)),
                detail: g.receipt_ref.as_ref().map(|r| format!("receipt: {r}")),
                ..Row::default()
            });
        }
    }
    evidence.sort_by(|a, b| a.text.cmp(&b.text));
    [corpus, obligations, evidence]
}

/// The journal: every Warrant's events, newest first.
#[must_use]
pub fn journal_rows(repo: &crate::repo::Repository) -> Vec<Row> {
    let mut journal = Vec::new();
    for dir in repo.warrant_dirs().unwrap_or_default() {
        let alias = dir.file_name().unwrap_or("?").to_owned();
        match crate::journal_cmd::load(&dir) {
            Ok(j) => journal.extend(j.events.iter().map(|e| Row {
                text: format!(
                    "{} {:<14} {:<28} {}",
                    e.occurred_at, alias, e.event_type, e.actor_ref
                ),
                command: format!("war admin journal {alias}"),
                detail: Some(format!(
                    "{}\n{}\n\n{}",
                    e.occurred_at, e.event_type, e.payload
                )),
                ..Row::default()
            })),
            Err(e) => journal.push(Row {
                text: format!("UNREADABLE {alias}: {e}"),
                command: format!("war admin journal {alias}"),
                ..Row::default()
            }),
        }
    }
    journal.sort_by(|a, b| b.text.cmp(&a.text));
    journal
}

/// The roadmap: phases in dependency order, from `war plan roadmap`'s view.
#[must_use]
pub fn roadmap_rows(view: &crate::roadmap_cmd::View) -> Vec<Row> {
    let mut roadmap: Vec<Row> = view
        .phases
        .iter()
        .map(|p| Row {
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
            ..Row::default()
        })
        .collect();
    if !view.accepted {
        roadmap.insert(
            0,
            Row {
                text: match view.pending_revision {
                    Some(n) => format!("HUMAN  roadmap revision {n} awaits one signature"),
                    None => "WARN   the roadmap atoms are not an accepted revision".to_owned(),
                },
                command: "war sign roadmap --ssh-sign".to_owned(),
                sign_target: view.pending_revision.map(|_| "roadmap".to_owned()),
                ..Row::default()
            },
        );
    }
    roadmap
}

/// What next: the Setup step if incomplete, then `war view next`'s actions
/// (human first), then doctor's non-passes, then `war check`'s errors deduped
/// by rule with counts, each with its remedy.
#[must_use]
pub fn help_rows(
    setup: Option<(&str, &str)>,
    next: Option<&crate::next::Next>,
    doctor: Option<&Report>,
    check: Option<&Report>,
) -> Vec<Row> {
    let mut rows = Vec::new();
    if let Some((title, question)) = setup {
        rows.push(Row {
            text: format!("setup: {title} — press Enter in the Setup view"),
            command: "war init".to_owned(),
            detail: Some(question.to_owned()),
            ..Row::default()
        });
    }
    if let Some(n) = next {
        // t-67ed: ready ticket items first, as `war view next` lists them.
        // Shown, never run from here: no auto, no signature.
        for r in &n.ready {
            rows.push(Row {
                text: format!("ready claim      {} — {}", r.ticket, r.text),
                command: r.command.clone(),
                ..Row::default()
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
                auto: (a.actor == crate::next::Actor::Agent && !a.command.starts_with("war sign"))
                    .then(|| argv::of_action(&a.command))
                    .flatten(),
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
                ..Row::default()
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
    if rows.is_empty() {
        rows.push(Row {
            text: "nothing pending, nothing red — `war view next` agrees".to_owned(),
            command: "war view next".to_owned(),
            ..Row::default()
        });
    }
    rows
}

/// This binary's identity and path, and a warning when `war` on PATH is a
/// different build — the 2026-09-23 failure, where a command handed over
/// ran an older `war` than the one that drafted it.
///
/// OW-WAR-0143: the comparison is the whole `--version` line, not its last
/// word. Two builds of one version (a release and a checkout 62 commits past
/// it) share the version and differ in the line; only the line tells them
/// apart. The remedy is `install::remedy`, the command for this install.
#[must_use]
pub fn binary_row() -> Row {
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
        detail: Some(detail),
        ..Row::default()
    }
}

pub fn remedy_row(diag: &Diagnostic, n: usize) -> Row {
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

/// The Tickets view from `ticket::board`: one row per ticket (state,
/// progress, priority, title; Enter shows `war view show`'s checklist and
/// notes), then one row per REMAINING item beneath it. Done items are in the
/// detail, not the list. No row carries a signature or an auto remedy.
#[must_use]
pub fn ticket_rows(board: &serde_json::Value) -> Vec<Row> {
    let mut rows = Vec::new();
    let list = board["tickets"].as_array().cloned().unwrap_or_default();
    if list.is_empty() {
        rows.push(Row {
            text: "no tickets yet — `war create \"what this work accomplishes\"`".to_owned(),
            command: "war create \"...\" --item \"...\"".to_owned(),
            ..Row::default()
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
            detail: detail.clone(),
            ..Row::default()
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
                detail: detail.clone(),
                ..Row::default()
            });
        }
    }
    rows
}

// ---- Repos ----------------------------------------------------------------

/// Everyone on the list, from the list alone; facts arrive from the reader.
pub fn set_repos(
    m: &mut Model,
    rows: Vec<(crate::projects::Tier, crate::projects::Row)>,
    hidden: usize,
) {
    (m.repos.tiers, m.repos.projects) = rows.into_iter().unzip();
    m.repos.hidden = hidden;
    if m.repos.projects.is_empty() {
        m.message =
            "no repositories yet: run any `war` command inside one and it is remembered".to_owned();
    }
    render_repos(m);
    let here = here(m);
    m.repos_sel = m
        .repos
        .projects
        .iter()
        .position(|r| here.as_deref() == Some(r.root.as_str()))
        .unwrap_or(0);
}

fn here(m: &Model) -> Option<String> {
    m.root.canonicalize_utf8().ok().map(String::from)
}

pub fn render_repos(m: &mut Model) {
    let here = here(m);
    let here = |root: &str| here.as_deref() == Some(root);
    let mut rows = Vec::new();
    for (r, tier) in m.repos.projects.iter().zip(&m.repos.tiers) {
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
            detail: Some(if r.missing {
                format!(
                    "{} holds no openwarrant.toml any more.\n\n`war admin projects --forget {}` removes it from the list.",
                    r.root, r.root
                )
            } else {
                format!("{}\n\n{facts}", r.root)
            }),
            ..Row::default()
        });
    }
    if m.repos.hidden > 0 {
        rows.push(Row {
            text: format!(
                "  {} more hidden: gone, scratch or linked worktrees",
                m.repos.hidden
            ),
            command: "war admin projects".to_owned(),
            detail: Some(
                "● has Warrants or tickets, ○ does not. `war admin projects` lists every \
                 entry, and `--forget <path>` drops one."
                    .to_owned(),
            ),
            ..Row::default()
        });
    }
    for (text, command, detail) in [
        (
            "+ a new repository in another directory (n)".to_owned(),
            "war init --root <directory>".to_owned(),
            "`n` asks for a directory, creates it if needed, and runs `war init` there.",
        ),
        (
            format!("+ set up {}", m.root),
            "war init".to_owned(),
            "Setup runs `war init` in this directory.",
        ),
    ] {
        rows.push(Row {
            text,
            command,
            detail: Some(detail.to_owned()),
            ..Row::default()
        });
    }
    m.repos.rows = rows;
}

// ---- rows of what is showing ----------------------------------------------

/// The rows of the overlay showing, or nothing.
#[must_use]
pub fn rows(m: &Model) -> &[Row] {
    match m.overlay {
        // Before the first read, Help still knows which binary this is.
        Overlay::Records(View::Help) if !m.snap.loaded => std::slice::from_ref(&m.binary),
        Overlay::Records(v) => m.snap.views.get(&v).map_or(&[], Vec::as_slice),
        Overlay::Repos => &m.repos.rows,
        Overlay::None => &[],
    }
}

/// The rows the filter keeps, with their original indices.
#[must_use]
pub fn visible(m: &Model) -> Vec<(usize, &Row)> {
    rows(m)
        .iter()
        .enumerate()
        .filter(|(_, r)| m.passes(&r.text))
        .collect()
}

#[must_use]
pub fn selected(m: &Model) -> usize {
    match m.overlay {
        Overlay::Records(v) => m.view_sel.get(&v).copied().unwrap_or(0),
        Overlay::Repos => m.repos_sel,
        Overlay::None => 0,
    }
}

pub fn select(m: &mut Model, at: usize) {
    match m.overlay {
        Overlay::Records(v) => {
            m.view_sel.insert(v, at);
        }
        Overlay::Repos => m.repos_sel = at,
        Overlay::None => {}
    }
}

#[must_use]
pub fn current(m: &Model) -> Option<&Row> {
    let v = visible(m);
    v.get(selected(m).min(v.len().saturating_sub(1)))
        .map(|(_, r)| *r)
}

fn current_index(m: &Model) -> Option<usize> {
    let v = visible(m);
    v.get(selected(m).min(v.len().saturating_sub(1)))
        .map(|(i, _)| *i)
}

/// `v` on a row: the request behind a signature, or the row's detail.
pub fn show(m: &mut Model) -> Option<Effect> {
    if let Some(t) = current(m).and_then(|r| r.sign_target.clone()) {
        return Some(Effect::Capture {
            title: format!("war sign {t} --show"),
            argv: argv::sign_show(&t),
        });
    }
    if let Some(r) = current(m) {
        m.popup = Some((
            r.command.clone(),
            r.detail.clone().unwrap_or_else(|| r.text.clone()),
            0,
        ));
    }
    None
}

/// `x` on a row: its auto remedy, off the main thread. Never a signature.
pub fn remedy(m: &mut Model) -> Option<Effect> {
    let Some(argv) = current(m).and_then(|r| r.auto.clone()) else {
        m.message = "no auto remedy on this row".to_owned();
        return None;
    };
    if argv.first().is_some_and(|a| a == "war") {
        return Some(Effect::Capture {
            title: argv.join(" "),
            argv: argv[1..].to_vec(),
        });
    }
    m.popup = Some((
        "not run from here".to_owned(),
        format!(
            "{}\n\nThis remedy is not a `war` command; run it in a shell.",
            argv.join(" ")
        ),
        0,
    ));
    None
}

/// A key in Records or Repos.
pub fn key(m: &mut Model, k: KeyEvent) -> Option<Effect> {
    let n = visible(m).len();
    let view = match m.overlay {
        Overlay::Records(v) => Some(v),
        _ => None,
    };
    let reading = m.reading && view == Some(View::Help);
    let docs = m.docs.len().max(1);
    match k.code {
        KeyCode::Char('j') | KeyCode::Down if reading => m.doc_index = (m.doc_index + 1) % docs,
        KeyCode::Char('k') | KeyCode::Up if reading => {
            m.doc_index = (m.doc_index + docs - 1) % docs;
        }
        KeyCode::Char(']') if reading => m.doc_index = (m.doc_index + 1) % docs,
        KeyCode::Char('[') if reading => m.doc_index = (m.doc_index + docs - 1) % docs,
        KeyCode::Char('j') | KeyCode::Down if n > 0 => {
            select(m, (selected(m) + 1).min(n - 1));
        }
        KeyCode::Char('k') | KeyCode::Up => select(m, selected(m).saturating_sub(1)),
        KeyCode::PageDown if n > 0 => select(m, (selected(m) + 20).min(n - 1)),
        KeyCode::PageUp => select(m, selected(m).saturating_sub(20)),
        KeyCode::Char(c @ ('[' | ']')) => {
            if let Some(v) = view {
                let next = v.stepped(if c == ']' { 1 } else { -1 });
                m.overlay = Overlay::Records(next);
                m.last_view = next;
            }
        }
        KeyCode::Char('d') if view == Some(View::Help) => m.reading = !m.reading,
        KeyCode::Char('v') => return show(m),
        KeyCode::Char('x') => return remedy(m),
        KeyCode::Char('c') => {
            return Some(Effect::Capture {
                title: "commit message (`war admin commit --write`)".to_owned(),
                argv: argv::commit_message(),
            });
        }
        KeyCode::Char('n') if m.overlay == Overlay::Repos => m.new_dir = Some(String::new()),
        KeyCode::Enter => return enter(m, view),
        _ => {}
    }
    None
}

/// `Enter` on a row of what is showing.
fn enter(m: &mut Model, view: Option<View>) -> Option<Effect> {
    match view {
        Some(View::Setup) => Some(Effect::Terminal {
            argv: argv::init(),
            root: None,
        }),
        // One editor, two doors: the view hands the terminal to
        // `war plan roadmap edit`, which ends in one signature.
        Some(View::Roadmap) => match current(m).and_then(|r| r.sign_target.clone()) {
            Some(t) => Some(Effect::Terminal {
                argv: super::sign_argv(&t),
                root: None,
            }),
            None => Some(Effect::Terminal {
                argv: argv::roadmap_edit(),
                root: None,
            }),
        },
        None => enter_repo(m),
        _ => {
            if let Some(t) = current(m).and_then(|r| r.sign_target.clone()) {
                return Some(Effect::Terminal {
                    argv: super::sign_argv(&t),
                    root: None,
                });
            }
            if let Some(r) = current(m) {
                m.popup = Some((
                    r.command.clone(),
                    r.detail.clone().unwrap_or_else(|| r.command.clone()),
                    0,
                ));
            }
            None
        }
    }
}

fn enter_repo(m: &mut Model) -> Option<Effect> {
    let total = rows(m).len();
    match current_index(m) {
        Some(i) if m.repos.projects.get(i).is_some_and(|r| !r.missing) => {
            Some(Effect::Open(Utf8PathBuf::from(&m.repos.projects[i].root)))
        }
        Some(i) if i + 2 == total => {
            m.new_dir = Some(String::new());
            None
        }
        Some(i) if i + 1 == total => {
            m.overlay = Overlay::Records(View::Setup);
            None
        }
        _ => {
            if let Some(r) = current(m) {
                m.popup = Some((r.command.clone(), r.detail.clone().unwrap_or_default(), 0));
            }
            None
        }
    }
}

// ---- drawing --------------------------------------------------------------

fn list_title(m: &Model) -> String {
    match m.overlay {
        Overlay::Repos => {
            " Repos — every repository you use; Enter opens, n starts a new one, Esc back "
                .to_owned()
        }
        Overlay::Records(v) => {
            let position = format!(
                "{}/{}",
                View::ALL.iter().position(|x| *x == v).unwrap_or(0) + 1,
                View::ALL.len()
            );
            let what = match v {
                View::Tickets => {
                    "state and progress, remaining items; Enter shows checklist and notes"
                        .to_owned()
                }
                View::Frontier => "stages by state; the command starts one".to_owned(),
                View::Corpus => "rungs; Enter shows the thirteen".to_owned(),
                View::Obligations => "disposition, verifier, admissibility".to_owned(),
                View::Evidence => "gate runs by class; x records again".to_owned(),
                View::Journal => "newest first; / filters by day or kind".to_owned(),
                View::Roadmap => "phases in order; Enter edits and signs once".to_owned(),
                View::Setup => format!("{}; Enter runs war init", m.snap.setup_step),
                View::Help => "what next (d: documents)".to_owned(),
            };
            format!(" {} — {what}  [ ] {position} · Esc back ", v.name())
        }
        Overlay::None => String::new(),
    }
}

pub fn draw_doc(f: &mut ratatui::Frame, m: &Model, body: Rect) {
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

pub fn draw(f: &mut ratatui::Frame, m: &Model, body: Rect) {
    if m.reading && m.overlay == Overlay::Records(View::Help) {
        draw_doc(f, m, body);
        return;
    }
    let items: Vec<ListItem> = visible(m)
        .iter()
        .map(|(_, r)| ListItem::new(Line::from(row_span(r))))
        .collect();
    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some(selected(m).min(items.len() - 1)));
    }
    let empty = items.is_empty();
    f.render_stateful_widget(
        List::new(items)
            .block(Block::default().borders(Borders::ALL).title(list_title(m)))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("▶ "),
        body,
        &mut state,
    );
    if empty {
        let inner = Rect {
            x: body.x + 2,
            y: body.y + 1,
            width: body.width.saturating_sub(4),
            height: 1,
        };
        f.render_widget(
            Paragraph::new(if m.overlay == Overlay::Repos {
                "reading the list…"
            } else if !m.snap.loaded {
                "reading the records…"
            } else if m.filter.is_empty() {
                "nothing here — not established, not blank"
            } else {
                "nothing matches the filter"
            })
            .style(Style::default().fg(Color::DarkGray)),
            inner,
        );
    }
}

fn row_span(r: &Row) -> Span<'static> {
    let style = if r.text.starts_with("ERROR")
        || r.text.starts_with("UNREADABLE")
        || r.text.starts_with("UNKNOWN")
    {
        Style::default().fg(Color::Red)
    } else if r.text.starts_with("WARN") || r.text.starts_with("BLOCKING") {
        Style::default().fg(Color::Yellow)
    } else if r.text.starts_with("HUMAN") {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    Span::styled(r.text.clone(), style)
}
