// SPDX-License-Identifier: Apache-2.0
//! What goes out to an executor, and what comes back (OW-WAR-0148 M15).
//!
//! # Out: the packet
//!
//! `oh.war/go-packet/v1`, one JSON document per attempt: the node, its
//! Warrant's description and notes, what it waited on, the checks that will
//! decide its tick, what the Warrant declares the work may touch, where the
//! work happens (worktree, branch, base commit), and the answer expected,
//! pre-filled with the attempt's bindings. Its `brief` is the same as
//! Markdown, for a harness that reads prose (Claude Code, an issue body).
//!
//! # Back: a Stage Submission
//!
//! The answer is a §51 Stage Submission whose `dispatch_id`, `attempt_id`,
//! `contract_digest` (the node's revision when it was handed out) and
//! `stage_id` (the node) are the packet's. It passes exactly the refusals
//! `war evidence submit` applies ([`crate::run_cmd::admit_answer`]), whoever
//! sent it, so no executor, native or external, can declare work done: an
//! answer asks to continue, to be checked (`verify`), to block, to amend or
//! to cancel, and the scheduler decides what happens next.
//!
//! # Declared allowed acts
//!
//! A light Warrant may declare what its work may touch in an optional atom,
//! `atoms/35-allowed.md`, plain Markdown:
//!
//! ```markdown
//! # Allowed
//!
//! ## Tools
//! - Read
//! - Edit
//!
//! ## Paths
//! - src/parser/**
//!
//! ## Commands
//! - cargo test -p parser
//! ```
//!
//! A directory Warrant's declared paths are its deliverables' files. With
//! `[go] allowed_acts = "projected"`, `war start` writes them as the
//! session's harness settings inside its worktree; with `declared`, they are
//! shown and nothing is written.

use serde::Serialize;

use crate::ticket::Ticket;

pub const SCHEMA: &str = "oh.war/go-packet/v1";

/// Where a light Warrant declares its allowed acts, relative to its directory.
pub const ALLOWED_FILE: &str = "atoms/35-allowed.md";

/// The files a session writes for itself in a worktree, never committed.
pub const SESSION_FILES: &[&str] = &[
    ".claude/settings.local.json",
    ".openwarrant/session.json",
    ".openwarrant/go-packet.json",
    ".openwarrant/state",
];

/// What a Warrant declares its work may touch. Empty is "nothing declared",
/// which restricts nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct Allowed {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<String>,
}

impl Allowed {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty() && self.paths.is_empty() && self.commands.is_empty()
    }
}

/// Parse an allowed-acts atom. Lines outside the three sections are prose.
#[must_use]
pub fn parse_allowed(text: &str) -> Allowed {
    let mut out = Allowed::default();
    let mut section: Option<&str> = None;
    for line in text.lines() {
        let t = line.trim();
        if let Some(h) = t.strip_prefix("## ") {
            section = match h.trim().to_ascii_lowercase().as_str() {
                "tools" => Some("tools"),
                "paths" => Some("paths"),
                "commands" => Some("commands"),
                _ => None,
            };
            continue;
        }
        if t.starts_with("# ") {
            section = None;
            continue;
        }
        let Some(value) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) else {
            continue;
        };
        let value = value.trim();
        let value = value
            .strip_prefix('`')
            .and_then(|v| v.strip_suffix('`'))
            .unwrap_or(value)
            .trim();
        if value.is_empty() {
            continue;
        }
        match section {
            Some("tools") => out.tools.push(value.to_owned()),
            Some("paths") => out.paths.push(value.trim_start_matches("./").to_owned()),
            Some("commands") => out.commands.push(value.to_owned()),
            _ => {}
        }
    }
    out
}

/// What a light Warrant declares, from its allowed-acts atom.
#[must_use]
pub fn allowed_of_ticket(t: &Ticket) -> Allowed {
    crate::vfs::read_to_string(t.dir.join(ALLOWED_FILE))
        .map(|text| parse_allowed(&text))
        .unwrap_or_default()
}

