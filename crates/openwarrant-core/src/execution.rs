// SPDX-License-Identifier: AGPL-3.0-or-later
//! Dispatch, submission, attempts, and the four remedies (SAS §47, §51–§53).
//! RQ-042, RQ-043, RQ-045.
//!
//! # The three rules with teeth
//!
//! §51.2, **no self-completion**: a performer may request `continue`, `verify`,
//! `block`, `amend`, or `cancel`, and *"SHALL NOT set or request authoritative
//! resolution."* [`RequestedNextAction`] contains exactly those five. There is no
//! `resolve` variant to guard against, so a performer cannot ask for one.
//!
//! §51.3, **claims are not evidence**: *"Claims are assertions to be tested.
//! Their structured form does not make them evidence."* [`PerformerClaim`] and
//! [`PerformerObservation`] are separate types from anything a verifier admits,
//! and observation admissibility defaults to `performer_report_only`.
//!
//! §53.5, **four remedies stay four**: a blocker needs a condition resolved, a
//! deviation needs exception authority, a decision needs an ADR, and a
//! discovered gap may require architecture or authoring correction. Collapsing
//! them means applying the wrong remedy — which usually means silently repairing
//! something §53.4 says must not be silently repaired.
//!
//! # Attempt lineage
//!
//! §52.5: *"Every attempt SHALL have one parent except the initial attempt.
//! Failure evidence is attached by the runtime or control plane, not selected and
//! rewritten by the performer."* [`Attempt::validate`] enforces the parent rule
//! per kind, and the evidence field is documented as control-plane-owned.

use serde::{Deserialize, Serialize};

use crate::vocab::vocabulary;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ExecutionError {
    #[error("unknown {vocabulary} {found:?}; SAS defines {known}")]
    UnknownTerm {
        vocabulary: &'static str,
        found: String,
        known: String,
    },
    #[error("dispatch {id:?} omits {field}, which §47.1 requires")]
    DispatchIncomplete { id: String, field: &'static str },
    #[error(
        "dispatch compilation for {id:?} omitted required normative source \
         {omitted_source:?}. §47.2: the compiler SHALL preserve every required \
         normative source, and §33.6 forbids dropping one to fit a budget"
    )]
    NormativeSourceOmitted {
        id: String,
        // NOT named `source`: thiserror reads that as the error's cause.
        omitted_source: String,
    },
    #[error(
        "dispatch {id:?} is a repair attempt carrying no prior failure evidence. \
         §47.2 requires the compiler to include it, and a repair that cannot see \
         what failed is a retry"
    )]
    RepairWithoutPriorEvidence { id: String },
    #[error(
        "actor projection {actor:?} for dispatch {id:?} alters the normative stage \
         contract: {difference}. §47.3 permits representation to differ and requires \
         the SAME normative contract"
    )]
    ProjectionAltersContract {
        id: String,
        actor: String,
        difference: String,
    },
    #[error(
        "submission {id:?} requests {action:?}. §51.2: the performer SHALL NOT set \
         or request authoritative resolution — it may request continue, verify, \
         block, amend, or cancel"
    )]
    SelfCompletionAttempted { id: String, action: String },
    #[error(
        "observation {id:?} claims admissibility {admissibility} on a \
         performer-authored report. §51.3: claims are assertions to be tested, and \
         their structured form does not make them evidence"
    )]
    ClaimPresentedAsEvidence {
        id: String,
        admissibility: Admissibility,
    },
    #[error(
        "attempt {id:?} is {kind} and names no parent. §52.5: every attempt SHALL \
         have one parent except the initial attempt"
    )]
    AttemptWithoutParent { id: String, kind: AttemptKind },
    #[error(
        "attempt {id:?} is initial and names parent {parent:?}. An initial attempt \
         is the first execution under a Contract Revision (§52.1)"
    )]
    InitialWithParent { id: String, parent: String },
    #[error(
        "attempt {id:?} is a replay but its basis changed ({change:?}). §52.2: a \
         replay uses an IDENTICAL logical basis — a changed basis is a repair or a \
         restart, and calling it a replay hides that the inputs moved"
    )]
    ReplayWithChangedBasis { id: String, change: String },
    #[error("attempt {id:?} is a repair and carries no prior failure evidence (§52.3)")]
    RepairWithoutEvidence { id: String },
    #[error("{remedy} {id:?} omits {field}, which §53 requires")]
    RemedyIncomplete {
        remedy: &'static str,
        id: String,
        field: &'static str,
    },
    #[error(
        "discovered gap {id:?} is marked repaired in place. §53.4: a discovered gap \
         is NOT silently repaired — it is dispositioned through clarification, \
         amendment, ADR, child WAR, or supersession"
    )]
    GapSilentlyRepaired { id: String },
    #[error(
        "decision proposal {id:?} is treated as normative without a proposed ADR. \
         §53.3: a decision proposal SHALL become a proposed ADR before it becomes \
         normative"
    )]
    DecisionNormativeWithoutAdr { id: String },
}

