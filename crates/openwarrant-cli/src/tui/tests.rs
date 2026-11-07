// SPDX-License-Identifier: Apache-2.0
//! The app driven as a person drives it: keys into `handle_key`, a frame out
//! of `draw`, over a fixture no file stands behind.

use super::model::{ClaimRow, Collision, Count, NoticeRow, PhaseRow, QuestionItem, ReadyRow, Row, WorkRow};
use super::*;
use crate::diagnostic::Diagnostic;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// The moment the fixture's clock stands at.
const NOW: u64 = 2_000_000_000;

fn key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn queue_row(target: &str) -> Row {
    Row {
        text: format!("authorize {target}"),
        command: format!("war sign {target} --ssh-sign"),
        sign_target: Some(target.to_owned()),
        ..Row::default()
    }
}

fn fixture() -> Model {
    let mut m = Model::new("/work/repo".into(), true, Row::default(), Vec::new());
    m.now_unix = NOW;
    let mut views = std::collections::BTreeMap::new();
    views.insert(
        View::Tickets,
        vec![Row {
            text: "open t-1a2b3 0/2 p1 Reset passwords".to_owned(),
            command: "war view show t-1a2b3".to_owned(),
            detail: Some("# t-1a2b3".to_owned()),
            ..Row::default()
        }],
    );
    views.insert(
        View::Evidence,
        vec![Row {
            text: "machine OW-WAR-0001 ops.check@1.0.0 run-1 pass".to_owned(),
            command: "war evidence record OW-WAR-0001 --gate ops.check@1.0.0".to_owned(),
            auto: Some(argv::record_gate("OW-WAR-0001", "ops.check@1.0.0")),
            ..Row::default()
        }],
    );
    m.apply(model::Snapshot {
        loaded: true,
        repository: true,
        setup_done: true,
        program: "fixture".to_owned(),
        sas_in_force: "1.1.0".to_owned(),
        views,
        queue: vec![queue_row("authorize:OW-WAR-0001"), queue_row("resolve:OW-WAR-0002")],
        questions: vec![QuestionItem {
            warrant: "OW-WAR-0001".to_owned(),
            id: "Q-001".to_owned(),
            question: "Which queue?".to_owned(),
            recommended: "the fast one".to_owned(),
            blocking: true,
            command: "war plan answer OW-WAR-0001 Q-001 \"...\" --as <you>".to_owned(),
        }],
        phases: vec![PhaseRow {
            id: "P1".to_owned(),
            title: "Foundations".to_owned(),
            achieved: "not yet".to_owned(),
            done: 2,
            total: 3,
            unknown: 1,
        }],
        work: vec![
            WorkRow {
                id: "t-1a2b3".to_owned(),
                title: "Reset passwords".to_owned(),
                state: "in_progress".to_owned(),
                level: "Ticket".to_owned(),
                critical: true,
                ..WorkRow::default()
            },
            WorkRow {
                id: "t-1a2b3/i-1".to_owned(),
                title: "Email template".to_owned(),
                state: "running".to_owned(),
                depth: 1,
                holder: Some("claude".to_owned()),
                ..WorkRow::default()
            },
        ],
        ready: vec![ReadyRow {
            id: "t-1a2b3/i-2".to_owned(),
            title: "Docs".to_owned(),
            critical: true,
        }],
        claims: vec![
            ClaimRow {
                target: "t-1a2b3/i-1".to_owned(),
                actor: Some("claude".to_owned()),
                since_unix: NOW - 125,
                lease_until_unix: Some(NOW + 222),
                renewed_unix: Some(NOW - 4),
                workspace: Some("/work/wt-a".to_owned()),
                scope: vec!["src/a/**".to_owned()],
            },
            ClaimRow {
                target: "t-9/i-1".to_owned(),
                actor: Some("codex".to_owned()),
                since_unix: NOW - 3700,
                lease_until_unix: Some(NOW - 60),
                renewed_unix: None,
                workspace: None,
                scope: vec!["src/a/b.rs".to_owned()],
            },
        ],
        collisions: vec![Collision {
            a: "t-1a2b3/i-1".to_owned(),
            b: "t-9/i-1".to_owned(),
            why: "src/a/** overlaps src/a/b.rs".to_owned(),
        }],
        notices: vec![NoticeRow {
            ticket: "t-1a2b3".to_owned(),
            actor: "claude".to_owned(),
            line: "docs/x.md changed on main: run war start".to_owned(),
            at: "2033-05-18T03:33:20Z".to_owned(),
            told: true,
            seen: false,
        }],
        progress: model::Progress {
            warrants: Some((12, 20)),
            phases: Some((1, 4)),
            items: Some((3, 1, 9)),
        },
        critical_path: vec!["t-1a2b3/i-2".to_owned()],
        critical_length: Some("30m".to_owned()),
        worktrees: Some(vec!["/work/repo".to_owned(), "/work/wt-a".to_owned()]),
        tokens: Count::Unknown,
        ..model::Snapshot::default()
    });
    m
}