/// What a directory Warrant declares: its deliverables' files.
#[must_use]
pub fn allowed_of_warrant(repo: &crate::repo::Repository, alias: &str) -> Allowed {
    let Ok(dir) = repo.warrant_dir(alias) else {
        return Allowed::default();
    };
    let paths = repo
        .load_deliverables(&dir)
        .map(|set| {
            set.records
                .iter()
                .filter_map(|d| crate::deliver::target_path(d).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    Allowed {
        paths,
        ..Allowed::default()
    }
}

/// A check that decides the node's tick.
#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub name: String,
    pub cmd: String,
}

/// The answer the packet expects, pre-filled.
#[derive(Debug, Clone, Serialize)]
pub struct Answer {
    pub dispatch_id: String,
    pub attempt_id: String,
    pub contract_digest: String,
    pub stage_id: String,
    /// One of these; `verify` asks for the checks and the landing.
    pub requested_next_action: &'static [&'static str],
}

/// `oh.war/go-packet/v1`.
#[derive(Debug, Clone, Serialize)]
pub struct Packet {
    pub schema: &'static str,
    pub dispatch_id: String,
    pub attempt_id: String,
    /// 1 for the first attempt.
    pub attempt: u32,
    pub node: String,
    pub kind: &'static str,
    pub warrant: String,
    pub title: String,
    pub text: String,
    /// The Warrant's description.
    pub description: String,
    /// Its dated notes, oldest first: what earlier attempts and people left.
    pub notes: Vec<String>,
    /// What this node waited on, all done now.
    pub depends_on: Vec<String>,
    pub checks: Vec<Check>,
    #[serde(skip_serializing_if = "Allowed::is_empty")]
    pub allowed: Allowed,
    /// The node's revision when it was handed out: the answer's
    /// `contract_digest`.
    pub revision: String,
    /// Where the work happens.
    pub worktree: String,
    pub branch: String,
    pub base: String,
    /// The checkout `war evidence go` runs from, where the plan is.
    pub root: String,
    pub answer: Answer,
    pub brief: String,
}

/// The five requests §51.2 permits; `resolve` is not one of them.
pub const ACTIONS: &[&str] = &["verify", "continue", "block", "amend", "cancel"];

/// The Markdown a harness or an issue reads: the work, its context, the
/// checks, what is declared, and the answer.
#[must_use]
pub fn brief(p: &Packet) -> String {
    let mut md = String::new();
    md.push_str(&format!(
        "# {}{}\n\n",
        p.title,
        if p.text.is_empty() || p.text == p.title {
            String::new()
        } else {
            format!(": {}", p.text)
        }
    ));
    md.push_str(&format!(
        "One piece of tracked work, `{}`, attempt {}. It has a worktree of its own: `{}`, on \
         branch `{}` from `{}`. Do the work there; commit it, or leave it in the tree and it \
         is committed for you.\n\n",
        p.node,
        p.attempt,
        p.worktree,
        p.branch,
        &p.base[..p.base.len().min(12)]
    ));
    if !p.description.is_empty() {
        md.push_str("## Context\n\n");
        md.push_str(&p.description);
        md.push_str("\n\n");
    }
    if !p.depends_on.is_empty() {
        md.push_str(&format!(
            "Done before this, and on the branch you start from: {}.\n\n",
            p.depends_on.join(", ")
        ));
    }
    if !p.checks.is_empty() {
        md.push_str(
            "## Checks\n\nAfter you answer, these run in the worktree; the work is ticked at \
             `observed` when they pass:\n\n",
        );
        for c in &p.checks {
            md.push_str(&format!("- {}: `{}`\n", c.name, c.cmd));
        }
        md.push('\n');
    }
    if !p.allowed.is_empty() {
        md.push_str("## What the Warrant declares this work touches\n\n");
        for (what, list) in [
            ("tools", &p.allowed.tools),
            ("paths", &p.allowed.paths),
            ("commands", &p.allowed.commands),
        ] {
            if !list.is_empty() {
                md.push_str(&format!("- {what}: {}\n", list.join(", ")));
            }
        }
        md.push('\n');
    }
    if !p.notes.is_empty() {
        md.push_str("## Notes so far\n\n");
        for n in &p.notes {
            md.push_str(&format!("- {n}\n"));
        }
        md.push('\n');
    }
    let answer = serde_json::json!({
        "dispatch_id": p.answer.dispatch_id,
        "attempt_id": p.answer.attempt_id,
        "contract_digest": p.answer.contract_digest,
        "stage_id": p.answer.stage_id,
        "requested_next_action": "verify",
    });
    md.push_str(&format!(
        "## Answer\n\nWhen you are finished, end with this JSON, as the last thing you print \
         (or, on an issue, as a comment). Set `requested_next_action` to `verify` when the work \
         is ready to be checked and landed, `continue` when there is more to do, or `block` \
         when a person has to decide something (say what in `blockers`). The checks and the \
         landing decide whether it is done.\n\n```json war-submission\n{}\n```\n",
        serde_json::to_string_pretty(&answer).unwrap_or_default()
    ));
    md
}

/// The answer in what a performer printed: the whole of it when it is JSON;
/// else the `result` of Claude Code's `--output-format json` (whose `usage`
/// is the token count); else the last fenced `json` block in it. With the
/// tokens the harness reported, when it reported any.
#[must_use]
pub fn answer_in(stdout: &str) -> (String, Option<u64>) {
    let trimmed = stdout.trim();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        // Claude Code's result envelope: the answer is its text.
        if v.get("type").and_then(serde_json::Value::as_str) == Some("result")
            && let Some(text) = v.get("result").and_then(serde_json::Value::as_str)
        {
            let tokens = tokens_in(v.get("usage"));
            let (inner, _) = answer_in(text);
            return (inner, tokens);
        }
        let tokens = tokens_in(v.get("usage"));
        return (trimmed.to_owned(), tokens);
    }
    // The last fenced block whose info string starts `json`.
    let mut last: Option<String> = None;
    let mut inside: Option<String> = None;
    for line in trimmed.lines() {
        let t = line.trim_start();
        match &mut inside {
            None if t.starts_with("```json") => inside = Some(String::new()),
            Some(buf) if t.starts_with("```") => {
                last = Some(std::mem::take(buf));
                inside = None;
            }
            Some(buf) => {
                buf.push_str(line);
                buf.push('\n');
            }
            None => {}
        }
    }
    (last.unwrap_or_else(|| trimmed.to_owned()), None)
}

