// SPDX-License-Identifier: Apache-2.0
//! Writers for the single Markdown source. Legacy sources retain their writer.
use std::io::Write as _;

use openwarrant_core::ticket::TicketManifest;

use super::{Outcome, Store, Ticket};
use crate::repo::RepoError;

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Render the same validated intent and items in the standard encoding.
pub(crate) fn document(m: &TicketManifest, intent: &str, checklist: &str) -> String {
    let mut summary = m.summary.clone().unwrap_or_else(|| m.title.clone());
    while summary.len() > openwarrant_core::format::SUMMARY_MAX {
        summary.pop();
    }
    let mut out = format!(
        "---\nformat: openwarrant/1.0\nprofile: ticket\nalias: {}\nuuid: {}\npriority: {}\ncreated_at: {}\ncreated_by: {}\n",
        m.id,
        m.uuid,
        m.priority,
        quote(&m.created_at),
        quote(&m.created_by)
    );
    out.push_str(&format!("summary: {}\n", quote(&summary)));
    for (key, value) in [
        ("type", m.kind.as_deref()),
        ("part_of", m.part_of.as_deref()),
        ("issue_url", m.issue_url.as_deref()),
        ("imported_from", m.imported_from.as_deref()),
        ("due", m.due.as_deref()),
    ] {
        if let Some(value) = value {
            out.push_str(&format!("{key}: {}\n", quote(value)));
        }
    }
    if !m.labels.is_empty() {
        out.push_str("labels:\n");
        for label in &m.labels {
            out.push_str(&format!("  - {}\n", quote(label)));
        }
    }
    if let Some(issue) = m.issue {
        out.push_str(&format!("issue: {issue}\n"));
    }
    out.push_str("---\n");
    out.push_str(intent.trim_end());
    out.push_str("\n\n## Checklist\n");
    let content = checklist.strip_prefix("# Checklist").unwrap_or(checklist);
    out.push_str(content);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// What a ticket's work may touch and run. Each list is optional, and none
/// of them changes how the ticket ends. `tools` and `commands` are extensions
/// (FORMAT-1.0 7.1): a reader that ignored them would run the work with no
/// limit, so they are critical and the document lists them in `requires`
/// (7.3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scope {
    pub paths: Vec<String>,
    pub tools: Vec<String>,
    pub commands: Vec<String>,
    /// What it waits on beyond its own repository ([`super::depends`]). Not a
    /// limit on the work: it says when the work can start.
    pub depends_on: Vec<String>,
    /// A change request's class ([`super::change`]).
    pub change: Option<String>,
    /// The clauses a change request changes.
    pub clauses: Vec<String>,
}

/// `text` with `scope` in its frontmatter.
pub(crate) fn with_scope(text: &str, scope: &Scope) -> String {
    let block = |key: &str, values: &[String]| {
        let mut out = format!("{key}:\n");
        for v in values {
            out.push_str(&format!("  - {}\n", quote(v)));
        }
        out
    };
    let mut lines = String::new();
    if !scope.paths.is_empty() {
        lines.push_str(&block("paths", &scope.paths));
    }
    let mut requires = Vec::new();
    if let Some(change) = &scope.change {
        lines.push_str(&format!("{}: {change}\n", super::change::CHANGE_KEY));
        requires.push(super::change::CHANGE_KEY.to_owned());
        lines.push_str(&block(super::change::CLAUSES_KEY, &scope.clauses));
        requires.push(super::change::CLAUSES_KEY.to_owned());
    }
    if !scope.depends_on.is_empty() {
        lines.push_str(&block(super::depends::KEY, &scope.depends_on));
        requires.push(super::depends::KEY.to_owned());
    }
    if !scope.commands.is_empty() {
        lines.push_str(&block(super::checks::COMMANDS_KEY, &scope.commands));
        requires.push(super::checks::COMMANDS_KEY.to_owned());
    }
    if !scope.tools.is_empty() {
        lines.push_str(&block(super::checks::TOOLS_KEY, &scope.tools));
        requires.push(super::checks::TOOLS_KEY.to_owned());
    }
    if !requires.is_empty() {
        lines.push_str(&block("requires", &requires));
    }
    let mut out = text.to_owned();
    if let Some(at) = out.find("\n---\n") {
        out.insert_str(at + 1, &lines);
    }
    out
}

