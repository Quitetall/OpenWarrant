// SPDX-License-Identifier: Apache-2.0
//! A git merge driver for ticket files (M11): `war admin merge-ticket`.
//!
//! Two agents on two branches tick two adjacent items of one checklist.
//! Each changed one line; git's text merge reads two changes that touch and
//! stops with a conflict. The same happens to two `war plan add`s (both append
//! after the last item) and two `war plan note`s (both append to the intent).
//! The files keep their layout and their bytes: what changes is how git
//! merges them, through this driver, named in `.gitattributes`
//! (`merge=war-ticket`) and configured once per clone by `war init` or
//! `war admin merge-ticket --install`.
//!
//! - A **checklist**, or a Format 1.0 ticket document (`docs/tickets/t-x.md`,
//!   which holds its checklist), merges item by item, keyed by item id: an item changed
//!   on one side takes that side's line; one changed identically on both is
//!   kept once; an item added on either side is kept (theirs after ours); an
//!   item removed on one side and untouched on the other is removed.
//! - An **intent** (or any other ticket atom) whose both sides only
//!   appended keeps the common text, then ours, then theirs; a second
//!   `## Notes` heading is not repeated.
//! - Anything else (one item changed differently on both sides, the header
//!   edited on both, an unnamed item) falls back to git's own text merge,
//!   with its conflict markers, and the driver exits non-zero so git stops
//!   and asks. Nothing is ever merged silently that git would have flagged
//!   as two different edits of one thing.
//!
//! Journals need no driver: they are append-only lines, and
//! `merge=union` keeps both sides' lines.

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use openwarrant_core::ticket::{self, CHECKLIST_ROLE};

/// The driver's name in `.gitattributes` and in git's configuration.
pub const DRIVER: &str = "war-ticket";

/// The lines `war init` puts in `.gitattributes`.
pub const GITATTRIBUTES: &str = "\
# war (M11): journals are append-only lines, so a merge keeps both sides'.
**/journal.jsonl merge=union
# war (M11): ticket checklists merge item by item, notes by appending
# (`war admin merge-ticket`; `war admin merge-ticket --install` configures this clone).
docs/tickets/*/atoms/*.md merge=war-ticket
docs/tickets/*.md merge=war-ticket
docs/tickets/*/ticket.md merge=war-ticket
docs/work/*.md merge=war-ticket
docs/work/*/*.md merge=war-ticket
";

/// How a merge came out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Merged {
    /// Item by item: the merged checklist.
    Items(String),
    /// Both sides only appended: common text, ours, theirs.
    Appends(String),
    /// Not mergeable here: git's text merge decides (why, for the message).
    Text(String),
}

/// Merge a ticket file three ways. `path` is the file's path in the
/// repository (git's `%P`), which says whether it is a checklist.
#[must_use]
pub fn merge(path: &str, base: &str, ours: &str, theirs: &str) -> Merged {
    if ours == theirs {
        return Merged::Appends(ours.to_owned());
    }
    let checklist = super::format::is_document_path(path)
        || Utf8Path::new(path).starts_with(crate::subject::WORK_ROOT)
        || Utf8Path::new(path)
            .file_name()
            .is_some_and(|n| n.contains("checklist") || n.contains(CHECKLIST_ROLE));
    if checklist {
        // R2: one ticket brought into two worktrees (`war start` on a ticket
        // held elsewhere) is added on both branches, so git has no base.
        // What both sides still agree on stands in for it.
        let common;
        let base = if base.is_empty() && same_document(ours, theirs) {
            common = common_base(ours, theirs);
            common.as_str()
        } else {
            base
        };
        return match items(base, ours, theirs) {
            Ok(m) => Merged::Items(m),
            Err(why) => Merged::Text(why),
        };
    }
    match appends(base, ours, theirs) {
        Some(m) => Merged::Appends(m),
        None => Merged::Text("both sides changed more than they appended".to_owned()),
    }
}

/// The item lines of a checklist, by id, and its other lines in order.
struct Lines<'a> {
    by_id: BTreeMap<String, &'a str>,
    order: Vec<(Option<String>, &'a str)>,
}

fn lines(text: &str) -> Result<Lines<'_>, String> {
    let c = ticket::parse(text);
    if let Some(f) = c.faults.first() {
        return Err(format!("line {}: {}", f.line, f.message));
    }
    let raw: Vec<&str> = text.split_inclusive('\n').collect();
    let mut at_line: BTreeMap<usize, String> = BTreeMap::new();
    for item in &c.items {
        let Some(id) = &item.id else {
            return Err(format!(
                "line {} is an item without an id; `war admin claim` names it",
                item.line + 1
            ));
        };
        at_line.insert(item.line, id.clone());
    }
    let mut by_id = BTreeMap::new();
    let mut order = Vec::new();
    for (n, line) in raw.into_iter().enumerate() {
        let id = at_line.get(&n).cloned();
        if let Some(id) = &id
            && by_id.insert(id.clone(), line).is_some()
        {
            return Err(format!("{id} appears twice"));
        }
        order.push((id, line));
    }
    Ok(Lines { by_id, order })
}

