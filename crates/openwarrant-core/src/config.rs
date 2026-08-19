// SPDX-License-Identifier: AGPL-3.0-or-later
//! Repository configuration — `openwarrant.toml` (SAS §60).
//!
//! This crate defines and validates the shape; the CLI reads the bytes (§79.1).

use serde::{Deserialize, Serialize};

/// The only repository-config schema this build understands.
pub const REPOSITORY_CONFIG_SCHEMA: &str = "oh.war/repository-config/v1";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error(
        "unknown repository-config schema {found:?}; this build understands \
         {expected:?} (SAS §69.3 — a breaking protocol change is not silently accepted)"
    )]
    UnknownSchema { found: String, expected: String },
    #[error("project.namespace is empty")]
    NamespaceEmpty,
    #[error(
        "project.namespace {found:?} is malformed; expected uppercase ASCII \
         letters, digits, or hyphens (it prefixes every local alias)"
    )]
    NamespaceMalformed { found: String },
    #[error("project.name is empty")]
    ProjectNameEmpty,
    #[error("paths.{field} is empty")]
    PathEmpty { field: &'static str },
    #[error("paths.{field} is {value:?}; configured paths must be relative to the repository root")]
    PathNotRelative { field: &'static str, value: String },
}

/// A validated project namespace, e.g. `OW`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Namespace(String);

impl Namespace {
    /// Parse a namespace.
    ///
    /// Surrounding whitespace is **trimmed, not rejected**: `" OW "` is accepted
    /// and stored as `"OW"`. This matches [`crate::LocalAlias::parse`], and the
    /// two must agree — a namespace that normalised differently from the alias
    /// prefix it is compared against would make
    /// [`crate::LocalAlias::parse_in`] reject correct input.
    pub fn parse(raw: &str) -> Result<Self, ConfigError> {
        let value = raw.trim();
        if value.is_empty() {
            return Err(ConfigError::NamespaceEmpty);
        }
        let well_formed = value
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
            && !value.starts_with('-')
            && !value.ends_with('-');
        if !well_formed {
            return Err(ConfigError::NamespaceMalformed {
                found: value.to_owned(),
            });
        }
        Ok(Self(value.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub namespace: Namespace,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knowledge_fabric_project_ref: Option<String>,
}

/// Where the controlled document trees live (§59, §60).
///
/// Paths are configurable and semantics are NOT inferred from them (§59):
/// a directory is the warrant tree because this field says so, not because it
/// happens to be named `warrants`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paths {
    #[serde(default = "Paths::default_sas")]
    pub sas: String,
    #[serde(default = "Paths::default_roadmap")]
    pub roadmap: String,
    #[serde(default = "Paths::default_adrs")]
    pub adrs: String,
    #[serde(default = "Paths::default_warrants")]
    pub warrants: String,
}

impl Paths {
    fn default_sas() -> String {
        "docs/sas".to_owned()
    }
    fn default_roadmap() -> String {
        "docs/roadmap".to_owned()
    }
    fn default_adrs() -> String {
        "docs/adr".to_owned()
    }
    fn default_warrants() -> String {
        "docs/warrants".to_owned()
    }

    fn validate(&self) -> Result<(), ConfigError> {
        for (field, value) in [
            ("sas", &self.sas),
            ("roadmap", &self.roadmap),
            ("adrs", &self.adrs),
            ("warrants", &self.warrants),
        ] {
            if value.trim().is_empty() {
                return Err(ConfigError::PathEmpty { field });
            }
            // An absolute path in repository configuration makes the repository
            // unclonable: it would resolve to one machine's layout.
            //
            // The `:` test catches a Windows drive prefix (`C:\...`). It is
            // deliberately coarse and will also reject a *relative* POSIX path
            // containing a colon, such as `docs:v2` — legal on Unix, and refused
            // here anyway. That trade is taken knowingly: a colon in a
            // configured document path is far more likely to be a drive letter
            // or a URI fragment than an intended directory name, and being
            // wrong in this direction costs a rename, while being wrong in the
            // other direction produces a repository that only builds on one
            // machine.
            if value.starts_with('/') || value.contains(':') {
                return Err(ConfigError::PathNotRelative {
                    field,
                    value: value.clone(),
                });
            }
        }
        Ok(())
    }
}

impl Default for Paths {
    fn default() -> Self {
        Self {
            sas: Self::default_sas(),
            roadmap: Self::default_roadmap(),
            adrs: Self::default_adrs(),
            warrants: Self::default_warrants(),
        }
    }
}

/// Generated-view policy (§59.2).
///
/// `commit = true, verify_drift = true` is what makes §17.3 enforceable: the
/// compiled parents are in Git, so a fresh compilation can be compared against
/// them. Setting `commit = false` is permitted by the SAS and removes that
/// comparison — the authority of the sources is unchanged either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedPolicy {
    #[serde(default = "GeneratedPolicy::yes")]
    pub commit: bool,
    #[serde(default = "GeneratedPolicy::yes")]
    pub verify_drift: bool,
}

impl GeneratedPolicy {
    fn yes() -> bool {
        true
    }
}

impl Default for GeneratedPolicy {
    fn default() -> Self {
        Self {
            commit: true,
            verify_drift: true,
        }
    }
}

/// `openwarrant.toml` (§60).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryConfig {
    pub schema: String,
    pub project: Project,
    #[serde(default)]
    pub paths: Paths,
    #[serde(default)]
    pub generated: GeneratedPolicy,
}

impl RepositoryConfig {
    /// Construct the configuration `war init` writes for a new repository.
    #[must_use]
    pub fn new(name: impl Into<String>, namespace: Namespace) -> Self {
        Self {
            schema: REPOSITORY_CONFIG_SCHEMA.to_owned(),
            project: Project {
                name: name.into(),
                namespace,
                knowledge_fabric_project_ref: None,
            },
            paths: Paths::default(),
            generated: GeneratedPolicy::default(),
        }
    }

    /// Fail-closed validation (§91.1 test 4).
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.schema != REPOSITORY_CONFIG_SCHEMA {
            return Err(ConfigError::UnknownSchema {
                found: self.schema.clone(),
                expected: REPOSITORY_CONFIG_SCHEMA.to_owned(),
            });
        }
        if self.project.name.trim().is_empty() {
            return Err(ConfigError::ProjectNameEmpty);
        }
        self.paths.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> RepositoryConfig {
        RepositoryConfig::new("OpenWarrant", Namespace::parse("OW").expect("valid"))
    }

    #[test]
    fn default_config_validates() {
        assert_eq!(valid().validate(), Ok(()));
    }

    #[test]
    fn default_generated_policy_commits_and_verifies() {
        let config = valid();
        assert!(config.generated.commit);
        assert!(config.generated.verify_drift, "drift check must default on");
    }

    /// §69.3 / §91.1 test 4: an unrecognised schema is refused, not ignored.
    #[test]
    fn unknown_schema_fails_closed() {
        let mut config = valid();
        config.schema = "oh.war/repository-config/v2".to_owned();
        assert_eq!(
            config.validate(),
            Err(ConfigError::UnknownSchema {
                found: "oh.war/repository-config/v2".to_owned(),
                expected: REPOSITORY_CONFIG_SCHEMA.to_owned(),
            })
        );
    }

    #[test]
    fn namespace_rules() {
        assert!(Namespace::parse("OW").is_ok());
        assert!(Namespace::parse("OPEN-HUMAN").is_ok());
        assert!(Namespace::parse("OW2").is_ok());
        for bad in ["", "  ", "ow", "-OW", "OW-", "O W", "OW_X"] {
            assert!(Namespace::parse(bad).is_err(), "expected {bad:?} refused");
        }
    }

    #[test]
    fn absolute_paths_are_refused() {
        let mut config = valid();
        config.paths.warrants = "/mnt/4tb/OpenWarrant/docs/warrants".to_owned();
        assert_eq!(
            config.validate(),
            Err(ConfigError::PathNotRelative {
                field: "warrants",
                value: "/mnt/4tb/OpenWarrant/docs/warrants".to_owned(),
            })
        );
    }

    #[test]
    fn empty_path_is_refused() {
        let mut config = valid();
        config.paths.adrs = "  ".to_owned();
        assert_eq!(
            config.validate(),
            Err(ConfigError::PathEmpty { field: "adrs" })
        );
    }
}
