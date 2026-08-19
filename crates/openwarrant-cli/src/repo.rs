// SPDX-License-Identifier: AGPL-3.0-or-later
//! Repository discovery and loading — the I/O half the core crate refuses (§79.1, §79.4).

use std::fmt;
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::{AtomSource, CompilationBasis};
use openwarrant_core::{
    AdrError, AdrRecord, Manifest, RepositoryConfig, ValidatedManifest, frontmatter,
};

use crate::diagnostic::{Diagnostic, Report};
use crate::init::CONFIG_FILE;

#[derive(Debug)]
pub enum RepoError {
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

        Ok(Loaded {
            dir: dir.to_owned(),
            basis: Some(CompilationBasis {
                manifest,
                manifest_source: relative_manifest,
                manifest_bytes,
                atoms,
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
