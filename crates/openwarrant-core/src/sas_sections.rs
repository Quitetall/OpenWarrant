// SPDX-License-Identifier: Apache-2.0
//! The SAS split into sections, losslessly (OW-WAR-0125, OW-ADR-0028).
//!
//! # What a section is
//!
//! A unit of the document is one of:
//!
//! - the **preamble**: every byte before the first boundary below (the title,
//!   the subtitle and the front-matter table);
//! - a **part**: a `# Part <n> — <title>` line and its prose up to the next
//!   boundary;
//! - an **appendix**: a `# Appendix <x> — <title>` line and everything up to
//!   the next boundary;
//! - a **numbered section**: a `## <n>. <title>` line and everything up to the
//!   next boundary, including its `### <n>.<m>` subsections.
//!
//! A boundary is one of those three heading forms at column zero, outside a
//! fenced code block. `###` and deeper stay inside their section. A heading
//! inside a fence is an example, not a section: the SAS shows whole Warrants
//! and ADR Overviews in fences, and giving their headings an identity would
//! give the document's own examples a place in it.
//!
//! # Lossless
//!
//! Sections are contiguous byte ranges that cover the input in order, so
//! [`join`] of [`split`] gives back the input byte for byte. That is what lets
//! a section digest stand for part of a revision: the revision's sha256 is
//! still the digest of the join.
//!
//! # Identity
//!
//! A numbered section's id is its number as written (`43`); a subsection's is
//! `43.5`. Cited as `sas://<NS>-SAS-43` or `sas://<NS>-SAS-43.5` ([`parse_ref`]).
//! The structural units are `preamble`, `part-<n>` and `appendix-<x>`; they
//! have digests and no citation form.
//!
//! No I/O: bytes in, values out.

use serde::Serialize;
use sha2::{Digest, Sha256};

/// Which kind of unit a [`Section`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionKind {
    Preamble,
    Part,
    Appendix,
    Numbered,
}

/// A `### <n>.<m>` heading inside a numbered section: its id, title, byte
/// range (to the next `###`-or-higher heading outside a fence, or the end of
/// the section) and sha256.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Subsection {
    pub id: String,
    pub title: String,
    pub start: usize,
    pub end: usize,
    pub sha256: String,
}

/// One unit of the document. `bytes` is the unit itself; `start..end` is where
/// it sits in the input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Section {
    pub id: String,
    pub kind: SectionKind,
    pub title: String,
    pub start: usize,
    pub end: usize,
    pub sha256: String,
    pub subsections: Vec<Subsection>,
    #[serde(skip)]
    pub bytes: Vec<u8>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A fence opener or closer: the run of backticks or tildes a line starts with
/// after at most three spaces, and what follows it.
fn fence_run(line: &[u8]) -> Option<(u8, usize, &[u8])> {
    let indent = line.iter().take_while(|b| **b == b' ').count();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let ch = *rest.first()?;
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let run = rest.iter().take_while(|b| **b == ch).count();
    (run >= 3).then(|| (ch, run, &rest[run..]))
}

/// The line without its terminator (`\n` or `\r\n`).
fn content(line: &[u8]) -> &[u8] {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r").unwrap_or(line)
}

/// `(digits, rest)` when `s` starts with one or more ASCII digits.
fn leading_number(s: &[u8]) -> Option<(&str, &[u8])> {
    let n = s.iter().take_while(|b| b.is_ascii_digit()).count();
    (n > 0).then(|| (std::str::from_utf8(&s[..n]).expect("ascii digits"), &s[n..]))
}

fn text(s: &[u8]) -> String {
    String::from_utf8_lossy(s).trim().to_owned()
}

/// A boundary heading: `(kind, id, title)`.
fn boundary(line: &[u8]) -> Option<(SectionKind, String, String)> {
    let line = content(line);
    if let Some(rest) = line.strip_prefix(b"## ") {
        let (n, rest) = leading_number(rest)?;
        let title = rest.strip_prefix(b". ")?;
        return Some((SectionKind::Numbered, n.to_owned(), text(title)));
    }
    for (prefix, kind, label) in [
        (&b"# Part "[..], SectionKind::Part, "part"),
        (&b"# Appendix "[..], SectionKind::Appendix, "appendix"),
    ] {
        if let Some(rest) = line.strip_prefix(prefix) {
            let token_len = rest.iter().take_while(|b| !b.is_ascii_whitespace()).count();
            if token_len == 0 {
                return None;
            }
            let token = text(&rest[..token_len]);
            let title = text(&rest[token_len..]);
            let title = title.trim_start_matches('—').trim().to_owned();
            return Some((kind, format!("{label}-{token}"), title));
        }
    }
    None
}

