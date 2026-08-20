// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resolution, dispute, annulment, and ongoing validation (SAS §56, §57).
//! RQ-058, RQ-059.
//!
//! # Resolution is a conjunction, not a judgement call
//!
//! §56.1 lists thirteen things resolution SHALL verify. [`ResolutionChecks`] is
//! those thirteen as named booleans, and [`ResolutionChecks::unmet`] returns the
//! ones that are false. A resolution cannot be recorded while any is unmet —
//! there is no override, because a resolution recorded over an unmet check is
//! precisely the manufactured completion this project exists to prevent.
//!
//! # Three words that are not synonyms
//!
//! §56.3, §56.5, and §56.6 draw distinctions that are easy to blur and expensive
//! to get wrong:
//!
//! - **falsified** is *"appropriate only when the profile contains a falsifiable
//!   claim, such as an experiment or feasibility hypothesis. An ordinary failed
//!   delivery is normally `not_satisfied`, `cancelled`, or remains blocked."*
//!   Calling a failed delivery "falsified" dresses a miss up as a finding.
//! - **annulment** *"records that the resolution may not be relied upon. The
//!   original resolution remains historical."* It does not delete anything.
//! - **supersession** *"records replacement, not invalidity."* The superseded
//!   resolution was and remains correct.
//!
//! [`CommonOutcome::falsified_requires_falsifiable_claim`] enforces the first,
//! and [`annul`] and [`supersede`] both return new standing while leaving the
//! original untouched.

use serde::{Deserialize, Serialize};

use crate::state::ResolutionStanding;
use crate::vocab::vocabulary;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ResolutionError {
    #[error("unknown {vocabulary} {found:?}; SAS defines {known}")]
    UnknownTerm {
        vocabulary: &'static str,
        found: String,
        known: String,
    },
    #[error(
        "resolution {id:?} cannot be recorded: {count} of §56.1's {total} \
         requirements are unmet — {unmet}"
    )]
    RequirementsUnmet {
        id: String,
        count: usize,
        total: usize,
        unmet: String,
    },
    #[error(
        "resolution {id:?} claims outcome `falsified`, but the profile contains no \
         falsifiable claim. §56.3: falsified is appropriate only for an experiment \
         or feasibility hypothesis — an ordinary failed delivery is not_satisfied, \
         cancelled, or remains blocked"
    )]
    FalsifiedWithoutFalsifiableClaim { id: String },
    #[error("resolution {id:?} omits {field}, which §56.2 records")]
    RecordIncomplete { id: String, field: &'static str },
    #[error(
        "dispute {id:?} omits {field}. §56.4 requires the challenged resolution, \
         grounds, affected evidence or judgment, reliance policy, owner, and \
         required re-verification"
    )]
    DisputeIncomplete { id: String, field: &'static str },
    #[error(
        "resolution {id:?} is {standing} and cannot be relied upon. §56.5: \
         annulment records that the resolution may not be relied upon, and the \
         original remains historical"
    )]
    NotReliable {
        id: String,
        standing: ResolutionStanding,
    },
    #[error("monitor {id:?} omits {field}, which §57 requires")]
    MonitorIncomplete { id: String, field: &'static str },
}

vocabulary!(
    /// §56.2's `common_outcome`.
    CommonOutcome, "common outcome", ResolutionError, {
        Satisfied => "satisfied",
        NotSatisfied => "not_satisfied",
        Falsified => "falsified",
        Cancelled => "cancelled",
        Blocked => "blocked",
    }
);

impl CommonOutcome {
    /// §56.3 — `falsified` needs a falsifiable claim behind it.
    ///
    /// The failure this prevents is a vocabulary one: reporting a delivery that
    /// did not land as "falsified" makes a miss read as a finding, and turns an
    /// unmet obligation into a scientific result.
    #[must_use]
    pub const fn falsified_requires_falsifiable_claim(self) -> bool {
        matches!(self, Self::Falsified)
    }

    /// Whether this outcome accepts the work.
    #[must_use]
    pub const fn accepts(self) -> bool {
        matches!(self, Self::Satisfied)
    }
}

vocabulary!(
    /// §57's trigger actions.
    MonitorAction, "monitor action", ResolutionError, {
        DisputeResolution => "dispute_resolution",
        OpenInvestigation => "open_investigation",
        OpenRemediation => "open_remediation",
        ProposeAdr => "propose_adr",
        OpenSupersedingWar => "open_superseding_war",
        RequireReVerification => "require_re_verification",
    }
);

