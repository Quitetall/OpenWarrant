// SPDX-License-Identifier: Apache-2.0
//! `war apply <package>`: an edited package read back into its documents
//! (OpenWarrant Format 1.0, `docs/format/FORMAT-1.0.md` section 10.5).
//!
//! An editable preset (`war context --preset human`) wraps each record's
//! text in markers naming its exact source:
//!
//! ```text
//! <!-- rec t-0d61d/S-intent docs/tickets/t-0d61d.md:10 sha256:<64 hex> -->
//! ## Intent
//! …
//! <!-- /rec -->
//! ```
//!
//! A person edits the text between markers and gives the package back. For
//! each marked block, in this order:
//!
//! 1. a block whose text equals the source span at the marked revision is
//!    ignored;
//! 2. one whose record's current revision is not the marked one is refused
//!    (`apply.stale`): someone changed it since the package was made;
//! 3. one that does not parse as a unit under section 3, in place of the
//!    span, is refused by the rule of the fault, and one whose id or type
//!    changed (or that holds more than its one unit) is refused
//!    (`apply.identity`);
//! 4. otherwise the span's bytes are replaced with the block's.
//!
//! A changed box is not a text edit: `[ ]` to `[x]` journals `work.item_done`
//! and `[x]` to `[ ]` `work.item_reopened`, by the actor applying the
//! package (a hand-ticked box is done at `claimed`, section 11.4), and the
//! line is rewritten as section 3.8 shows it, dated with the date of
//! `--at` (default now). A box line the edit added without an id is given
//! one. Blocks of one document are applied together or not at all; a
//! refusal in one document does not stop another. A block missing from the
//! package is left alone, and nothing outside markers is ever written back.
//!
//! The write is a compare-and-set (section 11.3): under the lock every
//! writer of the document takes (its directory's), the file must still be
//! the bytes the blocks were judged against, or nothing is written
//! (`work.stale-revision`).

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use openwarrant_core::format::{self, Outcome};
use serde::Serialize;

use crate::diagnostic::{Diagnostic, Report};
use crate::format_doc::Loaded;
use crate::repo::{RepoError, Repository};

/// The refusal of a block whose record moved on since the package was made.
pub const RULE_STALE: &str = "apply.stale";
/// The refusal of a block whose id or type changed.
pub const RULE_IDENTITY: &str = "apply.identity";
/// The refusal of a write whose document moved under it (section 11.3).
pub const RULE_STALE_REVISION: &str = "work.stale-revision";

const OPEN: &str = "<!-- rec ";
const CLOSE: &str = "<!-- /rec -->";

/// One marked block of a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// The record's global id.
    pub id: String,
    /// Repository-relative path of its source.
    pub source: String,
    /// The marked 1-based line.
    pub line: usize,
    /// `sha256:<64 hex>`: the revision the package was made at.
    pub revision: String,
    /// The text between the markers, exactly.
    pub text: String,
}

/// `<!-- rec <id> <path>:<line> sha256:<64 hex> -->`.
fn open_marker(line: &str) -> Option<(String, String, usize, String)> {
    let rest = line.strip_prefix(OPEN)?.strip_suffix(" -->")?;
    let mut parts = rest.split(' ');
    let (id, at, revision) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || id.is_empty() {
        return None;
    }
    let (source, n) = at.rsplit_once(':')?;
    let n: usize = n.parse().ok()?;
    let hex = revision.strip_prefix("sha256:")?;
    if source.is_empty()
        || hex.len() != 64
        || !hex
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return None;
    }
    Some((id.to_owned(), source.to_owned(), n, revision.to_owned()))
}

