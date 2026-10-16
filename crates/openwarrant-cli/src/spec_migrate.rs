// SPDX-License-Identifier: Apache-2.0
//! `war plan spec migrate` — a legacy SAS (`docs/sas/`) written as a Format 1.0
//! specification (FORMAT-1.0 §4.5, §14 "Migrating a specification").
//!
//! # What is written
//!
//! - `SPEC.md`: profile `spec`, the SAS's title, a fresh UUIDv7, `version`
//!   (the SAS revision its bytes are, else the latest on record, else
//!   `--version`), and Purpose and Scope from the SAS's opening sections
//!   (the numbered sections titled so, else the first two).
//! - one `spec/<area>.md` per top-level SAS section, `part_of: doc:SPEC.md`:
//!   a `# Part` or `# Appendix` and the numbered sections under it, or each
//!   numbered section when the SAS has no parts. Each numbered section is a
//!   section of its part, titled as the SAS titles it (`106. Requirement
//!   index`).
//! - each §106 requirement row a `requirement` in the part that holds the
//!   row: id `REQ-` and the row's number with `.` replaced by `-`
//!   (`WAR-SAS-101.1` → `REQ-101-1`; `WAR-SAS-RQ-001` → `REQ-001`), its
//!   text as `statement`, the old id as `legacy.id`.
//!
//! The SAS's text is carried as it is, with one change a reader needs: a
//! line outside a fence that would open a Format 1.0 unit (`# ` or `## `
//! at column zero, not one this writer emits) is written one level deeper
//! (`### `), as text. The SAS files are not touched and keep their
//! acceptance; the new specification is accepted by nobody until a person
//! accepts it.
//!
//! # Dry run by default
//!
//! Without `--write` nothing is written: the report names each file, its
//! size, and what war's own reader finds in it. With `--write` the files
//! are written only when none exists yet and the reader refuses none.

use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use openwarrant_core::format::Outcome;
use openwarrant_core::sas_sections::{self, Section, SectionKind};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

/// What the migration would write.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    /// Repository-relative path → bytes, in write order.
    pub files: Vec<(String, String)>,
    /// `legacy.id` → new id.
    pub requirements: BTreeMap<String, String>,
}

/// Options for [`run`].
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The SAS document; default the repository's (`[paths] sas`).
    pub sas: Option<Utf8PathBuf>,
    /// `version` for `SPEC.md` when no SAS revision names one.
    pub version: Option<String>,
    /// Write the files. Without it, a dry run.
    pub write: bool,
}

/// `WAR-SAS-101.1` → `REQ-101-1`; `WAR-SAS-RQ-001` → `REQ-001`.
#[must_use]
pub fn requirement_id(legacy: &str) -> String {
    let number = legacy
        .split_once("-SAS-")
        .map_or(legacy, |(_, n)| n)
        .trim_start_matches("RQ-");
    format!("REQ-{}", number.replace('.', "-"))
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-');
    // A file name a person can read: the first few words.
    let mut cut = String::new();
    for w in out.split('-') {
        if cut.len() + w.len() > 48 {
            break;
        }
        if !cut.is_empty() {
            cut.push('-');
        }
        cut.push_str(w);
    }
    cut
}

/// A title within §3.4's 120 bytes, cut at a character boundary.
fn title(s: &str) -> String {
    let s = s.trim();
    if s.len() <= 120 {
        return s.to_owned();
    }
    let mut end = 117;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", s[..end].trim_end())
}

/// A section heading's text: §A.2's `section-name` excludes ` · ` and a
/// `record-id | ` start, so either is written plainly.
fn section_name(s: &str) -> String {
    title(&s.replace(" · ", " - ").replace(" | ", " - "))
}

fn fence_run(line: &str) -> Option<(char, usize, &str)> {
    let indent = line.chars().take_while(|c| *c == ' ').count();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let ch = rest.chars().next()?;
    if ch != '`' && ch != '~' {
        return None;
    }
    let run = rest.chars().take_while(|c| *c == ch).count();
    (run >= 3).then(|| (ch, run, &rest[run..]))
}

/// The text with every `# ` and `## ` line outside a fence written as
/// `### `: carried as text, never read as a unit.
fn demote(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut fence: Option<(char, usize)> = None;
    for line in text.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\n', '\r']);
        if let Some((ch, run)) = fence {
            if let Some((c, r, rest)) = fence_run(bare)
                && c == ch
                && r >= run
                && rest.trim().is_empty()
            {
                fence = None;
            }
            out.push_str(line);
            continue;
        }
        if let Some((c, r, _)) = fence_run(bare) {
            fence = Some((c, r));
            out.push_str(line);
            continue;
        }
        if let Some(rest) = bare.strip_prefix("## ").or_else(|| bare.strip_prefix("# ")) {
            out.push_str("### ");
            out.push_str(rest);
            out.push_str(&line[bare.len()..]);
        } else {
            out.push_str(line);
        }
    }
    out
}

