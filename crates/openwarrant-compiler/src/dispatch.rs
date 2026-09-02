// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compile a Stage Dispatch (SAS §47) — *"the only packet given to a stateless
//! actor."*
//!
//! # Pure, and deterministic for fixed ids
//!
//! Everything here is a function of its arguments. The caller supplies the
//! `dispatch_id` and the [`Attempt`] (whose id is a UUIDv7 minted by the
//! caller), so two calls with the same inputs produce the same bytes and the
//! same `dispatch_digest` — which is §47.2's "produce deterministic canonical
//! bytes" made checkable, and is the reason this is not done inside a
//! rendered view: a view must be reproducible from the committed sources
//! alone, and a dispatch is minted per attempt.
//!
//! # The duties, and where each is discharged
//!
//! §47.2 lists eight. Stage-relevant selection and provenance live in the
//! [`ContextManifest`] the caller builds; this function checks the result
//! rather than trusting it. Required normative sources are derived from the
//! Warrant's required atoms — never handed in — and a manifest that omits one
//! is refused by name. Prior failure evidence comes from the attempt. The two
//! digests §65 names for this seam, `ContextManifest` and `Dispatch`, are
//! computed here for the first time; `AttemptBasis` is the third.
//!
//! # The digest is computed over the packet with the digest field empty
//!
//! Then written in. Computing it over a packet that already contains it would
//! be a fixed point nobody could verify from the outside.

use openwarrant_core::context::ContextManifest;
use openwarrant_core::execution::{
    Attempt, AttemptKind, CapabilityAuthorization, DISPATCH_API_VERSION, ResourceEnvelope,
    SUBMISSION_SCHEMA_REF, StageDispatch,
};
use openwarrant_core::milestones::{Milestone, Stage};
use serde::Serialize;

use crate::canonical::{CanonicalError, sha256_digest, to_canonical_string};
use crate::digest::DigestDomain;
use crate::ir::WarIr;
use crate::lower::CompilationBasis;

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("{0}")]
    Canonical(#[from] CanonicalError),
    #[error("{0}")]
    Execution(#[from] openwarrant_core::execution::ExecutionError),
    #[error("{0}")]
    Context(#[from] openwarrant_core::context::ContextError),
    #[error(
        "stage {stage:?} is not referenced by milestone {milestone:?}; a dispatch binds a stage \
         to the milestone that cites it (§47.1 milestone_id)"
    )]
    StageNotInMilestone { stage: String, milestone: String },
    #[error(
        "stage {stage:?} declares no executor_ref, so nothing says what runs it. Refused rather \
         than dispatched under the WAR id — the same rule `war blut` applies"
    )]
    UnboundStage { stage: String },
    #[error(
        "required atom {atom:?} is not in the context manifest and is not recorded as omitted. \
         §33.6: a required context item is never silently dropped"
    )]
    RequiredAtomUnaccounted { atom: String },
    #[error(
        "the context manifest lists {atom:?} as both included and omitted. A packet that says an \
         item is present and absent says nothing; refused rather than resolved by whichever \
         list was read first"
    )]
    ContradictoryContext { atom: String },
    #[error(
        "required atom {atom:?} is recorded as omitted from the context manifest. §47.2: every \
         required normative source is preserved; an omission with a reason is still an omission"
    )]
    RequiredAtomOmitted { atom: String },
}

/// Everything a dispatch is compiled from.
pub struct DispatchInputs<'a> {
    pub ir: &'a WarIr,
    pub basis: &'a CompilationBasis,
    pub milestone: &'a Milestone,
    pub stage: &'a Stage,
    pub attempt: &'a Attempt,
    pub context: &'a ContextManifest,
    pub resources: ResourceEnvelope,
    pub capability: CapabilityAuthorization,
    /// Minted by the caller. Keeping it out of this function is what makes the
    /// output a pure function of its inputs.
    pub dispatch_id: String,
}

/// The sources §47.2 says must survive projection: every required atom.
#[must_use]
pub fn required_normative_sources(basis: &CompilationBasis) -> Vec<String> {
    basis
        .atoms
        .iter()
        .filter(|a| a.required)
        .map(|a| a.source.clone())
        .collect()
}

