// SPDX-License-Identifier: Apache-2.0
//! Format 1.0 documents in a repository (`docs/format/FORMAT-1.0.md`,
//! OW-ADR-0032): where they live, and what `war check` says of them.
//!
//! # Where
//!
//! - A Warrant is one file (§4.1): `<warrants>/<alias>.md` or
//!   `<warrants>/<alias>/warrant.md`; a ticket `<tickets>/<alias>.md` or
//!   `<tickets>/<alias>/ticket.md`. Both forms of one alias are refused
//!   (`format.document-duplicate`). `war new --format 1.0` writes the
//!   first form. A directory holding `warrant.md` (or `ticket.md`) is no
//!   legacy Warrant (or ticket), whatever else it holds.
//! - A record document is a `docs/records/<area>/*.md` whose frontmatter
//!   declares `format:`; the `oh.war/records/v1` reader skips it.
//! - The specification (§4.5) is `SPEC.md` at the root and its parts,
//!   `spec/<area>.md` or any indexed document of profile `spec` (see
//!   [`spec_files`]), each when its frontmatter declares `format:`. Its
//!   records are global; its own record is `doc:<path>`, type `spec`. A
//!   file of profile `spec` that is not `SPEC.md` and names no `part_of` is
//!   a second root and is refused (`format.document-duplicate`).
//!
//! # Profiles
//!
//! Each document is read under its profile (§7.4): a built-in of §7.6, or
//! a Format 1.0 profile file `profiles/<name>.toml`
//! ([`openwarrant_core::format_profile`]). A refused file is reported
//! (`profile.invalid`) and its documents read `format.profile-unknown`.
//! A file without `format = "openwarrant/1.0"` is a legacy profile, read by
//! the legacy registry; it governs no document here.
//!
//! A file at a Warrant or ticket path whose name is an alias and whose
//! frontmatter declares no `format:` is refused (`format.unrecognised`, §6);
//! any other `.md` there is not read.
//!
//! # The optional manifest
//!
//! A `manifest.toml` beside `warrant.md` or `ticket.md` holds tool settings
//! only (§4.2): one that lists atoms or carries a key the frontmatter
//! defines is refused (`format.manifest-restates`).
//!
//! # What is read
//!
//! [`openwarrant_core::format::parse`] reads each document; its findings
//! keep their rule (§11.2) and are reported as `path:line`. A refused
//! document, or one read `UNKNOWN` as a whole (a newer major, a `requires`
//! not understood), adds nothing to the model ([`crate::model`]). Legacy
//! encodings are not read here and nothing about them changes.

use std::collections::BTreeSet;

use camino::{Utf8Path, Utf8PathBuf};

use openwarrant_core::format::{self, Document, Outcome};
use openwarrant_core::format_profile::{self, Lookup, Profiles};

use crate::corpus::Corpus;
use crate::diagnostic::{Diagnostic, Report};
use crate::repo::Repository;

/// What kind of place a document was found in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Warrant,
    Ticket,
    Records,
    /// `SPEC.md` or `spec/<area>.md` (§4.5).
    Spec,
}

/// One document read.
#[derive(Debug, Clone)]
pub struct Loaded {
    /// Repository-relative path.
    pub source: String,
    pub place: Place,
    pub doc: Document,
    /// The exact bytes, for spans.
    pub bytes: Vec<u8>,
}

impl Loaded {
    /// Whether it adds records to the model: neither refused nor `UNKNOWN`
    /// as a whole.
    #[must_use]
    pub fn admitted(&self) -> bool {
        !self.doc.refused() && !self.doc.unknown()
    }
}

/// A finding outside any document's own parse (a duplicate, a manifest).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    pub outcome: Outcome,
    pub rule: &'static str,
    pub file: String,
    pub line: usize,
    pub message: String,
}

/// Every Format 1.0 document of the repository.
#[derive(Debug, Clone, Default)]
pub struct FormatDocs {
    pub docs: Vec<Loaded>,
    pub faults: Vec<Fault>,
    /// The profiles the documents were read under.
    pub profiles: Profiles,
}

