// SPDX-License-Identifier: Apache-2.0
//! Instruction files (`CLAUDE.md`, `AGENTS.md`) as records, and the managed
//! pointer block `war` keeps in them (M16, decision 13).
//!
//! # Sections
//!
//! An instruction file is the repository's own Markdown, read as it is and
//! never edited by this reader. Each unindented `## ` heading outside a
//! fenced code block opens a **section**, a record of type `instruction`
//! whose id is `md:<file>#<slug>`: `<file>` the repository-relative path,
//! `<slug>` the heading as GitHub anchors it (lowercased; letters, digits,
//! `-` and `_` kept; spaces to `-`; everything else dropped; a repeated
//! slug gets `-1`, `-2`, … in order; an empty one is `section`).
//!
//! A section's span is `[start, end)` in UTF-8 bytes, from the first byte of
//! its heading line to the first byte of the next `#` or `##` heading, of
//! the managed block's begin marker, or the end of the file. Nested
//! headings, blank lines and line endings are part of it; nothing is
//! normalized. Its revision is `sha256:` over exactly those bytes, so an
//! edit to one section moves that section's revision and no other. Text
//! before the first section, under a `#` heading, or after the managed
//! block and before the next heading belongs to no section.
//!
//! Fences are recognised as in [`crate::record`]. An unclosed fence runs to
//! the end of the file, as CommonMark reads it; the reader does not refuse
//! the user's file for it.
//!
//! # The managed block
//!
//! ```text
//! <!-- openwarrant:begin -->
//! …
//! <!-- openwarrant:end -->
//! ```
//!
//! Each marker is a whole unindented line outside a fence. The block is
//! `war`'s: [`block_text`] is what it holds, [`upsert`] inserts or replaces
//! it, and every byte outside the markers is left as it was. It is no
//! section and inside no section's span, so rewriting it (a new version
//! stamp) moves no section's revision. The reader is lenient and the writer
//! is strict: a second block, a begin with no end, or an end with no begin
//! is reported ([`BlockError`]) by [`scan`] and refused by [`upsert`].
//!
//! # Citations
//!
//! A relation or a document names a section by its id, `md:CLAUDE.md#testing`
//! ([`crate::relation::is_target`]), optionally pinning a revision
//! (`@sha256:<hex>`). [`citations`] finds the ids a prose text names.
//!
//! Pure: parses over text (§79.1).

use sha2::{Digest as _, Sha256};

use crate::record::{fence_closes, fence_open};

/// The record type of a section.
pub const RECORD_TYPE: &str = "instruction";

/// The prefix of a section id.
pub const ID_PREFIX: &str = "md:";

/// The line that opens the managed block.
pub const BLOCK_BEGIN: &str = "<!-- openwarrant:begin -->";

/// The line that closes the managed block.
pub const BLOCK_END: &str = "<!-- openwarrant:end -->";

/// The start of the version stamp line inside the block: the line the CLI's
/// version-skew check reads, and the one the full AGENTS.md template ends
/// with.
pub const STAMP_PREFIX: &str = "<!-- openwarrant agents-md: written by war ";

/// One `##` section of an instruction file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// The heading text after `## `, as written (closing hashes trimmed).
    pub heading: String,
    pub slug: String,
    /// 1-based line of the heading.
    pub line: usize,
    /// `[start, end)` byte offsets of its span.
    pub start: usize,
    pub end: usize,
    /// `sha256:<hex>` of the span's bytes.
    pub revision: String,
}

/// Where the managed block sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    /// First byte of the begin marker's line.
    pub start: usize,
    /// One past the end marker's last byte, before its line ending.
    pub end: usize,
    /// 1-based lines of the two markers.
    pub begin_line: usize,
    pub end_line: usize,
}

/// A block a writer cannot place.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BlockError {
    #[error(
        "line {second}: a second openwarrant block (the first opens at line {first}); a file \
         holds one, so `war` cannot tell which to keep. Delete one of them, markers included"
    )]
    Duplicate { first: usize, second: usize },
    #[error(
        "line {line}: an openwarrant block opens here (`{BLOCK_BEGIN}`) and is never closed \
         with `{BLOCK_END}`. Add the end marker after the block, or delete the begin marker"
    )]
    Unterminated { line: usize },
    #[error(
        "line {line}: `{BLOCK_END}` closes no block; no `{BLOCK_BEGIN}` comes before it. \
         Delete it, or add the begin marker before the block"
    )]
    Unopened { line: usize },
    #[error(
        "line {line}: a fenced code block opened here is never closed, so a block appended at \
         the end would be read as code. Close the fence first"
    )]
    FenceUnclosed { line: usize },
}

