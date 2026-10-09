// SPDX-License-Identifier: Apache-2.0
//! Every development document is a type (OW-WAR-0148 M18; decisions 25-27).
//! docs/TYPES.md, "Every development document is a type", is the guide.
//!
//! - **Built-in types.** The roadmap, the specification and ADRs are types
//!   whose records are read from the stores that predate typed records
//!   (`encoding = "roadmap" | "sas" | "adr"`), and their rules are gated on
//!   the capabilities those types select. Releases, incidents and exit
//!   reports are core document types. This build ships all six
//!   ([`builtins`]); a program's own `profiles/<name>.toml` replaces one.
//! - **Packs.** `war plan types add <pack>` installs a versioned set of
//!   profile files from `packs/<name>/` of the repository, or a directory
//!   given by path, into `profiles/`, after every one of them is admitted
//!   beside the program's own types. Refused, writing nothing: a profile the
//!   registry refuses (a capability outside the closed set,
//!   `profile.capability-unknown`), a name a type of the program already
//!   has (`types.collision`), a pack that cannot be read
//!   (`types.pack-unknown`, `types.pack-malformed`).
//! - **Adoption.** `war plan type <file> <type>` records in
//!   `docs/types.toml` that a document is of a document type, and never
//!   touches the document's bytes ([`crate::doc_index`]).
//! - **The list.** `war plan types` names every type the program admits:
//!   its form, the store it reads, its capabilities, where it comes from.

use std::collections::BTreeSet;

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use openwarrant_core::projection::Store;
use openwarrant_core::{Capabilities, Capability};

use crate::diagnostic::{Diagnostic, Report};
use crate::doc_index::{self, Adoption, PackEntry, TYPES_FILE, TYPES_SCHEMA, TypesFile};
use crate::repo::{RepoError, Repository};

/// The types this build ships, admitted where a program declares none of
/// the same name (or, for a store's type, none reading the same store).
const BUILTIN: [(&str, &str); 6] = [
    (
        "(built in)/roadmap.toml",
        include_str!("../../../profiles/roadmap.toml"),
    ),
    (
        "(built in)/spec.toml",
        include_str!("../../../profiles/spec.toml"),
    ),
    (
        "(built in)/adr.toml",
        include_str!("../../../profiles/adr.toml"),
    ),
    (
        "(built in)/release.toml",
        include_str!("../../../profiles/release.toml"),
    ),
    (
        "(built in)/incident.toml",
        include_str!("../../../profiles/incident.toml"),
    ),
    (
        "(built in)/exit-report.toml",
        include_str!("../../../profiles/exit-report.toml"),
    ),
];

/// The built-in types, as `(file, bytes)` for
/// [`openwarrant_core::role::ProfileRegistry::with_builtins`].
pub fn builtins() -> impl Iterator<Item = (&'static str, &'static [u8])> {
    BUILTIN.iter().map(|(f, b)| (*f, b.as_bytes()))
}

/// The names of the built-in types.
fn builtin_names() -> BTreeSet<&'static str> {
    BUILTIN
        .iter()
        .map(|(f, _)| {
            let stem = f.strip_suffix(".toml").unwrap_or(f);
            stem.rsplit('/').next().unwrap_or(stem)
        })
        .collect()
}

/// The capabilities of the type that reads `store`: what its rules are
/// gated on.
#[must_use]
pub fn caps(repo: &Repository, store: Store) -> Capabilities {
    repo.profiles.store_capabilities(store)
}

/// Whether the type that reads `store` selects `cap`.
#[must_use]
pub fn has(repo: &Repository, store: Store, cap: Capability) -> bool {
    caps(repo, store).has(cap)
}

// ---- `war plan types` --------------------------------------------------------

/// One type, as `war plan types` lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TypeView {
    pub name: String,
    /// `contract` (a Warrant profile), `working` (a ticket), or `document`.
    pub form: String,
    /// The store it reads, for a store's type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<String>,
    pub capabilities: Vec<String>,
    /// `built in`, `profiles/<name>.toml`, or `pack <name> <version>`.
    pub from: String,
    /// The projections it declares.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub projections: Vec<String>,
}

