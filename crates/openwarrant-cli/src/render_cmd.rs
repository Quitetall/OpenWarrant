// SPDX-License-Identifier: Apache-2.0
//! `war render <projection> [--of <record or document>]` and the declared
//! documents `war compile` writes (OW-WAR-0148 M6; OW-ADR-0031).
//! `docs/DOCUMENTS.md` walks it; `docs/TYPES.md` has the reference.
//!
//! # Documents
//!
//! A record area (`docs/records/<area>/`) declares its documents in
//! `documents.toml` beside its record atoms:
//!
//! ```toml
//! schema = "oh.war/documents/v1"
//! title = "Password reset"
//!
//! [[document]]
//! type = "prd"            # a document type: profiles/prd.toml, form = "document"
//!
//! [[document]]
//! type = "agent-packet"
//! roots = ["REQ-pr1"]     # default: every record of the area
//! max_bytes = 16000       # default: the projection's own budget
//! ```
//!
//! A document's id is `<area>/<name>` (`name` defaults to the type). `war
//! compile` writes each projection its type declares to
//! `docs/records/<area>/generated/<name>.<md|json>` (or
//! `<name>.<projection>.<ext>` when the type declares more than one), and
//! `war check --generated` compares them with a fresh rendering: a
//! hand-edit is drift (`projection.drift`), a file no document produces is
//! drift too, and a rendering refused for its size is `projection.compile`.
//!
//! # Rendering
//!
//! [`input`] turns the compiled model into the renderer's input — each
//! record with its body (a record atom's span without heading and relation
//! lines, an obligation's statement with its scope, evidence and verdict, a
//! ticket item's text) and every relation — and
//! `openwarrant_compiler::project::render`, a pure function, does the rest.
//! `war render` writes nothing.

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8PathBuf;
use serde::Deserialize;

use openwarrant_compiler::project::{self, Edge, Input, Node, Projection, Subject};
use openwarrant_core::projection::{DocumentProfile, ProjectionDef};

use crate::corpus::Corpus;
use crate::diagnostic::{Diagnostic, Report};
use crate::model::Model;
use crate::repo::{RepoError, Repository};

/// The file an area declares its documents in.
pub const DOCUMENTS_FILE: &str = "documents.toml";
/// Its schema.
pub const DOCUMENTS_SCHEMA: &str = "oh.war/documents/v1";
/// Where an area's projections are written.
pub const GENERATED: &str = "generated";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentsFile {
    schema: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default, rename = "document")]
    documents: Vec<DocumentEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentEntry {
    #[serde(rename = "type")]
    doc_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    roots: Vec<String>,
    #[serde(default)]
    max_bytes: Option<usize>,
}

/// One declared document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    /// `<area>/<name>`.
    pub id: String,
    pub area: String,
    pub name: String,
    /// Its document type.
    pub doc_type: String,
    pub title: String,
    /// Empty: every record of the area.
    pub roots: Vec<String>,
    pub max_bytes: Option<usize>,
    /// Repository-relative path of its `documents.toml`.
    pub source: String,
    /// `sha256:` of that file.
    pub revision: String,
}

/// A refusal about a `documents.toml`, by rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    pub rule: &'static str,
    pub file: String,
    pub message: String,
}

fn areas(repo: &Repository) -> Vec<(String, Utf8PathBuf)> {
    let dir = repo.root.join(crate::records::DIR);
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, Utf8PathBuf)> = rd
        .filter_map(Result::ok)
        .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
        .filter(|p| p.is_dir())
        .filter_map(|p| Some((p.file_name()?.to_owned(), p.clone())))
        .collect();
    out.sort();
    out
}

fn is_word(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|c| c.is_ascii_lowercase())
        && c.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Every declared document of `repo`, sorted by id, and every refusal.
