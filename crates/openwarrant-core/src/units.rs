// SPDX-License-Identifier: Apache-2.0
//! The units of any Markdown document: what `war context` can select from a
//! long document without a Warrant or a type around it.
//!
//! A `##` heading outside a fence opens a unit, with the id and span an
//! instruction section has ([`crate::instruction`]), so `md:<file>#<slug>`
//! names the same bytes in either reader. A `###` heading opens a unit
//! nested in it, whose slug comes after every `##` slug, so adding a `###`
//! never renames a `##`. Text before the first `##` is the unit `lead`.
//!
//! An explicit id keeps a citation stable when a heading is reworded:
//! `## Friendly fire {#friendly-fire}`. The `{#…}` is not part of the title.
//! An id two headings claim stays with the first; the second gets a slug.
//!
//! Pure: a parse over text (§79.1).

use std::collections::BTreeSet;

use crate::instruction::slug;
use crate::record::{fence_closes, fence_open};

/// The id of the text before the first `##`.
pub const LEAD: &str = "lead";

/// One unit of a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    /// Its slug, unique in the document.
    pub id: String,
    /// The heading without its `#`s or explicit id.
    pub title: String,
    /// 1 for the lead, 2 for `##`, 3 for `###`.
    pub level: u8,
    /// 1-based line of its heading.
    pub line: usize,
    /// `[start, end)` in UTF-8 bytes.
    pub start: usize,
    pub end: usize,
    /// The `##` unit a `###` unit sits in.
    pub parent: Option<String>,
}

struct Heading {
    level: u8,
    title: String,
    explicit: Option<String>,
    line: usize,
    start: usize,
}

/// `(title, explicit id)` of a heading's text.
fn split_explicit(text: &str) -> (String, Option<String>) {
    let text = text.trim().trim_end_matches('#').trim_end();
    if let Some(open) = text.rfind("{#")
        && let Some(id) = text[open + 2..].strip_suffix('}')
        && !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return (text[..open].trim_end().to_owned(), Some(id.to_owned()));
    }
    (text.to_owned(), None)
}

/// The byte just past a leading frontmatter block, and the lines it took.
fn past_frontmatter(text: &str) -> (usize, usize) {
    let mut lines = text.split_inclusive('\n');
    if lines.next().map(str::trim_end) != Some("---") {
        return (0, 0);
    }
    let mut pos = 4;
    for (i, raw) in lines.enumerate() {
        pos += raw.len();
        if raw.trim_end() == "---" {
            return (pos.min(text.len()), i + 2);
        }
    }
    (0, 0)
}

fn headings(text: &str, from: usize, from_line: usize) -> Vec<Heading> {
    let mut out = Vec::new();
    let mut fence = None;
    let mut pos = from;
    for (i, raw) in text[from..].split_inclusive('\n').enumerate() {
        let start = pos;
        pos += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        if let Some(open) = fence {
            if fence_closes(line, open) {
                fence = None;
            }
            continue;
        }
        if let Some(open) = fence_open(line) {
            fence = Some(open);
            continue;
        }
        let (level, rest) = if let Some(r) = line.strip_prefix("### ") {
            (3, r)
        } else if let Some(r) = line.strip_prefix("## ") {
            (2, r)
        } else if let Some(r) = line.strip_prefix("# ") {
            (1, r)
        } else {
            continue;
        };
        let (title, explicit) = split_explicit(rest);
        out.push(Heading {
            level,
            title,
            explicit,
            line: from_line + i + 1,
            start,
        });
    }
    out
}

/// `base`, or the first of `base-1`, `base-2`, … not yet taken.
fn unique(base: String, seen: &mut BTreeSet<String>) -> String {
    let base = if base.is_empty() {
        "section".to_owned()
    } else {
        base
    };
    let mut id = base.clone();
    let mut n = 0;
    while seen.contains(&id) {
        n += 1;
        id = format!("{base}-{n}");
    }
    seen.insert(id.clone());
    id
}