/// Every type the program admits, sorted by name.
#[must_use]
pub fn list(repo: &Repository) -> Vec<TypeView> {
    let installed: Vec<PackEntry> = doc_index::read_types(repo)
        .ok()
        .flatten()
        .map(|t| t.packs)
        .unwrap_or_default();
    let from = |name: &str| -> String {
        if let Some(p) = installed
            .iter()
            .find(|p| p.profiles.iter().any(|n| n == name))
        {
            return format!("pack {} {}", p.name, p.version);
        }
        let file = format!("profiles/{name}.toml");
        if crate::vfs::is_file(repo.root.join(&file)) {
            file
        } else {
            "built in".to_owned()
        }
    };
    let words = |c: Capabilities| c.iter().map(|c| c.as_str().to_owned()).collect();
    let mut out: Vec<TypeView> = repo
        .profiles
        .definitions()
        .map(|d| TypeView {
            name: d.name.clone(),
            form: if d.is_working_form() {
                "working"
            } else {
                "contract"
            }
            .to_owned(),
            store: None,
            capabilities: words(d.capabilities()),
            from: from(&d.name),
            projections: Vec::new(),
        })
        .collect();
    // The ticket's working form is the ticket store's (built in unless the
    // program writes profiles/ticket.toml), not the registry's.
    if !out.iter().any(|t| t.name == "ticket")
        && let Ok(store) = crate::ticket::Store::open(repo, None)
    {
        out.push(TypeView {
            name: store.definition.name.clone(),
            form: "working".to_owned(),
            store: None,
            capabilities: words(store.definition.capabilities()),
            from: from(&store.definition.name),
            projections: Vec::new(),
        });
    }
    out.extend(repo.profiles.documents().map(|d| TypeView {
        name: d.name.clone(),
        form: "document".to_owned(),
        store: d.encoding.map(|s| s.as_str().to_owned()),
        capabilities: words(d.capabilities),
        from: from(&d.name),
        projections: d.projections.iter().map(|p| p.name.clone()).collect(),
    }));
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// `war plan types`, as lines.
#[must_use]
pub fn render(types: &[TypeView]) -> String {
    let mut s = String::new();
    for t in types {
        s.push_str(&format!(
            "{:<20} {:<9} {:<28} {}{}\n",
            t.name,
            t.form,
            t.from,
            t.capabilities.join(","),
            t.store
                .as_ref()
                .map(|s| format!(" · reads the {s} store"))
                .unwrap_or_default()
        ));
    }
    s
}

// ---- `war plan types add <pack>` -----------------------------------------------

/// The pack manifest's schema.
pub const PACK_SCHEMA: &str = "oh.war/pack/v1";
/// Where a repository keeps its packs.
pub const PACKS_DIR: &str = "packs";

/// `packs/<name>/pack.toml` (`oh.war/pack/v1`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackFile {
    schema: String,
    name: String,
    version: String,
    #[serde(default)]
    #[allow(dead_code)]
    note: Option<String>,
    profiles: Vec<String>,
}

/// What `war plan types add` did, or would do.
#[derive(Debug, Clone, Serialize)]
pub struct Installed {
    pub pack: String,
    pub version: String,
    pub source: String,
    /// The types it installs, by name.
    pub types: Vec<String>,
    /// The files it writes (or wrote), repository-relative.
    pub files: Vec<String>,
    /// `installed`, `would-install` (`--dry-run`), or `unchanged` (already
    /// installed, byte for byte).
    pub outcome: String,
}

fn refuse(report: &mut Report, rule: &str, file: impl Into<String>, message: String) {
    report.push(Diagnostic::error(rule, file, message));
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{}", openwarrant_compiler::sha256_hex(bytes))
}

