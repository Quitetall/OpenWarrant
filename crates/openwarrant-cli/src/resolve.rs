// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war resolve --dry-run` — evaluate §56.1's thirteen requirements.
//!
//! # Why this is a dry run and nothing else, for now
//!
//! §56.1 lists thirteen things resolution SHALL verify. `ResolutionChecks`
//! implemented all thirteen during alpha as named booleans, each tested to block
//! on its own, and was referenced by no binary. So the thirteen were a struct
//! nothing computed.
//!
//! This computes them from the corpus as it actually is, and reports which
//! block. It deliberately does NOT record a resolution: §56.2's record needs an
//! authorizer, an acting role and a stated meaning, and inventing those to make
//! the command feel complete is the substitution §40.7 forbids.
//!
//! # Where requirement 10 is read from, and why it moved
//!
//! Independence used to be answered from `openwarrant.toml`'s single global
//! `[independence]` block. That was unsound in the dangerous direction: one
//! block set true would make EVERY Warrant claim independence, including ones
//! nothing independent had ever looked at.
//!
//! It is now answered from the verification records attached to the Warrant
//! (`verifications/*.toml`, see [`openwarrant_core::verification`]). Each
//! verification carries the independence its verifier ACTUALLY had, and
//! `admissible_for` refuses self-verification, evidence-free verdicts, and
//! independence below §46.3's minimum for the level. An obligation with no
//! admissible verification does not count as dispositioned.
//!
//! # What is computed
//!
//! All thirteen, from records. None is a constant any more:
//!
//! - deliverables and artifact digests (§37, digests recomputed from the bytes);
//! - obligation dispositions and independence (§46, from verification records);
//! - gate results (§44.5, from persisted Gate Runs);
//! - the authorized contract revision (§28.4), judgments (§42), residual-risk
//!   authority (§36.2 with §27.2) and the resolver's role (§27), through
//!   [`Authority`];
//! - blocking unknowns (§36.3) and runtime receipts (§48.4, §49.3).
//!
//! Two answer `true` structurally: no blocker remains, and deviations are
//! dispositioned. Nothing else defaults to true — an unanswerable requirement is
//! unmet, which is §32's fail-closed rule applied to §56.
//!
//! # Computed is not the same as satisfied
//!
//! Wiring the last of these up closed no Warrant, and was not meant to. Four
//! requirements read records only a human may write: a role assignment in
//! `docs/authority/roles.toml`, then a signed authorization carrying judgments
//! and risk acceptances. §27.2 forbids an agent authorizing a proposed WAR,
//! accepting residual risk, or resolving a delivery, so no implementation here
//! can make those true.
//!
//! What implementation CAN do is make them answerable — turning "a constant says
//! no" into "this specific record is absent", which is a state somebody can act
//! on. That is the whole of the difference.
//!
//! # §38.6 is reported separately, and that separation is load-bearing
//!
//! Requirement 4 asks whether every obligation is *dispositioned*.
//! `not_established` is a disposition, so a Warrant can meet all thirteen while
//! every obligation on record says the claim was not established. See
//! [`would_resolve_satisfied`].
//!
//! # Two citations this file used to get wrong
//!
//! Residual-risk authority is §36.2 with §27.2, not §58 (which is
//! Representations). And the ten §39.3 adequacy warnings `war check` reports are
//! a gap in the REVIEW, not §36.3 blocking unknowns; see
//! [`Authority::no_required_unknown_remains`] for why they are not folded in.
//! Neither error changed a verdict — both were `false` regardless — but each
//! would have sent a reader to the wrong page.

use openwarrant_compiler::{DigestDomain, sha256_digest};
use openwarrant_core::authority::{AuthorityRegister, PolicyResolutionContext};
use openwarrant_core::deliverable::Deliverable;
use openwarrant_core::epistemic::Judgment;
use openwarrant_core::rationale::Assumption;
use openwarrant_core::resolution::{RESOLUTION_REQUIREMENTS, ResolutionChecks};
use openwarrant_core::verification::Verification;
use openwarrant_core::{GateBinding, GateDefinition, GateReceipt, GateRun};
use serde::Serialize;
#[cfg(test)]
use std::collections::BTreeSet;

use crate::authorize::AuthorizationRecord;
use crate::diagnostic::{Diagnostic, Report};
use crate::gate_evidence::AdmissibleGateRun;
use crate::repo::{RepoError, Repository};

/// Compute §56.1's thirteen from the corpus as it stands.
///
/// Each is answered from a record or answered `false`. Nothing is assumed true
/// because it is probably fine — an unanswerable requirement is unmet, which is
/// the same fail-closed rule §32 applies to Preflight.
fn evaluate(
    one: &crate::repo::Loaded,
    verifications: &[Verification],
    gate_runs: &[AdmissibleGateRun],
    authority: &Authority<'_>,
    artifacts: ArtifactChecks,
) -> ResolutionChecks {
    let validated = one.validated.as_ref();
    let basis = one.basis.as_ref();

    let assurance = validated
        .map(|v| v.assurance_level.to_string())
        .unwrap_or_else(|| "basic".to_owned());

    // The obligations this Warrant declares, by id.
    let declared: Vec<String> = basis
        .map(|b| {
            b.atoms
                .iter()
                .filter(|a| a.role == "assurance")
                .filter_map(|a| {
                    openwarrant_core::obligation::parse(&String::from_utf8_lossy(&a.bytes)).ok()
                })
                .flat_map(|set| set.obligations.into_iter().map(|o| o.id))
                .collect()
        })
        .unwrap_or_default();

    // §46.3 is now answered from the verifications that actually happened, not
    // from a project-wide declaration.
    //
    // The global `[independence]` block could only ever be an aspiration: one
    // block set true made EVERY Warrant claim independence, including ones
    // nothing independent had looked at. Independence is a property of a
    // verifier examining a claim, so it is read from the verification record.
    let admissible: Vec<&Verification> = verifications
        .iter()
        .filter(|v| v.admissible_for(&assurance).is_ok())
        .collect();

    // Every declared obligation must carry an ADMISSIBLE verification. A
    // verification that is present but inadmissible (self-verified, evidence-free,
    // or insufficiently independent) counts for nothing, which is the point.
    let obligations_dispositioned = !declared.is_empty()
        && declared
            .iter()
            .all(|id| admissible.iter().any(|v| &v.obligation == id));

    // Independence holds when something was actually verified independently.
    // With no verifications at all this is false, which is correct: nothing
    // independent happened.
    let independence_met = !admissible.is_empty()
        && declared
            .iter()
            .all(|id| admissible.iter().any(|v| &v.obligation == id));

    // Gate citations across this Warrant's assurance atoms (§43.5).
    //
    // Read from declared `- **gate:**` bullets, never by scanning prose for
    // `gate://` — a gate identified by pattern-matching prose is the
    // "string, not a gate" failure §43 exists to end.
    let cited: Vec<String> = basis
        .map(|b| {
            b.atoms
                .iter()
                .filter(|a| a.role == "assurance")
                .flat_map(|a| {
                    openwarrant_core::gate::cited_gate_uris(&String::from_utf8_lossy(&a.bytes))
                })
                .collect()
        })
        .unwrap_or_default();
    let gates_ok = every_required_gate_has_admissible_result(&cited, gate_runs);

    ResolutionChecks {
        exact_authorized_contract_revision: authority.contract_is_authorized(),
        required_deliverables_exist: artifacts.required_deliverables_exist,
        artifact_digests_verify: artifacts.artifact_digests_verify,
        every_required_obligation_dispositioned: obligations_dispositioned,
        every_required_gate_has_admissible_result: gates_ok,
        no_required_unknown_remains: authority.no_required_unknown_remains(),
        // §53.1: "an unmet condition preventing valid progress". A declared
        // §36.3 blocking unknown is exactly that, so requirement 7 is answered
        // from the same record as requirement 6 and deliberately computes the
        // same thing for now.
        //
        // The duplication is the honest option. §53.1 has its own shape —
        // condition_ref, owner_ref, required_to_unblock — raised during
        // execution, and nothing in this repository can record one. Leaving this
        // as a bare `true` while OW-WAR-0026 and OW-WAR-0040 declare themselves
        // unstartable made requirement 7 a false assertion, and a Warrant scored
        // HIGHER for declaring itself blocked than for declaring a weighable
        // risk. A correct duplicate beats a wrong constant.
        no_blocker_remains: authority.no_required_unknown_remains(),
        deviations_dispositioned: true,
        required_judgments_exist: authority.required_judgments_exist(),
        independence_requirements_met: independence_met,
        residual_risks_have_sufficient_authority: authority.residual_risks_are_covered(),
        runtime_receipts_match_the_basis: runtime_receipts_match_the_basis(basis),
        resolver_holds_the_role: authority.a_resolver_is_eligible(&assurance, &declared),
    }
}