impl FormatDocs {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty() && self.faults.is_empty()
    }
}

/// Whether `name` is an alias (§5): a record id or `t-<hex>`.
fn is_alias(name: &str) -> bool {
    openwarrant_core::relation::is_record_id(name) || openwarrant_core::ticket::is_ticket_id(name)
}

/// The ticket store's directory, as `[tickets] dir` names it.
fn tickets_dir(repo: &Repository) -> Utf8PathBuf {
    crate::ticket::Store::open(repo, None)
        .map_or_else(|_| repo.root.join(crate::ticket::DEFAULT_DIR), |s| s.dir)
}

/// The file name a directory work document uses in `place`.
const fn inner_name(place: Place) -> &'static str {
    match place {
        Place::Ticket => "ticket.md",
        _ => "warrant.md",
    }
}

/// Whether `dir` holds a Format 1.0 work document (`warrant.md` or
/// `ticket.md`): such a directory is never read as a legacy one.
#[must_use]
pub fn is_document_dir(dir: &Utf8Path) -> bool {
    crate::vfs::is_file(dir.join("warrant.md")) || crate::vfs::is_file(dir.join("ticket.md"))
}

fn sorted_entries(dir: &Utf8Path) -> Vec<Utf8PathBuf> {
    let mut out: Vec<Utf8PathBuf> = crate::vfs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// The repository's profiles (§7.5): every `profiles/*.toml` that is a
/// Format 1.0 profile, over the built-ins of §7.6.
#[must_use]
pub fn profiles(repo: &Repository) -> Profiles {
    let files: Vec<(String, Vec<u8>)> = sorted_entries(&repo.root.join("profiles"))
        .into_iter()
        .filter(|p| p.extension() == Some("toml") && crate::vfs::is_file(p))
        .filter_map(|p| Some((repo.relative(&p), crate::vfs::read(&p).ok()?)))
        .collect();
    Profiles::load(files.iter().map(|(f, b)| (f.as_str(), b.as_slice())))
}

/// `SPEC.md`, `spec/<area>.md`, and every other part, each when it
/// declares `format:` (§4.5).
///
/// A part SHOULD live at `spec/<area>.md` and MAY live elsewhere: any
/// document the development-document index reaches (`[documents] index`,
/// by default `docs/**/*.md` and the root's `*.md`) whose frontmatter
/// names profile `spec` is one. Record documents (`docs/records/<area>/`)
/// are read in their own place, under their own profile, so they are not
/// listed again here. One that names no `part_of` is a second root and is
/// refused by [`load`], wherever it is.
#[must_use]
pub fn spec_files(repo: &Repository) -> Vec<Utf8PathBuf> {
    let declares = |p: &Utf8Path| {
        crate::vfs::read(p).is_ok_and(|b| format::declares_format(&String::from_utf8_lossy(&b)))
    };
    let root = repo.root.join("SPEC.md");
    let mut out: Vec<Utf8PathBuf> = std::iter::once(root)
        .chain(
            sorted_entries(&repo.root.join("spec"))
                .into_iter()
                .filter(|p| p.extension() == Some("md")),
        )
        .filter(|p| crate::vfs::is_file(p) && declares(p))
        .collect();
    let records: BTreeSet<Utf8PathBuf> = crate::records::files(repo).into_iter().collect();
    let (globs, _) = crate::doc_index::globs(repo);
    for rel in crate::doc_index::walk(repo, &globs) {
        let p = repo.root.join(&rel);
        if !rel.ends_with(".md") || out.contains(&p) || records.contains(&p) {
            continue;
        }
        let Ok(bytes) = crate::vfs::read(&p) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        if format::declares_format(&text) && frontmatter_profile(&text) == Some("spec") {
            out.push(p);
        }
    }
    out
}

/// The `profile:` a document's frontmatter names, read without parsing the
/// document: the first such line between the opening `---` and the next.
fn frontmatter_profile(text: &str) -> Option<&str> {
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    for line in lines {
        let line = line.trim_end();
        if line == "---" {
            return None;
        }
        if let Some(v) = line.strip_prefix("profile:") {
            return Some(v.trim().trim_matches(['"', '\'']));
        }
    }
    None
}

/// Read every Format 1.0 document of `repo`.
#[must_use]
pub fn load(repo: &Repository) -> FormatDocs {
    let mut out = FormatDocs {
        profiles: profiles(repo),
        ..FormatDocs::default()
    };
    for bad in &out.profiles.invalid {
        out.faults.push(Fault {
            outcome: Outcome::Refusal,
            rule: format_profile::INVALID_RULE,
            file: bad.file.clone(),
            line: 1,
            message: format!(
                "{}; a document naming this profile reads `format.profile-unknown`",
                bad.detail
            ),
        });
    }
    // §7.5: a profile file written in 7.5's shape without `format` is a
    // legacy profile and never governs a Format 1.0 document: said once,
    // as a warning, beside its documents' `format.profile-unknown`.
    for p in sorted_entries(&repo.root.join("profiles")) {
        if p.extension() == Some("toml")
            && crate::vfs::read(&p).is_ok_and(|b| format_profile::is_format_shaped_legacy(&b))
        {
            out.faults.push(Fault {
                outcome: Outcome::Warning,
                rule: "profile.format-missing",
                file: repo.relative(&p),
                line: 1,
                message: format!(
                    "{} has a Format 1.0 profile's keys and no `format = \"openwarrant/1.0\"`: \
                     without it the file is a legacy profile (FORMAT-1.0 §7.5) and governs no \
                     Format 1.0 document, so a document naming it reads \
                     `format.profile-unknown`. Add the key if it is meant to govern them",
                    repo.relative(&p)
                ),
            });
        }
    }
    for (place, dir) in [
        (Place::Warrant, repo.warrants_dir()),
        (Place::Ticket, tickets_dir(repo)),
    ] {
        work_documents(repo, place, &dir, &mut out);
    }
    let others = spec_files(repo)
        .into_iter()
        .map(|p| (p, Place::Spec))
        .chain(
            crate::records::files(repo)
                .into_iter()
                .map(|p| (p, Place::Records)),
        );
    for (path, place) in others {
        let Ok(bytes) = crate::vfs::read(&path) else {
            continue;
        };
        if format::declares_format(&String::from_utf8_lossy(&bytes)) {
            let source = repo.relative(&path);
            let doc = format::parse_with(&bytes, &out.profiles, &source);
            out.docs.push(Loaded {
                source,
                place,
                doc,
                bytes,
            });
        }
    }
    // §4.5: one root specification. A file of profile `spec` that is not
    // `SPEC.md` and names no `part_of` is a second one.
    for d in &out.docs {
        if d.doc.profile == "spec"
            && d.source != "SPEC.md"
            && !d.doc.relations.iter().any(|r| r.kind == "part_of")
        {
            out.faults.push(Fault {
                outcome: Outcome::Refusal,
                rule: "format.document-duplicate",
                file: d.source.clone(),
                line: 1,
                message: format!(
                    "{} has profile `spec`, is not SPEC.md and names no `part_of`: a second \
                     root specification. A part says `part_of: doc:SPEC.md`",
                    d.source
                ),
            });
        }
    }
    let second: BTreeSet<String> = out
        .faults
        .iter()
        .filter(|f| f.rule == "format.document-duplicate")
        .map(|f| f.file.clone())
        .collect();
    out.docs.retain(|d| !second.contains(&d.source));
    out
}

fn work_documents(repo: &Repository, place: Place, dir: &Utf8Path, out: &mut FormatDocs) {
    let mut seen: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for path in sorted_entries(dir) {
        let Some(name) = path.file_name().map(str::to_owned) else {
            continue;
        };
        let (alias, file) = if crate::vfs::is_dir(&path) {
            let inner = path.join(inner_name(place));
            if !crate::vfs::exists(&inner) {
                continue;
            }
            (name, inner)
        } else if let Some(stem) = name.strip_suffix(".md") {
            if !is_alias(stem) {
                continue;
            }
            (stem.to_owned(), path.clone())
        } else {
            continue;
        };
        let bytes = match crate::vfs::read_regular(repo.root.as_std_path(), file.as_std_path()) {
            Ok(bytes) => bytes,
            Err(e) => {
                out.faults.push(Fault {
                    outcome: Outcome::Refusal,
                    rule: "format.document-unreadable",
                    file: repo.relative(&file),
                    line: 1,
                    message: format!("could not read the work document: {e}"),
                });
                continue;
            }
        };
        let source = repo.relative(&file);
        if let Some(first) = seen.get(&alias) {
            out.faults.push(Fault {
                outcome: Outcome::Refusal,
                rule: "format.document-duplicate",
                file: source,
                line: 1,
                message: format!(
                    "{alias} is both `{alias}.md` and `{alias}/{}`; a work document has one form",
                    inner_name(place)
                ),
            });
            let first = first.clone();
            out.docs.retain(|d| d.source != first);
            continue;
        }
        seen.insert(alias.clone(), source.clone());
        let mut doc = format::parse_with(&bytes, &out.profiles, &source);
        // §3.3: a document at a work-document path is a work document, so
        // it declares `alias:`.
        if !doc.refused() && !doc.unknown() && doc.alias.is_none() {
            doc.findings.push(format::Finding {
                outcome: Outcome::Refusal,
                rule: "format.field-missing",
                line: 1,
                message: format!("a work document declares `alias:` (here, {alias})"),
            });
        }
        if crate::vfs::is_dir(&path) {
            manifest_restates(repo, &path, &doc, out);
        }
        out.docs.push(Loaded {
            source,
            place,
            doc,
            bytes,
        });
    }
}

/// §4.2: a manifest beside the document holds no atoms and restates no key.
fn manifest_restates(repo: &Repository, dir: &Utf8Path, doc: &Document, out: &mut FormatDocs) {
    let path = dir.join("manifest.toml");
    let text = match crate::vfs::read_regular_to_string(repo.root.as_std_path(), path.as_std_path())
    {
        Ok(text) => text,
        Err(e) => {
            if crate::vfs::exists(&path) {
                out.faults.push(Fault {
                    outcome: Outcome::Refusal,
                    rule: "format.document-unreadable",
                    file: repo.relative(&path),
                    line: 1,
                    message: format!("could not read the adjacent manifest: {e}"),
                });
            }
            return;
        }
    };
    let Ok(value) = toml::from_str::<toml::Table>(&text) else {
        out.faults.push(Fault {
            outcome: Outcome::Refusal,
            rule: "format.manifest-restates",
            file: repo.relative(&path),
            line: 1,
            message: "the manifest beside the document does not parse as TOML".to_owned(),
        });
        return;
    };
    let declared: Vec<&str> = match out.profiles.lookup(&doc.profile) {
        Lookup::Found(p) => p.frontmatter_keys().collect(),
        _ => format::profile_keys(&doc.profile).to_vec(),
    };
    let defined: BTreeSet<&str> = format::CORE_KEYS
        .iter()
        .copied()
        .chain(declared)
        .chain(["atoms", "local_alias", "title", "schema"])
        .collect();
    for key in value.keys() {
        if defined.contains(key.as_str()) {
            let line = text
                .lines()
                .position(|l| {
                    let t = l.trim_start();
                    t.starts_with(&format!("{key} "))
                        || t.starts_with(&format!("{key}="))
                        || t.starts_with(&format!("[[{key}]]"))
                        || t.starts_with(&format!("[{key}]"))
                })
                .map_or(1, |i| i + 1);
            out.faults.push(Fault {
                outcome: Outcome::Refusal,
                rule: "format.manifest-restates",
                file: repo.relative(&path),
                line,
                message: format!(
                    "`{key}` is said by the document itself (or lists atoms); the manifest beside \
                     a Format 1.0 document holds tool settings only"
                ),
            });
        }
    }
}

fn place_of(file: &str, line: usize) -> String {
    format!("{file}:{line}")
}

fn diagnostic(outcome: Outcome, rule: &str, file: String, message: String) -> Diagnostic {
    match outcome {
        Outcome::Refusal => Diagnostic::error(rule, file, message),
        Outcome::Unknown => Diagnostic::unknown(rule, file, message),
        Outcome::Warning => Diagnostic::warn(rule, file, message),
    }
}

/// `war check`'s Format 1.0 rules, into `report`: each finding by its rule
/// at `path:line`, each relation target no record names a warning
/// (`format.target-unknown`), and a pass naming what was read when nothing
/// was refused or left `UNKNOWN`. Silent for a repository with no Format 1.0
/// document, so its check is what it was.
pub fn check(corpus: &Corpus, report: &mut Report) {
    let docs = corpus.format_docs();
    if docs.is_empty() {
        return;
    }
    for f in &docs.faults {
        report.push(diagnostic(
            f.outcome,
            f.rule,
            place_of(&f.file, f.line),
            f.message.clone(),
        ));
    }
    let mut blocked = !docs.faults.is_empty();
    for d in &docs.docs {
        for f in &d.doc.findings {
            blocked |= f.outcome != Outcome::Warning;
            report.push(diagnostic(
                f.outcome,
                f.rule,
                place_of(&d.source, f.line),
                f.message.clone(),
            ));
        }
    }
    // A relation to nothing is kept and reported (§3.6).
    if let Ok(model) = crate::model::build(corpus) {
        let ids: BTreeSet<&str> = model.records.iter().map(|r| r.id.as_str()).collect();
        // `war://<uuid>` is kept as authored and names the work document
        // with that uuid (§5).
        let uris: BTreeSet<String> = model
            .records
            .iter()
            .filter(|r| matches!(r.kind.as_str(), "warrant" | "ticket"))
            .filter_map(|r| r.fields.get("uuid").and_then(|v| v.as_str()))
            .map(|u| format!("war://{u}"))
            .collect();
        for d in docs.docs.iter().filter(|d| d.admitted()) {
            let mine: BTreeSet<&str> = model
                .records
                .iter()
                .filter(|r| r.source == d.source)
                .map(|r| r.id.as_str())
                .collect();
            for rel in model
                .relations
                .iter()
                .filter(|r| mine.contains(r.from.as_str()))
            {
                // An extension kind is inert: its target may lie outside the
                // repository, so it is never reported (§3.6).
                if !ids.contains(rel.to.as_str())
                    && !uris.contains(&rel.to)
                    && !rel.kind.contains('.')
                {
                    report.push(Diagnostic::warn(
                        "format.target-unknown",
                        place_of(&d.source, rel_line(d, &rel.from, &rel.kind)),
                        format!(
                            "{} {} {}: {} names no record; the relation is kept",
                            rel.from, rel.kind, rel.to, rel.to
                        ),
                    ));
                }
            }
        }
    }
    if !blocked {
        let units: usize = docs.docs.iter().map(|d| d.doc.units.len()).sum();
        report.push(Diagnostic::pass(
            "format.well-formed",
            format!(
                "{} Format 1.0 document(s), {units} unit(s): nothing refused, nothing UNKNOWN",
                docs.docs.len()
            ),
        ));
    }
}

/// The line a record's relation of `kind` was authored on, else its heading;
/// an item's relations are on its own line.
fn rel_line(d: &Loaded, from: &str, kind: &str) -> usize {
    let local = from.rsplit('/').next().unwrap_or(from);
    if let Some(u) = d.doc.units.iter().find(|u| u.id == local) {
        return u
            .relations
            .iter()
            .find(|r| r.kind == kind)
            .map_or(u.line, |r| r.line);
    }
    if let Some(it) = d
        .doc
        .units
        .iter()
        .flat_map(|u| &u.items)
        .find(|it| it.id.as_deref() == Some(local))
    {
        return it.line;
    }
    // The document's own record: its frontmatter relation key's line.
    d.doc
        .relations
        .iter()
        .find(|r| r.kind == kind)
        .map_or(1, |r| r.line)
}
