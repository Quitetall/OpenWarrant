// SPDX-License-Identifier: AGPL-3.0-or-later
//! Epistemic classes, evidence, and judgment (SAS §40, §41, §42; RQ-052, RQ-053).
//!
//! This is the distinction the specification exists to protect.
//!
//! §40 separates six kinds — claim, evidence item, observation, inference,
//! judgment, resolution — and §40.7 lists six substitutions the validator SHALL
//! REJECT. The most important is the first: a **performer assertion is not an
//! independent observation**. A thing that did the work saying it worked is not
//! evidence that it worked.
//!
//! # Unrepresentable where possible, refused where not
//!
//! The six classes are separate types with no `From` conversions between them,
//! so most substitutions cannot be written at all — you cannot pass a
//! [`Claim`] where an [`Observation`] is wanted. The substitutions that survive
//! typing are the ones about the *provenance* of a value rather than its shape:
//! an [`Observation`] really can be built from a performer's report, and only
//! [`Admissibility`] distinguishes it. Those are refused at validation, each by
//! its own named rule, because §40.7 lists six distinct substitutions and a
//! single "invalid evidence" error would prove only that something was rejected.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EpistemicError {
    #[error("unknown {kind} {found:?}; expected one of {known}")]
    Unknown {
        kind: &'static str,
        found: String,
        known: String,
    },
    #[error("prohibited substitution (§40.7): {substitution}. {detail}")]
    ProhibitedSubstitution {
        substitution: &'static str,
        detail: String,
    },
    #[error(
        "evidence {id:?} records no collection method; an observation must be method-bound (§40.3)"
    )]
    MethodUnbound { id: String },
    #[error("evidence {id:?} records no content digest; immutable bytes need an identity (§41.3)")]
    MissingDigest { id: String },
    #[error(
        "judgment {id:?} states no meaning; an approval with no stated meaning is invalid (§42)"
    )]
    JudgmentWithoutMeaning { id: String },
    #[error("judgment {id:?} names no acting role; a judgment is attributable (§40.5, §42)")]
    JudgmentWithoutRole { id: String },
    #[error(
        "judgment {id:?} was recommended by an agent and has not been authorized \
         through policy and an exercised role (§42)"
    )]
    UnauthorizedAgentJudgment { id: String },
}