/// A unit's text without its own heading line.
fn body(s: &Section) -> String {
    let text = String::from_utf8_lossy(&s.bytes);
    let rest = text.split_once('\n').map_or("", |(_, r)| r);
    demote(rest.trim_start_matches('\n'))
}

fn ensure_blank_end(s: &mut String) {
    while s.ends_with("\n\n") {
        s.pop();
    }
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s.push('\n');
}

/// One numbered section of a part, and the §106 rows it holds as
/// requirements after it.
fn numbered(s: &Section, out: &mut String, reqs: &mut BTreeMap<String, String>) {
    out.push_str(&format!("## {}. {}\n\n", s.id, section_name(&s.title)));
    out.push_str(&body(s));
    ensure_blank_end(out);
    let rows = openwarrant_core::sas::section_106(&String::from_utf8_lossy(&s.bytes));
    for (legacy, text) in rows {
        let id = requirement_id(&legacy);
        out.push_str(&format!(
            "## {id} · requirement — {}\nstatement: {}\nlegacy.id: {legacy}\n\n",
            title(&text),
            text.trim()
        ));
        reqs.insert(legacy, id);
    }
}

fn frontmatter(pairs: &[(&str, &str)]) -> String {
    let mut s = String::from("---\n");
    for (k, v) in pairs {
        s.push_str(&format!("{k}: {v}\n"));
    }
    s.push_str("---\n");
    s
}

/// A frontmatter scalar on one line.
fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The files a SAS becomes. `uuid` and `version` are given, so the same
/// SAS gives the same bytes.
#[must_use]
pub fn plan(sas: &[u8], source: &str, uuid: &str, version: &str) -> Plan {
    let sections = sas_sections::split(sas);
    let mut out = Plan::default();
    let preamble = sections
        .iter()
        .find(|s| s.kind == SectionKind::Preamble)
        .map(|s| String::from_utf8_lossy(&s.bytes).into_owned())
        .unwrap_or_default();
    let (doc_title, rest) = match preamble.trim_start().strip_prefix("# ") {
        Some(t) => {
            let (first, rest) = t.split_once('\n').unwrap_or((t, ""));
            (first.trim().to_owned(), rest.to_owned())
        }
        None => ("Specification".to_owned(), preamble.clone()),
    };
    let numbered_all: Vec<&Section> = sections
        .iter()
        .filter(|s| s.kind == SectionKind::Numbered)
        .collect();
    let by_title = |t: &str| {
        numbered_all
            .iter()
            .find(|s| s.title.eq_ignore_ascii_case(t))
            .copied()
    };
    let purpose = by_title("Purpose").or_else(|| numbered_all.first().copied());
    let scope = by_title("Scope").or_else(|| numbered_all.get(1).copied());

    // SPEC.md.
    let summary = one_line(&format!("{doc_title}, migrated from {source}."));
    let mut spec = frontmatter(&[
        ("format", "openwarrant/1.0"),
        ("profile", "spec"),
        ("uuid", uuid),
        ("version", version),
        ("summary", &summary),
    ]);
    spec.push_str(&format!("# {}\n\n", title(&doc_title)));
    let pre = demote(rest.trim_start_matches('\n'));
    if !pre.trim().is_empty() {
        spec.push_str(&pre);
        ensure_blank_end(&mut spec);
    }
    for (name, s) in [("Purpose", purpose), ("Scope", scope)] {
        spec.push_str(&format!("## {name}\n\n"));
        match s {
            Some(s) => spec.push_str(&body(s)),
            None => spec.push_str(&format!("The SAS ({source}) names no {name} section.\n")),
        }
        ensure_blank_end(&mut spec);
    }
    while spec.ends_with("\n\n") {
        spec.pop();
    }
    out.files.push(("SPEC.md".to_owned(), spec));

    // The parts: a `# Part` or `# Appendix` and what follows it; with no
    // part, each numbered section alone.
    let has_parts = sections
        .iter()
        .any(|s| matches!(s.kind, SectionKind::Part | SectionKind::Appendix));
    let mut areas: Vec<(String, String, Vec<&Section>)> = Vec::new();
    for s in &sections {
        match s.kind {
            SectionKind::Preamble => {}
            SectionKind::Part | SectionKind::Appendix => {
                areas.push((
                    format!("{}-{}", s.id, slug(&s.title)),
                    s.title.clone(),
                    vec![s],
                ));
            }
            SectionKind::Numbered if has_parts => {
                if let Some(a) = areas.last_mut() {
                    a.2.push(s);
                } else {
                    areas.push((
                        format!("{}-{}", s.id, slug(&s.title)),
                        s.title.clone(),
                        vec![s],
                    ));
                }
            }
            SectionKind::Numbered => {
                areas.push((
                    format!("section-{}-{}", s.id, slug(&s.title)),
                    format!("{}. {}", s.id, s.title),
                    vec![s],
                ));
            }
        }
    }
    for (area, area_title, units) in areas {
        let area = area.trim_end_matches('-').to_owned();
        let summary = one_line(&format!("{area_title}, from {source}."));
        let mut text = frontmatter(&[
            ("format", "openwarrant/1.0"),
            ("profile", "spec"),
            ("part_of", "doc:SPEC.md"),
            ("summary", &summary),
        ]);
        text.push_str(&format!("# {}\n\n", title(&area_title)));
        for s in units {
            if s.kind == SectionKind::Numbered {
                numbered(s, &mut text, &mut out.requirements);
            } else {
                text.push_str(&body(s));
                ensure_blank_end(&mut text);
            }
        }
        while text.ends_with("\n\n") {
            text.pop();
        }
        out.files.push((format!("spec/{area}.md"), text));
    }
    out
}

