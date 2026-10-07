// SPDX-License-Identifier: Apache-2.0
//! Record atoms, `oh.war/records/v1` (OW-WAR-0148 M3; OW-ADR-0031).
//!
//! A record atom is a Markdown file holding any number of records. Its
//! frontmatter names the schema and the profile that governs its records:
//!
//! ```markdown
//! ---
//! schema: oh.war/records/v1
//! profile: delivery
//! ---
//! # Password reset
//!
//! ## OUT-pr1 · outcome
//! A user who forgot their password regains access without support.
//!
//! ## REQ-pr1 · requirement
//! implements OUT-pr1
//!
//! A reset token expires 15 minutes after issue.
//! ```
//!
//! # Records
//!
//! A record opens at an unindented `## <ID> · <type>` line outside a fenced
//! code block (the separator is U+00B7 MIDDLE DOT between single spaces).
//! `<ID>` is a record id ([`crate::relation::is_record_id`]: `REQ-pr1`,
//! `OUT-001`), unique across the program; `<type>` a lowercase word the
//! governing profile declares. A `## ` line that carries a `·` and is not
//! that shape is refused rather than read as prose: a record that silently
//! failed to open would leave its text inside the record above it.
//!
//! # Byte spans and revisions
//!
//! RC.2's unit-span rules (`docs/sas/drafts/1.0.0-rc.2/format-contract.md`
//! F2) are the reference, with the heading as the marker:
//!
//! - a record's span is `[start, end)` in zero-based UTF-8 byte offsets,
//!   from the first byte of its heading line to the first byte of the next
//!   record heading, or end of file;
//! - it includes the heading, nested headings, blank lines and original
//!   line endings; nothing is normalized;
//! - its revision is `sha256:` over exactly those bytes, so editing one
//!   record moves that record's revision and no other.
//!
//! Text after the frontmatter and before the first record (a title, a
//! paragraph) belongs to no record and moves no revision. RC.2 requires it
//! to be whitespace; a record atom allows a title and prose there, because
//! no selection ever includes it.
//!
//! Fences, for recognition only (as RC.2): an unindented line of at least
//! three identical backticks or tildes opens one; the same character, at
//! least as many, and only trailing spaces or tabs closes it. A backtick
//! fence's info string may not contain a backtick (such a line opens
//! nothing). Inside a fence a heading or relation line is ordinary text. An
//! unclosed fence is refused.
//!
//! # Relation lines
//!
//! Inside a record, outside a fence, an unindented line of exactly
//! `<kind> <target>[, <target>…]` is a relation of that record:
//!
//! ```text
//! implements OUT-pr1
//! selected_over OPT-pr1, OPT-pr2
//! x.mentions OW-ADR-0031
//! evaluates REQ-pr1@sha256:<64 hex>
//! ```
//!
//! `<kind>` is a lowercase word or a namespaced `<ns>.<name>`; each
//! `<target>` begins with an uppercase letter or `t-` and is a record id, a
//! Warrant-scoped id (`OW-WAR-0148/OBL-001`) or a ticket or item
//! (`t-3f2a/i-9c01`), or begins with `md:` and holds a `#` and is a section
//! of an instruction file (`md:CLAUDE.md#testing`, [`crate::instruction`]),
//! optionally pinning a revision (`@sha256:<hex>`). A
//! line that has the shape but whose target does not parse is refused; a
//! line without the shape ("Tokens are single-use.") is prose. Whether the
//! kind is allowed is the profile's to say, and whether the target exists
//! is the corpus's ([`crate::relation`]).
//!
//! Pure: a parse over text (§79.1).

use sha2::{Digest as _, Sha256};

use crate::relation::{Target, is_record_id, parse_target};

/// The schema a record atom's frontmatter declares.
pub const RECORDS_SCHEMA: &str = "oh.war/records/v1";

/// The heading separator: ` · ` (U+00B7 between single spaces).
pub const SEPARATOR: &str = " \u{b7} ";