macro_rules! vocabulary {
    ($name:ident, $label:literal, { $($variant:ident => $text:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }
        }

        impl FromStr for $name {
            type Err = EpistemicError;
            fn from_str(s: &str) -> Result<Self, EpistemicError> {
                Self::ALL.iter().copied().find(|v| v.as_str() == s).ok_or_else(|| {
                    EpistemicError::Unknown {
                        kind: $label,
                        found: s.to_owned(),
                        known: Self::ALL.iter().map(|v| v.as_str()).collect::<Vec<_>>().join(", "),
                    }
                })
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

vocabulary!(InferenceKind, "inference kind", {
    Deductive => "deductive",
    Statistical => "statistical",
    Causal => "causal",
    Heuristic => "heuristic",
    Formal => "formal",
});

vocabulary!(EvidenceOrigin, "evidence origin", {
    Performer => "performer",
    Verifier => "verifier",
    GateRunner => "gate_runner",
    ExternalAuthority => "external_authority",
    Instrument => "instrument",
    KnowledgeFabric => "knowledge_fabric",
    Katana => "katana",
    Blut => "blut",
    Liminal => "liminal",
    HumanReviewer => "human_reviewer",
});

impl EvidenceOrigin {
    /// Whether evidence from this origin can be independent of the work.
    ///
    /// A performer is never independent of its own output — that is §40.7's
    /// first prohibited substitution, stated as a property of the origin.
    #[must_use]
    pub const fn can_be_independent(self) -> bool {
        match self {
            Self::Performer => false,
            Self::Verifier
            | Self::GateRunner
            | Self::ExternalAuthority
            | Self::Instrument
            | Self::KnowledgeFabric
            | Self::Katana
            | Self::Blut
            | Self::Liminal
            | Self::HumanReviewer => true,
        }
    }
}

vocabulary!(Admissibility, "admissibility", {
    Informative => "informative",
    PerformerReportOnly => "performer_report_only",
    Independent => "independent",
    AuthoritativeExternal => "authoritative_external",
    ControlledMeasurement => "controlled_measurement",
    Formal => "formal",
    Inadmissible => "inadmissible",
});

impl Admissibility {
    /// Whether this level may support an INDEPENDENT claim (RQ-053).
    #[must_use]
    pub const fn is_independent(self) -> bool {
        match self {
            Self::Independent
            | Self::AuthoritativeExternal
            | Self::ControlledMeasurement
            | Self::Formal => true,
            Self::Informative | Self::PerformerReportOnly | Self::Inadmissible => false,
        }
    }
}

/// §40.1 — a proposition requiring support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub statement: String,
    /// The bound on the claim (§38.4). A claim is only as wide as its evidence.
    pub scope: String,
}

/// §40.2 — immutable bytes or an authoritative record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub id: String,
    pub kind: String,
    pub origin: EvidenceOrigin,
    pub admissibility: Admissibility,
    /// §41.3 — immutable bytes need an identity.
    pub content_digest: Option<String>,
    /// §41.3 — how it was collected. An observation over evidence with no
    /// collection method is not method-bound (§40.3).
    pub collection_method: Option<String>,
    /// §41.4 — supplied by the actor; `recorded_at` is the receiving service's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occurred_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recorded_at: Option<String>,
}

impl EvidenceItem {
    pub fn validate(&self) -> Result<(), EpistemicError> {
        if self
            .content_digest
            .as_ref()
            .is_none_or(|d| d.trim().is_empty())
        {
            return Err(EpistemicError::MissingDigest {
                id: self.id.clone(),
            });
        }
        // §40.7 #1 — a performer's own output cannot be admitted as independent.
        if !self.origin.can_be_independent() && self.admissibility.is_independent() {
            return Err(EpistemicError::ProhibitedSubstitution {
                substitution: "performer assertion → independent observation",
                detail: format!(
                    "evidence {:?} originates from the performer but is admitted as {}. \
                     A performer report is admissible only as `performer_report_only` \
                     or `informative`",
                    self.id, self.admissibility
                ),
            });
        }
        Ok(())
    }

    /// §40.7 #2 — a generated report is not raw evidence.
    pub fn reject_report_as_raw(&self, is_derived_report: bool) -> Result<(), EpistemicError> {
        if is_derived_report && self.admissibility != Admissibility::Informative {
            return Err(EpistemicError::ProhibitedSubstitution {
                substitution: "generated report → raw evidence",
                detail: format!(
                    "evidence {:?} is a derived report; it describes evidence rather \
                     than being it (§37.3)",
                    self.id
                ),
            });
        }
        Ok(())
    }
}

/// §40.3 — a method-bound statement about evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub id: String,
    pub statement: String,
    /// The evidence this observes. An observation with no evidence is an
    /// assertion.
    pub evidence_refs: Vec<String>,
    /// §40.3 — the method that bound it.
    pub method: String,
    pub admissibility: Admissibility,
}

impl Observation {
    pub fn validate(&self) -> Result<(), EpistemicError> {
        if self.method.trim().is_empty() {
            return Err(EpistemicError::MethodUnbound {
                id: self.id.clone(),
            });
        }
        Ok(())
    }

    /// §40.7 #3 — a test pass is not a universal coverage claim.
    ///
    /// §38.4 already says sampling cannot support a universal claim; this is the
    /// same rule at the observation boundary, where the temptation actually
    /// lives: "all 142 tests passed" is a true observation that says nothing
    /// about the inputs nobody tested.
    pub fn reject_as_universal(&self, claim: &Claim) -> Result<(), EpistemicError> {
        let claim_is_universal = {
            let s = claim.scope.to_lowercase();
            s.contains("universal") || s.contains("every possible") || s.contains("all inputs")
        };
        if claim_is_universal {
            return Err(EpistemicError::ProhibitedSubstitution {
                substitution: "test pass → universal coverage claim",
                detail: format!(
                    "observation {:?} supports claim {:?}, whose scope is universal. \
                     A passing test set bounds the claim to what was tested",
                    self.id, claim.id
                ),
            });
        }
        Ok(())
    }
}

/// §40.4 — a reasoning step from premises or observations to a claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inference {
    pub id: String,
    pub kind: InferenceKind,
    pub statement: String,
    pub premise_refs: Vec<String>,
    pub claim_ref: String,
}

/// Who is making a judgment (§42).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JudgmentAuthority {
    /// §42 — an agent may RECOMMEND a judgment.
    AgentRecommendation,
    /// Authorized through policy and an exercised role.
    Authorized,
}

/// §40.5 / §42 — an attributable evaluative or policy choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Judgment {
    pub id: String,
    pub kind: String,
    pub statement: String,
    pub actor: String,
    /// §42 — the role actually exercised.
    pub acting_role: String,
    /// §42 — "An approval with no stated meaning is invalid."
    pub meaning: String,
    pub basis_refs: Vec<String>,
    pub authority: JudgmentAuthority,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limitations: Vec<String>,
}