/// The `version` `SPEC.md` gets: the SAS revision whose digest the bytes
/// are, else the latest on record, else `fallback`.
fn version_of(repo: &Repository, bytes: &[u8], fallback: Option<&str>) -> Option<String> {
    let digest = openwarrant_compiler::sha256_hex(bytes);
    let revisions = repo.load_sas_revisions().unwrap_or_default();
    revisions
        .iter()
        .find(|r| r.sha256 == digest)
        .map(|r| r.version.clone())
        .or_else(|| fallback.map(str::to_owned))
        .or_else(|| repo.latest_sas_revision().ok().flatten().map(|r| r.version))
}

/// `war plan spec migrate`.
pub fn run(repo: &Repository, opts: &Options) -> Result<(Report, Plan), RepoError> {
    let mut report = Report::default();
    let (path, bytes) = match &opts.sas {
        Some(p) => {
            let p = if p.is_absolute() {
                p.clone()
            } else {
                repo.root.join(p)
            };
            let b = std::fs::read(&p).map_err(|source| RepoError::Io {
                context: format!("could not read {p}"),
                source,
            })?;
            (p, b)
        }
        None => repo.sas_document()?,
    };
    let source = repo.relative(&path);
    let Some(version) = version_of(repo, &bytes, opts.version.as_deref()) else {
        report.push(Diagnostic::error(
            "spec.migrate-version",
            source,
            "no SAS revision names this document and none is on record; pass --version <semver>"
                .to_owned(),
        ));
        return Ok((report, Plan::default()));
    };
    let uuid = openwarrant_core::identity::WarUuid::mint().to_string();
    let plan = plan(&bytes, &source, &uuid, &version);
    // Each file read back by war's own reader, as it would be read once
    // written.
    let profiles = crate::format_doc::profiles(repo);
    let mut refused = 0usize;
    for (rel, text) in &plan.files {
        let doc = openwarrant_core::format::parse_with(text.as_bytes(), &profiles, rel);
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for f in &doc.findings {
            *counts.entry(f.rule).or_default() += 1;
            if f.outcome == Outcome::Refusal {
                refused += 1;
                report.push(Diagnostic::error(
                    f.rule,
                    format!("{rel}:{}", f.line),
                    f.message.clone(),
                ));
            }
        }
        let found = if counts.is_empty() {
            "nothing to report".to_owned()
        } else {
            counts
                .iter()
                .map(|(r, n)| format!("{r} ×{n}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        report.push(Diagnostic::pass(
            "spec.migrate-file",
            format!(
                "{rel}: {} bytes, {} unit(s); the reader finds {found}",
                text.len(),
                doc.units.len()
            ),
        ));
    }
    report.push(Diagnostic::pass(
        "spec.migrate-plan",
        format!(
            "{source} → SPEC.md and {} part(s), {} requirement(s) with `legacy.id`, version {version}",
            plan.files.len().saturating_sub(1),
            plan.requirements.len()
        ),
    ));
    if !opts.write {
        report.note(
            "Dry run: nothing written. `--write` writes these files; the SAS stays where it is and \
             keeps its acceptance, and the new specification is accepted by nobody until a person \
             accepts it (`war plan spec accept`, then `war sign spec`)."
                .to_owned(),
        );
        return Ok((report, plan));
    }
    if refused > 0 {
        report.push(Diagnostic::error(
            "spec.migrate-refused",
            source,
            format!("{refused} refusal(s) in the files above; nothing written"),
        ));
        return Ok((report, plan));
    }
    let existing: Vec<&str> = plan
        .files
        .iter()
        .map(|(rel, _)| rel.as_str())
        .filter(|rel| repo.root.join(rel).exists())
        .collect();
    if !existing.is_empty() {
        report.push(Diagnostic::error(
            "spec.migrate-exists",
            existing.join(", "),
            "already exists; a migration writes a new specification and overwrites nothing"
                .to_owned(),
        ));
        return Ok((report, plan));
    }
    for (rel, text) in &plan.files {
        let p = repo.root.join(rel);
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).map_err(|source| RepoError::Io {
                context: format!("could not create {dir}"),
                source,
            })?;
        }
        std::fs::write(&p, text).map_err(|source| RepoError::Io {
            context: format!("could not write {p}"),
            source,
        })?;
    }
    report.push(Diagnostic::pass(
        "spec.migrated",
        format!("wrote {} file(s)", plan.files.len()),
    ));
    Ok((report, plan))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwarrant_core::format_profile::Profiles;

    const SAS: &str = "# Demo Software Architecture Specification\n\n## A subtitle\n\n\
| Field | Value |\n|---|---|\n| Status | draft |\n\n\
# Part I — Foundations\n\nPart prose.\n\n## 1. Purpose\n\nWhy it exists.\n\n\
## 2. Scope\n\nWhat it covers.\n\n### 2.1 Out of scope\n\nNot this.\n\n\
# Part II — Index\n\n## 3. Requirement index\n\n\
| ID | Requirement |\n|---|---|\n| DEMO-SAS-RQ-001 | Every record has an id |\n\
| DEMO-SAS-RQ-002 | An edit lapses only its record |\n\n\
```markdown\n## not a unit\n```\n\n## Not numbered\n\ntext\n";

    fn parsed(text: &str, path: &str) -> openwarrant_core::format::Document {
        openwarrant_core::format::parse_with(
            text.as_bytes(),
            &Profiles::load(std::iter::empty::<(&str, &[u8])>()),
            path,
        )
    }

    #[test]
    fn ids_follow_section_14() {
        assert_eq!(requirement_id("WAR-SAS-101.1"), "REQ-101-1");
        assert_eq!(requirement_id("WAR-SAS-RQ-001"), "REQ-001");
    }

    #[test]
    fn a_sas_becomes_a_specification_the_reader_admits() {
        let p = plan(
            SAS.as_bytes(),
            "docs/sas/demo.md",
            "01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e01",
            "1.0.0",
        );
        let names: Vec<&str> = p.files.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            [
                "SPEC.md",
                "spec/part-I-foundations.md",
                "spec/part-II-index.md"
            ]
        );
        for (name, text) in &p.files {
            let d = parsed(text, name);
            let bad: Vec<_> = d
                .findings
                .iter()
                .filter(|f| f.outcome == Outcome::Refusal || f.outcome == Outcome::Unknown)
                .collect();
            assert!(bad.is_empty(), "{name}: {bad:?}\n{text}");
        }
        let spec = parsed(&p.files[0].1, "SPEC.md");
        let ids: Vec<&str> = spec.units.iter().map(|u| u.id.as_str()).collect();
        assert_eq!(ids, ["S-purpose", "S-scope"]);
        let index = parsed(&p.files[2].1, "spec/part-II-index.md");
        let reqs: Vec<_> = index
            .units
            .iter()
            .filter(|u| u.record_type == "requirement")
            .collect();
        assert_eq!(reqs.len(), 2);
        assert_eq!(reqs[0].id, "REQ-001");
        assert_eq!(
            reqs[0].fields.get("statement").and_then(|v| v.as_str()),
            Some("Every record has an id")
        );
        assert_eq!(
            reqs[0].ext.get("legacy.id").and_then(|v| v.as_str()),
            Some("DEMO-SAS-RQ-001")
        );
        assert!(index.relations.iter().any(|r| r.kind == "part_of"));
        // A `## ` line outside a fence is text; inside one it is untouched.
        let text = &p.files[2].1;
        assert!(text.contains("### Not numbered"));
        assert!(text.contains("```markdown\n## not a unit\n```"));
        assert_eq!(p.requirements.len(), 2);
    }

    #[test]
    fn the_same_sas_gives_the_same_bytes() {
        let a = plan(SAS.as_bytes(), "s.md", "u", "1.0.0");
        let b = plan(SAS.as_bytes(), "s.md", "u", "1.0.0");
        assert_eq!(a, b);
    }
}