/// The text lines (not items) of a checklist, in order.
fn prose<'a>(l: &Lines<'a>) -> Vec<&'a str> {
    l.order
        .iter()
        .filter(|(id, _)| id.is_none())
        .map(|(_, line)| *line)
        .collect()
}

/// Whether two documents are one ticket: the same `uuid` in their
/// frontmatter. Two tickets that only share an id are not.
fn same_document(ours: &str, theirs: &str) -> bool {
    let uuid = |text: &str| {
        openwarrant_core::format::parse(text.as_bytes())
            .fields
            .get("uuid")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
    };
    uuid(ours).is_some_and(|u| Some(u) == uuid(theirs))
}

/// A base for two copies of one document that git has no common ancestor
/// for: the lines before either side's first difference, and, of the items
/// both hold, each as both hold it or, where one side ticked it, as it was
/// before (an item is written open). An item the two sides hold differently
/// in any other way is left out, so [`items`] reads it as changed on both
/// sides and leaves it to git.
fn common_base(ours: &str, theirs: &str) -> String {
    let (Ok(a), Ok(b)) = (lines(ours), lines(theirs)) else {
        return String::new();
    };
    let done = |line: &str| ticket::parse(line).items.first().is_some_and(|i| i.done);
    let (pa, pb) = (prose(&a), prose(&b));
    let shared = pa.iter().zip(&pb).take_while(|(x, y)| x == y).count();
    let mut out: Vec<&str> = Vec::new();
    let mut prose_seen = 0;
    for (id, line) in &a.order {
        match id {
            None => {
                if prose_seen < shared {
                    out.push(line);
                }
                prose_seen += 1;
            }
            Some(id) => match b.by_id.get(id) {
                Some(other) if other == line => out.push(line),
                Some(other) if done(line) && !done(other) => out.push(other),
                Some(other) if done(other) && !done(line) => out.push(line),
                _ => {}
            },
        }
    }
    out.concat()
}

fn items(base: &str, ours: &str, theirs: &str) -> Result<String, String> {
    let (o, a, b) = (lines(base)?, lines(ours)?, lines(theirs)?);
    // The lines around the items: one side's edit, or the same edit.
    let (po, pa, pb) = (prose(&o), prose(&a), prose(&b));
    let take_theirs_prose = if pa == pb || pb == po {
        false
    } else if pa == po {
        true
    } else {
        return Err("the text around the items changed on both sides".to_owned());
    };
    let mut merged: BTreeMap<&str, Option<&str>> = BTreeMap::new();
    let ids: BTreeSet<&str> = o
        .by_id
        .keys()
        .chain(a.by_id.keys())
        .chain(b.by_id.keys())
        .map(String::as_str)
        .collect();
    for id in ids {
        let (lo, la, lb) = (
            o.by_id.get(id).copied(),
            a.by_id.get(id).copied(),
            b.by_id.get(id).copied(),
        );
        let line = if la == lb {
            la
        } else if lo == la {
            lb
        } else if lo == lb {
            la
        } else {
            return Err(format!("{id} changed differently on both sides"));
        };
        merged.insert(id, line);
    }
    // Ours' order, theirs' text lines if only theirs edited them; then the
    // items only theirs has, in theirs' order, after ours' last item.
    let (spine, other) = if take_theirs_prose {
        (&b, &a)
    } else {
        (&a, &b)
    };
    let mut out: Vec<String> = Vec::new();
    let mut last_item = None;
    for (id, line) in &spine.order {
        match id {
            None => out.push((*line).to_owned()),
            Some(id) => {
                if let Some(Some(l)) = merged.get(id.as_str()) {
                    out.push((*l).to_owned());
                    last_item = Some(out.len());
                }
            }
        }
    }
    let mut extra: Vec<String> = Vec::new();
    for (id, _) in &other.order {
        if let Some(id) = id
            && !spine.by_id.contains_key(id)
            && let Some(Some(l)) = merged.get(id.as_str())
        {
            extra.push((*l).to_owned());
        }
    }
    let at = last_item.unwrap_or(out.len());
    // A line taken from the end of a file without a newline gets one before
    // anything follows it.
    if !extra.is_empty() && at > 0 && !out[at - 1].ends_with('\n') {
        out[at - 1].push('\n');
    }
    for (k, l) in extra.into_iter().enumerate() {
        let mut l = l;
        if !l.ends_with('\n') {
            l.push('\n');
        }
        out.insert(at + k, l);
    }
    Ok(out.concat())
}

