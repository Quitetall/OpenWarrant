// SPDX-License-Identifier: Apache-2.0
//! Beads' issue JSONL, in and out (OW-WAR-0148 M10).
//!
//! The format is the one `bd export` writes (github.com/gastownhall/beads,
//! formerly steveyegge/beads; read at 77ab9807, 2026-10-07):
//! `internal/types/types.go` (`Issue`, `Dependency`, `Comment`, the
//! statuses and issue types) and `cmd/bd/export.go` (one object per line,
//! `"_type":"issue"`, with `dependency_count`, `dependent_count` and
//! `comment_count`). A `parent-child` dependency has the child as
//! `issue_id` and the parent as `depends_on_id`; a `blocks` dependency has
//! the waiting issue as `issue_id`.
//!
//! # The mapping
//!
//! | Beads | a Warrant in the light encoding |
//! |---|---|
//! | `id` | `imported_from = "beads:<id>"`; the Warrant's own id is derived from it |
//! | `title` | the title, and the text of its one item |
//! | `description`, `design`, `acceptance_criteria`, `notes` | the description; the last three as `## Design`, `## Acceptance criteria`, `## Notes from Beads` |
//! | `status` | `closed`: the item ticked (`closed_at`, `close_reason` on its line); any other built-in status: open |
//! | `priority` | the priority (0 to 4, the same scale) |
//! | `issue_type` | the type, when the ticket profile declares it |
//! | `labels` | the labels |
//! | `blocks` dependency | the item waits on that Warrant (`after t-…`) |
//! | `parent-child` dependency | `part_of` the parent |
//! | `comments` | dated notes |
//! | `created_at`, `created_by` | the same |
//!
//! Each issue becomes one Warrant with one item, its title, so a closed
//! issue reads done; an open epic with no blocking dependency has none, and
//! is worked through the Warrants part of it. Derived fields (the counts,
//! `updated_at`, `parent`) are dropped, since an export computes them again;
//! fields that say who holds an issue now (`assignee`, `owner`, leases) are
//! dropped and the import says which. Anything else that carries a
//! value is refused by name: a custom status, an issue type or label the
//! ticket profile does not admit, a dependency of another type, a field
//! this mapping does not know.

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use serde::Serialize;

use super::Fault;
use super::write::{Import, Incoming, IncomingItem, Note, Ref};
use crate::diagnostic::Diagnostic;
use crate::repo::{RepoError, Repository};
use crate::ticket::{Store, Ticket};

/// `beads:` in `imported_from`.
pub const PREFIX: &str = "beads:";

/// Read and mapped; refused fields are not set.
const MAPPED: &[&str] = &[
    "_type",
    "id",
    "title",
    "description",
    "design",
    "acceptance_criteria",
    "notes",
    "status",
    "priority",
    "issue_type",
    "labels",
    "dependencies",
    "comments",
    "created_at",
    "created_by",
    "closed_at",
    "close_reason",
];

/// Derived from the rest, recomputed by `war export beads`: dropped
/// silently, since nothing is lost.
const DERIVED: &[&str] = &[
    "updated_at",
    "dependency_count",
    "dependent_count",
    "comment_count",
    "comments_omitted",
    "parent",
    "is_blocked",
];

/// About who holds an issue now, or Beads' own storage: dropped, and said,
/// since a claim says who holds a Warrant, locally.
const DROPPED: &[&str] = &[
    "started_at",
    "closed_by_session",
    "lease_expires_at",
    "heartbeat_at",
    "lease_granted_node",
    "compaction_level",
    "compacted_at",
    "compacted_at_commit",
    "original_size",
    "wisp_plane",
    "assignee",
    "owner",
];

/// Beads' built-in statuses (`types.AllStatuses`).
const STATUSES: &[&str] = &[
    "open",
    "in_progress",
    "blocked",
    "deferred",
    "closed",
    "pinned",
    "hooked",
];

const RULE_MALFORMED: &str = "beads.malformed";

