// SPDX-License-Identifier: Apache-2.0
//! The adapter: today's corpus and `oh.war/model/v1` as the compiler's
//! [`Graph`].
//!
//! The model gives every record its id, type, source and revision, every
//! relation and every state. What Format 1.0 section 8 adds to a record
//! (`title`, `line`, `span`, `body`, `document`, `content_digest`) the
//! model does not carry yet; until it does, each is read here from the
//! source the format names, and each such read is marked
//! `ADAPTER(model §8: <member>)`. When the model carries the member, the
//! marked read is replaced by the model's value and nothing else moves.
//!
//! The legacy encodings are lowered as section 12 of the format says: a
//! directory Warrant's intent, basis and work-order atoms are its sections
//! `S-intent`, `S-basis`, `S-work-order`; its milestones are records; a
//! ticket's intent and checklist atoms are `S-intent` and `S-checklist`.
//! Those units are not model records yet (`ADAPTER(model §12)`).
//!
//! Only tracked inputs are read: the model's records, relations and its
//! `phase`, `checklist` and `disposition` states, and the files the records
//! cite. Claim locks, journals' clocks, the ledger and the environment are
//! never consulted here.

use std::collections::{BTreeMap, BTreeSet};

use crate::corpus::Corpus;
use crate::model::Model;

use super::{Edge, Finding, Graph, Unit, cut_title, slug};

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{}", openwarrant_compiler::sha256_hex(bytes))
}

/// The files records cite, read once each.
struct Files<'a> {
    root: &'a camino::Utf8Path,
    cache: BTreeMap<String, Option<String>>,
}

impl Files<'_> {
    fn text(&mut self, rel: &str) -> Option<&str> {
        let root = self.root;
        self.cache
            .entry(rel.to_owned())
            .or_insert_with(|| crate::vfs::read_to_string(root.join(rel)).ok())
            .as_deref()
    }
}

/// `[start, end)` of 0-based lines `from..to` in `text`, in bytes.
fn line_span(text: &str, from: usize, to: usize) -> (usize, usize) {
    let mut start = text.len();
    let mut end = text.len();
    let mut offset = 0;
    for (i, l) in text.split_inclusive('\n').enumerate() {
        if i == from {
            start = offset;
        }
        offset += l.len();
        if i + 1 == to {
            end = offset;
            break;
        }
    }
    (start, end.max(start))
}

/// Lines `from..to`, trailing blank lines left out. `(line 1-based, text)`.
fn lines_text(text: &str, from: usize, mut to: usize) -> Option<(usize, String)> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    to = to.min(lines.len());
    while to > from + 1 && lines[to - 1].trim().is_empty() {
        to -= 1;
    }
    if from >= to {
        return None;
    }
    let (s, e) = line_span(text, from, to);
    Some((from + 1, text[s..e].to_owned()))
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    let v = v
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .or_else(|| v.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')))
        .unwrap_or(v);
    v.replace("\\\"", "\"")
}

/// A YAML list entry `- id: <id>` (quoted or not) and the lines under it.
fn yaml_entry(text: &str, id: &str) -> Option<(usize, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let at = lines.iter().position(|l| {
        let t = l.trim_start();
        t.strip_prefix("- ")
            .and_then(|r| r.trim_start().strip_prefix("id:"))
            .is_some_and(|v| unquote(v) == id)
    })?;
    let indent = lines[at].len() - lines[at].trim_start().len();
    let end = lines[at + 1..]
        .iter()
        .position(|l| !l.trim().is_empty() && l.len() - l.trim_start().len() <= indent)
        .map_or(lines.len(), |p| at + 1 + p);
    lines_text(text, at, end)
}

