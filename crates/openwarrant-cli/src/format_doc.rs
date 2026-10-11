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

use crate::corpus::Corpus;
use crate::diagnostic::{Diagnostic, Report};
use crate::repo::Repository;

/// What kind of place a document was found in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Warrant,
    Ticket,
    Records,
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

/// Read every Format 1.0 document of `repo`.
#[must_use]
pub fn load(repo: &Repository) -> FormatDocs {
    let mut out = FormatDocs::default();
    for (place, dir) in [
        (Place::Warrant, repo.warrants_dir()),
        (Place::Ticket, tickets_dir(repo)),
    ] {
        work_documents(repo, place, &dir, &mut out);
    }
    for path in crate::records::files(repo) {
        let Ok(bytes) = crate::vfs::read(&path) else {
            continue;
        };
        if format::declares_format(&String::from_utf8_lossy(&bytes)) {
            let doc = format::parse(&bytes);
            out.docs.push(Loaded {
                source: repo.relative(&path),
                place: Place::Records,
                doc,
                bytes,
            });
        }
    }
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
            if !crate::vfs::is_file(&inner) {
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
        let Ok(bytes) = crate::vfs::read(&file) else {
            continue;
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
        let doc = format::parse(&bytes);
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
    let Ok(text) = crate::vfs::read_to_string(&path) else {
        return;
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
    let defined: BTreeSet<&str> = format::CORE_KEYS
        .iter()
        .chain(format::profile_keys(&doc.profile))
        .copied()
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
                if !ids.contains(rel.to.as_str()) && !rel.kind.contains('.') {
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
    d.doc
        .units
        .iter()
        .flat_map(|u| &u.items)
        .find(|it| it.id.as_deref() == Some(local))
        .map_or(1, |it| it.line)
}