vocabulary!(
    /// §51.2's five permitted requests. There is deliberately no `resolve`.
    RequestedNextAction, "requested next action", ExecutionError, {
        Continue => "continue",
        Verify => "verify",
        Block => "block",
        Amend => "amend",
        Cancel => "cancel",
    }
);

vocabulary!(
    /// §52's attempt kinds.
    AttemptKind, "attempt kind", ExecutionError, {
        Initial => "initial",
        Replay => "replay",
        Repair => "repair",
        Restart => "restart",
    }
);

impl AttemptKind {
    /// §52.5 — every attempt has a parent except the initial one.
    #[must_use]
    pub const fn requires_parent(self) -> bool {
        !matches!(self, Self::Initial)
    }

    /// §52.4 — a restart abandons the prior approach and needs authorization.
    #[must_use]
    pub const fn requires_authorization(self) -> bool {
        matches!(self, Self::Restart)
    }
}

vocabulary!(
    /// How much weight a performer observation may carry (§51.3).
    Admissibility, "admissibility", ExecutionError, {
        PerformerReportOnly => "performer_report_only",
        VerifierObserved => "verifier_observed",
        GateProduced => "gate_produced",
    }
);

impl Admissibility {
    /// Whether a PERFORMER may assert this level about its own work.
    ///
    /// Only `performer_report_only`. §51.3 says a claim's structured form does
    /// not make it evidence, and the other two describe who observed the thing —
    /// which is not the performer's to declare.
    #[must_use]
    pub const fn is_self_assertable(self) -> bool {
        matches!(self, Self::PerformerReportOnly)
    }
}

vocabulary!(
    /// §53's four remedies, which §53.5 requires to stay distinct.
    RemedyKind, "remedy", ExecutionError, {
        Blocker => "blocker",
        Deviation => "deviation",
        DecisionProposal => "decision_proposal",
        DiscoveredGap => "discovered_gap",
    }
);

impl RemedyKind {
    /// §53.5's reason each category exists, verbatim.
    #[must_use]
    pub const fn needs(self) -> &'static str {
        match self {
            Self::Blocker => "a condition resolved",
            Self::Deviation => "exception authority",
            Self::DecisionProposal => "an ADR",
            Self::DiscoveredGap => "architecture or authoring correction",
        }
    }
}

vocabulary!(
    /// §53.4's dispositions for a discovered gap. Repair-in-place is absent.
    GapDisposition, "gap disposition", ExecutionError, {
        Clarification => "clarification",
        Amendment => "amendment",
        Adr => "adr",
        ChildWar => "child_war",
        Supersession => "supersession",
    }
);

/// §47.1's Stage Dispatch — *"the only packet given to a stateless actor."*
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StageDispatch {
    pub dispatch_id: String,
    pub warrant_ref: String,
    pub contract_revision: u32,
    pub contract_digest: String,
    pub milestone_id: String,
    pub stage_id: String,
    pub attempt_id: String,
    pub attempt_kind: Option<AttemptKind>,
    pub attempt_basis_digest: String,
    pub objective: String,
    #[serde(default)]
    pub non_goals: Vec<String>,
    #[serde(default)]
    pub instructions: Vec<String>,
    pub workspace_basis_digest: String,
    pub context_manifest_digest: String,
    #[serde(default)]
    pub input_artifacts: Vec<String>,
    #[serde(default)]
    pub required_outputs: Vec<String>,
    #[serde(default)]
    pub obligation_refs: Vec<String>,
    #[serde(default)]
    pub capability_policy_digest: String,
    #[serde(default)]
    pub resource_envelope: ResourceEnvelope,
    /// §47.2 — recorded, not discarded.
    #[serde(default)]
    pub omitted_subgraphs: Vec<String>,
    /// §52.3 / §47.2 — a repair sees what failed.
    #[serde(default)]
    pub prior_failure_evidence_refs: Vec<String>,
}

