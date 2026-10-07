// SPDX-License-Identifier: Apache-2.0
//! An OpenSpec folder, read (OW-WAR-0148 M10).
//!
//! The format is OpenSpec's own (github.com/Fission-AI/OpenSpec, read at
//! 9111a765, 2026-10-06: `docs/concepts.md`, `openspec/changes/*` and
//! `src/core/parsers/requirement-blocks.ts`):
//!
//! ```text
//! openspec/
//!   specs/<capability>/spec.md        the current requirements
//!   changes/<change>/proposal.md      why and what
//!   changes/<change>/tasks.md         `- [ ] 1.1 ...` under `## 1. ...`
//!   changes/<change>/design.md        how (optional)
//!   changes/<change>/.openspec.yaml   `created: 2026-09-28` (optional)
//!   changes/<change>/specs/<capability>/spec.md
//!                                     deltas: `## ADDED|MODIFIED|REMOVED|RENAMED
//!                                     Requirements`, each `### Requirement: <name>`
//!   changes/archive/                  finished changes, not read
//! ```
//!
//! Read as: each change a Warrant (`openspec:<change>`), each task an item
//! (`openspec:<change>/1.1`, part of the change), each requirement a record
//! (`openspec:<capability>#<name as a slug>`, from the current spec, or from
//! the delta that adds it). A change `implements` the requirements its
//! deltas add or modify; it `openspec.removes` and `openspec.renames` the
//! ones it removes or renames (namespaced: carried, inert).
//!
//! Refused by rule, each naming its file and line: a task line that is not
//! one (`openspec.tasks-malformed`, also a task number used twice), a
//! requirement header outside the four delta sections or a FROM with no TO
//! (`openspec.delta-malformed`), a requirement declared twice in one spec
//! (`openspec.requirement-duplicate`), and a folder with neither `changes/`
//! nor `specs/` (`openspec.missing`).

use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};

use super::{Fault, Foreign, ForeignRecord, Task, Tree, heading, shown, slug, unfenced};
use crate::repo::Repository;

pub const KIND: &str = "openspec";
const TASKS: &str = "openspec.tasks-malformed";
const DELTA: &str = "openspec.delta-malformed";

/// The folder to read under `path`: `path` itself when it holds `changes/`
/// or `specs/`, else `path/openspec` when that does.
#[must_use]
pub fn locate(path: &Utf8Path) -> Option<Utf8PathBuf> {
    let holds =
        |p: &Utf8Path| crate::vfs::is_dir(p.join("changes")) || crate::vfs::is_dir(p.join("specs"));
    if holds(path) {
        Some(path.to_owned())
    } else if holds(&path.join("openspec")) {
        Some(path.join("openspec"))
    } else {
        None
    }
}

/// `### Requirement: <name>` → `<name>`.
fn requirement_name(line: &str) -> Option<&str> {
    let (level, text) = heading(line)?;
    if level != 3 {
        return None;
    }
    let (word, rest) = text.split_once(':')?;
    word.trim()
        .eq_ignore_ascii_case("requirement")
        .then(|| rest.trim())
        .filter(|n| !n.is_empty())
}

/// The record id of requirement `name` of `capability`.
#[must_use]
pub fn requirement_id(capability: &str, name: &str) -> String {
    format!("{KIND}:{capability}#{}", slug(name))
}

/// Every requirement block of a current spec: (name, 1-based line, the
/// block's bytes, from its header to the next header of level 3 or less).
fn blocks(text: &str) -> Vec<(String, usize, String)> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let visible: BTreeSet<usize> = unfenced(text).into_iter().map(|(n, _)| n).collect();
    let mut out = Vec::new();
    let mut open: Option<(String, usize, String)> = None;
    for (i, raw) in lines.iter().enumerate() {
        let n = i + 1;
        let is_heading = visible.contains(&n)
            && heading(raw.trim_end_matches(['\n', '\r'])).is_some_and(|(l, _)| l <= 3);
        if is_heading {
            if let Some(done) = open.take() {
                out.push(done);
            }
            if let Some(name) = requirement_name(raw.trim_end_matches(['\n', '\r'])) {
                open = Some((name.to_owned(), n, String::new()));
            }
        }
        if let Some((_, _, body)) = open.as_mut() {
            body.push_str(raw);
        }
    }
    if let Some(done) = open.take() {
        out.push(done);
    }
    out
}