#[must_use]
pub fn documents(repo: &Repository) -> (Vec<Declared>, Vec<Fault>) {
    let mut out = Vec::new();
    let mut faults = Vec::new();
    for (area, dir) in areas(repo) {
        let path = dir.join(DOCUMENTS_FILE);
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let rel = repo.relative(&path);
        let fault = |rule: &'static str, message: String| Fault {
            rule,
            file: rel.clone(),
            message,
        };
        let parsed = std::str::from_utf8(&bytes)
            .map_err(|e| e.to_string())
            .and_then(|t| toml::from_str::<DocumentsFile>(t).map_err(|e| e.to_string()));
        let file = match parsed {
            Ok(f) if f.schema == DOCUMENTS_SCHEMA => f,
            Ok(f) => {
                faults.push(fault(
                    "documents.malformed",
                    format!(
                        "schema {:?}; a documents file declares {DOCUMENTS_SCHEMA:?}",
                        f.schema
                    ),
                ));
                continue;
            }
            Err(e) => {
                faults.push(fault("documents.malformed", e));
                continue;
            }
        };
        let revision = format!("sha256:{}", openwarrant_compiler::sha256_hex(&bytes));
        let mut names = BTreeSet::new();
        for d in file.documents {
            let Some(profile) = repo.profiles.document(&d.doc_type) else {
                let known: Vec<&str> = repo.profiles.documents().map(|p| p.name.as_str()).collect();
                faults.push(fault(
                    "documents.type-unknown",
                    format!(
                        "document type {:?} is no document type of this program (known: {}); a \
                         document type is profiles/<name>.toml with `form = \"document\"`",
                        d.doc_type,
                        if known.is_empty() {
                            "none".to_owned()
                        } else {
                            known.join(", ")
                        }
                    ),
                ));
                continue;
            };
            let name = d.name.clone().unwrap_or_else(|| d.doc_type.clone());
            if !is_word(&name) {
                faults.push(fault(
                    "documents.malformed",
                    format!("document name {name:?} is not a lowercase word"),
                ));
                continue;
            }
            if !names.insert(name.clone()) {
                faults.push(fault(
                    "documents.duplicate",
                    format!("document {area}/{name} is declared twice; give one a `name`"),
                ));
                continue;
            }
            if d.max_bytes == Some(0) {
                faults.push(fault(
                    "documents.malformed",
                    format!("document {area}/{name}: max_bytes is 0"),
                ));
                continue;
            }
            out.push(Declared {
                id: format!("{area}/{name}"),
                area: area.clone(),
                name,
                doc_type: profile.name.clone(),
                title: d
                    .title
                    .or_else(|| file.title.clone())
                    .unwrap_or_else(|| area.clone()),
                roots: d.roots,
                max_bytes: d.max_bytes,
                source: rel.clone(),
                revision: revision.clone(),
            });
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    (out, faults)
}

// ---- The renderer's input.

/// A record atom record's body: its span without the heading line and its
/// relation lines, blank lines at either end trimmed.
fn body(text: &str, r: &openwarrant_core::record::AuthoredRecord) -> String {
    let relation_lines: BTreeSet<usize> = r.relations.iter().map(|x| x.line).collect();
    let lines: Vec<&str> = text[r.start..r.end]
        .split_inclusive('\n')
        .enumerate()
        .filter(|(i, _)| *i > 0 && !relation_lines.contains(&(r.line + i)))
        .map(|(_, l)| l.trim_end_matches(['\n', '\r']))
        .collect();
    let start = lines
        .iter()
        .position(|l| !l.trim().is_empty())
        .unwrap_or(lines.len());
    let end = lines
        .iter()
        .rposition(|l| !l.trim().is_empty())
        .map_or(start, |e| e + 1);
    lines[start..end].join("\n")
}

/// A record's line, body and fields, as the renderer reads them.
type Body = (Option<usize>, String, BTreeMap<String, String>);

/// One declared projection's path, and its bytes or why there are none.
pub type CompiledFile = (Utf8PathBuf, Result<String, RepoError>);

/// The renderer's input over the corpus: every model record with its body
/// and fields, and every relation.
#[must_use]
pub fn input(corpus: &Corpus, model: &Model) -> Input {
    let repo = corpus.repo();
    let mut bodies: BTreeMap<String, Body> = BTreeMap::new();
    let admitted = corpus.records();
    for path in crate::records::files(repo) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(atom) = openwarrant_core::record::parse(&text) else {
            continue;
        };
        let rel = repo.relative(&path);
        for r in &atom.records {
            // The first declaration stands; a refused duplicate has no body here.
            if admitted
                .get(&r.id)
                .is_some_and(|a| a.source == rel && a.line == r.line)
            {
                bodies.insert(
                    r.id.clone(),
                    (Some(r.line), body(&text, r), BTreeMap::new()),
                );
            }
        }
    }
    let verdict = |id: &str| -> Option<String> {
        model
            .states
            .iter()
            .find(|s| s.record == id && s.kind == "disposition")
            .map(|s| s.value.clone())
    };
    if let Ok(entries) = corpus.entries() {
        for e in entries {
            let Some(one) = e.ok() else { continue };
            let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
                continue;
            };
            let alias = validated.alias.to_string();
            for atom in basis.atoms.iter().filter(|a| a.role == "assurance") {
                let Ok(set) =
                    openwarrant_core::obligation::parse(&String::from_utf8_lossy(&atom.bytes))
                else {
                    continue;
                };
                for o in set.obligations {
                    let id = format!("{alias}/{}", o.id);
                    let mut fields = BTreeMap::new();
                    fields.insert("scope".to_owned(), o.scope.clone());
                    fields.insert("evidence".to_owned(), o.evidence.clone());
                    if let Some(v) = verdict(&id) {
                        fields.insert("verdict".to_owned(), v);
                    }
                    bodies.insert(id, (None, o.statement.clone(), fields));
                }
            }
        }
    }
    if let Ok((tickets, _)) = corpus.tickets() {
        for t in tickets {
            for item in &t.checklist.items {
                let Some(iid) = &item.id else { continue };
                let mut fields = BTreeMap::new();
                fields.insert(
                    "state".to_owned(),
                    if item.done { "done" } else { "open" }.to_owned(),
                );
                bodies.insert(
                    format!("{}/{iid}", t.id()),
                    (Some(item.line + 1), item.text.clone(), fields),
                );
            }
        }
    }
    let records = model
        .records
        .iter()
        .map(|r| {
            let (line, body, fields) = bodies.remove(&r.id).unwrap_or_default();
            Node {
                id: r.id.clone(),
                kind: r.kind.clone(),
                source: r.source.clone(),
                line,
                revision: r.revision.clone(),
                body,
                fields,
            }
        })
        .collect();
    let relations = model
        .relations
        .iter()
        .map(|r| Edge {
            from: r.from.clone(),
            kind: r.kind.clone(),
            to: r.to.clone(),
            to_revision: r.to_revision.clone(),
        })
        .collect();
    Input { records, relations }
}