/// Both sides appended to the common text: common, ours' tail, theirs'
/// tail, with a `## Notes` heading theirs added dropped when ours has one.
fn appends(base: &str, ours: &str, theirs: &str) -> Option<String> {
    let mine = ours.strip_prefix(base)?;
    let mut tail = theirs.strip_prefix(base)?;
    let heading = super::NOTES_HEADING;
    if ours.lines().any(|l| l.trim_end() == heading) {
        let trimmed = tail.trim_start_matches('\n');
        if let Some(rest) = trimmed.strip_prefix(heading) {
            tail = rest.trim_start_matches('\n');
        }
    }
    let mut out = String::with_capacity(base.len() + mine.len() + tail.len() + 1);
    out.push_str(base);
    out.push_str(mine);
    if !out.is_empty() && !out.ends_with('\n') && !tail.is_empty() {
        out.push('\n');
    }
    out.push_str(tail);
    Some(out)
}

/// The command git runs for `merge=war-ticket`. A `war` that lacks the
/// driver (not on PATH, or older than M11) fails its probe, and git's own
/// text merge runs instead, so a missing tool never leaves one side's text
/// standing as if it were the merge.
pub const DRIVER_COMMAND: &str = "if war merge-ticket --probe >/dev/null 2>&1; then \
     war merge-ticket %O %A %B %P; else git merge-file -L ours -L base -L theirs %A %O %B; fi";