/// The packs this build ships, installable in any repository: each its
/// manifest and profile files, the bytes of `packs/<name>/`.
const BUILTIN_PACKS: [(&str, &[(&str, &str)]); 2] = [
    (
        "ops",
        &[
            ("pack.toml", include_str!("../../../packs/ops/pack.toml")),
            (
                "runbook.toml",
                include_str!("../../../packs/ops/runbook.toml"),
            ),
            ("slo.toml", include_str!("../../../packs/ops/slo.toml")),
            (
                "rollout.toml",
                include_str!("../../../packs/ops/rollout.toml"),
            ),
            (
                "incident-review.toml",
                include_str!("../../../packs/ops/incident-review.toml"),
            ),
        ],
    ),
    (
        "quality",
        &[
            (
                "pack.toml",
                include_str!("../../../packs/quality/pack.toml"),
            ),
            (
                "threat-model.toml",
                include_str!("../../../packs/quality/threat-model.toml"),
            ),
            (
                "performance-budget.toml",
                include_str!("../../../packs/quality/performance-budget.toml"),
            ),
            (
                "test-charter.toml",
                include_str!("../../../packs/quality/test-charter.toml"),
            ),
        ],
    ),
];

/// Where a pack is read from.
enum Origin {
    /// A directory: `packs/<name>/` of the repository, or a path.
    Dir(Utf8PathBuf),
    /// One this build ships.
    Builtin(&'static [(&'static str, &'static str)]),
}

impl Origin {
    fn read(&self, file: &str) -> Option<Vec<u8>> {
        match self {
            Self::Dir(dir) => crate::vfs::read(dir.join(file)).ok(),
            Self::Builtin(files) => files
                .iter()
                .find(|(f, _)| *f == file)
                .map(|(_, b)| b.as_bytes().to_vec()),
        }
    }
}

/// The pack `pack` names: `packs/<pack>/` of the repository when it is a
/// word and that directory holds a pack, else the pack this build ships by
/// that name, else `pack` as a directory (relative to the working
/// directory, or absolute). With it, how it is recorded: the repository-
/// relative directory, `built in`, or `outside the repository`.
fn pack_origin(repo: &Repository, pack: &str) -> (Origin, String) {
    let word = !pack.is_empty()
        && pack
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    let local = repo.root.join(PACKS_DIR).join(pack);
    if word && crate::vfs::is_file(local.join("pack.toml")) {
        return (Origin::Dir(local), format!("{PACKS_DIR}/{pack}"));
    }
    if word && let Some((_, files)) = BUILTIN_PACKS.iter().find(|(n, _)| *n == pack) {
        return (Origin::Builtin(files), "built in".to_owned());
    }
    let dir = Utf8PathBuf::from(pack);
    let absolute = std::fs::canonicalize(&dir)
        .ok()
        .and_then(|p| Utf8PathBuf::from_path_buf(p).ok());
    let root = std::fs::canonicalize(&repo.root)
        .ok()
        .and_then(|p| Utf8PathBuf::from_path_buf(p).ok());
    let label = match (absolute, root) {
        (Some(a), Some(r)) => a
            .strip_prefix(&r)
            .map_or_else(|_| "outside the repository".to_owned(), |p| p.to_string()),
        _ => "outside the repository".to_owned(),
    };
    (Origin::Dir(dir), label)
}

/// `war plan types add <pack> [--dry-run]`: validate every profile of the
/// pack beside the program's own types, and only then copy them into
/// `profiles/` and record the install in `docs/types.toml`. Refused by
/// rule, writing nothing.
pub fn add(
    repo: &Repository,
    pack: &str,
    dry_run: bool,
) -> Result<(Report, Option<Installed>), RepoError> {
    let mut report = Report::default();
    let (origin, source) = pack_origin(repo, pack);
    let manifest_shown = match &origin {
        Origin::Dir(dir) => {
            let p = dir.join("pack.toml");
            p.strip_prefix(&repo.root)
                .map_or_else(|_| p.to_string(), ToString::to_string)
        }
        Origin::Builtin(_) => format!("(built in)/{pack}/pack.toml"),
    };
    let Some(manifest_bytes) = origin.read("pack.toml") else {
        let mut known: Vec<String> = crate::vfs::read_dir(repo.root.join(PACKS_DIR))
            .map(|rd| {
                rd.filter_map(Result::ok)
                    .filter(|e| e.path().join("pack.toml").is_file())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default();
        known.extend(BUILTIN_PACKS.iter().map(|(n, _)| (*n).to_owned()));
        known.sort();
        known.dedup();
        refuse(
            &mut report,
            "types.pack-unknown",
            pack,
            format!(
                "no pack {pack:?}: neither a pack of this repository or this build by that name, \
                 nor a directory holding pack.toml (`oh.war/pack/v1`). Packs: {}",
                known.join(", ")
            ),
        );
        return Ok((report, None));
    };
    let manifest: PackFile = match std::str::from_utf8(&manifest_bytes)
        .map_err(|e| e.to_string())
        .and_then(|t| toml::from_str(t).map_err(|e| e.to_string()))
    {
        Ok(m) => m,
        Err(e) => {
            refuse(
                &mut report,
                "types.pack-malformed",
                manifest_shown.clone(),
                e,
            );
            return Ok((report, None));
        }
    };
    if manifest.schema != PACK_SCHEMA || manifest.profiles.is_empty() {
        refuse(
            &mut report,
            "types.pack-malformed",
            manifest_shown.clone(),
            format!(
                "a pack manifest declares schema {PACK_SCHEMA:?} and at least one profile file; \
                 this one declares schema {:?} and {} file(s)",
                manifest.schema,
                manifest.profiles.len()
            ),
        );
        return Ok((report, None));
    }
    // Read every file the pack names: a plain `<name>.toml` beside pack.toml.
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for f in &manifest.profiles {
        let plain = f.ends_with(".toml")
            && f != "pack.toml"
            && !f.contains('/')
            && !f.contains('\\')
            && !f.starts_with('.');
        let bytes = if plain { origin.read(f) } else { None };
        let Some(bytes) = bytes else {
            refuse(
                &mut report,
                "types.pack-malformed",
                manifest_shown.clone(),
                format!("profile file {f:?} is not a readable `<name>.toml` beside pack.toml"),
            );
            return Ok((report, None));
        };
        files.push((f.clone(), bytes));
    }
    let mut pack_digest_input = manifest_bytes.clone();
    for (f, b) in &files {
        pack_digest_input.extend_from_slice(f.as_bytes());
        pack_digest_input.push(0);
        pack_digest_input.extend_from_slice(b);
    }
    let pack_digest = digest(&pack_digest_input);
    let types: Vec<String> = files
        .iter()
        .map(|(f, _)| f.trim_end_matches(".toml").to_owned())
        .collect();
    let written: Vec<String> = files.iter().map(|(f, _)| format!("profiles/{f}")).collect();

    // Already installed, byte for byte: nothing to write.
    let profiles_dir = repo.root.join("profiles");
    let same = files.iter().all(|(f, b)| {
        crate::vfs::read(profiles_dir.join(f))
            .ok()
            .is_some_and(|on_disk| &on_disk == b)
    });
    let recorded = doc_index::read_types(repo).ok().flatten().is_some_and(|t| {
        t.packs
            .iter()
            .any(|p| p.name == manifest.name && p.digest == pack_digest)
    });
    if same && recorded {
        report.push(Diagnostic::pass(
            "types.pack-installed",
            format!(
                "pack {} {} is installed already, byte for byte; nothing written",
                manifest.name, manifest.version
            ),
        ));
        return Ok((
            report,
            Some(Installed {
                pack: manifest.name,
                version: manifest.version,
                source,
                types,
                files: written,
                outcome: "unchanged".to_owned(),
            }),
        ));
    }

    // A name a type of the program already has: refused by name. The
    // program's own files, the built-in types, and every name the registry
    // admits (a Warrant profile's included).
    let mut taken: BTreeSet<String> = repo
        .profiles
        .definitions()
        .map(|d| d.name.clone())
        .chain(repo.profiles.documents().map(|d| d.name.clone()))
        .collect();
    taken.extend(builtin_names().into_iter().map(str::to_owned));
    let mut collisions = Vec::new();
    for (f, _) in &files {
        let name = f.trim_end_matches(".toml");
        let on_disk = profiles_dir.join(f);
        if taken.contains(name) || crate::vfs::is_file(&on_disk) {
            collisions.push(format!(
                "`{name}` ({})",
                if crate::vfs::is_file(&on_disk) {
                    format!("profiles/{f} exists")
                } else {
                    "a built-in type".to_owned()
                }
            ));
        }
    }
    if !collisions.is_empty() {
        refuse(
            &mut report,
            "types.collision",
            manifest_shown.clone(),
            format!(
                "pack {} {} would install {}, a name a type of this program already has; a \
                 type's name is unique, so nothing is installed. Rename the pack's type, or \
                 remove the program's own",
                manifest.name,
                manifest.version,
                collisions.join(", ")
            ),
        );
        return Ok((report, None));
    }

    // Every file admitted beside the program's own, by the same registry
    // that reads profiles/ when the repository opens.
    let mut program: Vec<(String, Vec<u8>)> = Vec::new();
    if let Ok(rd) = crate::vfs::read_dir(&profiles_dir) {
        for e in rd.filter_map(Result::ok) {
            let Ok(p) = Utf8PathBuf::from_path_buf(e.path()) else {
                continue;
            };
            if p.extension() == Some("toml")
                && let Ok(b) = crate::vfs::read(&p)
            {
                program.push((format!("profiles/{}", p.file_name().unwrap_or_default()), b));
            }
        }
    }
    program.extend(
        files
            .iter()
            .map(|(f, b)| (format!("{source}/{f}"), b.clone())),
    );
    program.sort();
    if let Err(e) = openwarrant_core::role::ProfileRegistry::with_builtins(
        program.iter().map(|(f, b)| (f.as_str(), b.as_slice())),
        builtins(),
    ) {
        refuse(
            &mut report,
            e.rule(),
            manifest_shown.clone(),
            format!(
                "pack {} {} is refused, and nothing is installed: {e}",
                manifest.name, manifest.version
            ),
        );
        return Ok((report, None));
    }

    let installed = Installed {
        pack: manifest.name.clone(),
        version: manifest.version.clone(),
        source: source.clone(),
        types: types.clone(),
        files: written.clone(),
        outcome: if dry_run {
            "would-install"
        } else {
            "installed"
        }
        .to_owned(),
    };
    if dry_run {
        report.push(Diagnostic::pass(
            "types.pack-installed",
            format!(
                "pack {} {} would install {}; nothing written",
                manifest.name,
                manifest.version,
                types.join(", ")
            ),
        ));
        return Ok((report, Some(installed)));
    }
    let mut index = doc_index::read_types(repo)
        .map_err(|f| RepoError::Message(format!("{}: {}: {}", f.rule, f.file, f.message)))?
        .unwrap_or_else(|| TypesFile {
            schema: TYPES_SCHEMA.to_owned(),
            ..TypesFile::default()
        });
    index.packs.retain(|p| p.name != manifest.name);
    index.packs.push(PackEntry {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        source,
        profiles: types.clone(),
        digest: pack_digest,
    });
    index.packs.sort_by(|a, b| a.name.cmp(&b.name));
    crate::vfs::create_dir_all(&profiles_dir).map_err(|source| RepoError::Io {
        context: format!("could not create {profiles_dir}"),
        source,
    })?;
    for (f, b) in &files {
        let path = profiles_dir.join(f);
        crate::vfs::write(&path, b).map_err(|source| RepoError::Io {
            context: format!("could not write {path}"),
            source,
        })?;
    }
    write_types(repo, &index)?;
    report.push(Diagnostic::pass(
        "types.pack-installed",
        format!(
            "pack {} {} installed: {} → profiles/; recorded in {TYPES_FILE}",
            manifest.name,
            manifest.version,
            types.join(", ")
        ),
    ));
    Ok((report, Some(installed)))
}

/// Write `docs/types.toml`.
fn write_types(repo: &Repository, index: &TypesFile) -> Result<(), RepoError> {
    let path = repo.root.join(TYPES_FILE);
    if let Some(parent) = path.parent() {
        crate::vfs::create_dir_all(parent).map_err(|source| RepoError::Io {
            context: format!("could not create {parent}"),
            source,
        })?;
    }
    let body = toml::to_string_pretty(index).map_err(|e| RepoError::Message(e.to_string()))?;
    let text = format!(
        "# The program's types (OW-WAR-0148 M18): the packs installed into profiles/\n\
         # (`war plan types add`) and the documents adopted in place (`war plan type`).\n\
         # An adoption names a document's type here and never changes the document.\n\
         {body}"
    );
    crate::vfs::write(&path, text).map_err(|source| RepoError::Io {
        context: format!("could not write {path}"),
        source,
    })
}

// ---- `war plan type <file> <type>` ---------------------------------------------

/// What `war plan type` did.
#[derive(Debug, Clone, Serialize)]
pub struct Adopted {
    pub path: String,
    #[serde(rename = "type")]
    pub doc_type: String,
    /// Its type before, if it was adopted as another.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub was: Option<String>,
    /// `adopted`, `retyped`, or `unchanged`.
    pub outcome: String,
    /// typed / total after the adoption.
    pub coverage: openwarrant_core::status::DocumentCoverage,
}

/// `war plan type <file> <type>`: record that `file` is a document of
/// `doc_type`, in `docs/types.toml`. The document's bytes are never read
/// for meaning and never written. Refused by rule, writing nothing: a file
/// that does not exist or lies outside the repository (`types.file-unknown`),
/// one a store's type, a record area or the instruction reader already
/// reads (`types.file-governed`), one under a `generated/`, Warrant or
/// ticket tree (`types.file-excluded`); a type that is not a document type
/// of this program (`types.type-unknown`), or one whose records are read
/// from a store (`types.type-not-adoptable`).
pub fn adopt(
    corpus: &crate::corpus::Corpus,
    file: &str,
    doc_type: &str,
) -> Result<(Report, Option<Adopted>), RepoError> {
    let repo = corpus.repo();
    let mut report = Report::default();
    let given = Utf8PathBuf::from(file);
    let absolute = if given.is_absolute() {
        given.clone()
    } else {
        std::env::current_dir()
            .ok()
            .and_then(|d| Utf8PathBuf::from_path_buf(d).ok())
            .map_or_else(|| repo.root.join(&given), |d| d.join(&given))
    };
    // Resolve against the root first when the path is relative and exists
    // there (`war --root <repo> plan type docs/x.md release`).
    let absolute = if !given.is_absolute() && crate::vfs::is_file(repo.root.join(&given)) {
        repo.root.join(&given)
    } else {
        absolute
    };
    let canon = std::fs::canonicalize(&absolute)
        .ok()
        .and_then(|p| Utf8PathBuf::from_path_buf(p).ok());
    let root = std::fs::canonicalize(&repo.root)
        .ok()
        .and_then(|p| Utf8PathBuf::from_path_buf(p).ok())
        .unwrap_or_else(|| repo.root.clone());
    let Some(rel) = canon
        .filter(|p| crate::vfs::is_file(p))
        .and_then(|p| p.strip_prefix(&root).ok().map(|r| r.as_str().to_owned()))
    else {
        refuse(
            &mut report,
            "types.file-unknown",
            file,
            format!(
                "{file} is no file of this repository; a document is adopted where it is, so it \
                 must exist under {}",
                repo.root
            ),
        );
        return Ok((report, None));
    };
    // The type: a document type of the program, whose records are not a
    // store's.
    let Some(profile) = repo.profiles.document(doc_type) else {
        let known: Vec<&str> = repo
            .profiles
            .documents()
            .filter(|d| d.encoding.is_none())
            .map(|d| d.name.as_str())
            .collect();
        refuse(
            &mut report,
            "types.type-unknown",
            doc_type,
            format!(
                "{doc_type:?} is no document type of this program (adoptable: {}); `war plan \
                 types` lists them, and `war plan types add <pack>` installs more",
                known.join(", ")
            ),
        );
        return Ok((report, None));
    };
    if let Some(store) = profile.encoding {
        refuse(
            &mut report,
            "types.type-not-adoptable",
            doc_type,
            format!(
                "{doc_type} is read from the {store} store, and a document is not added to a \
                 store by naming it; adopt it as a document type instead"
            ),
        );
        return Ok((report, None));
    }
    // The file: indexable, and governed by nothing else.
    let excluded = rel.split('/').any(|s| s == "generated")
        || rel.starts_with(&format!(
            "{}/",
            repo.config.paths.warrants.trim_end_matches('/')
        ))
        || crate::ticket::Store::open(repo, None)
            .ok()
            .is_some_and(|s| rel.starts_with(&format!("{}/", repo.relative(&s.dir))));
    if excluded {
        refuse(
            &mut report,
            "types.file-excluded",
            rel.clone(),
            format!(
                "{rel} is a projection or part of a Warrant or a ticket, which the document index \
                 never reads; adopt its source instead"
            ),
        );
        return Ok((report, None));
    }
    let index = corpus.documents();
    if let Some(d) = index.get(&rel)
        && d.governor.is_typed()
        && !matches!(d.governor, doc_index::Governor::Adopted(_))
    {
        let by = match &d.governor {
            doc_index::Governor::Store(t) => format!("the `{t}` type's store"),
            doc_index::Governor::Records => "its record area (docs/records/)".to_owned(),
            _ => "the instruction reader (M16)".to_owned(),
        };
        refuse(
            &mut report,
            "types.file-governed",
            rel.clone(),
            format!("{rel} is read as records already, by {by}; it is typed, and stays as it is"),
        );
        return Ok((report, None));
    }
    let mut types = doc_index::read_types(repo)
        .map_err(|f| RepoError::Message(format!("{}: {}: {}", f.rule, f.file, f.message)))?
        .unwrap_or_else(|| TypesFile {
            schema: TYPES_SCHEMA.to_owned(),
            ..TypesFile::default()
        });
    let was = types
        .documents
        .iter()
        .find(|a| a.path == rel)
        .map(|a| a.doc_type.clone());
    let before = index.coverage();
    if was.as_deref() == Some(doc_type) {
        report.push(Diagnostic::pass(
            "types.adopted",
            format!("{rel} is adopted as {doc_type} already; nothing written"),
        ));
        return Ok((
            report,
            Some(Adopted {
                path: rel,
                doc_type: doc_type.to_owned(),
                was,
                outcome: "unchanged".to_owned(),
                coverage: before,
            }),
        ));
    }
    types.documents.retain(|a| a.path != rel);
    types.documents.push(Adoption {
        path: rel.clone(),
        doc_type: doc_type.to_owned(),
    });
    types.documents.sort_by(|a, b| a.path.cmp(&b.path));
    // Only the index is written; the document itself never is.
    write_types(repo, &types)?;
    let indexed = index.get(&rel).is_some();
    let after = openwarrant_core::status::DocumentCoverage {
        typed: before.typed + usize::from(was.is_none()),
        untyped: before
            .untyped
            .saturating_sub(usize::from(indexed && was.is_none())),
        total: before.total + usize::from(!indexed),
    };
    report.push(Diagnostic::pass(
        "types.adopted",
        format!(
            "{rel} is adopted as {doc_type}{}: recorded in {TYPES_FILE}; the document is \
             unchanged. Documents typed: {} of {}",
            was.as_ref()
                .map(|w| format!(" (was {w})"))
                .unwrap_or_default(),
            after.typed,
            after.total
        ),
    ));
    Ok((
        report,
        Some(Adopted {
            path: rel,
            doc_type: doc_type.to_owned(),
            outcome: if was.is_some() { "retyped" } else { "adopted" }.to_owned(),
            was,
            coverage: after,
        }),
    ))
}

// ---- `war check` ---------------------------------------------------------------

/// `war check`'s rules over the index and `docs/types.toml`: each fault,
/// each adoption whose file is gone (`types.document-missing`) or whose
/// type the program no longer admits as a document type
/// (`types.type-unknown`), each installed pack whose types are gone
/// (`types.pack-missing`, a warning), and one pass naming the coverage.
/// Silent for a program with no `docs/types.toml` and a readable
/// `[documents]` table: its check is what it was.
pub fn check(corpus: &crate::corpus::Corpus, report: &mut Report) {
    let repo = corpus.repo();
    let index = corpus.documents();
    let mut refused = 0usize;
    for f in &index.faults {
        refused += 1;
        report.push(Diagnostic::error(f.rule, f.file.clone(), f.message.clone()));
    }
    let Some(types) = &index.types else {
        return;
    };
    for a in &types.documents {
        if !crate::vfs::is_file(repo.root.join(&a.path)) {
            refused += 1;
            report.push(Diagnostic::error(
                "types.document-missing",
                TYPES_FILE.to_owned(),
                format!(
                    "{} is adopted as {} and no longer exists; remove its entry, or restore it",
                    a.path, a.doc_type
                ),
            ));
        }
        match repo.profiles.document(&a.doc_type) {
            None => {
                refused += 1;
                report.push(Diagnostic::error(
                    "types.type-unknown",
                    TYPES_FILE.to_owned(),
                    format!(
                        "{} is adopted as {}, which is no document type of this program",
                        a.path, a.doc_type
                    ),
                ));
            }
            Some(p) if p.encoding.is_some() => {
                refused += 1;
                report.push(Diagnostic::error(
                    "types.type-not-adoptable",
                    TYPES_FILE.to_owned(),
                    format!(
                        "{} is adopted as {}, whose records are read from a store",
                        a.path, a.doc_type
                    ),
                ));
            }
            Some(_) => {}
        }
    }
    for p in &types.packs {
        let gone: Vec<&str> = p
            .profiles
            .iter()
            .filter(|n| {
                repo.profiles.document(n).is_none() && {
                    let n: &str = n;
                    !repo.profiles.names().contains(&n)
                }
            })
            .map(String::as_str)
            .collect();
        if !gone.is_empty() {
            report.push(Diagnostic::warn(
                "types.pack-missing",
                TYPES_FILE.to_owned(),
                format!(
                    "pack {} {} is recorded as installed, and this program admits none of {}",
                    p.name,
                    p.version,
                    gone.join(", ")
                ),
            ));
        }
    }
    if refused == 0 {
        let c = index.coverage();
        report.push(Diagnostic::pass(
            "types.well-formed",
            format!(
                "{} adopted document(s), {} pack(s); documents typed: {} of {}",
                types.documents.len(),
                types.packs.len(),
                c.typed,
                c.total
            ),
        ));
    }
}

/// A `war plan types add` or `war plan type` answer at a terminal: what was
/// done on stdout, a refusal or a warning on stderr, by rule.
pub fn print_human(report: &Report) {
    use crate::diagnostic::Severity;
    for d in &report.diagnostics {
        match d.severity {
            Severity::Error => eprintln!("refused ({}): {}", d.rule, d.message),
            Severity::Warn => eprintln!("warning ({}): {}", d.rule, d.message),
            Severity::Unknown => eprintln!("UNKNOWN ({}): {}", d.rule, d.message),
            _ => println!("{}", d.message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8Path;

    /// Every type this repository ships, and every pack, admitted together:
    /// no file refused, no name or projection collides.
    #[test]
    fn the_shipped_types_and_packs_are_admitted_together() {
        let root = Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        for dir in ["profiles", "packs/ops", "packs/quality"] {
            for e in std::fs::read_dir(root.join(dir)).unwrap().flatten() {
                let p = Utf8PathBuf::from_path_buf(e.path()).unwrap();
                if p.extension() == Some("toml") && p.file_name() != Some("pack.toml") {
                    files.push((
                        format!("{dir}/{}", p.file_name().unwrap()),
                        std::fs::read(&p).unwrap(),
                    ));
                }
            }
        }
        files.sort();
        let r = openwarrant_core::role::ProfileRegistry::with_builtins(
            files.iter().map(|(f, b)| (f.as_str(), b.as_slice())),
            builtins(),
        )
        .unwrap();
        for name in [
            "roadmap",
            "spec",
            "adr",
            "release",
            "incident",
            "exit-report",
            "runbook",
            "slo",
            "rollout",
            "incident-review",
            "threat-model",
            "performance-budget",
            "test-charter",
        ] {
            assert!(r.document(name).is_some(), "{name}");
        }
        assert_eq!(r.store_type(Store::Roadmap).unwrap().name, "roadmap");
        assert_eq!(r.store_type(Store::Sas).unwrap().name, "spec");
        assert_eq!(r.store_type(Store::Adr).unwrap().name, "adr");
    }

    /// The built-in bytes are the shipped files: one definition, not two.
    #[test]
    fn the_built_in_types_are_the_shipped_files() {
        let root = Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for (f, b) in BUILTIN {
            let name = f.rsplit('/').next().unwrap();
            let on_disk = std::fs::read_to_string(root.join("profiles").join(name)).unwrap();
            assert_eq!(on_disk, b, "{name}");
        }
    }
}