/// §56.1's thirteen requirements, verbatim and in order.
pub const RESOLUTION_REQUIREMENTS: [&str; 13] = [
    "exact authorized Contract Revision",
    "required deliverables exist",
    "artifact digests verify",
    "every required obligation is dispositioned",
    "every required gate has admissible result",
    "no required unknown remains",
    "no blocker remains",
    "deviations are dispositioned",
    "required judgments exist",
    "independence requirements are met",
    "residual risks have sufficient authority",
    "runtime receipts match the basis",
    "resolver holds the role",
];

/// §56.1 as thirteen named booleans.
///
/// Named rather than a count, so a report can say *which* requirement is unmet.
/// "11 of 13" tells a reader nothing about whether to worry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ResolutionChecks {
    pub exact_authorized_contract_revision: bool,
    pub required_deliverables_exist: bool,
    pub artifact_digests_verify: bool,
    pub every_required_obligation_dispositioned: bool,
    pub every_required_gate_has_admissible_result: bool,
    pub no_required_unknown_remains: bool,
    pub no_blocker_remains: bool,
    pub deviations_dispositioned: bool,
    pub required_judgments_exist: bool,
    pub independence_requirements_met: bool,
    pub residual_risks_have_sufficient_authority: bool,
    pub runtime_receipts_match_the_basis: bool,
    pub resolver_holds_the_role: bool,
}

impl ResolutionChecks {
    /// All thirteen satisfied — the only state in which a resolution is recordable.
    #[must_use]
    pub const fn all_met() -> Self {
        Self {
            exact_authorized_contract_revision: true,
            required_deliverables_exist: true,
            artifact_digests_verify: true,
            every_required_obligation_dispositioned: true,
            every_required_gate_has_admissible_result: true,
            no_required_unknown_remains: true,
            no_blocker_remains: true,
            deviations_dispositioned: true,
            required_judgments_exist: true,
            independence_requirements_met: true,
            residual_risks_have_sufficient_authority: true,
            runtime_receipts_match_the_basis: true,
            resolver_holds_the_role: true,
        }
    }

    /// Pair each §56.1 requirement with whether it is met, in the SAS's order.
    #[must_use]
    pub fn as_pairs(self) -> [(&'static str, bool); 13] {
        [
            (
                RESOLUTION_REQUIREMENTS[0],
                self.exact_authorized_contract_revision,
            ),
            (RESOLUTION_REQUIREMENTS[1], self.required_deliverables_exist),
            (RESOLUTION_REQUIREMENTS[2], self.artifact_digests_verify),
            (
                RESOLUTION_REQUIREMENTS[3],
                self.every_required_obligation_dispositioned,
            ),
            (
                RESOLUTION_REQUIREMENTS[4],
                self.every_required_gate_has_admissible_result,
            ),
            (RESOLUTION_REQUIREMENTS[5], self.no_required_unknown_remains),
            (RESOLUTION_REQUIREMENTS[6], self.no_blocker_remains),
            (RESOLUTION_REQUIREMENTS[7], self.deviations_dispositioned),
            (RESOLUTION_REQUIREMENTS[8], self.required_judgments_exist),
            (
                RESOLUTION_REQUIREMENTS[9],
                self.independence_requirements_met,
            ),
            (
                RESOLUTION_REQUIREMENTS[10],
                self.residual_risks_have_sufficient_authority,
            ),
            (
                RESOLUTION_REQUIREMENTS[11],
                self.runtime_receipts_match_the_basis,
            ),
            (RESOLUTION_REQUIREMENTS[12], self.resolver_holds_the_role),
        ]
    }

    /// The requirements that are not met, named.
    #[must_use]
    pub fn unmet(self) -> Vec<&'static str> {
        self.as_pairs()
            .into_iter()
            .filter(|(_, met)| !met)
            .map(|(name, _)| name)
            .collect()
    }
}

