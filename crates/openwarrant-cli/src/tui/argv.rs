// SPDX-License-Identifier: Apache-2.0
//! Every `war` command line the app builds, in one place, so each is a value
//! that can be parsed by the real command line before anyone runs it.

use std::path::Path;

use clap::Parser as _;

/// `war sign <target> --show`: the request, for reading.
#[must_use]
pub fn sign_show(target: &str) -> Vec<String> {
    words(&["sign", target, "--show"])
}

/// `war admin commit`: the drafted message, committing nothing.
#[must_use]
pub fn commit_message() -> Vec<String> {
    words(&["admin", "commit"])
}

/// `war init`.
#[must_use]
pub fn init() -> Vec<String> {
    words(&["init"])
}

/// `war plan roadmap edit`.
#[must_use]
pub fn roadmap_edit() -> Vec<String> {
    words(&["plan", "roadmap", "edit"])
}

/// `war sign --batch=<targets> --ssh-sign`: one dialog for all of them.
#[must_use]
pub fn sign_batch(targets: &[String]) -> Vec<String> {
    vec![
        "sign".to_owned(),
        format!("--batch={}", targets.join(",")),
        "--ssh-sign".to_owned(),
    ]
}

/// `war evidence record <alias> --gate <gate>`, with its `war`.
#[must_use]
pub fn record_gate(alias: &str, gate: &str) -> Vec<String> {
    words(&["war", "evidence", "record", alias, "--gate", gate])
}

pub(super) fn words(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| (*a).to_owned()).collect()
}

/// Whether `args` (without the leading `war`) is a command line the real CLI
/// accepts. The error is clap's own.
pub fn parses(args: &[String]) -> Result<(), String> {
    crate::Cli::try_parse_from(std::iter::once("war").chain(args.iter().map(String::as_str)))
        .map(drop)
        .map_err(|e| e.render().to_string())
}

/// Every command the builders above make, with sample values.
#[must_use]
pub fn table() -> Vec<Vec<String>> {
    vec![
        sign_show("OW-WAR-0001"),
        super::sign_argv("OW-WAR-0001"),
        sign_batch(&["OW-WAR-0001".to_owned(), "2.0.0".to_owned()]),
        commit_message(),
        init(),
        roadmap_edit(),
        record_gate("OW-WAR-0001", "ops.check@1.0.0")[1..].to_vec(),
    ]
}

/// A row's displayed command as argv, when it is a whole one: no placeholder,
/// no comment, no ellipsis.
#[must_use]
pub fn of_display(command: &str) -> Option<Vec<String>> {
    if command.contains('<') || command.contains("...") || command.contains('#') {
        return None;
    }
    let rest = command.strip_prefix("war ")?;
    Some(rest.split_whitespace().map(str::to_owned).collect())
}

/// The lines of a planted argv file: one command to a line, words separated
/// by spaces.
pub fn read_table(path: &Path) -> Result<Vec<Vec<String>>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split_whitespace().map(str::to_owned).collect())
        .collect())
}