/// §47.1's resource envelope.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ResourceEnvelope {
    #[serde(default)]
    pub wall_time_seconds: u64,
    #[serde(default)]
    pub cpu_cores: u32,
    #[serde(default)]
    pub memory_bytes: u64,
    #[serde(default)]
    pub gpu_count: u32,
    #[serde(default)]
    pub network_policy: String,
    #[serde(default)]
    pub spend_limit_currency: String,
    #[serde(default)]
    pub spend_limit_amount: String,
}

impl StageDispatch {
    /// §47.1 and §47.2.
    ///
    /// `required_normative_sources` comes from the contract, not the dispatch, so
    /// a compiler cannot satisfy the check by narrowing what it claims was
    /// required.
    pub fn validate(&self, required_normative_sources: &[String]) -> Result<(), ExecutionError> {
        for (field, value) in [
            ("warrant_ref", &self.warrant_ref),
            ("contract_digest", &self.contract_digest),
            ("stage_id", &self.stage_id),
            ("attempt_id", &self.attempt_id),
            ("attempt_basis_digest", &self.attempt_basis_digest),
            ("objective", &self.objective),
            ("workspace_basis_digest", &self.workspace_basis_digest),
            ("context_manifest_digest", &self.context_manifest_digest),
        ] {
            if value.trim().is_empty() {
                return Err(ExecutionError::DispatchIncomplete {
                    id: self.dispatch_id.clone(),
                    field,
                });
            }
        }
        // §47.2 — every required normative source survives projection.
        for source in required_normative_sources {
            if self.omitted_subgraphs.contains(source) {
                return Err(ExecutionError::NormativeSourceOmitted {
                    id: self.dispatch_id.clone(),
                    omitted_source: source.clone(),
                });
            }
        }
        // §47.2 — a repair that cannot see what failed is a retry.
        if self.attempt_kind == Some(AttemptKind::Repair)
            && self.prior_failure_evidence_refs.is_empty()
        {
            return Err(ExecutionError::RepairWithoutPriorEvidence {
                id: self.dispatch_id.clone(),
            });
        }
        Ok(())
    }

    /// §47.3 — representation may differ; the normative contract may not.
    ///
    /// Compares the fields that ARE the contract. Objective, instructions and
    /// formatting are presentation and may legitimately differ per actor.
    pub fn same_normative_contract_as(
        &self,
        other: &Self,
        actor: &str,
    ) -> Result<(), ExecutionError> {
        for (field, a, b) in [
            (
                "contract_digest",
                self.contract_digest.as_str(),
                other.contract_digest.as_str(),
            ),
            (
                "attempt_basis_digest",
                self.attempt_basis_digest.as_str(),
                other.attempt_basis_digest.as_str(),
            ),
            (
                "context_manifest_digest",
                self.context_manifest_digest.as_str(),
                other.context_manifest_digest.as_str(),
            ),
            (
                "workspace_basis_digest",
                self.workspace_basis_digest.as_str(),
                other.workspace_basis_digest.as_str(),
            ),
            ("stage_id", self.stage_id.as_str(), other.stage_id.as_str()),
        ] {
            if a != b {
                return Err(ExecutionError::ProjectionAltersContract {
                    id: self.dispatch_id.clone(),
                    actor: actor.to_owned(),
                    difference: format!("{field}: {a:?} vs {b:?}"),
                });
            }
        }
        if self.contract_revision != other.contract_revision {
            return Err(ExecutionError::ProjectionAltersContract {
                id: self.dispatch_id.clone(),
                actor: actor.to_owned(),
                difference: format!(
                    "contract_revision: {} vs {}",
                    self.contract_revision, other.contract_revision
                ),
            });
        }
        if self.obligation_refs != other.obligation_refs {
            return Err(ExecutionError::ProjectionAltersContract {
                id: self.dispatch_id.clone(),
                actor: actor.to_owned(),
                difference: "obligation_refs differ".to_owned(),
            });
        }
        Ok(())
    }
}

/// §51.1's performer claim. An assertion, not evidence (§51.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerformerClaim {
    pub id: String,
    pub statement: String,
}