/// The records of an area, in authored order: a document's default seeds.
fn area_seeds(corpus: &Corpus, area: &str) -> Vec<String> {
    let prefix = format!("{}/{area}/", crate::records::DIR);
    corpus
        .records()
        .records
        .iter()
        .filter(|r| r.source.starts_with(&prefix))
        .map(|r| r.id.clone())
        .collect()
}

/// The subject a declared document renders.
#[must_use]
pub fn subject_of(corpus: &Corpus, d: &Declared) -> Subject {
    Subject {
        id: d.id.clone(),
        kind: "document".to_owned(),
        title: d.title.clone(),
        seeds: if d.roots.is_empty() {
            area_seeds(corpus, &d.area)
        } else {
            d.roots.clone()
        },
        revision: Some(d.revision.clone()),
        source: Some(d.source.clone()),
    }
}

/// Where `war compile` writes projection `def` of document `d`.
#[must_use]
pub fn path_of(
    repo: &Repository,
    d: &Declared,
    profile: &DocumentProfile,
    def: &ProjectionDef,
) -> Utf8PathBuf {
    let file = if profile.projections.len() == 1 {
        format!("{}.{}", d.name, def.renderer.extension())
    } else {
        format!("{}.{}.{}", d.name, def.name, def.renderer.extension())
    };
    repo.root
        .join(crate::records::DIR)
        .join(&d.area)
        .join(GENERATED)
        .join(file)
}

