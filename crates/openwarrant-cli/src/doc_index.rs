// SPDX-License-Identifier: Apache-2.0
//! The development-document index (OW-WAR-0148 M18, decision 27): untyped
//! documents count.
//!
//! # What is indexed
//!
//! Every file `[documents] index` in `openwarrant.toml` matches, by default
//! `docs/**/*.md` and the root's `*.md` ([`DEFAULT_INDEX`]). Root-relative
//! globs: `*` and `?` within a segment, `**` for any number of segments. A
//! walk never enters a `generated/` directory (a projection is no source),
//! the Warrant tree, the ticket tree, a hidden directory, `target/` or
//! `node_modules/`.
//!
//! # Typed, or not
//!
//! An indexed document is **typed** when something reads it as records:
//!
//! - a store's type reads it: the specification (`spec`), an ADR atom
//!   (`adr`), the roadmap record's atoms and the plans it retires
//!   (`roadmap`), each only where that type selects `structure`;
//! - it is a record atom under `docs/records/<area>/`, governed by the
//!   profile its front matter names;
//! - it is an instruction file read as `instruction` records (M16);
//! - it was **adopted**: `war plan type <file> <type>` recorded it in
//!   [`TYPES_FILE`], mapping its path to a document type, and never touched
//!   its bytes.
//!
//! Every other indexed document is **untyped**. `war plan model` carries it
//! as a record `doc:<path>` of type `document` with the state `untyped`; an
//! adopted one is a record `doc:<path>` of its type, state `adopted`. The
//! share typed is `war status`'s `document_coverage`: typed and total, a
//! ladder, never a percentage.
//!
//! Nothing here writes, and nothing reads a document's text: whether it is
//! typed is a question about where it is and what names it.

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use serde::{Deserialize, Serialize};

use openwarrant_core::Capability;
use openwarrant_core::projection::Store;

use crate::corpus::Corpus;
use crate::repo::Repository;

/// Where adoptions and installed packs are recorded.
pub const TYPES_FILE: &str = "docs/types.toml";
/// Its schema.
pub const TYPES_SCHEMA: &str = "oh.war/types/v1";
/// What is indexed when `[documents] index` is not set.
pub const DEFAULT_INDEX: [&str; 2] = ["docs/**/*.md", "*.md"];
/// An indexed document's record id is this, then its path.
pub const ID_PREFIX: &str = "doc:";
/// The record type of an untyped document.
pub const UNTYPED_TYPE: &str = "document";

/// `docs/types.toml` (`oh.war/types/v1`): the packs installed and the
/// documents adopted in place. Written by `war plan types add` and
/// `war plan type`; both keep every entry they did not make.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypesFile {
    pub schema: String,
    #[serde(default, rename = "pack", skip_serializing_if = "Vec::is_empty")]
    pub packs: Vec<PackEntry>,
    #[serde(default, rename = "document", skip_serializing_if = "Vec::is_empty")]
    pub documents: Vec<Adoption>,
}

/// One pack installed into `profiles/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackEntry {
    pub name: String,
    pub version: String,
    /// Where it was installed from: `packs/ops`, or the path given.
    pub source: String,
    /// The types it installed, by name.
    pub profiles: Vec<String>,
    /// `sha256:` over its manifest and profile files as installed.
    pub digest: String,
}

/// One document adopted in place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Adoption {
    /// Repository-relative.
    pub path: String,
    /// A document type of the program.
    #[serde(rename = "type")]
    pub doc_type: String,
}

/// What reads an indexed document as records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Governor {
    /// Nothing: an untyped document.
    Untyped,
    /// A store's type (`spec`, `adr`, `roadmap`).
    Store(String),
    /// A record atom, governed by the profile its front matter names.
    Records,
    /// An instruction file (M16).
    Instructions,
    /// Adopted in place as this document type.
    Adopted(String),
}

impl Governor {
    /// Whether the document is typed.
    #[must_use]
    pub fn is_typed(&self) -> bool {
        !matches!(self, Self::Untyped)
    }
}

/// One indexed document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Doc {
    /// Repository-relative.
    pub path: String,
    pub governor: Governor,
}

impl Doc {
    /// Its record id: `doc:<path>`.
    #[must_use]
    pub fn id(&self) -> String {
        format!("{ID_PREFIX}{}", self.path)
    }
}

/// A refusal about the index or `docs/types.toml`, by rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    pub rule: &'static str,
    pub file: String,
    pub message: String,
}