/// A `### <n>.<m> <title>` heading: `(id, title)`.
fn subsection_heading(line: &[u8]) -> Option<(String, String)> {
    let rest = content(line).strip_prefix(b"### ")?;
    let (major, rest) = leading_number(rest)?;
    let rest = rest.strip_prefix(b".")?;
    let (minor, rest) = leading_number(rest)?;
    if !(rest.is_empty() || rest.starts_with(b" ")) {
        return None;
    }
    Some((format!("{major}.{minor}"), text(rest)))
}

/// Any ATX heading of level 1 to 3 at column zero: a subsection ends there.
fn heading_level(line: &[u8]) -> usize {
    let level = line.iter().take_while(|b| **b == b'#').count();
    if level > 0 && matches!(line.get(level), Some(b' ' | b'\n' | b'\r') | None) {
        level
    } else {
        0
    }
}

/// Split `bytes` into its sections, in order. Contiguous and covering: the
/// first starts at 0, each starts where the last ended, the last ends at
/// `bytes.len()`. The preamble is present even when empty, so an input with
/// no boundary is one preamble.
#[must_use]
pub fn split(bytes: &[u8]) -> Vec<Section> {
    struct Open {
        kind: SectionKind,
        id: String,
        title: String,
        start: usize,
        subs: Vec<(String, String, usize)>,
        // Ends of subsections: the start of any heading of level <= 3 after
        // a subsection opened.
        cuts: Vec<usize>,
    }
    let mut opens = vec![Open {
        kind: SectionKind::Preamble,
        id: "preamble".to_owned(),
        title: "Front matter".to_owned(),
        start: 0,
        subs: Vec::new(),
        cuts: Vec::new(),
    }];
    let mut fence: Option<(u8, usize)> = None;
    let mut pos = 0usize;
    for line in bytes.split_inclusive(|b| *b == b'\n') {
        let at = pos;
        pos += line.len();
        if let Some((ch, run)) = fence {
            if let Some((c, r, rest)) = fence_run(content(line))
                && c == ch
                && r >= run
                && rest.iter().all(u8::is_ascii_whitespace)
            {
                fence = None;
            }
            continue;
        }
        if let Some((c, r, _)) = fence_run(content(line)) {
            fence = Some((c, r));
            continue;
        }
        if let Some((kind, id, title)) = boundary(line) {
            opens.push(Open {
                kind,
                id,
                title,
                start: at,
                subs: Vec::new(),
                cuts: Vec::new(),
            });
            continue;
        }
        let current = opens.last_mut().expect("preamble is always open");
        if current.kind == SectionKind::Numbered
            && let Some((id, title)) = subsection_heading(line)
        {
            current.cuts.push(at);
            current.subs.push((id, title, at));
            continue;
        }
        let level = heading_level(line);
        if level > 0 && level <= 3 {
            current.cuts.push(at);
        }
    }

    let ends: Vec<usize> = opens
        .iter()
        .skip(1)
        .map(|o| o.start)
        .chain(std::iter::once(bytes.len()))
        .collect();
    opens
        .into_iter()
        .zip(ends)
        .map(|(o, end)| {
            let subsections = o
                .subs
                .iter()
                .map(|(id, title, start)| {
                    let sub_end = o.cuts.iter().copied().find(|c| c > start).unwrap_or(end);
                    Subsection {
                        id: id.clone(),
                        title: title.clone(),
                        start: *start,
                        end: sub_end,
                        sha256: sha256_hex(&bytes[*start..sub_end]),
                    }
                })
                .collect();
            let slice = &bytes[o.start..end];
            Section {
                id: o.id,
                kind: o.kind,
                title: o.title,
                start: o.start,
                end,
                sha256: sha256_hex(slice),
                subsections,
                bytes: slice.to_vec(),
            }
        })
        .collect()
}

