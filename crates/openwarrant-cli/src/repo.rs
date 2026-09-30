// SPDX-License-Identifier: AGPL-3.0-or-later
//! Repository discovery and loading — the I/O half the core crate refuses (§79.1, §79.4).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use crate::contained_file::{
    canonical_utf8, read_contained_regular_bounded, reject_symlink_components,
};

pub(crate) use crate::contained_file::read_repository_regular_bounded;

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::{AtomSource, CompilationBasis, ScopeSource};
use openwarrant_core::{
    AdrError, AdrRecord, Manifest, RepositoryConfig, ValidatedManifest, frontmatter,
};

use openwarrant_core::authority::{AuthorityRegister, RoleAssignment};
use openwarrant_core::deliverable::Deliverable;
use openwarrant_core::verification::Verification;

use crate::diagnostic::{Diagnostic, Report};
use crate::init::CONFIG_FILE;

pub use crate::repo_error::RepoError;

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_ATOM_BYTES: u64 = 16 * 1024 * 1024;
const MAX_WARRANT_ATOM_BYTES: u64 = 32 * 1024 * 1024;
const MAX_WARRANT_ATOMS: usize = 1024;
const MAX_SCOPE_BYTES: u64 = 4 * 1024 * 1024;

/// An initialized OpenWarrant repository.
#[derive(Debug, Clone)]
pub struct Repository {
    pub root: Utf8PathBuf,
    pub config: RepositoryConfig,
}

impl Repository {
    /// Find the nearest ancestor containing `openwarrant.toml`.
    pub fn discover(start: Option<Utf8PathBuf>) -> Result<Self, RepoError> {
        let start = match start {
            Some(path) => path,
            None => {
                let cwd = std::env::current_dir().map_err(|source| RepoError::Io {
                    context: "could not read the current directory".to_owned(),
                    source,
                })?;
                Utf8PathBuf::from_path_buf(cwd).map_err(|_| RepoError::NonUtf8Path)?
            }
        };

        let mut cursor: &Utf8Path = &start;
        loop {
            let candidate = cursor.join(CONFIG_FILE);
            if candidate.is_file() {
                return Self::open(cursor.to_owned());
            }
            match cursor.parent() {
                Some(parent) => cursor = parent,
                None => return Err(RepoError::NotFound { from: start }),
            }
        }
    }

    /// Open a repository whose root is already known.
    pub fn open(root: Utf8PathBuf) -> Result<Self, RepoError> {
        let path = root.join(CONFIG_FILE);
        let text = fs::read_to_string(&path).map_err(|source| RepoError::Io {
            context: format!("could not read {path}"),
            source,
        })?;
        let config: RepositoryConfig =
            toml::from_str(&text).map_err(|source| RepoError::ConfigParse {
                path: path.clone(),
                source,
            })?;
        config
            .validate()
            .map_err(|source| RepoError::ConfigInvalid { path, source })?;
        Ok(Self { root, config })
    }

    /// The configured warrants directory.
    #[must_use]
    pub fn warrants_dir(&self) -> Utf8PathBuf {
        self.root.join(&self.config.paths.warrants)
    }

    /// The actor this tool acts as when it performs work (§27.1).
    ///
    /// Fixed to `claude`, and deliberately not configurable from the command
    /// line. The performer identity is what every self-* check compares
    /// against — self-verification (§46), self-authorization (§27.2),
    /// self-resolution (§27.3 condition 4). A flag that let the caller rename
    /// the performer would let it walk out of all three by claiming to be
    /// somebody else.
    #[must_use]
    pub fn performer(&self) -> String {
        "claude".to_owned()
    }

    /// Role assignments in force for this repository (§27.4).
    ///
    /// `docs/authority/roles.toml` is authored by a human and by nothing else.
    /// There is no `war authority grant`: a command that could write this file
    /// would let an agent assign itself the roles §27.2 exists to withhold, so
    /// the register is read-only to every tool in this workspace.
    ///
    /// An absent file yields an empty register, and an empty register grants
    /// nobody anything — the fail-closed direction.
    pub fn load_authority_register(&self) -> Result<AuthorityRegister, RepoError> {
        let path = self.root.join("docs/authority/roles.toml");
        if !path.is_file() {
            return Ok(AuthorityRegister::default());
        }
        let text = fs::read_to_string(&path).map_err(|source| RepoError::Io {
            context: format!("could not read {path}"),
            source,
        })?;

        #[derive(serde::Deserialize)]
        struct File {
            #[serde(default)]
            assignment: Vec<RoleAssignment>,
        }

        let file: File = toml::from_str(&text)
            .map_err(|e| RepoError::Message(format!("could not parse {path}: {e}")))?;

        // A malformed assignment is refused for the whole file rather than
        // skipped. Skipping would silently drop authority, and an actor whose
        // grant quietly vanished reads identically to one that never had it.
        for assignment in &file.assignment {
            assignment
                .validate()
                .map_err(|e| RepoError::Message(format!("{path}: {e}")))?;
        }
        Ok(AuthorityRegister::new(file.assignment))
    }