/// Resolver identity recorded by a persisted §56.2 Resolution.
///
/// Dry-run resolution asks whether *some* actor is eligible. Verification of an
/// existing Resolution is stricter: the actor named by that record must have
/// held Resolver authority when the Resolution became effective.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RecordedResolver<'a> {
    pub actor: &'a str,
    pub effective_at: &'a str,
}

/// Artifact facts computed by the caller from one bounded evidence source.
///
/// The ordinary dry-run caller derives these from the worktree. A persisted
/// Resolution caller derives them from a pinned Git tree, so the shared §56.1
/// evaluator never reopens an attacker-controlled path after verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct ArtifactChecks {
    pub required_deliverables_exist: bool,
    pub artifact_digests_verify: bool,
}

/// Exact observed bytes for one deliverable in the artifact-manifest snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ArtifactSnapshotEntry {
    pub deliverable_id: String,
    pub target_ref: String,
    pub observed_sha256: String,
}

/// Exact Git provenance for one source consumed by the assurance snapshot.
///
/// The commit identity prevents an author-controlled wall-clock timestamp from
/// being the sole chronology proof. The caller additionally proves this commit
/// is a strict ancestor of the commit that first contains the Resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct RecordedEvidenceCommit {
    pub source_ref: String,
    pub commit: String,
    pub committed_unix_seconds: i64,
}

/// One validated Gate Definition loaded from the exact pinned repository blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct RecordedGateDefinition {
    pub source_ref: String,
    pub observed_sha256: String,
    pub definition: GateDefinition,
}

/// One raw stream emitted by a Gate Run and bound by its receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct RecordedGateOutput {
    pub run_id: String,
    pub stream: &'static str,
    pub source_ref: String,
    pub observed_sha256: String,
}

/// Repository-relative sources consumed to recompute a persisted Resolution.
///
/// Paths are part of the assurance snapshot because two records with identical
/// bodies but different provenance are not the same evidence object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RecordedEvidenceSources<'a> {
    pub verification_refs: &'a [String],
    pub deliverables_ref: &'a str,
    pub gate_run_source_refs: &'a [String],
    pub gate_binding_source_refs: &'a [String],
    pub gate_selection_source_refs: &'a [String],
    pub gate_fixture_source_refs: &'a [String],
    pub gate_raw_evidence_source_refs: &'a [String],
    pub gate_receipt_source_refs: &'a [String],
    pub evidence_commits: &'a [RecordedEvidenceCommit],
    pub judgments_ref: Option<&'a str>,
    pub rationale_ref: Option<&'a str>,
}

/// Already-opened, bounded inputs for verifying a persisted Resolution.
///
/// This is intentionally data-only. The ADR migration verifier owns the pinned
/// Git snapshot and supplies objects parsed from that snapshot; this module
/// owns the common §56.1 semantics. Keeping those responsibilities separate
/// avoids both live-worktree TOCTOU and a second copy of the thirteen checks.
pub(crate) struct RecordedResolutionInputs<'a> {
    pub one: &'a crate::repo::Loaded,
    pub verifications: &'a [Verification],
    pub deliverables: &'a [Deliverable],
    pub gate_runs: &'a [GateRun],
    pub admissible_gate_runs: &'a [AdmissibleGateRun],
    pub gate_definitions: &'a [RecordedGateDefinition],
    pub gate_bindings: &'a [GateBinding],
    pub gate_receipts: &'a [GateReceipt],
    pub gate_outputs: &'a [RecordedGateOutput],
    pub authorization: &'a AuthorizationRecord,
    pub current_contract_digest: &'a str,
    pub judgments: &'a [Judgment],
    pub assumptions: Option<&'a [Assumption]>,
    pub register: &'a AuthorityRegister,
    pub policy_allows_automated_resolution: bool,
    pub performer: &'a str,
    pub resolver: RecordedResolver<'a>,
    pub artifacts: ArtifactChecks,
    pub artifact_entries: &'a [ArtifactSnapshotEntry],
    pub sources: RecordedEvidenceSources<'a>,
}

/// Independently recomputed facts needed to accept a persisted Resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordedResolutionEvaluation {
    pub checks: ResolutionChecks,
    /// §38.6 outcome recomputed from admissible verification records. `None`
    /// means at least one required obligation remains unknown.
    pub outcome: Option<bool>,
    pub assurance_case_snapshot_digest: String,
    pub artifact_manifest_digest: String,
    pub gate_run_refs: Vec<String>,
    pub judgment_refs: Vec<String>,
    pub residual_risk_refs: Vec<String>,
}

#[derive(Serialize)]
struct AssuranceCaseSnapshot<'a> {
    contract_digest: &'a str,
    authorization: &'a AuthorizationRecord,
    verification_refs: &'a [String],
    verifications: &'a [Verification],
    gate_run_source_refs: &'a [String],
    gate_runs: &'a [GateRun],
    gate_definitions: &'a [RecordedGateDefinition],
    gate_binding_source_refs: &'a [String],
    gate_bindings: &'a [GateBinding],
    gate_selection_source_refs: &'a [String],
    gate_fixture_source_refs: &'a [String],
    gate_raw_evidence_source_refs: &'a [String],
    gate_receipt_source_refs: &'a [String],
    gate_receipts: &'a [GateReceipt],
    gate_outputs: &'a [RecordedGateOutput],
    evidence_commits: &'a [RecordedEvidenceCommit],
    judgments_ref: Option<&'a str>,
    judgments: &'a [Judgment],
    rationale_ref: Option<&'a str>,
    assumptions: Option<&'a [Assumption]>,
    checks: ResolutionChecks,
    outcome: Option<bool>,
}

#[derive(Serialize)]
struct ArtifactManifestSnapshot<'a> {
    deliverables_ref: &'a str,
    deliverables: &'a [Deliverable],
    artifacts: &'a [ArtifactSnapshotEntry],
}

fn record_ids<T>(records: &[T], id: impl Fn(&T) -> &str) -> Result<Vec<String>, RepoError> {
    let mut refs: Vec<String> = records.iter().map(|record| id(record).to_owned()).collect();
    if refs.iter().any(|reference| reference.trim().is_empty()) {
        return Err(RepoError::Message(
            "Resolution evidence contains a blank record identifier".to_owned(),
        ));
    }
    refs.sort();
    if refs.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(RepoError::Message(format!(
            "Resolution evidence contains duplicate record identifier {:?}",
            refs.windows(2)
                .find(|pair| pair[0] == pair[1])
                .map(|pair| pair[0].as_str())
                .unwrap_or_default()
        )));
    }
    Ok(refs)
}