impl Judgment {
    pub fn validate(&self) -> Result<(), EpistemicError> {
        if self.meaning.trim().is_empty() {
            return Err(EpistemicError::JudgmentWithoutMeaning {
                id: self.id.clone(),
            });
        }
        if self.acting_role.trim().is_empty() {
            return Err(EpistemicError::JudgmentWithoutRole {
                id: self.id.clone(),
            });
        }
        Ok(())
    }

    /// §42 — an agent recommendation becomes authorized only through policy and
    /// an exercised role.
    pub fn require_authorized(&self) -> Result<(), EpistemicError> {
        match self.authority {
            JudgmentAuthority::Authorized => Ok(()),
            JudgmentAuthority::AgentRecommendation => {
                Err(EpistemicError::UnauthorizedAgentJudgment {
                    id: self.id.clone(),
                })
            }
        }
    }

    /// §40.7 #4 — model confidence is not an authorized judgment.
    pub fn reject_model_confidence(&self) -> Result<(), EpistemicError> {
        let s = self.statement.to_lowercase();
        let smells_of_confidence = ["confidence", "probability", "likelihood", "score"]
            .iter()
            .any(|w| s.contains(w));
        if smells_of_confidence && self.authority == JudgmentAuthority::AgentRecommendation {
            return Err(EpistemicError::ProhibitedSubstitution {
                substitution: "model confidence → authorized judgment",
                detail: format!(
                    "judgment {:?} expresses a model's confidence and is not authorized \
                     through an exercised role",
                    self.id
                ),
            });
        }
        Ok(())
    }
}

/// §40.6 — the organizational adjudication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Adjudication {
    pub id: String,
    pub statement: String,
    /// The obligations this adjudication rests on.
    pub obligation_refs: Vec<String>,
}

impl Adjudication {
    /// §40.7 #5 — a gate verdict is not a resolution.
    ///
    /// A gate says what a check returned. A resolution says what the
    /// organization concluded. Treating the first as the second removes the
    /// organization from its own decision.
    pub fn reject_gate_verdict_as_resolution(gate_run_id: &str) -> EpistemicError {
        EpistemicError::ProhibitedSubstitution {
            substitution: "gate verdict → resolution",
            detail: format!(
                "gate run {gate_run_id:?} produced a verdict; a verdict is an input to \
                 adjudication, not the adjudication"
            ),
        }
    }

    /// §40.7 #6 — a resolution is not an empirical observation.
    ///
    /// The organization concluding something does not make it measured.
    pub fn reject_as_observation(&self) -> EpistemicError {
        EpistemicError::ProhibitedSubstitution {
            substitution: "resolution → empirical observation",
            detail: format!(
                "resolution {:?} is an organizational adjudication; citing it as an \
                 observation would make a decision into a measurement",
                self.id
            ),
        }
    }
}

