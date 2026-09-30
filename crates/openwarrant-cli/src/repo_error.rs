// SPDX-License-Identifier: AGPL-3.0-or-later
//! Structured failures shared by repository discovery and contained-file I/O.

use std::fmt;

use camino::Utf8PathBuf;

use crate::init::CONFIG_FILE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GateReceiptCommitState {
    DurabilityUnknown,
    CleanupIncomplete,
    ContainmentChanged,
}

impl fmt::Display for GateReceiptCommitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DurabilityUnknown => "durability_unknown",
            Self::CleanupIncomplete => "cleanup_incomplete",
            Self::ContainmentChanged => "containment_changed",
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
    GateReceiptCommitted {
        path: Utf8PathBuf,
        state: GateReceiptCommitState,
        detail: String,
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
            Self::GateReceiptCommitted {
                path,
                state,
                detail,
            } => write!(
                f,
                "Gate receipt {path} reached commit point with state {state}: {detail}"
            ),
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
