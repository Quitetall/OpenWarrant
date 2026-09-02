// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lowering a Compilation Basis to the canonical IR (SAS §14, §63).

use openwarrant_core::{Manifest, ValidatedManifest};
use serde::Serialize;

use crate::canonical::{CanonicalError, sha256_digest};
use crate::digest::{DigestDomain, sha256_hex};
use crate::ir::{
    API_VERSION, FormatBasis, Identity, ImplementsEdge, Integrity, KIND, ParentEdge, Relations,
    SCHEMA_PACK_ID, SCHEMA_PACK_VERSION, SourceAndComposition, SourceAtom, SourceScope, WarIr,
};

/// One atom's exact source, as read (SAS §62.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomSource {
    pub ordinal: u32,
    pub role: String,
    pub jurisdiction: String,
    /// The source location as written in the manifest, not as resolved on this
    /// machine. An absolute path would make the digest host-dependent.
    pub source: String,
    pub bytes: Vec<u8>,
    pub required: bool,
}

/// One optional machine-readable scope sidecar, as read with the Warrant.
///
/// Scope is authored next to a Warrant rather than inferred from a branch or a
/// pull request. Its exact bytes join the Compilation Basis so changing what a
/// Warrant permits changes the compiled contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeSource {
    /// Repository-relative sidecar location.
    pub source: String,
    /// Exact sidecar bytes, read once with the rest of the Basis.
    pub bytes: Vec<u8>,
}

/// The SAS revision a compilation is pinned against (§14 "SAS revision").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SasPin {
    pub version: String,
    /// sha256 of the document bytes, lowercase hex, no prefix.
    pub sha256: String,
}

/// Everything needed to reproduce one compilation (SAS §14).
///
/// §14: a compilation "SHALL NOT silently mix independently changing inputs."
/// Holding the bytes here rather than re-reading them during lowering is what
/// makes that true — the Basis is captured once and lowering is a pure function
/// of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilationBasis {
    pub manifest: Manifest,
    pub manifest_source: String,
    pub manifest_bytes: Vec<u8>,
    pub atoms: Vec<AtomSource>,
    /// Optional machine-readable scope. Older Warrants remain valid without
    /// one; a Bonsai-backed gate requires it explicitly.
    pub scope: Option<ScopeSource>,
    /// §14 — which SAS this Warrant implements. `None` until a revision is
    /// recorded (OW-WAR-0058); once one is, every workspace basis digest
    /// moves, exactly once, which is the point.
    pub sas: Option<SasPin>,
}

