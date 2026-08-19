// SPDX-License-Identifier: AGPL-3.0-or-later
//! WAR identity: UUIDv7 (§12.2), local alias (§12.3), enterprise ID (§12.4).
//!
//! The three layers exist because they answer different questions and have
//! different authorities. The UUID is what the record *is*; it is minted offline
//! and never changes. The local alias is what humans in one repository *call* it;
//! it is convenient, repository-scoped, and explicitly NOT a global identifier
//! (§12.1, RQ-002). The enterprise ID is what the organization officially
//! recognises, and only Knowledge Fabric may allocate it (RQ-003).
//!
//! Conflating these is the failure this module exists to prevent: SAS §91.3
//! test 18 requires that two repositories using the same local alias do not
//! collide, and test 20 requires that an official enterprise ID cannot be
//! fabricated locally.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Errors from constructing identity values.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IdentityError {
    #[error("local alias is empty")]
    AliasEmpty,
    #[error(
        "local alias {alias:?} is malformed; expected <NAMESPACE>-WAR-<NNNN> \
         with an uppercase namespace and at least four digits"
    )]
    AliasMalformed { alias: String },
    #[error("local alias {alias:?} has namespace {found:?}, but this repository is {expected:?}")]
    AliasNamespaceMismatch {
        alias: String,
        found: String,
        expected: String,
    },
    #[error("{value:?} is not a valid UUID: {source}")]
    UuidMalformed {
        value: String,
        #[source]
        source: uuid::Error,
    },
    #[error(
        "UUID {value} is version {found}, but WAR identity requires UUIDv7 (SAS §12.2, RQ-001)"
    )]
    UuidNotV7 { value: Uuid, found: usize },
}

/// The immutable global identity of a WAR (§12.2, RQ-001).
///
/// Minted offline at creation. Registration with Knowledge Fabric adds an
/// enterprise identifier alongside this value; it never replaces it (§12.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WarUuid(Uuid);

impl WarUuid {
    /// Mint a new identity.
    ///
    /// v7 rather than v4 so identities sort by creation time, which makes an
    /// append-only journal and a listing of warrants agree on order without a
    /// separate sequence number.
    ///
    /// Named `mint`, not `new`, and deliberately WITHOUT a `Default` impl.
    /// Clippy suggests `Default` for any argument-less `new`, but a `Default`
    /// that returns a different value on every call is a trap: a containing
    /// struct deriving `Default` would silently mint a fresh identity for a
    /// record that already has one. Identity allocation must be an explicit act.
    #[must_use]
    pub fn mint() -> Self {
        Self(Uuid::now_v7())
    }

    /// Adopt an existing UUID, rejecting any version other than 7.
    ///
    /// Fail-closed rather than accepting and normalising: a v4 UUID in a
    /// manifest means the record was minted by something that does not follow
    /// this protocol, and silently accepting it would lose that signal.
    pub fn from_uuid(value: Uuid) -> Result<Self, IdentityError> {
        match value.get_version_num() {
            7 => Ok(Self(value)),
            found => Err(IdentityError::UuidNotV7 { value, found }),
        }
    }

    #[must_use]
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for WarUuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for WarUuid {
    type Err = IdentityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parsed = Uuid::parse_str(s).map_err(|source| IdentityError::UuidMalformed {
            value: s.to_owned(),
            source,
        })?;
        Self::from_uuid(parsed)
    }
}

/// A repository-scoped human label such as `OW-WAR-0001` (§12.3).
///
/// Deliberately NOT globally unique. Two repositories may both hold
/// `OW-WAR-0001` and they are different records; only the UUID distinguishes
/// them (RQ-002, §91.3 test 18).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LocalAlias(String);

impl LocalAlias {
    /// Parse an alias of the form `<NAMESPACE>-WAR-<NNNN>`.
    pub fn parse(raw: &str) -> Result<Self, IdentityError> {
        let alias = raw.trim();
        if alias.is_empty() {
            return Err(IdentityError::AliasEmpty);
        }

        let malformed = || IdentityError::AliasMalformed {
            alias: alias.to_owned(),
        };

        // Split from the RIGHT: a namespace may legitimately contain a hyphen
        // (`OPEN-HUMAN-WAR-0001`), so splitting from the left would truncate it.
        let (namespace, rest) = alias.rsplit_once("-WAR-").ok_or_else(malformed)?;

        if namespace.is_empty()
            || !namespace
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(malformed());
        }
        if rest.len() < 4 || !rest.chars().all(|c| c.is_ascii_digit()) {
            return Err(malformed());
        }