fn screen(m: &Model) -> String {
    frame(m, 150, 44)
}

// ---- the shape ------------------------------------------------------------

#[test]
fn tab_flips_and_digits_go_direct_and_no_key_differs_by_case_alone() {
    let mut m = fixture();
    assert_eq!(m.mode, Mode::Watch);
    handle_key(&mut m, press(KeyCode::Tab));
    assert_eq!(m.mode, Mode::Act);
    handle_key(&mut m, press(KeyCode::Tab));
    assert_eq!(m.mode, Mode::Watch);
    handle_key(&mut m, key('2'));
    assert_eq!(m.mode, Mode::Act);
    handle_key(&mut m, key('1'));
    assert_eq!(m.mode, Mode::Watch);
    // The table of keys binds nothing by case.
    let keys: Vec<&str> = KEYS.lines().filter_map(|l| l.split("  ").next()).collect();
    for k in keys.iter().flat_map(|k| k.split_whitespace()) {
        if k.len() == 1 {
            let other = if k == k.to_uppercase() {
                k.to_lowercase()
            } else {
                k.to_uppercase()
            };
            assert!(
                other == k || !keys.iter().any(|l| l.split_whitespace().any(|w| w == other)),
                "{k} and {other} are both keys"
            );
        }
    }
}

#[test]
fn records_comes_up_over_either_mode_and_goes_away_again() {
    let mut m = fixture();
    handle_key(&mut m, key('2'));
    handle_key(&mut m, key('3'));
    assert_eq!(m.overlay, Overlay::Records(View::Tickets));
    assert_eq!(m.mode, Mode::Act, "the mode underneath stays");
    handle_key(&mut m, key(']'));
    assert_eq!(m.overlay, Overlay::Records(View::Frontier));
    handle_key(&mut m, press(KeyCode::Esc));
    assert_eq!(m.overlay, Overlay::None);
    handle_key(&mut m, key('3'));
    assert_eq!(
        m.overlay,
        Overlay::Records(View::Frontier),
        "3 comes back to the view last read"
    );
    assert_eq!(handle_key(&mut m, key('p')), Some(Effect::ReadRepos));
    assert_eq!(m.overlay, Overlay::Repos);
}

#[test]
fn every_view_and_mode_draws_in_a_cramped_terminal_without_a_panic() {
    let mut m = fixture();
    for (w, h) in [(150, 44), (40, 12), (12, 4), (1, 1)] {
        for mode in [Mode::Watch, Mode::Act] {
            m.mode = mode;
            m.overlay = Overlay::None;
            let _ = frame(&m, w, h);
            for view in View::ALL {
                m.overlay = Overlay::Records(view);
                let _ = frame(&m, w, h);
            }
        }
    }
}

// ---- honest Watch ---------------------------------------------------------

#[test]
fn no_usage_recorded_reads_unknown_never_a_number() {
    let m = fixture();
    let s = screen(&m);
    assert!(s.contains("TOKENS UNKNOWN"), "{s}");
    assert!(
        !s.split('$').skip(1).any(|t| t.starts_with(|c: char| c.is_ascii_digit())),
        "no dollars: {s}"
    );
    assert!(!s.contains("% complete"), "no percentage: {s}");
    assert!(s.contains("12/20 resolved"), "{s}");
    assert!(s.contains("3 open, 1 active, 9 done"), "{s}");
}

