// SPDX-License-Identifier: Apache-2.0
//! Repository discovery and loading — the I/O half the core crate refuses (§79.1, §79.4).

use crate::vfs as fs;
use std::fmt;

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::{AtomSource, CompilationBasis, SasPin, ScopeSource};
use openwarrant_core::{
    AdrError, AdrRecord, Manifest, RepositoryConfig, ValidatedManifest, frontmatter,
};

use openwarrant_core::authority::{ActorKind, ActorRole, AuthorityRegister, RoleAssignment};
use openwarrant_core::deliverable::Deliverable;
use openwarrant_core::role::{ProfileDefinition, ProfileRegistry};
use openwarrant_core::verification::Verification;

use crate::diagnostic::{Diagnostic, Report};
use crate::init::CONFIG_FILE;

// OW-WAR-0130: version negotiation between `war` and its records. A child of
// this module, declared here because the module list in `lib.rs` is not this
// Warrant's to change, and because discovery and loading are its callers.
#[path = "compat.rs"]
pub(crate) mod compat;

/// `[intake]` in `openwarrant.toml` (OW-WAR-0141). Every key is absent by
/// default; the table itself is absent by default.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntakePolicy {
    /// argv, not a shell string, with `{id}` where the issue number goes,
    /// e.g. `["gh", "issue", "view", "{id}", "--json", "number,title,body,url"]`.
    /// A read: an argv naming a write subcommand is refused before it runs.
    #[serde(default)]
    pub fetch_argv: Vec<String>,
    /// §74.4 "review or policy approval": intake drafts are applied under
    /// this policy, and `plan/pipeline.json` records `review: policy` — never
    /// `reviewed`, because nobody reviewed the proposal. The human's review
    /// is the authorization of the contract, which this does not touch.
    #[serde(default)]
    pub policy_approval: bool,
    /// Wall-clock bound on one fetch. 0 or absent means 30.
    #[serde(default)]
    pub fetch_timeout_secs: u64,
    /// OW-WAR-0148 M5, approved by the owner (2026-10-02): `[intake.writeback]`,
    /// what runs when a ticket made from an issue becomes done. Absent: no
    /// write ever runs. Kept apart from `fetch_argv`, whose "is a read"
    /// refusal it does not loosen.
    #[serde(default)]
    pub writeback: Option<WritebackPolicy>,
}

/// `[intake.writeback]`: argv templates, not shell strings. `{id}` is the
/// issue number; `{body}` is the comment, one argv element.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WritebackPolicy {
    /// e.g. `["gh", "issue", "comment", "{id}", "--body", "{body}"]`.
    #[serde(default)]
    pub comment_argv: Vec<String>,
    /// e.g. `["gh", "issue", "close", "{id}"]`.
    #[serde(default)]
    pub close_argv: Vec<String>,
    /// Wall-clock bound on each write. 0 or absent means 30.
    #[serde(default)]
    pub timeout_secs: u64,
}