/// One declared rendering: its document, projection, path, and the
/// rendering or why it was refused.
pub struct Compiled<'a> {
    pub document: &'a Declared,
    pub projection: &'a ProjectionDef,
    pub path: Utf8PathBuf,
    pub rendered: Result<Projection, project::RenderError>,
}

/// Every declared document's projections, rendered over `input`.
#[must_use]
pub fn compile_all<'a>(
    corpus: &'a Corpus,
    declared: &'a [Declared],
    input: &Input,
    bounded: bool,
) -> Vec<Compiled<'a>> {
    let repo = corpus.repo();
    let mut out = Vec::new();
    for d in declared {
        let Some(profile) = repo.profiles.document(&d.doc_type) else {
            continue;
        };
        let subject = subject_of(corpus, d);
        for def in &profile.projections {
            let rendered = if bounded {
                project::render(input, profile, def, &subject, d.max_bytes)
            } else {
                project::render_unbounded(input, profile, def, &subject)
            };
            out.push(Compiled {
                document: d,
                projection: def,
                path: path_of(repo, d, profile, def),
                rendered,
            });
        }
    }
    out
}

/// What `war compile` writes and `war check --generated` compares: each
/// declared projection's path and bytes, or why it cannot be rendered.
/// Empty, and reading nothing more, for a program that declares no document.
pub fn compiled(corpus: &Corpus) -> Result<Vec<CompiledFile>, RepoError> {
    let (declared, _) = documents(corpus.repo());
    if declared.is_empty() {
        return Ok(Vec::new());
    }
    let model = crate::model::build(corpus)?;
    let input = input(corpus, &model);
    Ok(compile_all(corpus, &declared, &input, true)
        .into_iter()
        .map(|c| {
            let bytes = c
                .rendered
                .map(|p| p.content)
                .map_err(|e| RepoError::Message(format!("{}: {}: {e}", e.rule(), c.document.id)));
            (c.path, bytes)
        })
        .collect())
}