/// Every marked block of a package, in order. Text outside markers (the
/// index, headers, state lines) is not read; an open marker never closed
/// is no block.
#[must_use]
pub fn blocks(package: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut cur: Option<Block> = None;
    for raw in package.split_inclusive('\n') {
        let line = raw.trim_end_matches('\n');
        match &mut cur {
            None => {
                if let Some((id, source, line, revision)) = open_marker(line) {
                    cur = Some(Block {
                        id,
                        source,
                        line,
                        revision,
                        text: String::new(),
                    });
                }
            }
            Some(b) => {
                if line == CLOSE {
                    out.extend(cur.take());
                } else {
                    b.text.push_str(raw);
                }
            }
        }
    }
    out
}

/// One refusal: its rule and where (`path:line`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Refused {
    pub rule: String,
    pub file: String,
    pub message: String,
}

/// What `war apply` did with one document.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Applied {
    pub source: String,
    /// The records whose text was replaced.
    pub edited: Vec<String>,
    /// Items ticked (`work.item_done`).
    pub done: Vec<String>,
    /// Items opened again (`work.item_reopened`).
    pub reopened: Vec<String>,
    /// Ids given to box lines the edit added without one.
    pub named: Vec<String>,
    /// Blocks left alone: unchanged since the package was made.
    pub unchanged: usize,
}

/// `oh.war/apply/v1`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ApplyOutcome {
    pub schema: &'static str,
    pub applied: Vec<Applied>,
    pub refused: Vec<Refused>,
    /// Blocks whose source is no Format 1.0 document here: not written.
    pub skipped: Vec<String>,
}

/// Who applies, and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub actor: String,
    /// `YYYY-MM-DD`: what a ticked line is dated.
    pub date: String,
}

/// The record a block names, in its document: its span and, for an item,
/// its id.
struct Target {
    start: usize,
    end: usize,
    record_type: String,
    /// The local id: a unit's, or an item's.
    local: String,
    item: bool,
}

fn target_of(d: &Loaded, global: &str) -> Option<Target> {
    let local = match &d.doc.alias {
        Some(a) => global.strip_prefix(&format!("{a}/"))?,
        None => global,
    };
    for u in &d.doc.units {
        if u.id == local {
            return Some(Target {
                start: u.start,
                end: u.end,
                record_type: u.record_type.clone(),
                local: local.to_owned(),
                item: false,
            });
        }
        if let Some(i) = u.items.iter().find(|i| i.id.as_deref() == Some(local)) {
            return Some(Target {
                start: i.start,
                end: i.end,
                record_type: "item".to_owned(),
                local: local.to_owned(),
                item: true,
            });
        }
    }
    None
}

fn refused(rule: &str, source: &str, line: usize, message: String) -> Refused {
    Refused {
        rule: rule.to_owned(),
        file: format!("{source}:{line}"),
        message,
    }
}

/// Step 3: the block read as a unit under section 3, in place of the span.
/// `Err` is the refusal.
fn parse_in_place(
    d: &Loaded,
    t: &Target,
    b: &Block,
    profiles: &openwarrant_core::format_profile::Profiles,
) -> Result<(), Refused> {
    let mut trial = d.bytes[..t.start].to_vec();
    trial.extend_from_slice(b.text.as_bytes());
    trial.extend_from_slice(&d.bytes[t.end..]);
    let doc = format::parse_with(&trial, profiles, &d.source);
    if let Some(f) = doc
        .findings
        .iter()
        .filter(|f| f.outcome == Outcome::Refusal)
        .min_by_key(|f| f.line)
    {
        return Err(refused(
            f.rule,
            &d.source,
            f.line,
            format!("{}: the edited block does not parse: {}", b.id, f.message),
        ));
    }
    let end = t.start + b.text.len();
    let identity = |why: &str| {
        refused(
            RULE_IDENTITY,
            &d.source,
            b.line,
            format!(
                "{}: {why}; a package edits a record's text, never which record it is",
                b.id
            ),
        )
    };
    if t.item {
        let same = doc.units.iter().flat_map(|u| u.items.iter()).find(|i| {
            i.start == t.start && i.end == end && i.id.as_deref() == Some(t.local.as_str())
        });
        return match same {
            Some(_) => Ok(()),
            None => Err(identity("the block is no longer that one item line")),
        };
    }
    let Some(u) = doc.units.iter().find(|u| u.start == t.start) else {
        return Err(identity("the block no longer starts a unit"));
    };
    if u.id != t.local || u.record_type != t.record_type {
        return Err(identity(&format!(
            "the block is now `{}` of type `{}`, not `{}` of type `{}`",
            u.id, u.record_type, t.local, t.record_type
        )));
    }
    // One unit: the next one starts where the block ends.
    if u.end != end {
        return Err(identity("the block holds more than its one unit"));
    }
    Ok(())
}

