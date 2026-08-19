// SPDX-License-Identifier: AGPL-3.0-or-later
//! Digest domains and domain-separated preimages (SAS §65).
//!
//! §65.2 requires that hashing operate on a domain-separated preimage:
//!
//! ```json
//! { "digest_domain": "oh.war/contract/v1", "payload": { } }
//! ```
//!
//! The reason is stated in the SAS and is worth repeating where the code lives:
//! two different semantic objects can serialise to identical JSON. A contract
//! and an artifact record that happen to share a shape would otherwise hash to
//! the same value, and a digest that collides across domains cannot be used to
//! prove which of the two was signed.
//!
//! # What is deliberately missing
//!
//! `sha256_hex` hashes bytes. It does NOT canonicalize a JSON value, because
//! canonicalization is RFC 8785 (§65.2) and the implementation of RFC 8785 binds
//! the wire format for every cross-system digest OpenWarrant will ever mint.
//! Choosing that library is an implementation ADR (§80), scheduled before
//! OW-WAR-0003 lands. Shipping a placeholder canonicalizer now would mint
//! digests that a later correction would silently invalidate.

use sha2::{Digest, Sha256};

/// The digest domains a conforming implementation must compute (§65).
///
/// All fifteen are listed here even though Phase 1 computes only a few: the
/// vocabulary is protocol surface, and a later phase adding a domain should be
/// an edit to this enum rather than a new string literal invented at a call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DigestDomain {
    AtomSource,
    Manifest,
    CompositionRevision,
    WorkspaceBasis,
    SemanticGraph,
    Contract,
    ContextManifest,
    Dispatch,
    AttemptBasis,
    Artifact,
    GateBinding,
    GateRun,
    AssuranceCaseSnapshot,
    Resolution,
    WarExport,
}

impl DigestDomain {
    /// Every domain, in declaration order. Used by conformance tests.
    pub const ALL: [Self; 15] = [
        Self::AtomSource,
        Self::Manifest,
        Self::CompositionRevision,
        Self::WorkspaceBasis,
        Self::SemanticGraph,
        Self::Contract,
        Self::ContextManifest,
        Self::Dispatch,
        Self::AttemptBasis,
        Self::Artifact,
        Self::GateBinding,
        Self::GateRun,
        Self::AssuranceCaseSnapshot,
        Self::Resolution,
        Self::WarExport,
    ];

    /// The stable URI written into the preimage's `digest_domain` field.
    #[must_use]
    pub const fn as_uri(self) -> &'static str {
        match self {
            Self::AtomSource => "oh.war/atom-source/v1",
            Self::Manifest => "oh.war/manifest/v1",
            Self::CompositionRevision => "oh.war/composition-revision/v1",
            Self::WorkspaceBasis => "oh.war/workspace-basis/v1",
            Self::SemanticGraph => "oh.war/semantic-graph/v1",
            Self::Contract => "oh.war/contract/v1",
            Self::ContextManifest => "oh.war/context-manifest/v1",
            Self::Dispatch => "oh.war/dispatch/v1",
            Self::AttemptBasis => "oh.war/attempt-basis/v1",
            Self::Artifact => "oh.war/artifact/v1",
            Self::GateBinding => "oh.war/gate-binding/v1",
            Self::GateRun => "oh.war/gate-run/v1",
            Self::AssuranceCaseSnapshot => "oh.war/assurance-case-snapshot/v1",
            Self::Resolution => "oh.war/resolution/v1",
            Self::WarExport => "oh.war/war-export/v1",
        }
    }
}

/// SHA-256 over already-canonical bytes, lowercase hex.
///
/// §65.1: cross-system WAR digests use SHA-256 unless a later protocol revision
/// says otherwise, and the algorithm is always explicit.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    let mut hex = String::with_capacity(out.len() * 2);
    for byte in out {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// §65 lists fifteen domains. If the SAS grows one, this fails and the
    /// vocabulary gets updated deliberately rather than drifting.
    #[test]
    fn all_fifteen_domains_are_present() {
        assert_eq!(DigestDomain::ALL.len(), 15);
    }

    /// §91.1 test 6: different digest domains produce different preimages.
    ///
    /// Enforced here at its root: no two domains may share a URI. If they did,
    /// domain separation would be decorative.
    #[test]
    fn domain_uris_are_unique() {
        let uris: BTreeSet<&str> = DigestDomain::ALL.iter().map(|d| d.as_uri()).collect();
        assert_eq!(
            uris.len(),
            DigestDomain::ALL.len(),
            "digest domain URIs must be pairwise distinct"
        );
    }

    #[test]
    fn domain_uris_are_namespaced_and_versioned() {
        for domain in DigestDomain::ALL {
            let uri = domain.as_uri();
            assert!(
                uri.starts_with("oh.war/"),
                "{uri} must use the oh.war family"
            );
            assert!(uri.ends_with("/v1"), "{uri} must carry an explicit version");
        }
    }

    /// Pinned against the published SHA-256 of the empty string and of "abc".
    /// A frozen external vector, not a value read back out of this
    /// implementation — an implementation-derived expectation would pass even
    /// if the hash were wrong.
    #[test]
    fn sha256_matches_published_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