/// The index: every document, sorted by path, and every fault.
#[derive(Debug, Clone, Default)]
pub struct Index {
    pub docs: Vec<Doc>,
    pub faults: Vec<Fault>,
    /// `docs/types.toml`, when it exists and parses.
    pub types: Option<TypesFile>,
}

/// typed / total, as `war status --json` reports it (`document_coverage`).
impl Index {
    #[must_use]
    pub fn coverage(&self) -> openwarrant_core::status::DocumentCoverage {
        let typed = self.docs.iter().filter(|d| d.governor.is_typed()).count();
        openwarrant_core::status::DocumentCoverage {
            typed,
            untyped: self.docs.len() - typed,
            total: self.docs.len(),
        }
    }

    /// The document at `path` (repository-relative), if indexed.
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&Doc> {
        self.docs.iter().find(|d| d.path == path)
    }
}

/// The globs `[documents] index` names, or the default; a table that does
/// not parse is a fault and the default is used.
pub fn globs(repo: &Repository) -> (Vec<String>, Option<Fault>) {
    #[derive(Deserialize, Default)]
    #[serde(deny_unknown_fields)]
    struct Policy {
        #[serde(default)]
        index: Option<Vec<String>>,
    }
    #[derive(Deserialize)]
    struct File {
        #[serde(default)]
        documents: Option<Policy>,
    }
    let default = || DEFAULT_INDEX.iter().map(|s| (*s).to_owned()).collect();
    let path = repo.root.join(crate::init::CONFIG_FILE);
    let Ok(text) = crate::vfs::read_to_string(&path) else {
        return (default(), None);
    };
    match toml::from_str::<File>(&text) {
        Ok(f) => (
            f.documents.and_then(|p| p.index).unwrap_or_else(default),
            None,
        ),
        Err(e) => (
            default(),
            Some(Fault {
                rule: "index.config",
                file: crate::init::CONFIG_FILE.to_owned(),
                message: format!(
                    "the [documents] table does not parse, so the default index ({}) is used: {e}",
                    DEFAULT_INDEX.join(", ")
                ),
            }),
        ),
    }
}

/// `docs/types.toml`, if it exists: parsed, or why not.
pub fn read_types(repo: &Repository) -> Result<Option<TypesFile>, Fault> {
    let path = repo.root.join(TYPES_FILE);
    let Ok(text) = crate::vfs::read_to_string(&path) else {
        return Ok(None);
    };
    let fault = |message: String| Fault {
        rule: "types.malformed",
        file: TYPES_FILE.to_owned(),
        message,
    };
    let file: TypesFile = toml::from_str(&text).map_err(|e| fault(e.to_string()))?;
    if file.schema != TYPES_SCHEMA {
        return Err(fault(format!(
            "schema {:?}; this file declares {TYPES_SCHEMA:?}",
            file.schema
        )));
    }
    Ok(Some(file))
}

/// Directories a walk never enters, by name.
fn skipped(name: &str) -> bool {
    name.starts_with('.') || name == "target" || name == "node_modules" || name == "generated"
}

/// The directories, relative to the root, a walk never enters: the Warrant
/// tree and the ticket tree.
fn pruned(repo: &Repository) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    out.insert(
        repo.config
            .paths
            .warrants
            .trim_end_matches('/')
            .to_owned(),
    );
    if let Ok(store) = crate::ticket::Store::open(repo, None) {
        out.insert(repo.relative(&store.dir));
    }
    out
}

fn children(dir: &Utf8Path) -> Vec<(String, bool)> {
    let Ok(rd) = crate::vfs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, bool)> = rd
        .filter_map(Result::ok)
        .filter_map(|e| {
            let ft = e.file_type().ok()?;
            let name = e.file_name().into_string().ok()?;
            (ft.is_dir() || ft.is_file()).then_some((name, ft.is_dir()))
        })
        .collect();
    out.sort();
    out
}

fn join(rel: &str, name: &str) -> String {
    if rel.is_empty() {
        name.to_owned()
    } else {
        format!("{rel}/{name}")
    }
}