fn evidence_exists(
    reference: &str,
    gate_runs: &[AdmissibleGateRun],
    artifacts: &[ArtifactSnapshotEntry],
) -> bool {
    if let Some(run_id) = reference.strip_prefix("gate-run://") {
        return !run_id.is_empty() && gate_runs.iter().any(|run| run.id() == run_id);
    }
    if let Some(deliverable_id) = reference.strip_prefix("artifact://") {
        return !deliverable_id.is_empty()
            && artifacts
                .iter()
                .any(|artifact| artifact.deliverable_id == deliverable_id);
    }
    reference.starts_with("sha256:")
        && artifacts
            .iter()
            .any(|artifact| artifact.observed_sha256 == reference)
}

/// Recompute §56.1, §38.6, and the exact evidence bindings for a persisted Resolution.
///
/// This deliberately accepts no serialized `ResolutionChecks`. A record cannot
/// make itself valid by carrying thirteen `true` values; each value is derived
/// from bounded snapshot objects through the same evaluator used by
/// `war resolve --dry-run`. The returned digests and reference sets are the
/// only bindings a persisted Resolution may claim.
pub(crate) fn evaluate_recorded_resolution(
    inputs: RecordedResolutionInputs<'_>,
) -> Result<RecordedResolutionEvaluation, RepoError> {
    if inputs.one.basis.is_none() || inputs.one.validated.is_none() || !inputs.one.report.is_ready()
    {
        return Err(RepoError::Message(format!(
            "{} does not compile cleanly, so its Resolution evidence cannot be evaluated",
            inputs.one.alias()
        )));
    }
    if let Some(verification) = inputs
        .verifications
        .iter()
        .find(|verification| verification.performer != inputs.performer)
    {
        return Err(RepoError::Message(format!(
            "verification of {} names performer {:?}, expected exact performer {:?}",
            verification.obligation, verification.performer, inputs.performer
        )));
    }

    let declared = declared_obligations(inputs.one);
    if let Some(verification) = inputs
        .verifications
        .iter()
        .find(|verification| !declared.contains(&verification.obligation))
    {
        return Err(RepoError::Message(format!(
            "verification names undeclared obligation {:?}",
            verification.obligation
        )));
    }
    if let Some(verification) = inputs.verifications.iter().find(|verification| {
        !evidence_exists(
            &verification.evidence,
            inputs.admissible_gate_runs,
            inputs.artifact_entries,
        )
    }) {
        return Err(RepoError::Message(format!(
            "verification of {} cites unresolved or unsupported evidence {:?}",
            verification.obligation, verification.evidence
        )));
    }

    let assurance = inputs
        .one
        .validated
        .as_ref()
        .map(|validated| validated.assurance_level.to_string())
        .unwrap_or_else(|| "basic".to_owned());
    for obligation in &declared {
        let count = inputs
            .verifications
            .iter()
            .filter(|verification| {
                &verification.obligation == obligation
                    && verification.admissible_for(&assurance).is_ok()
            })
            .count();
        if count != 1 {
            return Err(RepoError::Message(format!(
                "obligation {obligation:?} has {count} admissible verification records; exactly one is required for a persisted Resolution"
            )));
        }
    }

    let authority = Authority {
        register: inputs.register,
        authorization: Some(inputs.authorization),
        current_contract_digest: Some(inputs.current_contract_digest),
        judgments: inputs.judgments,
        assumptions: inputs.assumptions,
        policy_allows_automated_resolution: inputs.policy_allows_automated_resolution,
        performer: inputs.performer,
        recorded_resolver: Some(inputs.resolver),
    };
    let checks = evaluate(
        inputs.one,
        inputs.verifications,
        inputs.admissible_gate_runs,
        &authority,
        inputs.artifacts,
    );

    let admissible: Vec<&Verification> = inputs
        .verifications
        .iter()
        .filter(|verification| verification.admissible_for(&assurance).is_ok())
        .collect();
    let outcome = would_resolve_satisfied(&declared, &admissible);

    let gate_run_refs = record_ids(inputs.gate_runs, |run| run.id.as_str())?;
    let admitted_gate_run_refs = record_ids(inputs.admissible_gate_runs, |run| run.id())?;
    if gate_run_refs != admitted_gate_run_refs {
        return Err(RepoError::Message(
            "Resolution Gate Run snapshot differs from receipt-admitted Gate Runs".to_owned(),
        ));
    }
    let judgment_refs = record_ids(inputs.judgments, |judgment| judgment.id.as_str())?;
    let residual_risk_refs = record_ids(
        &inputs
            .assumptions
            .unwrap_or_default()
            .iter()
            .filter(|assumption| {
                assumption.epistemic_status
                    == openwarrant_core::rationale::EpistemicStatus::AcceptedResidualRisk
            })
            .collect::<Vec<_>>(),
        |assumption| assumption.id.as_str(),
    )?;

    let assurance_snapshot = AssuranceCaseSnapshot {
        contract_digest: inputs.current_contract_digest,
        authorization: inputs.authorization,
        verification_refs: inputs.sources.verification_refs,
        verifications: inputs.verifications,
        gate_run_source_refs: inputs.sources.gate_run_source_refs,
        gate_runs: inputs.gate_runs,
        gate_definitions: inputs.gate_definitions,
        gate_binding_source_refs: inputs.sources.gate_binding_source_refs,
        gate_bindings: inputs.gate_bindings,
        gate_selection_source_refs: inputs.sources.gate_selection_source_refs,
        gate_fixture_source_refs: inputs.sources.gate_fixture_source_refs,
        gate_raw_evidence_source_refs: inputs.sources.gate_raw_evidence_source_refs,
        gate_receipt_source_refs: inputs.sources.gate_receipt_source_refs,
        gate_receipts: inputs.gate_receipts,
        gate_outputs: inputs.gate_outputs,
        evidence_commits: inputs.sources.evidence_commits,
        judgments_ref: inputs.sources.judgments_ref,
        judgments: inputs.judgments,
        rationale_ref: inputs.sources.rationale_ref,
        assumptions: inputs.assumptions,
        checks,
        outcome,
    };
    let artifact_manifest = ArtifactManifestSnapshot {
        deliverables_ref: inputs.sources.deliverables_ref,
        deliverables: inputs.deliverables,
        artifacts: inputs.artifact_entries,
    };
    let assurance_case_snapshot_digest =
        sha256_digest(DigestDomain::AssuranceCaseSnapshot, &assurance_snapshot)
            .map(|digest| format!("sha256:{digest}"))
            .map_err(|error| {
                RepoError::Message(format!(
                    "cannot canonicalize Resolution assurance snapshot: {error}"
                ))
            })?;
    let artifact_manifest_digest = sha256_digest(DigestDomain::Artifact, &artifact_manifest)
        .map(|digest| format!("sha256:{digest}"))
        .map_err(|error| {
            RepoError::Message(format!(
                "cannot canonicalize Resolution artifact manifest: {error}"
            ))
        })?;

    Ok(RecordedResolutionEvaluation {
        checks,
        outcome,
        assurance_case_snapshot_digest,
        artifact_manifest_digest,
        gate_run_refs,
        judgment_refs,
        residual_risk_refs,
    })
}

/// Everything §27, §28.4, §42 and §36.2 need, read once from disk.
///
/// Grouped into one type rather than four more parameters because all four
/// requirements are answers to the same question — *who decided this, and were
/// they entitled to* — and splitting them across the call site made it easy to
/// pass one and forget the rest.
pub struct Authority<'a> {
    pub register: &'a AuthorityRegister,
    /// The persisted authorization, if the Warrant has one.
    pub authorization: Option<&'a AuthorizationRecord>,
    /// The contract digest the Warrant compiles to RIGHT NOW. `None` when it
    /// would not compile, which makes requirement 1 unanswerable rather than
    /// satisfied.
    pub current_contract_digest: Option<&'a str>,
    pub judgments: &'a [Judgment],
    /// `None` means no `rationale.toml` — the residual-risk question was never
    /// asked. `Some(&[])` means it was asked and the answer was none. Law 15:
    /// those are different, and only one of them may pass.
    pub assumptions: Option<&'a [Assumption]>,
    /// §27.3 condition 1 — repository policy, set by a human.
    pub policy_allows_automated_resolution: bool,
    /// The actor whose work is being resolved, from [`Repository::performer`].
    ///
    /// Threaded rather than written as a literal here, and the reason is the
    /// direction the two could diverge. §27.3 condition 4 is "performer and
    /// resolver identities are distinct", so this name is what stops an actor
    /// closing its own delivery. A second copy of the string that fell out of
    /// step with `performer()` would not fail loudly — it would compare the
    /// resolver against a name nobody uses any more, find them distinct, and
    /// let the real performer through.
    pub performer: &'a str,
    /// Exact resolver named by a persisted Resolution. `None` preserves
    /// dry-run behavior, which asks whether any eligible resolver exists.
    pub recorded_resolver: Option<RecordedResolver<'a>>,
}