/// §56.2's resolution record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    pub id: String,
    pub common_outcome: CommonOutcome,
    pub profile_outcome: String,
    pub contract_revision: u32,
    pub contract_digest: String,
    pub assurance_case_snapshot_digest: String,
    pub artifact_manifest_digest: String,
    #[serde(default)]
    pub gate_run_refs: Vec<String>,
    #[serde(default)]
    pub judgment_refs: Vec<String>,
    #[serde(default)]
    pub residual_risk_refs: Vec<String>,
    pub resolved_by_ref: String,
    pub acting_role_ref: String,
    /// §56.2's `meaning`. What acceptance actually asserts, in one sentence —
    /// this is the field that stops "resolved" being read as "correct".
    pub meaning: String,
    pub effective_at: String,
    pub recorded_at: String,
    #[serde(default = "Resolution::valid")]
    pub standing: ResolutionStanding,
}

impl Resolution {
    const fn valid() -> ResolutionStanding {
        ResolutionStanding::Valid
    }

    /// §56.1 and §56.3 — record only what has actually been verified.
    ///
    /// `profile_has_falsifiable_claim` comes from the composition profile, not
    /// from the resolver's opinion, so `falsified` cannot be reached by asserting
    /// that the work was an experiment after it failed.
    pub fn validate(
        &self,
        checks: ResolutionChecks,
        profile_has_falsifiable_claim: bool,
    ) -> Result<(), ResolutionError> {
        let unmet = checks.unmet();
        if !unmet.is_empty() {
            return Err(ResolutionError::RequirementsUnmet {
                id: self.id.clone(),
                count: unmet.len(),
                total: RESOLUTION_REQUIREMENTS.len(),
                unmet: unmet.join("; "),
            });
        }
        if self.common_outcome.falsified_requires_falsifiable_claim()
            && !profile_has_falsifiable_claim
        {
            return Err(ResolutionError::FalsifiedWithoutFalsifiableClaim {
                id: self.id.clone(),
            });
        }
        for (field, value) in [
            ("contract_digest", &self.contract_digest),
            (
                "assurance_case_snapshot_digest",
                &self.assurance_case_snapshot_digest,
            ),
            ("artifact_manifest_digest", &self.artifact_manifest_digest),
            ("resolved_by_ref", &self.resolved_by_ref),
            ("acting_role_ref", &self.acting_role_ref),
            ("meaning", &self.meaning),
            ("effective_at", &self.effective_at),
        ] {
            if value.trim().is_empty() {
                return Err(ResolutionError::RecordIncomplete {
                    id: self.id.clone(),
                    field,
                });
            }
        }
        Ok(())
    }

    /// §56.5 — whether this resolution may be relied upon.
    #[must_use]
    pub const fn is_reliable(&self) -> bool {
        matches!(self.standing, ResolutionStanding::Valid)
    }

    pub fn require_reliable(&self) -> Result<(), ResolutionError> {
        if self.is_reliable() {
            Ok(())
        } else {
            Err(ResolutionError::NotReliable {
                id: self.id.clone(),
                standing: self.standing,
            })
        }
    }
}

/// §56.4's dispute.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Dispute {
    pub id: String,
    pub challenged_resolution: String,
    pub grounds: String,
    #[serde(default)]
    pub affected_evidence_or_judgment: Vec<String>,
    /// What may still be relied upon while the dispute is open. §56.4 lists this
    /// because "under dispute" with no reliance policy leaves every consumer
    /// guessing.
    pub reliance_policy: String,
    pub owner: String,
    pub required_re_verification: String,
}

impl Dispute {
    pub fn validate(&self) -> Result<(), ResolutionError> {
        for (field, value) in [
            ("challenged_resolution", &self.challenged_resolution),
            ("grounds", &self.grounds),
            ("reliance_policy", &self.reliance_policy),
            ("owner", &self.owner),
            ("required_re_verification", &self.required_re_verification),
        ] {
            if value.trim().is_empty() {
                return Err(ResolutionError::DisputeIncomplete {
                    id: self.id.clone(),
                    field,
                });
            }
        }
        if self.affected_evidence_or_judgment.is_empty() {
            return Err(ResolutionError::DisputeIncomplete {
                id: self.id.clone(),
                field: "affected_evidence_or_judgment",
            });
        }
        Ok(())
    }
}

/// §56.4 — open a dispute. Returns the DISPUTED copy; the original is unchanged.
#[must_use]
pub fn dispute(original: &Resolution) -> Resolution {
    Resolution {
        standing: ResolutionStanding::Disputed,
        ..original.clone()
    }
}