/// Which delta section a `## ` heading opens, if it is one of the four.
fn section(text: &str) -> Option<&'static str> {
    let t = text.split_whitespace().collect::<Vec<_>>().join(" ");
    ["ADDED", "MODIFIED", "REMOVED", "RENAMED"]
        .into_iter()
        .find(|w| t.eq_ignore_ascii_case(&format!("{w} Requirements")))
}

/// A `FROM:`/`TO:` line of a RENAMED section, or a REMOVED bullet naming
/// a requirement: (`FROM`|`TO`|`-`, name).
fn named_line(line: &str) -> Option<(&'static str, String)> {
    let t = line
        .trim_start()
        .trim_start_matches(['-', '*', '+'])
        .trim_start();
    let (side, rest) = if let Some(r) = t.strip_prefix("FROM:") {
        ("FROM", r)
    } else if let Some(r) = t.strip_prefix("TO:") {
        ("TO", r)
    } else if line.trim_start().starts_with(['-', '*', '+']) {
        ("-", t)
    } else {
        return None;
    };
    let rest = rest.trim().trim_matches('`').trim();
    requirement_name(rest).map(|n| (side, n.to_owned()))
}

/// Read the OpenSpec folder at `root` (already located).
#[must_use]
pub fn read(repo: &Repository, root: &Utf8Path) -> Tree {
    let mut tree = Tree {
        kind: KIND,
        root: root.to_owned(),
        ..Tree::default()
    };
    if !crate::vfs::is_dir(root.join("changes")) && !crate::vfs::is_dir(root.join("specs")) {
        tree.faults.push(Fault::new(
            "openspec.missing",
            &shown(repo, root),
            0,
            "neither changes/ nor specs/ is here; an OpenSpec folder holds them",
        ));
        return tree;
    }
    // ---- the current specs: one record per requirement.
    let mut known: BTreeMap<String, usize> = BTreeMap::new();
    for cap in super::subdirs(&root.join("specs")) {
        let capability = cap.file_name().unwrap_or_default().to_owned();
        let path = cap.join("spec.md");
        let file = shown(repo, &path);
        let text = match super::read_optional(&path) {
            Ok(Some(t)) => t,
            Ok(None) => continue,
            Err(e) => {
                tree.faults
                    .push(Fault::new("openspec.unreadable", &file, 0, e));
                continue;
            }
        };
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        for (name, line, body) in blocks(&text) {
            let id = requirement_id(&capability, &name);
            if let Some(first) = seen.insert(id.clone(), line) {
                tree.faults.push(Fault::new(
                    "openspec.requirement-duplicate",
                    &file,
                    line,
                    format!(
                        "requirement {name:?} is also on line {first}; a name says one requirement"
                    ),
                ));
                continue;
            }
            known.insert(id.clone(), tree.records.len());
            tree.records.push(ForeignRecord {
                id,
                kind: "requirement",
                title: name,
                source: file.clone(),
                line,
                revision: super::digest(body.as_bytes()),
            });
        }
    }
    // ---- the changes.
    for dir in super::subdirs(&root.join("changes")) {
        let name = dir.file_name().unwrap_or_default().to_owned();
        if name == "archive" {
            continue;
        }
        let id = format!("{KIND}:{name}");
        let proposal = dir.join("proposal.md");
        let proposal_text = match super::read_optional(&proposal) {
            Ok(t) => t,
            Err(e) => {
                tree.faults.push(Fault::new(
                    "openspec.unreadable",
                    &shown(repo, &proposal),
                    0,
                    e,
                ));
                None
            }
        };
        let (title, description) = proposal_text
            .as_deref()
            .map(title_and_why)
            .unwrap_or_default();
        let title = title
            .filter(|t| !t.is_empty() && !t.eq_ignore_ascii_case("proposal"))
            .unwrap_or_else(|| name.replace('-', " "));
        let created = super::read_optional(&dir.join(".openspec.yaml"))
            .ok()
            .flatten()
            .and_then(|y| {
                y.lines()
                    .find_map(|l| l.strip_prefix("created:"))
                    .map(|v| v.trim().trim_matches(['"', '\'']).to_owned())
            })
            .and_then(|d| super::timestamp(&d));
        let mut w = Foreign {
            id: id.clone(),
            name: name.clone(),
            title,
            description,
            source: shown(
                repo,
                if proposal_text.is_some() {
                    &proposal
                } else {
                    &dir
                },
            ),
            revision: super::digest(proposal_text.as_deref().unwrap_or_default().as_bytes()),
            dir: dir.clone(),
            created,
            ..Foreign::default()
        };
        // Tasks.
        let tasks_path = dir.join("tasks.md");
        match super::read_optional(&tasks_path) {
            Ok(Some(text)) => {
                let file = shown(repo, &tasks_path);
                let (lines, faults) = super::task_lines(&text, &file, TASKS);
                tree.faults.extend(faults);
                let mut first: BTreeMap<String, usize> = BTreeMap::new();
                let mut unnumbered = 0;
                for t in lines {
                    let key = match task_number(&t.text) {
                        Some((n, _)) => n.to_owned(),
                        None => {
                            unnumbered += 1;
                            format!("task-{unnumbered}")
                        }
                    };
                    if let Some(at) = first.insert(key.clone(), t.line) {
                        tree.faults.push(Fault::new(
                            TASKS,
                            &file,
                            t.line,
                            format!("task {key} is also on line {at}; a number names one task"),
                        ));
                        continue;
                    }
                    w.tasks.push(Task {
                        id: format!("{id}/{key}"),
                        key,
                        text: t.text.clone(),
                        done: t.done,
                        line: t.line,
                        revision: t.revision,
                        after: Vec::new(),
                    });
                }
                w.tasks_file = Some(file);
            }
            Ok(None) => {}
            Err(e) => tree.faults.push(Fault::new(
                "openspec.unreadable",
                &shown(repo, &tasks_path),
                0,
                e,
            )),
        }
        // Deltas.
        for cap in super::subdirs(&dir.join("specs")) {
            let capability = cap.file_name().unwrap_or_default().to_owned();
            let path = cap.join("spec.md");
            let file = shown(repo, &path);
            let text = match super::read_optional(&path) {
                Ok(Some(t)) => t,
                Ok(None) => continue,
                Err(e) => {
                    tree.faults
                        .push(Fault::new("openspec.unreadable", &file, 0, e));
                    continue;
                }
            };
            let added: BTreeMap<usize, (String, String)> = blocks(&text)
                .into_iter()
                .map(|(name, line, body)| (line, (name, body)))
                .collect();
            let mut current: Option<&'static str> = None;
            let mut pending_from: Option<(String, usize)> = None;
            for (n, raw) in unfenced(&text) {
                if let Some((level, h)) = heading(raw)
                    && level <= 2
                {
                    if let Some((from, at)) = pending_from.take() {
                        tree.faults.push(Fault::new(
                            DELTA,
                            &file,
                            at,
                            format!("FROM {from:?} has no TO; a rename names both"),
                        ));
                    }
                    current = section(h);
                    continue;
                }
                if let Some(req) = requirement_name(raw) {
                    let rid = requirement_id(&capability, req);
                    match current {
                        Some(word @ ("ADDED" | "MODIFIED")) => {
                            if !known.contains_key(&rid) {
                                let body = added.get(&n).map(|(_, b)| b.as_str()).unwrap_or(raw);
                                known.insert(rid.clone(), tree.records.len());
                                tree.records.push(ForeignRecord {
                                    id: rid.clone(),
                                    kind: "requirement",
                                    title: req.to_owned(),
                                    source: file.clone(),
                                    line: n,
                                    revision: super::digest(body.as_bytes()),
                                });
                            }
                            tree.relations
                                .push((id.clone(), "implements".to_owned(), rid.clone()));
                            w.changes
                                .push(format!("{word} {capability}: {req} (`{rid}`)"));
                        }
                        Some("REMOVED") => {
                            tree.relations.push((
                                id.clone(),
                                "openspec.removes".to_owned(),
                                rid.clone(),
                            ));
                            w.changes
                                .push(format!("REMOVED {capability}: {req} (`{rid}`)"));
                        }
                        _ => tree.faults.push(Fault::new(
                            DELTA,
                            &file,
                            n,
                            format!(
                                "requirement {req:?} is outside an ADDED, MODIFIED, REMOVED or \
                                 RENAMED Requirements section, so what the change does to it is \
                                 not said"
                            ),
                        )),
                    }
                    continue;
                }
                match (current, named_line(raw)) {
                    (Some("REMOVED"), Some(("-", req))) => {
                        let rid = requirement_id(&capability, &req);
                        tree.relations.push((
                            id.clone(),
                            "openspec.removes".to_owned(),
                            rid.clone(),
                        ));
                        w.changes
                            .push(format!("REMOVED {capability}: {req} (`{rid}`)"));
                    }
                    (Some("RENAMED"), Some(("FROM", req))) => {
                        if let Some((from, at)) = pending_from.replace((req, n)) {
                            tree.faults.push(Fault::new(
                                DELTA,
                                &file,
                                at,
                                format!("FROM {from:?} has no TO; a rename names both"),
                            ));
                        }
                    }
                    (Some("RENAMED"), Some(("TO", req))) => match pending_from.take() {
                        Some((from, _)) => {
                            let rid = requirement_id(&capability, &from);
                            tree.relations.push((
                                id.clone(),
                                "openspec.renames".to_owned(),
                                rid.clone(),
                            ));
                            w.changes
                                .push(format!("RENAMED {capability}: {from} to {req} (`{rid}`)"));
                        }
                        None => tree.faults.push(Fault::new(
                            DELTA,
                            &file,
                            n,
                            format!("TO {req:?} follows no FROM; a rename names both"),
                        )),
                    },
                    _ => {}
                }
            }
            if let Some((from, at)) = pending_from.take() {
                tree.faults.push(Fault::new(
                    DELTA,
                    &file,
                    at,
                    format!("FROM {from:?} has no TO; a rename names both"),
                ));
            }
        }
        for t in &w.tasks {
            tree.relations
                .push((t.id.clone(), "part_of".to_owned(), id.clone()));
        }
        tree.warrants.push(w);
    }
    tree
}