/// The sections back into one document, byte for byte.
#[must_use]
pub fn join(sections: &[Section]) -> Vec<u8> {
    let mut out = Vec::with_capacity(sections.iter().map(|s| s.bytes.len()).sum());
    for s in sections {
        out.extend_from_slice(&s.bytes);
    }
    out
}

/// A reference to a numbered section or subsection: `sas://<NS>-SAS-<n>` or
/// `sas://<NS>-SAS-<n>.<m>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionRef {
    /// The namespace as written (`WAR` in `sas://WAR-SAS-43.5`).
    pub namespace: String,
    /// `43` or `43.5`.
    pub section: String,
}

impl SectionRef {
    /// The number of the section this names: `43` for `43.5`.
    #[must_use]
    pub fn major(&self) -> &str {
        self.section.split('.').next().unwrap_or(&self.section)
    }
}

/// Read a section reference.
///
/// `Ok(None)` when `s` is not one at all (another scheme, or a §106
/// requirement reference such as `sas://WAR-SAS-RQ-022`). `Err` when it is a
/// `sas://<NS>-SAS-…` reference whose remainder is neither a requirement id
/// nor `<n>` or `<n>.<m>`: a malformed section reference is refused, not
/// passed over as prose.
pub fn parse_ref(s: &str) -> Result<Option<SectionRef>, String> {
    let Some(rest) = s.trim().strip_prefix("sas://") else {
        return Ok(None);
    };
    let Some((namespace, tail)) = rest.split_once("-SAS-") else {
        return Ok(None);
    };
    if tail.starts_with("RQ-") {
        return Ok(None);
    }
    let digits = |p: &str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
    let well_formed = match tail.split_once('.') {
        None => digits(tail),
        Some((a, b)) => digits(a) && digits(b),
    };
    if namespace.is_empty() || !well_formed {
        return Err(format!(
            "{s:?} is not a section reference: the form is sas://<NS>-SAS-<n> or \
             sas://<NS>-SAS-<n>.<m>"
        ));
    }
    Ok(Some(SectionRef {
        namespace: namespace.to_owned(),
        section: tail.to_owned(),
    }))
}

/// The digest of what `reference` names in `sections`: the numbered
/// section's, or the subsection's. `None` when the document has no such
/// section or subsection. The first match wins, so a document that numbers
/// two sections alike resolves to the earlier one.
#[must_use]
pub fn resolve<'a>(sections: &'a [Section], reference: &SectionRef) -> Option<&'a str> {
    let section = sections
        .iter()
        .find(|s| s.kind == SectionKind::Numbered && s.id == reference.major())?;
    if reference.section.contains('.') {
        section
            .subsections
            .iter()
            .find(|sub| sub.id == reference.section)
            .map(|sub| sub.sha256.as_str())
    } else {
        Some(section.sha256.as_str())
    }
}

/// The namespace the document's §106 rows carry (`WAR` for `WAR-SAS-RQ-001`),
/// which is the namespace its section references carry. `None` when §106 has
/// no rows.
#[must_use]
pub fn namespace(text: &str) -> Option<String> {
    crate::sas::section_106(text)
        .keys()
        .find_map(|id| id.split_once("-SAS-RQ-").map(|(ns, _)| ns.to_owned()))
}

/// What changed between two splits, by section id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SectionDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
}

impl SectionDiff {
    /// Compared by id and digest. Order is the order ids appear in `after`
    /// (added, changed) and in `before` (removed).
    #[must_use]
    pub fn between(before: &[Section], after: &[Section]) -> Self {
        let find = |set: &'_ [Section], id: &str| -> Option<String> {
            set.iter().find(|s| s.id == id).map(|s| s.sha256.clone())
        };
        let mut out = Self::default();
        for s in after {
            match find(before, &s.id) {
                None => out.added.push(s.id.clone()),
                Some(d) if d != s.sha256 => out.changed.push(s.id.clone()),
                Some(_) => {}
            }
        }
        for s in before {
            if find(after, &s.id).is_none() {
                out.removed.push(s.id.clone());
            }
        }
        out
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "# Title\n\n## Sub-title\n\n| a | b |\n\n# Part I — First\n\nintro\n\n## 1. One\n\ntext\n\n### 1.1 Alpha\n\na\n\n### 1.2 Beta\n\n```markdown\n## 999. Fake\n### 1.9 Fake\n# Part IX — Fake\n```\n\nb\n\n#### deep\n\n## 2. Two\n\n### Law 1 — not numbered\n\n# Appendix A — Shapes\n\n~~~~\n```\n## 998. Also fake\n```\n~~~~\nend";