/// Tokens from a usage object: `tokens`, else input plus output (plus
/// cache reads and writes, which a budget counts too).
fn tokens_in(usage: Option<&serde_json::Value>) -> Option<u64> {
    let u = usage?;
    if let Some(t) = u.get("tokens").and_then(serde_json::Value::as_u64) {
        return Some(t);
    }
    let parts = [
        "input_tokens",
        "output_tokens",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
    ];
    let mut any = false;
    let mut total = 0u64;
    for p in parts {
        if let Some(n) = u.get(p).and_then(serde_json::Value::as_u64) {
            any = true;
            total = total.saturating_add(n);
        }
    }
    any.then_some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_acts_are_read_by_section() {
        let a = parse_allowed(
            "# Allowed\n\nProse.\n\n## Tools\n- Read\n- `Edit`\n\n## Paths\n- ./src/**\n\n## \
             Commands\n- `cargo test -p parser`\n\n## Other\n- ignored\n",
        );
        assert_eq!(a.tools, ["Read", "Edit"]);
        assert_eq!(a.paths, ["src/**"]);
        assert_eq!(a.commands, ["cargo test -p parser"]);
        assert!(parse_allowed("# Allowed\n").is_empty());
    }

    #[test]
    fn the_answer_is_found_where_a_harness_puts_it() {
        let plain = r#"{"dispatch_id":"d","usage":{"tokens":42}}"#;
        assert_eq!(answer_in(plain), (plain.to_owned(), Some(42)));
        let claude = serde_json::json!({
            "type": "result",
            "result": "Done.\n```json war-submission\n{\"dispatch_id\":\"d\"}\n```\n",
            "usage": {"input_tokens": 10, "output_tokens": 5},
        })
        .to_string();
        let (a, t) = answer_in(&claude);
        assert_eq!(a.trim(), r#"{"dispatch_id":"d"}"#);
        assert_eq!(t, Some(15));
        let fenced = "notes\n```json\n{\"a\":1}\n```\nmore\n```json\n{\"b\":2}\n```\n";
        assert_eq!(answer_in(fenced).0.trim(), r#"{"b":2}"#);
        assert_eq!(answer_in("not json").1, None, "unreported, not zero");
    }
}
