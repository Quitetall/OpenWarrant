// SPDX-License-Identifier: Apache-2.0
//! Receipt eligibility over a captured current Compilation Basis (OW-WAR-0149).
//!
//! This is not a receipt store or an assurance gate. The caller supplies one
//! selected recorded attempt per runtime stage and authenticates its provenance,
//! policy and provider adapter. Historical attempts stay in the caller's store;
//! submitting several candidates here is ambiguous, not a request to pick a winner.

use std::collections::BTreeMap;

use openwarrant_core::{ValidatedManifest, document::runtime::*, milestones::ExecutorKind};

use crate::{CompilationBasis, DigestDomain, lower, required_normative_sources, sha256_digest};

/// Caller-selected recorded attempt, captured receipt and native verifier.
pub struct RuntimeStageEvidence<'a> {
    pub expectation: RuntimeExpectation<'a>,
    pub raw_receipt: &'a [u8],
    pub verifier: Option<&'a dyn ReceiptVerifier>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageReceiptAssessment {
    pub stage_id: String,
    pub receipt: ReceiptAssessment,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeBasisAssessment {
    pub standing: ReceiptStanding,
    pub code: &'static str,
    pub detail: String,
    pub stages: Vec<StageReceiptAssessment>,
}

fn result(
    standing: ReceiptStanding,
    code: &'static str,
    detail: impl Into<String>,
) -> RuntimeBasisAssessment {
    RuntimeBasisAssessment {
        standing,
        code,
        detail: detail.into(),
        stages: vec![],
    }
}