/// Compile one Stage Dispatch.
pub fn compile_dispatch(inputs: DispatchInputs<'_>) -> Result<StageDispatch, DispatchError> {
    let DispatchInputs {
        ir,
        basis,
        milestone,
        stage,
        attempt,
        context,
        resources,
        capability,
        dispatch_id,
    } = inputs;

    if !milestone.stage_refs.iter().any(|s| s == &stage.id) {
        return Err(DispatchError::StageNotInMilestone {
            stage: stage.id.clone(),
            milestone: milestone.id.clone(),
        });
    }
    if stage
        .executor_ref
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Err(DispatchError::UnboundStage {
            stage: stage.id.clone(),
        });
    }
    attempt.validate()?;
    context.validate()?;

    // §33.6 / §47.2 — every required atom is either in the manifest or
    // recorded as omitted with a reason. Neither is not an option.
    let required = required_normative_sources(basis);
    let omitted: Vec<String> = context.omitted.iter().map(|o| o.id.clone()).collect();
    // A manifest that lists one item on both sides is not resolved by whichever
    // list this loop happens to consult first; it is refused. From external
    // review: the earlier check short-circuited on `included` and would have
    // passed such a manifest through to a packet claiming both.
    for o in &omitted {
        if context.included.iter().any(|i| &i.id == o) {
            return Err(DispatchError::ContradictoryContext { atom: o.clone() });
        }
    }
    for source in &required {
        // Refused HERE and again in `validate`. Two layers on purpose: this one
        // names the rule at the point the manifest is read, and the second
        // holds if a caller ever bypasses the first.
        if omitted.contains(source) {
            return Err(DispatchError::RequiredAtomOmitted {
                atom: source.clone(),
            });
        }
        if !context.included.iter().any(|i| &i.id == source) {
            return Err(DispatchError::RequiredAtomUnaccounted {
                atom: source.clone(),
            });
        }
    }

    let contract_digest = ir.contract_digest()?;
    let context_manifest_digest = sha256_digest(DigestDomain::ContextManifest, context)?;

    #[derive(Serialize)]
    struct AttemptBasisView<'a> {
        contract_digest: &'a str,
        attempt_id: &'a str,
        attempt_kind: AttemptKind,
        parent_attempt_ref: &'a str,
        basis_change: &'a str,
        prior_failure_evidence_refs: &'a [String],
    }
    let attempt_basis_digest = sha256_digest(
        DigestDomain::AttemptBasis,
        &AttemptBasisView {
            contract_digest: &contract_digest,
            attempt_id: &attempt.id,
            attempt_kind: attempt.kind,
            parent_attempt_ref: &attempt.parent_attempt_ref,
            basis_change: &attempt.basis_change,
            prior_failure_evidence_refs: &attempt.prior_failure_evidence_refs,
        },
    )?;

    let mut dispatch = StageDispatch {
        api_version: DISPATCH_API_VERSION.to_owned(),
        dispatch_id,
        warrant_ref: format!("war://{}", ir.identity.uuid),
        contract_revision: ir.contract_revision,
        contract_digest,
        milestone_id: milestone.id.clone(),
        stage_id: stage.id.clone(),
        attempt_id: attempt.id.clone(),
        attempt_kind: Some(attempt.kind),
        attempt_basis_digest,
        objective: stage
            .title
            .clone()
            .or_else(|| milestone.title.clone())
            .unwrap_or_else(|| format!("{} of {}", stage.id, milestone.id)),
        non_goals: section_bullets(basis, "work_order", "## Frozen Surfaces"),
        instructions: section_bullets(basis, "work_order", "## Premade Instructions"),
        workspace_basis_ref: format!("basis://{}", basis.manifest_source),
        workspace_basis_digest: ir.integrity.workspace_basis_digest.clone(),
        context_manifest_ref: format!(
            "artifact://context-manifest/sha256:{context_manifest_digest}"
        ),
        context_manifest_digest,
        input_artifacts: stage.inputs.iter().map(|p| p.name.clone()).collect(),
        required_outputs: stage.outputs.iter().map(|p| p.name.clone()).collect(),
        obligation_refs: milestone.obligation_refs.clone(),
        capability_authorization: capability,
        resource_envelope: resources,
        submission_schema_ref: SUBMISSION_SCHEMA_REF.to_owned(),
        omitted_subgraphs: omitted,
        prior_failure_evidence_refs: attempt.prior_failure_evidence_refs.clone(),
        dispatch_digest: String::new(),
    };

    // §47.2 — the digest, over the packet with the digest field empty.
    dispatch.dispatch_digest = sha256_digest(DigestDomain::Dispatch, &dispatch)?;
    dispatch.validate(&required)?;
    Ok(dispatch)
}

