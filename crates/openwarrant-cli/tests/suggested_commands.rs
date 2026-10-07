// SPDX-License-Identifier: Apache-2.0
//! Every command `war` suggests is runnable as printed (M9).
//!
//! A diagnostic that tells a reader to run `war pins --refresh OW-WAR-0001`
//! when the CLI wants `war pins --refresh --alias OW-WAR-0001` sends them
//! into a usage error, and an agent reads that as one more wall. Two sources
//! of suggested commands are held to the real clap parser here:
//!
//! - every remedy in `remedy::TABLE`, built from a finding that names an
//!   alias, a deliverable and a path, as `war check` would print it;
//! - every backticked `war …` in a string of the non-test source of the CLI,
//!   core and compiler crates: the messages, notes and hints the binary
//!   prints, whose first backticked command is also what `remedy_for` falls
//!   back to.
//!
//! Placeholders are filled before parsing (`<alias>`, `{alias}` become a
//! value, `a|b` the first alternative, a trailing `…` is dropped), so what is
//! checked is the shape of the command: its subcommands, flags and
//! arguments. A bare mention of a command family (`war sas`) passes when clap
//! answers with that family's help; anything else clap refuses fails here,
//! by file, line and command, so a rename (M12) that leaves a suggestion
//! behind is caught.

use std::path::{Path, PathBuf};

use clap::error::ErrorKind;
use openwarrant_cli::diagnostic::Diagnostic;
use openwarrant_cli::remedy;

/// Split like a shell would for the simple cases a message carries: words,
/// and "double" or 'single' quoted words. Each word says whether it was
/// quoted, so a quoted `"..."` stays a value while a bare `…` is a gap.
fn split(line: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut quoted = false;
    for c in line.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => cur.push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                quoted = true;
            }
            (None, c) if c.is_whitespace() => {
                if quoted || !cur.is_empty() {
                    out.push((std::mem::take(&mut cur), quoted));
                }
                quoted = false;
            }
            (None, c) => cur.push(c),
        }
    }
    if quoted || !cur.is_empty() {
        out.push((cur, quoted));
    }
    out
}

/// A suggested command with its placeholders filled, and whether its last
/// word was a bare `…` (a gap the reader fills, which may also mean "and the
/// rest").
fn normalize(span: &str) -> (Vec<String>, bool) {
    let mut s = span
        .replace("\\\"", "\"")
        .replace("\\n#", " ")
        .replace("\\n", " ")
        .replace("{{", "\u{1}")
        .replace("}}", "\u{2}");
    // `{alias}` / `{a}` (format arguments) and `<alias>` (prose placeholders).
    for (open, close) in [('{', '}'), ('<', '>')] {
        while let (Some(i), Some(j)) = (s.find(open), s.find(close)) {
            if j < i {
                break;
            }
            s.replace_range(i..=j, "1");
        }
    }
    let s = s.replace('\u{1}', "{").replace('\u{2}', "}");
    let mut trailing_gap = false;
    let words: Vec<String> = split(&s)
        .into_iter()
        // `[alias]`: an optional part, left out.
        .filter(|(w, q)| *q || !(w.starts_with('[') && w.ends_with(']')))
        .map(|(w, q)| {
            let gap = !q && (w == "…" || w == "...");
            trailing_gap = gap;
            if gap {
                "1".to_owned()
            } else if !q && w.contains('|') {
                // `behaviour-change|added-refusal`: the first alternative.
                w.split('|').next().unwrap_or_default().to_owned()
            } else {
                w
            }
        })
        .collect();
    (words, trailing_gap)
}

/// How strictly a command is held.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Strict {
    /// A remedy's argv: it parses whole.
    Whole,
    /// A command named in prose: its shape must be right (every subcommand
    /// and flag exists, no argument is in the wrong place), and operands it
    /// leaves out (`war evidence record` without the alias) are the reader's
    /// to supply.
    Shape,
}

/// What clap says about one command. `Ok` when it parses, or when a bare
/// mention of a command family is answered with that family's help.
fn parses(cmd: &clap::Command, argv: &[String], strict: Strict) -> Result<(), String> {
    match cmd.clone().try_get_matches_from(argv) {
        Ok(_) => Ok(()),
        Err(e) => {
            let text = e.to_string();
            match e.kind() {
                ErrorKind::DisplayHelp
                | ErrorKind::DisplayVersion
                | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
                | ErrorKind::MissingSubcommand => Ok(()),
                // `war new` named as a command, not handed over as one to run.
                ErrorKind::MissingRequiredArgument if argv.len() <= 2 => Ok(()),
                ErrorKind::MissingRequiredArgument if strict == Strict::Shape => Ok(()),
                // A flag named last without its value (`war plan --proposal`).
                ErrorKind::InvalidValue
                    if strict == Strict::Shape && text.contains("but none was supplied") =>
                {
                    Ok(())
                }
                _ => Err(text.lines().next().unwrap_or_default().to_owned()),
            }
        }
    }
}