impl BlockError {
    /// The rule each refusal is reported under.
    #[must_use]
    pub const fn rule(&self) -> &'static str {
        match self {
            Self::Duplicate { .. } => "agents-md.block-duplicate",
            Self::Unterminated { .. } => "agents-md.block-unterminated",
            Self::Unopened { .. } => "agents-md.block-unopened",
            Self::FenceUnclosed { .. } => "agents-md.fence-unclosed",
        }
    }

    /// The 1-based line it is about.
    #[must_use]
    pub const fn line(&self) -> usize {
        match self {
            Self::Duplicate { second: line, .. }
            | Self::Unterminated { line }
            | Self::Unopened { line }
            | Self::FenceUnclosed { line } => *line,
        }
    }
}

/// An instruction file as read: its sections, its blocks, and what is
/// wrong with the blocks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scan {
    pub sections: Vec<Section>,
    /// Every complete block, in order; an unterminated one is not here.
    pub blocks: Vec<Block>,
    /// Block problems, in line order. Sections are read regardless: an
    /// unterminated block runs to the end of the file and holds no section.
    pub problems: Vec<BlockError>,
    /// The line of a fence still open at the end of the file.
    pub open_fence: Option<usize>,
}

fn digest(bytes: &[u8]) -> String {
    format!(
        "sha256:{}",
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}

/// The heading level (1 or 2) and text of `line`, when it is an
/// unindented `#` or `##` heading.
fn heading(line: &str) -> Option<(usize, &str)> {
    let level = line.bytes().take_while(|b| *b == b'#').count();
    if !(1..=2).contains(&level) {
        return None;
    }
    let rest = &line[level..];
    if !(rest.is_empty() || rest.starts_with([' ', '\t'])) {
        return None;
    }
    // An ATX closing sequence (`## Title ##`) is not part of the text.
    let text = rest.trim();
    let text = match text.trim_end_matches('#') {
        t if t.len() < text.len() && (t.is_empty() || t.ends_with([' ', '\t'])) => t.trim_end(),
        _ => text,
    };
    Some((level, text))
}

/// The GitHub anchor of a heading's text, before de-duplication.
#[must_use]
pub fn slug(heading: &str) -> String {
    // A link's target is not part of its text: `[text](url)` anchors as
    // `text`.
    let mut plain = String::with_capacity(heading.len());
    let mut rest = heading;
    while let Some(i) = rest.find("](") {
        plain.push_str(&rest[..i]);
        rest = &rest[i + 2..];
        match rest.find(')') {
            Some(j) => rest = &rest[j + 1..],
            None => rest = "",
        }
    }
    plain.push_str(rest);
    let mut out = String::with_capacity(plain.len());
    for c in plain.trim().chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            out.extend(c.to_lowercase());
        } else if c == ' ' {
            out.push('-');
        }
    }
    out
}

/// The id of section `slug` in `file`: `md:<file>#<slug>`.
#[must_use]
pub fn section_id(file: &str, slug: &str) -> String {
    format!("{ID_PREFIX}{file}#{slug}")
}

/// Whether `s` is a section id: `md:`, a relative path (no whitespace, `#`,
/// `@` or `,`; no empty, `.` or `..` segment), `#`, and a slug (no
/// whitespace, `#`, `@` or `,`).
#[must_use]
pub fn is_section_id(s: &str) -> bool {
    let Some((path, slug)) = s.strip_prefix(ID_PREFIX).and_then(|r| r.split_once('#')) else {
        return false;
    };
    let plain = |t: &str| {
        !t.is_empty()
            && !t
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '#' | '@' | ','))
    };
    plain(path) && plain(slug) && path.split('/').all(|seg| !matches!(seg, "" | "." | ".."))
}