/// `war import openspec <dir>`: each change a Warrant in the light
/// encoding, its tasks the items, its spec deltas named in its
/// description. Refused, with every fault named, when anything in the
/// folder does not read.
pub fn import(
    repo: &Repository,
    store: &crate::ticket::Store,
    path: &Utf8Path,
) -> Result<super::write::Import, Vec<Fault>> {
    let Some(root) = locate(path) else {
        return Err(vec![Fault::new(
            "openspec.missing",
            &shown(repo, path),
            0,
            "neither changes/ nor specs/, nor an openspec/ folder holding them, is here",
        )]);
    };
    let tree = read(repo, &root);
    if !tree.faults.is_empty() {
        return Err(tree.faults);
    }
    let now = crate::gate_cmd::receipt::rfc3339_from_secs(crate::ticket::now_secs());
    let incoming = tree
        .warrants
        .iter()
        .map(|w| {
            let mut body = w.description.clone();
            if !body.is_empty() {
                body.push_str("\n\n");
            }
            body.push_str(&format!(
                "## From OpenSpec\n\nImported from the OpenSpec change `changes/{}/`.",
                w.name
            ));
            if !w.changes.is_empty() {
                body.push_str("\n\nRequirements it changes:\n");
                for c in &w.changes {
                    body.push_str(&format!("\n- {c}"));
                }
            }
            super::write::Incoming {
                source: w.id.clone(),
                title: w.title.clone(),
                body,
                notes: Vec::new(),
                priority: openwarrant_core::ticket::DEFAULT_PRIORITY,
                kind: None,
                labels: Vec::new(),
                part_of: None,
                created_at: w.created.clone().unwrap_or_else(|| now.clone()),
                created_by: store.actor.clone(),
                items: w
                    .tasks
                    .iter()
                    .map(|t| super::write::IncomingItem {
                        key: t.key.clone(),
                        text: t.text.clone(),
                        done: t.done,
                        done_by: t.done.then(|| "unknown".to_owned()),
                        done_on: None,
                        note: None,
                        after: Vec::new(),
                    })
                    .collect(),
            }
        })
        .collect();
    Ok(super::write::Import {
        format: KIND,
        origin: shown(repo, &root),
        incoming,
        records: Vec::new(),
        warnings: Vec::new(),
    })
}