/// The box state of every identified item of `text` (a Format 1.0 document).
fn boxes(text: &str) -> BTreeMap<String, bool> {
    crate::ticket::format::checklist_of(text)
        .items
        .iter()
        .filter_map(|i| Some((i.id.clone()?, i.done)))
        .collect()
}

/// Byte offset of each line's start.
fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect()
}

/// The events one document's apply journals.
struct Event {
    kind: &'static str,
    item: String,
}

/// One document's blocks, judged and, when every one passes, written.
fn apply_document(
    repo: &Repository,
    d: &Loaded,
    blocks: &[&Block],
    profiles: &openwarrant_core::format_profile::Profiles,
    opts: &Options,
) -> Result<Result<Applied, Refused>, RepoError> {
    let mut applied = Applied {
        source: d.source.clone(),
        ..Applied::default()
    };
    // (start, end, new text) of each changed span, by start.
    let mut edits: Vec<(usize, usize, String, String)> = Vec::new();
    for b in blocks {
        if format::sha256(b.text.as_bytes()) == b.revision {
            applied.unchanged += 1;
            continue;
        }
        let stale = |why: String| refused(RULE_STALE, &d.source, b.line, why);
        let Some(t) = target_of(d, &b.id) else {
            return Ok(Err(stale(format!(
                "{} is no longer a record of {}; the package was made before it changed",
                b.id, d.source
            ))));
        };
        let now = format::sha256(&d.bytes[t.start..t.end]);
        if now != b.revision {
            return Ok(Err(stale(format!(
                "{} changed since the package was made (marked {}, now {}); make the package again",
                b.id,
                &b.revision[..19.min(b.revision.len())],
                &now[..19.min(now.len())]
            ))));
        }
        if let Err(r) = parse_in_place(d, &t, b, profiles) {
            return Ok(Err(r));
        }
        if edits.iter().any(|(s, e, _, _)| t.start < *e && *s < t.end) {
            return Ok(Err(stale(format!(
                "{} is marked twice in the package",
                b.id
            ))));
        }
        edits.push((t.start, t.end, b.text.clone(), b.id.clone()));
    }
    if edits.is_empty() {
        return Ok(Ok(applied));
    }
    edits.sort_by_key(|e| e.0);
    let original = String::from_utf8_lossy(&d.bytes).into_owned();
    // The document with every span replaced, and where each block now lies.
    let mut text = String::with_capacity(original.len());
    let mut ranges = Vec::new();
    let mut at = 0;
    for (s, e, new, id) in &edits {
        text.push_str(&original[at..*s]);
        ranges.push((text.len(), text.len() + new.len()));
        text.push_str(new);
        at = *e;
        applied.edited.push(id.clone());
    }
    text.push_str(&original[at..]);
    // Box lines inside an applied block: a changed box is an event, its line
    // rewritten as 3.8 shows; a new box line without an id gets one.
    let before = boxes(&original);
    let inside = |line: usize, starts: &[usize]| {
        starts
            .get(line)
            .is_some_and(|&o| ranges.iter().any(|(s, e)| *s <= o && o < *e))
    };
    let mut events = Vec::new();
    let starts = line_starts(&text);
    let checklist = crate::ticket::format::checklist_of(&text);
    let mut taken: BTreeSet<String> = checklist
        .items
        .iter()
        .filter_map(|i| i.id.clone())
        .collect();
    let mut lines: Vec<(usize, String)> = Vec::new();
    for item in &checklist.items {
        if !inside(item.line, &starts) {
            continue;
        }
        let Some(id) = &item.id else {
            let id = openwarrant_core::ticket::item_id(
                &openwarrant_core::WarUuid::mint().to_string(),
                &taken,
            );
            taken.insert(id.clone());
            let mut named = item.named(&id);
            if named.done {
                named = named.ticked(&opts.actor, &opts.date, None);
                events.push(Event {
                    kind: crate::ticket::event::ITEM_DONE,
                    item: id.clone(),
                });
            }
            lines.push((item.line, named.render()));
            applied.named.push(id);
            continue;
        };
        let line = text[starts[item.line]..]
            .split('\n')
            .next()
            .unwrap_or_default()
            .trim_end_matches('\r');
        match (before.get(id), item.done) {
            (Some(false), true) => {
                lines.push((
                    item.line,
                    item.ticked(&opts.actor, &opts.date, None).render(),
                ));
                events.push(Event {
                    kind: crate::ticket::event::ITEM_DONE,
                    item: id.clone(),
                });
            }
            (Some(true), false) => {
                lines.push((item.line, crate::ticket::format::reopened_line(line)));
                events.push(Event {
                    kind: crate::ticket::event::ITEM_REOPENED,
                    item: id.clone(),
                });
            }
            _ => {}
        }
    }
    for (n, line) in &lines {
        text = openwarrant_core::ticket::replace_line(&text, *n, line);
    }
    let path = repo.root.join(&d.source);
    // Section 11.3: under the document's lock, the bytes the blocks were
    // judged against, or nothing is written.
    let _held = path.parent().and_then(dir_lock);
    let current = crate::vfs::read(&path).map_err(|source| RepoError::Io {
        context: format!("could not read {path}"),
        source,
    })?;
    if current != d.bytes {
        return Ok(Err(refused(
            RULE_STALE_REVISION,
            &d.source,
            1,
            format!(
                "{} changed while the package was being applied; nothing was written. Apply it again",
                d.source
            ),
        )));
    }
    if let Err(r) = crate::compile::atomic::write_if(
        &path,
        text.as_bytes(),
        &crate::compile::atomic::Prestate::of(&d.bytes),
    ) {
        return Ok(Err(refused(
            RULE_STALE_REVISION,
            &d.source,
            1,
            format!("{}: {}; nothing was written", d.source, r.rule),
        )));
    }
    if let (Some(alias), Some(uuid)) = (
        d.doc.alias.as_deref(),
        d.doc
            .fields
            .get("uuid")
            .and_then(format::FieldValue::as_str),
    ) {
        let dir = evidence_dir(&path, alias);
        for e in events {
            let target = format!("{alias}/{}", e.item);
            let episode = episode(&dir, e.kind, &target)?;
            let payload = if e.kind == crate::ticket::event::ITEM_DONE {
                applied.done.push(target.clone());
                serde_json::json!({"item": e.item, "target": target, "on": opts.date, "episode": episode})
            } else {
                applied.reopened.push(target.clone());
                serde_json::json!({"item": e.item, "target": target, "reason": "reopened in a package (war apply)", "episode": episode})
            };
            crate::journal_cmd::record(&dir, uuid, e.kind, &opts.actor, &payload.to_string())?;
        }
    }
    Ok(Ok(applied))
}