/// Check every Katana/BLUT stage in the current required milestone sources.
///
/// The validated manifest must describe this captured basis. Source capture and
/// reference resolution are the caller's responsibility; this function performs
/// no I/O. It recomputes the current contract/workspace and canonical dispatch
/// digests, then delegates native seals and execution facts to the provider.
/// No runtime stages is a match only with a valid required milestone source.
pub fn assess_runtime_basis(
    basis: &CompilationBasis,
    validated: &ValidatedManifest,
    evidence: &[RuntimeStageEvidence<'_>],
) -> RuntimeBasisAssessment {
    use ReceiptStanding::{Matches, Refused, Unknown};
    if validated.raw != basis.manifest {
        return result(
            Refused,
            "runtime-basis.manifest-mismatch",
            "validated manifest differs from captured basis",
        );
    }
    // A missing required source must not shrink the runtime stage inventory.
    for entry in &basis.manifest.atoms {
        let matching: Vec<_> = basis
            .atoms
            .iter()
            .filter(|a| a.ordinal == entry.ordinal)
            .collect();
        if matching.len() != 1 {
            return result(
                Unknown,
                "runtime-basis.source-missing",
                format!(
                    "expected exactly one captured source for ordinal {}",
                    entry.ordinal
                ),
            );
        }
        let atom = matching[0];
        if atom.role != entry.role
            || atom.required != entry.required
            || Some(atom.source.as_str()) != entry.path.as_deref().or(entry.r#ref.as_deref())
        {
            return result(
                Refused,
                "runtime-basis.source-mismatch",
                "captured source differs from manifest declaration",
            );
        }
    }
    if basis.atoms.len() != basis.manifest.atoms.len() {
        return result(
            Refused,
            "runtime-basis.extra-source",
            "captured basis has undeclared sources",
        );
    }
    let milestone_atoms: Vec<_> = basis
        .atoms
        .iter()
        .filter(|a| a.role == "milestones")
        .collect();
    if !milestone_atoms.iter().any(|a| a.required) {
        return result(
            Unknown,
            "runtime-basis.milestones-missing",
            "no required milestone source establishes the stage inventory",
        );
    }
    let mut stages = BTreeMap::new();
    let mut milestones = BTreeMap::new();
    for atom in milestone_atoms {
        let graph = std::str::from_utf8(&atom.bytes)
            .ok()
            .and_then(|text| openwarrant_core::milestones::parse(text).ok());
        let Some(graph) = graph else {
            return result(
                Refused,
                "runtime-basis.invalid-milestones",
                format!("invalid milestone source {}", atom.source),
            );
        };
        for milestone in graph.milestones {
            if milestones.insert(milestone.id.clone(), milestone).is_some() {
                return result(
                    Refused,
                    "runtime-basis.duplicate-milestone",
                    "milestone identity repeated across sources",
                );
            }
        }
        for stage in graph.stages {
            if stages.insert(stage.id.clone(), stage).is_some() {
                return result(
                    Refused,
                    "runtime-basis.duplicate-stage",
                    "stage identity repeated across sources",
                );
            }
        }
    }
    let runtime_stages: BTreeMap<_, _> = stages
        .iter()
        .filter(|(_, s)| matches!(s.executor_kind, ExecutorKind::Katana | ExecutorKind::Blut))
        .collect();
    let mut selected = BTreeMap::new();
    for record in evidence {
        let id = &record.expectation.dispatch.stage_id;
        if !runtime_stages.contains_key(id) {
            return result(
                Refused,
                "runtime-basis.unexpected-stage",
                format!("receipt supplied for non-runtime or absent stage {id}"),
            );
        }
        if selected.insert(id, record).is_some() {
            return result(
                Refused,
                "runtime-basis.ambiguous-attempt",
                format!("more than one selected attempt for {id}"),
            );
        }
    }
    let ir = match lower(basis, validated) {
        Ok(ir) => ir,
        Err(error) => return result(Refused, "runtime-basis.invalid-contract", error.to_string()),
    };
    let contract = match ir.contract_digest() {
        Ok(digest) => digest,
        Err(error) => return result(Refused, "runtime-basis.invalid-contract", error.to_string()),
    };
    let warrant_ref = format!("war://{}", ir.identity.uuid);
    let required = required_normative_sources(basis);
    let mut assessment = result(
        Matches,
        "runtime-basis.matches",
        "every current runtime stage has a matching receipt",
    );
    for (id, stage) in runtime_stages {
        let receipt = if let Some(record) = selected.get(id) {
            let expected = &record.expectation;
            let dispatch = expected.dispatch;
            let kind = match stage.executor_kind {
                ExecutorKind::Katana => ProviderKind::Katana,
                _ => ProviderKind::Blut,
            };
            let milestone_matches = milestones
                .get(&dispatch.milestone_id)
                .is_some_and(|m| m.stage_refs.contains(id));
            let mut unsigned = dispatch.clone();
            unsigned.dispatch_digest.clear();
            let digest_matches = sha256_digest(DigestDomain::Dispatch, &unsigned)
                .is_ok_and(|d| d == dispatch.dispatch_digest);
            if dispatch.warrant_ref != warrant_ref
                || dispatch.contract_digest != contract
                || dispatch.workspace_basis_digest != ir.integrity.workspace_basis_digest
                || dispatch.validate(&required).is_err()
                || !milestone_matches
                || !digest_matches
                || expected.provider.kind != kind
                || stage
                    .executor_ref
                    .as_deref()
                    .is_none_or(|r| r.trim().is_empty())
            {
                stage_result(
                    Refused,
                    "runtime-basis.dispatch-mismatch",
                    "dispatch does not match the current basis, milestone or provider kind",
                )
            } else {
                assess_runtime_receipt(expected, record.raw_receipt, record.verifier)
            }
        } else {
            stage_result(
                Unknown,
                "runtime-basis.receipt-missing",
                "no selected recorded receipt for this runtime stage",
            )
        };
        if receipt.standing == Refused
            || (receipt.standing == Unknown && assessment.standing == Matches)
        {
            assessment.standing = receipt.standing;
        }
        assessment.stages.push(StageReceiptAssessment {
            stage_id: id.clone(),
            receipt,
        });
    }
    if assessment.standing != Matches {
        assessment.code = "runtime-basis.incomplete";
        assessment.detail =
            "current runtime stages have refused or unknown evidence; see per-stage observations"
                .into();
    }
    assessment
}

fn stage_result(standing: ReceiptStanding, code: &'static str, detail: &str) -> ReceiptAssessment {
    ReceiptAssessment {
        standing,
        code,
        detail: detail.into(),
        raw_digest: None,
        provider_receipt_digest: None,
        cost_observation: Observation::Unknown,
    }
}