pub(crate) fn validate(text: &str) -> Result<(), String> {
    let doc = openwarrant_core::format::parse(text.as_bytes());
    let faults: Vec<_> = doc
        .findings
        .iter()
        .filter(|f| {
            f.outcome == openwarrant_core::format::Outcome::Refusal
                || f.outcome == openwarrant_core::format::Outcome::Unknown && doc.unknown()
        })
        .map(|f| format!("{} at line {}: {}", f.rule, f.line, f.message))
        .collect();
    if faults.is_empty() {
        Ok(())
    } else {
        Err(faults.join("; "))
    }
}

/// Publish a complete source without replacing a pre-existing document.
pub(crate) fn create(source: &camino::Utf8Path, text: &str) -> Result<(), RepoError> {
    validate(text).map_err(|why| RepoError::Message(format!("ticket.document-invalid: {why}")))?;
    let dir = source.parent().unwrap_or(source);
    // Named by the document: single-file tickets share one directory, and
    // two creates at once must not share one staging file.
    let temporary = dir.join(format!(
        ".{}.war-tmp",
        source.file_name().unwrap_or("source")
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(super::io(format!("could not stage {source}")))?;
    file.write_all(text.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(super::io(format!("could not write {source}")))?;
    let result = std::fs::hard_link(&temporary, source).map_err(super::io(format!(
        "could not publish {source} without replacement"
    )));
    let _ = std::fs::remove_file(&temporary);
    result
}

fn field(text: &str, key: &str, value: Option<&str>) -> String {
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = String::new();
    let mut inside = false;
    let mut matched = false;
    let mut skip_continuation = false;
    let emit = |out: &mut String, value: &str| {
        out.push_str(&format!(
            "{key}:{}{value}{nl}",
            if value.starts_with('\n') { "" } else { " " }
        ));
    };
    for (index, line) in text.split_inclusive('\n').enumerate() {
        if index == 0 {
            inside = line.trim() == "---";
            out.push_str(line);
            continue;
        }
        if inside && skip_continuation && line.starts_with("  ") {
            continue;
        }
        skip_continuation = false;
        if inside && line.trim() == "---" {
            if !matched && let Some(value) = value {
                emit(&mut out, value);
            }
            inside = false;
        }
        if inside && line.starts_with(&format!("{key}:")) {
            if let Some(value) = value {
                emit(&mut out, value);
            }
            matched = true;
            skip_continuation = true;
        } else {
            out.push_str(line);
        }
    }
    out
}

/// Apply only requested header edits; preserve other fields, units and IDs.
pub(crate) fn edit(
    store: &Store,
    t: &Ticket,
    changes: &serde_json::Map<String, serde_json::Value>,
    if_rev: Option<&str>,
) -> Result<Outcome, RepoError> {
    let written = super::rewrite(&t.intent_path, |text| {
        if let Some(given) = if_rev {
            let doc = openwarrant_core::format::parse(text.as_bytes());
            let current = super::revision_of(&text.as_bytes()[..doc.head_end]);
            if !super::same_revision(given, &current) {
                return Err(Box::new(super::stale_revision(
                    store,
                    t,
                    t.id(),
                    &t.intent_path,
                    given,
                    &current,
                )));
            }
        }
        let mut next = text.to_owned();
        for (key, value) in changes {
            let rendered = match value {
                serde_json::Value::Null => None,
                serde_json::Value::String(value) => Some(quote(value)),
                serde_json::Value::Array(values) => {
                    if values.is_empty() {
                        None
                    } else {
                        Some(
                            values
                                .iter()
                                .map(|v| format!("\n  - {}", quote(v.as_str().unwrap_or_default())))
                                .collect::<String>(),
                        )
                    }
                }
                _ => Some(value.to_string()),
            };
            next = field(&next, key, rendered.as_deref());
        }
        validate(&next).map_err(|why| {
            Box::new(Outcome::refused(
                "ticket.document-invalid",
                store.rel(&t.intent_path),
                why,
            ))
        })?;
        Ok((next, ()))
    })?;
    if let Err(refusal) = written {
        return Ok(*refusal);
    }
    // The same change can repeat (a priority set back, by someone else), and a
    // repeated payload is an idempotency conflict after the document was
    // written. The edit's own id keeps every edit's key distinct.
    store.journal(
        t,
        super::event::EDITED,
        &serde_json::json!({
            "ticket": t.id(),
            "changed": changes,
            "edit": super::WarUuid::mint().to_string(),
        }),
    )?;
    Ok(Outcome::ok(
        format!("{}: updated {}", t.id(), store.rel(&t.intent_path)),
        serde_json::json!({"schema":"oh.war/ticket-edit/v1","ticket":t.id(),"changed":changes}),
    ))
}

/// Translate validated parts to records in the same source, never a side atom.
pub(crate) fn append_parts(
    store: &Store,
    t: &Ticket,
    parts: &[serde_json::Value],
) -> Result<(), RepoError> {
    let written = super::rewrite(&t.intent_path, |text| {
        with_parts(t.id(), text, parts)
            .map(|next| (next, ()))
            .map_err(|(rule, why)| Box::new(Outcome::refused(rule, store.rel(&t.intent_path), why)))
    })?;
    written.map_err(|r| RepoError::Message(r.human))
}

/// The document `text` (alias `alias`) with `parts` appended as records, as
/// `war plan add` writes them ([`append_parts`]): a test or KPI is a
/// `CHK-<name>` record, a milestone an `M-<item>` record holding its item.
/// A test or KPI `evaluates` its `item`, else every item the document has;
/// a payload's own `"evaluates"` names the target instead (`war admin
/// migrate`: a whole-Warrant part evaluates the Warrant). `Err`: the rule
/// and why, nothing written.
pub(crate) fn with_parts(
    alias: &str,
    text: &str,
    parts: &[serde_json::Value],
) -> Result<String, (&'static str, String)> {
    let mut next = text.trim_end().to_owned();
    next.push('\n');
    let doc = openwarrant_core::format::parse(text.as_bytes());
    let ids: Vec<_> = doc
        .units
        .iter()
        .flat_map(|u| u.items.iter())
        .filter_map(|i| i.id.as_deref())
        .collect();
    for part in parts {
        let kind = part["kind"].as_str().unwrap_or_default();
        let name = part["name"].as_str().unwrap_or("milestone");
        if kind == "milestone" {
            let item = part["item"].as_str().unwrap_or_default();
            let marker = format!("M-{item}");
            let parsed = openwarrant_core::format::parse(next.as_bytes());
            if let Some(unit) = parsed.units.iter().find(|u| u.id == marker) {
                let span = &next[unit.start..unit.end];
                let mut lines = span.lines().map(str::to_owned).collect::<Vec<_>>();
                lines.retain(|line| !line.starts_with("min:"));
                if let Some(min) = part["min"].as_str() {
                    lines.insert(1, format!("min: {min}"));
                }
                next.replace_range(unit.start..unit.end, &(lines.join("\n") + "\n"));
            } else {
                let checklist = super::format::checklist_of(&next);
                let target = checklist.item(item).ok_or_else(|| {
                    (
                        "ticket.item-unknown",
                        format!("milestone item {item} is unavailable"),
                    )
                })?;
                let start = next
                    .split_inclusive('\n')
                    .take(target.line)
                    .map(str::len)
                    .sum::<usize>();
                let line = next
                    .split_inclusive('\n')
                    .nth(target.line)
                    .unwrap_or_default()
                    .to_owned();
                next.replace_range(start..start + line.len(), "");
                next.push_str(&format!("\n## {marker} · milestone — {}\n", target.text));
                if let Some(min) = part["min"].as_str() {
                    next.push_str(&format!("min: {min}\n"));
                }
                next.push('\n');
                next.push_str(&line);
                if !next.ends_with('\n') {
                    next.push('\n');
                }
            }
            continue;
        }
        let command = part["cmd"].as_str().unwrap_or_default();
        if command.contains(['\r', '\n']) {
            return Err((
                "ticket.part-malformed",
                "a run is one command line; put a multiline program in a script file".to_owned(),
            ));
        }
        let triggers = part["on"].as_str().unwrap_or("done");
        next.push_str(&format!(
            "\n## CHK-{name} · {} — {name}\nrun: {command}\non: {triggers}\n",
            if kind == "kpi" { "kpi" } else { "check" }
        ));
        if let Some(timeout) = part["timeout"].as_u64() {
            next.push_str(&format!("timeout: {timeout}\n"));
        }
        if let Some(reopen) = part["reopen"].as_bool() {
            next.push_str(&format!("reopen: {reopen}\n"));
        }
        let targets = match (part["evaluates"].as_str(), part["item"].as_str()) {
            (Some(target), _) => vec![target],
            (None, Some(id)) => vec![id],
            (None, None) => ids.clone(),
        };
        next.push_str(&format!(
            "evaluates {}\n",
            if targets.is_empty() {
                alias.to_owned()
            } else {
                targets.join(", ")
            }
        ));
        if kind == "kpi" {
            for key in ["direction", "mode"] {
                if let Some(value) = part[key].as_str() {
                    next.push_str(&format!("{key}: {value}\n"));
                }
            }
            if !part["target"].is_null() {
                next.push_str(&format!("target: {}\n", part["target"]));
            }
        }
    }
    validate(&next).map_err(|why| ("ticket.part-malformed", why))?;
    Ok(next)
}