/// The work document's evidence directory (section 4.3): `<dir>/<alias>/`
/// beside a one-file document, its own directory otherwise.
fn evidence_dir(path: &Utf8Path, alias: &str) -> camino::Utf8PathBuf {
    let parent = path.parent().unwrap_or_else(|| Utf8Path::new("."));
    if path.file_name() == Some(&format!("{alias}.md")) {
        parent.join(alias)
    } else {
        parent.to_path_buf()
    }
}

/// Section 11.2: the number of events of `kind` for `target` already in the
/// journal, so a fact that happens again is a new event.
fn episode(dir: &Utf8Path, kind: &str, target: &str) -> Result<usize, RepoError> {
    if !crate::vfs::is_file(dir.join("journal.jsonl")) {
        return Ok(0);
    }
    let journal = crate::journal_cmd::load(dir)?;
    Ok(journal
        .events
        .iter()
        .filter(|e| crate::journal_cmd::work_type(&e.event_type) == kind)
        .filter(|e| {
            serde_json::from_str::<serde_json::Value>(&e.payload)
                .ok()
                .and_then(|p| p.get("target").and_then(|t| t.as_str().map(str::to_owned)))
                .is_some_and(|t| t == target)
        })
        .count())
}

/// One writer at a time in `dir` (as every ticket write takes it): an
/// advisory lock held until the file is dropped.
fn dir_lock(dir: &Utf8Path) -> Option<std::fs::File> {
    let f = std::fs::File::open(dir).ok()?;
    f.lock().ok()?;
    Some(f)
}

