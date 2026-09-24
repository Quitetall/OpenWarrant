// SPDX-License-Identifier: Apache-2.0
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
use openwarrant_core::tokens::{self, TokenAccount};
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
    /// RQ-046 / §47.2: the estimate exceeds the budget. The largest items are
    /// named, so the fix is a cut the author makes (§33.6), not one made here.
    #[error(
        "the selected context is ~{estimated_tokens} tokens against a budget of {budget_tokens} \
         ({method}); largest: {}",
        largest
            .iter()
            .map(|(id, t)| format!("{id} (~{t} tokens)"))
            .collect::<Vec<_>>()
            .join(", ")
    )]
    OverBudget {
        estimated_tokens: u64,
        budget_tokens: u64,
        method: String,
        /// The three largest items, largest first, each with its estimate.
        largest: Vec<(String, u64)>,
    },
    /// RQ-046: a Dispatch declares its estimate and budget. One compiled with
    /// no token account at all would carry neither, so it is not emitted.
    #[error(
        "no token account was supplied: a Dispatch declares its token estimate and budget \
         (RQ-046, §47.2), and none is emitted without them"
    )]
    TokensUnrecorded,
}

/// What the compiler needs to account a Dispatch's size (§33.7, §47.1): the
/// byte count of every selected context item, and the budget it must fit. The
/// estimate itself is computed here by [`openwarrant_core::tokens::METHOD`],
/// never handed in, so no caller can record a number the rule did not judge.
#[derive(Debug, Clone, Copy)]
pub struct TokenInputs<'a> {
    /// `(item id, bytes)` for each selected context item.
    pub item_bytes: &'a [(String, u64)],
    /// The stage's `budget_tokens`, or the repository's default.
    pub budget_tokens: u64,
}