/// Every unit of `text`, in document order: a `###` follows its `##`.
#[must_use]
pub fn scan(text: &str) -> Vec<Unit> {
    let (from, from_line) = past_frontmatter(text);
    let found = headings(text, from, from_line);
    let mut seen: BTreeSet<String> = BTreeSet::from([LEAD.to_owned()]);
    // Explicit ids first, so a slug never takes one that is claimed later.
    let mut ids: Vec<Option<String>> = found
        .iter()
        .map(|h| {
            h.explicit
                .as_ref()
                .filter(|id| h.level > 1 && seen.insert((*id).clone()))
                .cloned()
        })
        .collect();
    for level in [2, 3] {
        for (h, id) in found.iter().zip(ids.iter_mut()) {
            if h.level == level && id.is_none() {
                *id = Some(unique(slug(&h.title), &mut seen));
            }
        }
    }
    let first_two = found.iter().position(|h| h.level == 2);
    let lead_end = first_two.map_or(text.len(), |i| found[i].start);
    let mut out = Vec::with_capacity(found.len() + 1);
    if !text[from..lead_end].trim().is_empty() {
        let title = found
            .iter()
            .take_while(|h| h.start < lead_end)
            .find(|h| h.level == 1)
            .map_or_else(|| "Lead".to_owned(), |h| h.title.clone());
        out.push(Unit {
            id: LEAD.to_owned(),
            title,
            level: 1,
            line: from_line + 1,
            start: from,
            end: lead_end,
            parent: None,
        });
    }
    let mut parent: Option<String> = None;
    for (i, h) in found.iter().enumerate() {
        if h.level == 1 || (h.level == 3 && parent.is_none()) {
            continue;
        }
        let end = found[i + 1..]
            .iter()
            .find(|n| n.level <= h.level)
            .map_or(text.len(), |n| n.start);
        let id = ids[i].clone().unwrap_or_default();
        if h.level == 2 {
            parent = Some(id.clone());
        }
        out.push(Unit {
            id,
            title: h.title.clone(),
            level: h.level,
            line: h.line,
            start: h.start,
            end,
            parent: (h.level == 3).then(|| parent.clone()).flatten(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "---\nformat: x\n---\n# Combat\n\nThe lead.\n\n## Damage\n\ntext\n\n### Armour\n\nmore\n\n## Friendly fire {#ff}\n\n```\n## not a heading\n```\n\n### Armour\n\nagain\n";

    fn ids(text: &str) -> Vec<String> {
        scan(text).into_iter().map(|u| u.id).collect()
    }

    #[test]
    fn units_nest_and_ids_come_from_headings() {
        assert_eq!(ids(DOC), ["lead", "damage", "armour", "ff", "armour-1"]);
        let units = scan(DOC);
        assert_eq!(units[0].title, "Combat");
        assert_eq!(units[3].title, "Friendly fire");
        assert_eq!(units[2].parent.as_deref(), Some("damage"));
        assert_eq!(units[4].parent.as_deref(), Some("ff"));
    }

    #[test]
    fn a_unit_ends_at_the_next_heading_of_its_level_or_higher() {
        let units = scan(DOC);
        let text = |u: &Unit| &DOC[u.start..u.end];
        assert!(text(&units[1]).starts_with("## Damage"));
        assert!(text(&units[1]).contains("### Armour"));
        assert!(text(&units[1]).ends_with("more\n\n"));
        assert!(text(&units[3]).contains("## not a heading"));
        assert_eq!(units[1].line, 8);
    }

    #[test]
    fn a_new_subsection_never_renames_a_section() {
        let before = "## Budget\nx\n## Budget\ny\n";
        let after = "## Intro\n### Budget\nz\n## Budget\nx\n## Budget\ny\n";
        assert_eq!(ids(before), ["budget", "budget-1"]);
        // The `###` took its slug after every `##`.
        assert_eq!(ids(after), ["intro", "budget-2", "budget", "budget-1"]);
    }

    #[test]
    fn section_ids_match_instruction_sections() {
        let text = "# T\n\n## Build\nx\n### Sub\ny\n## Build\nz\n";
        let sections: Vec<String> = crate::instruction::scan(text)
            .sections
            .into_iter()
            .map(|s| s.slug)
            .collect();
        let units: Vec<String> = scan(text)
            .into_iter()
            .filter(|u| u.level == 2)
            .map(|u| u.id)
            .collect();
        assert_eq!(sections, units);
    }

    #[test]
    fn an_explicit_id_survives_a_reworded_heading_and_a_claimed_one_does_not_repeat() {
        assert_eq!(ids("## New words {#ff}\nx\n"), ["ff"]);
        assert_eq!(ids("## A {#ff}\nx\n## B {#ff}\ny\n"), ["ff", "b"]);
        // Not an id: left in the title.
        let u = scan("## Use {#a b}\nx\n");
        assert_eq!(u[0].title, "Use {#a b}");
    }

    #[test]
    fn an_empty_lead_is_no_unit_and_a_document_with_no_sections_is_all_lead() {
        assert_eq!(ids("# Title\n\n## One\nx\n").len(), 2);
        assert_eq!(ids("\n\n## One\nx\n"), ["one"]);
        assert_eq!(ids("just prose\n"), ["lead"]);
        assert!(scan("").is_empty());
    }
}