impl Authority<'_> {
    /// §56.1 requirement 1 — the EXACT authorized Contract Revision.
    ///
    /// Both halves are load-bearing. A record in any state but `Authorized` is a
    /// proposal, and one whose digest no longer matches the tree authorizes a
    /// revision that has since been edited — the precise failure the word
    /// "exact" exists to catch.
    #[must_use]
    pub fn contract_is_authorized(&self) -> bool {
        let (Some(record), Some(current)) = (self.authorization, self.current_contract_digest)
        else {
            return false;
        };
        crate::authorize::authorizes_current_contract(record, current)
    }

    /// §56.1 requirement 6 — no required unknown remains.
    ///
    /// §36.3 is the record this asks about: an assumption carrying
    /// `epistemic_status: blocking_unknown`, which
    /// [`EpistemicStatus::blocks_readiness`] already identifies. A Warrant
    /// resting on an unresolved blocking unknown may not close, because the
    /// thing it does not know is by its own declaration load-bearing.
    ///
    /// Absent `rationale.toml` is `false`, not vacuously true. The same
    /// asked-versus-unasked rule the residual-risk check applies: a Warrant that
    /// never declared its assumptions has not shown it has no blocking ones, and
    /// Law 15 keeps those two apart.
    ///
    /// # What this deliberately does NOT count
    ///
    /// An earlier comment here read "the adequacy warnings ARE required
    /// unknowns", and that conflated two different things. `war check` reports
    /// ten Warrants whose §39.3 adequacy review executed no attacks — a real gap,
    /// and one worth fixing — but it is a gap in the REVIEW, not an assumption
    /// the Warrant declared it was unsure about. Folding it in here would make
    /// requirement 6 unfixable by the record it names, since resolving it would
    /// mean running attacks rather than resolving an unknown.
    #[must_use]
    pub fn no_required_unknown_remains(&self) -> bool {
        use openwarrant_core::rationale::EpistemicStatus;

        let Some(assumptions) = self.assumptions else {
            return false;
        };
        !assumptions
            .iter()
            .any(|a| a.epistemic_status == EpistemicStatus::BlockingUnknown)
    }

    /// §56.1 requirement 9 — required judgments exist.
    ///
    /// A judgment counts only if it validates AND its author is recorded as
    /// authorized (§42's `require_authorized`). A Warrant needs a judgment for
    /// every residual risk it declares; one that declares none needs none, and
    /// that is a real pass rather than a vacuous one — but only once the
    /// question has been asked, which is what `assumptions.is_some()` records.
    #[must_use]
    pub fn required_judgments_exist(&self) -> bool {
        let Some(assumptions) = self.assumptions else {
            return false;
        };
        let required = crate::authorize::residual_risks_in(assumptions);
        if !self
            .judgments
            .iter()
            .all(|j| j.validate().is_ok() && j.require_authorized().is_ok())
        {
            return false;
        }
        required
            .iter()
            .all(|risk| self.judgments.iter().any(|j| refers_to(j, risk)))
    }

    /// §56.1 requirement 11 — residual risks have SUFFICIENT AUTHORITY.
    ///
    /// Distinct from requirement 9, which only asks whether the judgments exist.
    /// This asks whether the actor who made each one was entitled to accept
    /// organizational residual risk — §27.2 forbids it to agents outright, and
    /// holding `judge` is not holding `risk_acceptor`.
    #[must_use]
    pub fn residual_risks_are_covered(&self) -> bool {
        let Some(assumptions) = self.assumptions else {
            return false;
        };
        crate::authorize::residual_risks_in(assumptions)
            .iter()
            .all(|risk| {
                self.judgments.iter().any(|j| {
                    refers_to(j, risk)
                        && self
                            .register
                            .actor(&j.actor)
                            .is_some_and(|a| a.may_accept_residual_risk().is_ok())
                })
            })
    }

    /// §56.1 requirement 13 — the resolver holds the role.
    ///
    /// Asks the register whether ANY assigned actor could lawfully resolve this
    /// Warrant, given the performer and §27.3's conditions. `false` means nobody
    /// may close it — which is the honest answer for a repository whose role
    /// register is empty, and is why an empty register grants nothing.
    #[must_use]
    pub fn a_resolver_is_eligible(&self, assurance: &str, declared: &[String]) -> bool {
        let context = PolicyResolutionContext {
            policy_allows: self.policy_allows_automated_resolution,
            assurance_level: assurance,
            // Conservative on purpose: an obligation is treated as
            // non-mechanical unless the Warrant declares none at all.
            // §27.3 lets a policy service close only mechanical work, and
            // guessing in the permissive direction here would hand a
            // machine exactly the Warrants it may not touch.
            all_obligations_mechanical: declared.is_empty(),
            residual_risk_judgment_required: self
                .assumptions
                .is_none_or(|a| !crate::authorize::residual_risks_in(a).is_empty()),
        };
        if let Some(recorded) = self.recorded_resolver {
            return self
                .register
                .actor(recorded.actor)
                .is_some_and(|assignment| {
                    openwarrant_core::legacy_disposition::is_canonical_utc(
                        &assignment.effective_time,
                    ) && openwarrant_core::legacy_disposition::is_canonical_utc(
                        recorded.effective_at,
                    ) && assignment.effective_time.as_str() <= recorded.effective_at
                        && assignment.may_resolve(self.performer, context).is_ok()
                });
        }
        self.register
            .eligible_resolver(self.performer, context)
            .is_some()
    }
}

/// Whether a judgment addresses a given residual risk.
///
/// Matched through the assumption's own `judgment_ref` when it declares one, and
/// otherwise through the judgment's `basis_refs`. Both directions are checked
/// because §36.2 points assumption→judgment while §42 points judgment→basis, and
/// a record written from either side must be readable from the other.
fn refers_to(judgment: &Judgment, risk: &crate::authorize::RequestedResidualRisk) -> bool {
    let by_ref = !risk.judgment_ref.is_empty()
        && (risk.judgment_ref == judgment.id
            || risk.judgment_ref == format!("judgment://{}", judgment.id));
    let by_basis = judgment
        .basis_refs
        .iter()
        .any(|b| b == &risk.assumption_id || b == &format!("assumption://{}", risk.assumption_id));
    by_ref || by_basis
}