    /// The persisted authorization for one Warrant (§28.4), if any.
    ///
    /// A malformed record is an error, never an absent one: "this Warrant was
    /// never authorized" and "its authorization would not parse" must not
    /// report identically, because only one of them is safe to work around.
    pub fn load_authorization(
        &self,
        dir: &Utf8Path,
    ) -> Result<Option<crate::authorize::AuthorizationRecord>, RepoError> {
        let path = dir.join("authorization.toml");
        if !path.is_file() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path).map_err(|source| RepoError::Io {
            context: format!("could not read {path}"),
            source,
        })?;
        toml::from_str(&text)
            .map(Some)
            .map_err(|e| RepoError::Message(format!("could not parse {path}: {e}")))
    }

    /// Judgments recorded for one Warrant (§42).
    pub fn load_judgments(
        &self,
        dir: &Utf8Path,
    ) -> Result<Vec<openwarrant_core::Judgment>, RepoError> {
        let path = dir.join("judgments.toml");
        if !path.is_file() {
            return Ok(vec![]);
        }
        let text = fs::read_to_string(&path).map_err(|source| RepoError::Io {
            context: format!("could not read {path}"),
            source,
        })?;
        toml::from_str::<crate::authorize::JudgmentRecord>(&text)
            .map(|r| r.judgment)
            .map_err(|e| RepoError::Message(format!("could not parse {path}: {e}")))
    }

    /// Assumptions declared for one Warrant (§36), if the sidecar exists.
    ///
    /// `Ok(None)` means no `rationale.toml` — the question was never asked.
    /// `Ok(Some(vec![]))` means it was asked and the answer was none. The
    /// resolver treats those differently and would be unsound if it could not
    /// tell them apart (Law 15: Unknown is neither failure nor pass).
    pub fn load_rationale(
        &self,
        dir: &Utf8Path,
    ) -> Result<Option<Vec<openwarrant_core::rationale::Assumption>>, RepoError> {
        let path = dir.join("rationale.toml");
        if !path.is_file() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path).map_err(|source| RepoError::Io {
            context: format!("could not read {path}"),
            source,
        })?;

        #[derive(serde::Deserialize)]
        struct File {
            #[serde(default)]
            assumption: Vec<openwarrant_core::rationale::Assumption>,
        }

        let file: File = toml::from_str(&text)
            .map_err(|e| RepoError::Message(format!("could not parse {path}: {e}")))?;
        for assumption in &file.assumption {
            assumption
                .validate()
                .map_err(|e| RepoError::Message(format!("{path}: {} — {e}", assumption.id)))?;
        }
        Ok(Some(file.assumption))
    }

    /// Every Warrant directory, sorted by name.
    ///
    /// A directory is a Warrant because it contains a `manifest.toml`, not
    /// because of what it is called — §59: "Semantics are not inferred from
    /// paths alone."
    pub fn warrant_dirs(&self) -> Result<Vec<Utf8PathBuf>, RepoError> {
        let dir = self.warrants_dir();
        if !dir.is_dir() {
            return Ok(vec![]);
        }
        let mut out = Vec::new();
        let entries = fs::read_dir(&dir).map_err(|source| RepoError::Io {
            context: format!("could not read {dir}"),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| RepoError::Io {
                context: format!("could not read an entry in {dir}"),
                source,
            })?;
            let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
                continue;
            };
            if path.join("manifest.toml").is_file() {
                out.push(path);
            }
        }
        out.sort();
        Ok(out)
    }

    /// Resolve a Warrant directory by local alias.
    pub fn warrant_dir(&self, alias: &str) -> Result<Utf8PathBuf, RepoError> {
        let dirs = self.warrant_dirs()?;
        for dir in &dirs {
            if dir.file_name() == Some(alias) {
                return Ok(dir.clone());
            }
        }
        Err(RepoError::UnknownWarrant {
            alias: alias.to_owned(),
            known: dirs
                .iter()
                .filter_map(|d| d.file_name().map(str::to_owned))
                .collect(),
        })
    }

    /// Load one Warrant into a Compilation Basis.
    ///
    /// Reads the manifest, validates it, then reads every declared atom's exact
    /// bytes. Missing atoms and unreadable frontmatter become diagnostics rather
    /// than hard failures, so `war check` can report EVERY problem in one run
    /// instead of stopping at the first — a checker that stops early makes the
    /// author re-run it once per defect.
    pub fn load_warrant(&self, dir: &Utf8Path) -> Result<Loaded, RepoError> {
        let (repository_root, warrant_root) = self.safe_warrant_roots(dir)?;
        let manifest_path = dir.join("manifest.toml");
        let manifest_bytes = read_contained_regular_bounded(
            &manifest_path,
            &repository_root,
            &warrant_root,
            "Warrant root",
            "manifest",
            MAX_MANIFEST_BYTES,
        )?
        .bytes;
        let manifest_text = String::from_utf8_lossy(&manifest_bytes).into_owned();
        let manifest: Manifest =
            toml::from_str(&manifest_text).map_err(|source| RepoError::ManifestParse {
                path: manifest_path.clone(),
                source,
            })?;

        let mut report = Report::default();
        let relative_manifest = self.relative(&manifest_path);

        let validated = match manifest.validate(Some(self.config.project.namespace.as_str())) {
            Ok(v) => v,
            Err(source) => {
                report.push(Diagnostic::error(
                    "manifest.invalid",
                    relative_manifest.clone(),
                    source.to_string(),
                ));
                return Ok(Loaded {
                    dir: dir.to_owned(),
                    basis: None,
                    validated: None,
                    report,
                });
            }
        };

        if manifest.atoms.len() > MAX_WARRANT_ATOMS {
            report.push(Diagnostic::error(
                "manifest.resource-limit",
                relative_manifest.clone(),
                format!(
                    "manifest declares {} atoms, exceeding {MAX_WARRANT_ATOMS} atom limit",
                    manifest.atoms.len()
                ),
            ));
            return Ok(Loaded {
                dir: dir.to_owned(),
                basis: None,
                validated: Some(validated),
                report,
            });
        }

        let mut atoms = Vec::new();
        let mut atom_sources = BTreeSet::new();
        let mut atom_target_sources = BTreeMap::new();
        let mut atom_bytes = 0_u64;
        let mut adr_atoms_root: Option<Result<Utf8PathBuf, String>> = None;
        for entry in &manifest.atoms {
            let Some(rel) = entry.path.as_deref() else {
                // A `ref =` atom is bound to an authority we cannot resolve
                // offline. Not an error and not a pass (Law 15).
                report.push(Diagnostic::unknown(
                    "atom.bound-unresolvable",
                    relative_manifest.clone(),
                    format!(
                        "atom at ordinal {} is bound by `ref` and cannot be resolved \
                         offline; federation is not implemented",
                        entry.ordinal
                    ),
                ));
                continue;
            };

            let is_adr_source = entry.role == "adr";
            let path_validation = if is_adr_source {
                validate_adr_relative_source(rel, "ADR atom source")
            } else {
                validate_safe_relative_source(rel, "atom source")
            };
            if let Err(error) = path_validation {
                report.push(Diagnostic::error(
                    "atom.path-unsafe",
                    relative_manifest.clone(),
                    format!("declared at ordinal {}: {error}", entry.ordinal),
                ));
                continue;
            }

            if !atom_sources.insert(rel) {
                report.push(Diagnostic::error(
                    "atom.path-duplicate",
                    relative_manifest.clone(),
                    format!(
                        "atom source {rel:?} is declared more than once; duplicate sources are refused"
                    ),
                ));
                continue;
            }

            let (source_root, source_root_label) = if is_adr_source {
                if adr_atoms_root.is_none() {
                    adr_atoms_root = Some(
                        self.safe_adr_atoms_root(&repository_root)
                            .map_err(|error| error.to_string()),
                    );
                }
                match adr_atoms_root.as_ref().expect("ADR root state initialized") {
                    Ok(root) => (root.as_path(), "ADR atoms root"),
                    Err(error) => {
                        report.push(Diagnostic::error(
                            "atom.unreadable",
                            relative_manifest.clone(),
                            format!("declared at ordinal {}: {error}", entry.ordinal),
                        ));
                        continue;
                    }
                }
            } else {
                (warrant_root.as_path(), "Warrant root")
            };

            let remaining_atom_bytes = MAX_WARRANT_ATOM_BYTES
                .checked_sub(atom_bytes)
                .expect("loaded atom bytes never exceed their enforced budget");
            if remaining_atom_bytes == 0 {
                report.push(Diagnostic::error(
                    "atom.resource-limit",
                    relative_manifest.clone(),
                    format!(
                        "declared at ordinal {}: Warrant atom bytes exceed {MAX_WARRANT_ATOM_BYTES} byte aggregate limit",
                        entry.ordinal
                    ),
                ));
                continue;
            }

            let path = if is_adr_source {
                match resolve_adr_source_path(&warrant_root, rel, &repository_root) {
                    Ok(path) => path,
                    Err(error) => {
                        report.push(Diagnostic::error(
                            "atom.path-unsafe",
                            relative_manifest.clone(),
                            format!("declared at ordinal {}: {error}", entry.ordinal),
                        ));
                        continue;
                    }
                }
            } else {
                dir.join(rel)
            };
            let read = match read_contained_regular_bounded(
                &path,
                &repository_root,
                source_root,
                source_root_label,
                "atom",
                MAX_ATOM_BYTES.min(remaining_atom_bytes),
            ) {
                Ok(bytes) => bytes,
                Err(error) => {
                    let rule = match &error {
                        RepoError::Io { source, .. }
                            if source.kind() == std::io::ErrorKind::NotFound =>
                        {
                            "atom.missing"
                        }
                        _ => "atom.unreadable",
                    };
                    report.push(Diagnostic::error(
                        rule,
                        self.relative(&path),
                        format!("declared at ordinal {}: {error}", entry.ordinal),
                    ));
                    continue;
                }
            };
            if let Some(first_source) = atom_target_sources.get(&read.identity) {
                report.push(Diagnostic::error(
                    "atom.path-duplicate",
                    relative_manifest.clone(),
                    format!(
                        "atom source {rel:?} resolves to {}, the same opened file identity already declared by {first_source:?}; duplicate targets are refused",
                        read.canonical
                    ),
                ));
                continue;
            }
            atom_target_sources.insert(read.identity, rel.to_owned());
            let bytes = read.bytes;
            atom_bytes = atom_bytes
                .checked_add(bytes.len() as u64)
                .expect("atom byte budget is below u64::MAX");

            // Jurisdiction comes from the atom's own frontmatter when it has
            // one; a `.yaml` structured atom (§62.1) has none, and that is not
            // a defect.
            let text = String::from_utf8_lossy(&bytes);
            let jurisdiction = match frontmatter::parse(&text) {
                Ok(fm) => fm.scalar("jurisdiction").unwrap_or("authored").to_owned(),
                Err(err) => {
                    if rel.ends_with(".md") {
                        report.push(Diagnostic::error(
                            "atom.frontmatter",
                            self.relative(&path),
                            err.to_string(),
                        ));
                    }
                    "authored".to_owned()
                }
            };

            atoms.push(AtomSource {
                ordinal: entry.ordinal,
                role: entry.role.clone(),
                jurisdiction,
                source: rel.to_owned(),
                bytes,
                required: entry.required,
            });
        }

        let scope_path = dir.join("scope.toml");
        let scope = match fs::symlink_metadata(&scope_path) {
            Ok(_) => match read_contained_regular_bounded(
                &scope_path,
                &repository_root,
                &warrant_root,
                "Warrant root",
                "scope",
                MAX_SCOPE_BYTES,
            ) {
                Ok(read) => Some(ScopeSource {
                    source: self.relative(&scope_path),
                    bytes: read.bytes,
                }),
                Err(source) => {
                    report.push(Diagnostic::error(
                        "scope.unreadable",
                        self.relative(&scope_path),
                        source.to_string(),
                    ));
                    None
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                report.push(Diagnostic::error(
                    "scope.unreadable",
                    self.relative(&scope_path),
                    error.to_string(),
                ));
                None
            }
        };

        Ok(Loaded {
            dir: dir.to_owned(),
            basis: Some(CompilationBasis {
                manifest,
                manifest_source: relative_manifest,
                manifest_bytes,
                atoms,
                scope,
            }),
            validated: Some(validated),
            report,
        })
    }

    fn safe_warrant_roots(&self, dir: &Utf8Path) -> Result<(Utf8PathBuf, Utf8PathBuf), RepoError> {
        validate_safe_relative_source(&self.config.paths.warrants, "configured warrants path")?;

        let configured_warrants = self.warrants_dir();
        let warrant_relative = dir.strip_prefix(&configured_warrants).map_err(|_| {
            RepoError::Message(format!(
                "Warrant directory {dir} is outside configured warrants directory {configured_warrants}"
            ))
        })?;
        validate_safe_relative_source(warrant_relative.as_str(), "Warrant directory")?;

        let repository_root = canonical_utf8(&self.root, "repository root")?;
        let warrants_root = canonical_utf8(&configured_warrants, "configured warrants root")?;
        if !warrants_root.starts_with(&repository_root) {
            return Err(RepoError::Message(format!(
                "configured warrants root {warrants_root} escapes repository root {repository_root}"
            )));
        }
        if !fs::metadata(&warrants_root)
            .map_err(|source| RepoError::Io {
                context: format!("could not inspect configured warrants root {warrants_root}"),
                source,
            })?
            .is_dir()
        {
            return Err(RepoError::Message(format!(
                "configured warrants root {warrants_root} is not a directory"
            )));
        }

        let warrant_root = canonical_utf8(dir, "Warrant root")?;
        if !warrant_root.starts_with(&warrants_root) || warrant_root == warrants_root {
            return Err(RepoError::Message(format!(
                "Warrant root {warrant_root} is not contained by configured warrants root {warrants_root}"
            )));
        }
        if !fs::metadata(&warrant_root)
            .map_err(|source| RepoError::Io {
                context: format!("could not inspect Warrant root {warrant_root}"),
                source,
            })?
            .is_dir()
        {
            return Err(RepoError::Message(format!(
                "Warrant root {warrant_root} is not a directory"
            )));
        }

        Ok((repository_root, warrant_root))
    }

    fn safe_adr_atoms_root(&self, repository_root: &Utf8Path) -> Result<Utf8PathBuf, RepoError> {
        validate_safe_relative_source(&self.config.paths.adrs, "configured ADR path")?;
        let adr_atoms_root = canonical_utf8(&self.adr_atoms_dir(), "configured ADR atoms root")?;
        if !adr_atoms_root.starts_with(repository_root) {
            return Err(RepoError::Message(format!(
                "configured ADR atoms root {adr_atoms_root} escapes repository root {repository_root}"
            )));
        }
        if !fs::metadata(&adr_atoms_root)
            .map_err(|source| RepoError::Io {
                context: format!("could not inspect ADR atoms root {adr_atoms_root}"),
                source,
            })?
            .is_dir()
        {
            return Err(RepoError::Message(format!(
                "configured ADR atoms root {adr_atoms_root} is not a directory"
            )));
        }
        Ok(adr_atoms_root)
    }

    /// The configured ADR atoms directory.
    #[must_use]
    pub fn adr_atoms_dir(&self) -> Utf8PathBuf {
        self.root.join(&self.config.paths.adrs).join("atoms")
    }

    /// Where the generated Warrant Overview is written (§17.5 `status`).
    #[must_use]
    pub fn warrant_overview_path(&self) -> Utf8PathBuf {
        self.root
            .join(&self.config.paths.warrants)
            .join("generated")
            .join("WARRANT_OVERVIEW.md")
    }

    /// Where the generated ADR Overview is written (§19.6).
    #[must_use]
    pub fn adr_overview_path(&self) -> Utf8PathBuf {
        self.root
            .join(&self.config.paths.adrs)
            .join("generated")
            .join("ADR_OVERVIEW.md")
    }

    /// Load every ADR atom, plus whatever failed to parse.
    ///
    /// Returns parse failures rather than aborting, so `war check` reports every
    /// malformed ADR in one run instead of one per invocation.
    pub fn load_adrs(&self) -> Result<AdrCorpus, RepoError> {
        let dir = self.adr_atoms_dir();
        if !dir.is_dir() {
            return Ok(AdrCorpus::default());
        }
        let mut paths = Vec::new();
        for entry in fs::read_dir(&dir).map_err(|source| RepoError::Io {
            context: format!("could not read {dir}"),
            source,
        })? {
            let entry = entry.map_err(|source| RepoError::Io {
                context: format!("could not read an entry in {dir}"),
                source,
            })?;
            let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
                continue;
            };
            if path.extension() == Some("md") {
                paths.push(path);
            }
        }
        // Deterministic order: the overview must not depend on readdir order.
        paths.sort();

        let mut records = Vec::new();
        let mut failures = Vec::new();
        for path in paths {
            let relative = self.relative(&path);
            let text = fs::read_to_string(&path).map_err(|source| RepoError::Io {
                context: format!("could not read {path}"),
                source,
            })?;
            match AdrRecord::parse(&relative, &text) {
                Ok(record) => records.push(record),
                Err(err) => failures.push((relative, err)),
            }
        }
        Ok(AdrCorpus { records, failures })
    }

    /// Verification records for one Warrant (§46, §38.5).
    ///
    /// Each `verifications/*.toml` is one obligation verified by one verifier.
    /// A missing directory is not an error — it means nothing has been verified,
    /// which is a true and common state, distinct from a verification that
    /// failed to parse.
    pub fn load_verifications(&self, dir: &Utf8Path) -> Result<VerificationSet, RepoError> {
        let vdir = dir.join("verifications");
        if !vdir.is_dir() {
            return Ok(VerificationSet::default());
        }
        let mut paths = Vec::new();
        for entry in fs::read_dir(&vdir).map_err(|source| RepoError::Io {
            context: format!("could not read {vdir}"),
            source,
        })? {
            let entry = entry.map_err(|source| RepoError::Io {
                context: format!("could not read an entry in {vdir}"),
                source,
            })?;
            let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
                continue;
            };
            if path.extension() == Some("toml") {
                paths.push(path);
            }
        }
        paths.sort();

        let mut records = Vec::new();
        let mut failures = Vec::new();
        for path in paths {
            let relative = self.relative(&path);
            let text = fs::read_to_string(&path).map_err(|source| RepoError::Io {
                context: format!("could not read {path}"),
                source,
            })?;
            match toml::from_str::<Verification>(&text) {
                Ok(v) => records.push(v),
                Err(e) => failures.push((relative, e.to_string())),
            }
        }
        Ok(VerificationSet { records, failures })
    }

    /// Deliverables declared for one Warrant (§37).
    ///
    /// A single `deliverables.toml` rather than a directory: a Warrant declares a
    /// handful, they are read together, and one file keeps them reviewable as a
    /// set.
    pub fn load_deliverables(&self, dir: &Utf8Path) -> Result<DeliverableSet, RepoError> {
        let path = dir.join("deliverables.toml");
        if !path.is_file() {
            return Ok(DeliverableSet::default());
        }
        let text = fs::read_to_string(&path).map_err(|source| RepoError::Io {
            context: format!("could not read {path}"),
            source,
        })?;

        #[derive(serde::Deserialize)]
        struct File {
            #[serde(default)]
            deliverable: Vec<Deliverable>,
        }

        match toml::from_str::<File>(&text) {
            Ok(f) => Ok(DeliverableSet {
                records: f.deliverable,
                failures: vec![],
            }),
            Err(e) => Ok(DeliverableSet {
                records: vec![],
                failures: vec![(self.relative(&path), e.to_string())],
            }),
        }
    }

    /// A repository-relative path, for diagnostics and for the IR.
    ///
    /// Absolute paths must never reach the IR: they would make a digest depend
    /// on where the repository happens to be checked out.
    #[must_use]
    pub fn relative(&self, path: &Utf8Path) -> String {
        match path.strip_prefix(&self.root) {
            Ok(relative) => join_git_path_components(relative.iter()),
            // This branch is defensive: callers should only feed repository
            // paths, but diagnostics must still use Git-style separators if a
            // foreign path reaches the boundary on Windows.
            Err(_) => path.as_str().replace('\\', "/"),
        }
    }
}