/// Lower a validated Basis to the canonical IR.
///
/// Pure: no clock, no environment, no filesystem. Compiling the same Basis twice
/// yields byte-identical output, which is what makes the drift check meaningful
/// rather than noise (OW-WAR-0004 OBL-002).
pub fn lower(
    basis: &CompilationBasis,
    validated: &ValidatedManifest,
) -> Result<WarIr, CanonicalError> {
    let manifest_digest = sha256_hex(&basis.manifest_bytes);

    let atoms: Vec<SourceAtom> = basis
        .atoms
        .iter()
        .map(|atom| SourceAtom {
            ordinal: atom.ordinal,
            role: atom.role.clone(),
            jurisdiction: atom.jurisdiction.clone(),
            source: atom.source.clone(),
            atom_source_digest: sha256_hex(&atom.bytes),
            required: atom.required,
        })
        .collect();

    let source_and_composition = SourceAndComposition {
        manifest_source: basis.manifest_source.clone(),
        manifest_digest,
        atoms,
        scope: basis.scope.as_ref().map(|scope| SourceScope {
            source: scope.source.clone(),
            scope_source_digest: sha256_hex(&scope.bytes),
        }),
    };

    let relations = Relations {
        implements: basis
            .manifest
            .implements
            .iter()
            .map(|i| ImplementsEdge {
                r#ref: i.r#ref.clone(),
                contribution: i.contribution.clone(),
            })
            .collect(),
        roadmap: basis
            .manifest
            .roadmap
            .iter()
            .map(|r| r.r#ref.clone())
            .collect(),
        parents: basis
            .manifest
            .parents
            .iter()
            .map(|p| ParentEdge {
                r#ref: p.r#ref.clone(),
                // Validation already refused a parent without one; this default
                // is unreachable and exists only so lowering is total.
                contract_revision: p.contract_revision.unwrap_or_default(),
                contract_digest: p.contract_digest.clone(),
            })
            .collect(),
    };

    let identity = Identity {
        uuid: validated.uuid.to_string(),
        local_alias: validated.alias.to_string(),
        // Empty means "not yet allocated" and is recorded as absent, never as
        // an empty official identity (§12.4).
        enterprise_id: Some(basis.manifest.enterprise_id.clone())
            .filter(|id| !id.trim().is_empty()),
        title: basis.manifest.title.clone(),
        profile: validated.profile.to_string(),
        assurance_level: validated.assurance_level.to_string(),
    };

    let format_basis = FormatBasis {
        package_id: SCHEMA_PACK_ID.to_owned(),
        version: SCHEMA_PACK_VERSION.to_owned(),
        root_schema_id: KIND.to_owned(),
        profile_schema_id: validated.profile.to_string(),
        sas_revision: basis.sas.as_ref().map(|p| p.version.clone()),
        sas_digest: basis.sas.as_ref().map(|p| format!("sha256:{}", p.sha256)),
    };

    // The composition revision is the ordered atom set; the workspace basis is
    // that plus the manifest and the schema pack. Two domains, because they
    // answer different questions: "did the composition change?" and "could this
    // compilation be reproduced?"
    #[derive(Serialize)]
    struct CompositionView<'a> {
        atoms: &'a [SourceAtom],
        // Preserve every pre-scope composition digest. `None` must be absent,
        // not serialized as `null`, or merely adding this optional capability
        // rewrites every existing generated Warrant.
        #[serde(skip_serializing_if = "Option::is_none")]
        scope: &'a Option<SourceScope>,
    }
    let composition_revision_digest = sha256_digest(
        DigestDomain::CompositionRevision,
        &CompositionView {
            atoms: &source_and_composition.atoms,
            scope: &source_and_composition.scope,
        },
    )?;

    #[derive(Serialize)]
    struct BasisView<'a> {
        format_basis: &'a FormatBasis,
        source_and_composition: &'a SourceAndComposition,
    }
    let workspace_basis_digest = sha256_digest(
        DigestDomain::WorkspaceBasis,
        &BasisView {
            format_basis: &format_basis,
            source_and_composition: &source_and_composition,
        },
    )?;

    Ok(WarIr {
        api_version: API_VERSION.to_owned(),
        kind: KIND.to_owned(),
        format_basis,
        identity,
        source_and_composition,
        relations,
        contract_coverage: WarIr::current_coverage(),
        // §28.1: identity persists across revisions. Revision 1 until
        // authorization exists to create a second (OW-ADR-0004).
        contract_revision: 1,
        integrity: Integrity {
            algorithm: "sha256".to_owned(),
            workspace_basis_digest,
            composition_revision_digest,
        },
        execution: None,
        assurance_case: None,
        resolution: None,
        extensions: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwarrant_core::MANIFEST_SCHEMA;
    use openwarrant_core::manifest::{AtomEntry, Implements};

    fn basis() -> (CompilationBasis, ValidatedManifest) {
        let manifest = Manifest {
            schema: MANIFEST_SCHEMA.to_owned(),
            uuid: "01a018db-19fc-7f2a-8e39-69730f255e33".to_owned(),
            local_alias: "OW-WAR-0001".to_owned(),
            enterprise_id: String::new(),
            title: "A warrant".to_owned(),
            profile: "delivery".to_owned(),
            assurance_level: Some("basic".to_owned()),
            implements: vec![Implements {
                r#ref: "sas://WAR-SAS-RQ-070".to_owned(),
                contribution: Some("partial".to_owned()),
            }],
            roadmap: vec![],
            parents: vec![],
            supersedes: vec![],
            currency: None,
            atoms: ["intent", "basis", "work_order", "milestones", "assurance"]
                .iter()
                .enumerate()
                .map(|(i, role)| AtomEntry {
                    ordinal: (i as u32 + 1) * 10,
                    role: (*role).to_owned(),
                    path: Some(format!("atoms/{role}.md")),
                    r#ref: None,
                    required: true,
                })
                .collect(),
        };
        let validated = manifest.validate(Some("OW")).expect("valid");
        let atoms = manifest
            .atoms
            .iter()
            .map(|a| AtomSource {
                ordinal: a.ordinal,
                role: a.role.clone(),
                jurisdiction: "authored".to_owned(),
                source: a.path.clone().expect("path"),
                bytes: format!("# {}\n\nbody\n", a.role).into_bytes(),
                required: a.required,
            })
            .collect();
        (
            CompilationBasis {
                manifest_source: "docs/warrants/OW-WAR-0001/manifest.toml".to_owned(),
                manifest_bytes: b"(manifest bytes)".to_vec(),
                manifest,
                atoms,
                scope: None,
                sas: None,
            },
            validated,
        )
    }

    #[test]
    fn lowering_is_deterministic() {
        let (b, v) = basis();
        let a = lower(&b, &v).expect("lowers");
        let c = lower(&b, &v).expect("lowers");
        assert_eq!(a, c);
    }

    #[test]
    fn atom_digests_track_atom_bytes() {
        let (mut b, v) = basis();
        let before = lower(&b, &v).expect("lowers");

        b.atoms[0].bytes = b"# intent\n\nDIFFERENT body\n".to_vec();
        let after = lower(&b, &v).expect("lowers");

        assert_ne!(
            before.source_and_composition.atoms[0].atom_source_digest,
            after.source_and_composition.atoms[0].atom_source_digest,
            "an atom's digest must follow its bytes"
        );
        assert_ne!(
            before.contract_digest().expect("digest"),
            after.contract_digest().expect("digest"),
            "changing an atom changes what was authorized"
        );
    }

    #[test]
    fn scope_bytes_move_the_contract_digest() {
        let (mut basis, validated) = basis();
        basis.scope = Some(ScopeSource {
            source: "docs/warrants/OW-WAR-0001/scope.toml".to_owned(),
            bytes: b"schema = \"oh.war/bonsai-scope/v1\"\n".to_vec(),
        });
        let before = lower(&basis, &validated).expect("lowers");

        basis.scope.as_mut().expect("scope").bytes =
            b"schema = \"oh.war/bonsai-scope/v1\"\nrepository = \"github:example/repo\"\n".to_vec();
        let after = lower(&basis, &validated).expect("lowers");

        assert_ne!(
            before.contract_digest().expect("digest"),
            after.contract_digest().expect("digest"),
            "machine scope changes what the Warrant authorizes"
        );
    }

    #[test]
    fn an_unallocated_enterprise_id_is_absent() {
        let (b, v) = basis();
        assert_eq!(lower(&b, &v).expect("lowers").identity.enterprise_id, None);
    }

    #[test]
    fn relations_are_carried_through() {
        let (b, v) = basis();
        let ir = lower(&b, &v).expect("lowers");
        assert_eq!(ir.relations.implements.len(), 1);
        assert_eq!(ir.relations.implements[0].r#ref, "sas://WAR-SAS-RQ-070");
    }

    /// The two integrity digests answer different questions and must not be
    /// the same value, or one of them is redundant.
    #[test]
    fn workspace_basis_and_composition_digests_differ() {
        let (b, v) = basis();
        let ir = lower(&b, &v).expect("lowers");
        assert_ne!(
            ir.integrity.workspace_basis_digest,
            ir.integrity.composition_revision_digest
        );
    }

    /// Reordering atoms is a composition change, not a cosmetic one.
    #[test]
    fn atom_order_affects_the_composition_digest() {
        let (b, v) = basis();
        let before = lower(&b, &v).expect("lowers");

        let mut reordered = b.clone();
        reordered.atoms.reverse();
        let after = lower(&reordered, &v).expect("lowers");

        assert_ne!(
            before.integrity.composition_revision_digest,
            after.integrity.composition_revision_digest
        );
    }
}