/// §56.1 — every gate a required obligation cites must have an admissible result.
///
/// "Admissible" is §44.5's conjunction, already modelled as
/// [`GateRun::satisfies_required_pass`]: askable AND completed AND pass. An
/// unaskable gate cannot pass (SAS §99 criterion 19), and a run that timed out
/// is a blocking unknown rather than a failure (RQ-054).
///
/// A cited gate with NO recorded run is unmet, not vacuously satisfied. That is
/// the whole point: the gate must have been asked, not merely referenced.
///
/// # Zero cited gates is UNMET, and this comment used to say the opposite
///
/// The prose here claimed "obligations citing no gate impose nothing — the
/// requirement is about gates that were cited", while the code below has always
/// returned `false` for an empty list. One of them was wrong, and it was the
/// prose: a reader trusting it would conclude a Warrant citing no gate had
/// passed requirement 5, when it had failed.
///
/// The code is kept. §56.1 asks whether every required gate has an admissible
/// result, and a Warrant that names no gate has produced no mechanical proof of
/// anything — reporting that as satisfied would let a Warrant clear the
/// requirement by declaring nothing, which is the shape of false completion this
/// resolver exists to refuse. It is the same rule
/// [`required_deliverables_exist`] applies to a Warrant that declares no
/// deliverables.
///
/// The consequence is real and is not a defect: OW-WAR-0001 through 0018 were
/// authored before the Gate Registry existed, cite no gates, and therefore
/// cannot satisfy requirement 5. Adding gate bullets to their assurance atoms
/// now would move the contract digest and would be editing a document to make a
/// tool go green. It needs an amendment somebody authorizes, not a quiet edit.
#[must_use]
pub(crate) fn every_required_gate_has_admissible_result(
    cited_uris: &[String],
    runs: &[AdmissibleGateRun],
) -> bool {
    if cited_uris.is_empty() {
        return false;
    }
    cited_uris.iter().all(|uri| {
        let key = uri.trim_start_matches("gate://");
        runs.iter().any(|run| run.gate() == key)
    })
}

/// §56.1 requirement 12 — runtime receipts match the basis.
///
/// # Which stages this is actually about
///
/// §48.4 is a clause of §48, *Katana integration*: "Katana SHALL return, at
/// minimum: session identity, Dispatch digest, PromptIR digest, ... receipt
/// digest." §49.3 gives BLUT the equivalent duty for its own lineage. Those two
/// are the executors that return receipts.
///
/// A `human`, `agent` or `service` stage does not produce a §48.4 receipt. It
/// submits through §47's Stage Submission, which is a different record with a
/// different shape, and demanding a runtime receipt from a person would make
/// this requirement permanently unmeetable for the ordinary case.
///
/// So a Warrant that dispatches nothing to a runtime has no receipts to match,
/// and that is a genuine pass rather than a vacuous one: the milestones atom is
/// REQUIRED by the delivery profile, so the question was always asked, and the
/// answer "no runtime stages" is recorded in the contract itself.
///
/// # Where this is strict
///
/// A Warrant with a `katana` or `blut` stage needs a receipt, and none exists
/// anywhere in this repository — Katana has no checkout and nothing has been
/// dispatched. Those Warrants report unmet, which is correct.
///
/// Reading the executor kind from the atom is safe because the atom is part of
/// the Compilation Basis: mis-declaring a Katana stage as `human` to dodge this
/// would change the contract digest, and is a false declaration rather than a
/// hole in this check.
///
/// A Warrant whose milestones atom will not parse is unmet, not exempt.
#[must_use]
pub fn runtime_receipts_match_the_basis(
    basis: Option<&openwarrant_compiler::CompilationBasis>,
) -> bool {
    use openwarrant_core::milestones::ExecutorKind;

    let Some(basis) = basis else {
        return false;
    };
    let atoms: Vec<_> = basis
        .atoms
        .iter()
        .filter(|a| a.role == "milestones")
        .collect();
    if atoms.is_empty() {
        // The profile requires one. Its absence means the Warrant did not
        // compile as claimed, and an unanswerable requirement is unmet.
        return false;
    }

    let mut runtime_stages = 0usize;
    for atom in atoms {
        // `from_utf8`, not `from_utf8_lossy`. Lossy decoding replaces invalid
        // bytes with U+FFFD, which can turn bytes that described a katana stage
        // into YAML that parses cleanly and describes no runtime stage at all —
        // a silent pass in the one direction that matters. Invalid UTF-8 lands
        // in the same place as unparseable YAML: unmet.
        let Ok(text) = std::str::from_utf8(&atom.bytes) else {
            return false;
        };
        let Ok(graph) = openwarrant_core::milestones::parse(text) else {
            return false;
        };
        runtime_stages += graph
            .stages
            .iter()
            .filter(|s| matches!(s.executor_kind, ExecutorKind::Katana | ExecutorKind::Blut))
            .count();
    }

    // No receipt store exists yet, so any runtime stage is unmet. Written as a
    // comparison rather than `runtime_stages == 0` so that wiring receipts in
    // later is a change to this one expression.
    runtime_stages == 0
}

/// §38.6 — would this resolve SATISFIED, given the dispositions on record?
///
/// # This is a different question from §56.1's thirteen, and conflating them
/// # manufactures a false completion
///
/// Requirement 4 asks whether every obligation is *dispositioned*. A verdict of
/// `not_established` IS a disposition, so it satisfies requirement 4 — correctly:
/// §38.5 is about whether the question was answered, not about the answer.
///
/// Nothing in §56.1 then asks what the answers WERE. So a Warrant could meet all
/// thirteen requirements while every one of its obligations came back
/// `not_established`, and a resolver reading only the thirteen would call it
/// ready to close. §38.6 is the clause that stops this: a delivery resolves
/// satisfied only when every required obligation is established, or accepted
/// with residual risk under sufficient authority.
///
/// [`Disposition::permits_satisfied`] and [`ObligationSet::aggregate`] both
/// modelled this from the start and no binary called either — the same "a
/// function nothing computed" shape this module documents about itself. It is
/// computed here, from the ADMISSIBLE verifications rather than from the atom,
/// because an inadmissible verdict must not contribute to an outcome any more
/// than it contributes to a disposition.
///
/// `None` means the question cannot be answered yet: some obligation has no
/// admissible verdict at all. That is neither satisfied nor unsatisfied, which
/// is Law 15 — Unknown is neither failure nor pass.
#[must_use]
pub fn would_resolve_satisfied(declared: &[String], admissible: &[&Verification]) -> Option<bool> {
    if declared.is_empty() {
        return None;
    }
    let mut all_permit = true;
    for id in declared {
        let verdict = admissible.iter().find(|v| &v.obligation == id)?;
        if !verdict.disposition.permits_satisfied() {
            all_permit = false;
        }
    }
    Some(all_permit)
}

/// §37 — a required deliverable exists when it VALIDATES and its target is
/// actually present.
///
/// Validation alone would accept a record pointing at a file nobody produced,
/// which is precisely the claim this requirement exists to test. Zero required
/// deliverables is `false`, not vacuously true: a Warrant that declares nothing
/// has not shown that anything was delivered.
#[must_use]
pub fn required_deliverables_exist(
    root: &camino::Utf8Path,
    deliverables: &[Deliverable],
    declared_obligations: &[String],
) -> bool {
    let required: Vec<&Deliverable> = deliverables.iter().filter(|d| d.required).collect();
    !required.is_empty()
        && required
            .iter()
            .all(|d| d.validate(declared_obligations).is_ok() && root.join(&d.target_ref).exists())
}

/// §37.2 — a content-addressed deliverable's recorded digest must match the
/// bytes on disk.
///
/// A digest that is merely PRESENT proves nothing, so this RECOMPUTES it from
/// the file. A deliverable claiming content addressing whose target cannot be
/// read fails rather than being skipped — an unreadable artifact is not a
/// verified one.
#[must_use]
pub fn artifact_digests_verify(root: &camino::Utf8Path, deliverables: &[Deliverable]) -> bool {
    let addressed: Vec<&Deliverable> = deliverables
        .iter()
        .filter(|d| d.content_addressed)
        .collect();
    !addressed.is_empty()
        && addressed.iter().all(|d| {
            let Some(p) = d.provenance.as_ref() else {
                return false;
            };
            let recorded = p.content_digest.trim_start_matches("sha256:");
            match std::fs::read(root.join(&d.target_ref)) {
                Ok(bytes) => openwarrant_compiler::sha256_hex(&bytes) == recorded,
                Err(_) => false,
            }
        })
}

const MAX_LIVE_RESOLUTION_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;