#[test]
fn a_recorded_count_is_shown_as_it_is() {
    let mut m = fixture();
    m.snap.tokens = Count::Known(4_242);
    let s = screen(&m);
    assert!(s.contains("TOKENS 4242"), "{s}");
    assert!(!s.contains("TOKENS UNKNOWN"), "{s}");
}

#[test]
fn claims_say_who_how_long_what_lease_and_where_and_name_the_renewal() {
    let m = fixture();
    let s = screen(&m);
    assert!(s.contains("claude t-1a2b3/i-1"), "{s}");
    assert!(s.contains("2m 05s"), "{s}");
    assert!(s.contains("3m 42s left"), "{s}");
    assert!(s.contains("lock renewed 4s ago"), "{s}");
    assert!(s.contains("expired 1m 00s ago"), "{s}");
    assert!(s.contains("lock renewed UNKNOWN"), "{s}");
    assert!(s.contains("COLLISION"), "{s}");
    assert!(!s.to_lowercase().contains("heartbeat"), "{s}");
}

#[test]
fn a_lease_count_comes_from_the_leases() {
    let m = fixture();
    let s = screen(&m);
    assert!(s.contains("1 live, 1 expired"), "{s}");
    assert!(!s.contains("LEASES: fresh"), "{s}");
}

#[test]
fn a_notice_shows_how_far_it_got() {
    let m = fixture();
    let s = screen(&m);
    assert!(s.contains("told"), "{s}");
    assert!(s.contains("docs/x.md changed on main"), "{s}");
}

#[test]
fn a_source_that_could_not_be_read_is_said() {
    let mut m = fixture();
    m.snap.faults.push("the claims: permission denied".to_owned());
    let s = screen(&m);
    assert!(s.contains("UNKNOWN the claims: permission denied"), "{s}");
}

#[test]
fn without_herdr_the_panel_says_so_and_shows_the_working_tree() {
    let mut m = fixture();
    m.live.herdr = model::Herdr::Absent {
        why: "no socket".to_owned(),
    };
    m.live.text = "git status\n M src/a.rs\n".to_owned();
    m.snap.progress_lines = vec!["2033 OW-WAR-0001 evidence.recorded claude".to_owned()];
    let s = screen(&m);
    assert!(s.contains("herdr: not running"), "{s}");
    assert!(s.contains("working tree and journal"), "{s}");
    assert!(s.contains("M src/a.rs"), "{s}");
    assert!(s.contains("evidence.recorded"), "{s}");
}

#[test]
fn with_herdr_the_panel_follows_the_agent_and_brackets_step() {
    let mut m = fixture();
    let agent = |pane: &str, title: &str| {
        serde_json::from_value::<crate::herdr::HerdrAgent>(serde_json::json!({
            "pane_id": pane, "agent": "claude", "terminal_title_stripped": title,
            "agent_status": "working",
        }))
        .expect("an agent")
    };
    let mut live = model::Live {
        herdr: model::Herdr::Connected {
            version: "0.9.5".to_owned(),
        },
        agents: vec![agent("w9:p1", "elsewhere"), agent("w1:p1", "first"), agent("w1:p2", "second")],
        text: "agent output".to_owned(),
    };
    live.agents[1].cwd = "/work/wt-a/sub".to_owned();
    live.agents[2].cwd = "/work/repo".to_owned();
    live.agents[0].cwd = "/work/repo-other".to_owned();
    m.apply_live(live.clone());
    let s = screen(&m);
    assert!(s.contains("herdr: 0.9.5, 2 here of 3 agent(s)"), "{s}");
    assert!(s.contains("[claude] first"), "{s}");
    m.apply_live(live);
    assert_eq!(m.focused_agent().map(|a| a.pane_id.as_str()), Some("w1:p1"));
    handle_key(&mut m, key(']'));
    assert_eq!(m.focused_agent().map(|a| a.pane_id.as_str()), Some("w1:p2"));
    handle_key(&mut m, key(']'));
    handle_key(&mut m, key(']'));
    assert_eq!(m.focused_agent().map(|a| a.pane_id.as_str()), Some("w1:p1"));
}

// ---- signing --------------------------------------------------------------