/// `war apply <package>`: every document the package marks, each applied
/// whole or refused whole.
pub fn apply(repo: &Repository, package: &str, opts: &Options) -> Result<ApplyOutcome, RepoError> {
    let docs = crate::format_doc::load(repo);
    let mut by_source: BTreeMap<&str, Vec<&Block>> = BTreeMap::new();
    let all = blocks(package);
    for b in &all {
        by_source.entry(b.source.as_str()).or_default().push(b);
    }
    let mut out = ApplyOutcome {
        schema: "oh.war/apply/v1",
        ..ApplyOutcome::default()
    };
    for (source, blocks) in by_source {
        let Some(d) = docs
            .docs
            .iter()
            .find(|d| d.source == source && d.admitted())
        else {
            out.skipped.push(source.to_owned());
            continue;
        };
        match apply_document(repo, d, &blocks, &docs.profiles, opts)? {
            Ok(a) => out.applied.push(a),
            Err(r) => out.refused.push(r),
        }
    }
    Ok(out)
}

/// The report of an apply: each refusal an error by its rule at `path:line`,
/// each skipped source a warning, each applied document a pass.
#[must_use]
pub fn report_of(o: &ApplyOutcome) -> Report {
    let mut report = Report::default();
    for r in &o.refused {
        report.push(Diagnostic::error(
            r.rule.clone(),
            r.file.clone(),
            r.message.clone(),
        ));
    }
    for s in &o.skipped {
        report.push(Diagnostic::warn(
            "apply.skipped",
            s.clone(),
            format!("{s} is no Format 1.0 document here; its blocks were not written back"),
        ));
    }
    for a in &o.applied {
        let mut what = Vec::new();
        if !a.edited.is_empty() {
            what.push(format!("{} edited", a.edited.len()));
        }
        if !a.done.is_empty() {
            what.push(format!("done {}", a.done.join(", ")));
        }
        if !a.reopened.is_empty() {
            what.push(format!("reopened {}", a.reopened.join(", ")));
        }
        if !a.named.is_empty() {
            what.push(format!("named {}", a.named.join(", ")));
        }
        if a.unchanged > 0 {
            what.push(format!("{} unchanged", a.unchanged));
        }
        report.push(Diagnostic::pass(
            "apply.applied",
            format!(
                "{}: {}",
                a.source,
                if what.is_empty() {
                    "nothing to apply".to_owned()
                } else {
                    what.join("; ")
                }
            ),
        ));
    }
    if o.applied.is_empty() && o.refused.is_empty() && o.skipped.is_empty() {
        report.push(Diagnostic::pass(
            "apply.empty",
            "the package marks no block (`war context <target> --preset human` makes one)",
        ));
    }
    report
}

#[cfg(test)]
mod tests;