/// §56.5 — annul. *"The original resolution remains historical."*
///
/// Returns a new record rather than mutating, and takes `&Resolution` so the
/// original cannot be consumed. Annulment says the resolution may not be relied
/// upon; it does not say it never happened.
#[must_use]
pub fn annul(original: &Resolution) -> Resolution {
    Resolution {
        standing: ResolutionStanding::Annulled,
        ..original.clone()
    }
}

/// §56.6 — supersession *"records replacement, not invalidity."*
///
/// The superseded resolution keeps `Valid` standing, because it was and remains
/// correct for the contract it was recorded against. Marking it `Annulled` would
/// assert something false about work that was fine.
#[must_use]
pub fn supersede(original: &Resolution, superseding_id: &str) -> Supersession {
    Supersession {
        superseded: original.id.clone(),
        superseding: superseding_id.to_owned(),
        superseded_standing: original.standing,
    }
}

/// §56.6's record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Supersession {
    pub superseded: String,
    pub superseding: String,
    /// Carried forward unchanged. Replacement is not invalidity.
    pub superseded_standing: ResolutionStanding,
}

/// §57's post-resolution monitor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Monitor {
    pub id: String,
    pub metric_ref: String,
    pub trigger_condition: String,
    pub action: MonitorAction,
}

impl Monitor {
    pub fn validate(&self) -> Result<(), ResolutionError> {
        for (field, value) in [
            ("metric_ref", &self.metric_ref),
            ("trigger_condition", &self.trigger_condition),
        ] {
            if value.trim().is_empty() {
                return Err(ResolutionError::MonitorIncomplete {
                    id: self.id.clone(),
                    field,
                });
            }
        }
        Ok(())
    }