        Ok(Self(alias.to_owned()))
    }

    /// Parse and additionally require the alias to belong to `namespace`.
    ///
    /// A manifest whose alias namespace disagrees with the repository's
    /// configured namespace is a copied-and-not-renamed record, which is exactly
    /// how two warrants come to share an ordinal.
    pub fn parse_in(raw: &str, namespace: &str) -> Result<Self, IdentityError> {
        let alias = Self::parse(raw)?;
        let found = alias.namespace();
        if found != namespace {
            return Err(IdentityError::AliasNamespaceMismatch {
                alias: alias.0.clone(),
                found: found.to_owned(),
                expected: namespace.to_owned(),
            });
        }
        Ok(alias)
    }

    /// The namespace segment, e.g. `OW` in `OW-WAR-0001`.
    #[must_use]
    pub fn namespace(&self) -> &str {
        self.0
            .rsplit_once("-WAR-")
            .map(|(ns, _)| ns)
            .unwrap_or(&self.0)
    }

    /// The zero-padded ordinal segment, e.g. `0001` in `OW-WAR-0001`.
    #[must_use]
    pub fn ordinal_str(&self) -> &str {
        self.0.rsplit_once("-WAR-").map(|(_, n)| n).unwrap_or("")
    }

    /// The ordinal as a number, if it fits.
    #[must_use]
    pub fn ordinal(&self) -> Option<u64> {
        self.ordinal_str().parse().ok()
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LocalAlias {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minted_identity_is_v7() {
        let id = WarUuid::mint();
        assert_eq!(id.as_uuid().get_version_num(), 7);
    }

    #[test]
    fn minted_identities_are_distinct() {
        assert_ne!(WarUuid::mint(), WarUuid::mint());
    }

    /// RQ-001: identity is UUIDv7. A v4 UUID is rejected rather than adopted.
    #[test]
    fn non_v7_uuid_is_refused() {
        let v4 = Uuid::parse_str("f47ac10b-58cc-4372-a567-0e02b2c3d479").expect("valid uuid");
        assert_eq!(
            WarUuid::from_uuid(v4),
            Err(IdentityError::UuidNotV7 {
                value: v4,
                found: 4
            })
        );
    }

    #[test]
    fn alias_round_trips() {
        let alias = LocalAlias::parse("OW-WAR-0001").expect("valid alias");
        assert_eq!(alias.namespace(), "OW");
        assert_eq!(alias.ordinal_str(), "0001");
        assert_eq!(alias.ordinal(), Some(1));
        assert_eq!(alias.as_str(), "OW-WAR-0001");
    }

    /// A hyphenated namespace must survive parsing; splitting from the left
    /// would report the namespace as `OPEN`.
    #[test]
    fn hyphenated_namespace_survives() {
        let alias = LocalAlias::parse("OPEN-HUMAN-WAR-0042").expect("valid alias");
        assert_eq!(alias.namespace(), "OPEN-HUMAN");
        assert_eq!(alias.ordinal(), Some(42));
    }

    #[test]
    fn malformed_aliases_are_refused() {
        for bad in [
            "",
            "   ",
            "OW-0001",     // missing the -WAR- infix
            "ow-WAR-0001", // lowercase namespace
            "OW-WAR-1",    // fewer than four digits
            "OW-WAR-00a1", // non-digit ordinal
            "-WAR-0001",   // empty namespace
        ] {
            assert!(
                LocalAlias::parse(bad).is_err(),
                "expected {bad:?} to be refused"
            );
        }
    }

    #[test]
    fn namespace_mismatch_is_refused() {
        let err = LocalAlias::parse_in("LIM-WAR-0001", "OW").expect_err("should mismatch");
        assert_eq!(
            err,
            IdentityError::AliasNamespaceMismatch {
                alias: "LIM-WAR-0001".to_owned(),
                found: "LIM".to_owned(),
                expected: "OW".to_owned(),
            }
        );
    }

    /// §91.3 test 18: the same alias in two repositories is two records.
    /// The alias is not an identity, so it cannot be used to compare them.
    #[test]
    fn same_alias_different_repositories_are_distinct_records() {
        let alias_a = LocalAlias::parse("OW-WAR-0001").expect("valid");
        let alias_b = LocalAlias::parse("OW-WAR-0001").expect("valid");
        assert_eq!(alias_a, alias_b, "aliases themselves compare equal");

        let record_a = WarUuid::mint();
        let record_b = WarUuid::mint();
        assert_ne!(
            record_a, record_b,
            "identical aliases must not imply identical records"
        );
    }
}
