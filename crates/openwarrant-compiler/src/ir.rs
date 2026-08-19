// SPDX-License-Identifier: AGPL-3.0-or-later
//! The canonical WAR IR (SAS §63) and the Compilation Basis it is built from
//! (§14).
//!
//! §63: "The canonical IR is a typed semantic object, not a Markdown AST."

use serde::{Deserialize, Serialize};

use crate::canonical::{CanonicalError, sha256_digest};
use crate::digest::DigestDomain;

/// The API version every canonical document carries (§63).
pub const API_VERSION: &str = "oh.war/v1";
/// The record kind (§4.2).
pub const KIND: &str = "work_authorization_record";

/// The schema pack this build implements (§64).
pub const SCHEMA_PACK_ID: &str = "openwarrant-schema-pack";
pub const SCHEMA_PACK_VERSION: &str = "0.1.0";

/// Pins schema, vocabulary, profile, and state-machine versions (§64).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatBasis {
    pub package_id: String,
    pub version: String,
    pub root_schema_id: String,
    pub profile_schema_id: String,
}

/// UUID, aliases, enterprise ID, title, profile (§63.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub uuid: String,
    pub local_alias: String,
    /// Absent until Knowledge Fabric allocates one (§12.4). Serialised as absent
    /// rather than as `""`, because "no official identity" and "an official
    /// identity that is the empty string" are different claims.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enterprise_id: Option<String>,
    pub title: String,
    pub profile: String,
    pub assurance_level: String,
}

/// One atom as it entered the compilation (§63.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceAtom {
    pub ordinal: u32,
    pub role: String,
    pub jurisdiction: String,
    pub source: String,
    /// SHA-256 over the atom's exact source bytes (§62.2, §65 `atom_source_digest`).
    pub atom_source_digest: String,
    pub required: bool,
}

/// Workspace Basis and composition (§63.3, §14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceAndComposition {
    pub manifest_source: String,
    pub manifest_digest: String,
    pub atoms: Vec<SourceAtom>,
}

/// Typed relation edges (§63.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relations {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implements: Vec<ImplementsEdge>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roadmap: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parents: Vec<ParentEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImplementsEdge {
    pub r#ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contribution: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParentEdge {
    pub r#ref: String,
    pub contract_revision: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_digest: Option<String>,
}

/// Digests and checkpoints (§63.11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Integrity {
    pub algorithm: String,
    pub workspace_basis_digest: String,
    pub composition_revision_digest: String,
}

/// The canonical WAR IR (§63).
///
/// # Absent is not empty
///
/// Sections Phase 1 does not populate — `execution`, `assurance_case`,
/// `resolution` — are `Option` and are OMITTED from the canonical JSON, not
/// serialised as `{}`.
///
/// This distinction is load-bearing and is the single highest-value invariant in
/// the type. If absent and empty serialised identically, the first Warrant to
/// populate `execution` would change the canonical bytes — and therefore the
/// digest — of every Warrant compiled before it, retroactively invalidating
/// digests that were correct when minted. `absent_and_empty_differ` pins it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarIr {
    pub api_version: String,
    pub kind: String,
    pub format_basis: FormatBasis,
    pub identity: Identity,
    pub source_and_composition: SourceAndComposition,
    pub relations: Relations,
    pub integrity: Integrity,
    /// Which §28.5 elements the contract digest covers (OW-ADR-0004).
    pub contract_coverage: openwarrant_core::ContractCoverage,
    /// The contract revision this compilation represents (§28).
    pub contract_revision: u32,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assurance_case: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<serde_json::Value>,
    /// Unknown namespaced extensions survive a round trip (§69.4, §91.1 test 5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