/// One parsed record atom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordAtom {
    /// The profile that governs every record in it.
    pub profile: String,
    pub records: Vec<AuthoredRecord>,
}

/// One record as authored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredRecord {
    pub id: String,
    /// The type, as written: a profile noun.
    pub record_type: String,
    /// 1-based line of its heading.
    pub line: usize,
    /// `[start, end)` byte offsets of its span in the file.
    pub start: usize,
    pub end: usize,
    /// `sha256:<hex>` of the span's bytes.
    pub revision: String,
    pub relations: Vec<AuthoredRelation>,
}

/// One relation line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredRelation {
    /// The kind as written; [`crate::relation::RelationKind::parse`] places it.
    pub kind: String,
    pub target: Target,
    /// 1-based line.
    pub line: usize,
}

/// Why a record atom was refused. Every variant names the line it is about
/// (0 for the file as a whole).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RecordError {
    #[error("line {line}: frontmatter: {detail}")]
    Frontmatter { line: usize, detail: String },
    #[error("line 1: schema {found:?}; a record atom declares `schema: {RECORDS_SCHEMA}`")]
    Schema { found: String },
    #[error(
        "line 1: no `profile:` in the frontmatter; a record atom names the profile that \
         governs its records"
    )]
    NoProfile,
    #[error(
        "line {line}: {found:?} looks like a record heading and is not one; a record opens \
         at `## <ID> · <type>`, the ID like `REQ-pr1` and the type a lowercase word"
    )]
    Heading { line: usize, found: String },
    #[error(
        "line {line}: {found:?} is a relation line whose target {target:?} names no record \
         id; a target is `REQ-pr1`, `OW-WAR-0001/OBL-001` or `t-3f2a/i-9c01`, optionally \
         `@sha256:<64 hex>`"
    )]
    Relation {
        line: usize,
        found: String,
        target: String,
    },
    #[error("line {line}: a fence opened here is never closed")]
    UnclosedFence { line: usize },
}

