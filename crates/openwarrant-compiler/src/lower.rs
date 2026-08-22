// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lowering a Compilation Basis to the canonical IR (SAS §14, §63).

use openwarrant_core::{Manifest, ValidatedManifest};
use serde::Serialize;

use crate::canonical::{CanonicalError, sha256_digest};
use crate::digest::{DigestDomain, sha256_hex};
use crate::ir::{
    API_VERSION, FormatBasis, Identity, ImplementsEdge, Integrity, KIND, ParentEdge, Relations,
    SCHEMA_PACK_ID, SCHEMA_PACK_VERSION, SourceAndComposition, SourceAtom, WarIr,
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
    };

    // The composition revision is the ordered atom set; the workspace basis is
    // that plus the manifest and the schema pack. Two domains, because they
    // answer different questions: "did the composition change?" and "could this
    // compilation be reproduced?"
    #[derive(Serialize)]
    struct CompositionView<'a> {
        atoms: &'a [SourceAtom],
    }
    let composition_revision_digest = sha256_digest(
        DigestDomain::CompositionRevision,
        &CompositionView {
            atoms: &source_and_composition.atoms,
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