impl IntakePolicy {
    #[must_use]
    pub fn fetch_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(if self.fetch_timeout_secs == 0 {
            30
        } else {
            self.fetch_timeout_secs
        })
    }
}

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
    /// The profiles this program admits (OW-WAR-0140): the two core ones and
    /// every `profiles/<name>.toml`, read once when the repository opens.
    pub profiles: ProfileRegistry,
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
            if fs::is_file(&candidate) {
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
            .map_err(|source| RepoError::ConfigInvalid {
                path: path.clone(),
                source,
            })?;
        // OW-WAR-0130: `[project] requires_war`, once, before any record is
        // read. `discover` reaches every repository through here, so a `war`
        // the repository does not admit reads nothing of it.
        compat::check(&config, &path).map_err(RepoError::Message)?;
        // OW-WAR-0138: with `[authority] store`, the protected policy keys are
        // the store's from here on, for every consumer. Without it, nothing
        // changes.
        let mut config = config;
        crate::authority_check::govern(&mut config);
        let profiles = load_profiles(&root)?;
        Ok(Self {
            root,
            config,
            profiles,
        })
    }

    /// The configured warrants directory.
    #[must_use]
    pub fn warrants_dir(&self) -> Utf8PathBuf {
        self.root.join(&self.config.paths.warrants)
    }

    /// The actor this tool acts as when it performs work (§27.1).
    ///
    /// `[project] performer` in `openwarrant.toml`, default `claude`, and
    /// deliberately not configurable from the command line. The performer
    /// identity is what every self-* check compares against —
    /// self-verification (§46), self-authorization (§27.2), self-resolution
    /// (§27.3 condition 4). A flag that let the caller rename the performer
    /// would let it walk out of all three by claiming to be somebody else;
    /// a committed, human-written file cannot be reached that way.
    #[must_use]
    pub fn performer(&self) -> String {
        self.config
            .project
            .performer
            .clone()
            .unwrap_or_else(|| "claude".to_owned())
    }

    /// `[intake]` (OW-WAR-0141): how a ticket reaches `war plan`. `None`
    /// when the table is absent, which is the default and means `--issue`
    /// starts no process.
    ///
    /// Read here rather than on `RepositoryConfig`, whose struct is not this
    /// Warrant's to change: the core parser ignores a table it does not know,
    /// so the table is parsed from the same file on its own, fail-closed on
    /// an unknown key or a wrong type.
    pub fn intake_policy(&self) -> Result<Option<IntakePolicy>, RepoError> {
        let path = self.root.join(CONFIG_FILE);
        let text = fs::read_to_string(&path).map_err(|source| RepoError::Io {
            context: format!("could not read {path}"),
            source,
        })?;
        #[derive(serde::Deserialize)]
        struct File {
            #[serde(default)]
            intake: Option<IntakePolicy>,
        }
        let file: File = toml::from_str(&text).map_err(|e| {
            RepoError::Message(format!("intake.config: {path}: the [intake] table: {e}"))
        })?;
        Ok(file.intake)
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
        if !fs::is_file(&path) {
            return Ok(AuthorityRegister::default());
        }
        let text = fs::read_to_string(&path).map_err(|source| RepoError::Io {
            context: format!("could not read {path}"),
            source,
        })?;
        // Every authority verdict reads the register, and `war next` asked
        // ~1,400 times (t-280c). The parse is a function of this path and
        // these bytes only, so it is memoized for the process keyed by both:
        // an edited register is a different key, never a stale grant. The
        // file is still read on every call; a register that will not parse
        // or validate is not cached and is refused again each time.
        let cache = REGISTERS.get_or_init(Default::default);
        if let Some(known) = cache.lock().ok().and_then(|c| {
            c.get(&path)
                .filter(|(k, _)| *k == text)
                .map(|(_, v)| v.clone())
        }) {
            return Ok(known);
        }

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
        let register = AuthorityRegister::new(file.assignment);
        if let Ok(mut c) = cache.lock() {
            c.insert(path, (text, register.clone()));
        }
        Ok(register)
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
        if !fs::is_file(&path) {
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
        if !fs::is_file(&path) {
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
        if !fs::is_file(&path) {
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
        if !fs::is_file(&path) {
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
        if !fs::is_dir(&dir) {
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
            if fs::is_file(path.join("manifest.toml")) {
                out.push(path);
            }
        }
        out.sort();
        Ok(out)
    }

    /// Resolve a Warrant directory by local alias.
    pub fn warrant_dir(&self, alias: &str) -> Result<Utf8PathBuf, RepoError> {
        // The same answer as the scan below, without it: the Warrant is the
        // directory named `alias` holding a manifest. The scan read and
        // sorted every entry per call, and callers ask per Warrant, so a
        // 1,000-Warrant `war next` spent most of its time here. Only a plain
        // one-component name takes the direct path.
        let plain =
            !alias.is_empty() && alias != "." && alias != ".." && !alias.contains(['/', '\\']);
        if plain {
            let dir = self.warrants_dir().join(alias);
            if fs::is_file(dir.join("manifest.toml")) {
                return Ok(dir);
            }
        }
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
        // OW-WAR-0130: a record newer than this `war` reads is UNKNOWN by
        // name, whatever else its own reader makes of it.
        for newer in compat::newer_records(dir, &|p: &Utf8Path| self.relative(p)) {
            report.push(newer);
        }

        let validated = match manifest
            .validate_in(Some(self.config.project.namespace.as_str()), &self.profiles)
        {
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
                Ok(fm) => {
                    if rel.ends_with(".md") && entry.role != "adr" {
                        for detail in header_mismatches(&fm, &manifest, entry) {
                            report.push(Diagnostic::error(
                                "atom.header",
                                self.relative(&path),
                                detail,
                            ));
                        }
                    }
                    fm.scalar("jurisdiction").unwrap_or("authored").to_owned()
                }
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

        if let Some(definition) = self
            .profiles
            .definition(&validated.profile)
            .filter(|d| d.extends.is_some())
        {
            self.profile_checks(dir, &validated, definition, &atoms, &mut report);
        }
        if let Some(pin) = &manifest.profile_digest {
            report.push(self.profile_pin(dir, &validated, pin, &relative_manifest));
        }

        let scope_path = dir.join("scope.toml");
        let scope = if fs::is_file(&scope_path) {
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
        if !fs::is_dir(&dir) {
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
        let mut texts = Vec::with_capacity(paths.len());
        for path in paths {
            let text = fs::read_to_string(&path).map_err(|source| RepoError::Io {
                context: format!("could not read {path}"),
                source,
            })?;
            texts.push((path, text));
        }
        // Every `load_warrant` asks for its SAS pin, and `war next` loaded
        // a Warrant ~1,300 times: parsing the same few revision files each
        // time was 40% of its run (t-280c). The parse is a function of
        // exactly these paths and bytes, so it is memoized for the process
        // keyed by them — a changed, added or removed revision is a
        // different key, never a stale answer. The files are still read on
        // every call; only the parse is shared. A parse or validation
        // failure is not cached.
        let cache = SAS_REVISIONS.get_or_init(Default::default);
        if let Some(known) = cache.lock().ok().and_then(|c| {
            c.get(&dir)
                .filter(|(k, _)| *k == texts)
                .map(|(_, v)| v.clone())
        }) {
            return Ok(known);
        }
        let mut out = Vec::new();
        for (path, text) in &texts {
            let r: openwarrant_core::SasRevision = toml::from_str(text)
                .map_err(|e| RepoError::Message(format!("could not parse {path}: {e}")))?;
            r.validate()
                .map_err(|e| RepoError::Message(format!("{path}: {e}")))?;
            out.push(r);
        }
        if let Ok(mut c) = cache.lock() {
            c.insert(dir, (texts, out.clone()));
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
        // OW-ADR-0016: the latest amendment that names a `sas_revision` re-pins
        // the Warrant ahead of its authorization, so the contract digest moves
        // and a new authorization revision is what `war sign --list` shows.
        // A named revision with no record falls through to the authorization's
        // pin; `war check` reports it as `sas.pin-unknown`.
        let amended = amendment_sas_revision(dir)
            .and_then(|(v, _)| all.iter().find(|r| r.version == v).cloned());
        let pinned = match amended {
            Some(r) => Some(r),
            None => self
                .load_authorization(dir)
                .ok()
                .flatten()
                .and_then(|a| a.sas_revision)
                .and_then(|v| all.iter().find(|r| r.version == v).cloned()),
        };
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
        if !fs::is_dir(&dir) {
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
        if !fs::is_dir(&vdir) {
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

    /// Corrections on file for one Warrant (OW-WAR-0064): every
    /// `corrections/*.toml`, sorted by path, parse failures kept beside the
    /// records so a malformed correction is reported rather than skipped.
    pub fn load_corrections(&self, dir: &Utf8Path) -> Result<CorrectionSet, RepoError> {
        let cdir = dir.join("corrections");
        if !fs::is_dir(&cdir) {
            return Ok(CorrectionSet::default());
        }
        let mut paths = Vec::new();
        for entry in fs::read_dir(&cdir).map_err(|source| RepoError::Io {
            context: format!("could not read {cdir}"),
            source,
        })? {
            let entry = entry.map_err(|source| RepoError::Io {
                context: format!("could not read an entry in {cdir}"),
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
            match toml::from_str::<openwarrant_core::correction::CorrectionRecord>(&text) {
                Ok(r) => records.push((relative, r)),
                Err(e) => failures.push((relative, e.to_string())),
            }
        }
        Ok(CorrectionSet { records, failures })
    }

    /// Deliverables declared for one Warrant (§37).
    ///
    /// A single `deliverables.toml` rather than a directory: a Warrant declares a
    /// handful, they are read together, and one file keeps them reviewable as a
    /// set.
    pub fn load_deliverables(&self, dir: &Utf8Path) -> Result<DeliverableSet, RepoError> {
        let path = dir.join("deliverables.toml");
        if !fs::is_file(&path) {
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

    /// OW-ADR-0031: the profile file a manifest pinned, against the file as
    /// it stands. Drift is a warning while the Warrant is unsigned — the
    /// draft can be re-pinned — and an error once a human has authorized it,
    /// because the signature covered the type the pin names. A manifest
    /// without the pin never reaches here.
    fn profile_pin(
        &self,
        dir: &Utf8Path,
        validated: &ValidatedManifest,
        pin: &str,
        manifest_file: &str,
    ) -> Diagnostic {
        let alias = validated.alias.to_string();
        let profile = &validated.profile;
        let current = self
            .profiles
            .definition(profile)
            .and_then(|d| d.digest.clone());
        if current.as_deref() == Some(pin) {
            return Diagnostic::pass(
                "profile.pinned",
                format!("{alias}: profile {profile} is the file it was composed against ({pin})"),
            );
        }
        let authorized = self
            .load_authorization(dir)
            .ok()
            .flatten()
            .is_some_and(|a| {
                a.revision.state == openwarrant_core::contract::RevisionState::Authorized
            });
        let message = format!(
            "{alias}: profile {profile} was pinned at {pin} and profiles/{profile}.toml is now {}. {}",
            current.as_deref().unwrap_or("absent"),
            if authorized {
                "The authorization signed the type as pinned; the type it now names is not \
                 the one signed. Restore the profile file, or amend the Warrant and \
                 re-authorize it"
            } else {
                "Unsigned, so the draft may be re-pinned to the file as it stands; once \
                 authorized this is an error"
            }
        );
        if authorized {
            Diagnostic::error("profile.pin-drift", manifest_file.to_owned(), message)
        } else {
            Diagnostic::warn("profile.pin-drift", manifest_file.to_owned(), message)
        }
    }

    /// What an extending profile asks of its Warrants beyond the manifest
    /// (OW-WAR-0140 M3): which definition it composed against, that its
    /// acceptance authority may perform the existing resolution act, and
    /// that its reference roles link rather than copy (§22.3).
    fn profile_checks(
        &self,
        dir: &Utf8Path,
        validated: &ValidatedManifest,
        definition: &ProfileDefinition,
        atoms: &[AtomSource],
        report: &mut Report,
    ) {
        let alias = validated.alias.to_string();
        let profile = &validated.profile;
        // Unanswered is a warning on a draft and an error once a human has
        // authorized the contract, as `atom.preset-unanswered` is.
        let authorized = self
            .load_authorization(dir)
            .ok()
            .flatten()
            .is_some_and(|a| {
                a.revision.state == openwarrant_core::contract::RevisionState::Authorized
            });
        let unanswered = |rule: &str, file: String, message: String| {
            if authorized {
                Diagnostic::error(rule, file, message)
            } else {
                Diagnostic::warn(rule, file, message)
            }
        };
        report.push(Diagnostic::pass(
            "profile.registered",
            format!(
                "{alias}: profile {profile} extends {} and is defined at {}",
                profile.core(),
                definition.digest.as_deref().unwrap_or("(built in)")
            ),
        ));
        let file_of = |role: &str| {
            atoms
                .iter()
                .find(|a| a.role == role)
                .map(|a| self.relative(&dir.join(&a.source)))
                .unwrap_or_default()
        };
        if !definition.approved {
            report.push(Diagnostic::warn(
                "profile.unapproved",
                format!("profiles/{profile}.toml"),
                format!(
                    "{alias}: profile {profile} is not approved (`approved = false`). Until it \
                     is, a Warrant of it links to, not replaces, the contractual and finance \
                     records (§22.3), and nothing in it is a legal, financial or quality \
                     instrument"
                ),
            ));
        }
        if let Some(role) = definition.acceptance_role.as_deref() {
            report.push(
                self.acceptance_authority(&alias, role, &file_of(role), atoms)
                    .unwrap_or_else(|message| {
                        unanswered("profile.acceptance-authority", file_of(role), message)
                    }),
            );
        }
        for role in &definition.reference_roles {
            let file = file_of(role);
            let Some(atom) = atoms.iter().find(|a| &a.role == role) else {
                continue;
            };
            let text = String::from_utf8_lossy(&atom.bytes);
            let refs = references(&text);
            if refs.is_empty() {
                report.push(unanswered(
                    "profile.reference-missing",
                    file,
                    format!(
                        "{alias}: {role} cites no reference. Its terms are linked, never \
                         copied (§22.3): name the contractual or finance record by URI \
                         (`kf://…`, `war://…`)"
                    ),
                ));
                continue;
            }
            for reference in refs {
                report.push(self.resolve_reference(&alias, role, &file, &reference));
            }
        }
    }

    /// The acceptance authority a profile's acceptance atom names, checked
    /// against the register with no new role: acceptance is the existing
    /// resolution act, so the actor must be a human holding `resolver` who
    /// did not perform the work (§27.1, §27.2). `Err` carries the message
    /// when the atom names nobody: unanswered, which the caller grades.
    fn acceptance_authority(
        &self,
        alias: &str,
        role: &str,
        file: &str,
        atoms: &[AtomSource],
    ) -> Result<Diagnostic, String> {
        const RULE: &str = "profile.acceptance-authority";
        let named = atoms
            .iter()
            .find(|a| a.role == role)
            .and_then(|a| {
                frontmatter::parse(&String::from_utf8_lossy(&a.bytes))
                    .ok()
                    .and_then(|fm| fm.scalar("acceptance_authority").map(str::to_owned))
            })
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());
        let Some(actor) = named else {
            return Err(format!(
                "{alias}: {role} names no `acceptance_authority` in its header; \
                 acceptance is a resolution, and somebody must be entitled to make it"
            ));
        };
        let register = match self.load_authority_register() {
            Ok(register) => register,
            Err(e) => {
                return Ok(Diagnostic::error(
                    RULE,
                    file,
                    format!("{alias}: the register could not be read: {e}"),
                ));
            }
        };
        let Some(assignment) = register.actor(&actor) else {
            return Ok(Diagnostic::error(
                RULE,
                file,
                format!(
                    "{alias}: acceptance authority {actor:?} has no assignment in \
                     docs/authority/roles.toml; only a human the register names may accept"
                ),
            ));
        };
        if assignment.actor_kind != ActorKind::Human {
            return Ok(Diagnostic::error(
                RULE,
                file,
                format!(
                    "{alias}: acceptance authority {actor:?} is {kind}-kind; acceptance is \
                     the resolution act, which only a human records here (§27.1, §27.2)",
                    kind = format!("{:?}", assignment.actor_kind).to_lowercase()
                ),
            ));
        }
        if actor == self.performer() {
            return Ok(Diagnostic::error(
                RULE,
                file,
                format!(
                    "{alias}: acceptance authority {actor:?} is the performer; nobody accepts \
                     their own delivery (§27.2)"
                ),
            ));
        }
        if !assignment.holds(ActorRole::Resolver) {
            return Ok(Diagnostic::error(
                RULE,
                file,
                format!(
                    "{alias}: acceptance authority {actor:?} does not hold `resolver` in \
                     docs/authority/roles.toml; acceptance is `war resolve`, and no other \
                     role grants it"
                ),
            ));
        }
        Ok(Diagnostic::pass(
            RULE,
            format!(
                "{alias}: acceptance authority {actor:?} is a human holding resolver; \
                 acceptance is `war resolve` by that actor"
            ),
        ))
    }

    /// One reference from a reference role: resolved, or UNKNOWN when this
    /// repository cannot answer (Law 15, U-004). Never a pass it did not see.
    fn resolve_reference(
        &self,
        alias: &str,
        role: &str,
        file: &str,
        reference: &str,
    ) -> Diagnostic {
        const RULE: &str = "profile.reference";
        let (scheme, rest) = reference.split_once("://").unwrap_or(("", reference));
        match scheme {
            "kf" => Diagnostic::unknown(
                RULE,
                file,
                format!(
                    "{alias}: {role} cites {reference}; no Knowledge Fabric is reachable from \
                     this repository, so whether it resolves is not known"
                ),
            ),
            "war" => {
                let found = self.warrant_dirs().ok().is_some_and(|dirs| {
                    dirs.iter().any(|d| {
                        d.file_name() == Some(rest)
                            || fs::read_to_string(d.join("manifest.toml"))
                                .is_ok_and(|t| t.contains(&format!("uuid = \"{rest}\"")))
                    })
                });
                if found {
                    Diagnostic::pass(
                        RULE,
                        format!("{alias}: {role} cites {reference}, a Warrant in this repository"),
                    )
                } else {
                    Diagnostic::unknown(
                        RULE,
                        file,
                        format!(
                            "{alias}: {role} cites {reference}, which is not in this repository; \
                             another may hold it"
                        ),
                    )
                }
            }
            _ => Diagnostic::unknown(
                RULE,
                file,
                format!("{alias}: {role} cites {reference}, which cannot be resolved offline"),
            ),
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

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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

/// Corrections on file for one Warrant (OW-WAR-0064), each with the
/// repository-relative path it was read from — the path is what `war check`
/// digests against the journal's `record_digest`.
#[derive(Debug, Default)]
pub struct CorrectionSet {
    pub records: Vec<(String, openwarrant_core::correction::CorrectionRecord)>,
    /// `(repository-relative path, why it would not parse)`.
    pub failures: Vec<(String, String)>,
}

impl CorrectionSet {
    /// The corrections for one deliverable, in file order.
    pub fn for_deliverable(&self, id: &str) -> Vec<openwarrant_core::correction::Correction> {
        self.records
            .iter()
            .filter(|(_, r)| r.correction.deliverable_id == id)
            .map(|(_, r)| r.correction.clone())
            .collect()
    }
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
    /// The capabilities this Warrant's kind selects (OW-ADR-0031), read from
    /// `registry`. A manifest that did not validate has no kind; it gets
    /// every capability, which is the path every Warrant took before kinds
    /// had capabilities, so nothing it reports goes quiet.
    #[must_use]
    pub fn capabilities(
        &self,
        registry: &openwarrant_core::role::ProfileRegistry,
    ) -> openwarrant_core::Capabilities {
        self.validated
            .as_ref()
            .map_or(openwarrant_core::Capabilities::ALL, |v| {
                registry.capabilities(&v.profile)
            })
    }

    /// `Err` naming the absent capability, by rule, when this Warrant's kind
    /// does not select `cap`: the refusal `war authorize`, `war verify`, `war
    /// resolve` and `war sign` give a kind that lacks the act's capability.
    pub fn require(
        &self,
        registry: &openwarrant_core::role::ProfileRegistry,
        cap: openwarrant_core::Capability,
    ) -> Result<(), RepoError> {
        if self.capabilities(registry).has(cap) {
            return Ok(());
        }
        let profile = self
            .validated
            .as_ref()
            .map(|v| v.profile.to_string())
            .unwrap_or_default();
        Err(RepoError::Message(capability_absent(
            &self.alias(),
            &profile,
            cap,
        )))
    }

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

/// The `sas_revision` the latest amendment under `amendments/` names, with the
/// file that names it (OW-ADR-0016). Read from the file, not the record
/// struct: the struct is pinned by a resolved Warrant, and the pin is one
/// top-level scalar on the side. "Latest" is `crate::amendment_id`'s order —
/// ordinal (`AM-<n>` or `AM-<n>-<hash>`), then `effective_time`, then name —
/// so `AM-1000` follows `AM-901`, and of two `AM-004-<hash>` re-pins merged
/// from two branches the later-dated one decides. Only an unindented
/// `sas_revision:` line counts — a key nested under `semantic_diff:` is not
/// the pin.
#[must_use]
pub fn amendment_sas_revision(dir: &Utf8Path) -> Option<(String, Utf8PathBuf)> {
    let files: Vec<Utf8PathBuf> = crate::amendment_id::files(dir)
        .into_iter()
        .map(|f| f.path)
        .filter(|p| p.extension() == Some("yaml"))
        .collect();
    files.into_iter().rev().find_map(|path| {
        let text = fs::read_to_string(&path).ok()?;
        let version = text.lines().find_map(|line| {
            let v = line
                .strip_prefix("sas_revision:")?
                .trim()
                .trim_matches('"')
                .trim();
            (!v.is_empty()).then(|| v.to_owned())
        })?;
        Some((version, path))
    })
}

/// The schema a Markdown atom's header names (SAS §62).
const ATOM_SCHEMA: &str = "oh.war/atom/v1";

/// `atom.header` (OW-WAR-0122): where a Markdown atom's header disagrees with
/// the manifest entry that declares it, one sentence per key.
///
/// The manifest decides composition (§61.1); the header only restates it.
/// Before this rule the restatement was read for `jurisdiction` and nothing
/// else, so an atom copied in from another Warrant, or one whose header named
/// another role, compiled as whatever the manifest said it was. A restatement
/// nobody compares is a second source of truth that can silently disagree.
///
/// `order` is compared as a number, so `order: 05` restates ordinal 5; a
/// value that is not a number is a different value, not a missing one.
/// Unknown keys are not this rule's business: the reader keeps them (§62.3).
fn header_mismatches(
    fm: &frontmatter::Frontmatter,
    manifest: &Manifest,
    entry: &openwarrant_core::AtomEntry,
) -> Vec<String> {
    let ordinal = entry.ordinal.to_string();
    let expected: [(&str, &str, &str); 4] = [
        ("schema", ATOM_SCHEMA, "the atom schema"),
        (
            "warrant_uuid",
            manifest.uuid.as_str(),
            "the manifest's `uuid`",
        ),
        ("role", entry.role.as_str(), "the manifest's role"),
        ("order", ordinal.as_str(), "the manifest's ordinal"),
    ];
    let mut out = Vec::new();
    for (key, want, whose) in expected {
        let found = match fm.get(key) {
            None => {
                out.push(format!(
                    "the header has no `{key}`; {whose} is {want:?} (declared at ordinal {})",
                    entry.ordinal
                ));
                continue;
            }
            Some(frontmatter::Value::List(items)) => format!("a list {items:?}"),
            Some(frontmatter::Value::Scalar(s)) => {
                let same = if key == "order" {
                    s.parse::<u32>().is_ok_and(|n| n == entry.ordinal)
                } else {
                    s == want
                };
                if same {
                    continue;
                }
                format!("{s:?}")
            }
        };
        out.push(format!(
            "the header's `{key}` is {found}, but {whose} is {want:?} (declared at ordinal {})",
            entry.ordinal
        ));
    }
    out
}

/// The words of a `capability.absent` refusal (OW-ADR-0031).
#[must_use]
pub fn capability_absent(alias: &str, profile: &str, cap: openwarrant_core::Capability) -> String {
    format!(
        "capability.absent: {alias}: profile {profile} does not select the `{cap}` \
         capability, so nothing of it is {}. Its kind is data: profiles/{profile}.toml \
         (OW-ADR-0031)",
        match cap {
            openwarrant_core::Capability::Authorization => "authorized",
            openwarrant_core::Capability::Verification => "verified",
            openwarrant_core::Capability::Resolution => "resolved",
            openwarrant_core::Capability::Evidence => "recorded as evidence",
            openwarrant_core::Capability::Stages => "dispatched as a stage",
            _ => "read under it",
        }
    )
}

/// `profiles/*.toml` under `root`, read into a registry (OW-WAR-0140). No
/// directory means the two core profiles and nothing else. A definition the
/// registry refuses refuses the repository: a Warrant of that profile would
/// otherwise read as having an unknown profile, which is not what is wrong.
fn load_profiles(root: &Utf8Path) -> Result<ProfileRegistry, RepoError> {
    let dir = root.join("profiles");
    if !fs::is_dir(&dir) {
        return Ok(ProfileRegistry::builtin());
    }
    let entries = fs::read_dir(&dir).map_err(|source| RepoError::Io {
        context: format!("could not read {dir}"),
        source,
    })?;
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
            continue;
        };
        if path.extension() == Some("toml") && fs::is_file(&path) {
            let bytes = fs::read(&path).map_err(|source| RepoError::Io {
                context: format!("could not read {path}"),
                source,
            })?;
            files.push((
                format!("profiles/{}", path.file_name().unwrap_or_default()),
                bytes,
            ));
        }
    }
    files.sort();
    ProfileRegistry::with_definitions(files.iter().map(|(f, b)| (f.as_str(), b.as_slice())))
        .map_err(|e| RepoError::Message(format!("{}: {e}", e.rule())))
}

/// Every `scheme://…` token in an atom's text, in order, once each.
fn references(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for token in text.split(|c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '`' | '<' | '>' | '(' | ')' | '[' | ']' | '"' | '\'' | ','
            )
    }) {
        let token = token.trim_end_matches(['.', ';', ':']);
        let Some((scheme, rest)) = token.split_once("://") else {
            continue;
        };
        let scheme_ok = scheme
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase())
            && scheme.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '+' | '.' | '-')
            });
        if scheme_ok && !rest.is_empty() && !out.iter().any(|r| r == token) {
            out.push(token.to_owned());
        }
    }
    out
}

/// Parsed SAS revisions this process has already read, per revisions
/// directory, with the exact (path, text) pairs they were parsed from
/// (`Repository::load_sas_revisions`).
type SasRevisionMemo = std::collections::HashMap<
    Utf8PathBuf,
    (
        Vec<(Utf8PathBuf, String)>,
        Vec<openwarrant_core::SasRevision>,
    ),
>;
static SAS_REVISIONS: std::sync::OnceLock<std::sync::Mutex<SasRevisionMemo>> =
    std::sync::OnceLock::new();

/// Authority registers this process has already parsed, per path, with the
/// exact text each was parsed from (`Repository::load_authority_register`).
type RegisterMemo = std::collections::HashMap<Utf8PathBuf, (String, AuthorityRegister)>;
static REGISTERS: std::sync::OnceLock<std::sync::Mutex<RegisterMemo>> = std::sync::OnceLock::new();

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch repository with this corpus's `openwarrant.toml` (t-280c).
    fn scratch_repo(name: &str) -> (Utf8PathBuf, Repository) {
        let dir = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!("war-repo-memo-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("docs/sas/revisions")).unwrap();
        fs::create_dir_all(dir.join("docs/authority")).unwrap();
        fs::copy(
            corpus().join("openwarrant.toml"),
            dir.join("openwarrant.toml"),
        )
        .unwrap();
        let repo = Repository::open(dir.clone()).expect("scratch repository opens");
        (dir, repo)
    }

    fn corpus() -> Utf8PathBuf {
        Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize_utf8()
            .unwrap()
    }

    /// t-280c: the parsed SAS revisions are shared within a process, but a
    /// revision added, edited or broken after the first read is seen on the
    /// next one — the memo is keyed by the bytes, never a stale answer, and a
    /// file that no longer parses is refused rather than answered from memory.
    #[test]
    fn the_sas_revision_memo_follows_every_edit_and_refuses_a_broken_file() {
        let (dir, repo) = scratch_repo("sas");
        let revs = dir.join("docs/sas/revisions");
        let src = corpus().join("docs/sas/revisions");
        fs::copy(src.join("1.0.0.toml"), revs.join("1.0.0.toml")).unwrap();
        let first = repo.load_sas_revisions().unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(
            repo.load_sas_revisions().unwrap(),
            first,
            "a repeat read agrees"
        );

        fs::copy(src.join("1.1.0.toml"), revs.join("1.1.0.toml")).unwrap();
        let versions: Vec<String> = repo
            .load_sas_revisions()
            .unwrap()
            .into_iter()
            .map(|r| r.version)
            .collect();
        assert_eq!(versions, ["1.0.0", "1.1.0"], "an added revision is seen");

        let text = fs::read_to_string(revs.join("1.1.0.toml")).unwrap();
        fs::write(revs.join("1.1.0.toml"), format!("{text}\n[[[not toml")).unwrap();
        assert!(
            repo.load_sas_revisions().is_err(),
            "a revision broken after it was read is refused, not remembered"
        );
        fs::write(revs.join("1.1.0.toml"), &text).unwrap();
        assert_eq!(repo.load_sas_revisions().unwrap().len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    /// t-280c: the same for the authority register — a grant removed after
    /// the first read is gone on the next, and a register that stops parsing
    /// is refused, however many times the old one was read.
    #[test]
    fn the_register_memo_follows_every_edit_and_refuses_a_broken_register() {
        let (dir, repo) = scratch_repo("register");
        let roles = dir.join("docs/authority/roles.toml");
        let one = |actor: &str| {
            format!(
                "[[assignment]]\nactor = \"{actor}\"\nactor_kind = \"human\"\n\
                 roles = [\"authorizer\"]\nassigned_by = \"{actor}\"\n\
                 effective_time = \"2026-01-01T00:00:00Z\"\n"
            )
        };
        fs::write(&roles, format!("{}\n{}", one("ada"), one("bob"))).unwrap();
        let reg = repo.load_authority_register().unwrap();
        assert!(reg.actor("ada").is_some() && reg.actor("bob").is_some());
        let again = repo.load_authority_register().unwrap();
        assert!(again.actor("bob").is_some());

        fs::write(&roles, one("ada")).unwrap();
        let reg = repo.load_authority_register().unwrap();
        assert!(reg.actor("ada").is_some());
        assert!(
            reg.actor("bob").is_none(),
            "a withdrawn grant is not remembered"
        );

        fs::write(
            &roles,
            format!("{}\n[[assignment]]\nactor = 1\n", one("ada")),
        )
        .unwrap();
        assert!(
            repo.load_authority_register().is_err(),
            "a register broken after it was read is refused, not remembered"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    const UUID: &str = "01a0d04c-5ee5-7ba2-9dc3-b9c50dcc6ba1";

    fn manifest() -> Manifest {
        toml::from_str(&format!(
            "schema = \"oh.war/manifest/v1\"\nuuid = \"{UUID}\"\nlocal_alias = \"T-WAR-0001\"\n\
             title = \"t\"\nprofile = \"delivery\"\nassurance_level = \"basic\"\n\n\
             [[atoms]]\nordinal = 10\nrole = \"intent\"\npath = \"atoms/10-intent.md\"\nrequired = true\n"
        ))
        .unwrap()
    }

    fn mismatches(header: &str) -> Vec<String> {
        let m = manifest();
        let fm = frontmatter::parse(&format!("---\n{header}---\n\n# Intent\n")).unwrap();
        header_mismatches(&fm, &m, &m.atoms[0])
    }

    #[test]
    fn a_header_that_restates_its_manifest_entry_passes() {
        let ok = format!(
            "schema: oh.war/atom/v1\nwarrant_uuid: {UUID}\nrole: intent\norder: 10\nx.note: kept\n"
        );
        assert_eq!(mismatches(&ok), Vec::<String>::new());
        // `order` is a number, not a string.
        let padded =
            format!("schema: oh.war/atom/v1\nwarrant_uuid: {UUID}\nrole: intent\norder: 010\n");
        assert_eq!(mismatches(&padded), Vec::<String>::new());
    }

    #[test]
    fn each_key_is_named_with_both_values_or_as_missing() {
        let bad = format!("schema: oh.war/atom/v1\nwarrant_uuid: {UUID}\nrole: basis\norder: 20\n");
        let got = mismatches(&bad);
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(got[0].contains("`role` is \"basis\"") && got[0].contains("\"intent\""));
        assert!(got[1].contains("`order` is \"20\"") && got[1].contains("\"10\""));

        let missing = "role: intent\norder: 10\n";
        let got = mismatches(missing);
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(got[0].contains("no `schema`"));
        assert!(got[1].contains("no `warrant_uuid`"));
    }

    #[test]
    fn a_list_where_a_scalar_belongs_is_a_different_value() {
        let bad = format!(
            "schema: oh.war/atom/v1\nwarrant_uuid: {UUID}\nrole:\n  - intent\norder: nine\n"
        );
        let got = mismatches(&bad);
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(got[0].contains("`role` is a list"));
        assert!(got[1].contains("`order` is \"nine\""));
    }
}