/// §51.1's performer observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerformerObservation {
    pub id: String,
    pub statement: String,
    #[serde(default)]
    pub evidence_ref: String,
    #[serde(default = "PerformerObservation::default_admissibility")]
    pub admissibility: Admissibility,
}

impl PerformerObservation {
    const fn default_admissibility() -> Admissibility {
        Admissibility::PerformerReportOnly
    }

    /// §51.3 — a performer cannot upgrade its own report.
    pub fn validate(&self) -> Result<(), ExecutionError> {
        if self.admissibility.is_self_assertable() {
            Ok(())
        } else {
            Err(ExecutionError::ClaimPresentedAsEvidence {
                id: self.id.clone(),
                admissibility: self.admissibility,
            })
        }
    }
}

/// §51.1's Stage Submission — §37.4's claim envelope, in execution form.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StageSubmission {
    pub dispatch_id: String,
    pub attempt_id: String,
    pub contract_digest: String,
    pub stage_id: String,
    #[serde(default)]
    pub claims: Vec<PerformerClaim>,
    #[serde(default)]
    pub artifact_refs: Vec<String>,
    #[serde(default)]
    pub performer_observations: Vec<PerformerObservation>,
    #[serde(default)]
    pub blockers: Vec<Blocker>,
    #[serde(default)]
    pub deviation_proposals: Vec<Deviation>,
    #[serde(default)]
    pub decision_proposals: Vec<DecisionProposal>,
    #[serde(default)]
    pub discovered_gaps: Vec<DiscoveredGap>,
    #[serde(default)]
    pub unresolved_items: Vec<String>,
    pub requested_next_action: Option<RequestedNextAction>,
}

impl StageSubmission {
    pub fn validate(&self) -> Result<(), ExecutionError> {
        for o in &self.performer_observations {
            o.validate()?;
        }
        for g in &self.discovered_gaps {
            g.validate()?;
        }
        for d in &self.decision_proposals {
            d.validate()?;
        }
        for b in &self.blockers {
            b.validate()?;
        }
        for d in &self.deviation_proposals {
            d.validate()?;
        }
        Ok(())
    }

    /// §51.2 — reject an action outside the permitted five, by name.
    ///
    /// The typed field cannot express `resolve`; this exists for actions arriving
    /// as text from a stateless actor.
    pub fn validate_requested_action(
        id: &str,
        action: &str,
    ) -> Result<RequestedNextAction, ExecutionError> {
        action
            .parse()
            .map_err(|_| ExecutionError::SelfCompletionAttempted {
                id: id.to_owned(),
                action: action.to_owned(),
            })
    }
}

/// §52's attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    pub id: String,
    pub kind: AttemptKind,
    #[serde(default)]
    pub parent_attempt_ref: String,
    /// §52.2 — a replay's basis is unchanged; `"none"` is the SAS's own value.
    #[serde(default)]
    pub basis_change: String,
    /// §52.5 — attached by the runtime or control plane, NOT selected and
    /// rewritten by the performer. Recorded here; ownership is enforced by who
    /// may write an attempt, not by this type.
    #[serde(default)]
    pub prior_failure_evidence_refs: Vec<String>,
    #[serde(default)]
    pub prior_work_product_ref: String,
    #[serde(default)]
    pub authorized_by: String,
}

impl Attempt {
    pub fn validate(&self) -> Result<(), ExecutionError> {
        if self.kind.requires_parent() && self.parent_attempt_ref.trim().is_empty() {
            return Err(ExecutionError::AttemptWithoutParent {
                id: self.id.clone(),
                kind: self.kind,
            });
        }
        if self.kind == AttemptKind::Initial && !self.parent_attempt_ref.trim().is_empty() {
            return Err(ExecutionError::InitialWithParent {
                id: self.id.clone(),
                parent: self.parent_attempt_ref.clone(),
            });
        }
        // §52.2 — "identical logical basis". A replay whose basis moved is a
        // repair or a restart wearing a cheaper label.
        if self.kind == AttemptKind::Replay {
            let change = self.basis_change.trim();
            if !change.is_empty() && change != "none" {
                return Err(ExecutionError::ReplayWithChangedBasis {
                    id: self.id.clone(),
                    change: change.to_owned(),
                });
            }
        }
        if self.kind == AttemptKind::Repair && self.prior_failure_evidence_refs.is_empty() {
            return Err(ExecutionError::RepairWithoutEvidence {
                id: self.id.clone(),
            });
        }
        Ok(())
    }
}

