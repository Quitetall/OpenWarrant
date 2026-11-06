// SPDX-License-Identifier: Apache-2.0
//! Markdown milestone tables use the existing import planner and Warrant writer.
//! Re-imports never overwrite maintained work; changed rows become proposals.
use super::{
    Fault, digest, heading, resolve, shown, unfenced,
    write::{self, Import, Incoming, IncomingItem},
};
use crate::{
    repo::{RepoError, Repository},
    ticket::{Outcome, Store},
};
use camino::Utf8Path;
use serde_json::json;
use std::collections::BTreeSet;

const ROW_MARKER: &str = "<!-- openwarrant-table-row: ";

fn cells(line: &str) -> Option<Vec<String>> {
    let line = line.trim();
    if !line.starts_with('|') || !line.ends_with('|') {
        return None;
    }
    let mut values = Vec::new();
    let mut part = String::new();
    let mut escaped = false;
    for c in line[1..line.len() - 1].chars() {
        if escaped {
            part.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '|' {
            values.push(part.trim().to_owned());
            part.clear();
        } else {
            part.push(c);
        }
    }
    if escaped {
        part.push('\\');
    }
    values.push(part.trim().to_owned());
    Some(values)
}

fn column(headers: &[String], names: &[&str]) -> Result<Option<usize>, String> {
    let found: Vec<_> = headers
        .iter()
        .enumerate()
        .filter(|(_, h)| names.iter().any(|n| h.eq_ignore_ascii_case(n)))
        .map(|(i, _)| i)
        .collect();
    match found.as_slice() {
        [] => Ok(None),
        [i] => Ok(Some(*i)),
        _ => Err(format!(
            "ambiguous columns for {}; keep one or rename the others",
            names.join("/")
        )),
    }
}

pub fn run(
    repo: &Repository,
    store: &Store,
    path: &Utf8Path,
    section: Option<&str>,
) -> Result<Outcome, RepoError> {
    let path = resolve(repo, path);
    let file = shown(repo, &path);
    let refuse =
        |rule, line, message: String| write::refused(&[Fault::new(rule, &file, line, message)]);
    let relative = match path.strip_prefix(&repo.root) {
        Ok(p) if !p.components().any(|c| c.as_str() == "..") => p.to_string(),
        _ => return Ok(refuse("table.source-path", 0, "table source must be inside the repository so its identity and preserved path are portable".into())),
    };
    let bytes = match crate::vfs::read_regular(repo.root.as_std_path(), path.as_std_path()) {
        Ok(b) if b.len() <= 4 * 1024 * 1024 => b,
        Ok(_) => {
            return Ok(refuse(
                "table.source-limit",
                0,
                "table source exceeds 4 MiB".into(),
            ));
        }
        Err(e) => return Ok(refuse("table.source-unreadable", 0, e.to_string())),
    };
    let text = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => return Ok(refuse("table.source-encoding", 0, e.to_string())),
    };
    let lines = unfenced(&text);
    let mut current = String::new();
    let mut candidates = Vec::new();
    for (i, (line, raw)) in lines.iter().enumerate() {
        if let Some((_, name)) = heading(raw) {
            current = name.to_owned();
        }
        if section.is_some_and(|s| s != current) {
            continue;
        }
        let Some(headers) = cells(raw) else {
            continue;
        };
        let Some((_, separator)) = lines.get(i + 1) else {
            continue;
        };
        let Some(separators) = cells(separator) else {
            continue;
        };
        if separators.len() != headers.len()
            || !separators.iter().all(|s| {
                let t = s.trim_matches(':');
                t.len() >= 3 && t.bytes().all(|b| b == b'-')
            })
        {
            continue;
        }
        let key = match column(&headers, &["id", "package", "milestone"]) {
            Ok(v) => v,
            Err(e) => return Ok(refuse("table.mapping-ambiguous", *line, e)),
        };
        let title = match column(&headers, &["outcome", "title", "description"]) {
            Ok(v) => v,
            Err(e) => return Ok(refuse("table.mapping-ambiguous", *line, e)),
        };
        let (Some(key), Some(title)) = (key, title) else {
            continue;
        };
        let state = match column(&headers, &["engineering state", "state", "status"]) {
            Ok(v) => v,
            Err(e) => return Ok(refuse("table.mapping-ambiguous", *line, e)),
        };
        let dependencies = match column(&headers, &["depends on", "depends_on", "after"]) {
            Ok(v) => v,
            Err(e) => return Ok(refuse("table.mapping-ambiguous", *line, e)),
        };
        candidates.push((i, current.clone(), headers, key, title, state, dependencies));
    }
    if candidates.len() != 1 {
        return Ok(refuse(
            "table.mapping-ambiguous",
            0,
            format!(
                "found {} matching tables; select one with --section and use one ID/Package/Milestone column plus one Outcome/Title/Description column",
                candidates.len()
            ),
        ));
    }
    let (start, section, headers, key, title, state, dependencies) = candidates.remove(0);
    let snapshot = format!(
        "docs/imports/table/{}/sources/{relative}",
        openwarrant_compiler::sha256_hex(text.as_bytes())
    );
    let (existing, diagnostics) = store.load_all()?;
    if !diagnostics.is_empty() {
        return Ok(refuse(
            "table.store-invalid",
            0,
            "fix unreadable Warrant records before importing; no mapping is guessed".into(),
        ));
    }
    let mut import = Import {
        format: "table",
        origin: file.clone(),
        records: vec![(snapshot.clone(), text.clone())],
        ..Import::default()
    };
    let mut keys = BTreeSet::new();
    let mut proposals = Vec::new();
    for (line, raw) in lines.iter().skip(start + 2) {
        let Some(row) = cells(raw) else {
            break;
        };
        if row.len() != headers.len() {
            return Ok(refuse(
                "table.row-shape",
                *line,
                "row has a different number of cells; escape literal pipes as \\|".into(),
            ));
        }
        let id = row[key].trim_matches('`').trim();
        if id.is_empty() || !keys.insert(id.to_owned()) {
            return Ok(refuse(
                "table.row-identity",
                *line,
                format!("missing or duplicate row ID {id:?}; use a unique stable key"),
            ));
        }
        if row[title].is_empty() {
            return Ok(refuse(
                "table.row-title",
                *line,
                "outcome/title is empty".into(),
            ));
        }
        let identity = format!("table:{relative}#{section}/{id}");
        let row_digest = digest(
            &serde_jcs::to_vec(&json!({"headers":headers,"row":row}))
                .map_err(|e| RepoError::Message(e.to_string()))?,
        );
        let mut body = format!(
            "Imported from `{relative}`, section `{section}`, row `{id}`.\n\n{ROW_MARKER}{row_digest} -->\n\n"
        );
        for (name, value) in headers.iter().zip(&row) {
            body.push_str(&format!("> **{name}:** {value}\n>\n"));
        }
        body.push_str(&format!("\nOriginal document: [{relative}]({snapshot}).\n\nImported completion is a source claim. It grants no assurance or authority.\n"));
        let status = state.map_or("", |i| row[i].as_str()).to_ascii_lowercase();
        let done = matches!(
            status.as_str(),
            "done" | "complete" | "completed" | "implemented"
        );
        if let Some(old) = existing
            .iter()
            .find(|t| t.manifest.imported_from.as_deref() == Some(&identity))
        {
            let old_digest = old
                .intent
                .lines()
                .find_map(|l| l.strip_prefix(ROW_MARKER))
                .and_then(|s| s.strip_suffix(" -->"));
            if old_digest != Some(row_digest.as_str()) {
                let base = digest(old.checklist_text.as_bytes());
                let change = json!({"id":old.id(),"source":identity,"base":base,"before":old.intent,"after":body,"row_revision":row_digest,"qualification_established":false});
                let proposal_digest = digest(
                    &serde_jcs::to_vec(&change).map_err(|e| RepoError::Message(e.to_string()))?,
                );
                let mut change = change;
                change["proposal_digest"] = json!(proposal_digest);
                proposals.push(change);
            }
        }
        import.incoming.push(Incoming {
            source: identity,
            title: row[title].clone(),
            body,
            notes: vec![],
            priority: 2,
            kind: None,
            labels: vec![],
            part_of: None,
            created_at: crate::gate_cmd::receipt::rfc3339_from_secs(crate::ticket::now_secs()),
            created_by: store.actor.clone(),
            items: vec![IncomingItem {
                key: id.to_owned(),
                text: row[title].clone(),
                done,
                done_by: Some("imported-source-claim".into()),
                done_on: None,
                note: Some("Unverified source-table claim".into()),
                after: dependencies.map_or_else(Vec::new, |column| {
                    row[column]
                        .split([',', ';'])
                        .map(|s| s.trim().trim_matches('`').trim())
                        .filter(|s| !s.is_empty() && *s != "-" && !s.eq_ignore_ascii_case("none"))
                        .map(|key| write::Ref::Warrant(format!("table:{relative}#{section}/{key}")))
                        .collect()
                }),
            }],
        });
    }
    if import.incoming.is_empty() {
        return Ok(refuse(
            "table.empty",
            0,
            "selected table has no data rows".into(),
        ));
    }
    let mut out = write::apply(repo, store, &import)?;
    if !out.is_refused() {
        out.result["source_snapshot"] = json!(snapshot);
        out.result["proposals"] = json!(proposals);
        if !proposals.is_empty() {
            out.human.push_str("\nChanged source rows are proposals; maintained Warrants and local edits were not overwritten.");
        }
    }
    Ok(out)
}