fn live_artifact_entries(
    repo: &Repository,
    deliverables: &[Deliverable],
) -> Vec<ArtifactSnapshotEntry> {
    deliverables
        .iter()
        .filter(|deliverable| deliverable.required || deliverable.content_addressed)
        .filter_map(|deliverable| {
            crate::repo::read_repository_regular_bounded(
                &repo.root.join(&deliverable.target_ref),
                &repo.root,
                "Resolution artifact",
                MAX_LIVE_RESOLUTION_ARTIFACT_BYTES,
            )
            .ok()
            .map(|bytes| ArtifactSnapshotEntry {
                deliverable_id: deliverable.id.clone(),
                target_ref: deliverable.target_ref.clone(),
                observed_sha256: format!("sha256:{}", openwarrant_compiler::sha256_hex(&bytes)),
            })
        })
        .collect()
}

#[cfg(test)]
fn live_gate_subjects(
    contract_digest: &str,
    artifact_entries: &[ArtifactSnapshotEntry],
) -> BTreeSet<String> {
    std::iter::once(format!("contract:sha256:{contract_digest}"))
        .chain(
            artifact_entries
                .iter()
                .map(|artifact| artifact.observed_sha256.clone()),
        )
        .collect()
}

/// `war resolve <alias> --dry-run`.
pub fn run(repo: &Repository, alias: &str) -> Result<Report, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let mut report = Report::default();

    let verifications = repo.load_verifications(&dir)?;
    let deliverables = repo.load_deliverables(&dir)?;

    let performer = repo.performer();
    let register = repo.load_authority_register()?;
    let authorization = repo.load_authorization(&dir)?;
    let judgments = repo.load_judgments(&dir)?;
    let assumptions = repo.load_rationale(&dir)?;
    // The digest the Warrant compiles to right now, which requirement 1 compares
    // the signature against. A Warrant that will not compile yields `None`, and
    // requirement 1 is then unanswerable rather than satisfied.
    let current_contract_digest = match (&one.basis, &one.validated) {
        (Some(basis), Some(validated)) => openwarrant_compiler::lower(basis, validated)
            .ok()
            .and_then(|ir| ir.contract_digest().ok()),
        _ => None,
    };
    let authority = Authority {
        register: &register,
        authorization: authorization.as_ref(),
        current_contract_digest: current_contract_digest.as_deref(),
        judgments: &judgments,
        assumptions: assumptions.as_deref(),
        policy_allows_automated_resolution: repo.config.policy.allow_automated_resolution,
        performer: &performer,
        recorded_resolver: None,
    };
    let declared = declared_obligations(&one);
    let artifacts = ArtifactChecks {
        required_deliverables_exist: required_deliverables_exist(
            &repo.root,
            &deliverables.records,
            &declared,
        ),
        artifact_digests_verify: artifact_digests_verify(&repo.root, &deliverables.records),
    };
    let artifact_entries = live_artifact_entries(repo, &deliverables.records);
    let gate_evidence = if let Some(contract_digest) = current_contract_digest.as_ref() {
        let basis = crate::gate_evidence::ResolutionEvidenceBasis::new(
            contract_digest,
            artifact_entries.iter().map(|artifact| {
                crate::gate_evidence::ResolutionEvidenceArtifact {
                    deliverable_id: artifact.deliverable_id.clone(),
                    target_ref: artifact.target_ref.clone(),
                    observed_sha256: artifact.observed_sha256.clone(),
                }
            }),
            authorization
                .as_ref()
                .and_then(|record| {
                    crate::authorize::gate_bindings_for_current(record, contract_digest)
                })
                .into_iter()
                .flatten()
                .cloned(),
        );
        match basis {
            Ok(basis) => {
                crate::gate_evidence::load_admissible_for_resolution(repo, &basis, &register)
            }
            Err(error) => crate::gate_evidence::GateEvidenceSet {
                runs: Vec::new(),
                failures: vec![("resolution-evidence-basis".to_owned(), error)],
            },
        }
    } else {
        crate::gate_evidence::GateEvidenceSet::default()
    };

    let checks = evaluate(
        &one,
        &verifications.records,
        &gate_evidence.runs,
        &authority,
        artifacts,
    );
    for (path, why) in &deliverables.failures {
        report.push(Diagnostic::error(
            "deliverables.malformed",
            path.clone(),
            format!("{why} — an unreadable deliverables file is not an absent one"),
        ));
    }
    for (path, why) in &verifications.failures {
        report.push(Diagnostic::error(
            "verification.malformed",
            path.clone(),
            format!("{why} — an unreadable verification is not an absent one"),
        ));
    }
    for (path, why) in &gate_evidence.failures {
        report.push(Diagnostic::warn(
            "gate-evidence.rejected-candidate",
            path.clone(),
            format!("{why} — rejected evidence is never promoted into Resolution"),
        ));
    }

    // §38.6, reported alongside the thirteen and never folded into them. See
    // `would_resolve_satisfied` for why these are different questions.
    let assurance = one
        .validated
        .as_ref()
        .map(|v| v.assurance_level.to_string())
        .unwrap_or_else(|| "basic".to_owned());
    let declared = declared_obligations(&one);
    let admissible: Vec<&Verification> = verifications
        .records
        .iter()
        .filter(|v| v.admissible_for(&assurance).is_ok())
        .collect();
    let outcome = would_resolve_satisfied(&declared, &admissible);
    let unestablished: Vec<&str> = declared
        .iter()
        .filter(|id| {
            admissible
                .iter()
                .find(|v| &&v.obligation == id)
                .is_some_and(|v| !v.disposition.permits_satisfied())
        })
        .map(String::as_str)
        .collect();

    let unmet = checks.unmet();

    if unmet.is_empty() {
        report.push(Diagnostic::pass(
            "resolution.requirements",
            format!(
                "{alias}: all {} §56.1 requirements are met",
                RESOLUTION_REQUIREMENTS.len()
            ),
        ));
        report.note(
            "All thirteen are met, but no resolution has been RECORDED. §56.2's record \
             needs an authorizer, an acting role, and a stated meaning — none of which \
             this command may invent."
                .to_owned(),
        );
        push_outcome(&mut report, alias, outcome, &unestablished);
        return Ok(report);
    }

    // Each unmet requirement is named. "9 of 13" tells a reader nothing about
    // whether to worry; the names tell them what to fix.
    for (group, requirement, met) in checks
        .as_pairs()
        .into_iter()
        .map(|(name, met)| ("§56.1", name, met))
    {
        if met {
            report.push(Diagnostic::pass(
                "resolution.requirement-met",
                format!("{alias}: {group} {requirement}"),
            ));
        } else {
            report.push(Diagnostic::unknown(
                "resolution.requirement-unmet",
                repo.relative(&dir.join("manifest.toml")),
                format!("{alias}: {group} {requirement} — not established"),
            ));
        }
    }

    report.note(format!(
        "{} of {} §56.1 requirements are unmet, so {alias} cannot be resolved. That is \
         the correct outcome, not a defect: closing a Warrant while independence is \
         absent and no authorization record exists would manufacture the false \
         completion this system exists to prevent.",
        unmet.len(),
        RESOLUTION_REQUIREMENTS.len()
    ));
    push_outcome(&mut report, alias, outcome, &unestablished);
    Ok(report)
}

/// The obligation ids a Warrant declares, as the parser reads them.
pub(crate) fn declared_obligations(one: &crate::repo::Loaded) -> Vec<String> {
    one.basis
        .as_ref()
        .map(|b| {
            b.atoms
                .iter()
                .filter(|a| a.role == "assurance")
                .filter_map(|a| {
                    openwarrant_core::obligation::parse(&String::from_utf8_lossy(&a.bytes)).ok()
                })
                .flat_map(|set| set.obligations.into_iter().map(|o| o.id))
                .collect()
        })
        .unwrap_or_default()
}