/// §53.1.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Blocker {
    pub id: String,
    pub condition_ref: String,
    pub reason: String,
    pub owner_ref: String,
    pub required_to_unblock: String,
}

impl Blocker {
    pub fn validate(&self) -> Result<(), ExecutionError> {
        for (field, value) in [
            ("condition_ref", &self.condition_ref),
            ("reason", &self.reason),
            ("owner_ref", &self.owner_ref),
            ("required_to_unblock", &self.required_to_unblock),
        ] {
            if value.trim().is_empty() {
                return Err(ExecutionError::RemedyIncomplete {
                    remedy: "blocker",
                    id: self.id.clone(),
                    field,
                });
            }
        }
        Ok(())
    }
}

/// §53.2.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Deviation {
    pub id: String,
    pub affected_contract_path: String,
    pub proposed_change: String,
    pub reason: String,
    /// §53.2's impact block. Stated, because a deviation whose impact is unstated
    /// cannot be granted exception authority on any informed basis.
    pub impact: String,
}

impl Deviation {
    pub fn validate(&self) -> Result<(), ExecutionError> {
        for (field, value) in [
            ("affected_contract_path", &self.affected_contract_path),
            ("proposed_change", &self.proposed_change),
            ("reason", &self.reason),
            ("impact", &self.impact),
        ] {
            if value.trim().is_empty() {
                return Err(ExecutionError::RemedyIncomplete {
                    remedy: "deviation",
                    id: self.id.clone(),
                    field,
                });
            }
        }
        Ok(())
    }
}

/// §53.3.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DecisionProposal {
    pub id: String,
    pub statement: String,
    /// §53.3 — a decision proposal SHALL become a proposed ADR before it becomes
    /// normative.
    #[serde(default)]
    pub proposed_adr_ref: String,
    #[serde(default)]
    pub treated_as_normative: bool,
}

impl DecisionProposal {
    pub fn validate(&self) -> Result<(), ExecutionError> {
        if self.treated_as_normative && self.proposed_adr_ref.trim().is_empty() {
            return Err(ExecutionError::DecisionNormativeWithoutAdr {
                id: self.id.clone(),
            });
        }
        Ok(())
    }
}

/// §53.4.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DiscoveredGap {
    pub id: String,
    pub statement: String,
    /// What under-specified the thing: contract, SAS, ADR, gate, or source.
    #[serde(default)]
    pub under_specified: String,
    #[serde(default)]
    pub disposition: Option<GapDisposition>,
    /// Whether someone just fixed it in place. §53.4 forbids exactly this, so it
    /// has to be expressible in order to be refusable.
    #[serde(default)]
    pub repaired_in_place: bool,
}