impl RecordError {
    /// The 1-based line it is about.
    #[must_use]
    pub const fn line(&self) -> usize {
        match self {
            Self::Frontmatter { line, .. }
            | Self::Heading { line, .. }
            | Self::Relation { line, .. }
            | Self::UnclosedFence { line } => *line,
            Self::Schema { .. } | Self::NoProfile => 1,
        }
    }
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

fn is_type(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|c| c.is_ascii_lowercase())
        && c.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn is_kind_word(s: &str) -> bool {
    !s.is_empty()
        && s.split('.').all(|w| {
            let mut c = w.chars();
            c.next().is_some_and(|c| c.is_ascii_lowercase())
                && c.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        })
}

/// A fence opener: (character, run length), when `line` opens one.
pub(crate) fn fence_open(line: &str) -> Option<(char, usize)> {
    let c = line.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let n = line.chars().take_while(|x| *x == c).count();
    if n < 3 {
        return None;
    }
    let info = &line[n..];
    (c != '`' || !info.contains('`')).then_some((c, n))
}

/// Whether `line` closes the fence `(c, n)` opened.
pub(crate) fn fence_closes(line: &str, (c, n): (char, usize)) -> bool {
    let run = line.chars().take_while(|x| *x == c).count();
    run >= n
        && line[run * c.len_utf8()..]
            .chars()
            .all(|x| x == ' ' || x == '\t')
}

/// The record heading on `line`, if it is one: `Ok(Some((id, type)))`; a
/// `## ` line with a `·` that is not the shape is `Err`.
fn heading(line: &str) -> Result<Option<(String, String)>, ()> {
    let Some(rest) = line.strip_prefix("## ") else {
        return Ok(None);
    };
    if !rest.contains('\u{b7}') {
        return Ok(None);
    }
    let rest = rest.trim_end_matches([' ', '\t']);
    let Some((id, ty)) = rest.split_once(SEPARATOR) else {
        return Err(());
    };
    if is_record_id(id) && is_type(ty) {
        Ok(Some((id.to_owned(), ty.to_owned())))
    } else {
        Err(())
    }
}

/// The relation on `line`, if it has a relation line's shape: `Ok(None)` for
/// prose, `Err(target)` for the shape with a target that does not parse.
fn relation(line: &str) -> Result<Option<(String, Vec<Target>)>, String> {
    let line = line.trim_end_matches([' ', '\t']);
    let Some((kind, rest)) = line.split_once(' ') else {
        return Ok(None);
    };
    if !is_kind_word(kind) {
        return Ok(None);
    }
    let tokens: Vec<&str> = rest.split(", ").collect();
    let shaped = tokens.iter().all(|t| {
        !t.is_empty()
            && !t.contains(char::is_whitespace)
            && (t.starts_with(|c: char| c.is_ascii_uppercase())
                || t.starts_with("t-")
                || (t.starts_with(crate::instruction::ID_PREFIX) && t.contains('#')))
    });
    if !shaped {
        return Ok(None);
    }
    let mut targets = Vec::new();
    for t in tokens {
        targets.push(parse_target(t).ok_or_else(|| t.to_owned())?);
    }
    Ok(Some((kind.to_owned(), targets)))
}

/// Parse one record atom.
pub fn parse(text: &str) -> Result<RecordAtom, RecordError> {
    // Frontmatter: `---`, keys, `---`. The body starts after the closing line.
    let mut lines = text.split_inclusive('\n');
    let first = lines.next().unwrap_or_default();
    if first.trim_end() != "---" {
        return Err(RecordError::Frontmatter {
            line: 1,
            detail: format!("a record atom begins with `---` and `schema: {RECORDS_SCHEMA}`"),
        });
    }
    let mut offset = first.len();
    let mut line_no = 1usize;
    let mut closed = false;
    for l in lines {
        line_no += 1;
        offset += l.len();
        if l.trim_end() == "---" {
            closed = true;
            break;
        }
    }
    if !closed {
        return Err(RecordError::Frontmatter {
            line: 1,
            detail: "the frontmatter is never closed with `---`".to_owned(),
        });
    }
    let front =
        crate::frontmatter::parse(&text[..offset]).map_err(|e| RecordError::Frontmatter {
            line: 1,
            detail: e.to_string(),
        })?;
    let schema = front.scalar("schema").unwrap_or_default();
    if schema != RECORDS_SCHEMA {
        return Err(RecordError::Schema {
            found: schema.to_owned(),
        });
    }
    let profile = front
        .scalar("profile")
        .filter(|p| !p.trim().is_empty())
        .ok_or(RecordError::NoProfile)?
        .trim()
        .to_owned();

    let mut records: Vec<AuthoredRecord> = Vec::new();
    let mut fence: Option<((char, usize), usize)> = None;
    let mut pos = offset;
    for raw in text[offset..].split_inclusive('\n') {
        line_no += 1;
        let start = pos;
        pos += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        if let Some((open, _)) = fence {
            if fence_closes(line, open) {
                fence = None;
            }
            continue;
        }
        if let Some(open) = fence_open(line) {
            fence = Some((open, line_no));
            continue;
        }
        match heading(line) {
            Err(()) => {
                return Err(RecordError::Heading {
                    line: line_no,
                    found: line.to_owned(),
                });
            }
            Ok(Some((id, record_type))) => {
                if let Some(last) = records.last_mut() {
                    last.end = start;
                }
                records.push(AuthoredRecord {
                    id,
                    record_type,
                    line: line_no,
                    start,
                    end: text.len(),
                    revision: String::new(),
                    relations: Vec::new(),
                });
                continue;
            }
            Ok(None) => {}
        }
        let Some(current) = records.last_mut() else {
            continue;
        };
        match relation(line) {
            Ok(None) => {}
            Ok(Some((kind, targets))) => {
                for target in targets {
                    current.relations.push(AuthoredRelation {
                        kind: kind.clone(),
                        target,
                        line: line_no,
                    });
                }
            }
            Err(target) => {
                return Err(RecordError::Relation {
                    line: line_no,
                    found: line.to_owned(),
                    target,
                });
            }
        }
    }
    if let Some((_, line)) = fence {
        return Err(RecordError::UnclosedFence { line });
    }
    for r in &mut records {
        r.revision = digest(&text.as_bytes()[r.start..r.end]);
    }
    Ok(RecordAtom { profile, records })
}

/// One obligation's `evaluates`: (obligation id, target, 1-based line).
pub type Evaluates = (String, Target, usize);

/// The `evaluates` relations an assurance atom's obligations declare, as
/// `(obligation id, target, 1-based line)`: a `- **evaluates:** REQ-pr1,
/// REQ-pr2@sha256:<hex>` bullet under a `### OBL-…` heading. The obligation
/// parser reads such a bullet as prose and is unchanged by it; this reads
/// the relation. `Err((line, token))` for a target that does not parse.
pub fn obligation_evaluates(text: &str) -> Result<Vec<Evaluates>, (usize, String)> {
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    for (n, line) in text.lines().enumerate() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("### ") {
            let id = rest
                .split_once('\u{2014}')
                .or_else(|| rest.split_once(" - "))
                .map_or(rest, |(id, _)| id)
                .trim();
            current = id.starts_with("OBL-").then(|| id.to_owned());
            continue;
        }
        if t.starts_with("## ") {
            current = None;
            continue;
        }
        let Some(obl) = &current else { continue };
        let Some(value) = t
            .strip_prefix("- **")
            .and_then(|r| r.split_once(":**"))
            .filter(|(k, _)| k.trim().eq_ignore_ascii_case("evaluates"))
            .map(|(_, v)| v.trim())
        else {
            continue;
        };
        for token in value.split(',').map(|s| s.trim().trim_matches('`')) {
            if token.is_empty() {
                continue;
            }
            let target = parse_target(token).ok_or_else(|| (n + 1, token.to_owned()))?;
            out.push((obl.clone(), target, n + 1));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ATOM: &str = "---\nschema: oh.war/records/v1\nprofile: delivery\n---\n# Password reset\n\n## OUT-pr1 · outcome\nA user regains access.\n\n## REQ-pr1 · requirement\nimplements OUT-pr1\n\nA token expires 15 minutes after issue.\n\n```text\n## NOT-1 · requirement\nimplements NOPE-1\n```\n\n## DEC-pr1 · decision\nselected_over OPT-pr1, OPT-pr2\nconstrains REQ-pr1\nx.mentions OW-ADR-0031\nTokens are signed.\n";

    #[test]
    fn records_open_at_headings_and_carry_their_relations() {
        let a = parse(ATOM).unwrap();
        assert_eq!(a.profile, "delivery");
        let ids: Vec<&str> = a.records.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            ["OUT-pr1", "REQ-pr1", "DEC-pr1"],
            "the fenced heading is text"
        );
        let req = &a.records[1];
        assert_eq!(req.record_type, "requirement");
        assert_eq!(req.relations.len(), 1, "the fenced relation line is text");
        assert_eq!(req.relations[0].kind, "implements");
        assert_eq!(req.relations[0].target.id, "OUT-pr1");
        let dec = &a.records[2];
        let rel: Vec<(&str, &str)> = dec
            .relations
            .iter()
            .map(|r| (r.kind.as_str(), r.target.id.as_str()))
            .collect();
        assert_eq!(
            rel,
            [
                ("selected_over", "OPT-pr1"),
                ("selected_over", "OPT-pr2"),
                ("constrains", "REQ-pr1"),
                ("x.mentions", "OW-ADR-0031")
            ]
        );
    }

    #[test]
    fn spans_are_exact_and_contiguous_and_one_edit_moves_one_revision() {
        let a = parse(ATOM).unwrap();
        assert!(ATOM[a.records[0].start..].starts_with("## OUT-pr1"));
        assert_eq!(a.records[0].end, a.records[1].start);
        assert_eq!(a.records[1].end, a.records[2].start);
        assert_eq!(a.records[2].end, ATOM.len());
        let edited = ATOM.replace("15 minutes", "10 minutes");
        let b = parse(&edited).unwrap();
        assert_eq!(a.records[0].revision, b.records[0].revision);
        assert_ne!(a.records[1].revision, b.records[1].revision);
        assert_eq!(a.records[2].revision, b.records[2].revision);
        // The prelude is no record's.
        let retitled = ATOM.replace("# Password reset", "# Password reset, v2");
        let c = parse(&retitled).unwrap();
        for (x, y) in a.records.iter().zip(&c.records) {
            assert_eq!(x.revision, y.revision);
        }
        // CRLF is kept, not normalized.
        let crlf = ATOM.replace('\n', "\r\n");
        let d = parse(&crlf).unwrap();
        assert_ne!(a.records[0].revision, d.records[0].revision);
        assert_eq!(d.records.len(), 3);
    }

    #[test]
    fn malformed_atoms_are_refused_by_line() {
        let bad = ATOM.replace("## DEC-pr1 · decision", "## dec-pr1 · decision");
        assert!(matches!(
            parse(&bad),
            Err(RecordError::Heading { line: 20, .. })
        ));
        let bad = ATOM.replace("## DEC-pr1 · decision", "## DEC-pr1 ·decision");
        assert!(matches!(parse(&bad), Err(RecordError::Heading { .. })));
        let bad = ATOM.replace("constrains REQ-pr1", "constrains REQ-pr1@sha256:00");
        assert!(matches!(parse(&bad), Err(RecordError::Relation { .. })));
        let bad = ATOM.replace("```\n\n## DEC", "\n\n## DEC");
        assert!(matches!(
            parse(&bad),
            Err(RecordError::UnclosedFence { .. })
        ));
        let bad = ATOM.replace("schema: oh.war/records/v1", "schema: oh.war/records/v9");
        assert!(matches!(parse(&bad), Err(RecordError::Schema { .. })));
        let bad = ATOM.replace("profile: delivery\n", "");
        assert_eq!(parse(&bad), Err(RecordError::NoProfile));
    }

    #[test]
    fn prose_is_not_a_relation() {
        for prose in [
            "Tokens are single-use.",
            "constrains everything",
            "implements the reset flow",
            "see OW-ADR-0031 for why",
            "see md:CLAUDE.md for the rules",
        ] {
            assert_eq!(relation(prose), Ok(None), "{prose}");
        }
        // A section of an instruction file is a target; one that does not
        // parse, in the shape, is refused rather than read as prose.
        let (kind, targets) = relation("constrains md:CLAUDE.md#testing")
            .unwrap()
            .unwrap();
        assert_eq!(kind, "constrains");
        assert_eq!(targets[0].id, "md:CLAUDE.md#testing");
        assert_eq!(
            relation("constrains md:../CLAUDE.md#testing"),
            Err("md:../CLAUDE.md#testing".to_owned())
        );
    }

    #[test]
    fn obligations_declare_what_they_evaluate() {
        let pin = format!("sha256:{}", "b".repeat(64));
        let text = format!(
            "## Acceptance Obligations\n\n### OBL-001 — a token at minute 16 is refused\n- **scope:** the endpoint.\n- **evaluates:** REQ-pr1@{pin}, CON-pr1\n- **evidence:** a plant.\n\n### OBL-002 — other\n- **scope:** x\n- **evidence:** y\n"
        );
        let e = obligation_evaluates(&text).unwrap();
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].0, "OBL-001");
        assert_eq!(e[0].1.pin.as_deref(), Some(pin.as_str()));
        assert_eq!(e[1].1.id, "CON-pr1");
        let bad = text.replace("CON-pr1", "con");
        assert_eq!(obligation_evaluates(&bad), Err((5, "con".to_owned())));
    }
}