    #[test]
    fn join_of_split_is_the_input_and_ranges_cover_it() {
        for input in [DOC, "", "no headings\n", "## 1. Only\n", "\n\n## 3. X\r\n"] {
            let sections = split(input.as_bytes());
            assert_eq!(join(&sections), input.as_bytes());
            let mut at = 0;
            for s in &sections {
                assert_eq!(s.start, at);
                assert_eq!(&input.as_bytes()[s.start..s.end], s.bytes.as_slice());
                at = s.end;
            }
            assert_eq!(at, input.len());
        }
    }

    #[test]
    fn sections_are_the_boundaries_outside_fences() {
        let sections = split(DOC.as_bytes());
        let ids: Vec<&str> = sections.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(
            ids,
            ["preamble", "part-I", "1", "2", "appendix-A"],
            "a heading in a fence (``` or ~~~~) is not a section"
        );
        assert_eq!(sections[2].title, "One");
        assert_eq!(sections[1].title, "First");
        let subs: Vec<&str> = sections[2]
            .subsections
            .iter()
            .map(|s| s.id.as_str())
            .collect();
        assert_eq!(subs, ["1.1", "1.2"]);
        // 1.2 runs through its fence and stops at no `####`, only at the end.
        let beta = &sections[2].subsections[1];
        assert!(DOC[beta.start..beta.end].contains("## 999. Fake"));
        assert!(DOC[beta.start..beta.end].contains("#### deep"));
        assert!(sections[3].subsections.is_empty());
    }

    #[test]
    fn an_unfenced_heading_is_a_section() {
        // The refusal beside the fence case: the same line outside a fence
        // does make a section, so the fence test is not passing vacuously.
        let doc = DOC.replace("```markdown\n## 999. Fake", "## 999. Fake\n```markdown");
        let ids: Vec<String> = split(doc.as_bytes()).into_iter().map(|s| s.id).collect();
        assert!(ids.contains(&"999".to_owned()));
    }

    #[test]
    fn references_parse_and_resolve() {
        let sections = split(DOC.as_bytes());
        let r = parse_ref("sas://WAR-SAS-1.2").unwrap().unwrap();
        assert_eq!((r.namespace.as_str(), r.major()), ("WAR", "1"));
        assert!(resolve(&sections, &r).is_some());
        let whole = parse_ref("sas://WAR-SAS-2").unwrap().unwrap();
        assert_eq!(
            resolve(&sections, &whole),
            Some(sections[3].sha256.as_str())
        );
        for missing in [
            "sas://WAR-SAS-999",
            "sas://WAR-SAS-1.9",
            "sas://WAR-SAS-2.1",
        ] {
            let r = parse_ref(missing).unwrap().unwrap();
            assert!(resolve(&sections, &r).is_none(), "{missing}");
        }
        assert_eq!(parse_ref("sas://WAR-SAS-RQ-022"), Ok(None));
        assert_eq!(parse_ref("adr://OW-ADR-0016"), Ok(None));
        for bad in [
            "sas://WAR-SAS-43.5.1",
            "sas://WAR-SAS-x",
            "sas://-SAS-4",
            "sas://WAR-SAS-",
        ] {
            assert!(parse_ref(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_diff_names_only_the_section_that_changed() {
        let before = split(DOC.as_bytes());
        assert!(SectionDiff::between(&before, &before).is_empty());
        let after = split(DOC.replace("\na\n", "\nA\n").as_bytes());
        let d = SectionDiff::between(&before, &after);
        assert_eq!(d.changed, ["1"]);
        assert!(d.added.is_empty() && d.removed.is_empty());
    }
}