fn join_git_path_components<'a>(components: impl IntoIterator<Item = &'a str>) -> String {
    let mut joined = String::new();
    for component in components {
        if !joined.is_empty() {
            joined.push('/');
        }
        joined.push_str(component);
    }
    joined
}

fn validate_safe_relative_source(path: &str, label: &str) -> Result<(), RepoError> {
    let windows_drive = path.as_bytes().get(1) == Some(&b':')
        && path.as_bytes().first().is_some_and(u8::is_ascii_alphabetic);
    if path.is_empty()
        || Utf8Path::new(path).is_absolute()
        || windows_drive
        || path.contains('\\')
        || path.contains(':')
        || path.split('/').any(|component| {
            component.is_empty()
                || component == "."
                || component == ".."
                || component.ends_with('.')
                || component.ends_with(' ')
        })
    {
        return Err(RepoError::Message(format!(
            "{label} {path:?} is unsafe; expected a non-empty relative path with no empty, '.', '..', backslash, colon, trailing-dot, or trailing-space components"
        )));
    }
    Ok(())
}

fn validate_adr_relative_source(path: &str, label: &str) -> Result<(), RepoError> {
    let windows_drive = path.as_bytes().get(1) == Some(&b':')
        && path.as_bytes().first().is_some_and(u8::is_ascii_alphabetic);
    if path.is_empty()
        || Utf8Path::new(path).is_absolute()
        || windows_drive
        || path.contains('\\')
        || path.contains(':')
        || path.split('/').any(|component| {
            component.is_empty()
                || component == "."
                || (component != ".." && (component.ends_with('.') || component.ends_with(' ')))
        })
    {
        return Err(RepoError::Message(format!(
            "{label} {path:?} is unsafe; expected a non-empty relative path with no empty, '.', backslash, colon, trailing-dot, or trailing-space components"
        )));
    }
    Ok(())
}

