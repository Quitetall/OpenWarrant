// SPDX-License-Identifier: AGPL-3.0-or-later
//! Repository discovery and loading — the I/O half the core crate refuses (§79.1, §79.4).

use std::fmt;
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::{AtomSource, CompilationBasis, SasPin, ScopeSource};
use openwarrant_core::{
    AdrError, AdrRecord, Manifest, RepositoryConfig, ValidatedManifest, frontmatter,
};

use openwarrant_core::authority::{AuthorityRegister, RoleAssignment};
use openwarrant_core::deliverable::Deliverable;
use openwarrant_core::verification::Verification;

use crate::diagnostic::{Diagnostic, Report};
use crate::init::CONFIG_FILE;

#[derive(Debug)]
pub enum RepoError {
    /// A command-level failure that is not about locating or parsing the
    /// repository — an unknown view name, an uncompilable Warrant. Kept
    /// separate from the structured variants so it cannot absorb them.
    Message(String),
    NotFound {
        from: Utf8PathBuf,
    },
    NonUtf8Path,
    Io {
        context: String,
        source: std::io::Error,
    },
    ConfigParse {
        path: Utf8PathBuf,
        source: toml::de::Error,
    },
    ConfigInvalid {
        path: Utf8PathBuf,
        source: openwarrant_core::ConfigError,
    },
    ManifestParse {
        path: Utf8PathBuf,
        source: toml::de::Error,
    },
    InvalidSasEncoding(std::str::Utf8Error),
    InvalidSasDeclarations(openwarrant_core::SasError),
    UnknownWarrant {
        alias: String,
        known: Vec<String>,
    },
}

impl fmt::Display for RepoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { from } => write!(
                f,
                "no {CONFIG_FILE} found in {from} or any parent directory. \
                 Run `war init --namespace <NS>` to create one."
            ),
            Self::Message(m) => write!(f, "{m}"),
            Self::NonUtf8Path => write!(f, "the current directory is not valid UTF-8"),
            Self::Io { context, source } => write!(f, "{context}: {source}"),
            Self::ConfigParse { path, source } => write!(f, "{path}: {source}"),
            Self::ConfigInvalid { path, source } => write!(f, "{path}: {source}"),
            Self::ManifestParse { path, source } => write!(f, "{path}: {source}"),
            Self::InvalidSasEncoding(source) => write!(f, "selected SAS is not UTF-8: {source}"),
            Self::InvalidSasDeclarations(source) => {
                write!(f, "selected SAS declarations are invalid: {source}")
            }
            Self::UnknownWarrant { alias, known } => write!(
                f,
                "no Warrant {alias:?} in this repository. Known: {}",
                if known.is_empty() {
                    "(none)".to_owned()
                } else {
                    known.join(", ")
                }
            ),
        }
    }
}

impl std::error::Error for RepoError {}

/// An initialized OpenWarrant repository.
#[derive(Debug, Clone)]
pub struct Repository {
    pub root: Utf8PathBuf,
    pub config: RepositoryConfig,
}

/// A declaration snapshot from exactly one document. `revision == None` is
/// unregistered draft inspection, never evidence of accepted SAS authority.
#[derive(Debug, Clone)]
pub struct SasSnapshot {
    pub revision: Option<openwarrant_core::SasRevision>,
    pub declarations: openwarrant_core::sas::SasDeclarations,
}

impl SasSnapshot {
    pub fn is_authoritative(&self) -> bool {
        self.revision
            .as_ref()
            .is_some_and(openwarrant_core::SasRevision::is_accepted)
    }
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
    /// The §56.2 record, if one has been ingested (`war resolve --response`).
    pub fn load_resolution(
        &self,
        dir: &Utf8Path,
    ) -> Result<Option<crate::resolution_cmd::ResolutionRecord>, RepoError> {
        let path = dir.join("resolution.toml");
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
        let manifest_path = dir.join("manifest.toml");
        let manifest_bytes = fs::read(&manifest_path).map_err(|source| RepoError::Io {
            context: format!("could not read {manifest_path}"),
            source,
        })?;
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

        let mut atoms = Vec::new();
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

            let path = dir.join(rel);
            let bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(source) => {
                    report.push(Diagnostic::error(
                        "atom.missing",
                        self.relative(&path),
                        format!("declared at ordinal {}: {source}", entry.ordinal),
                    ));
                    continue;
                }
            };

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
        let scope = if scope_path.is_file() {
            match fs::read(&scope_path) {
                Ok(bytes) => Some(ScopeSource {
                    source: self.relative(&scope_path),
                    bytes,
                }),
                Err(source) => {
                    report.push(Diagnostic::error(
                        "scope.unreadable",
                        self.relative(&scope_path),
                        source.to_string(),
                    ));
                    None
                }
            }
        } else {
            None
        };