/// A TOML array-of-tables entry holding `id = "<id>"`, through the line
/// before the next `[[`.
fn toml_entry(text: &str, id: &str) -> Option<(usize, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let at = lines.iter().position(|l| {
        l.trim()
            .strip_prefix("id")
            .and_then(|r| r.trim_start().strip_prefix('='))
            .is_some_and(|v| unquote(v) == id)
    })?;
    let start = lines[..at]
        .iter()
        .rposition(|l| l.trim_start().starts_with("[["))?;
    let end = lines[at + 1..]
        .iter()
        .position(|l| l.trim_start().starts_with("[["))
        .map_or(lines.len(), |p| at + 1 + p);
    lines_text(text, start, end)
}

/// A value `key: v` (YAML) or `key = v` (TOML) inside `text`, first match.
fn key_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let t = l.trim_start().trim_start_matches("- ").trim_start();
        let rest = t.strip_prefix(key)?;
        let rest = rest.trim_start();
        let v = rest.strip_prefix(':').or_else(|| rest.strip_prefix('='))?;
        let v = unquote(v);
        (!v.is_empty()).then_some(v)
    })
}

/// A legacy `### OBL-NNN — statement` block, through the line before the
/// next heading of level 1 to 3.
fn obligation_block(text: &str, oid: &str) -> Option<(usize, String, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let at = lines.iter().position(|l| {
        l.strip_prefix("### ")
            .and_then(|r| r.strip_prefix(oid))
            .is_some_and(|r| r.is_empty() || !r.starts_with(|c: char| c.is_ascii_alphanumeric()))
    })?;
    let title = lines[at]["### ".len() + oid.len()..]
        .trim_start_matches([' ', '—', '-', ':', '·', '–'])
        .trim()
        .to_owned();
    let end = lines[at + 1..]
        .iter()
        .position(|l| l.starts_with("# ") || l.starts_with("## ") || l.starts_with("### "))
        .map_or(lines.len(), |p| at + 1 + p);
    let (line, t) = lines_text(text, at, end)?;
    Some((line, t, title))
}

/// The first `# ` heading of a Markdown text.
fn h1(text: &str) -> Option<String> {
    let mut in_front = text.starts_with("---\n") || text.starts_with("---\r\n");
    for (i, l) in text.lines().enumerate() {
        if in_front {
            if i > 0 && l.trim_end() == "---" {
                in_front = false;
            }
            continue;
        }
        if let Some(t) = l.strip_prefix("# ") {
            return Some(t.trim().to_owned());
        }
    }
    None
}

/// The line of `text` (terminator included) whose digest is `revision`.
fn line_with_digest(text: &str, revision: &str) -> Option<(usize, String)> {
    text.split_inclusive('\n')
        .enumerate()
        .find(|(_, l)| digest(l.as_bytes()) == revision)
        .map(|(i, l)| (i + 1, l.to_owned()))
}

fn checklist_title(line: &str) -> String {
    let t = line.trim();
    let t = t.trim_start_matches(['-', '*']).trim_start();
    let t = t
        .strip_prefix("[ ]")
        .or_else(|| t.strip_prefix("[x]"))
        .or_else(|| t.strip_prefix("[X]"))
        .unwrap_or(t)
        .trim_start();
    let t = t.split(" — done by ").next().unwrap_or(t);
    let t = match t.rfind(" (i-") {
        Some(p) => &t[..p],
        None => t,
    };
    t.trim().to_owned()
}

