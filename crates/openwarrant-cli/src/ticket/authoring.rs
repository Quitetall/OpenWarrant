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
        out.push_str(&format!(
            "labels: [{}]\n",
            m.labels
                .iter()
                .map(|v| quote(v))
                .collect::<Vec<_>>()
                .join(", ")
        ));
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
pub(crate) fn create(
    store: &Store,
    dir: &camino::Utf8Path,
    text: &str,
) -> Result<camino::Utf8PathBuf, RepoError> {
    validate(text).map_err(|why| RepoError::Message(format!("ticket.document-invalid: {why}")))?;
    let source = store
        .dir
        .join(format!("{}.md", dir.file_name().unwrap_or_default()));
    let temporary = dir.join(".source.war-tmp");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(super::io(format!("could not stage {source}")))?;
    file.write_all(text.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(super::io(format!("could not write {source}")))?;
    let result = std::fs::hard_link(&temporary, &source).map_err(super::io(format!(
        "could not publish {source} without replacement"
    )));
    let _ = std::fs::remove_file(&temporary);
    result?;
    Ok(source)
}

fn field(text: &str, key: &str, value: Option<&str>) -> String {
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = String::new();
    let mut inside = false;
    let mut matched = false;
    for (index, line) in text.split_inclusive('\n').enumerate() {
        if index == 0 {
            inside = line.trim() == "---";
            out.push_str(line);
            continue;
        }
        if inside && line.trim() == "---" {
            if !matched && let Some(value) = value {
                out.push_str(&format!("{key}: {value}{nl}"));
            }
            inside = false;
        }
        if inside && line.starts_with(&format!("{key}:")) {
            if let Some(value) = value {
                out.push_str(&format!("{key}: {value}{nl}"));
            }
            matched = true;
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
            let current = super::revision_of(text[..doc.head_end].as_bytes());
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
                serde_json::Value::Array(values) => Some(format!(
                    "[{}]",
                    values
                        .iter()
                        .map(|v| quote(v.as_str().unwrap_or_default()))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
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
    store.journal(
        t,
        super::event::EDITED,
        &serde_json::json!({"ticket":t.id(),"changed":changes}),
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
                        Box::new(Outcome::refused(
                            "ticket.item-unknown",
                            store.rel(&t.intent_path),
                            format!("milestone item {item} is unavailable"),
                        ))
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
            next.push_str(&format!(
                "\n## CHK-{name} · {} — {name}\nrun: {}\non: done\n",
                if kind == "kpi" { "kpi" } else { "check" },
                quote(part["cmd"].as_str().unwrap_or_default())
            ));
            let targets = part["item"]
                .as_str()
                .map_or_else(|| ids.clone(), |id| vec![id]);
            next.push_str(&format!(
                "evaluates {}\n",
                if targets.is_empty() {
                    t.id().to_owned()
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
        validate(&next).map_err(|why| {
            Box::new(Outcome::refused(
                "ticket.part-malformed",
                store.rel(&t.intent_path),
                why,
            ))
        })?;
        Ok((next, ()))
    })?;
    written.map_err(|r| RepoError::Message(r.human))
}