/// The six substitutions §40.7 requires the validator to reject.
///
/// Enumerated so a test can assert all six are covered — a list that silently
/// shrinks is how a prohibition stops being enforced.
pub const PROHIBITED_SUBSTITUTIONS: [&str; 6] = [
    "performer assertion → independent observation",
    "generated report → raw evidence",
    "test pass → universal coverage claim",
    "model confidence → authorized judgment",
    "gate verdict → resolution",
    "resolution → empirical observation",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(origin: EvidenceOrigin, adm: Admissibility) -> EvidenceItem {
        EvidenceItem {
            id: "EVD-001".to_owned(),
            kind: "gate_output".to_owned(),
            origin,
            admissibility: adm,
            content_digest: Some("a".repeat(64)),
            collection_method: Some("gate://x".to_owned()),
            occurred_at: None,
            recorded_at: None,
        }
    }

    fn judgment(authority: JudgmentAuthority, statement: &str) -> Judgment {
        Judgment {
            id: "JUD-001".to_owned(),
            kind: "adequacy".to_owned(),
            statement: statement.to_owned(),
            actor: "brian".to_owned(),
            acting_role: "maintainer".to_owned(),
            meaning: "the gate set is adequate for alpha".to_owned(),
            basis_refs: vec!["observation://OBS-001".to_owned()],
            authority,
            limitations: vec![],
        }
    }

    /// §40.7 lists six. If this list shrinks, a prohibition stopped being
    /// enforced without anyone deciding to stop enforcing it.
    #[test]
    fn all_six_prohibited_substitutions_are_enumerated() {
        assert_eq!(PROHIBITED_SUBSTITUTIONS.len(), 6);
        assert_eq!(
            PROHIBITED_SUBSTITUTIONS,
            [
                "performer assertion → independent observation",
                "generated report → raw evidence",
                "test pass → universal coverage claim",
                "model confidence → authorized judgment",
                "gate verdict → resolution",
                "resolution → empirical observation",
            ]
        );
    }

    /// §40.7 #1 — THE most important rule in the system.
    #[test]
    fn a_performer_report_cannot_be_admitted_as_independent() {
        for adm in [
            Admissibility::Independent,
            Admissibility::AuthoritativeExternal,
            Admissibility::ControlledMeasurement,
            Admissibility::Formal,
        ] {
            let e = evidence(EvidenceOrigin::Performer, adm);
            match e.validate() {
                Err(EpistemicError::ProhibitedSubstitution { substitution, .. }) => {
                    assert_eq!(substitution, PROHIBITED_SUBSTITUTIONS[0]);
                }
                other => panic!("performer + {adm} must be refused, got {other:?}"),
            }
        }
    }

    #[test]
    fn a_performer_report_is_admissible_as_what_it_is() {
        for adm in [
            Admissibility::PerformerReportOnly,
            Admissibility::Informative,
        ] {
            evidence(EvidenceOrigin::Performer, adm)
                .validate()
                .expect("a performer report is fine when labelled as one");
        }
    }

    #[test]
    fn a_verifier_may_be_independent() {
        evidence(EvidenceOrigin::Verifier, Admissibility::Independent)
            .validate()
            .expect("a verifier is not the performer");
        assert!(!EvidenceOrigin::Performer.can_be_independent());
        assert!(EvidenceOrigin::GateRunner.can_be_independent());
    }

    /// §40.7 #2
    #[test]
    fn a_generated_report_is_not_raw_evidence() {
        let e = evidence(EvidenceOrigin::Verifier, Admissibility::Independent);
        match e.reject_report_as_raw(true) {
            Err(EpistemicError::ProhibitedSubstitution { substitution, .. }) => {
                assert_eq!(substitution, PROHIBITED_SUBSTITUTIONS[1]);
            }
            other => panic!("got {other:?}"),
        }
        e.reject_report_as_raw(false).expect("raw evidence is fine");
    }

    /// §40.7 #3
    #[test]
    fn a_test_pass_is_not_a_universal_coverage_claim() {
        let obs = Observation {
            id: "OBS-001".to_owned(),
            statement: "the verifier selected 142 tests and all 142 passed".to_owned(),
            evidence_refs: vec!["EVD-001".to_owned()],
            method: "cargo test".to_owned(),
            admissibility: Admissibility::Independent,
        };
        let universal = Claim {
            id: "CLM-001".to_owned(),
            statement: "the parser is correct".to_owned(),
            scope: "universal over all inputs".to_owned(),
        };
        match obs.reject_as_universal(&universal) {
            Err(EpistemicError::ProhibitedSubstitution { substitution, .. }) => {
                assert_eq!(substitution, PROHIBITED_SUBSTITUTIONS[2]);
            }
            other => panic!("got {other:?}"),
        }

        let bounded = Claim {
            id: "CLM-002".to_owned(),
            statement: "the corpus parses".to_owned(),
            scope: "the 40 Warrants under docs/warrants".to_owned(),
        };
        obs.reject_as_universal(&bounded)
            .expect("a bounded claim is supportable by a test run");
    }

    /// §40.7 #4
    #[test]
    fn model_confidence_is_not_an_authorized_judgment() {
        let j = judgment(
            JudgmentAuthority::AgentRecommendation,
            "confidence 0.94 that the residual risk is acceptable",
        );
        match j.reject_model_confidence() {
            Err(EpistemicError::ProhibitedSubstitution { substitution, .. }) => {
                assert_eq!(substitution, PROHIBITED_SUBSTITUTIONS[3]);
            }
            other => panic!("got {other:?}"),
        }
        // The same statement, authorized through an exercised role, is a judgment.
        judgment(
            JudgmentAuthority::Authorized,
            "confidence 0.94 that the residual risk is acceptable",
        )
        .reject_model_confidence()
        .expect("authorization is what makes it a judgment");
    }

    /// §40.7 #5 and #6
    #[test]
    fn gate_verdicts_and_resolutions_do_not_substitute_for_each_other() {
        let err = Adjudication::reject_gate_verdict_as_resolution("GR-001");
        match err {
            EpistemicError::ProhibitedSubstitution { substitution, .. } => {
                assert_eq!(substitution, PROHIBITED_SUBSTITUTIONS[4]);
            }
            other => panic!("got {other:?}"),
        }

        let adj = Adjudication {
            id: "RES-001".to_owned(),
            statement: "satisfied".to_owned(),
            obligation_refs: vec!["OBL-001".to_owned()],
        };
        match adj.reject_as_observation() {
            EpistemicError::ProhibitedSubstitution { substitution, .. } => {
                assert_eq!(substitution, PROHIBITED_SUBSTITUTIONS[5]);
            }
            other => panic!("got {other:?}"),
        }
    }

    /// §42 — "An approval with no stated meaning is invalid."
    #[test]
    fn a_judgment_without_meaning_or_role_is_invalid() {
        let mut j = judgment(JudgmentAuthority::Authorized, "adequate");
        j.meaning = "  ".to_owned();
        assert_eq!(
            j.validate(),
            Err(EpistemicError::JudgmentWithoutMeaning {
                id: "JUD-001".to_owned()
            })
        );

        let mut j = judgment(JudgmentAuthority::Authorized, "adequate");
        j.acting_role = String::new();
        assert_eq!(
            j.validate(),
            Err(EpistemicError::JudgmentWithoutRole {
                id: "JUD-001".to_owned()
            })
        );
    }

    /// §42 — an agent may RECOMMEND; authorization is a separate act.
    #[test]
    fn an_agent_recommendation_is_not_an_authorized_judgment() {
        assert_eq!(
            judgment(JudgmentAuthority::AgentRecommendation, "adequate").require_authorized(),
            Err(EpistemicError::UnauthorizedAgentJudgment {
                id: "JUD-001".to_owned()
            })
        );
        judgment(JudgmentAuthority::Authorized, "adequate")
            .require_authorized()
            .expect("authorized");
    }

    /// §40.3 — an observation is METHOD-BOUND. Without a method it is an
    /// assertion wearing an observation's name.
    #[test]
    fn an_observation_without_a_method_is_refused() {
        let obs = Observation {
            id: "OBS-001".to_owned(),
            statement: "it works".to_owned(),
            evidence_refs: vec![],
            method: String::new(),
            admissibility: Admissibility::Informative,
        };
        assert_eq!(
            obs.validate(),
            Err(EpistemicError::MethodUnbound {
                id: "OBS-001".to_owned()
            })
        );
    }

    #[test]
    fn evidence_without_a_digest_is_refused() {
        let mut e = evidence(EvidenceOrigin::Verifier, Admissibility::Independent);
        e.content_digest = None;
        assert_eq!(
            e.validate(),
            Err(EpistemicError::MissingDigest {
                id: "EVD-001".to_owned()
            })
        );
    }

    /// §41.1 and §41.2 vocabularies, transcribed as external expectations.
    #[test]
    fn vocabularies_match_the_sas() {
        assert_eq!(
            EvidenceOrigin::ALL
                .iter()
                .map(|o| o.as_str())
                .collect::<Vec<_>>(),
            [
                "performer",
                "verifier",
                "gate_runner",
                "external_authority",
                "instrument",
                "knowledge_fabric",
                "katana",
                "blut",
                "liminal",
                "human_reviewer",
            ]
        );
        assert_eq!(
            Admissibility::ALL
                .iter()
                .map(|a| a.as_str())
                .collect::<Vec<_>>(),
            [
                "informative",
                "performer_report_only",
                "independent",
                "authoritative_external",
                "controlled_measurement",
                "formal",
                "inadmissible",
            ]
        );
        assert_eq!(
            InferenceKind::ALL
                .iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>(),
            ["deductive", "statistical", "causal", "heuristic", "formal"]
        );
    }

    /// The six classes are separate TYPES, and that separation is enforced by
    /// the COMPILER, not by this test.
    ///
    /// There is no runtime assertion that can prove the absence of a `From`
    /// impl. What this pins instead is the property that would be lost if
    /// someone added one: an [`Observation`] carries a method and an
    /// admissibility that a [`Claim`] has no field for, so collapsing the two
    /// would have to DROP data — which a reviewer can see in the diff.
    ///
    /// Said plainly rather than dressed up as a stronger check: if a future
    /// change adds `impl From<Claim> for Observation`, this test still passes.
    /// The defence is that such an impl cannot be written without inventing a
    /// method and an admissibility out of nothing.
    #[test]
    fn observations_carry_what_claims_cannot() {
        let obs = Observation {
            id: "OBS-001".to_owned(),
            statement: "142 tests passed".to_owned(),
            evidence_refs: vec!["EVD-001".to_owned()],
            method: "cargo test".to_owned(),
            admissibility: Admissibility::Independent,
        };
        // A claim has no method and no admissibility. Turning one into the other
        // means fabricating both.
        assert!(!obs.method.is_empty());
        assert!(obs.admissibility.is_independent());
        assert!(!obs.evidence_refs.is_empty());
    }
}