impl DiscoveredGap {
    pub fn validate(&self) -> Result<(), ExecutionError> {
        if self.repaired_in_place {
            return Err(ExecutionError::GapSilentlyRepaired {
                id: self.id.clone(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    /// §51.2's five, transcribed — and nothing else.
    #[test]
    fn the_requested_actions_match_the_sas() {
        assert_eq!(
            RequestedNextAction::ALL
                .iter()
                .map(|a| a.as_str())
                .collect::<Vec<_>>(),
            ["continue", "verify", "block", "amend", "cancel"]
        );
    }

    /// §51.2 — a performer cannot request authoritative resolution, because the
    /// type cannot express it and text asking for it is refused.
    #[test]
    fn a_performer_cannot_request_resolution() {
        for forbidden in [
            "resolve",
            "resolved",
            "satisfied",
            "accept",
            "complete",
            "close",
        ] {
            assert!(
                RequestedNextAction::from_str(forbidden).is_err(),
                "{forbidden:?} parsed as a permitted action"
            );
            assert!(
                matches!(
                    StageSubmission::validate_requested_action("SUB-1", forbidden),
                    Err(ExecutionError::SelfCompletionAttempted { .. })
                ),
                "{forbidden:?} was accepted"
            );
        }
        for allowed in ["continue", "verify", "block", "amend", "cancel"] {
            assert!(StageSubmission::validate_requested_action("SUB-1", allowed).is_ok());
        }
    }

    /// §51.3 — structured form does not make a claim evidence.
    #[test]
    fn a_performer_cannot_upgrade_its_own_report_to_evidence() {
        for claimed in [Admissibility::VerifierObserved, Admissibility::GateProduced] {
            let o = PerformerObservation {
                id: "POB-001".into(),
                statement: "The local test exited zero.".into(),
                evidence_ref: "artifact://local-log".into(),
                admissibility: claimed,
            };
            assert!(
                matches!(
                    o.validate(),
                    Err(ExecutionError::ClaimPresentedAsEvidence { .. })
                ),
                "a performer asserted {claimed}"
            );
        }
        let honest = PerformerObservation {
            id: "POB-001".into(),
            statement: "The local test exited zero.".into(),
            evidence_ref: "artifact://local-log".into(),
            admissibility: Admissibility::PerformerReportOnly,
        };
        assert_eq!(honest.validate(), Ok(()));
    }

    /// The default must be the weakest. A performer report that defaults to
    /// anything else is a claim promoted by omission.
    #[test]
    fn observation_admissibility_defaults_to_performer_report_only() {
        let parsed: PerformerObservation = serde_json::from_str(
            r#"{"id":"POB-1","statement":"it worked","evidence_ref":"artifact://x"}"#,
        )
        .expect("deserialize");
        assert_eq!(parsed.admissibility, Admissibility::PerformerReportOnly);
    }

    // ---- §52 -------------------------------------------------------------

    fn attempt(kind: AttemptKind) -> Attempt {
        Attempt {
            id: "A2".into(),
            kind,
            parent_attempt_ref: if kind == AttemptKind::Initial {
                String::new()
            } else {
                "attempt://A1".into()
            },
            basis_change: String::new(),
            prior_failure_evidence_refs: if kind == AttemptKind::Repair {
                vec!["gate-run://GR-1".into()]
            } else {
                vec![]
            },
            prior_work_product_ref: String::new(),
            authorized_by: String::new(),
        }
    }

    /// §52.5 — one parent except the initial attempt, both directions.
    #[test]
    fn every_attempt_has_one_parent_except_the_initial() {
        for kind in [
            AttemptKind::Replay,
            AttemptKind::Repair,
            AttemptKind::Restart,
        ] {
            assert!(kind.requires_parent(), "{kind}");
            let mut a = attempt(kind);
            a.parent_attempt_ref.clear();
            assert!(
                matches!(
                    a.validate(),
                    Err(ExecutionError::AttemptWithoutParent { .. })
                ),
                "{kind} with no parent was accepted"
            );
        }
        assert!(!AttemptKind::Initial.requires_parent());
        assert_eq!(attempt(AttemptKind::Initial).validate(), Ok(()));

        let mut orphan_initial = attempt(AttemptKind::Initial);
        orphan_initial.parent_attempt_ref = "attempt://A0".into();
        assert!(matches!(
            orphan_initial.validate(),
            Err(ExecutionError::InitialWithParent { .. })
        ));
    }

    /// §52.2 — "identical logical basis". A replay whose basis moved is a repair
    /// or a restart wearing a cheaper label, and the label is what decides
    /// whether anyone re-authorizes.
    #[test]
    fn a_replay_whose_basis_changed_is_not_a_replay() {
        let mut r = attempt(AttemptKind::Replay);
        r.basis_change = "workspace bumped to a newer commit".into();
        assert!(matches!(
            r.validate(),
            Err(ExecutionError::ReplayWithChangedBasis { .. })
        ));

        r.basis_change = "none".into();
        assert_eq!(r.validate(), Ok(()));
    }

    /// §52.3 — a repair that cannot see what failed is a retry.
    #[test]
    fn a_repair_needs_the_prior_failure_evidence() {
        let mut r = attempt(AttemptKind::Repair);
        r.prior_failure_evidence_refs.clear();
        assert!(matches!(
            r.validate(),
            Err(ExecutionError::RepairWithoutEvidence { .. })
        ));
    }

    #[test]
    fn only_a_restart_requires_authorization() {
        assert!(AttemptKind::Restart.requires_authorization());
        for k in [
            AttemptKind::Initial,
            AttemptKind::Replay,
            AttemptKind::Repair,
        ] {
            assert!(!k.requires_authorization(), "{k}");
        }
    }

    // ---- §47 -------------------------------------------------------------

    fn dispatch() -> StageDispatch {
        StageDispatch {
            dispatch_id: "D-1".into(),
            warrant_ref: "war://uuid".into(),
            contract_revision: 3,
            contract_digest: "sha256:c".into(),
            milestone_id: "M2".into(),
            stage_id: "STAGE-003".into(),
            attempt_id: "A1".into(),
            attempt_kind: Some(AttemptKind::Initial),
            attempt_basis_digest: "sha256:b".into(),
            objective: "implement the scalar backend".into(),
            non_goals: vec![],
            instructions: vec![],
            workspace_basis_digest: "sha256:w".into(),
            context_manifest_digest: "sha256:m".into(),
            input_artifacts: vec![],
            required_outputs: vec![],
            obligation_refs: vec!["OBL-001".into()],
            capability_policy_digest: "sha256:p".into(),
            resource_envelope: ResourceEnvelope::default(),
            omitted_subgraphs: vec![],
            prior_failure_evidence_refs: vec![],
        }
    }

    /// §47.2 — a required normative source cannot be projected away.
    #[test]
    fn dispatch_compilation_cannot_omit_a_required_normative_source() {
        let mut d = dispatch();
        d.omitted_subgraphs = vec!["ctx://governing-policy".into()];
        let err = d
            .validate(&["ctx://governing-policy".to_owned()])
            .unwrap_err();
        assert!(
            matches!(err, ExecutionError::NormativeSourceOmitted { .. }),
            "{err}"
        );

        // Omitting something NOT required is exactly what projection is for.
        assert_eq!(d.validate(&["ctx://something-else".to_owned()]), Ok(()));
    }

    #[test]
    fn a_repair_dispatch_carries_what_failed() {
        let mut d = dispatch();
        d.attempt_kind = Some(AttemptKind::Repair);
        assert!(matches!(
            d.validate(&[]),
            Err(ExecutionError::RepairWithoutPriorEvidence { .. })
        ));
        d.prior_failure_evidence_refs = vec!["gate-run://GR-1".into()];
        assert_eq!(d.validate(&[]), Ok(()));
    }

    /// §47.3 — representation may differ per actor; the contract may not.
    #[test]
    fn actor_projections_may_differ_in_form_but_not_in_contract() {
        let human = dispatch();

        // Legitimate: same contract, different presentation.
        let mut katana = human.clone();
        katana.objective = "Implement scalar backend. Steps: 1) ... 2) ...".into();
        katana.instructions = vec!["run the harness".into()];
        katana.non_goals = vec!["do not touch the SIMD path".into()];
        assert_eq!(human.same_normative_contract_as(&katana, "katana"), Ok(()));

        // Not legitimate: the contract itself moved.
        for (name, mut altered) in [
            ("digest", human.clone()),
            ("revision", human.clone()),
            ("obligations", human.clone()),
        ] {
            match name {
                "digest" => altered.contract_digest = "sha256:other".into(),
                "revision" => altered.contract_revision = 4,
                _ => altered.obligation_refs = vec!["OBL-002".into()],
            }
            assert!(
                matches!(
                    human.same_normative_contract_as(&altered, "katana"),
                    Err(ExecutionError::ProjectionAltersContract { .. })
                ),
                "{name} changed and was accepted"
            );
        }
    }

    // ---- §53 -------------------------------------------------------------

    /// §53.5 — the four remedies stay four, and each needs something different.
    #[test]
    fn the_four_remedies_stay_distinct() {
        assert_eq!(
            RemedyKind::ALL
                .iter()
                .map(|r| r.as_str())
                .collect::<Vec<_>>(),
            [
                "blocker",
                "deviation",
                "decision_proposal",
                "discovered_gap"
            ]
        );
        assert_eq!(RemedyKind::Blocker.needs(), "a condition resolved");
        assert_eq!(RemedyKind::Deviation.needs(), "exception authority");
        assert_eq!(RemedyKind::DecisionProposal.needs(), "an ADR");
        assert_eq!(
            RemedyKind::DiscoveredGap.needs(),
            "architecture or authoring correction"
        );
        // Four distinct remedies. If two ever agree, applying one for the other
        // becomes invisible.
        let mut needs: Vec<&str> = RemedyKind::ALL.iter().map(|r| r.needs()).collect();
        needs.sort_unstable();
        let before = needs.len();
        needs.dedup();
        assert_eq!(needs.len(), before, "two remedies need the same thing");
    }

    /// §53.4 — a discovered gap is NOT silently repaired.
    #[test]
    fn a_discovered_gap_cannot_be_repaired_in_place() {
        let mut g = DiscoveredGap {
            id: "GAP-001".into(),
            statement: "the contract does not say which fixture version".into(),
            under_specified: "contract".into(),
            disposition: None,
            repaired_in_place: true,
        };
        assert!(matches!(
            g.validate(),
            Err(ExecutionError::GapSilentlyRepaired { .. })
        ));

        g.repaired_in_place = false;
        g.disposition = Some(GapDisposition::Amendment);
        assert_eq!(g.validate(), Ok(()));
    }

    /// §53.4's five dispositions — and repair-in-place is not among them.
    #[test]
    fn the_gap_dispositions_match_the_sas() {
        assert_eq!(
            GapDisposition::ALL
                .iter()
                .map(|d| d.as_str())
                .collect::<Vec<_>>(),
            [
                "clarification",
                "amendment",
                "adr",
                "child_war",
                "supersession"
            ]
        );
        assert!(GapDisposition::from_str("repair_in_place").is_err());
    }

    /// §53.3 — a decision becomes normative only through a proposed ADR.
    #[test]
    fn a_decision_cannot_become_normative_without_an_adr() {
        let mut d = DecisionProposal {
            id: "DEC-001".into(),
            statement: "use the restricted reader".into(),
            proposed_adr_ref: String::new(),
            treated_as_normative: true,
        };
        assert!(matches!(
            d.validate(),
            Err(ExecutionError::DecisionNormativeWithoutAdr { .. })
        ));
        d.proposed_adr_ref = "adr://proposed/OW-ADR-0007".into();
        assert_eq!(d.validate(), Ok(()));
    }

    #[test]
    fn each_remedy_names_its_missing_field() {
        let b = Blocker {
            id: "BLK-1".into(),
            condition_ref: "PRE-002".into(),
            reason: String::new(),
            owner_ref: "role://fixture-owner".into(),
            required_to_unblock: "Restore or supersede fixture.".into(),
        };
        match b.validate() {
            Err(ExecutionError::RemedyIncomplete { remedy, field, .. }) => {
                assert_eq!(remedy, "blocker");
                assert_eq!(field, "reason");
            }
            other => panic!("accepted a blocker with no reason: {other:?}"),
        }

        let d = Deviation {
            id: "DEV-1".into(),
            affected_contract_path: "/execution/network".into(),
            proposed_change: "policy: allowlisted".into(),
            reason: "Dependency absent from cache.".into(),
            impact: String::new(),
        };
        match d.validate() {
            Err(ExecutionError::RemedyIncomplete { remedy, field, .. }) => {
                assert_eq!(remedy, "deviation");
                assert_eq!(field, "impact");
            }
            other => panic!("accepted a deviation with no stated impact: {other:?}"),
        }
    }

    #[test]
    fn a_submission_validates_every_remedy_it_carries() {
        let s = StageSubmission {
            dispatch_id: "D-1".into(),
            attempt_id: "A1".into(),
            contract_digest: "sha256:c".into(),
            stage_id: "STAGE-003".into(),
            discovered_gaps: vec![DiscoveredGap {
                id: "GAP-001".into(),
                statement: "under-specified".into(),
                under_specified: "contract".into(),
                disposition: None,
                repaired_in_place: true,
            }],
            requested_next_action: Some(RequestedNextAction::Verify),
            ..StageSubmission::default()
        };
        assert!(matches!(
            s.validate(),
            Err(ExecutionError::GapSilentlyRepaired { .. })
        ));
    }

    #[test]
    fn vocabularies_round_trip() {
        for &a in AttemptKind::ALL {
            assert_eq!(AttemptKind::from_str(a.as_str()), Ok(a));
        }
        for &a in Admissibility::ALL {
            assert_eq!(Admissibility::from_str(a.as_str()), Ok(a));
        }
        for &r in RemedyKind::ALL {
            assert_eq!(RemedyKind::from_str(r.as_str()), Ok(r));
        }
    }

    #[test]
    fn a_dispatch_round_trips_through_json() {
        let d = dispatch();
        let s = serde_json::to_string(&d).expect("serialize");
        assert_eq!(
            serde_json::from_str::<StageDispatch>(&s).expect("deserialize"),
            d
        );
    }
}