/// A prose command: parsed as written, then without a trailing gap.
fn shape_ok(cmd: &clap::Command, span: &str) -> Result<(), String> {
    let (mut argv, trailing_gap) = normalize(span);
    match parses(cmd, &argv, Strict::Shape) {
        Ok(()) => Ok(()),
        Err(_) if trailing_gap => {
            argv.pop();
            parses(cmd, &argv, Strict::Shape)
        }
        Err(why) => Err(why),
    }
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).expect("readable source dir") {
        let p = e.expect("entry").path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Every backticked `war …` in the non-test, non-comment part of `text`,
/// with the line it starts on. Rust's string continuation (`\` at the end of
/// a line, leading whitespace dropped) is joined first, so a command broken
/// across source lines is read whole.
fn spans(text: &str) -> Vec<(usize, String)> {
    let body = text.split("#[cfg(test)]").next().unwrap_or(text);
    let mut out = Vec::new();
    let mut joined = String::new();
    let mut start_line = 0;
    for (n, line) in body.lines().enumerate() {
        let t = line.trim_start();
        if t.starts_with("//") {
            continue;
        }
        if joined.is_empty() {
            start_line = n + 1;
            joined.push_str(line);
        } else {
            joined.push_str(t);
        }
        if let Some(stripped) = joined.strip_suffix('\\') {
            joined = stripped.to_owned();
            continue;
        }
        let mut rest = joined.as_str();
        while let Some(i) = rest.find("`war ") {
            let after = &rest[i + 1..];
            let Some(end) = after.find('`') else { break };
            out.push((start_line, after[..end].to_owned()));
            rest = &after[end + 1..];
        }
        joined.clear();
    }
    out
}

fn with_cli<T: Send + 'static>(f: impl FnOnce(clap::Command) -> T + Send + 'static) -> T {
    // The binary builds its clap tree on an 8 MiB main stack; libtest's 2 MiB
    // worker is too small for the unoptimized tree (main.rs says the same).
    std::thread::Builder::new()
        .stack_size(8 << 20)
        .spawn(move || f(openwarrant_cli::command()))
        .expect("spawn")
        .join()
        .expect("the command tree builds")
}

#[test]
fn every_remedy_parses_as_printed() {
    let failures = with_cli(|cmd| {
        let mut failures = Vec::new();
        for rule in remedy::TABLE {
            let d = Diagnostic::error(
                *rule,
                "docs/warrants/OW-WAR-0001/deliverables.toml".to_owned(),
                "OW-WAR-0001: D-002 records sha256:aa for crates/x.rs but the file is now \
                 sha256:bb",
            );
            let Some(r) = remedy::remedy_for(&d) else {
                failures.push(format!("{rule}: no remedy"));
                continue;
            };
            if r.argv.first().map(String::as_str) != Some("war") {
                continue;
            }
            if let Err(why) = parses(&cmd, &r.argv, Strict::Whole) {
                failures.push(format!("{rule}: `{}`: {why}", r.command()));
            }
        }
        failures
    });
    assert!(
        failures.is_empty(),
        "remedies clap refuses:\n{}",
        failures.join("\n")
    );
}

#[test]
fn every_suggested_command_in_the_source_parses_as_printed() {
    // The CLI's own source, and the two crates whose messages it prints.
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for c in [
        "openwarrant-cli",
        "openwarrant-core",
        "openwarrant-compiler",
    ] {
        rust_files(&crates.join(c).join("src"), &mut files);
    }
    files.sort();
    let mut found: Vec<(String, String)> = Vec::new();
    for f in &files {
        let text = std::fs::read_to_string(f).expect("readable");
        let rel = f.strip_prefix(&crates).unwrap_or(f).display().to_string();
        for (line, span) in spans(&text) {
            found.push((format!("{rel}:{line}"), span));
        }
    }
    assert!(
        found.len() > 100,
        "found only {} suggested commands",
        found.len()
    );
    let total = found.len();
    eprintln!("{total} suggested commands in the source checked against clap");
    let failures = with_cli(move |cmd| {
        found
            .into_iter()
            // `war {command}`: a command built at run time, not a suggestion.
            .filter(|(_, span)| !span["war ".len()..].starts_with('{'))
            // `war schemas` exists only in a build with the `schema` feature.
            .filter(|(_, span)| cfg!(feature = "schema") || !span.starts_with("war schemas"))
            .filter_map(|(at, span)| {
                shape_ok(&cmd, &span)
                    .err()
                    .map(|why| format!("{at}: `{span}`: {why}"))
            })
            .collect::<Vec<_>>()
    });
    assert!(
        failures.is_empty(),
        "{} of {total} suggested commands clap refuses:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The checks above have teeth: the command the M9 report found, a renamed
/// subcommand and an unknown flag are refused; their fixed forms parse.
#[test]
fn a_suggestion_clap_refuses_is_caught() {
    with_cli(|cmd| {
        assert!(shape_ok(&cmd, "war pins --refresh OW-WAR-0001").is_err());
        assert!(shape_ok(&cmd, "war pins --refresh {alias}").is_err());
        assert_eq!(shape_ok(&cmd, "war pins --refresh --alias {alias}"), Ok(()));
        assert!(shape_ok(&cmd, "war no-such-command x").is_err());
        assert!(shape_ok(&cmd, "war check --no-such-flag").is_err());
        assert!(shape_ok(&cmd, "war authorize/resolve/sas accept … --response").is_err());
        // A family named alone, and an operand left for the reader.
        assert_eq!(shape_ok(&cmd, "war sas"), Ok(()));
        assert_eq!(shape_ok(&cmd, "war evidence record"), Ok(()));
        // Strict, as a remedy is held: the operand is required.
        let (argv, _) = normalize("war evidence record");
        assert!(parses(&cmd, &argv, Strict::Whole).is_err());
        // A quoted "..." is a value; a bare trailing … is a gap.
        assert_eq!(shape_ok(&cmd, "war done {id} --note \"...\""), Ok(()));
        assert_eq!(shape_ok(&cmd, "war sign <alias> --ssh-sign …"), Ok(()));
        assert_eq!(
            normalize(
                "war sign <alias>/<D-id> --kind behaviour-change|added-refusal --meaning \"…\""
            )
            .0,
            [
                "war",
                "sign",
                "1/1",
                "--kind",
                "behaviour-change",
                "--meaning",
                "…"
            ]
        );
    });
}