#[test]
fn one_check_signs_one_and_several_sign_as_one_batch() {
    let mut m = fixture();
    handle_key(&mut m, key('2'));
    handle_key(&mut m, key(' '));
    assert_eq!(
        handle_key(&mut m, key('s')),
        Some(Effect::Terminal {
            argv: sign_argv("authorize:OW-WAR-0001"),
            root: None
        })
    );
    handle_key(&mut m, key('a'));
    let Some(Effect::Terminal { argv, .. }) = handle_key(&mut m, key('s')) else {
        panic!("a batch is a child that owns the terminal");
    };
    assert_eq!(argv[1], "--batch=authorize:OW-WAR-0001,resolve:OW-WAR-0002");
}

/// Refusal: nothing checked signs nothing, and a check does not outlive the
/// act it was on or slide to the row that took its place.
#[test]
fn a_check_follows_its_act_not_its_row() {
    let mut m = fixture();
    handle_key(&mut m, key('2'));
    assert_eq!(handle_key(&mut m, key('s')), None, "nothing is checked");
    assert!(m.message.starts_with("nothing checked"));
    handle_key(&mut m, key('j'));
    handle_key(&mut m, key(' '));
    assert_eq!(act::checked_targets(&m), ["resolve:OW-WAR-0002"]);
    let mut next = m.snap.clone();
    next.queue.remove(0);
    m.apply(next);
    assert_eq!(act::checked_targets(&m), ["resolve:OW-WAR-0002"]);
    let mut next = m.snap.clone();
    next.queue = vec![queue_row("accept:2.0.0")];
    m.apply(next);
    assert!(act::checked_targets(&m).is_empty());
}

#[test]
fn v_reads_the_request_and_enter_signs_the_row() {
    let mut m = fixture();
    handle_key(&mut m, key('2'));
    assert_eq!(
        handle_key(&mut m, key('v')),
        Some(Effect::Capture {
            title: "war sign authorize:OW-WAR-0001 --show".to_owned(),
            argv: argv::sign_show("authorize:OW-WAR-0001"),
        })
    );
    assert_eq!(
        handle_key(&mut m, press(KeyCode::Enter)),
        Some(Effect::Terminal {
            argv: sign_argv("authorize:OW-WAR-0001"),
            root: None
        })
    );
}

#[test]
fn x_runs_a_remedy_off_the_main_thread_and_never_signs() {
    let mut m = fixture();
    handle_key(&mut m, key('3'));
    m.overlay = Overlay::Records(View::Evidence);
    assert_eq!(
        handle_key(&mut m, key('x')),
        Some(Effect::Capture {
            title: "war evidence record OW-WAR-0001 --gate ops.check@1.0.0".to_owned(),
            argv: argv::record_gate("OW-WAR-0001", "ops.check@1.0.0")[1..].to_vec(),
        })
    );
}

/// What does not work yet says so, and runs nothing.
#[test]
fn an_action_that_does_not_work_yet_reads_as_unavailable() {
    let mut m = fixture();
    assert_eq!(handle_key(&mut m, key('d')), None);
    assert!(m.message.contains("unavailable"), "{}", m.message);
    assert_eq!(handle_key(&mut m, key('a')), None);
    assert!(m.message.contains("unavailable"), "{}", m.message);
}