    /// §57: *"Monitoring is distinct from the original completion proof unless
    /// the contract explicitly includes it."*
    ///
    /// A monitor that has not fired is not evidence that the deliverable is
    /// correct, and a green dashboard is the easiest thing in this system to
    /// mistake for a proof.
    #[must_use]
    pub const fn contributes_to_completion_proof(contract_includes_monitoring: bool) -> bool {
        contract_includes_monitoring
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn resolution(outcome: CommonOutcome) -> Resolution {
        Resolution {
            id: "RES-001".into(),
            common_outcome: outcome,
            profile_outcome: "delivered".into(),
            contract_revision: 3,
            contract_digest: "sha256:c".into(),
            assurance_case_snapshot_digest: "sha256:a".into(),
            artifact_manifest_digest: "sha256:m".into(),
            gate_run_refs: vec!["GR-1".into()],
            judgment_refs: vec![],
            residual_risk_refs: vec![],
            resolved_by_ref: "person://quitetall".into(),
            acting_role_ref: "role-assignment://resolver".into(),
            meaning: "Accept the declared deliverables against the bounded obligations.".into(),
            effective_at: "2026-08-20T00:00:00Z".into(),
            recorded_at: "server-assigned".into(),
            standing: ResolutionStanding::Valid,
        }
    }

    /// §56.1's thirteen, transcribed in order.
    #[test]
    fn the_resolution_requirements_match_the_sas() {
        assert_eq!(RESOLUTION_REQUIREMENTS.len(), 13);
        assert_eq!(
            RESOLUTION_REQUIREMENTS[0],
            "exact authorized Contract Revision"
        );
        assert_eq!(RESOLUTION_REQUIREMENTS[12], "resolver holds the role");
        assert_eq!(
            ResolutionChecks::all_met().as_pairs().len(),
            RESOLUTION_REQUIREMENTS.len(),
            "a requirement was added to the list but not to the checks"
        );
        assert!(ResolutionChecks::all_met().unmet().is_empty());
    }

    /// Every one of the thirteen must be able to block on its own. A conjunction
    /// where one term does nothing is a conjunction with a hole in it.
    #[test]
    fn each_of_the_thirteen_blocks_resolution_on_its_own() {
        type Unset = (&'static str, fn(&mut ResolutionChecks));
        let setters: [Unset; 13] = [
            ("exact authorized Contract Revision", |c| {
                c.exact_authorized_contract_revision = false;
            }),
            ("required deliverables exist", |c| {
                c.required_deliverables_exist = false;
            }),
            ("artifact digests verify", |c| {
                c.artifact_digests_verify = false;
            }),
            ("every required obligation is dispositioned", |c| {
                c.every_required_obligation_dispositioned = false;
            }),
            ("every required gate has admissible result", |c| {
                c.every_required_gate_has_admissible_result = false;
            }),
            ("no required unknown remains", |c| {
                c.no_required_unknown_remains = false;
            }),
            ("no blocker remains", |c| c.no_blocker_remains = false),
            ("deviations are dispositioned", |c| {
                c.deviations_dispositioned = false;
            }),
            ("required judgments exist", |c| {
                c.required_judgments_exist = false;
            }),
            ("independence requirements are met", |c| {
                c.independence_requirements_met = false;
            }),
            ("residual risks have sufficient authority", |c| {
                c.residual_risks_have_sufficient_authority = false;
            }),
            ("runtime receipts match the basis", |c| {
                c.runtime_receipts_match_the_basis = false;
            }),
            ("resolver holds the role", |c| {
                c.resolver_holds_the_role = false;
            }),
        ];
        for (name, unset) in setters {
            let mut checks = ResolutionChecks::all_met();
            unset(&mut checks);
            assert_eq!(checks.unmet(), vec![name], "{name} did not block alone");

            let err = resolution(CommonOutcome::Satisfied)
                .validate(checks, false)
                .unwrap_err();
            assert!(err.to_string().contains(name), "{name} unnamed in: {err}");
        }
    }

    #[test]
    fn a_fully_verified_resolution_records() {
        assert_eq!(
            resolution(CommonOutcome::Satisfied).validate(ResolutionChecks::all_met(), false),
            Ok(())
        );
    }

    /// §56.3 — the distinction that stops a miss reading as a finding.
    #[test]
    fn a_failed_delivery_cannot_be_dressed_up_as_falsified() {
        let r = resolution(CommonOutcome::Falsified);
        let err = r.validate(ResolutionChecks::all_met(), false).unwrap_err();
        assert!(
            matches!(
                err,
                ResolutionError::FalsifiedWithoutFalsifiableClaim { .. }
            ),
            "{err}"
        );

        // With a genuine falsifiable claim, it is the right word.
        assert_eq!(r.validate(ResolutionChecks::all_met(), true), Ok(()));

        // And the ordinary outcomes need no such claim.
        for outcome in [
            CommonOutcome::NotSatisfied,
            CommonOutcome::Cancelled,
            CommonOutcome::Blocked,
        ] {
            assert_eq!(
                resolution(outcome).validate(ResolutionChecks::all_met(), false),
                Ok(()),
                "{outcome} should not require a falsifiable claim"
            );
        }
    }

    /// §56.2's `meaning` is what stops "resolved" being read as "correct".
    #[test]
    fn a_resolution_must_state_what_it_means() {
        let mut r = resolution(CommonOutcome::Satisfied);
        r.meaning.clear();
        match r.validate(ResolutionChecks::all_met(), false) {
            Err(ResolutionError::RecordIncomplete { field, .. }) => {
                assert_eq!(field, "meaning");
            }
            other => panic!("a resolution with no stated meaning was accepted: {other:?}"),
        }
    }

    // ---- §56.4, §56.5, §56.6 ---------------------------------------------

    /// §56.5 — annulment does not delete. The original is untouched.
    #[test]
    fn annulment_leaves_the_original_historical() {
        let original = resolution(CommonOutcome::Satisfied);
        let before = original.clone();

        let annulled = annul(&original);
        assert_eq!(annulled.standing, ResolutionStanding::Annulled);
        assert!(!annulled.is_reliable());
        assert!(matches!(
            annulled.require_reliable(),
            Err(ResolutionError::NotReliable { .. })
        ));

        assert_eq!(original, before, "annulment mutated the original");
        assert!(original.is_reliable(), "the historical record still stands");
        // Everything except standing is carried forward.
        assert_eq!(annulled.contract_digest, original.contract_digest);
        assert_eq!(annulled.gate_run_refs, original.gate_run_refs);
    }

    /// §56.6 — replacement is not invalidity. This is the distinction most
    /// likely to be flattened, because both feel like "no longer current".
    #[test]
    fn supersession_does_not_invalidate_the_superseded() {
        let original = resolution(CommonOutcome::Satisfied);
        let s = supersede(&original, "RES-002");

        assert_eq!(s.superseded, "RES-001");
        assert_eq!(s.superseding, "RES-002");
        assert_eq!(
            s.superseded_standing,
            ResolutionStanding::Valid,
            "supersession marked the superseded resolution invalid"
        );
        assert!(
            original.is_reliable(),
            "a superseded resolution was and remains correct for its contract"
        );
    }

    #[test]
    fn a_dispute_moves_standing_without_touching_the_original() {
        let original = resolution(CommonOutcome::Satisfied);
        let disputed = dispute(&original);
        assert_eq!(disputed.standing, ResolutionStanding::Disputed);
        assert!(!disputed.is_reliable());
        assert!(original.is_reliable());
    }

    /// §56.4's six fields.
    #[test]
    fn a_dispute_must_state_all_six_things() {
        let full = Dispute {
            id: "DIS-001".into(),
            challenged_resolution: "RES-001".into(),
            grounds: "the gate that produced GR-1 was invalidated".into(),
            affected_evidence_or_judgment: vec!["GR-1".into()],
            reliance_policy: "do not rely pending re-verification".into(),
            owner: "QuiteTall".into(),
            required_re_verification: "re-run under gate v1.0.1".into(),
        };
        assert_eq!(full.validate(), Ok(()));

        type Blank = (&'static str, fn(&mut Dispute));
        let blanks: [Blank; 6] = [
            ("challenged_resolution", |d| d.challenged_resolution.clear()),
            ("grounds", |d| d.grounds.clear()),
            ("reliance_policy", |d| d.reliance_policy.clear()),
            ("owner", |d| d.owner.clear()),
            ("required_re_verification", |d| {
                d.required_re_verification.clear();
            }),
            ("affected_evidence_or_judgment", |d| {
                d.affected_evidence_or_judgment.clear();
            }),
        ];
        for (name, blank) in blanks {
            let mut d = full.clone();
            blank(&mut d);
            match d.validate() {
                Err(ResolutionError::DisputeIncomplete { field, .. }) => {
                    assert_eq!(field, name);
                }
                other => panic!("a dispute without {name} was accepted: {other:?}"),
            }
        }
    }

    // ---- §57 -------------------------------------------------------------

    /// §57's six trigger actions, transcribed.
    #[test]
    fn the_monitor_actions_match_the_sas() {
        assert_eq!(
            MonitorAction::ALL
                .iter()
                .map(|a| a.as_str())
                .collect::<Vec<_>>(),
            [
                "dispute_resolution",
                "open_investigation",
                "open_remediation",
                "propose_adr",
                "open_superseding_war",
                "require_re_verification",
            ]
        );
    }

    /// §57 — a green monitor is not a completion proof unless the contract said
    /// it was. This is the easiest thing here to mistake for evidence.
    #[test]
    fn monitoring_is_not_a_completion_proof_by_default() {
        assert!(!Monitor::contributes_to_completion_proof(false));
        assert!(Monitor::contributes_to_completion_proof(true));
    }

    #[test]
    fn a_monitor_needs_a_metric_and_a_condition() {
        let m = Monitor {
            id: "MON-001".into(),
            metric_ref: "telemetry://latency-p99".into(),
            trigger_condition: "p99 > 250ms for 3 consecutive windows".into(),
            action: MonitorAction::OpenSupersedingWar,
        };
        assert_eq!(m.validate(), Ok(()));

        let mut bare = m.clone();
        bare.trigger_condition.clear();
        match bare.validate() {
            Err(ResolutionError::MonitorIncomplete { field, .. }) => {
                assert_eq!(field, "trigger_condition");
            }
            other => panic!("a monitor with no trigger was accepted: {other:?}"),
        }
    }

    #[test]
    fn vocabularies_round_trip() {
        for &o in CommonOutcome::ALL {
            assert_eq!(CommonOutcome::from_str(o.as_str()), Ok(o));
        }
        for &a in MonitorAction::ALL {
            assert_eq!(MonitorAction::from_str(a.as_str()), Ok(a));
        }
        assert!(CommonOutcome::from_str("done").is_err());
    }

    #[test]
    fn a_resolution_round_trips_through_json() {
        let r = resolution(CommonOutcome::Satisfied);
        let s = serde_json::to_string(&r).expect("serialize");
        assert_eq!(
            serde_json::from_str::<Resolution>(&s).expect("deserialize"),
            r
        );
    }
}