/// Read an instruction file: its sections and its managed blocks.
#[must_use]
pub fn scan(text: &str) -> Scan {
    let mut out = Scan::default();
    let mut fence: Option<((char, usize), usize)> = None;
    // (start, line) of a block that is open.
    let mut block: Option<(usize, usize)> = None;
    // (heading, line, start) of the section that is open.
    let mut open: Option<(String, usize, usize)> = None;
    let mut seen: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut close = |open: &mut Option<(String, usize, usize)>, end: usize, out: &mut Scan| {
        if let Some((heading, line, start)) = open.take() {
            let base = match slug(&heading) {
                s if s.is_empty() => "section".to_owned(),
                s => s,
            };
            // GitHub's rule: the first is `x`, then `x-1`, `x-2`, … skipping
            // any that an earlier heading already took.
            let mut slug = base.clone();
            let mut n = seen.get(&base).copied().unwrap_or(0);
            while seen.contains_key(&slug) {
                n += 1;
                slug = format!("{base}-{n}");
            }
            seen.insert(base.clone(), n);
            seen.entry(slug.clone()).or_insert(0);
            out.sections.push(Section {
                heading,
                slug,
                line,
                start,
                end,
                revision: digest(&text.as_bytes()[start..end]),
            });
        }
    };
    let mut pos = 0usize;
    for (i, raw) in text.split_inclusive('\n').enumerate() {
        let line_no = i + 1;
        let start = pos;
        pos += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        let marker = line.trim_end_matches([' ', '\t']);
        if let Some((b_start, b_line)) = block {
            if marker == BLOCK_END {
                out.blocks.push(Block {
                    start: b_start,
                    end: start + line.len(),
                    begin_line: b_line,
                    end_line: line_no,
                });
                block = None;
            } else if marker == BLOCK_BEGIN {
                // A begin inside a block: the first never closed before it.
                out.problems.push(BlockError::Unterminated { line: b_line });
                block = Some((start, line_no));
            }
            continue;
        }
        if let Some((f, _)) = fence {
            if fence_closes(line, f) {
                fence = None;
            }
            continue;
        }
        if let Some(f) = fence_open(line) {
            fence = Some((f, line_no));
            continue;
        }
        if marker == BLOCK_BEGIN {
            close(&mut open, start, &mut out);
            block = Some((start, line_no));
            continue;
        }
        if marker == BLOCK_END {
            out.problems.push(BlockError::Unopened { line: line_no });
            continue;
        }
        if let Some((level, text_)) = heading(line) {
            close(&mut open, start, &mut out);
            if level == 2 {
                open = Some((text_.to_owned(), line_no, start));
            }
        }
    }
    close(&mut open, text.len(), &mut out);
    if let Some((_, line)) = block {
        out.problems.push(BlockError::Unterminated { line });
    }
    if let [first, second, ..] = out.blocks.as_slice() {
        out.problems.push(BlockError::Duplicate {
            first: first.begin_line,
            second: second.begin_line,
        });
    }
    out.problems.sort_by_key(BlockError::line);
    out.open_fence = fence.map(|(_, line)| line);
    out
}

/// The one block of `text`, if it has one; `Err` with the first problem
/// when a writer could not tell where it is.
pub fn find_block(text: &str) -> Result<Option<Block>, BlockError> {
    let s = scan(text);
    match s.problems.into_iter().next() {
        Some(e) => Err(e),
        None => Ok(s.blocks.first().copied()),
    }
}

/// What the block says, between its markers and including them, with no
/// line ending after the end marker. `version` is the `war` that writes it.
#[must_use]
pub fn block_text(version: &str) -> String {
    [
        BLOCK_BEGIN,
        "<!-- Written by `war admin agents-md --block`, which rewrites the lines between these markers. -->",
        "Ordinary coding needs no Warrant and no ticket: work here as in any repository.",
        "OpenWarrant tracks optional plans and checklists in this repository; run",
        "`war view prime` to see what is tracked (open work, who holds what, recent notes).",
        &format!("{STAMP_PREFIX}{version} -->"),
        BLOCK_END,
    ]
    .join("\n")
}

/// What [`upsert`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// The file had no block; one was appended.
    Inserted,
    /// The block was rewritten.
    Updated,
    /// The block already said exactly this.
    Unchanged,
}