/// `war admin merge-ticket --install`, and `war init`: the `.gitattributes`
/// lines (each appended when absent) and this clone's driver in git's
/// configuration. Returns what it did, a line each.
pub fn install(root: &Utf8Path) -> Result<Vec<String>, String> {
    let mut did = Vec::new();
    let path = root.join(".gitattributes");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let have: BTreeSet<&str> = existing.lines().map(str::trim).collect();
    let missing: Vec<&str> = GITATTRIBUTES
        .lines()
        .filter(|l| !l.starts_with('#') && !have.contains(l.trim()))
        .collect();
    if !missing.is_empty() {
        let mut next = existing.clone();
        if !next.is_empty() && !next.ends_with('\n') {
            next.push('\n');
        }
        if missing.len()
            == GITATTRIBUTES
                .lines()
                .filter(|l| !l.starts_with('#'))
                .count()
        {
            next.push_str(GITATTRIBUTES);
        } else {
            for l in &missing {
                next.push_str(l);
                next.push('\n');
            }
        }
        std::fs::write(&path, next).map_err(|e| format!("could not write {path}: {e}"))?;
        did.push(format!("{path}: {}", missing.join("; ")));
    }
    for (key, value) in [
        (
            "name",
            "war: ticket checklists item by item, notes appended",
        ),
        ("driver", DRIVER_COMMAND),
    ] {
        let out = std::process::Command::new("git")
            .args(["config", &format!("merge.{DRIVER}.{key}"), value])
            .current_dir(root)
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|e| format!("could not run git config: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "git config merge.{DRIVER}.{key}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
    }
    did.push(format!(
        "git config merge.{DRIVER}.driver: `war admin merge-ticket` for this clone"
    ));
    Ok(did)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str =
        "# Checklist\n\n- [ ] One (i-0001)\n- [ ] Two (i-0002)\n- [ ] Three (i-0003)\n";

    #[test]
    fn adjacent_ticks_and_two_adds_merge_item_by_item() {
        let ours = BASE.replace(
            "- [ ] One (i-0001)",
            "- [x] One (i-0001) — done by a, 2026-10-07",
        ) + "- [ ] Ours (i-00aa)\n";
        let theirs = BASE.replace(
            "- [ ] Two (i-0002)",
            "- [x] Two (i-0002) — done by b, 2026-10-07",
        ) + "- [ ] Theirs (i-00bb)\n";
        let Merged::Items(m) = merge(
            "docs/tickets/t-1/atoms/15-checklist.md",
            BASE,
            &ours,
            &theirs,
        ) else {
            panic!("not merged item by item");
        };
        assert_eq!(
            m,
            "# Checklist\n\n- [x] One (i-0001) — done by a, 2026-10-07\n- [x] Two (i-0002) — done \
             by b, 2026-10-07\n- [ ] Three (i-0003)\n- [ ] Ours (i-00aa)\n- [ ] Theirs (i-00bb)\n"
        );
    }

    #[test]
    fn one_item_changed_two_ways_is_left_to_git() {
        let ours = BASE.replace("- [ ] One (i-0001)", "- [x] One (i-0001) — done by a");
        let theirs = BASE.replace("- [ ] One (i-0001)", "- [ ] One, reworded (i-0001)");
        assert!(matches!(
            merge("docs/tickets/t-1/atoms/15-checklist.md", BASE, &ours, &theirs),
            Merged::Text(why) if why.contains("i-0001")
        ));
        // Removed on one side, changed on the other: also git's.
        let gone = BASE.replace("- [ ] One (i-0001)\n", "");
        assert!(matches!(
            merge(
                "docs/tickets/t-1/atoms/15-checklist.md",
                BASE,
                &gone,
                &theirs
            ),
            Merged::Text(_)
        ));
    }

    /// A single-file ticket holds its checklist, so two branches ticking
    /// adjacent items and adding one each merge as a checklist does; one item
    /// changed two ways is still git's to stop on.
    #[test]
    fn a_single_file_ticket_merges_item_by_item() {
        let head = "---\nformat: openwarrant/1.0\nalias: t-43c0\n---\n# Layout\n\nWhy.\n\n## Checklist\n\n";
        let base = format!("{head}- [ ] One (i-0001)\n- [ ] Two (i-0002)\n");
        let ours = format!(
            "{head}- [x] One (i-0001) — done by a, 2026-11-06\n- [ ] Two (i-0002)\n- [ ] Added (i-0003)\n"
        );
        let theirs = format!(
            "{head}- [ ] One (i-0001)\n- [x] Two (i-0002) — done by b, 2026-11-06\n- [ ] Also (i-0004)\n"
        );
        let Merged::Items(m) = merge("docs/tickets/t-43c0.md", &base, &ours, &theirs) else {
            panic!("not merged item by item");
        };
        assert!(m.starts_with(head), "{m}");
        for line in [
            "- [x] One (i-0001) — done by a",
            "- [x] Two (i-0002) — done by b",
            "- [ ] Added (i-0003)",
            "- [ ] Also (i-0004)",
        ] {
            assert!(m.contains(line), "{line} in {m}");
        }
        let reworded = format!("{head}- [ ] One, reworded (i-0001)\n- [ ] Two (i-0002)\n");
        assert!(matches!(
            merge("docs/tickets/t-43c0.md", &base, &ours, &reworded),
            Merged::Text(why) if why.contains("i-0001")
        ));
    }

    /// R2: `war start` on a ticket held elsewhere writes the same document
    /// on a second branch, so the merge is add/add with no base.
    #[test]
    fn one_document_added_on_both_branches_merges_item_by_item() {
        let doc = |one: &str, two: &str, tail: &str| {
            format!(
                "---\nformat: openwarrant/1.0\nprofile: ticket\nalias: t-43c0\nuuid: 019a0000-0000-7000-8000-000000000001\n---\n# T\n\n## Checklist\n\n- [{one}] first `i-0001`\n- [{two}] second `i-0002`\n{tail}"
            )
        };
        // Ours ticked the first item and added a third; theirs ticked the
        // second.
        let ours = doc("x", " ", "- [ ] third `i-0003`\n");
        let theirs = doc(" ", "x", "");
        let Merged::Items(m) = merge("docs/tickets/t-43c0.md", "", &ours, &theirs) else {
            panic!("add/add of one document is left to git");
        };
        assert_eq!(m, doc("x", "x", "- [ ] third `i-0003`\n"));
        // Ours appended a note; theirs ticked an item.
        let noted = doc(" ", " ", "\n## Notes\n\n- a note\n");
        let Merged::Items(m) = merge("docs/tickets/t-43c0.md", "", &noted, &theirs) else {
            panic!("a note on one side and a tick on the other is left to git");
        };
        assert_eq!(m, doc(" ", "x", "\n## Notes\n\n- a note\n"));
        // The refusals: one item reworded two ways, and two tickets that
        // only share an id, stay git's to flag.
        let reworded = theirs.replace("second", "second, reworded");
        let other = ours.replace("second", "second, differently");
        assert!(matches!(
            merge("docs/tickets/t-43c0.md", "", &other, &reworded),
            Merged::Text(_)
        ));
        let stranger = theirs.replace("000000000001", "000000000002");
        assert!(matches!(
            merge("docs/tickets/t-43c0.md", "", &ours, &stranger),
            Merged::Text(_)
        ));
    }

    #[test]
    fn two_first_notes_keep_one_heading() {
        let base = "# T\n\nWhy.\n";
        let ours = format!("{base}\n## Notes\n\n- **a:** one\n");
        let theirs = format!("{base}\n## Notes\n\n- **b:** two\n");
        assert_eq!(
            merge("docs/tickets/t-1/atoms/10-intent.md", base, &ours, &theirs),
            Merged::Appends(format!("{base}\n## Notes\n\n- **a:** one\n- **b:** two\n"))
        );
        let edited = "# T\n\nWhy, rewritten.\n";
        assert!(matches!(
            merge("docs/tickets/t-1/atoms/10-intent.md", base, edited, &theirs),
            Merged::Text(_)
        ));
    }
}