fn expand(
    root: &Utf8Path,
    rel: &str,
    segs: &[&str],
    pruned: &BTreeSet<String>,
    out: &mut BTreeSet<String>,
) {
    let dir = if rel.is_empty() {
        root.to_owned()
    } else {
        root.join(rel)
    };
    match segs.split_first() {
        None => {}
        Some((&"**", rest)) => {
            expand(root, rel, rest, pruned, out);
            for (name, is_dir) in children(&dir) {
                let sub = join(rel, &name);
                if is_dir && !skipped(&name) && !pruned.contains(&sub) {
                    expand(root, &sub, segs, pruned, out);
                }
            }
        }
        Some((seg, rest)) => {
            for (name, is_dir) in children(&dir) {
                if !crate::acceptance::glob_matches(seg, &name) {
                    continue;
                }
                let sub = join(rel, &name);
                if rest.is_empty() && !is_dir {
                    out.insert(sub);
                } else if !rest.is_empty() && is_dir && !skipped(&name) && !pruned.contains(&sub) {
                    expand(root, &sub, rest, pruned, out);
                }
            }
        }
    }
}

/// The paths the globs match, relative and sorted. A glob that is absolute
/// or climbs out with `..` matches nothing.
#[must_use]
pub fn walk(repo: &Repository, globs: &[String]) -> BTreeSet<String> {
    let pruned = pruned(repo);
    let mut out = BTreeSet::new();
    for pattern in globs {
        let p = pattern.trim().trim_start_matches("./");
        if p.is_empty() || p.starts_with('/') || p.split('/').any(|s| s == "..") {
            continue;
        }
        let segs: Vec<&str> = p.split('/').filter(|s| !s.is_empty()).collect();
        expand(&repo.root, "", &segs, &pruned, &mut out);
    }
    out
}

/// Every file a store's type reads, each with the type's name, where that
/// type selects `structure`: the specification, the ADR atoms, the roadmap
/// record's atoms and the plans its record retires.
#[must_use]
pub fn store_files(corpus: &Corpus) -> BTreeMap<String, String> {
    let repo = corpus.repo();
    let mut out = BTreeMap::new();
    let type_name = |store: Store| -> Option<String> {
        repo.profiles
            .store_type(store)
            .filter(|t| t.has(Capability::Structure))
            .map(|t| t.name.clone())
    };
    if let Some(name) = type_name(Store::Sas)
        && let Ok((path, _)) = repo.sas_document()
    {
        out.insert(repo.relative(&path), name);
    }
    if let Some(name) = type_name(Store::Adr) {
        let dir = repo.adr_atoms_dir();
        for (file, is_dir) in children(&dir) {
            if !is_dir && file.ends_with(".md") {
                out.insert(repo.relative(&dir.join(&file)), name.clone());
            }
        }
    }
    if let Some(name) = type_name(Store::Roadmap)
        && let Ok(Some(rm)) = corpus.roadmap()
    {
        for a in &rm.manifest.atoms {
            out.insert(repo.relative(&rm.dir.join(&a.path)), name.clone());
        }
        for r in &rm.manifest.retires {
            out.insert(r.path.trim_start_matches("./").to_owned(), name.clone());
        }
    }
    out
}

/// The index over `corpus`.
#[must_use]
pub fn load(corpus: &Corpus) -> Index {
    let repo = corpus.repo();
    let (globs, config_fault) = globs(repo);
    let mut faults: Vec<Fault> = config_fault.into_iter().collect();
    let types = match read_types(repo) {
        Ok(t) => t,
        Err(f) => {
            faults.push(f);
            None
        }
    };
    let mut paths = walk(repo, &globs);
    let adopted: BTreeMap<String, String> = types
        .iter()
        .flat_map(|t| &t.documents)
        .filter(|a| crate::vfs::is_file(repo.root.join(&a.path)))
        .map(|a| (a.path.clone(), a.doc_type.clone()))
        .collect();
    // An adopted document is indexed wherever it is.
    paths.extend(adopted.keys().cloned());
    let stores = store_files(corpus);
    let records: BTreeSet<String> = crate::records::files(repo)
        .iter()
        .map(|p| repo.relative(p))
        .collect();
    let instructions: BTreeSet<String> = crate::instructions::files(repo)
        .iter()
        .map(|p| repo.relative(p))
        .collect();
    let docs = paths
        .into_iter()
        .map(|path| {
            let governor = if let Some(t) = stores.get(&path) {
                Governor::Store(t.clone())
            } else if records.contains(&path) {
                Governor::Records
            } else if instructions.contains(&path) {
                Governor::Instructions
            } else if let Some(t) = adopted.get(&path) {
                Governor::Adopted(t.clone())
            } else {
                Governor::Untyped
            };
            Doc { path, governor }
        })
        .collect();
    Index {
        docs,
        faults,
        types,
    }
}