/// `text` with its block set to [`block_text`]`(version)`. A file without a
/// block gets one appended after its last line; a file with one has the
/// bytes between its markers replaced. Bytes outside the markers are not touched:
/// the original is a prefix of an insert's result, and the bytes before the
/// begin marker and after the end marker are those of an update's.
pub fn upsert(text: &str, version: &str) -> Result<(String, Change), BlockError> {
    let s = scan(text);
    if let Some(e) = s.problems.into_iter().next() {
        return Err(e);
    }
    let crlf = text.contains("\r\n");
    let mut block = block_text(version);
    if crlf {
        block = block.replace('\n', "\r\n");
    }
    let nl = if crlf { "\r\n" } else { "\n" };
    if let Some(b) = s.blocks.first() {
        if text[b.start..b.end] == block {
            return Ok((text.to_owned(), Change::Unchanged));
        }
        let mut out = String::with_capacity(text.len() + block.len());
        out.push_str(&text[..b.start]);
        out.push_str(&block);
        out.push_str(&text[b.end..]);
        return Ok((out, Change::Updated));
    }
    if let Some(line) = s.open_fence {
        return Err(BlockError::FenceUnclosed { line });
    }
    // Appended on the line after the last one, with no blank line between:
    // the last section's span then ends exactly where the file did, so
    // adding the block moves no section's revision. A file whose last line
    // has no line ending gets one, which is the one byte added before it.
    let sep = if text.is_empty() || text.ends_with('\n') {
        ""
    } else {
        nl
    };
    Ok((format!("{text}{sep}{block}{nl}"), Change::Inserted))
}

/// The version a block's stamp names.
#[must_use]
pub fn block_stamp(text: &str, block: &Block) -> Option<String> {
    text[block.start..block.end].lines().find_map(|l| {
        let v = l
            .trim()
            .strip_prefix(STAMP_PREFIX)?
            .strip_suffix("-->")?
            .trim();
        (!v.is_empty()).then(|| v.to_owned())
    })
}