#[test]
fn q_quits_from_the_list_and_only_closes_a_popup() {
    let mut m = fixture();
    m.popup = Some(("t".to_owned(), "x".to_owned(), 0));
    assert_eq!(handle_key(&mut m, key('q')), None);
    assert!(m.popup.is_none());
    assert_eq!(handle_key(&mut m, key('q')), Some(Effect::Quit));
    assert_eq!(
        handle_key(&mut m, KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        Some(Effect::Quit)
    );
}

// ---- the command lines ----------------------------------------------------

/// Every command line the app builds is one the real CLI accepts.
#[test]
fn every_argv_parses_with_the_real_cli() {
    for line in argv::table() {
        argv::parses(&line).unwrap_or_else(|e| panic!("war {}: {e}", line.join(" ")));
    }
}

/// Refusal: the three lines the old app built, none of which ever ran, are
/// what this check exists to catch.
#[test]
fn a_bad_argv_is_refused() {
    for bad in [
        &["ticket", "new", "x"][..],
        &["answer", "t/Q-1", "--response", "x"],
        &["admin", "claim", "t-1", "--actor", "m"],
    ] {
        assert!(argv::parses(&argv::words(bad)).is_err(), "{bad:?}");
    }
}

#[test]
fn a_displayed_command_counts_only_when_it_is_whole() {
    assert_eq!(
        argv::of_display("war view show t-1"),
        Some(argv::words(&["view", "show", "t-1"]))
    );
    assert_eq!(argv::of_display("war init --root <directory>"), None);
    assert_eq!(argv::of_display("war check  # rule"), None);
    assert_eq!(argv::of_display("war create \"...\""), None);
    assert_eq!(argv::of_display("cargo install"), None);
}

// ---- snapshots ------------------------------------------------------------

/// A frame against its committed text. `OW_TUI_BLESS=1` writes it instead:
/// read the diff before committing one.
fn snapshot(name: &str, text: &str) {
    let path = camino::Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/tui/snapshots")
        .join(format!("{name}.txt"));
    if std::env::var_os("OW_TUI_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("snapshots directory");
        std::fs::write(&path, text).expect("write the snapshot");
    }
    let want = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(text, want, "{name} differs from {path}");
}

#[test]
fn watch_and_act_match_their_snapshots() {
    let mut m = fixture();
    m.live.herdr = model::Herdr::Absent {
        why: "no socket".to_owned(),
    };
    m.live.text = "git status\n M src/a.rs\n".to_owned();
    snapshot("watch", &screen(&m));
    handle_key(&mut m, key('2'));
    snapshot("act", &screen(&m));
    handle_key(&mut m, key('3'));
    snapshot("records", &screen(&m));
}

// ---- documents ------------------------------------------------------------

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

/// OW-WAR-0112 OBL-006: every embedded document renders in Help without a
/// panic — through `md::render` and the same Paragraph `draw_doc` builds,
/// into a normal and a cramped terminal, scrolled to its end. Refusal side:
/// the renderer is seen to have produced text, so an empty rendering does
/// not pass as "no panic".
#[test]
fn every_embedded_document_renders_in_help() {
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

/// The views are the ones the document lists, in the order they step.
#[test]
fn the_views_are_documented_in_order() {
    let doc = include_str!("../../../../docs/TUI.md");
    let mut last = 0;
    for v in View::ALL {
        let name = v.name();
        let at = doc
            .find(&format!("## {name}"))
            .unwrap_or_else(|| panic!("no `## {name}` in docs/TUI.md"));
        assert!(at > last, "{name} is out of order");
        last = at;
    }
}

/// t-67ed: the Tickets view shows a ticket's state and progress and its
/// REMAINING items; a done item is left to the detail. Refusal side: no row
/// is a signature or an auto remedy — the view is read-only.
#[test]
fn the_tickets_view_renders_remaining_items_and_signs_nothing() {
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
    let rows = records::ticket_rows(&board);
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
    // No tickets: the view says how to make one, and still signs nothing.
    let empty = records::ticket_rows(&serde_json::json!({"tickets": []}));
    assert_eq!(empty.len(), 1);
    assert!(empty[0].text.starts_with("no tickets yet"));
}

/// The row a remedy makes: an `auto` remedy is runnable from `x`, a human
/// one is a signing target for Enter, and never the other way.
#[test]
fn remedy_rows_keep_auto_and_human_apart() {
    let drift = Diagnostic::error(
        "deliverable.digest-drift",
        "docs/warrants/OW-WAR-0002/deliverables.toml".to_owned(),
        "OW-WAR-0002: D-002 records sha256:a for x but the file is now sha256:b",
    );
    let r = records::remedy_row(&drift, 1);
    assert!(r.auto.is_none(), "{r:?}");
    // M12: `war sign correct …` drafts the request; it is no target for
    // the sign key, whose `war sign <target>` is the signature.
    assert!(r.sign_target.is_none(), "{r:?}");
    let stale = Diagnostic::warn(
        "deliverable.pin-stale",
        "docs/warrants/OW-WAR-0003/deliverables.toml".to_owned(),
        "OW-WAR-0003: D-001 records sha256:a and the file is now sha256:b",
    );
    let r = records::remedy_row(&stale, 2);
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