        Ok(Loaded {
            dir: dir.to_owned(),
            basis: Some(CompilationBasis {
                manifest,
                manifest_source: relative_manifest,
                manifest_bytes,
                atoms,
                scope,
                // §14 — the SAS revision is a Basis input. An AUTHORIZED Warrant
                // compiles against the revision its authorization recorded, so
                // a later SAS revision cannot move its contract digest out from
                // under the signature; an unauthorized one follows the latest
                // recorded revision. Absent until any revision is recorded.
                sas: self.sas_pin_for(dir)?,
            }),
            validated: Some(validated),
            report,
        })
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

    /// The SAS document: exactly one Markdown file under the configured path.
    ///
    /// Zero is an error and so is two. A repository with two candidate
    /// documents has no single thing to pin, and guessing which one is
    /// normative is the failure §101.6 exists to prevent.
    pub fn sas_document(&self) -> Result<(Utf8PathBuf, Vec<u8>), RepoError> {
        let dir = self.root.join(&self.config.paths.sas);
        let mut docs: Vec<Utf8PathBuf> = fs::read_dir(&dir)
            .map_err(|source| RepoError::Io {
                context: format!("could not read {dir}"),
                source,
            })?
            .filter_map(Result::ok)
            .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
            .filter(|p| p.extension() == Some("md"))
            .collect();
        docs.sort();
        match docs.as_slice() {
            [one] => {
                let bytes = fs::read(one).map_err(|source| RepoError::Io {
                    context: format!("could not read {one}"),
                    source,
                })?;
                Ok((one.clone(), bytes))
            }
            [] => Err(RepoError::Message(format!(
                "no SAS document (*.md) under {dir}"
            ))),
            many => Err(RepoError::Message(format!(
                "{} SAS documents under {dir}; there must be exactly one to pin: {}",
                many.len(),
                many.iter()
                    .map(|p| self.relative(p))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    }

    #[must_use]
    pub fn sas_revisions_dir(&self) -> Utf8PathBuf {
        self.root.join(&self.config.paths.sas).join("revisions")
    }

    #[must_use]
    pub fn sas_revision_path(&self, version: &str) -> Utf8PathBuf {
        self.sas_revisions_dir().join(format!("{version}.toml"))
    }

    /// Every SAS revision on record (§101), validated. A malformed record is an
    /// error for the set: an accepted revision that quietly failed to load
    /// would read as "the document is unpinned", which is the wrong direction.
    pub fn load_sas_revisions(&self) -> Result<Vec<openwarrant_core::SasRevision>, RepoError> {
        let dir = self.sas_revisions_dir();
        if !dir.is_dir() {
            return Ok(vec![]);
        }
        let mut paths: Vec<Utf8PathBuf> = fs::read_dir(&dir)
            .map_err(|source| RepoError::Io {
                context: format!("could not read {dir}"),
                source,
            })?
            .filter_map(Result::ok)
            .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
            .filter(|p| p.extension() == Some("toml"))
            .collect();
        paths.sort();
        let mut out = Vec::new();
        for path in paths {
            let text = fs::read_to_string(&path).map_err(|source| RepoError::Io {
                context: format!("could not read {path}"),
                source,
            })?;
            let r: openwarrant_core::SasRevision = toml::from_str(&text)
                .map_err(|e| RepoError::Message(format!("could not parse {path}: {e}")))?;
            r.validate()
                .map_err(|e| RepoError::Message(format!("{path}: {e}")))?;
            out.push(r);
        }
        Ok(out)
    }

    /// The revision the document is held to: newest accepted, else newest
    /// proposed, else none.
    /// The SAS revision this Warrant compiles against (see `load_warrant`).
    ///
    /// An authorization naming a revision that has no record is an error
    /// surfaced by `war check` as `sas.pin-unknown`; here it falls back to the
    /// latest so the Warrant still compiles and the check can name the fault.
    pub fn sas_pin_for(&self, dir: &Utf8Path) -> Result<Option<SasPin>, RepoError> {
        let all = self.load_sas_revisions()?;
        let pinned = self
            .load_authorization(dir)
            .ok()
            .flatten()
            .and_then(|a| a.sas_revision)
            .and_then(|v| all.iter().find(|r| r.version == v).cloned());
        let chosen = match pinned {
            Some(r) => Some(r),
            None => crate::sas::pin_of(&all).cloned(),
        };
        Ok(chosen.map(|r| SasPin {
            version: r.version,
            sha256: r.sha256,
        }))
    }

    pub fn latest_sas_revision(&self) -> Result<Option<openwarrant_core::SasRevision>, RepoError> {
        let all = self.load_sas_revisions()?;
        Ok(crate::sas::pin_of(&all).cloned())
    }

    /// Bind declarations to the selected revision's source and digest. An
    /// explicit revision pin takes precedence over the repository selection.
    /// A changed document is unavailable, never replaced with another program's
    /// phase table. An unregistered document is inspectable only as a draft.
    pub fn sas_snapshot(&self, pin: Option<&SasPin>) -> Result<SasSnapshot, RepoError> {
        let (path, bytes) = self.sas_document()?;
        let all = self.load_sas_revisions()?;
        let revision = match pin {
            Some(pin) => Some(
                all.iter()
                    .find(|r| r.version == pin.version && r.sha256 == pin.sha256)
                    .ok_or_else(|| {
                        RepoError::Message(format!(
                            "selected SAS revision {} at sha256:{} is unavailable",
                            pin.version, pin.sha256
                        ))
                    })?
                    .clone(),
            ),
            None => crate::sas::pin_of(&all).cloned(),
        };
        if let Some(r) = &revision
            && (r.source != self.relative(&path)
                || r.sha256 != openwarrant_compiler::sha256_hex(&bytes))
        {
            return Err(RepoError::Message(format!(
                "selected SAS revision {} source/digest mismatch; phase and requirement authority unavailable",
                r.version
            )));
        }
        let text = std::str::from_utf8(&bytes).map_err(RepoError::InvalidSasEncoding)?;
        let declarations = openwarrant_core::sas::SasDeclarations::parse(text)
            .map_err(RepoError::InvalidSasDeclarations)?;
        if declarations.phases.is_empty() || declarations.requirements.is_empty() {
            return Err(RepoError::Message(
                "SAS phase or requirement declarations are missing; authority unavailable"
                    .to_owned(),
            ));
        }
        if let Some(r) = &revision
            && r.requirements != declarations.requirements
        {
            return Err(RepoError::Message(format!(
                "selected SAS revision {} requirement snapshot mismatch; authority unavailable",
                r.version
            )));
        }
        Ok(SasSnapshot {
            revision,
            declarations,
        })
    }

    /// Where the corpus projection is written (§17.5 `status`, corpus form).
    ///
    /// Two files from one build: the Markdown a person reads and the canonical
    /// JSON an agent reads. Named `CORPUS_STATUS` rather than `STATUS` because
    /// `status` is already the per-Warrant §17.5 view name.
    #[must_use]
    pub fn corpus_status_md_path(&self) -> Utf8PathBuf {
        self.root
            .join(&self.config.paths.warrants)
            .join("generated")
            .join("CORPUS_STATUS.md")
    }

    #[must_use]
    pub fn corpus_status_html_path(&self) -> Utf8PathBuf {
        self.root
            .join(&self.config.paths.warrants)
            .join("generated")
            .join("CORPUS_STATUS.html")
    }

    #[must_use]
    pub fn corpus_status_json_path(&self) -> Utf8PathBuf {
        self.root
            .join(&self.config.paths.warrants)
            .join("generated")
            .join("CORPUS_STATUS.json")
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
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .as_str()
            .to_owned()
    }
}

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
mod program_sas_tests {
    use super::*;
    use openwarrant_core::sas::{SasAcceptance, SasRevision};
    use openwarrant_core::status::{Achieved, SasAuthority};

    struct Fixture(Repository);

    impl Fixture {
        fn new(namespace: &str) -> Self {
            let root = Utf8PathBuf::from_path_buf(std::env::temp_dir().join(format!(
                "ow-program-sas-{}",
                openwarrant_core::WarUuid::mint()
            )))
            .expect("UTF-8 temp path");
            fs::create_dir_all(&root).expect("fixture directory");
            crate::init::run(namespace, Some("Fixture"), Some(root.clone())).expect("init");
            Self(Repository::open(root).expect("repository"))
        }

        fn document(&self, phases: std::ops::RangeInclusive<i32>) -> String {
            let text = format!(
                "# Test SAS\n## 98. Implementation phases\n{}\n## 106. Requirements\n| LIM-SAS-RQ-001 | Test requirement |\n",
                phases
                    .map(|n| format!("### Phase {n} — Phase {n}\n\nExit:\n\n- phase {n} exit.\n"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            fs::write(self.0.root.join("docs/sas/SAS.md"), &text).expect("SAS");
            text
        }

        fn record(&self, text: &str, accepted: bool) {
            let mut r = SasRevision::proposed(
                "1",
                "docs/sas/SAS.md",
                openwarrant_compiler::sha256_hex(text.as_bytes()),
                None,
                openwarrant_core::sas::section_106(text),
                false,
            );
            if accepted {
                r = r
                    .accept(SasAcceptance {
                        accepted_by: "fixture-human".to_owned(),
                        actor_kind: openwarrant_core::contract::ActorKind::Human,
                        acting_role: "test owner".to_owned(),
                        meaning: "test fixture only".to_owned(),
                        effective_time: "2026-09-05T00:00:00Z".to_owned(),
                        adr_ref: None,
                    })
                    .expect("fixture acceptance");
            }
            fs::create_dir_all(self.0.sas_revisions_dir()).expect("revision directory");
            fs::write(
                self.0.sas_revision_path("1"),
                toml::to_string(&r).expect("revision TOML"),
            )
            .expect("revision");
        }

        fn warrant(&self, refs: &[&str]) -> Utf8PathBuf {
            let dir = crate::new::run(
                &self.0,
                "Fixture intervention",
                openwarrant_core::Profile::Delivery,
            )
            .expect("new");
            let path = dir.join("manifest.toml");
            let mut manifest = fs::read_to_string(&path).expect("manifest");
            for r in refs {
                manifest.push_str(&format!("\n[[roadmap]]\nref = {r:?}\n"));
            }
            fs::write(path, manifest).expect("manifest refs");
            dir
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0.root);
        }
    }

    #[test]
    fn all_fourteen_liminal_phases_come_from_selected_sas_and_configured_namespace() {
        let f = Fixture::new("LIM");
        let text = f.document(-1..=12);
        f.record(&text, true);
        let status = crate::status::build(&f.0).expect("status");
        assert_eq!(status.release.authority, SasAuthority::Accepted);
        let refs: Vec<_> = status
            .objectives
            .iter()
            .filter_map(|o| o.roadmap_ref.as_ref())
            .collect();
        assert_eq!(
            refs.iter().map(|r| r.phase).collect::<Vec<_>>(),
            (-1..=12).collect::<Vec<_>>()
        );
        assert!(refs.iter().all(|r| r.prefix == "LIM"));
        for phase in -1..=12 {
            let dir = f.warrant(&[&format!("roadmap://LIM-PHASE-{phase}/exit")]);
            let loaded = f.0.load_warrant(&dir).expect("load");
            let mut report = crate::diagnostic::Report::default();
            crate::check::check_traceability(&f.0, &loaded, &loaded.alias(), &mut report);
            assert!(report.is_ready(), "{report:?}");
        }
    }

    #[test]
    fn wrong_program_undeclared_duplicate_and_malformed_references_are_refused() {
        let f = Fixture::new("LIM");
        let text = f.document(-1..=12);
        f.record(&text, true);
        for (refs, rule) in [
            (vec!["roadmap://OW-PHASE-1"], "roadmap.wrong-program"),
            (vec!["roadmap://LIM-PHASE-13"], "roadmap.undeclared-phase"),
            (vec!["roadmap://LIM-PHASE--2"], "roadmap.undeclared-phase"),
            (vec!["roadmap://LIM-PHASE--01"], "roadmap.malformed"),
            (
                vec!["roadmap://LIM-PHASE-1", "LIM-PHASE-1"],
                "roadmap.duplicate",
            ),
        ] {
            let dir = f.warrant(&refs);
            let loaded = f.0.load_warrant(&dir).expect("load");
            let mut report = crate::diagnostic::Report::default();
            crate::check::check_traceability(&f.0, &loaded, &loaded.alias(), &mut report);
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|d| d.rule == rule && d.severity == crate::diagnostic::Severity::Error),
                "{report:?}"
            );
        }
        let status = crate::status::build(&f.0).expect("status");
        assert!(
            status
                .warrants
                .iter()
                .all(|w| w.rung == openwarrant_core::status::WarrantRung::Invalid)
        );
        assert_eq!(
            status.objectives.last().expect("unassigned").warrants.len(),
            4
        );
        assert_eq!(
            status
                .objectives
                .iter()
                .map(|o| o.warrants.len())
                .sum::<usize>(),
            5
        );
    }

    #[test]
    fn missing_ambiguous_or_drifted_sas_is_unavailable_without_fallback() {
        let f = Fixture::new("LIM");
        let status = crate::status::build(&f.0).expect("missing status");
        assert!(matches!(
            status.release.authority,
            SasAuthority::Unavailable { .. }
        ));
        assert!(status.objectives.iter().all(|o| o.roadmap_ref.is_none()));
        let text = f.document(-1..=12);
        f.record(&text, true);
        fs::write(f.0.root.join("docs/sas/second.md"), &text).expect("ambiguous");
        assert!(
            f.0.sas_snapshot(None)
                .expect_err("multiple documents")
                .to_string()
                .contains("exactly one")
        );
        fs::remove_file(f.0.root.join("docs/sas/second.md")).expect("remove duplicate fixture");
        fs::write(
            f.0.root.join("docs/sas/SAS.md"),
            format!("{text}\n### Phase 13 — forged\n"),
        )
        .expect("drift");
        let status = crate::status::build(&f.0).expect("drift status");
        assert!(
            matches!(&status.release.authority, SasAuthority::Unavailable { reason } if reason.contains("source/digest mismatch"))
        );
        assert!(status.objectives.iter().all(|o| o.roadmap_ref.is_none()));
        assert!(status.requirements.is_empty());
    }

    #[test]
    fn draft_inspection_is_explicit_and_prose_cannot_establish_completion() {
        let f = Fixture::new("LIM");
        let text = f.document(-1..=12);
        fs::write(
            f.0.root.join("docs/roadmap/PRODUCTION_ROADMAP.md"),
            "All phases **resolved**. LIM-SAS-RQ-001 satisfied.",
        )
        .expect("false prose claim");
        for proposed in [false, true] {
            if proposed {
                f.record(&text, false);
            }
            let status = crate::status::build(&f.0).expect("draft status");
            assert_eq!(status.release.authority, SasAuthority::Draft);
            assert!(
                status
                    .objectives
                    .iter()
                    .all(|o| matches!(o.achieved, Achieved::NotDerivable { .. }))
            );
            assert_eq!(status.release.requirements.satisfied, 0);
        }
    }

    #[test]
    fn duplicate_declarations_and_mismatched_selected_pin_are_unavailable() {
        let f = Fixture::new("LIM");
        let text = f.document(-1..=12);
        for extra in [
            "### Phase -1 — duplicate\n",
            "| LIM-SAS-RQ-001 | duplicate |\n",
        ] {
            let invalid = if extra.starts_with("### Phase") {
                text.replace(
                    "## 106. Requirements",
                    &format!("{extra}## 106. Requirements"),
                )
            } else {
                format!("{text}{extra}")
            };
            fs::write(f.0.root.join("docs/sas/SAS.md"), invalid).expect("duplicate");
            assert!(
                f.0.sas_snapshot(None)
                    .expect_err("duplicate")
                    .to_string()
                    .contains("duplicate SAS")
            );
            assert!(crate::sas::propose(&f.0, "invalid").is_err());
            assert!(!f.0.sas_revision_path("invalid").exists());
        }
        fs::write(f.0.root.join("docs/sas/SAS.md"), &text).expect("restore fixture");
        f.record(&text, true);
        let pin = SasPin {
            version: "1".to_owned(),
            sha256: "0".repeat(64),
        };
        assert!(
            f.0.sas_snapshot(Some(&pin))
                .expect_err("pin mismatch")
                .to_string()
                .contains("unavailable")
        );
    }

    #[test]
    fn openwarrant_declared_phases_remain_valid_and_phase_eleven_is_undeclared() {
        let f = Fixture::new("OW");
        let text = f.document(0..=10);
        f.record(&text, true);
        let status = crate::status::build(&f.0).expect("OW status");
        assert_eq!(
            status
                .objectives
                .iter()
                .filter(|o| o.roadmap_ref.is_some())
                .count(),
            11
        );
        let dir = f.warrant(&["roadmap://OW-PHASE-11"]);
        let loaded = f.0.load_warrant(&dir).expect("load");
        let mut report = crate::diagnostic::Report::default();
        crate::check::check_traceability(&f.0, &loaded, &loaded.alias(), &mut report);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.rule == "roadmap.undeclared-phase"),
            "{report:?}"
        );
    }
}
