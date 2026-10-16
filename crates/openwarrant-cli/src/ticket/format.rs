// SPDX-License-Identifier: Apache-2.0
//! Format 1.0 tickets in the ticket loop (FORMAT-1.0 §4.1, §11).
//!
//! A Format 1.0 ticket is one file, `<tickets>/<alias>.md` or
//! `<tickets>/<alias>/ticket.md`; its journal lies in its evidence directory,
//! `<tickets>/<alias>/` (§4.3), appended in `v: 2` (§11.2). It is read here
//! as the same [`Ticket`] a directory ticket is: its frontmatter stands for
//! the manifest, the document for both atoms, and its items (§3.8: list
//! lines in a unit's body, outside a fence and a continuation) for the
//! checklist. So `war claim`, `war done`, `war admin release`, `war admin
//! reopen`, `war add`, `war note`, `war next` and `war view ready` work it
//! exactly as they work a directory ticket, through the same claims and the
//! same compare-and-set.
//!
//! What it never is: a manifest written back. Nothing here writes a
//! `manifest.toml` or an atom beside the document; a command that would
//! (`war edit`, `war plan promote`, `war add --test`) is refused for a
//! Format 1.0 ticket by name (`ticket.format-document`), and the frontmatter
//! is edited as the text it is.

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_core::format::{self, FieldValue};
use openwarrant_core::ticket::{self, Checklist, DEFAULT_PRIORITY, TicketManifest};

use super::{Store, Ticket, revision_of};
use crate::diagnostic::Diagnostic;

/// The file name a directory-form Format 1.0 ticket keeps its document in.
pub const INNER: &str = "ticket.md";

/// What the frontmatter's `format:` said, standing in for a manifest's
/// schema (the Ticket is never written back as a manifest).
const SCHEMA: &str = "openwarrant/1.0";

/// The refusal for a command that writes something a Format 1.0 ticket does
/// not have (a manifest, an atom beside the document).
pub const UNSUPPORTED: &str = "ticket.format-document";

/// The alias of a single-file Format 1.0 ticket named `name`
/// (`t-3f2a.md`), when it is one.
#[must_use]
pub fn single_file(name: &str) -> Option<&str> {
    name.strip_suffix(".md")
        .filter(|stem| ticket::is_ticket_id(stem))
}

/// Whether `place` (`<path>` or `<path>:<line>`) is a Format 1.0 ticket's
/// document: `.../ticket.md` or `.../<alias>.md`.
#[must_use]
pub fn is_document_path(place: &str) -> bool {
    let path = place.rsplit_once(':').map_or(place, |(p, line)| {
        if line.chars().all(|c| c.is_ascii_digit()) {
            p
        } else {
            place
        }
    });
    let name = path.rsplit('/').next().unwrap_or(path);
    name == INNER || single_file(name).is_some()
}

/// The document of a Format 1.0 ticket whose evidence directory is `dir`:
/// `dir/ticket.md`, else `<dir>.md` beside it.
#[must_use]
pub fn document_of(dir: &Utf8Path) -> Option<Utf8PathBuf> {
    let inner = dir.join(INNER);
    if crate::vfs::is_file(&inner) {
        return Some(inner);
    }
    let name = dir.file_name()?;
    let single = dir.parent()?.join(format!("{name}.md"));
    crate::vfs::is_file(&single).then_some(single)
}