/// The canonical bytes an actor receives (§47.2 "deterministic canonical bytes").
pub fn canonical_json(dispatch: &StageDispatch) -> Result<String, CanonicalError> {
    to_canonical_string(dispatch)
}

/// Bullets under one `## Heading` in an atom of the given role, as plain lines.
///
/// A small reader for the two work-order sections a dispatch lifts verbatim.
/// Not a Markdown parser; it stops at the next `## ` and keeps only `- ` and
/// `N. ` lines, which is the shape every work order in this corpus uses.
fn section_bullets(basis: &CompilationBasis, role: &str, heading: &str) -> Vec<String> {
    let mut out = Vec::new();
    for atom in basis.atoms.iter().filter(|a| a.role == role) {
        let Ok(text) = std::str::from_utf8(&atom.bytes) else {
            continue;
        };
        let mut inside = false;
        for line in text.lines() {
            if line.starts_with("## ") {
                inside = line.trim() == heading;
                continue;
            }
            if !inside {
                continue;
            }
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("- ") {
                out.push(rest.trim().to_owned());
            } else if t
                .split_once(". ")
                .is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            {
                out.push(
                    t.split_once(". ")
                        .map(|(_, r)| r.trim().to_owned())
                        .unwrap_or_default(),
                );
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::{AtomSource, lower};
    use openwarrant_core::context::{ContextItem, Holder, Omission, Precedence, TrustClass};
    use openwarrant_core::{Manifest, ValidatedManifest};

    const MILESTONES: &str = r#"schema: "oh.war/milestones/v1"

milestones:
  - id: "M1"
    title: "the one milestone"
    stage_refs: ["STAGE-001", "STAGE-002"]
    obligation_refs: ["OBL-001"]

stages:
  - id: "STAGE-001"
    title: "bound"
    executor_kind: "blut"
    responsibility_tier: "T1"
    executor_ref: "materialize"
    inputs: ["corpus:war/corpus"]
    outputs: ["filtered:war/corpus"]

  - id: "STAGE-002"
    title: "unbound"
    executor_kind: "human"
    responsibility_tier: "T1"
"#;

    const WORK_ORDER: &str = "# Work Order\n\n## Deliverables\n\n1. The thing.\n\n## Frozen Surfaces\n\n- the wire format\n\n## Premade Instructions\n\n- do not guess\n- say so when unsure\n";

    fn atom(ordinal: u32, role: &str, source: &str, bytes: &str, required: bool) -> AtomSource {
        AtomSource {
            ordinal,
            role: role.to_owned(),
            jurisdiction: "authored".to_owned(),
            source: source.to_owned(),
            bytes: bytes.as_bytes().to_vec(),
            required,
        }
    }

    fn fixture() -> (CompilationBasis, ValidatedManifest) {
        let manifest = Manifest {
            schema: openwarrant_core::MANIFEST_SCHEMA.to_owned(),
            uuid: "01a018db-19fc-7f2a-8e39-69730f255e33".to_owned(),
            local_alias: "OW-WAR-0001".to_owned(),
            enterprise_id: String::new(),
            title: "t".to_owned(),
            profile: "delivery".to_owned(),
            assurance_level: Some("basic".to_owned()),
            implements: vec![],
            roadmap: vec![],
            parents: vec![],
            supersedes: vec![],
            // The delivery profile requires these five; a manifest declaring
            // fewer does not validate, which is §16.4 doing its job.
            atoms: [
                (10, "intent", "atoms/10-intent.md"),
                (20, "basis", "atoms/20-basis.md"),
                (40, "work_order", "atoms/40-work-order.md"),
                (45, "milestones", "atoms/45-milestones.yaml"),
                (60, "assurance", "atoms/60-assurance.md"),
            ]
            .into_iter()
            .map(|(ordinal, role, path)| openwarrant_core::AtomEntry {
                ordinal,
                role: role.to_owned(),
                path: Some(path.to_owned()),
                r#ref: None,
                required: true,
            })
            .collect(),
            currency: None,
        };
        let validated = manifest
            .validate(Some("OW"))
            .expect("fixture manifest validates");
        let basis = CompilationBasis {
            manifest_source: "docs/warrants/OW-WAR-0001/manifest.toml".to_owned(),
            manifest_bytes: b"(manifest)".to_vec(),
            manifest,
            atoms: vec![
                atom(10, "intent", "atoms/10-intent.md", "# Intent\n\nx\n", true),
                atom(20, "basis", "atoms/20-basis.md", "# Basis\n\nx\n", true),
                atom(40, "work_order", "atoms/40-work-order.md", WORK_ORDER, true),
                atom(
                    45,
                    "milestones",
                    "atoms/45-milestones.yaml",
                    MILESTONES,
                    true,
                ),
                atom(
                    60,
                    "assurance",
                    "atoms/60-assurance.md",
                    "# Assurance\n\n## Acceptance Obligations\n\n### OBL-001 — it works\n- **scope:** x.\n- **evidence:** y.\n",
                    true,
                ),
                atom(
                    70,
                    "rationale",
                    "atoms/70-rationale.md",
                    "optional\n",
                    false,
                ),
            ],
            scope: None,
        };
        (basis, validated)
    }

    fn context_for(basis: &CompilationBasis) -> ContextManifest {
        ContextManifest {
            workspace_basis_ref: "basis://m".to_owned(),
            workspace_basis_digest: "sha256:w".to_owned(),
            included: basis
                .atoms
                .iter()
                .filter(|a| a.required)
                .map(|a| ContextItem {
                    id: a.source.clone(),
                    role: openwarrant_core::context::ContextRole::Normative,
                    required: true,
                    holder: Holder {
                        kind: "git".to_owned(),
                        repository: "r".to_owned(),
                        commit_sha: "0".repeat(40),
                        path: a.source.clone(),
                    },
                    content_digest: format!("sha256:{}", crate::sha256_hex(&a.bytes)),
                    selector_sections: vec![],
                    classification: "internal".to_owned(),
                    trust: TrustClass::AuthoritativeInternal,
                    taints: vec![],
                    precedence: Some(Precedence::AuthorizedWarContract),
                })
                .collect(),
            omitted: vec![Omission {
                id: "atoms/70-rationale.md".to_owned(),
                reason: "optional; not stage-relevant".to_owned(),
                required: false,
            }],
            unresolved: vec![],
            conflicts: vec![],
            effective_classification: "internal".to_owned(),
            policy_digest: String::new(),
            compiler_digest: "test".to_owned(),
        }
    }

    fn attempt(kind: AttemptKind) -> Attempt {
        Attempt {
            id: "A-fixed".to_owned(),
            kind,
            parent_attempt_ref: if kind.requires_parent() {
                "attempt://p".to_owned()
            } else {
                String::new()
            },
            basis_change: String::new(),
            prior_failure_evidence_refs: vec![],
            prior_work_product_ref: String::new(),
            authorized_by: String::new(),
        }
    }

    fn compile(
        basis: &CompilationBasis,
        validated: &ValidatedManifest,
        context: &ContextManifest,
        stage: &str,
        attempt: &Attempt,
    ) -> Result<StageDispatch, DispatchError> {
        let ir = lower(basis, validated).expect("lowers");
        let graph = openwarrant_core::milestones::parse(MILESTONES).expect("graph parses");
        let stage = graph.stages.iter().find(|s| s.id == stage).expect("stage");
        let milestone = &graph.milestones[0];
        compile_dispatch(DispatchInputs {
            ir: &ir,
            basis,
            milestone,
            stage,
            attempt,
            context,
            resources: ResourceEnvelope::default(),
            capability: CapabilityAuthorization::default(),
            dispatch_id: "D-fixed".to_owned(),
        })
    }

    /// OBL-002 — same inputs, same bytes, same digest; different context,
    /// different both.
    #[test]
    fn a_dispatch_is_byte_deterministic_for_fixed_ids() {
        let (basis, validated) = fixture();
        let ctx = context_for(&basis);
        let a = compile(
            &basis,
            &validated,
            &ctx,
            "STAGE-001",
            &attempt(AttemptKind::Initial),
        )
        .expect("compiles");
        let b = compile(
            &basis,
            &validated,
            &ctx,
            "STAGE-001",
            &attempt(AttemptKind::Initial),
        )
        .expect("compiles");
        assert_eq!(canonical_json(&a).unwrap(), canonical_json(&b).unwrap());
        assert_eq!(a.dispatch_digest, b.dispatch_digest);
        assert!(!a.dispatch_digest.is_empty());
        assert_eq!(a.api_version, DISPATCH_API_VERSION);
        assert_eq!(a.input_artifacts, vec!["corpus"]);
        assert_eq!(a.required_outputs, vec!["filtered"]);
        assert_eq!(a.instructions, vec!["do not guess", "say so when unsure"]);
        assert_eq!(a.non_goals, vec!["the wire format"]);
        assert_eq!(a.obligation_refs, vec!["OBL-001"]);
        assert_eq!(a.omitted_subgraphs, vec!["atoms/70-rationale.md"]);

        let mut ctx2 = ctx.clone();
        ctx2.effective_classification = "restricted".to_owned();
        let c = compile(
            &basis,
            &validated,
            &ctx2,
            "STAGE-001",
            &attempt(AttemptKind::Initial),
        )
        .expect("compiles");
        assert_ne!(a.context_manifest_digest, c.context_manifest_digest);
        assert_ne!(
            a.dispatch_digest, c.dispatch_digest,
            "the dispatch digest covers the context"
        );
    }

    /// The digest is over the packet with the digest field empty, so it can
    /// be recomputed by anyone holding the packet.
    #[test]
    fn the_dispatch_digest_is_recomputable_from_the_packet() {
        let (basis, validated) = fixture();
        let ctx = context_for(&basis);
        let d = compile(
            &basis,
            &validated,
            &ctx,
            "STAGE-001",
            &attempt(AttemptKind::Initial),
        )
        .expect("compiles");
        let mut blank = d.clone();
        blank.dispatch_digest.clear();
        let recomputed = sha256_digest(DigestDomain::Dispatch, &blank).expect("digests");
        assert_eq!(recomputed, d.dispatch_digest);
    }

    /// OBL-003 — a required atom missing from the manifest, and not recorded
    /// as omitted, is refused by name.
    #[test]
    fn a_required_atom_dropped_from_context_is_refused_by_name() {
        let (basis, validated) = fixture();
        let mut ctx = context_for(&basis);
        ctx.included.retain(|i| i.id != "atoms/40-work-order.md");
        let err = compile(
            &basis,
            &validated,
            &ctx,
            "STAGE-001",
            &attempt(AttemptKind::Initial),
        )
        .expect_err("must refuse");
        match err {
            DispatchError::RequiredAtomUnaccounted { atom } => {
                assert_eq!(atom, "atoms/40-work-order.md")
            }
            other => panic!("wrong refusal: {other}"),
        }
        // Recorded as omitted is refused by name too — by the explicit branch,
        // not only by `validate` downstream.
        let mut ctx = context_for(&basis);
        ctx.included.retain(|i| i.id != "atoms/40-work-order.md");
        ctx.omitted.push(Omission {
            id: "atoms/40-work-order.md".to_owned(),
            reason: "r".to_owned(),
            required: true,
        });
        match compile(
            &basis,
            &validated,
            &ctx,
            "STAGE-001",
            &attempt(AttemptKind::Initial),
        ) {
            Err(DispatchError::RequiredAtomOmitted { atom }) => {
                assert_eq!(atom, "atoms/40-work-order.md");
            }
            other => panic!("expected RequiredAtomOmitted, got {other:?}"),
        }
    }

    /// From external review: an item on both lists is a contradiction, and a
    /// check that short-circuits on `included` would let it through.
    #[test]
    fn an_item_both_included_and_omitted_is_refused() {
        let (basis, validated) = fixture();
        let mut ctx = context_for(&basis);
        ctx.omitted.push(Omission {
            id: "atoms/10-intent.md".to_owned(),
            reason: "r".to_owned(),
            required: false,
        });
        match compile(
            &basis,
            &validated,
            &ctx,
            "STAGE-001",
            &attempt(AttemptKind::Initial),
        ) {
            Err(DispatchError::ContradictoryContext { atom }) => {
                assert_eq!(atom, "atoms/10-intent.md");
            }
            other => panic!("expected ContradictoryContext, got {other:?}"),
        }
    }

    #[test]
    fn an_unbound_stage_and_a_repair_without_evidence_are_refused() {
        let (basis, validated) = fixture();
        let ctx = context_for(&basis);
        assert!(matches!(
            compile(
                &basis,
                &validated,
                &ctx,
                "STAGE-002",
                &attempt(AttemptKind::Initial)
            ),
            Err(DispatchError::UnboundStage { .. })
        ));
        let err = compile(
            &basis,
            &validated,
            &ctx,
            "STAGE-001",
            &attempt(AttemptKind::Repair),
        )
        .expect_err("a repair with no prior evidence must be refused");
        assert!(matches!(err, DispatchError::Execution(_)), "{err}");
    }
}