impl WarIr {
    /// The contract digest — the identity of what was authorized (§65).
    ///
    /// Computed over identity, composition, and relations. It deliberately does
    /// NOT include the generated projections: §91.1 test 3 requires that
    /// changing how a document renders cannot change the contract digest,
    /// because rendering is downstream of meaning.
    pub fn contract_digest(&self) -> Result<String, CanonicalError> {
        #[derive(Serialize)]
        struct ContractView<'a> {
            /// OW-ADR-0004: the §28.5 elements this digest actually covers, IN
            /// the preimage. A digest over five elements and one over seventeen
            /// are therefore different values even for identical content, so a
            /// partial digest can never be mistaken for a complete one.
            coverage: &'a openwarrant_core::ContractCoverage,
            format_basis: &'a FormatBasis,
            identity: &'a Identity,
            source_and_composition: &'a SourceAndComposition,
            relations: &'a Relations,
        }
        sha256_digest(
            DigestDomain::Contract,
            &ContractView {
                coverage: &self.contract_coverage,
                format_basis: &self.format_basis,
                identity: &self.identity,
                source_and_composition: &self.source_and_composition,
                relations: &self.relations,
            },
        )
    }

    /// The §28.5 elements a compilation can cover today.
    ///
    /// Deliberately a function rather than a constant: each element joins as its
    /// Warrant lands (deliverables OW-WAR-0015, obligations OW-WAR-0016, gates
    /// OW-WAR-0019, capabilities OW-WAR-0023), and every addition MOVES the
    /// contract digest. That churn is the point — a Warrant authorized under a
    /// five-element contract was authorized over less than one authorized under
    /// seventeen, and the digest must say so.
    #[must_use]
    pub fn current_coverage() -> openwarrant_core::ContractCoverage {
        use openwarrant_core::ContractElement as E;
        openwarrant_core::ContractCoverage::new([
            E::Intent,
            E::Scope,
            E::BasisRequirements,
            E::Assumptions,
            E::Constraints,
            E::AdrReferences,
            E::Milestones,
            E::Stages,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::to_canonical_string;

    fn ir() -> WarIr {
        WarIr {
            api_version: API_VERSION.to_owned(),
            kind: KIND.to_owned(),
            format_basis: FormatBasis {
                package_id: SCHEMA_PACK_ID.to_owned(),
                version: SCHEMA_PACK_VERSION.to_owned(),
                root_schema_id: KIND.to_owned(),
                profile_schema_id: "delivery".to_owned(),
            },
            identity: Identity {
                uuid: "01a018db-19fc-7f2a-8e39-69730f255e33".to_owned(),
                local_alias: "OW-WAR-0001".to_owned(),
                enterprise_id: None,
                title: "A warrant".to_owned(),
                profile: "delivery".to_owned(),
                assurance_level: "basic".to_owned(),
            },
            source_and_composition: SourceAndComposition {
                manifest_source: "docs/warrants/OW-WAR-0001/manifest.toml".to_owned(),
                manifest_digest: "0".repeat(64),
                atoms: vec![],
            },
            relations: Relations {
                implements: vec![],
                roadmap: vec![],
                parents: vec![],
            },
            integrity: Integrity {
                algorithm: "sha256".to_owned(),
                workspace_basis_digest: "1".repeat(64),
                composition_revision_digest: "2".repeat(64),
            },
            contract_coverage: WarIr::current_coverage(),
            contract_revision: 1,
            execution: None,
            assurance_case: None,
            resolution: None,
            extensions: None,
        }
    }

    /// THE invariant. An absent section and an empty one must not serialise the
    /// same way, or populating a section later silently reprices every digest
    /// minted before it.
    #[test]
    fn absent_and_empty_differ() {
        let absent = ir();
        let mut empty = ir();
        empty.execution = Some(serde_json::json!({}));

        let a = to_canonical_string(&absent).expect("canonicalizes");
        let e = to_canonical_string(&empty).expect("canonicalizes");

        assert!(
            !a.contains("execution"),
            "an absent section must be omitted entirely, got: {a}"
        );
        assert!(
            e.contains("\"execution\":{}"),
            "an empty section must be present and empty, got: {e}"
        );
        assert_ne!(a, e, "absent and empty must never canonicalize alike");
        assert_ne!(
            absent.contract_digest().expect("digest"),
            // The contract view excludes execution, so this pair SHOULD match;
            // asserted below instead. Here we only compare the full documents.
            {
                let mut other = ir();
                other.identity.title = "A different warrant".to_owned();
                other.contract_digest().expect("digest")
            }
        );
    }

    /// §91.1 test 3: how a document renders cannot change what was authorized.
    #[test]
    fn generated_sections_do_not_move_the_contract_digest() {
        let bare = ir();
        let mut with_generated = ir();
        with_generated.execution = Some(serde_json::json!({"attempts": [1, 2, 3]}));
        with_generated.resolution = Some(serde_json::json!({"verdict": "passed"}));

        assert_eq!(
            bare.contract_digest().expect("digest"),
            with_generated.contract_digest().expect("digest"),
            "execution and resolution are downstream of the contract"
        );
    }

    /// Changing anything the contract IS must move its digest.
    #[test]
    fn contract_content_moves_the_contract_digest() {
        let base = ir();
        let baseline = base.contract_digest().expect("digest");

        let mut retitled = ir();
        retitled.identity.title = "Something else".to_owned();
        assert_ne!(baseline, retitled.contract_digest().expect("digest"));

        let mut reprofiled = ir();
        reprofiled.identity.profile = "decision".to_owned();
        assert_ne!(baseline, reprofiled.contract_digest().expect("digest"));

        let mut reparented = ir();
        reparented.relations.parents.push(ParentEdge {
            r#ref: "war://01a018db-19fc-7f34-92db-54b2dca5446d".to_owned(),
            contract_revision: 1,
            contract_digest: None,
        });
        assert_ne!(baseline, reparented.contract_digest().expect("digest"));
    }

    #[test]
    fn api_version_and_kind_are_pinned() {
        let canonical = to_canonical_string(&ir()).expect("canonicalizes");
        assert!(canonical.contains(r#""api_version":"oh.war/v1""#));
        assert!(canonical.contains(r#""kind":"work_authorization_record""#));
    }

    /// A document we can write must be a document we can read.
    ///
    /// This caught a real defect: `Relations` and several `Option` fields
    /// carried `skip_serializing_if` without `default`, so a minimal IR
    /// serialised fine and then failed to deserialise with "missing field
    /// implements". §68.3 requires export→import→export to preserve semantic
    /// identity, and that is impossible if the import half rejects our own
    /// output.
    ///
    /// Kept as a MINIMAL fixture on purpose — every optional field empty or
    /// absent — because that is the shape that exercises every
    /// `skip_serializing_if`. A populated fixture would have passed while the
    /// bug was live.
    #[test]
    fn a_minimal_ir_round_trips_through_its_own_canonical_form() {
        let minimal = ir();
        let json = to_canonical_string(&minimal).expect("canonicalizes");
        let back: WarIr = serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("our own canonical output must deserialize: {e}\n{json}"));
        assert_eq!(back, minimal);
        // And re-exporting must reproduce the same bytes (§68.3).
        assert_eq!(to_canonical_string(&back).expect("canonicalizes"), json);
    }

    /// §91.1 test 5: unknown namespaced extensions survive a round trip.
    #[test]
    fn extensions_survive_a_round_trip() {
        let mut with_ext = ir();
        with_ext.extensions = Some(serde_json::json!({"x.lab": {"protocol": "A"}}));
        let json = to_canonical_string(&with_ext).expect("canonicalizes");
        let back: WarIr = serde_json::from_str(&json).expect("round trips");
        assert_eq!(back.extensions, with_ext.extensions);
        assert_eq!(back, with_ext);
    }

    /// An absent enterprise ID is absent, never the empty string (§12.4).
    #[test]
    fn absent_enterprise_id_is_omitted_not_empty() {
        let canonical = to_canonical_string(&ir()).expect("canonicalizes");
        assert!(!canonical.contains("enterprise_id"));
    }

    #[test]
    fn canonical_output_is_stable_across_two_computations() {
        let first = to_canonical_string(&ir()).expect("canonicalizes");
        let second = to_canonical_string(&ir()).expect("canonicalizes");
        assert_eq!(first, second);
    }
}