fn one(fields: &std::collections::BTreeMap<String, FieldValue>, key: &str) -> Option<String> {
    fields
        .get(key)
        .and_then(FieldValue::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn many(fields: &std::collections::BTreeMap<String, FieldValue>, key: &str) -> Vec<String> {
    match fields.get(key) {
        Some(FieldValue::One(s)) => s
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
        Some(FieldValue::Many(v)) => v.clone(),
        None => Vec::new(),
    }
}

/// The 1-based lines of `doc`'s items (§3.8): only these are checklist
/// items; a box line in the preamble, a fence or a continuation is text.
fn item_lines(doc: &format::Document) -> std::collections::BTreeSet<usize> {
    doc.units
        .iter()
        .flat_map(|u| u.items.iter().map(|i| i.line))
        .collect()
}

/// Whether the journal in `dir` says the document `alias` itself is done
/// (FORMAT-1.0 §11.1): of the `work.item_done` and `work.item_reopened`
/// events naming it, the later one (§11.2: later `occurred_at`, then the
/// greater `id`) is a done. A journal that does not read says nothing.
#[must_use]
pub fn done_whole(dir: &Utf8Path, alias: &str) -> bool {
    let Ok(journal) = crate::journal_cmd::load(dir) else {
        return false;
    };
    let mut last: Option<&openwarrant_core::journal::JournalEvent> = None;
    for e in journal.events.iter().filter(|e| {
        matches!(
            e.event_type.as_str(),
            super::event::ITEM_DONE | super::event::ITEM_REOPENED
        ) && serde_json::from_str::<serde_json::Value>(&e.payload)
            .ok()
            .and_then(|p| p.get("target").and_then(|t| t.as_str()).map(str::to_owned))
            .as_deref()
            == Some(alias)
    }) {
        last = Some(last.map_or(e, |l| crate::journal_cmd::later(l, e)));
    }
    last.is_some_and(|e| e.event_type == super::event::ITEM_DONE)
}

/// The document's items as a checklist: the box lines §3.8 reads as items,
/// parsed as a checklist line is. Faults are the format reader's to report
/// (`war check`), so none is kept here.
#[must_use]
pub fn checklist_of(text: &str) -> Checklist {
    let doc = format::parse(text.as_bytes());
    checklist_in(text, &doc)
}

fn checklist_in(text: &str, doc: &format::Document) -> Checklist {
    let lines = item_lines(doc);
    let mut c = ticket::parse(text);
    c.items.retain(|i| lines.contains(&(i.line + 1)));
    c.faults.clear();
    c
}

impl Store {
    /// Read the Format 1.0 ticket in `file`, whose alias is `alias`. `Err`
    /// names what is wrong with it: a document the format reader refuses,
    /// or reads `UNKNOWN` as a whole, is never worked.
    pub(crate) fn load_format(&self, file: &Utf8Path, alias: &str) -> Result<Ticket, Diagnostic> {
        let rel = self.rel(file);
        let bytes = crate::vfs::read(file).map_err(|e| {
            Diagnostic::error(
                "ticket.atom-missing",
                rel.clone(),
                format!("could not read it: {e}"),
            )
        })?;
        let doc = format::parse(&bytes);
        if let Some(f) = doc.findings.iter().find(|f| {
            f.outcome == format::Outcome::Refusal
                || (f.outcome == format::Outcome::Unknown && doc.unknown())
        }) {
            return Err(Diagnostic::new(
                if f.outcome == format::Outcome::Refusal {
                    crate::diagnostic::Severity::Error
                } else {
                    crate::diagnostic::Severity::Unknown
                },
                f.rule,
                Some(format!("{rel}:{}", f.line)),
                format!("not worked: {}", f.message),
            ));
        }
        let text = String::from_utf8(bytes.clone()).map_err(|e| {
            Diagnostic::error("format.encoding", rel.clone(), format!("not UTF-8: {e}"))
        })?;
        if doc.alias.as_deref() != Some(alias) {
            return Err(Diagnostic::error(
                "ticket.id-mismatch",
                rel,
                format!(
                    "the file is {alias}'s and its frontmatter says alias {}; a Format 1.0 \
                     ticket's file is named by its alias",
                    doc.alias.as_deref().unwrap_or("nothing")
                ),
            ));
        }
        let fields = &doc.fields;
        let part_of = doc
            .relations
            .iter()
            .find(|r| r.kind == "part_of")
            .map(|r| r.target.id.clone());
        let manifest = TicketManifest {
            schema: SCHEMA.to_owned(),
            id: alias.to_owned(),
            uuid: one(fields, "uuid").unwrap_or_default(),
            title: doc.title.clone().unwrap_or_else(|| alias.to_owned()),
            summary: one(fields, "summary"),
            profile: doc.profile.clone(),
            priority: one(fields, "priority")
                .and_then(|p| p.parse().ok())
                .unwrap_or(DEFAULT_PRIORITY),
            created_at: one(fields, "created_at").unwrap_or_default(),
            created_by: one(fields, "created_by").unwrap_or_default(),
            promoted_to: one(fields, "promoted_to"),
            kind: one(fields, "type"),
            labels: many(fields, "labels"),
            part_of,
            issue: one(fields, "issue").and_then(|n| n.parse().ok()),
            issue_url: one(fields, "issue_url"),
            imported_from: one(fields, "imported_from"),
            due: one(fields, "due"),
            atoms: Vec::new(),
        };
        let checklist = checklist_in(&text, &doc);
        let checklist_empty = checklist.items.is_empty();
        let depends_on: Vec<ticket::Blocker> = doc
            .relations
            .iter()
            .filter(|r| r.kind == "depends_on")
            .filter_map(|r| ticket::Blocker::parse(&r.target.id))
            .collect();
        // What a directory ticket's intent atom says, for the views and the
        // search that read it: the title, the preamble, and the Notes
        // section. Writes go to the document itself (`intent_path`).
        let mut intent = format!("# {}\n\n{}", manifest.title, doc.body.trim());
        if let Some(u) = doc.units.iter().find(|u| u.id == "S-notes") {
            intent.push_str("\n\n");
            intent.push_str(text.get(u.start..u.end).unwrap_or_default());
        }
        intent.push('\n');
        Ok(Ticket {
            dir: self.dir.join(alias),
            manifest,
            intent_path: file.to_owned(),
            checklist_path: file.to_owned(),
            intent,
            checklist_text: text,
            checklist,
            // §11.3: a frontmatter write's base is the document's own
            // record, its frontmatter and preamble — the revision `war plan
            // model` gives it.
            revision: revision_of(bytes.get(..doc.head_end).unwrap_or_default()),
            blocked: None,
            format: true,
            done_whole: checklist_empty && done_whole(&self.dir.join(alias), alias),
            depends_on,
        })
    }

    /// Every Format 1.0 ticket of the store's directory among `entries`
    /// (its sorted listing), and a diagnostic for each that cannot be
    /// worked. `dirs_to_look_in` names the directories that may hold a
    /// `ticket.md` (those no directory ticket was read from): only those
    /// are stat'ed.
    pub(crate) fn format_tickets(
        &self,
        entries: &[Utf8PathBuf],
        dirs_to_look_in: &dyn Fn(&Utf8Path) -> bool,
    ) -> Vec<Result<Ticket, Diagnostic>> {
        let mut found: Vec<(String, Utf8PathBuf)> = Vec::new();
        for path in entries {
            let Some(name) = path.file_name() else {
                continue;
            };
            if let Some(alias) = single_file(name) {
                found.push((alias.to_owned(), path.clone()));
            } else if ticket::is_ticket_id(name) && dirs_to_look_in(path) {
                let inner = path.join(INNER);
                if crate::vfs::is_file(&inner) {
                    found.push((name.to_owned(), inner));
                }
            }
        }
        found.sort();
        let mut out = Vec::new();
        let mut i = 0;
        while i < found.len() {
            let (alias, file) = &found[i];
            // §4.1: both forms of one alias is refused; neither is worked.
            if found.get(i + 1).is_some_and(|(next, _)| next == alias) {
                out.push(Err(Diagnostic::error(
                    "format.document-duplicate",
                    format!("{}:1", self.rel(file)),
                    format!(
                        "{alias} is both `{alias}.md` and `{alias}/{INNER}`; neither is worked"
                    ),
                )));
                while found.get(i).is_some_and(|(a, _)| a == alias) {
                    i += 1;
                }
                continue;
            }
            out.push(self.load_format(file, alias));
            i += 1;
        }
        out
    }
}

/// The document with every item that has no id given one, and the ids
/// given, in order (§3.8: a writer assigns an id). Only those lines change;
/// a box line that is not an item is left as the text it is.
pub(crate) fn name_unnamed(
    text: &str,
    fresh: &mut dyn FnMut(&std::collections::BTreeSet<String>) -> String,
) -> (String, Vec<String>) {
    let c = checklist_of(text);
    let mut taken = ticket::parse(text).ids();
    let mut out = text.to_owned();
    let mut named = Vec::new();
    for item in c.items.iter().filter(|i| i.id.is_none()) {
        let id = fresh(&taken);
        taken.insert(id.clone());
        out = ticket::replace_line(&out, item.line, &item.named(&id).render());
        named.push(id);
    }
    (out, named)
}

/// A done item's line (no terminator) open again: its box `[ ]`, its done
/// suffix (`— done by …`, or ` -- done by …`) gone, every other byte kept.
/// For a directory ticket's checklist and a Format 1.0 document alike.
#[must_use]
pub fn reopened_line(line: &str) -> String {
    let cut = [ticket::DONE_SEPARATOR, " -- done by "]
        .iter()
        .filter_map(|sep| line.find(sep))
        .min()
        .unwrap_or(line.len());
    let head = &line[..cut];
    match head.find("[x]").or_else(|| head.find("[X]")) {
        Some(at) => format!("{}[ ]{}", &head[..at], &head[at + 3..]),
        None => head.to_owned(),
    }
}

/// The line ending the document uses.
fn newline_of(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

/// `text` with `chunk` (whole lines, each ended) inserted at byte `at`.
fn insert_at(text: &str, at: usize, chunk: &str) -> String {
    let mut out = String::with_capacity(text.len() + chunk.len() + 2);
    out.push_str(&text[..at]);
    if at > 0 && !text[..at].ends_with('\n') {
        out.push_str(newline_of(text));
    }
    out.push_str(chunk);
    out.push_str(&text[at..]);
    out
}

/// Where a unit's own text ends: after its last line that is not blank, so
/// what is inserted there sits with it, not after the blank lines before
/// the next heading.
fn content_end(text: &str, start: usize, end: usize) -> usize {
    let span = &text[start..end];
    let mut at = start;
    let mut offset = start;
    for line in span.split_inclusive('\n') {
        offset += line.len();
        if !line.trim().is_empty() {
            at = offset;
        }
    }
    at
}

/// The document with the item `line` (no terminator) added: after its last
/// item; with none, at the end of its Checklist section (§4.1: the section
/// that holds the items), which is added at the end when it has none.
#[must_use]
pub fn append_item(text: &str, line: &str) -> String {
    let nl = newline_of(text);
    let doc = format::parse(text.as_bytes());
    if let Some(last) = doc
        .units
        .iter()
        .flat_map(|u| &u.items)
        .max_by_key(|i| i.end)
    {
        return insert_at(text, last.end, &format!("{line}{nl}"));
    }
    if let Some(u) = doc.units.iter().find(|u| u.id == "S-checklist") {
        let at = content_end(text, u.start, u.end);
        return insert_at(text, at, &format!("{nl}{line}{nl}"));
    }
    let mut out = text.to_owned();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push_str(nl);
    }
    out.push_str(&format!("{nl}## Checklist{nl}{nl}{line}{nl}"));
    out
}

/// The document with a note `entry` (whole lines, each ended) added at the
/// end of its `## Notes` section, which is added at the end when it has
/// none. The entry is a plain list line, never a box: it is no item.
#[must_use]
pub fn append_note(text: &str, entry: &str, heading: &str) -> String {
    let nl = newline_of(text);
    let doc = format::parse(text.as_bytes());
    if let Some(u) = doc.units.iter().find(|u| u.id == "S-notes") {
        let at = content_end(text, u.start, u.end);
        let heading_end = text[u.start..u.end]
            .find('\n')
            .map_or(u.end, |n| u.start + n + 1);
        // A section holding only its heading gets the blank line after it.
        let lead = if at <= heading_end { nl } else { "" };
        return insert_at(text, at, &format!("{lead}{entry}"));
    }
    let mut out = text.to_owned();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push_str(nl);
    }
    out.push_str(&format!("{nl}{heading}{nl}{nl}{entry}"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "---\nformat: openwarrant/1.0\nprofile: ticket\nalias: t-3f2a\nuuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4e\nsummary: Reset passwords by email.\npriority: 1\ndue: 2026-11-01\n---\n# Password reset\n\n- [ ] not an item: the preamble (i-0001)\n\n## Checklist\n\n- [ ] Reset endpoint (i-9c01)\n- [ ] Email template\n\n```\n- [ ] in a fence (i-ffff)\n```\n\n## Notes\n\nnothing yet\n";

    #[test]
    fn only_the_items_of_units_are_checklist_items() {
        let c = checklist_of(DOC);
        let texts: Vec<&str> = c.items.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(texts, ["Reset endpoint", "Email template"]);
        assert!(c.item("i-0001").is_none(), "a preamble box is text");
        assert!(c.item("i-ffff").is_none(), "a fenced box is text");
    }

    #[test]
    fn naming_touches_only_the_unnamed_item() {
        let mut n = 0;
        let (out, named) = name_unnamed(DOC, &mut |_| {
            n += 1;
            format!("i-abc{n}")
        });
        assert_eq!(named, ["i-abc1"]);
        assert!(out.contains("- [ ] Email template (i-abc1)\n"), "{out}");
        assert!(out.contains("- [ ] not an item: the preamble (i-0001)\n"));
        assert_eq!(out.lines().count(), DOC.lines().count());
    }

    #[test]
    fn an_item_is_appended_after_the_last_item_or_into_the_checklist() {
        let out = append_item(DOC, "- [ ] Docs (i-77be)");
        assert!(
            out.contains("- [ ] Email template\n- [ ] Docs (i-77be)\n\n```"),
            "{out}"
        );
        let empty = "---\nformat: openwarrant/1.0\nprofile: ticket\nalias: t-1\n---\n# T\n\n## Checklist\n\n## Notes\n\nx\n";
        let out = append_item(empty, "- [ ] First (i-1111)");
        assert!(
            out.contains("## Checklist\n\n- [ ] First (i-1111)\n\n## Notes"),
            "{out}"
        );
        let none = "---\nformat: openwarrant/1.0\nprofile: ticket\nalias: t-1\n---\n# T\n";
        let out = append_item(none, "- [ ] First (i-1111)");
        assert!(
            out.ends_with("# T\n\n## Checklist\n\n- [ ] First (i-1111)\n"),
            "{out}"
        );
        assert_eq!(checklist_of(&out).items.len(), 1);
    }

    #[test]
    fn a_reopened_line_keeps_everything_but_the_box_and_the_suffix() {
        assert_eq!(
            reopened_line(
                "  * [x] Reset endpoint (i-9c01, after i-4d11) — done by claude, 2026-09-25 [observed]: POST /reset"
            ),
            "  * [ ] Reset endpoint (i-9c01, after i-4d11)"
        );
        assert_eq!(
            reopened_line("- [X] ascii (i-1) -- done by a, 2026-01-01"),
            "- [ ] ascii (i-1)"
        );
        // An open line is its own reopening.
        assert_eq!(reopened_line("- [ ] open (i-1)"), "- [ ] open (i-1)");
    }

    #[test]
    fn a_note_goes_under_notes_and_is_no_item() {
        let out = append_note(DOC, "- **2026-10-15 10:00 UTC, claude:** hi\n", "## Notes");
        assert!(
            out.ends_with("nothing yet\n- **2026-10-15 10:00 UTC, claude:** hi\n"),
            "{out}"
        );
        assert_eq!(checklist_of(&out).items.len(), 2);
    }
}