/// The section ids `text` names: each `md:<path>#<slug>` that starts at a
/// word boundary (not after a letter, digit, `.`, `/`, `-` or `_`), with
/// surrounding punctuation trimmed, and any `@sha256:` pin dropped.
#[must_use]
pub fn citations(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(i) = text[from..].find(ID_PREFIX) {
        let at = from + i;
        from = at + ID_PREFIX.len();
        let boundary = at == 0 || {
            let c = bytes[at - 1];
            !(c.is_ascii_alphanumeric() || matches!(c, b'.' | b'/' | b'-' | b'_'))
        };
        if !boundary {
            continue;
        }
        let end = text[at..]
            .find(|c: char| {
                c.is_whitespace()
                    || matches!(
                        c,
                        '`' | '(' | ')' | '[' | ']' | '<' | '>' | '"' | '\'' | ',' | ';' | '*'
                    )
            })
            .map_or(text.len(), |j| at + j);
        let token = text[at..end].trim_end_matches(['.', ':', '!', '?']);
        let id = token.split_once('@').map_or(token, |(id, _)| id);
        if is_section_id(id) && !out.iter().any(|x| x == id) {
            out.push(id.to_owned());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "# Project\n\nIntro.\n\n## Testing\nRun `cargo test`.\n\n### Detail\nmore\n\n```text\n## Not a heading\n```\n\n## Style & Lint (strict)\nfmt\n\n## Testing\nagain\n";

    #[test]
    fn each_h2_is_a_section_with_an_exact_span() {
        let s = scan(FILE);
        let slugs: Vec<&str> = s.sections.iter().map(|x| x.slug.as_str()).collect();
        assert_eq!(slugs, ["testing", "style--lint-strict", "testing-1"]);
        assert!(FILE[s.sections[0].start..].starts_with("## Testing\n"));
        assert_eq!(s.sections[0].end, s.sections[1].start);
        assert_eq!(s.sections[2].end, FILE.len());
        assert_eq!(s.sections[0].line, 5);
        assert!(s.problems.is_empty() && s.blocks.is_empty());
        // The fenced heading is text inside "testing".
        assert!(FILE[s.sections[0].start..s.sections[0].end].contains("## Not a heading"));
    }

    #[test]
    fn one_edit_moves_one_revision_and_the_block_moves_none() {
        let a = scan(FILE);
        let b = scan(&FILE.replace("fmt\n", "fmt and clippy\n"));
        assert_eq!(a.sections[0].revision, b.sections[0].revision);
        assert_ne!(a.sections[1].revision, b.sections[1].revision);
        assert_eq!(a.sections[2].revision, b.sections[2].revision);
        // Adding the block, and restamping it, moves no section's revision.
        let (with, change) = upsert(FILE, "1.0.0").unwrap();
        assert_eq!(change, Change::Inserted);
        let c = scan(&with);
        assert_eq!(a.sections, c.sections);
        let (newer, change) = upsert(&with, "2.0.0").unwrap();
        assert_eq!(change, Change::Updated);
        let d = scan(&newer);
        assert_eq!(c.sections, d.sections);
        assert_eq!(d.blocks.len(), 1);
    }

    #[test]
    fn upsert_is_idempotent_and_keeps_every_byte_outside_the_markers() {
        for text in [
            "",
            "no newline at end",
            "one\n",
            "blank after\n\n",
            "a\r\nb\r\n",
        ] {
            let (once, _) = upsert(text, "1.0.0").unwrap();
            assert!(once.starts_with(text), "{text:?}");
            let (twice, change) = upsert(&once, "1.0.0").unwrap();
            assert_eq!(change, Change::Unchanged);
            assert_eq!(once, twice);
            assert!(find_block(&once).unwrap().is_some());
        }
        let text = format!("top\n\n{}\n\ntail\n", block_text("0.1.0"));
        let (out, change) = upsert(&text, "1.0.0").unwrap();
        assert_eq!(change, Change::Updated);
        assert!(out.starts_with("top\n\n<!-- openwarrant:begin -->"));
        assert!(out.ends_with("<!-- openwarrant:end -->\n\ntail\n"));
        let b = find_block(&out).unwrap().unwrap();
        assert_eq!(block_stamp(&out, &b).as_deref(), Some("1.0.0"));
    }

    #[test]
    fn a_writer_refuses_two_blocks_an_unterminated_one_and_a_stray_end() {
        let one = block_text("1.0.0");
        let two = format!("{one}\n\nx\n\n{one}\n");
        assert!(matches!(
            upsert(&two, "1.0.0"),
            Err(BlockError::Duplicate {
                first: 1,
                second: 11
            })
        ));
        let open = format!("a\n{BLOCK_BEGIN}\nb\n");
        assert_eq!(
            upsert(&open, "1.0.0"),
            Err(BlockError::Unterminated { line: 2 })
        );
        let stray = format!("a\n{BLOCK_END}\n");
        assert_eq!(
            upsert(&stray, "1.0.0"),
            Err(BlockError::Unopened { line: 2 })
        );
        let fence = "a\n```\nopen\n";
        assert_eq!(
            upsert(fence, "1.0.0"),
            Err(BlockError::FenceUnclosed { line: 2 })
        );
        // Markers inside a fence are text, so this file has no block.
        let fenced = format!("```\n{BLOCK_BEGIN}\n```\n");
        assert_eq!(find_block(&fenced), Ok(None));
        // The reader is lenient: an unterminated block holds no section, and
        // the sections before it are read.
        let s = scan(&format!("## A\nx\n{BLOCK_BEGIN}\n## B\n"));
        assert_eq!(s.sections.len(), 1);
        assert_eq!(s.problems, [BlockError::Unterminated { line: 3 }]);
    }

    #[test]
    fn slugs_follow_github() {
        assert_eq!(slug("Testing"), "testing");
        assert_eq!(
            slug("When a Warrant's type requires sign-off"),
            "when-a-warrants-type-requires-sign-off"
        );
        assert_eq!(slug("`war prime` & friends"), "war-prime--friends");
        assert_eq!(slug("See [the docs](https://x.y/z)"), "see-the-docs");
        assert_eq!(slug("snake_case"), "snake_case");
        let s = scan("## !!!\n\n## Build ##\n");
        assert_eq!(s.sections[0].slug, "section");
        assert_eq!(s.sections[1].slug, "build");
        assert_eq!(s.sections[1].heading, "Build");
    }

    #[test]
    fn section_ids_and_citations() {
        for ok in [
            "md:CLAUDE.md#testing",
            "md:crates/x/CLAUDE.md#build-1",
            "md:AGENTS.md#é",
        ] {
            assert!(is_section_id(ok), "{ok}");
        }
        for bad in [
            "md:CLAUDE.md",
            "md:#x",
            "md:CLAUDE.md#",
            "md:../x.md#a",
            "md:a b.md#c",
            "CLAUDE.md#x",
            "md:a//b#c",
        ] {
            assert!(!is_section_id(bad), "{bad}");
        }
        let text = "Follow md:CLAUDE.md#testing. Also (md:AGENTS.md#tools), `md:CLAUDE.md#testing`,\nnot README.md:12 nor xmd:a#b, pinned md:CLAUDE.md#style@sha256:00.";
        assert_eq!(
            citations(text),
            [
                "md:CLAUDE.md#testing",
                "md:AGENTS.md#tools",
                "md:CLAUDE.md#style"
            ]
        );
    }
}
