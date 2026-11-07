// SPDX-License-Identifier: Apache-2.0
//! The retirement of old spellings (t-3c16c W1.6; docs/COMMANDS.md).
//!
//! `war --help` shows six commands and `war sign`. Everything else stays
//! reachable under a group, and every earlier spelling keeps its bytes while it
//! is retired. Retirement is one line on stderr, and only where a person reads
//! it: at a terminal, or with `OPENWARRANT_DEPRECATIONS=warn` for someone
//! sweeping a script. A pipe sees nothing, because scripts and plants compare
//! output. No spelling is removed by this table: removal is a later, separate
//! act, after a stable release has carried the notice.
//!
//! Three kinds of word stay out of the table: a daily command, a group, and
//! the words something other than a person types ([`KEPT`]). A group's own
//! spelling of a fold, `war view next` beside `war status --next`, stays
//! current too: messages still name it, and retiring it starts with them.

/// A spelling with a new home that is not simply its group: the old words,
/// and what replaces them. The words of `old` match a whole prefix of the
/// command line, longest first.
pub const FOLDS: &[(&str, &str)] = &[
    ("view timeline", "status --timeline"),
    ("view tui", "war"),
    ("view progress", "view overview"),
    ("view tickets", "view warrants"),
    ("view ls", "view warrants"),
    ("admin version", "--version"),
    ("next", "status --next"),
    ("ready", "status --ready"),
    ("show", "status <id> --view full_warrant"),
    ("tui", "war"),
    ("progress", "view overview"),
    ("tickets", "view warrants"),
    ("ls", "view warrants"),
    ("claim", "start <id> --claim"),
    ("release", "done <id> --release"),
    ("reopen", "done <id> --reopen --reason \"...\""),
    ("version", "--version"),
];

/// Hidden words something other than a person types, or that a format names:
/// they keep working with no notice and no end date. `context`, FORMAT-1.0
/// §10.3 and `llms.txt`. `apply`, §10.5. `merge-ticket`, written into clones'
/// git configuration. `mcp` and `sdk`, in harness configuration. `host`, the
/// Liminal protocol.
pub const KEPT: &[&str] = &["context", "apply", "merge-ticket", "mcp", "sdk", "host"];

/// The first two words of a command line, past the global flags: what
/// [`replacement`] reads.
#[must_use]
pub fn words(argv: &[String]) -> Vec<&str> {
    let mut out = Vec::new();
    let mut args = argv.iter().map(String::as_str);
    while out.len() < 2 {
        match args.next() {
            None | Some("--") => break,
            Some("--root") => {
                let _ = args.next();
            }
            Some(a) if a.starts_with('-') => {
                // A flag after the first word belongs to the command.
                if !out.is_empty() {
                    break;
                }
            }
            Some(a) => out.push(a),
        }
    }
    out
}

/// What replaces the spelling `words` opens with, or `None` when it is
/// current.
#[must_use]
pub fn replacement(words: &[&str]) -> Option<String> {
    let first = *words.first()?;
    if let [a, b, ..] = words {
        let two = format!("{a} {b}");
        if let Some((_, new)) = FOLDS.iter().find(|(old, _)| *old == two) {
            return Some(spelled(new));
        }
    }
    if crate::GROUPS.contains(&first) {
        return None;
    }
    if let Some((_, new)) = FOLDS.iter().find(|(old, _)| *old == first) {
        return Some(spelled(new));
    }
    if crate::DAILY.contains(&first) || KEPT.contains(&first) {
        return None;
    }
    crate::GROUP_MEMBERS
        .iter()
        .find(|(_, members)| members.contains(&first))
        .map(|(group, _)| format!("war {group} {first}"))
}

fn spelled(new: &str) -> String {
    if new == "war" {
        new.to_owned()
    } else {
        format!("war {new}")
    }
}

/// The line a retired spelling earns, or `None`.
#[must_use]
pub fn notice(argv: &[String]) -> Option<String> {
    let words = words(argv);
    let new = replacement(&words)?;
    Some(format!(
        "note: `war {}` still works and is being retired; use `{new}` (docs/COMMANDS.md)",
        words.join(" ")
    ))
}

/// Whether a retired spelling is said: to a person, or on request.
#[must_use]
pub fn says_so(at_a_terminal: bool, env: Option<&str>) -> bool {
    match env {
        Some("off") => false,
        Some("warn") => true,
        _ => at_a_terminal,
    }
}

/// Print [`notice`] for this process's command line, where [`says_so`] holds.
pub fn announce(json: bool) {
    use std::io::IsTerminal as _;
    let env = std::env::var("OPENWARRANT_DEPRECATIONS").ok();
    if json || !says_so(std::io::stderr().is_terminal(), env.as_deref()) {
        return;
    }
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if let Some(line) = notice(&argv) {
        eprintln!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    fn new_for(line: &str) -> Option<String> {
        replacement(&words(&argv(line)))
    }

    #[test]
    fn a_retired_spelling_names_what_replaces_it() {
        assert_eq!(new_for("view timeline").unwrap(), "war status --timeline");
        assert_eq!(new_for("next --at x").unwrap(), "war status --next");
        assert_eq!(
            new_for("--root /x --json next").unwrap(),
            "war status --next"
        );
        assert_eq!(new_for("compile").unwrap(), "war admin compile");
        assert_eq!(new_for("gate list").unwrap(), "war evidence gate");
        assert_eq!(new_for("tui").unwrap(), "war");
    }

    #[test]
    fn a_current_spelling_says_nothing() {
        for line in [
            "",
            "init",
            "create x",
            "start",
            "done --note x",
            "status",
            "check",
            "sign OW-WAR-0001",
            "sign authorize OW-WAR-0001",
            "admin compile",
            "view board",
            "view next",
            "admin claim t-1",
            "plan add t-1 x",
            "plan new x",
            "context t-1",
            "mcp",
            "merge-ticket a b c d",
        ] {
            assert_eq!(new_for(line), None, "{line}");
        }
    }

    #[test]
    fn only_a_person_or_someone_who_asked_is_told() {
        assert!(says_so(true, None));
        assert!(!says_so(false, None), "a pipe compares bytes");
        assert!(says_so(false, Some("warn")));
        assert!(!says_so(true, Some("off")));
    }

    #[test]
    fn every_fold_names_its_group_spelling_too() {
        for (old, _) in FOLDS {
            if let Some((group, word)) = old.split_once(' ') {
                assert!(crate::group_member(group, word), "{old}");
            }
        }
    }
}