fn section_title(role: &str) -> String {
    let base = role.rsplit('.').next().unwrap_or(role).replace('_', " ");
    let mut c = base.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The primary state of each record (section 10.2): `phase`, else
/// `checklist`, else `disposition`.
fn primary_states(model: &Model) -> BTreeMap<&str, String> {
    let mut best: BTreeMap<&str, (usize, String)> = BTreeMap::new();
    for s in &model.states {
        let rank = match s.kind.as_str() {
            "phase" => 0,
            "checklist" => 1,
            "disposition" => 2,
            _ => continue,
        };
        let e = best
            .entry(s.record.as_str())
            .or_insert((rank, s.value.clone()));
        if rank < e.0 {
            *e = (rank, s.value.clone());
        }
    }
    best.into_iter().map(|(k, (_, v))| (k, v)).collect()
}

/// The graph of the corpus.
#[must_use]
pub fn graph(corpus: &Corpus, model: &Model) -> Graph {
    let repo = corpus.repo();
    let mut files = Files {
        root: &repo.root,
        cache: BTreeMap::new(),
    };
    let states = primary_states(model);
    let mut g = Graph {
        name: repo.config.project.name.clone(),
        ..Graph::default()
    };
    let mut lowered: Vec<Unit> = Vec::new();
    let mut lowered_edges: Vec<Edge> = Vec::new();

    // ---- What the corpus knows beyond the model: titles, uuids, paths.
    let mut warrants: BTreeMap<String, (String, String, Vec<String>)> = BTreeMap::new();
    if let Ok(entries) = corpus.entries() {
        for e in entries {
            let Some(one) = e.ok() else { continue };
            let (Some(basis), Some(v)) = (&one.basis, &one.validated) else {
                continue;
            };
            let alias = v.alias.to_string();
            // Legacy `paths`: the files the Warrant's deliverables declare.
            // ADAPTER(model §8: fields.paths) — a Format 1.0 document's
            // frontmatter `paths` replaces this.
            let paths: Vec<String> = repo
                .load_deliverables(&one.dir)
                .map(|set| {
                    set.records
                        .iter()
                        .map(|d| d.target_ref.clone())
                        .filter(|p| !p.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            warrants.insert(
                alias.clone(),
                (v.raw.title.clone(), v.uuid.to_string(), paths),
            );
            // ADAPTER(model §12): the atoms as sections, the milestones as
            // records. Not model records yet.
            for atom in &basis.atoms {
                if matches!(atom.role.as_str(), "assurance" | "milestones") {
                    if atom.role == "milestones"
                        && let Ok(graph) = openwarrant_core::milestones::parse(
                            &String::from_utf8_lossy(&atom.bytes),
                        )
                    {
                        let source = repo.relative(&one.dir.join(&atom.source));
                        let text = String::from_utf8_lossy(&atom.bytes).into_owned();
                        for m in &graph.milestones {
                            let id = format!("{alias}/{}", m.id);
                            let (line, span) =
                                yaml_entry(&text, &m.id).map_or((1, None), |(l, t)| (l, Some(t)));
                            lowered.push(Unit {
                                id: id.clone(),
                                kind: "milestone".to_owned(),
                                title: m.title.clone(),
                                document: Some(alias.clone()),
                                source: source.clone(),
                                line,
                                revision: digest(&atom.bytes),
                                text: span,
                                ..Unit::default()
                            });
                            lowered_edges.push(Edge {
                                from: id,
                                kind: "part_of".to_owned(),
                                to: alias.clone(),
                            });
                        }
                    }
                    continue;
                }
                let title = section_title(&atom.role);
                lowered.push(Unit {
                    id: format!("{alias}/S-{}", slug(&title)),
                    kind: "section".to_owned(),
                    title: Some(title),
                    document: Some(alias.clone()),
                    source: repo.relative(&one.dir.join(&atom.source)),
                    line: 1,
                    revision: digest(&atom.bytes),
                    text: Some(String::from_utf8_lossy(&atom.bytes).into_owned()),
                    ..Unit::default()
                });
            }
        }
    }
    let mut tickets: BTreeMap<String, (String, Option<String>, String)> = BTreeMap::new();
    let mut items: BTreeMap<String, (usize, String)> = BTreeMap::new();
    if let Ok((list, _)) = corpus.tickets() {
        for t in list {
            let tid = t.id().to_owned();
            tickets.insert(
                tid.clone(),
                (
                    t.manifest.title.clone(),
                    t.manifest.summary.clone(),
                    t.manifest.uuid.clone(),
                ),
            );
            for item in &t.checklist.items {
                if let Some(iid) = &item.id {
                    items.insert(format!("{tid}/{iid}"), (item.line + 1, item.text.clone()));
                }
            }
            // ADAPTER(model §12): the ticket's atoms as its sections.
            for (path, text, title) in [
                (&t.intent_path, &t.intent, "Intent"),
                (&t.checklist_path, &t.checklist_text, "Checklist"),
            ] {
                lowered.push(Unit {
                    id: format!("{tid}/S-{}", slug(title)),
                    kind: "section".to_owned(),
                    title: Some(title.to_owned()),
                    document: Some(tid.clone()),
                    source: repo.relative(path),
                    line: 1,
                    revision: digest(text.as_bytes()),
                    text: Some(text.clone()),
                    ..Unit::default()
                });
            }
        }
    }
    let adr_titles: BTreeMap<String, String> = corpus
        .adrs()
        .map(|a| {
            a.records
                .iter()
                .map(|r| (r.local_alias.clone(), r.title.clone()))
                .collect()
        })
        .unwrap_or_default();
    let authored = corpus.records();
    let format1 = super::format1::documents(repo);

    // ---- Every model record.
    for r in &model.records {
        let mut u = Unit {
            id: r.id.clone(),
            kind: r.kind.clone(),
            source: r.source.clone(),
            line: 1,
            revision: r.revision.clone(),
            state: states.get(r.id.as_str()).cloned(),
            ..Unit::default()
        };
        let (parent, local) = match r.id.split_once('/') {
            Some((p, l)) if warrants.contains_key(p) || tickets.contains_key(p) => {
                (Some(p.to_owned()), l.to_owned())
            }
            _ => (None, r.id.clone()),
        };
        // ADAPTER(model §8: title, line, span, document) — each type's own
        // source, read the way the legacy encoding lays it out.
        match r.kind.as_str() {
            "warrant" => {
                u.work = true;
                u.text = files.text(&r.source).map(str::to_owned);
                if let Some((title, uuid, paths)) = warrants.get(&r.id) {
                    u.title = Some(title.clone());
                    u.uuid = Some(uuid.clone());
                    u.paths.clone_from(paths);
                }
            }
            "ticket" => {
                u.work = true;
                u.text = files.text(&r.source).map(str::to_owned);
                if let Some((title, summary, uuid)) = tickets.get(&r.id) {
                    u.title = Some(title.clone());
                    u.summary.clone_from(summary);
                    u.uuid = Some(uuid.clone());
                }
            }
            "obligation" => {
                u.document.clone_from(&parent);
                if let Some((line, text, title)) = files
                    .text(&r.source)
                    .and_then(|t| obligation_block(t, &local))
                {
                    u.line = line;
                    u.text = Some(text);
                    u.title = Some(title);
                }
            }
            "stage" | "phase" => {
                if r.kind == "stage" {
                    u.document.clone_from(&parent);
                }
                if let Some((line, text)) =
                    files.text(&r.source).and_then(|t| yaml_entry(t, &local))
                {
                    u.title = key_value(&text, "title");
                    u.line = line;
                    u.text = Some(text);
                }
            }
            "deliverable" => {
                if let Some((line, text)) =
                    files.text(&r.source).and_then(|t| toml_entry(t, &local))
                {
                    u.title = key_value(&text, "title");
                    u.line = line;
                    u.text = Some(text);
                }
            }
            "question" => {
                u.text = files.text(&r.source).map(str::to_owned);
                u.title = u.text.as_deref().and_then(|t| key_value(t, "question"));
            }
            "item" if items.contains_key(&r.id) => {
                let (line, title) = &items[&r.id];
                u.document.clone_from(&parent);
                u.line = *line;
                u.title = Some(title.clone());
                u.text = files
                    .text(&r.source)
                    .and_then(|t| t.split_inclusive('\n').nth(line - 1))
                    .map(str::to_owned);
            }
            "instruction" => {
                if let Some(text) = files.text(&r.source) {
                    let scan = openwarrant_core::instruction::scan(text);
                    if let Some(s) = scan.sections.iter().find(|s| s.revision == r.revision) {
                        u.line = s.line;
                        u.title = Some(s.heading.clone());
                        u.text = Some(text[s.start..s.end].to_owned());
                    }
                }
            }
            "section" | "spec" if !authored.records.iter().any(|a| a.id == r.id) => {
                if let Some(text) = files.text(&r.source) {
                    if r.kind == "spec" {
                        u.title = h1(text);
                        u.text = Some(text.to_owned());
                    } else {
                        let bytes = text.as_bytes();
                        let sections = openwarrant_core::sas_sections::split(bytes);
                        let hit = sections.iter().find_map(|s| {
                            if format!("sha256:{}", s.sha256) == r.revision {
                                return Some((s.title.clone(), s.start, s.end));
                            }
                            s.subsections
                                .iter()
                                .find(|x| format!("sha256:{}", x.sha256) == r.revision)
                                .map(|x| (x.title.clone(), x.start, x.end))
                        });
                        if let Some((title, start, end)) = hit
                            && let Some(span) = text.get(start..end)
                        {
                            u.line = text[..start].matches('\n').count() + 1;
                            u.title = Some(title);
                            u.text = Some(span.to_owned());
                        }
                    }
                }
            }
            "requirement" if !authored.records.iter().any(|a| a.id == r.id) => {
                // A §106 table row: `| <id> | <title> |`.
                if let Some(text) = files.text(&r.source) {
                    let prefix = format!("| {} |", r.id);
                    if let Some((i, l)) = text
                        .split_inclusive('\n')
                        .enumerate()
                        .find(|(_, l)| l.starts_with(&prefix))
                    {
                        u.line = i + 1;
                        u.title = l
                            .split('|')
                            .nth(2)
                            .map(|t| t.trim().to_owned())
                            .filter(|t| !t.is_empty());
                        u.text = Some(l.to_owned());
                    }
                }
            }
            "adr" => {
                u.text = files.text(&r.source).map(str::to_owned);
                u.title = adr_titles.get(&r.id).cloned();
            }
            _ => {}
        }
        // Authored record atoms (`docs/records/`): the record's own span.
        if u.text.is_none()
            && let Some(a) = authored.get(&r.id)
            && a.source == r.source
            && let Some(text) = files.text(&r.source)
            && let Ok(atom) = openwarrant_core::record::parse(text)
            && let Some(rec) = atom
                .records
                .iter()
                .find(|x| x.id == r.id && x.line == a.line)
        {
            let span = &text[rec.start..rec.end];
            let relation_lines: BTreeSet<usize> = rec.relations.iter().map(|x| x.line).collect();
            u.line = rec.line;
            u.title = span
                .lines()
                .enumerate()
                .skip(1)
                .find(|(i, l)| !relation_lines.contains(&(rec.line + i)) && !l.trim().is_empty())
                .map(|(_, l)| cut_title(l));
            u.text = Some(span.to_owned());
        }
        // Anything else: the whole file when the revision is the file's, or
        // the one line whose digest it is (an item read in place).
        if u.text.is_none()
            && let Some(text) = files.text(&r.source)
        {
            if digest(text.as_bytes()) == r.revision {
                u.text = Some(text.to_owned());
            } else if let Some((line, l)) = line_with_digest(text, &r.revision) {
                u.line = line;
                if u.kind == "item" {
                    u.title = Some(checklist_title(&l));
                }
                u.text = Some(l);
            }
        }
        if u.title.is_none()
            && let Some(t) = u.text.as_deref()
        {
            u.title = h1(t).or_else(|| key_value(t, "title"));
        }
        // ADAPTER(model §8: fields.summary, fields.paths) — a Format 1.0
        // work document's frontmatter, read where it declares this alias.
        if u.work
            && let Some(doc) = format1
                .iter()
                .find(|d| d.alias.as_deref() == Some(r.id.as_str()))
        {
            if let Some(s) = doc.get("summary") {
                u.summary = Some(s.to_owned());
            }
            if !doc.list("paths").is_empty() {
                u.paths = doc.list("paths").to_vec();
            }
        }
        if u.text.is_none() {
            g.diagnostics.push(Finding {
                rule: "context.span-unknown".to_owned(),
                record: r.id.clone(),
                message: format!(
                    "{}: the span of this {} in {} could not be located; it is given by its summary only",
                    r.id, r.kind, r.source
                ),
            });
        }
        // ADAPTER(model §8: content_digest) — the model will carry it.
        u.content_digest = None;
        g.add(u);
    }
    for u in lowered {
        g.add(u);
    }
    g.edges = model
        .relations
        .iter()
        .map(|r| Edge {
            from: r.from.clone(),
            kind: r.kind.clone(),
            to: r.to.clone(),
        })
        .chain(lowered_edges)
        .collect();
    g.edges.sort();
    g.edges.dedup();
    g.diagnostics.sort();
    g.diagnostics.dedup();
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_yaml_entry_runs_to_its_sibling_and_leaves_trailing_blanks_out() {
        let y = "stages:\n  - id: \"S-1\"\n    title: \"one\"\n\n  - id: S-2\n    title: two\nnext: x\n";
        let (line, t) = yaml_entry(y, "S-1").unwrap();
        assert_eq!(
            (line, t.as_str()),
            (2, "  - id: \"S-1\"\n    title: \"one\"\n")
        );
        let (line, t) = yaml_entry(y, "S-2").unwrap();
        assert_eq!((line, t.as_str()), (5, "  - id: S-2\n    title: two\n"));
        assert_eq!(key_value(&t, "title").as_deref(), Some("two"));
        assert!(yaml_entry(y, "S-3").is_none());
    }

    #[test]
    fn a_toml_entry_keeps_its_subtables_and_stops_at_the_next() {
        let t = "schema = \"x\"\n\n[[d]]\nid = \"D-001\"\ntitle = \"a\"\n\n[d.p]\nk = 1\n\n[[d]]\nid = \"D-002\"\n";
        let (line, s) = toml_entry(t, "D-001").unwrap();
        assert_eq!(line, 3);
        assert_eq!(s, "[[d]]\nid = \"D-001\"\ntitle = \"a\"\n\n[d.p]\nk = 1\n");
        assert!(toml_entry(t, "D-003").is_none());
    }

    #[test]
    fn an_obligation_block_stops_at_the_next_heading_and_not_at_a_longer_id() {
        let a =
            "# A\n\n### OBL-001 — first\n- **scope:** x\n\n### OBL-0010 — tenth\nbody\n## Next\n";
        let (line, t, title) = obligation_block(a, "OBL-001").unwrap();
        assert_eq!((line, title.as_str()), (3, "first"));
        assert_eq!(t, "### OBL-001 — first\n- **scope:** x\n");
        let (_, t, _) = obligation_block(a, "OBL-0010").unwrap();
        assert_eq!(t, "### OBL-0010 — tenth\nbody\n");
        assert!(obligation_block(a, "OBL-002").is_none());
    }

    #[test]
    fn a_checklist_line_reads_as_its_title() {
        assert_eq!(
            checklist_title("- [x] Reset endpoint (i-9c01) — done by claude, 2026-09-25: POST\n"),
            "Reset endpoint"
        );
        assert_eq!(checklist_title("- [ ] Docs (i-77be, after i-4d11)"), "Docs");
        assert_eq!(section_title("work_order"), "Work order");
        assert_eq!(section_title("ticket.checklist"), "Checklist");
    }
}