/// Files under an area's `generated/` that no declared projection produces.
#[must_use]
pub fn orphans(repo: &Repository, produced: &BTreeSet<Utf8PathBuf>) -> Vec<Utf8PathBuf> {
    let mut out = Vec::new();
    for (_, dir) in areas(repo) {
        let Ok(rd) = std::fs::read_dir(dir.join(GENERATED)) else {
            continue;
        };
        for p in rd
            .filter_map(Result::ok)
            .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
        {
            if p.is_file() && !produced.contains(&p) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// `war check --generated`'s comparison of declared projections, into
/// `report`: `projection.drift` per file (a hand-edit, or a file no
/// document produces), `projection.missing`, `projection.compile`. Silent for
/// a program that declares no document and has no generated projection.
pub fn check_generated(corpus: &Corpus, report: &mut Report) {
    let repo = corpus.repo();
    let compiled = match compiled(corpus) {
        Ok(c) => c,
        Err(e) => {
            report.push(Diagnostic::error(
                "projection.compile",
                crate::records::DIR.to_owned(),
                e.to_string(),
            ));
            return;
        }
    };
    let produced: BTreeSet<Utf8PathBuf> = compiled.iter().map(|(p, _)| p.clone()).collect();
    for (path, bytes) in compiled {
        let relative = repo.relative(&path);
        match bytes {
            Err(e) => report.push(Diagnostic::error(
                "projection.compile",
                relative,
                e.to_string(),
            )),
            Ok(expected) => match std::fs::read_to_string(&path) {
                Ok(actual) if actual == expected => report.push(Diagnostic::pass(
                    "projection.drift",
                    format!("{relative} matches a fresh rendering"),
                )),
                Ok(_) => report.push(Diagnostic::error(
                    "projection.drift",
                    relative.clone(),
                    format!(
                        "the committed {relative} differs from a fresh rendering; it was edited \
                         by hand or its records changed without `war compile`. Edit the \
                         records, then recompile"
                    ),
                )),
                Err(_) if repo.config.generated.commit => report.push(Diagnostic::error(
                    "projection.missing",
                    relative,
                    "missing, and this repository commits generated views; run `war compile`"
                        .to_owned(),
                )),
                Err(_) => {}
            },
        }
    }
    for p in orphans(repo, &produced) {
        let relative = repo.relative(&p);
        report.push(Diagnostic::error(
            "projection.drift",
            relative.clone(),
            format!(
                "{relative} is produced by no declared document; a projection is generated, \
                 never authored. Declare its document in {DOCUMENTS_FILE}, or remove it"
            ),
        ));
    }
}

/// `war check`'s document rules: each `documents.toml` refusal, each root
/// that is no record, and one pass when nothing is refused. Silent for a
/// program that declares no document.
pub fn check(corpus: &Corpus, report: &mut Report) {
    let repo = corpus.repo();
    let (declared, faults) = documents(repo);
    if declared.is_empty() && faults.is_empty() {
        return;
    }
    let mut refused = faults.len();
    for f in faults {
        report.push(Diagnostic::error(f.rule, f.file, f.message));
    }
    let authored = corpus.records();
    let mut model: Option<Option<Model>> = None;
    for d in &declared {
        for root in &d.roots {
            if authored.get(root).is_some() {
                continue;
            }
            let m = model.get_or_insert_with(|| crate::model::build(corpus).ok());
            if m.as_ref()
                .is_some_and(|m| m.records.iter().any(|r| &r.id == root))
            {
                continue;
            }
            refused += 1;
            report.push(Diagnostic::error(
                "documents.root-unknown",
                d.source.clone(),
                format!(
                    "document {}: root {root} is not a record of this corpus; `war model \
                     --json` lists every record id",
                    d.id
                ),
            ));
        }
    }
    if refused == 0 {
        let areas: BTreeSet<&str> = declared.iter().map(|d| d.area.as_str()).collect();
        report.push(Diagnostic::pass(
            "documents.well-formed",
            format!(
                "{} document(s) in {} area(s), each of a declared document type",
                declared.len(),
                areas.len()
            ),
        ));
    }
}

/// The declared projections that select `id`, for `war impact`: each with
/// `id` and the other `reached` records it also selects. A projection that
/// does not select `id` is not listed, whatever else it shows.
#[must_use]
pub fn selecting(
    corpus: &Corpus,
    model: &Model,
    id: &str,
    reached: &BTreeSet<&str>,
) -> Vec<crate::impact::Projection> {
    let repo = corpus.repo();
    let (declared, _) = documents(repo);
    if declared.is_empty() {
        return Vec::new();
    }
    let input = input(corpus, model);
    let mut out = Vec::new();
    for c in compile_all(corpus, &declared, &input, false) {
        let Ok(p) = c.rendered else { continue };
        if !p.selects(id) {
            continue;
        }
        let mut selects = vec![id.to_owned()];
        selects.extend(
            p.selects
                .iter()
                .filter(|s| s.id != id && reached.contains(s.id.as_str()))
                .map(|s| s.id.clone()),
        );
        out.push(crate::impact::Projection {
            path: repo.relative(&c.path),
            scope: "declared".to_owned(),
            selects,
        });
    }
    out
}

/// What `--of` named.
enum Of {
    Document(Declared),
    Area(String),
    Record(String),
}

/// `war render`: the rendering, or a report saying why there is none.
/// Writes nothing.
pub fn run(
    corpus: &Corpus,
    projection: &str,
    of: Option<&str>,
    max_bytes: Option<usize>,
) -> Result<(Report, Option<Projection>), RepoError> {
    let repo = corpus.repo();
    let mut report = Report::default();
    let Some((profile, def)) = repo.profiles.projection(projection) else {
        let known: Vec<&str> = repo
            .profiles
            .documents()
            .flat_map(|d| d.projections.iter().map(|p| p.name.as_str()))
            .collect();
        report.push(Diagnostic::error(
            "projection.unknown",
            projection.to_owned(),
            format!(
                "{projection} is no projection of this program's document types (known: {}); a \
                 projection is declared by a profile with `form = \"document\"`",
                if known.is_empty() {
                    "none".to_owned()
                } else {
                    known.join(", ")
                }
            ),
        ));
        return Ok((report, None));
    };
    let (declared, _) = documents(repo);
    let target =
        match of {
            Some(x) => {
                if let Some(d) = declared.iter().find(|d| d.id == x) {
                    Of::Document(d.clone())
                } else if !x.contains('/')
                    && is_word(x)
                    && repo.root.join(crate::records::DIR).join(x).is_dir()
                {
                    Of::Area(x.to_owned())
                } else {
                    Of::Record(x.to_owned())
                }
            }
            None => {
                let mine: Vec<&Declared> = declared
                    .iter()
                    .filter(|d| d.doc_type == profile.name)
                    .collect();
                match mine.as_slice() {
                    [one] => Of::Document((*one).clone()),
                    _ => {
                        report.push(Diagnostic::error(
                        "projection.of-missing",
                        projection.to_owned(),
                        format!(
                            "name what to render with --of: a record id, an area, or a document \
                             ({})",
                            if mine.is_empty() {
                                format!("no document of type {} is declared", profile.name)
                            } else {
                                mine.iter().map(|d| d.id.as_str()).collect::<Vec<_>>().join(", ")
                            }
                        ),
                    ));
                        return Ok((report, None));
                    }
                }
            }
        };
    let model = crate::model::build(corpus)?;
    let (subject, budget) = match target {
        Of::Document(d) => (subject_of(corpus, &d), max_bytes.or(d.max_bytes)),
        Of::Area(area) => {
            let title = declared
                .iter()
                .find(|d| d.area == area)
                .map_or_else(|| area.clone(), |d| d.title.clone());
            (
                Subject {
                    id: area.clone(),
                    kind: "area".to_owned(),
                    title,
                    seeds: area_seeds(corpus, &area),
                    revision: None,
                    source: None,
                },
                max_bytes,
            )
        }
        Of::Record(id) => {
            if !model.records.iter().any(|r| r.id == id) {
                report.push(Diagnostic::error(
                    "projection.of-unknown",
                    id.clone(),
                    format!(
                        "{id} is no declared document ({}), record area or record of this \
                         corpus",
                        if declared.is_empty() {
                            "none is declared".to_owned()
                        } else {
                            declared
                                .iter()
                                .map(|d| d.id.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        }
                    ),
                ));
                return Ok((report, None));
            }
            (
                Subject {
                    id: id.clone(),
                    kind: "record".to_owned(),
                    title: id.clone(),
                    seeds: vec![id],
                    revision: None,
                    source: None,
                },
                max_bytes,
            )
        }
    };
    let input = input(corpus, &model);
    match project::render(&input, profile, def, &subject, budget) {
        Ok(p) => {
            report.push(Diagnostic::pass("projection.rendered", summary(&p)));
            Ok((report, Some(p)))
        }
        Err(e) => {
            report.push(Diagnostic::error(
                e.rule(),
                subject.id.clone(),
                e.to_string(),
            ));
            Ok((report, None))
        }
    }
}

/// One line about a rendering.
#[must_use]
pub fn summary(p: &Projection) -> String {
    format!(
        "{} of {}: {} bytes{}, {} record(s) selected, {} mentioned, {} trace run(s)",
        p.projection,
        p.subject.id,
        p.bytes,
        p.max_bytes
            .map(|m| format!(" of a {m}-byte budget"))
            .unwrap_or_default(),
        p.selects.len(),
        p.mentions.len(),
        p.trace.len()
    )
}
