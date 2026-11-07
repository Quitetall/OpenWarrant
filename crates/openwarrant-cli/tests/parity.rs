// SPDX-License-Identifier: Apache-2.0
//! The surface lost nothing (t-3c16c W1.6): docs/COMMANDS.md's parity table
//! covers the whole clap tree, and every spelling in it parses.
//!
//! The table is the claim and the tree is the fact. A command with no row, a
//! row naming a spelling that no longer parses, or a retiring word the table
//! and `surface` disagree on each fails here, with the refusal seen beside it.

use std::collections::BTreeSet;

use clap::error::ErrorKind;
use openwarrant_cli::surface;

/// The clap tree outgrows a test thread's stack, as `main.rs` says.
fn big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(16 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

/// `(now, others, state)` of each row.
fn rows() -> Vec<(String, Vec<String>, String)> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/COMMANDS.md");
    let text = std::fs::read_to_string(path).unwrap();
    let start = text.find("<!-- parity:start -->").expect("the table");
    let end = text.find("<!-- parity:end -->").expect("its end");
    let ticks = |cell: &str| -> Vec<String> {
        cell.split('`')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect()
    };
    text[start..end]
        .lines()
        .filter(|l| l.starts_with("| ") && !l.starts_with("| ---") && !l.starts_with("| Function"))
        .map(|l| {
            let cells: Vec<&str> = l.trim_matches('|').split(" | ").collect();
            assert_eq!(cells.len(), 5, "{l}");
            let now = ticks(cells[1]).remove(0);
            (now, ticks(cells[2]), cells[4].trim().to_owned())
        })
        .collect()
}

fn asked(spelling: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for w in spelling.split_whitespace() {
        words.push(if w.starts_with('<') || w.starts_with('"') {
            "x".to_owned()
        } else {
            w.to_owned()
        });
    }
    words
}

/// Whether the real parser knows the spelling: its help is what it answers.
fn parses(cmd: &clap::Command, spelling: &str) -> Result<(), String> {
    let mut argv = asked(spelling);
    if argv == ["war"] {
        return Ok(());
    }
    if !argv.iter().any(|a| a == "--version") {
        argv.push("--help".to_owned());
    }
    match cmd.clone().try_get_matches_from(&argv) {
        Err(e) if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) => Ok(()),
        Err(e) => Err(format!("{spelling}: {e}")),
        Ok(_) => Err(format!("{spelling}: answered with no help")),
    }
}

/// Every word of the tree: each top-level command and each group's members,
/// aliases included, as `war <words>`.
fn tree(cmd: &clap::Command) -> BTreeSet<String> {
    let mut all = BTreeSet::new();
    for sub in cmd.get_subcommands() {
        let name = sub.get_name();
        if name == "help" {
            continue;
        }
        let spellings: Vec<&str> = std::iter::once(name).chain(sub.get_all_aliases()).collect();
        for word in spellings {
            all.insert(format!("war {word}"));
        }
        if openwarrant_cli::GROUPS.contains(&name) {
            for member in sub.get_subcommands().filter(|m| m.get_name() != "help") {
                for word in std::iter::once(member.get_name()).chain(member.get_all_aliases()) {
                    all.insert(format!("war {name} {word}"));
                }
            }
        }
    }
    all
}

#[test]
fn every_command_in_the_tree_has_a_row() {
    let missing = big_stack(|| {
        let cmd = openwarrant_cli::command();
        let mut named = BTreeSet::new();
        for (now, others, _) in rows() {
            named.insert(now);
            named.extend(others);
        }
        // A daily command and a group are their own row's first words.
        let covered = |spelling: &str| {
            named.contains(spelling)
                || named
                    .iter()
                    .any(|n| n.starts_with(spelling) && n[spelling.len()..].starts_with(' '))
        };
        let mut missing: Vec<String> = tree(&cmd).into_iter().filter(|s| !covered(s)).collect();
        // The refusal: a planted command is seen as missing.
        let planted = cmd.clone().subcommand(clap::Command::new("planted"));
        assert!(
            tree(&planted).iter().any(|s| !covered(s)),
            "the walk would not notice a command with no row"
        );
        missing.retain(|s| s != "war schemas" && s != "war admin schemas");
        missing
    });
    assert!(
        missing.is_empty(),
        "no row in docs/COMMANDS.md: {missing:?}"
    );
}

#[test]
fn every_spelling_in_the_table_parses() {
    let wrong = big_stack(|| {
        let cmd = openwarrant_cli::command();
        // The refusal: a spelling that does not exist does not parse.
        assert!(parses(&cmd, "war frobnicate").is_err());
        assert!(parses(&cmd, "war status --no-such-flag").is_err());
        let mut wrong = Vec::new();
        for (now, others, _) in rows() {
            for spelling in std::iter::once(now).chain(others) {
                wrong.extend(parses(&cmd, &spelling).err());
            }
        }
        wrong
    });
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn the_table_and_the_retirement_list_agree() {
    let mut disagree = Vec::new();
    for (_, others, state) in rows() {
        for spelling in others {
            let argv = asked(&spelling);
            let words: Vec<&str> = argv[1..].iter().map(String::as_str).collect();
            let retiring = surface::replacement(&words).is_some();
            let top_level = words.len() == 1;
            if top_level && retiring != state.contains("retiring") {
                disagree.push(format!("{spelling}: {state}"));
            }
        }
    }
    assert!(disagree.is_empty(), "{disagree:#?}");
    // The refusal: a current spelling is not read as retiring.
    assert_eq!(surface::replacement(&["status"]), None);
}

#[test]
fn each_flag_of_the_six_is_the_command_it_folds() {
    let cmd = big_stack(openwarrant_cli::command);
    for (fold, old) in [
        ("war status --next --help", "war view next"),
        ("war status --ready --help", "war view ready"),
        ("war status x --view status --help", "war view show"),
        ("war start x --claim --help", "war admin claim"),
        ("war done x --release --help", "war admin release"),
    ] {
        assert!(
            parses(&cmd, fold.trim_end_matches(" --help")).is_ok(),
            "{fold}"
        );
        assert!(parses(&cmd, old).is_ok(), "{old}");
    }
    // Each is refused where it would mean nothing.
    for wrong in [
        "war status x --next",
        "war done --release",
        "war done x --reopen",
        "war start x --steal",
    ] {
        let argv = asked(wrong);
        let refused = big_stack({
            let cmd = cmd.clone();
            move || cmd.try_get_matches_from(argv).is_err()
        });
        assert!(refused, "{wrong} parsed");
    }
}