fn empty(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Null => true,
        serde_json::Value::Bool(b) => !b,
        serde_json::Value::Number(n) => n.as_f64() == Some(0.0),
        serde_json::Value::String(s) => s.is_empty(),
        serde_json::Value::Array(a) => a.is_empty(),
        serde_json::Value::Object(o) => o.is_empty(),
    }
}

fn text(o: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    o.get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .filter(|s| !s.trim().is_empty())
}

/// `2026-01-02T03:04:05Z` as a note's stamp, `2026-01-02 03:04:05`.
fn note_stamp(ts: &str) -> String {
    ts.replacen('T', " ", 1).trim_end_matches('Z').to_owned()
}

/// Read `path` and map every issue in it, or name every fault.
pub fn read(repo: &Repository, store: &Store, path: &Utf8Path) -> Result<Import, Vec<Fault>> {
    let file = super::shown(repo, path);
    let bytes = crate::vfs::read(path).map_err(|e| {
        vec![Fault::new(
            "import.unreadable",
            &file,
            0,
            format!("could not read it: {e}"),
        )]
    })?;
    let body = String::from_utf8(bytes).map_err(|e| {
        vec![Fault::new(
            RULE_MALFORMED,
            &file,
            0,
            format!("not UTF-8 ({e})"),
        )]
    })?;
    let mut faults = Vec::new();
    let mut incoming = Vec::new();
    let mut dropped: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut loose: Vec<String> = Vec::new();
    for (n, line) in body.lines().enumerate() {
        let n = n + 1;
        if line.trim().is_empty() {
            continue;
        }
        let fault = |rule: &'static str, message: String| Fault::new(rule, &file, n, message);
        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                faults.push(fault(RULE_MALFORMED, format!("not a JSON object: {e}")));
                continue;
            }
        };
        let Some(o) = value.as_object() else {
            faults.push(fault(RULE_MALFORMED, "not a JSON object".to_owned()));
            continue;
        };
        match o.get("_type").and_then(serde_json::Value::as_str) {
            None | Some("issue") => {}
            Some(other) => {
                faults.push(fault(
                    "beads.not-an-issue",
                    format!(
                        "a `{other}` line, not an issue; export issues only (`bd export` \
                         without --include-memories)"
                    ),
                ));
                continue;
            }
        }
        let Some(id) = text(o, "id") else {
            faults.push(fault(RULE_MALFORMED, "an issue with no id".to_owned()));
            continue;
        };
        let source = format!("{PREFIX}{id}");
        if !openwarrant_core::ticket::is_import_source(&source) {
            faults.push(fault(
                RULE_MALFORMED,
                format!("issue id {id:?} holds whitespace or a control character"),
            ));
            continue;
        }
        for (key, v) in o {
            if MAPPED.contains(&key.as_str()) || DERIVED.contains(&key.as_str()) {
                continue;
            }
            if let Some(d) = DROPPED.iter().find(|d| **d == key.as_str()) {
                if !empty(v) {
                    *dropped.entry(d).or_default() += 1;
                }
                continue;
            }
            if !empty(v) {
                faults.push(fault(
                    "beads.field-unmapped",
                    format!(
                        "{id}: `{key}` carries a value and has no place in a Warrant; clear it, \
                         or keep this issue in Beads"
                    ),
                ));
            }
        }
        let Some(title) = text(o, "title") else {
            faults.push(fault(RULE_MALFORMED, format!("{id}: no title")));
            continue;
        };
        let status = o
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("open");
        if !STATUSES.contains(&status) {
            faults.push(fault(
                "beads.status-unmapped",
                format!(
                    "{id}: status {status:?} is a custom status; the built-in ones are {}",
                    STATUSES.join(", ")
                ),
            ));
            continue;
        }
        let closed = status == "closed";
        if !closed && status != "open" {
            loose.push(format!("{id} ({status})"));
        }
        let priority = match o.get("priority") {
            None => openwarrant_core::ticket::DEFAULT_PRIORITY,
            Some(p) => match p.as_u64().filter(|p| *p <= 4) {
                Some(p) => p as u8,
                None => {
                    faults.push(fault(
                        "beads.priority-unmapped",
                        format!("{id}: priority {p} is outside 0 ..= 4"),
                    ));
                    continue;
                }
            },
        };
        let kind = text(o, "issue_type");
        if let Some(k) = &kind
            && let Some(why) = store.definition.fields.refuse_type(k)
        {
            faults.push(fault(
                "beads.type-unmapped",
                format!("{id}: issue_type {k:?}: {why}; declare it in profiles/ticket.toml"),
            ));
        }
        let mut labels = BTreeSet::new();
        for l in o
            .get("labels")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(l) = l.as_str() else {
                faults.push(fault(
                    RULE_MALFORMED,
                    format!("{id}: a label that is not text"),
                ));
                continue;
            };
            if !openwarrant_core::ticket::is_field_word(l) {
                faults.push(fault(
                    "beads.label-unmapped",
                    format!(
                        "{id}: label {l:?} is not a word a Warrant's label can be (lowercase, \
                         [a-z0-9_-], 1 to 40)"
                    ),
                ));
            } else if let Some(why) = store.definition.fields.refuse_label(l) {
                faults.push(fault("beads.label-unmapped", format!("{id}: {why}")));
            } else {
                labels.insert(l.to_owned());
            }
        }
        let mut part_of = None;
        let mut after = Vec::new();
        for d in o
            .get("dependencies")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let from = d.get("issue_id").and_then(serde_json::Value::as_str);
            let to = d.get("depends_on_id").and_then(serde_json::Value::as_str);
            let kind = d
                .get("type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let Some(to) = to.filter(|t| !t.is_empty()) else {
                faults.push(fault(
                    RULE_MALFORMED,
                    format!("{id}: a dependency names no depends_on_id"),
                ));
                continue;
            };
            if from.is_some_and(|f| f != id) {
                faults.push(fault(
                    RULE_MALFORMED,
                    format!(
                        "{id}: a dependency of another issue ({})",
                        from.unwrap_or("")
                    ),
                ));
                continue;
            }
            match kind {
                "blocks" => after.push(Ref::Warrant(format!("{PREFIX}{to}"))),
                "parent-child" if part_of.is_none() => part_of = Some(format!("{PREFIX}{to}")),
                "parent-child" => faults.push(fault(
                    "beads.dependency-unmapped",
                    format!("{id}: a second parent ({to}); a Warrant is part of one Warrant"),
                )),
                other => faults.push(fault(
                    "beads.dependency-unmapped",
                    format!(
                        "{id}: a `{other}` dependency on {to}; only `blocks` (an item waits) and \
                         `parent-child` (part_of) map to a Warrant"
                    ),
                )),
            }
        }
        let Some(created_at) = o
            .get("created_at")
            .and_then(serde_json::Value::as_str)
            .and_then(super::timestamp)
        else {
            faults.push(fault(
                RULE_MALFORMED,
                format!("{id}: created_at is missing or not RFC 3339"),
            ));
            continue;
        };
        let mut notes = Vec::new();
        for c in o
            .get("comments")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let at = c
                .get("created_at")
                .and_then(serde_json::Value::as_str)
                .and_then(super::timestamp);
            let body = c.get("text").and_then(serde_json::Value::as_str);
            match (at, body) {
                (Some(at), Some(body)) if !body.trim().is_empty() => notes.push(Note {
                    at: note_stamp(&at),
                    who: c
                        .get("author")
                        .and_then(serde_json::Value::as_str)
                        .filter(|a| !a.trim().is_empty())
                        .unwrap_or("unknown")
                        .to_owned(),
                    text: body.to_owned(),
                }),
                _ => faults.push(fault(
                    RULE_MALFORMED,
                    format!("{id}: a comment without created_at or text"),
                )),
            }
        }
        let mut body = text(o, "description").unwrap_or_default();
        for (key, heading) in [
            ("design", "## Design"),
            ("acceptance_criteria", "## Acceptance criteria"),
            ("notes", "## Notes from Beads"),
        ] {
            if let Some(t) = text(o, key) {
                if !body.is_empty() {
                    body.push_str("\n\n");
                }
                body.push_str(heading);
                body.push_str("\n\n");
                body.push_str(t.trim());
            }
        }
        let epic_open = kind.as_deref() == Some("epic") && !closed;
        let items = if epic_open && after.is_empty() {
            Vec::new()
        } else {
            vec![IncomingItem {
                key: "issue".to_owned(),
                text: title.clone(),
                done: closed,
                done_by: closed.then(|| "unknown".to_owned()),
                done_on: if closed {
                    o.get("closed_at")
                        .and_then(serde_json::Value::as_str)
                        .and_then(super::timestamp)
                } else {
                    None
                },
                note: text(o, "close_reason").filter(|_| closed),
                after,
            }]
        };
        incoming.push(Incoming {
            source,
            title,
            body,
            notes,
            priority,
            kind,
            labels: labels.into_iter().collect(),
            part_of,
            created_at,
            created_by: text(o, "created_by").unwrap_or_default(),
            items,
        });
    }
    if !faults.is_empty() {
        return Err(faults);
    }
    let mut warnings = Vec::new();
    if !dropped.is_empty() {
        warnings.push(Diagnostic::warn(
            "beads.field-dropped",
            file.clone(),
            format!(
                "dropped, as who holds an issue now (a claim says that here, locally): {}",
                dropped
                    .iter()
                    .map(|(k, n)| format!("{k} on {n} issue(s)"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }
    if !loose.is_empty() {
        warnings.push(Diagnostic::warn(
            "beads.status-open",
            file.clone(),
            format!(
                "read as open, since a Warrant's state is its ticked items and its claims: {}",
                loose.join(", ")
            ),
        ));
    }
    Ok(Import {
        format: "beads",
        origin: file,
        incoming,
        records: Vec::new(),
        warnings,
    })
}

// ---- export ----------------------------------------------------------------

/// One line of `war export beads`, in Beads' field order.
#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    #[serde(rename = "_type")]
    pub record_type: &'static str,
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub design: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub acceptance_criteria: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub notes: String,
    pub status: &'static str,
    pub priority: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_type: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub created_by: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_reason: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<Comment>,
    pub dependency_count: usize,
    pub dependent_count: usize,
    pub comment_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Dependency {
    pub issue_id: String,
    pub depends_on_id: String,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Comment {
    pub id: String,
    pub issue_id: String,
    pub author: String,
    pub text: String,
    pub created_at: String,
}

/// The description split back into Beads' four text fields.
fn split_body(body: &str) -> [String; 4] {
    let mut parts: [Vec<&str>; 4] = Default::default();
    let mut at = 0;
    for line in body.lines() {
        match line.trim_end() {
            "## Design" => at = 1,
            "## Acceptance criteria" => at = 2,
            "## Notes from Beads" => at = 3,
            _ => parts[at].push(line),
        }
    }
    parts.map(|p| p.join("\n").trim().to_owned())
}

/// `**2026-01-02 03:04:05 UTC, alice:** text`, as `war note` writes it
/// (minutes) or an import does (seconds): (timestamp, author, text).
fn parse_note(note: &str) -> Option<(String, String, String)> {
    let rest = note.strip_prefix("**")?;
    let (head, text) = rest.split_once(":** ")?;
    let (stamp, who) = head.split_once(" UTC, ")?;
    let who = who.split(", on ").next().unwrap_or(who);
    let at = match stamp.len() {
        16 => format!("{}:00Z", stamp.replacen(' ', "T", 1)),
        19 => format!("{}Z", stamp.replacen(' ', "T", 1)),
        _ => return None,
    };
    Some((at, who.to_owned(), text.replace("\n  ", "\n")))
}

/// `war export beads`: every Warrant in the light encoding as Beads issue
/// JSONL, sorted by id. A Warrant imported from Beads gets its original id
/// back; one whose items are not just its title is an issue with one child
/// issue (`<id>.<n>`, `parent-child`) per item.
pub fn export(store: &Store) -> Result<(String, Vec<Diagnostic>), RepoError> {
    let (tickets, faults) = store.load_all()?;
    let warnings: Vec<Diagnostic> = faults
        .into_iter()
        .map(|d| {
            Diagnostic::warn(
                d.rule.clone(),
                d.file.clone().unwrap_or_default(),
                format!("not exported: {}", d.message),
            )
        })
        .collect();
    let export_id = |t: &Ticket| -> String {
        t.manifest
            .imported_from
            .as_deref()
            .and_then(|s| s.strip_prefix(PREFIX))
            .map_or_else(|| t.id().to_owned(), str::to_owned)
    };
    let ids: BTreeMap<&str, String> = tickets.iter().map(|t| (t.id(), export_id(t))).collect();
    // A Warrant is one issue when it has no items, or one item that is its
    // title; otherwise each item is a child issue.
    let single = |t: &Ticket| -> bool {
        t.checklist.items.is_empty()
            || (t.checklist.items.len() == 1 && t.checklist.items[0].text == t.manifest.title)
    };
    let shape: BTreeMap<&str, bool> = tickets.iter().map(|t| (t.id(), single(t))).collect();
    let item_issue = |ticket: &str, item: &str| -> Option<String> {
        let t = tickets.iter().find(|t| t.id() == ticket)?;
        let base = ids.get(ticket)?.clone();
        if shape.get(ticket).copied().unwrap_or(true) {
            return Some(base);
        }
        let n = t
            .checklist
            .items
            .iter()
            .position(|i| i.id.as_deref() == Some(item))?;
        Some(format!("{base}.{}", n + 1))
    };
    let mut issues: Vec<Issue> = Vec::new();
    for t in &tickets {
        let id = ids[t.id()].clone();
        let created_at = t.manifest.created_at.clone();
        let [description, design, acceptance, notes] =
            split_body(&crate::ticket::description(&t.intent));
        let closed_on = |it: &openwarrant_core::ticket::Item| -> Option<String> {
            it.done_on
                .as_deref()
                .and_then(super::timestamp)
                .or_else(|| it.done.then(|| created_at.clone()))
        };
        let blocks = |issue: &str, it: &openwarrant_core::ticket::Item| -> Vec<Dependency> {
            it.after
                .iter()
                .filter_map(|b| match b {
                    openwarrant_core::ticket::Blocker::Item { item } => item_issue(t.id(), item),
                    openwarrant_core::ticket::Blocker::Ticket { ticket } => {
                        ids.get(ticket.as_str()).cloned()
                    }
                    openwarrant_core::ticket::Blocker::ItemOf { ticket, item } => {
                        item_issue(ticket, item)
                    }
                })
                .map(|to| Dependency {
                    issue_id: issue.to_owned(),
                    depends_on_id: to,
                    kind: "blocks",
                    created_at: created_at.clone(),
                })
                .collect()
        };
        let mut dependencies = Vec::new();
        if let Some(parent) = t.manifest.part_of.as_deref().and_then(|p| ids.get(p)) {
            dependencies.push(Dependency {
                issue_id: id.clone(),
                depends_on_id: parent.clone(),
                kind: "parent-child",
                created_at: created_at.clone(),
            });
        }
        let one = shape[t.id()];
        let first = t.checklist.items.first().filter(|_| one);
        if let Some(it) = first {
            dependencies.extend(blocks(&id, it));
        }
        let comments: Vec<Comment> = crate::ticket::notes(&t.intent)
            .iter()
            .filter_map(|n| parse_note(n))
            .enumerate()
            .map(|(n, (at, who, text))| Comment {
                id: format!("{id}-c{}", n + 1),
                issue_id: id.clone(),
                author: who,
                text,
                created_at: at,
            })
            .collect();
        let done = if one {
            first.is_some_and(|i| i.done)
        } else {
            t.checklist.is_done()
        };
        let closed_at = if one {
            first.filter(|i| i.done).and_then(closed_on)
        } else if done {
            t.checklist.items.iter().filter_map(closed_on).max()
        } else {
            None
        };
        issues.push(Issue {
            record_type: "issue",
            id: id.clone(),
            title: t.manifest.title.clone(),
            description,
            design,
            acceptance_criteria: acceptance,
            notes,
            status: if done { "closed" } else { "open" },
            priority: t.manifest.priority,
            issue_type: t.manifest.kind.clone(),
            created_at: created_at.clone(),
            created_by: t.manifest.created_by.clone(),
            updated_at: closed_at.clone().unwrap_or_else(|| created_at.clone()),
            close_reason: first.filter(|i| i.done).and_then(|i| i.note.clone()),
            closed_at,
            labels: t.manifest.labels.clone(),
            comment_count: comments.len(),
            dependency_count: dependencies.len(),
            dependencies,
            comments,
            dependent_count: 0,
        });
        if !one {
            for (n, it) in t.checklist.items.iter().enumerate() {
                let child = format!("{id}.{}", n + 1);
                let mut deps = vec![Dependency {
                    issue_id: child.clone(),
                    depends_on_id: id.clone(),
                    kind: "parent-child",
                    created_at: created_at.clone(),
                }];
                deps.extend(blocks(&child, it));
                let closed_at = it.done.then(|| closed_on(it)).flatten();
                issues.push(Issue {
                    record_type: "issue",
                    id: child,
                    title: it.text.clone(),
                    description: String::new(),
                    design: String::new(),
                    acceptance_criteria: String::new(),
                    notes: String::new(),
                    status: if it.done { "closed" } else { "open" },
                    priority: t.manifest.priority,
                    issue_type: Some("task".to_owned()),
                    created_at: created_at.clone(),
                    created_by: t.manifest.created_by.clone(),
                    updated_at: closed_at.clone().unwrap_or_else(|| created_at.clone()),
                    close_reason: it.note.clone().filter(|_| it.done),
                    closed_at,
                    labels: Vec::new(),
                    dependency_count: deps.len(),
                    dependencies: deps,
                    comments: Vec::new(),
                    comment_count: 0,
                    dependent_count: 0,
                });
            }
        }
    }
    let mut dependents: BTreeMap<String, usize> = BTreeMap::new();
    for i in &issues {
        for d in &i.dependencies {
            *dependents.entry(d.depends_on_id.clone()).or_default() += 1;
        }
    }
    for i in &mut issues {
        i.dependent_count = dependents.get(&i.id).copied().unwrap_or(0);
    }
    issues.sort_by(|a, b| a.id.cmp(&b.id));
    let mut out = String::new();
    for i in &issues {
        out.push_str(
            &serde_json::to_string(i)
                .map_err(|e| RepoError::Message(format!("could not render {}: {e}", i.id)))?,
        );
        out.push('\n');
    }
    Ok((out, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_parse_back_from_either_stamp() {
        assert_eq!(
            parse_note("**2026-01-02 03:04 UTC, alice:** hello\n  more"),
            Some((
                "2026-01-02T03:04:00Z".to_owned(),
                "alice".to_owned(),
                "hello\nmore".to_owned()
            ))
        );
        assert_eq!(
            parse_note("**2026-01-02 03:04:05 UTC, bob, on i-1a2b:** x"),
            Some((
                "2026-01-02T03:04:05Z".to_owned(),
                "bob".to_owned(),
                "x".to_owned()
            ))
        );
        assert_eq!(parse_note("not a note"), None);
    }

    #[test]
    fn a_body_splits_back_into_beads_fields() {
        let [d, de, a, n] = split_body(
            "What it is.\n\n## Design\n\nThe design.\n\n## Acceptance criteria\n\nIt works.\n\n\
             ## Notes from Beads\n\nA note.",
        );
        assert_eq!(
            (d.as_str(), de.as_str(), a.as_str(), n.as_str()),
            ("What it is.", "The design.", "It works.", "A note.")
        );
    }
}