/// Estimate `inputs` and judge it against its budget (RQ-046, §47.2).
///
/// # Errors
///
/// [`DispatchError::OverBudget`] when the estimate exceeds the budget, naming
/// the three largest items.
pub fn token_account(inputs: TokenInputs<'_>) -> Result<TokenAccount, DispatchError> {
    let TokenInputs {
        item_bytes,
        budget_tokens,
    } = inputs;
    let total: u64 = item_bytes
        .iter()
        .map(|(_, b)| *b)
        .fold(0u64, u64::saturating_add);
    let estimated_tokens = tokens::estimate(total);
    if estimated_tokens > budget_tokens {
        let mut sorted: Vec<&(String, u64)> = item_bytes.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        return Err(DispatchError::OverBudget {
            estimated_tokens,
            budget_tokens,
            method: tokens::METHOD.to_owned(),
            largest: sorted
                .into_iter()
                .take(3)
                .map(|(id, b)| (id.clone(), tokens::estimate(*b)))
                .collect(),
        });
    }
    Ok(TokenAccount {
        estimated_tokens,
        budget_tokens,
        method: tokens::METHOD.to_owned(),
    })
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
    /// §33.7, RQ-046: the item bytes and the budget. The compiler computes
    /// the estimate and refuses over budget; `None` is refused as
    /// [`DispatchError::TokensUnrecorded`], never emitted without `tokens`.
    pub tokens: Option<TokenInputs<'a>>,
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
        tokens,
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
    // RQ-046 / §47.2 — the compiler records the estimate and budget and
    // refuses over budget. Here, not in a caller, so every caller inherits it.
    let tokens = token_account(tokens.ok_or(DispatchError::TokensUnrecorded)?)?;
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
        // The manifest's own `required` flag on an omission is not trusted.
        // `ContextManifest::validate` refuses an omission that ADMITS it was
        // required; this refuses one that claims it was not, by checking
        // against the contract's required atoms rather than the manifest's
        // account of them.
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
        tokens: Some(tokens),
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
            } else if !t.is_empty()
                && line.starts_with([' ', '\t'])
                && let Some(last) = out.last_mut()
            {
                // A wrapped bullet: an indented line after an item continues
                // it. Dropping these lost every instruction longer than one
                // line (OW-WAR-0064's intent names it); joining them moves the
                // dispatch digest of every Warrant whose atoms wrap, which is
                // the correct consequence of the packet finally being whole.
                last.push(' ');
                last.push_str(t);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wrapped_bullet_is_one_instruction_and_an_unindented_line_is_not() {
        let text = "## Premade Instructions\n\n- first line\n  continues here\n  and here\n- second\n1. numbered\n   wrapped too\nnot a continuation\n\n## Other\n\n- elsewhere\n";
        let (mut basis, _) = fixture();
        basis.atoms = vec![atom(40, "work_order", "atoms/40-work-order.md", text, true)];
        let got = section_bullets(&basis, "work_order", "## Premade Instructions");
        assert_eq!(
            got,
            vec![
                "first line continues here and here".to_owned(),
                "second".to_owned(),
                "numbered wrapped too".to_owned(),
            ]
        );
    }
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
            sas: None,
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

    /// The byte count of each required atom, as the CLI's selector reports it.
    fn item_bytes(basis: &CompilationBasis) -> Vec<(String, u64)> {
        basis
            .atoms
            .iter()
            .filter(|a| a.required)
            .map(|a| (a.source.clone(), a.bytes.len() as u64))
            .collect()
    }

    fn compile(
        basis: &CompilationBasis,
        validated: &ValidatedManifest,
        context: &ContextManifest,
        stage: &str,
        attempt: &Attempt,
    ) -> Result<StageDispatch, DispatchError> {
        let items = item_bytes(basis);
        compile_with_tokens(
            basis,
            validated,
            context,
            stage,
            attempt,
            Some(TokenInputs {
                item_bytes: &items,
                budget_tokens: 1_000_000,
            }),
        )
    }

    fn compile_with_tokens(
        basis: &CompilationBasis,
        validated: &ValidatedManifest,
        context: &ContextManifest,
        stage: &str,
        attempt: &Attempt,
        tokens: Option<TokenInputs<'_>>,
    ) -> Result<StageDispatch, DispatchError> {
        let ir = lower(basis, validated).expect("lowers");
        let graph = openwarrant_core::milestones::parse(MILESTONES).expect("graph parses");
        let stage = graph.stages.iter().find(|s| s.id == stage).expect("stage");
        let milestone = &graph.milestones[0];
        compile_dispatch(DispatchInputs {
            tokens,
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
        // Recorded as omitted is refused by name too. The omission is marked
        // `required: false` — a manifest LYING about whether the atom was
        // required. `ContextManifest::validate` trusts the manifest's own flag
        // and would pass this; the compiler checks the flag against the
        // contract's required atoms and does not.
        let mut ctx = context_for(&basis);
        ctx.included.retain(|i| i.id != "atoms/40-work-order.md");
        ctx.omitted.push(Omission {
            id: "atoms/40-work-order.md".to_owned(),
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

    /// OW-WAR-0129 OBL-001 — the budget rule lives in the compiler. Exactly at
    /// budget is emitted with the compiler's own estimate; one token over is
    /// refused naming the largest items; no account at all is refused.
    #[test]
    fn the_compiler_refuses_over_budget_and_emits_at_budget() {
        let (basis, validated) = fixture();
        let ctx = context_for(&basis);
        let items = item_bytes(&basis);
        let total: u64 = items.iter().map(|(_, b)| *b).sum();
        let estimate = tokens::estimate(total);
        assert!(estimate > 1, "the fixture has something to estimate");
        let with_budget = |budget_tokens| {
            compile_with_tokens(
                &basis,
                &validated,
                &ctx,
                "STAGE-001",
                &attempt(AttemptKind::Initial),
                Some(TokenInputs {
                    item_bytes: &items,
                    budget_tokens,
                }),
            )
        };

        // In budget, and exactly at it: emitted, the account recorded as
        // computed here, and inside the digest.
        for budget in [estimate + 100, estimate] {
            let d = with_budget(budget).expect("in budget compiles");
            assert_eq!(
                d.tokens,
                Some(TokenAccount {
                    estimated_tokens: estimate,
                    budget_tokens: budget,
                    method: tokens::METHOD.to_owned(),
                })
            );
        }
        let low = with_budget(estimate + 100).expect("compiles");
        let high = with_budget(estimate + 101).expect("compiles");
        assert_ne!(
            low.dispatch_digest, high.dispatch_digest,
            "tokens are inside the dispatch digest (§47.1)"
        );

        // One token over: refused, with the numbers and the largest items.
        match with_budget(estimate - 1) {
            Err(DispatchError::OverBudget {
                estimated_tokens,
                budget_tokens,
                method,
                largest,
            }) => {
                assert_eq!(estimated_tokens, estimate);
                assert_eq!(budget_tokens, estimate - 1);
                assert_eq!(method, tokens::METHOD);
                assert_eq!(largest.len(), 3);
                // The milestones atom is the largest in the fixture.
                assert_eq!(largest[0].0, "atoms/45-milestones.yaml");
                assert!(largest.windows(2).all(|w| w[0].1 >= w[1].1));
                let err = with_budget(estimate - 1).expect_err("refused");
                assert!(
                    err.to_string()
                        .contains("largest: atoms/45-milestones.yaml")
                );
            }
            other => panic!("expected OverBudget, got {other:?}"),
        }

        // No token account: refused, never emitted without `tokens`.
        match compile_with_tokens(
            &basis,
            &validated,
            &ctx,
            "STAGE-001",
            &attempt(AttemptKind::Initial),
            None,
        ) {
            Err(DispatchError::TokensUnrecorded) => {}
            other => panic!("expected TokensUnrecorded, got {other:?}"),
        }
    }
}