/// Report §38.6 as its own line, never folded into the thirteen.
///
/// A Warrant meeting all thirteen requirements with an obligation on record as
/// `not_established` is READY to be resolved and would resolve NOT SATISFIED.
/// Printing only "all thirteen met" would let a reader take the first half of
/// that sentence for the whole of it.
fn push_outcome(report: &mut Report, alias: &str, outcome: Option<bool>, unestablished: &[&str]) {
    match outcome {
        Some(true) => report.push(Diagnostic::pass(
            "resolution.outcome",
            format!("{alias}: §38.6 every required obligation is established or accepted"),
        )),
        Some(false) => report.note(format!(
            "§38.6: {alias} would resolve NOT SATISFIED even once the §56.1 \
             requirements are met. {} obligation(s) are on record as not established \
             or refuted: {}. A disposition is not an establishment — requirement 4 \
             asks whether the question was answered, and §38.6 asks what the answer \
             was.",
            unestablished.len(),
            unestablished.join(", ")
        )),
        None => report.note(format!(
            "§38.6: whether {alias} would resolve satisfied is UNKNOWN — at least one \
             declared obligation has no admissible verification. Unknown is neither \
             failure nor pass (Law 15)."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use openwarrant_core::deliverable::{ArtifactProvenance, DeliverableKind};

    fn scratch(label: &str) -> Utf8PathBuf {
        let mut p = Utf8PathBuf::from_path_buf(std::env::temp_dir()).expect("utf-8 temp");
        p.push(format!("ow-deliv-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("scratch");
        p
    }

    fn provenance(digest: &str) -> ArtifactProvenance {
        ArtifactProvenance {
            producer: "p".into(),
            producing_attempt: "a".into(),
            contract_digest: "c".into(),
            input_digests: vec![],
            tool_or_runtime_identity: "t".into(),
            creation_method: "authored".into(),
            content_digest: digest.into(),
            media_type: "text/plain".into(),
            classification: "internal".into(),
            retention: "r".into(),
            source_holder: "git".into(),
        }
    }

    fn deliverable(target: &str, digest: Option<&str>) -> Deliverable {
        Deliverable {
            id: "D-001".into(),
            title: "t".into(),
            kind: DeliverableKind::File,
            target_ref: target.into(),
            required: true,
            content_addressed: digest.is_some(),
            provenance_required: digest.is_some(),
            obligation_refs: vec![],
            provenance: digest.map(provenance),
        }
    }

    fn admitted_gate_run(id: &str, gate: &str) -> AdmissibleGateRun {
        crate::gate_evidence::admitted_for_test(GateRun {
            id: id.to_owned(),
            gate: gate.to_owned(),
            askability: openwarrant_core::Askability::Askable,
            execution_status: openwarrant_core::ExecutionStatus::Completed,
            verdict: openwarrant_core::Verdict::Pass,
            reason_code: Some(openwarrant_core::ReasonCode::Passed),
        })
    }

    const GATE: &str = "software.repo.war-check@1.0.0";

    #[test]
    fn live_gate_subjects_use_the_receipt_contract_identity() {
        let contract_digest = "a".repeat(64);
        let subjects = live_gate_subjects(&contract_digest, &[]);

        assert_eq!(
            subjects,
            [format!("contract:sha256:{contract_digest}")]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn a_cited_gate_with_an_askable_completed_pass_is_admissible() {
        let runs = vec![admitted_gate_run("GR-1", GATE)];
        assert!(every_required_gate_has_admissible_result(
            &[format!("gate://{GATE}")],
            &runs
        ));
    }

    /// A cited gate with NO recorded run is unmet, not vacuously satisfied.
    /// The gate must have been ASKED, not merely referenced.
    #[test]
    fn a_cited_gate_with_no_run_is_unmet() {
        assert!(!every_required_gate_has_admissible_result(
            &[format!("gate://{GATE}")],
            &[]
        ));
    }

    /// A passing run for a DIFFERENT gate must not satisfy this citation.
    #[test]
    fn a_run_for_another_gate_does_not_count() {
        let runs = vec![admitted_gate_run("GR-1", "some.other.gate@1.0.0")];
        assert!(!every_required_gate_has_admissible_result(
            &[format!("gate://{GATE}")],
            &runs
        ));
    }

    /// Every cited gate must pass, not merely one of them.
    #[test]
    fn one_admitted_gate_does_not_carry_an_unadmitted_sibling() {
        let runs = vec![admitted_gate_run("GR-1", GATE)];
        assert!(!every_required_gate_has_admissible_result(
            &[
                format!("gate://{GATE}"),
                "gate://second.gate@1.0.0".to_string()
            ],
            &runs
        ));
    }

    /// The happy path, so the refusals below mean something.
    #[test]
    fn a_present_target_with_a_matching_digest_verifies() {
        let root = scratch("ok");
        std::fs::write(root.join("artifact.txt"), b"hello").expect("write");
        let digest = format!("sha256:{}", openwarrant_compiler::sha256_hex(b"hello"));
        let d = vec![deliverable("artifact.txt", Some(&digest))];
        assert!(required_deliverables_exist(&root, &d, &[]));
        assert!(artifact_digests_verify(&root, &d));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A digest that is merely PRESENT must not pass. This is the check that
    /// makes content addressing mean anything.
    #[test]
    fn a_digest_that_does_not_match_the_bytes_is_refused() {
        let root = scratch("mismatch");
        std::fs::write(root.join("artifact.txt"), b"hello").expect("write");
        let d = vec![deliverable(
            "artifact.txt",
            Some(&format!("sha256:{}", "0".repeat(64))),
        )];
        assert!(!artifact_digests_verify(&root, &d));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A record pointing at a file nobody produced is the exact claim §37 tests.
    #[test]
    fn a_missing_target_fails_both_predicates() {
        let root = scratch("missing");
        let digest = format!("sha256:{}", openwarrant_compiler::sha256_hex(b"hello"));
        let d = vec![deliverable("never-written.txt", Some(&digest))];
        assert!(!required_deliverables_exist(&root, &d, &[]));
        assert!(
            !artifact_digests_verify(&root, &d),
            "an unreadable artifact is not a verified one"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Claims content addressing, carries no digest.
    #[test]
    fn content_addressed_without_provenance_is_refused() {
        let root = scratch("noprov");
        std::fs::write(root.join("a.txt"), b"x").expect("write");
        let mut d = deliverable("a.txt", None);
        d.content_addressed = true;
        assert!(!artifact_digests_verify(&root, &[d]));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Declaring nothing is not the same as having delivered everything.
    #[test]
    fn an_empty_deliverable_set_satisfies_neither() {
        let root = scratch("empty");
        assert!(!required_deliverables_exist(&root, &[], &[]));
        assert!(!artifact_digests_verify(&root, &[]));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A deliverable pointing at proof nobody wrote (§38).
    #[test]
    fn a_deliverable_citing_an_undeclared_obligation_is_refused() {
        let root = scratch("dangling");
        std::fs::write(root.join("a.txt"), b"x").expect("write");
        let mut d = deliverable("a.txt", None);
        d.obligation_refs = vec!["OBL-999".into()];
        assert!(!required_deliverables_exist(
            &root,
            &[d],
            &["OBL-001".to_string()]
        ));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// §56.1's thirteen, and the fact that `unmet` names them rather than
    /// counting them.
    #[test]
    fn unmet_requirements_are_named_not_counted() {
        let mut checks = ResolutionChecks::all_met();
        checks.independence_requirements_met = false;
        let unmet = checks.unmet();
        assert_eq!(unmet, vec!["independence requirements are met"]);
    }

    #[test]
    fn persisted_record_identifiers_must_be_nonblank_and_unique() {
        #[derive(Clone)]
        struct Record(&'static str);

        let blank = record_ids(&[Record(" ")], |record| record.0)
            .expect_err("blank persisted evidence id must fail");
        assert!(blank.to_string().contains("blank record identifier"));

        let duplicate = record_ids(&[Record("R-1"), Record("R-1")], |record| record.0)
            .expect_err("duplicate persisted evidence id must fail");
        assert!(
            duplicate
                .to_string()
                .contains("duplicate record identifier")
        );
    }

    fn basis_with_milestones(yaml: &str) -> openwarrant_compiler::CompilationBasis {
        openwarrant_compiler::CompilationBasis {
            manifest_source: "manifest.toml".to_owned(),
            manifest_bytes: b"(manifest)".to_vec(),
            manifest: openwarrant_core::Manifest {
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
                atoms: vec![],
                currency: None,
            },
            atoms: vec![openwarrant_compiler::AtomSource {
                ordinal: 45,
                role: "milestones".to_owned(),
                jurisdiction: "authored".to_owned(),
                source: "atoms/45-milestones.yaml".to_owned(),
                bytes: yaml.as_bytes().to_vec(),
                required: true,
            }],
            scope: None,
        }
    }

    // Shaped after a real atom, schema line included. A fixture that merely
    // looked like YAML would fail to parse and the "no runtime stages" case
    // would then pass for the wrong reason — the unparseable branch, not the
    // one under test.
    const HUMAN_ONLY: &str = r#"schema: "oh.war/milestones/v1"

milestones:
  - id: "M-001"
    title: "m"
    stage_refs: ["STAGE-001"]

stages:
  - id: "STAGE-001"
    title: "s"
    executor_kind: "human"
    responsibility_tier: "T1"
"#;

    const WITH_KATANA: &str = r#"schema: "oh.war/milestones/v1"

milestones:
  - id: "M-001"
    title: "m"
    stage_refs: ["STAGE-001"]

stages:
  - id: "STAGE-001"
    title: "s"
    executor_kind: "katana"
    responsibility_tier: "T1"
"#;

    /// §56.1 requirement 12 must be able to both pass and refuse, or wiring it
    /// up replaced one constant with another.
    ///
    /// The pass case is a Warrant that dispatches nothing to a runtime; the
    /// refusal is one katana stage with no receipt behind it.
    #[test]
    fn runtime_receipts_pass_without_runtime_stages_and_refuse_with_them() {
        let human = basis_with_milestones(HUMAN_ONLY);
        assert!(
            runtime_receipts_match_the_basis(Some(&human)),
            "a Warrant with no katana or blut stage has no receipts to match"
        );

        let katana = basis_with_milestones(WITH_KATANA);
        assert!(
            !runtime_receipts_match_the_basis(Some(&katana)),
            "a katana stage needs a receipt, and none exists"
        );

        assert!(
            !runtime_receipts_match_the_basis(None),
            "a Warrant that did not compile cannot answer this"
        );

        let no_milestones = openwarrant_compiler::CompilationBasis {
            atoms: vec![],
            ..basis_with_milestones(HUMAN_ONLY)
        };
        assert!(
            !runtime_receipts_match_the_basis(Some(&no_milestones)),
            "the delivery profile requires a milestones atom; its absence is unmet, not exempt"
        );

        let unparseable = basis_with_milestones("stages: [ this is not yaml");
        assert!(
            !runtime_receipts_match_the_basis(Some(&unparseable)),
            "an unreadable milestones atom is not an empty one"
        );
    }

    use openwarrant_core::rationale::EpistemicStatus;

    fn authority_with(assumptions: Option<&[Assumption]>) -> Authority<'_> {
        static EMPTY: AuthorityRegister = AuthorityRegister {
            assignments: Vec::new(),
        };
        Authority {
            register: &EMPTY,
            authorization: None,
            current_contract_digest: None,
            judgments: &[],
            assumptions,
            policy_allows_automated_resolution: false,
            performer: "claude",
            recorded_resolver: None,
        }
    }

    fn assumption(id: &str, status: EpistemicStatus) -> Assumption {
        Assumption {
            id: id.to_owned(),
            statement: "s".to_owned(),
            epistemic_status: status,
            evidence_refs: vec![],
            judgment_ref: String::new(),
            consequence_if_false: "c".to_owned(),
            resolution_requirement: "r".to_owned(),
            validated_by: vec![],
        }
    }

    /// §56.1 requirement 6 in all three of its states.
    ///
    /// The commit that introduced this check claimed both new requirements were
    /// tested in both directions and only requirement 12 was. External review
    /// caught the overstatement; this is the missing half.
    #[test]
    fn a_blocking_unknown_refuses_and_its_absence_passes_only_once_asked() {
        assert!(
            !authority_with(None).no_required_unknown_remains(),
            "no rationale.toml means the question was never asked, which is not an answer"
        );

        let none_blocking = [assumption("A-001", EpistemicStatus::AcceptedResidualRisk)];
        assert!(
            authority_with(Some(&none_blocking)).no_required_unknown_remains(),
            "assumptions declared, none blocking — asked and answered"
        );

        assert!(
            authority_with(Some(&[])).no_required_unknown_remains(),
            "an empty declaration is still a declaration"
        );

        let blocking = [
            assumption("A-001", EpistemicStatus::AcceptedResidualRisk),
            assumption("A-002", EpistemicStatus::BlockingUnknown),
        ];
        assert!(
            !authority_with(Some(&blocking[..])).no_required_unknown_remains(),
            "one blocking unknown among several assumptions must still refuse"
        );
    }

    /// Nothing this command computes may default to true. A requirement it
    /// cannot answer is unmet, which is §32's fail-closed rule applied to §56.
    #[test]
    fn an_unanswerable_requirement_is_unmet_not_assumed() {
        let checks = ResolutionChecks::default();
        assert_eq!(
            checks.unmet().len(),
            RESOLUTION_REQUIREMENTS.len(),
            "a default ResolutionChecks must satisfy nothing"
        );
    }

    #[test]
    fn persisted_resolution_checks_named_resolver_not_any_eligible_actor() {
        use std::collections::BTreeSet;

        use openwarrant_core::{ActorKind, ActorRole, RoleAssignment};

        let assignment = |actor: &str, kind, effective_time: &str| RoleAssignment {
            actor: actor.to_owned(),
            actor_kind: kind,
            roles: BTreeSet::from([ActorRole::Resolver]),
            assigned_by: "human:owner".to_owned(),
            effective_time: effective_time.to_owned(),
            note: None,
        };
        let register = AuthorityRegister::new(vec![
            assignment("OtherHuman", ActorKind::Human, "2026-08-20T00:00:00Z"),
            assignment("Named", ActorKind::Agent, "2026-08-20T00:00:00Z"),
        ]);
        let authority = Authority {
            register: &register,
            authorization: None,
            current_contract_digest: None,
            judgments: &[],
            assumptions: Some(&[]),
            policy_allows_automated_resolution: false,
            performer: "claude",
            recorded_resolver: Some(RecordedResolver {
                actor: "Named",
                effective_at: "2026-08-27T00:00:00Z",
            }),
        };
        assert!(
            !authority.a_resolver_is_eligible("basic", &["OBL-001".to_owned()]),
            "another eligible human cannot substitute for named agent"
        );

        let register = AuthorityRegister::new(vec![assignment(
            "Named",
            ActorKind::Human,
            "2026-08-28T00:00:00Z",
        )]);
        let authority = Authority {
            register: &register,
            recorded_resolver: Some(RecordedResolver {
                actor: "Named",
                effective_at: "2026-08-27T00:00:00Z",
            }),
            ..authority
        };
        assert!(
            !authority.a_resolver_is_eligible("basic", &["OBL-001".to_owned()]),
            "role assignment effective after Resolution cannot authorize it"
        );

        let register = AuthorityRegister::new(vec![assignment(
            "Named",
            ActorKind::Human,
            "2026-08-26T00:00:00Z",
        )]);
        let authority = Authority {
            register: &register,
            recorded_resolver: Some(RecordedResolver {
                actor: "Named",
                effective_at: "2026-08-27T00:00:00Z",
            }),
            ..authority
        };
        assert!(authority.a_resolver_is_eligible("basic", &["OBL-001".to_owned()]));
    }
}