/// Resolve an ADR source containing parent components without handing `..` to
/// filesystem traversal. Before removing a child component, prove its current
/// path exists as a real directory with no symlink or reparse-point component;
/// this keeps lexical normalization equivalent to filesystem traversal.
fn resolve_adr_source_path(
    warrant_root: &Utf8Path,
    source: &str,
    repository_root: &Utf8Path,
) -> Result<Utf8PathBuf, RepoError> {
    if !warrant_root.starts_with(repository_root) {
        return Err(RepoError::Message(format!(
            "Warrant root {warrant_root} is outside repository root {repository_root}"
        )));
    }

    let mut resolved = warrant_root.to_owned();
    for component in source.split('/') {
        if component == ".." {
            if resolved == repository_root {
                return Err(RepoError::Message(format!(
                    "ADR atom source {source:?} escapes repository root {repository_root}"
                )));
            }
            reject_symlink_components(&resolved, repository_root, "ADR atom source traversal")?;
            let metadata = fs::metadata(&resolved).map_err(|source_error| RepoError::Io {
                context: format!(
                    "could not inspect ADR atom source traversal component {resolved}"
                ),
                source: source_error,
            })?;
            if !metadata.is_dir() {
                return Err(RepoError::Message(format!(
                    "ADR atom source {source:?} traverses parent from non-directory {resolved}"
                )));
            }
            if !resolved.pop() || !resolved.starts_with(repository_root) {
                return Err(RepoError::Message(format!(
                    "ADR atom source {source:?} escapes repository root {repository_root}"
                )));
            }
        } else {
            resolved.push(component);
        }
    }
    Ok(resolved)
}

#[cfg(all(test, unix))]
use crate::contained_file::open_capability_candidate;
#[cfg(all(test, windows))]
use crate::contained_file::validate_regular_size;
#[cfg(test)]
use crate::contained_file::{OpenedPathExpectation, open_regular_candidate, verify_opened_path};
#[cfg(all(test, target_os = "linux"))]
use crate::contained_file::{with_test_after_first_read_replacement, with_test_linux_proc_fd_root};

/// The ADR corpus as read from disk.
///
/// Parse failures travel alongside the records rather than replacing them: one
/// malformed ADR must not hide the other twenty, and `war check` reports every
/// problem in a single run.
#[derive(Debug, Default)]
pub struct AdrCorpus {
    pub records: Vec<AdrRecord>,
    /// `(repository-relative path, why it would not parse)`.
    pub failures: Vec<(String, AdrError)>,
}

/// Deliverables for one Warrant, and a parse failure if the file would not read.
///
/// A malformed `deliverables.toml` yields NO records and a recorded failure,
/// never silently zero deliverables — "the file is broken" and "this Warrant
/// declares none" are different states and must not report identically.
#[derive(Debug, Default)]
pub struct DeliverableSet {
    pub records: Vec<Deliverable>,
    pub failures: Vec<(String, String)>,
}

/// Verification records for one Warrant, and the ones that would not parse.
///
/// Failures travel alongside the records for the same reason `AdrCorpus` does:
/// an unreadable verification is not an absent one, and silently treating it as
/// absent would turn a malformed record into "nothing was verified" — which
/// reads identically to the honest state and is not.
#[derive(Debug, Default)]
pub struct VerificationSet {
    pub records: Vec<Verification>,
    /// `(repository-relative path, why it would not parse)`.
    pub failures: Vec<(String, String)>,
}