/// A task's leading number (`1.1`, `2.`, `3.10`) and the text after it.
fn task_number(text: &str) -> Option<(&str, &str)> {
    let end = text
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_digit() || *c == '.'))
        .map_or(text.len(), |(i, _)| i);
    let number = text[..end].trim_end_matches('.');
    let rest = &text[end..];
    (!number.is_empty()
        && number.starts_with(|c: char| c.is_ascii_digit())
        && (rest.is_empty() || rest.starts_with(' ')))
    .then(|| (number, rest.trim()))
}

/// A proposal's title (its first `# ` heading, `Change:` or `Proposal:`
/// dropped) and its first section's text (`## Why`, as written).
fn title_and_why(text: &str) -> (Option<String>, String) {
    let mut title = None;
    let mut why: Vec<&str> = Vec::new();
    let mut in_first = false;
    let mut sections = 0;
    for (_, raw) in unfenced(text) {
        if let Some((level, h)) = heading(raw) {
            if level == 1 && title.is_none() {
                let h = h
                    .strip_prefix("Change:")
                    .or_else(|| h.strip_prefix("Proposal:"))
                    .unwrap_or(h);
                title = Some(h.trim().to_owned());
                continue;
            }
            if level == 2 {
                sections += 1;
                in_first = sections == 1;
                continue;
            }
        }
        if in_first {
            why.push(raw);
        }
    }
    (title, why.join("\n").trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_numbers_and_requirement_headers() {
        assert_eq!(task_number("1.1 Add it"), Some(("1.1", "Add it")));
        assert_eq!(task_number("2. Then"), Some(("2", "Then")));
        assert_eq!(task_number("Unnumbered"), None);
        assert_eq!(task_number("1.1x"), None);
        assert_eq!(
            requirement_name("### Requirement: Report the version"),
            Some("Report the version")
        );
        assert_eq!(requirement_name("#### Scenario: x"), None);
        assert_eq!(section("ADDED  Requirements"), Some("ADDED"));
        assert_eq!(section("Added Requirements"), Some("ADDED"));
        assert_eq!(section("Purpose"), None);
        assert_eq!(
            named_line("- FROM: `### Requirement: Old name`"),
            Some(("FROM", "Old name".to_owned()))
        );
        assert_eq!(
            named_line("- `### Requirement: Gone`"),
            Some(("-", "Gone".to_owned()))
        );
    }

    #[test]
    fn requirement_blocks_end_at_the_next_header() {
        let text = "# X\n\n## Requirements\n### Requirement: One\nThe system SHALL a.\n\n#### Scenario: s\n- **WHEN** x\n\n### Requirement: Two\nThe system SHALL b.\n";
        let got: Vec<(String, usize)> = blocks(text).into_iter().map(|(n, l, _)| (n, l)).collect();
        assert_eq!(got, vec![("One".to_owned(), 4), ("Two".to_owned(), 10)]);
        let (_, _, body) = &blocks(text)[0];
        assert!(body.contains("#### Scenario: s") && !body.contains("Two"));
    }
}