/// A Warrant read from disk, with whatever went wrong while reading it.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub dir: Utf8PathBuf,
    /// `None` when the manifest itself was invalid, so nothing downstream can
    /// be trusted.
    pub basis: Option<CompilationBasis>,
    pub validated: Option<ValidatedManifest>,
    pub report: Report,
}

impl Loaded {
    /// The local alias, taken from the directory name when the manifest could
    /// not be validated.
    #[must_use]
    pub fn alias(&self) -> String {
        self.validated
            .as_ref()
            .map(|v| v.alias.to_string())
            .or_else(|| self.dir.file_name().map(str::to_owned))
            .unwrap_or_else(|| self.dir.to_string())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::fs::File;
    #[cfg(windows)]
    use std::fs::OpenOptions;

    use openwarrant_core::Namespace;

    use super::*;

    struct TestRepository {
        _scratch: tempfile::TempDir,
        repo: Repository,
        warrant: Utf8PathBuf,
    }

    impl TestRepository {
        fn new() -> Self {
            let scratch = tempfile::Builder::new()
                .prefix("openwarrant-safe-loader-")
                .tempdir()
                .expect("create unique temporary repository");
            let root = Utf8PathBuf::from_path_buf(scratch.path().to_owned())
                .expect("UTF-8 temporary directory");
            let warrant = root.join("docs/warrants/OW-WAR-9999");
            fs::create_dir_all(warrant.join("atoms")).expect("create test Warrant");
            fs::create_dir_all(root.join("docs/adr/atoms")).expect("create test ADR tree");

            let repo = Repository {
                root,
                config: RepositoryConfig::new(
                    "safe-loader-test",
                    Namespace::parse("OW").expect("valid namespace"),
                ),
            };
            let fixture = Self {
                _scratch: scratch,
                repo,
                warrant,
            };
            fixture.write_manifest("atoms/10-intent.md");
            fixture.write_atoms();
            fixture
        }

        fn write_manifest(&self, intent_path: &str) {
            fs::write(
                self.warrant.join("manifest.toml"),
                manifest_with_intent_path(intent_path),
            )
            .expect("write manifest");
        }

        fn write_atoms(&self) {
            for (name, role, order) in [
                ("10-intent.md", "intent", 10),
                ("20-basis.md", "basis", 20),
                ("40-work-order.md", "work_order", 40),
                ("60-assurance.md", "assurance", 60),
            ] {
                fs::write(
                    self.warrant.join("atoms").join(name),
                    markdown_atom(role, order),
                )
                .expect("write atom");
            }
            fs::write(
                self.warrant.join("atoms/45-milestones.yaml"),
                "milestones: []\n",
            )
            .expect("write milestones");
        }

        fn load(&self) -> Result<Loaded, RepoError> {
            self.repo.load_warrant(&self.warrant)
        }
    }

    fn markdown_atom(role: &str, order: u32) -> String {
        format!(
            "---\nschema: oh.war/atom/v1\nwarrant_uuid: \
             01a018db-19fc-7f2a-8e39-69730f255e33\nrole: {role}\n\
             jurisdiction: authored\norder: {order}\nclassification: internal\n---\n\n# Test\n"
        )
    }

    fn manifest_with_intent_path(intent_path: &str) -> String {
        format!(
            r#"schema = "oh.war/manifest/v1"
uuid = "01a018db-19fc-7f2a-8e39-69730f255e33"
local_alias = "OW-WAR-9999"
enterprise_id = ""
title = "Safe loader fixture"
profile = "delivery"

[[atoms]]
ordinal = 10
role = "intent"
path = {intent_path:?}
required = true

[[atoms]]
ordinal = 20
role = "basis"
path = "atoms/20-basis.md"
required = true

[[atoms]]
ordinal = 40
role = "work_order"
path = "atoms/40-work-order.md"
required = true

[[atoms]]
ordinal = 45
role = "milestones"
path = "atoms/45-milestones.yaml"
required = true

[[atoms]]
ordinal = 60
role = "assurance"
path = "atoms/60-assurance.md"
required = true
"#
        )
    }

    fn manifest_with_extra_atom(role: &str, path: &str) -> String {
        let mut manifest = manifest_with_intent_path("atoms/10-intent.md");
        manifest.push_str(&format!(
            "\n[[atoms]]\nordinal = 30\nrole = {role:?}\npath = {path:?}\nrequired = false\n"
        ));
        manifest
    }

    fn has_diagnostic(loaded: &Loaded, rule: &str, message_fragment: &str) -> bool {
        loaded.report.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule == rule && diagnostic.message.contains(message_fragment)
        })
    }

    #[test]
    fn valid_warrant_reads_all_declared_atoms() {
        let fixture = TestRepository::new();
        let loaded = fixture.load().expect("valid Warrant loads");
        assert_eq!(loaded.basis.expect("basis").atoms.len(), 5);
        assert!(loaded.report.diagnostics.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn adr_role_may_resolve_parent_components_into_configured_adr_atoms() {
        let fixture = TestRepository::new();
        let adr_path = fixture.repo.root.join("docs/adr/atoms/OW-ADR-9999-test.md");
        fs::write(&adr_path, markdown_atom("adr", 30)).expect("write ADR atom");
        fs::write(
            fixture.warrant.join("manifest.toml"),
            manifest_with_extra_atom("adr", "../../adr/atoms/OW-ADR-9999-test.md"),
        )
        .expect("write manifest");

        let loaded = fixture.load().expect("bound ADR source loads");
        assert!(!has_diagnostic(&loaded, "atom.path-unsafe", ""));
        assert!(!has_diagnostic(&loaded, "atom.unreadable", ""));
        let basis = loaded.basis.expect("basis");
        assert!(basis.atoms.iter().any(|atom| {
            atom.role == "adr" && atom.source == "../../adr/atoms/OW-ADR-9999-test.md"
        }));
    }

    #[cfg(unix)]
    #[test]
    fn adr_role_may_normalize_parent_after_a_real_child_directory() {
        let fixture = TestRepository::new();
        let adr_path = fixture.repo.root.join("docs/adr/atoms/OW-ADR-9999-test.md");
        fs::write(&adr_path, markdown_atom("adr", 30)).expect("write ADR atom");
        fs::write(
            fixture.warrant.join("manifest.toml"),
            manifest_with_extra_atom("adr", "../../adr/atoms/../atoms/OW-ADR-9999-test.md"),
        )
        .expect("write manifest");

        let loaded = fixture.load().expect("normalized ADR source loads");
        assert!(!has_diagnostic(&loaded, "atom.path-unsafe", ""));
        assert!(!has_diagnostic(&loaded, "atom.unreadable", ""));
        assert!(loaded.basis.expect("basis").atoms.iter().any(|atom| {
            atom.role == "adr" && atom.source == "../../adr/atoms/../atoms/OW-ADR-9999-test.md"
        }));
    }

    #[cfg(unix)]
    #[test]
    fn adr_aliases_to_same_opened_file_identity_are_refused() {
        let fixture = TestRepository::new();
        let adr_path = fixture.repo.root.join("docs/adr/atoms/OW-ADR-9999-test.md");
        fs::write(&adr_path, markdown_atom("adr", 30)).expect("write ADR atom");
        let mut manifest = manifest_with_extra_atom("adr", "../../adr/atoms/OW-ADR-9999-test.md");
        manifest.push_str(
            r#"
[[atoms]]
ordinal = 31
role = "adr"
path = "../../../docs/adr/atoms/OW-ADR-9999-test.md"
required = false
"#,
        );
        fs::write(fixture.warrant.join("manifest.toml"), manifest).expect("write manifest");

        let loaded = fixture.load().expect("ADR alias is a diagnostic");
        assert!(has_diagnostic(
            &loaded,
            "atom.path-duplicate",
            "same opened file identity"
        ));
        assert_eq!(
            loaded
                .basis
                .expect("remaining basis")
                .atoms
                .iter()
                .filter(|atom| atom.role == "adr")
                .count(),
            1
        );
    }

    #[cfg(unix)]
    #[test]
    fn parent_components_remain_forbidden_for_non_adr_roles() {
        let fixture = TestRepository::new();
        fs::write(
            fixture.repo.root.join("docs/adr/atoms/OW-ADR-9999-test.md"),
            markdown_atom("adr", 30),
        )
        .expect("write ADR atom");
        fs::write(
            fixture.warrant.join("manifest.toml"),
            manifest_with_extra_atom("validation", "../../adr/atoms/OW-ADR-9999-test.md"),
        )
        .expect("write manifest");

        let loaded = fixture
            .load()
            .expect("unsafe ordinary source is diagnostic");
        assert!(has_diagnostic(&loaded, "atom.path-unsafe", "'..'"));
        assert!(
            !loaded
                .basis
                .expect("remaining basis")
                .atoms
                .iter()
                .any(|atom| atom.ordinal == 30)
        );
    }

    #[cfg(unix)]
    #[test]
    fn adr_role_cannot_escape_configured_adr_atoms_root() {
        let fixture = TestRepository::new();
        fs::write(
            fixture.repo.root.join("docs/outside-adr-root.md"),
            markdown_atom("adr", 30),
        )
        .expect("write outside source");
        fs::write(
            fixture.warrant.join("manifest.toml"),
            manifest_with_extra_atom("adr", "../../outside-adr-root.md"),
        )
        .expect("write manifest");

        let loaded = fixture.load().expect("escaping ADR source is diagnostic");
        assert!(has_diagnostic(
            &loaded,
            "atom.unreadable",
            "outside ADR atoms root"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn adr_role_parent_components_cannot_escape_repository_root() {
        let fixture = TestRepository::new();
        fs::write(
            fixture.warrant.join("manifest.toml"),
            manifest_with_extra_atom("adr", "../../../../outside-repository.md"),
        )
        .expect("write manifest");

        let loaded = fixture
            .load()
            .expect("repository escape is a bounded diagnostic");
        assert!(has_diagnostic(
            &loaded,
            "atom.path-unsafe",
            "escapes repository root"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn absolute_adr_role_source_is_refused_before_open() {
        let fixture = TestRepository::new();
        fs::write(
            fixture.warrant.join("manifest.toml"),
            manifest_with_extra_atom("adr", "/dev/zero"),
        )
        .expect("write manifest");

        let loaded = fixture.load().expect("absolute ADR source is diagnostic");
        assert!(has_diagnostic(&loaded, "atom.path-unsafe", "is unsafe"));
    }

    #[cfg(unix)]
    #[test]
    fn repository_bounded_reader_accepts_only_contained_regular_files() {
        let fixture = TestRepository::new();
        let artifact = fixture.repo.root.join("artifact.bin");
        fs::write(&artifact, b"bounded").expect("write artifact");

        assert_eq!(
            read_repository_regular_bounded(&artifact, &fixture.repo.root, "artifact", 7)
                .expect("contained artifact"),
            b"bounded"
        );
        let error = read_repository_regular_bounded(
            Utf8Path::new("/dev/zero"),
            &fixture.repo.root,
            "artifact",
            7,
        )
        .expect_err("device outside repository must fail");
        assert!(
            error
                .to_string()
                .contains("not lexically below repository root"),
            "{error}"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn repository_bounded_reader_refuses_a_nested_mount() {
        let error = read_repository_regular_bounded(
            Utf8Path::new("/proc/version"),
            Utf8Path::new("/"),
            "artifact",
            1024 * 1024,
        )
        .expect_err("a file reached through a nested mount must fail closed");

        assert!(
            error.to_string().contains("mount boundary"),
            "unexpected refusal: {error}"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn repository_bounded_reader_refuses_a_mount_at_the_allowed_root() {
        let error = match read_contained_regular_bounded(
            Utf8Path::new("/proc/version"),
            Utf8Path::new("/"),
            Utf8Path::new("/proc"),
            "allowed root",
            "artifact",
            1024 * 1024,
        ) {
            Ok(_) => panic!("a mount used as the allowed root must fail closed"),
            Err(error) => error,
        };

        assert!(
            error.to_string().contains("mount boundary"),
            "unexpected refusal: {error}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn capability_backend_refuses_a_symlink_escape() {
        use std::os::unix::fs::symlink;

        let fixture = TestRepository::new();
        let outside = fixture.repo.root.join("outside-capability.bin");
        let link = fixture.warrant.join("inside-link.bin");
        fs::write(&outside, b"outside").expect("write outside artifact");
        symlink(&outside, &link).expect("create escaping symlink");

        open_capability_candidate(&link, &fixture.warrant)
            .expect_err("capability backend must refuse an escaping symlink");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn capability_backend_refuses_a_mount_below_the_repository_root() {
        let error = open_capability_candidate(Utf8Path::new("/proc/version"), Utf8Path::new("/"))
            .expect_err("capability backend must refuse a nested mount");

        assert_eq!(error.raw_os_error(), Some(libc::EXDEV), "{error}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn repository_bounded_reader_falls_back_when_procfs_is_unavailable() {
        let fixture = TestRepository::new();
        let artifact = fixture.repo.root.join("artifact.bin");
        fs::write(&artifact, b"bounded").expect("write artifact");
        let unavailable_proc_fd = fixture.repo.root.join("missing-proc-self-fd");

        let bytes = with_test_linux_proc_fd_root(&unavailable_proc_fd, || {
            read_repository_regular_bounded(&artifact, &fixture.repo.root, "artifact", 7)
        })
        .expect("contained artifact remains readable without procfs");

        assert_eq!(bytes, b"bounded");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn repository_bounded_reader_falls_back_when_procfs_root_is_not_a_directory() {
        let fixture = TestRepository::new();
        let artifact = fixture.repo.root.join("artifact.bin");
        fs::write(&artifact, b"bounded").expect("write artifact");
        let unavailable_proc_fd = fixture.repo.root.join("proc-self-fd-placeholder");
        fs::write(&unavailable_proc_fd, b"not a directory").expect("write procfs placeholder");

        let bytes = with_test_linux_proc_fd_root(&unavailable_proc_fd, || {
            read_repository_regular_bounded(&artifact, &fixture.repo.root, "artifact", 7)
        })
        .expect("contained artifact remains readable when procfs root is not a directory");

        assert_eq!(bytes, b"bounded");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn repository_bounded_reader_refuses_bytes_that_change_between_reads() {
        let fixture = TestRepository::new();
        let artifact = fixture.repo.root.join("artifact.bin");
        fs::write(&artifact, b"original").expect("write original artifact");

        let error = with_test_after_first_read_replacement(&artifact, b"modified", || {
            read_repository_regular_bounded(&artifact, &fixture.repo.root, "artifact", 8)
        })
        .expect_err("equal-length content replacement must fail closed");

        assert!(error.to_string().contains("not byte-stable"), "{error}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn procfs_fallback_rejects_replacement_at_the_requested_path() {
        let fixture = TestRepository::new();
        let artifact = fixture.repo.root.join("artifact.bin");
        fs::write(&artifact, b"original").expect("write original artifact");

        let canonical = canonical_utf8(&artifact, "artifact").expect("canonical artifact");
        let expected = fs::metadata(&canonical).expect("inspect original artifact");
        let file = open_regular_candidate(&canonical).expect("open original artifact");
        let opened = file.metadata().expect("inspect opened original artifact");

        fs::remove_file(&artifact).expect("unlink original artifact");
        fs::write(&artifact, b"replaced").expect("write equal-length replacement");

        let expectation = OpenedPathExpectation {
            requested: &artifact,
            canonical: &canonical,
            repository_root: &fixture.repo.root,
            allowed_root: &fixture.repo.root,
            allowed_root_label: "repository root",
            label: "artifact",
            metadata: &expected,
        };
        let unavailable_proc_fd = fixture.repo.root.join("missing-proc-self-fd");
        let error = with_test_linux_proc_fd_root(&unavailable_proc_fd, || {
            verify_opened_path(&file, &expectation, &opened)
        })
        .expect_err("replacement identity must fail closed");

        assert!(
            error.to_string().contains("stable file identity"),
            "{error}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn repository_bounded_reader_rejects_limit_sentinel_overflow() {
        let fixture = TestRepository::new();
        let artifact = fixture.repo.root.join("artifact.bin");
        fs::write(&artifact, b"bounded").expect("write artifact");

        let error =
            read_repository_regular_bounded(&artifact, &fixture.repo.root, "artifact", u64::MAX)
                .expect_err("an unbounded sentinel must fail closed");
        assert!(error.to_string().contains("cannot enforce"), "{error}");
    }

    #[cfg(windows)]
    #[test]
    fn windows_repository_bounded_reader_accepts_a_contained_regular_file() {
        let fixture = TestRepository::new();
        let artifact = fixture.repo.root.join("artifact.bin");
        fs::write(&artifact, b"bounded").expect("write artifact");

        assert_eq!(
            read_repository_regular_bounded(&artifact, &fixture.repo.root, "artifact", 7)
                .expect("contained artifact"),
            b"bounded"
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_repository_reader_refuses_a_preexisting_writer() {
        use std::os::windows::fs::OpenOptionsExt;

        use windows_sys::Win32::Storage::FileSystem::{
            FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        };

        let fixture = TestRepository::new();
        let artifact = fixture.repo.root.join("artifact.bin");
        fs::write(&artifact, b"bounded").expect("write artifact");
        let _writer = OpenOptions::new()
            .write(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .open(&artifact)
            .expect("hold a write-capable handle");

        let error = read_repository_regular_bounded(&artifact, &fixture.repo.root, "artifact", 7)
            .expect_err("reader must refuse a concurrent writer");
        match error {
            RepoError::Io { source, .. } => {
                assert_eq!(source.raw_os_error(), Some(32), "{source}");
            }
            other => panic!("expected Windows sharing violation, got {other}"),
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_opened_reparse_point_is_explicitly_refused() {
        use std::os::windows::fs::symlink_file;

        let fixture = TestRepository::new();
        let target = fixture.repo.root.join("target.bin");
        let link = fixture.repo.root.join("linked.bin");
        fs::write(&target, b"bounded").expect("write target");
        symlink_file(&target, &link).expect("create file symlink");

        let file = open_regular_candidate(&link).expect("open reparse point itself");
        let metadata = file.metadata().expect("inspect opened reparse point");
        let error = validate_regular_size(&link, "artifact", &metadata, 7)
            .expect_err("opened reparse point must fail closed");

        assert!(error.to_string().contains("reparse point"), "{error}");
    }

    #[cfg(windows)]
    #[test]
    fn windows_opened_handle_path_mismatch_is_refused() {
        let fixture = TestRepository::new();
        let artifact = fixture.repo.root.join("artifact.bin");
        let other = fixture.repo.root.join("other.bin");
        fs::write(&artifact, b"bounded").expect("write artifact");
        fs::write(&other, b"bounded").expect("write other artifact");

        let canonical = canonical_utf8(&artifact, "artifact").expect("canonical artifact");
        let wrong_canonical = canonical_utf8(&other, "other").expect("canonical other artifact");
        let expected = fs::metadata(&canonical).expect("inspect artifact");
        let file = open_regular_candidate(&canonical).expect("open artifact");
        let opened = file.metadata().expect("inspect opened artifact");
        let expectation = OpenedPathExpectation {
            requested: &artifact,
            canonical: &wrong_canonical,
            repository_root: &fixture.repo.root,
            allowed_root: &fixture.repo.root,
            allowed_root_label: "repository root",
            label: "artifact",
            metadata: &expected,
        };

        let error = verify_opened_path(&file, &expectation, &opened)
            .expect_err("handle final path mismatch must fail closed");
        assert!(
            error.to_string().contains("not contained at expected path"),
            "{error}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_hard_link_aliases_share_one_opened_file_identity() {
        let fixture = TestRepository::new();
        let first = fixture
            .repo
            .root
            .join("docs/adr/atoms/OW-ADR-9998-first.md");
        let alias = fixture
            .repo
            .root
            .join("docs/adr/atoms/OW-ADR-9999-alias.md");
        fs::write(&first, markdown_atom("adr", 30)).expect("write ADR atom");
        fs::hard_link(&first, &alias).expect("create hard-link alias");

        let mut manifest = manifest_with_extra_atom("adr", "../../adr/atoms/OW-ADR-9998-first.md");
        manifest.push_str(
            r#"
[[atoms]]
ordinal = 31
role = "adr"
path = "../../adr/atoms/OW-ADR-9999-alias.md"
required = false
"#,
        );
        fs::write(fixture.warrant.join("manifest.toml"), manifest).expect("write manifest");

        let loaded = fixture.load().expect("hard-link alias is a diagnostic");
        assert!(has_diagnostic(
            &loaded,
            "atom.path-duplicate",
            "same opened file identity"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn absolute_device_atom_is_refused_without_opening_it() {
        let fixture = TestRepository::new();
        fixture.write_manifest("/dev/zero");

        let loaded = fixture.load().expect("unsafe atom is a diagnostic");
        assert!(has_diagnostic(
            &loaded,
            "atom.path-unsafe",
            "expected a non-empty relative path"
        ));
        assert_eq!(loaded.basis.expect("remaining basis").atoms.len(), 4);
    }

    #[cfg(unix)]
    #[test]
    fn warrant_directory_dotdot_alias_is_refused_before_manifest_access() {
        let fixture = TestRepository::new();
        let aliased = fixture
            .warrant
            .parent()
            .expect("warrants directory")
            .join("OW-WAR-9999/../OW-WAR-9999");

        let error = fixture
            .repo
            .load_warrant(&aliased)
            .expect_err("dotdot Warrant directory must fail");
        assert!(error.to_string().contains("is unsafe"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn empty_dot_dotdot_and_backslash_atom_components_are_refused() {
        let fixture = TestRepository::new();
        for unsafe_path in [
            "",
            ".",
            "../outside.md",
            "atoms/./10-intent.md",
            "atoms//10-intent.md",
            "atoms/../10-intent.md",
            "atoms\\10-intent.md",
        ] {
            fixture.write_manifest(unsafe_path);
            let loaded = fixture.load().expect("unsafe atom is a diagnostic");
            assert!(
                has_diagnostic(&loaded, "atom.path-unsafe", "is unsafe"),
                "path {unsafe_path:?} was not refused: {:?}",
                loaded.report.diagnostics
            );
        }
    }

    #[test]
    fn authored_relative_paths_reject_ntfs_alias_forms_platform_neutrally() {
        for unsafe_path in [
            "atoms/name:stream.md",
            "atoms/trailing.",
            "atoms/trailing /child.md",
        ] {
            assert!(
                validate_safe_relative_source(unsafe_path, "atom source").is_err(),
                "ordinary path {unsafe_path:?} must fail"
            );
            assert!(
                validate_adr_relative_source(unsafe_path, "ADR atom source").is_err(),
                "ADR path {unsafe_path:?} must fail"
            );
        }
    }

    #[test]
    fn git_paths_join_components_with_forward_slashes() {
        assert_eq!(
            join_git_path_components(["docs", "warrants", "OW-WAR-0003", "manifest.toml"]),
            "docs/warrants/OW-WAR-0003/manifest.toml"
        );
    }

    #[cfg(unix)]
    #[test]
    fn duplicate_atom_source_path_is_refused_without_a_second_read() {
        let fixture = TestRepository::new();
        let manifest = manifest_with_intent_path("atoms/10-intent.md").replace(
            "path = \"atoms/20-basis.md\"",
            "path = \"atoms/10-intent.md\"",
        );
        fs::write(fixture.warrant.join("manifest.toml"), manifest).expect("write manifest");

        let loaded = fixture.load().expect("duplicate path is a diagnostic");
        assert!(has_diagnostic(
            &loaded,
            "atom.path-duplicate",
            "declared more than once"
        ));
        assert_eq!(loaded.basis.expect("remaining basis").atoms.len(), 4);
    }

    #[cfg(unix)]
    #[test]
    fn excessive_atom_count_refuses_the_entire_basis() {
        let fixture = TestRepository::new();
        let mut manifest = manifest_with_intent_path("atoms/10-intent.md");
        for ordinal in 0..=(MAX_WARRANT_ATOMS - 5) {
            manifest.push_str(&format!(
                "\n[[atoms]]\nordinal = {}\nrole = \"test.extra\"\npath = \
                 \"atoms/extra-{ordinal}.yaml\"\nrequired = false\n",
                1000 + ordinal
            ));
        }
        fs::write(fixture.warrant.join("manifest.toml"), manifest).expect("write manifest");

        let loaded = fixture.load().expect("resource refusal is diagnostic");
        assert!(has_diagnostic(
            &loaded,
            "manifest.resource-limit",
            "exceeding"
        ));
        assert!(
            loaded.basis.is_none(),
            "partial oversized basis must not exist"
        );
    }

    #[cfg(unix)]
    #[test]
    fn aggregate_atom_byte_budget_refuses_remaining_atoms() {
        let fixture = TestRepository::new();
        for path in [
            "atoms/10-intent.md",
            "atoms/20-basis.md",
            "atoms/40-work-order.md",
        ] {
            File::create(fixture.warrant.join(path))
                .expect("replace atom")
                .set_len(MAX_ATOM_BYTES)
                .expect("grow atom");
        }

        let loaded = fixture.load().expect("aggregate budget is diagnostic");
        assert!(has_diagnostic(
            &loaded,
            "atom.resource-limit",
            "aggregate limit"
        ));
        let retained: u64 = loaded
            .basis
            .expect("bounded partial basis")
            .atoms
            .iter()
            .map(|atom| atom.bytes.len() as u64)
            .sum();
        assert!(retained <= MAX_WARRANT_ATOM_BYTES);
    }

    #[cfg(unix)]
    #[test]
    fn oversized_manifest_is_refused_before_parsing() {
        let fixture = TestRepository::new();
        File::create(fixture.warrant.join("manifest.toml"))
            .expect("replace manifest")
            .set_len(MAX_MANIFEST_BYTES + 1)
            .expect("grow manifest");

        let error = fixture.load().expect_err("oversized manifest must fail");
        assert!(error.to_string().contains("exceeding"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn oversized_atom_and_scope_are_both_reported() {
        let fixture = TestRepository::new();
        File::create(fixture.warrant.join("atoms/10-intent.md"))
            .expect("replace atom")
            .set_len(MAX_ATOM_BYTES + 1)
            .expect("grow atom");
        File::create(fixture.warrant.join("scope.toml"))
            .expect("create scope")
            .set_len(MAX_SCOPE_BYTES + 1)
            .expect("grow scope");

        let loaded = fixture.load().expect("bounded leaf errors aggregate");
        assert!(has_diagnostic(&loaded, "atom.unreadable", "exceeding"));
        assert!(has_diagnostic(&loaded, "scope.unreadable", "exceeding"));
    }

    #[cfg(unix)]
    #[test]
    fn non_regular_atom_and_scope_are_both_reported() {
        let fixture = TestRepository::new();
        fs::remove_file(fixture.warrant.join("atoms/10-intent.md")).expect("remove atom");
        fs::create_dir(fixture.warrant.join("atoms/10-intent.md")).expect("directory atom");
        fs::create_dir(fixture.warrant.join("scope.toml")).expect("directory scope");

        let loaded = fixture.load().expect("non-regular leaf errors aggregate");
        assert!(has_diagnostic(
            &loaded,
            "atom.unreadable",
            "not a regular file"
        ));
        assert!(has_diagnostic(
            &loaded,
            "scope.unreadable",
            "not a regular file"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn non_regular_manifest_is_refused() {
        let fixture = TestRepository::new();
        fs::remove_file(fixture.warrant.join("manifest.toml")).expect("remove manifest");
        fs::create_dir(fixture.warrant.join("manifest.toml")).expect("directory manifest");

        let error = fixture.load().expect_err("directory manifest must fail");
        assert!(error.to_string().contains("not a regular file"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn manifest_symlink_escape_is_refused() {
        use std::os::unix::fs::symlink;

        let fixture = TestRepository::new();
        let outside = fixture.repo.root.join("outside-manifest.toml");
        fs::write(&outside, manifest_with_intent_path("atoms/10-intent.md"))
            .expect("write external manifest");
        fs::remove_file(fixture.warrant.join("manifest.toml")).expect("remove manifest");
        symlink(&outside, fixture.warrant.join("manifest.toml")).expect("symlink manifest");

        let error = fixture.load().expect_err("escaping manifest must fail");
        assert!(
            error.to_string().contains("contains symlink component"),
            "{error}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn atom_and_scope_symlink_escapes_are_both_reported() {
        use std::os::unix::fs::symlink;

        let fixture = TestRepository::new();
        let outside_atom = fixture.repo.root.join("outside-atom.md");
        let outside_scope = fixture.repo.root.join("outside-scope.toml");
        fs::write(&outside_atom, markdown_atom("intent", 10)).expect("external atom");
        fs::write(&outside_scope, "version = 1\n").expect("external scope");
        fs::remove_file(fixture.warrant.join("atoms/10-intent.md")).expect("remove atom");
        symlink(&outside_atom, fixture.warrant.join("atoms/10-intent.md")).expect("symlink atom");
        symlink(&outside_scope, fixture.warrant.join("scope.toml")).expect("symlink scope");

        let loaded = fixture.load().expect("escaping leaves are diagnostics");
        assert!(has_diagnostic(
            &loaded,
            "atom.unreadable",
            "contains symlink component"
        ));
        assert!(has_diagnostic(
            &loaded,
            "scope.unreadable",
            "contains symlink component"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn adr_role_symlink_cannot_escape_configured_adr_atoms_root() {
        use std::os::unix::fs::symlink;

        let fixture = TestRepository::new();
        let outside = fixture.repo.root.join("outside-adr-source.md");
        let link = fixture.repo.root.join("docs/adr/atoms/linked.md");
        fs::write(&outside, markdown_atom("adr", 30)).expect("write outside ADR source");
        symlink(&outside, &link).expect("symlink ADR source");
        fs::write(
            fixture.warrant.join("manifest.toml"),
            manifest_with_extra_atom("adr", "../../adr/atoms/linked.md"),
        )
        .expect("write manifest");

        let loaded = fixture.load().expect("escaping ADR symlink is diagnostic");
        assert!(has_diagnostic(
            &loaded,
            "atom.unreadable",
            "contains symlink component"
        ));
    }
}
