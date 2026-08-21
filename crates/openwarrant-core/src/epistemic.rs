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
    #[error(
        "record {id:?} declares class {class:?}, which is not one of §40's record \
         kinds: evidence, observation, inference, judgment"
    )]
    UnknownRecordClass { id: String, class: String },
    /// §42's authority, wrong or missing.
    ///
    /// This has its own variant because it used to borrow
    /// [`Self::UnknownRecordClass`], packing an explanation into the `class`
    /// field. The resulting message told the author their *class* was not one
    /// of §40's four — while the class was fine and the authority was the
    /// problem — and pointed at the wrong line of their record. A diagnostic
    /// that misnames the failing field is worse than a vague one: it spends the
    /// reader's attention on the part that was already correct.
    #[error(
        "judgment {id:?} declares authority {authority:?}. §42 defines two: \
         agent_recommendation, authorized. An empty value usually means the \
         `- **authority:**` bullet is missing"
    )]
    UnknownJudgmentAuthority { id: String, authority: String },
    #[error(
        "evidence {id:?} supplies its own `recorded at`. §40.2 — recorded_at is \
         assigned by the authority that received the record, never by its author. \
         Refused rather than ignored, so the author cannot believe it was accepted"
    )]
    AuthorSuppliedRecordedAt { id: String },
    #[error(
        "inference {id:?} rests on premise {premise:?}, which is not a declared \
         record. An inference resting on a premise nobody wrote is the defect \
         OW-WAR-0016 caught for milestones citing obligations nobody wrote"
    )]
    DanglingPremise { id: String, premise: String },
    #[error("judgment {id:?} cites basis {basis:?}, which is not a declared record")]
    DanglingBasis { id: String, basis: String },
    #[error(
        "inference {id:?} declares no premises. §40.4 — a reasoning step from \
         premises to a claim that names no premise has not reasoned from anything"
    )]
    InferenceWithoutPremises { id: String },
    #[error("inference {id:?} names no claim it supports (§40.4)")]
    InferenceWithoutClaim { id: String },
    #[error(
        "Circular claim/evidence graph — {detail}. Two records that cite each \
         other both RESOLVE and together support nothing: each rests only on the \
         other. §36.4 requires the graph to be acyclic"
    )]
    CircularEvidence { detail: String },
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

impl Inference {
    /// §40.4 / §91.11 test 78 — an inference with no premises is an assertion.
    ///
    /// This validation did not exist: `Inference` was a struct with no rules, so
    /// "inference with no premises fails" had nothing to fail it. A reasoning
    /// step whose premises are empty has not reasoned from anything.
    pub fn validate(&self) -> Result<(), EpistemicError> {
        if self.premise_refs.is_empty() {
            return Err(EpistemicError::InferenceWithoutPremises {
                id: self.id.clone(),
            });
        }
        if self.claim_ref.trim().is_empty() {
            return Err(EpistemicError::InferenceWithoutClaim {
                id: self.id.clone(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod judgment_authority_tests {
    use super::*;

    /// The failing field must be named correctly. This previously reported a
    /// bad authority through the record-CLASS error, so an author with a
    /// missing `- **authority:**` bullet was told their `- **class:**` was not
    /// one of §40's four — sending them to a line that was already right.
    #[test]
    fn a_bad_authority_is_not_reported_as_a_bad_class() {
        let err = JudgmentAuthority::parse("").expect_err("empty is not an authority");
        let msg = err.to_string();
        assert!(msg.contains("authority"), "{msg}");
        assert!(
            !msg.contains("record class") && !msg.contains("§40's record"),
            "a bad authority must not be reported as a bad class: {msg}"
        );
        assert!(
            msg.contains("bullet is missing"),
            "an empty value should say what is actually missing: {msg}"
        );
    }

    #[test]
    fn both_of_42s_authorities_parse() {
        assert!(JudgmentAuthority::parse("authorized").is_ok());
        assert!(JudgmentAuthority::parse("agent_recommendation").is_ok());
    }
}

impl JudgmentAuthority {
    /// Parse §42's two authorities by name.
    pub fn parse(raw: &str) -> Result<Self, EpistemicError> {
        match raw.trim() {
            "agent_recommendation" => Ok(Self::AgentRecommendation),
            "authorized" => Ok(Self::Authorized),
            other => Err(EpistemicError::UnknownJudgmentAuthority {
                id: "judgment".to_owned(),
                authority: other.to_owned(),
            }),
        }
    }
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

/// Parse §40's evidence records from an assurance atom's `## Evidence` section.
///
/// # Why they live in the assurance atom
///
/// §40's classes exist to keep an acceptance argument honest, and the acceptance
/// argument is the assurance atom. Putting evidence anywhere else would let an
/// obligation cite a record a reader of the obligation cannot see.
///
/// The record shape is the one the corpus already uses for obligations —
/// `### ID — statement` followed by `- **key:** value` bullets — so an author who
/// can write an obligation can write evidence without learning a second format,
/// and the same declared-bullet discipline applies: a channel is a declared
/// field, never prose that happens to contain a word.
pub mod records {
    use super::{
        Admissibility, EpistemicError, EvidenceItem, EvidenceOrigin, Inference, InferenceKind,
        Judgment, JudgmentAuthority, Observation,
    };
    use std::str::FromStr;

    /// Everything §40 lets an assurance atom record.
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct EvidenceSection {
        pub evidence: Vec<EvidenceItem>,
        pub observations: Vec<Observation>,
        pub inferences: Vec<Inference>,
        pub judgments: Vec<Judgment>,
    }

    impl EvidenceSection {
        #[must_use]
        pub fn is_empty(&self) -> bool {
            self.evidence.is_empty()
                && self.observations.is_empty()
                && self.inferences.is_empty()
                && self.judgments.is_empty()
        }

        #[must_use]
        pub fn len(&self) -> usize {
            self.evidence.len()
                + self.observations.len()
                + self.inferences.len()
                + self.judgments.len()
        }

        /// §40.4 — an inference's premises must resolve to records that exist.
        ///
        /// An inference resting on a premise nobody wrote is the same defect as a
        /// milestone citing an obligation nobody wrote, which OW-WAR-0016 caught
        /// in this corpus.
        pub fn validate_references(&self) -> Result<(), EpistemicError> {
            let known: std::collections::BTreeSet<&str> = self
                .evidence
                .iter()
                .map(|e| e.id.as_str())
                .chain(self.observations.iter().map(|o| o.id.as_str()))
                .chain(self.inferences.iter().map(|i| i.id.as_str()))
                .collect();

            for inference in &self.inferences {
                for premise in &inference.premise_refs {
                    if !known.contains(premise.as_str()) {
                        return Err(EpistemicError::DanglingPremise {
                            id: inference.id.clone(),
                            premise: premise.clone(),
                        });
                    }
                }
            }
            for judgment in &self.judgments {
                for basis in &judgment.basis_refs {
                    if !known.contains(basis.as_str()) {
                        return Err(EpistemicError::DanglingBasis {
                            id: judgment.id.clone(),
                            basis: basis.clone(),
                        });
                    }
                }
            }

            // §36.4 / §91.10 test 74 — the claim/evidence graph SHALL be acyclic.
            //
            // Resolving every premise is NOT sufficient, and that is the whole
            // point of this check. Two records that cite each other both resolve
            // and together support nothing: each rests only on the other. A
            // record citing itself resolves too.
            //
            // `ClaimGraph` implemented this detection during alpha and was
            // reached by nothing, so the §36.4 prohibition applied to a graph
            // nobody built.
            let mut graph = crate::rationale::ClaimGraph::default();
            for inference in &self.inferences {
                for premise in &inference.premise_refs {
                    graph.add(&inference.id, premise);
                }
            }
            for judgment in &self.judgments {
                for basis in &judgment.basis_refs {
                    graph.add(&judgment.id, basis);
                }
            }
            graph
                .validate()
                .map_err(|e| EpistemicError::CircularEvidence {
                    detail: e.to_string(),
                })?;

            Ok(())
        }
    }

    fn bullet(line: &str) -> Option<(String, String)> {
        let t = line.trim().strip_prefix("- **")?;
        let (key, rest) = t.split_once(":**")?;
        Some((key.trim().to_lowercase(), rest.trim().to_owned()))
    }

    pub(crate) fn list(value: &str) -> Vec<String> {
        value
            .split(',')
            .map(|s| s.trim().trim_matches('`').to_owned())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Read the `## Evidence` section, if the atom has one.
    ///
    /// An atom without the section yields an empty set, not an error: §40 records
    /// are optional, and the 49 Warrants authored before this existed must keep
    /// parsing unchanged.
    pub fn parse(source: &str) -> Result<EvidenceSection, EpistemicError> {
        let mut out = EvidenceSection::default();
        let mut in_section = false;
        let mut id = String::new();
        let mut statement = String::new();
        let mut fields: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();

        // A record is complete when the next heading or the section ends.
        fn flush(
            out: &mut EvidenceSection,
            id: &str,
            statement: &str,
            fields: &std::collections::BTreeMap<String, String>,
        ) -> Result<(), EpistemicError> {
            if id.is_empty() {
                return Ok(());
            }
            let get = |k: &str| fields.get(k).cloned().unwrap_or_default();
            let class = get("class");

            match class.as_str() {
                "evidence" => {
                    let item = EvidenceItem {
                        id: id.to_owned(),
                        kind: get("kind"),
                        origin: EvidenceOrigin::from_str(&get("origin"))?,
                        admissibility: Admissibility::from_str(&get("admissibility"))?,
                        content_digest: Some(get("digest")).filter(|d| !d.is_empty()),
                        collection_method: Some(get("method")).filter(|m| !m.is_empty()),
                        occurred_at: Some(get("occurred at")).filter(|o| !o.is_empty()),
                        // §40.2 / §91.11 test 81 — `recorded_at` is assigned by the
                        // authority that received the record, never by its author.
                        // An author-supplied value is REFUSED here rather than
                        // ignored, because ignoring it silently would leave the
                        // author believing it was accepted.
                        recorded_at: None,
                    };
                    if fields.contains_key("recorded at") {
                        return Err(EpistemicError::AuthorSuppliedRecordedAt { id: id.to_owned() });
                    }
                    item.validate()?;
                    out.evidence.push(item);
                }
                "observation" => {
                    let o = Observation {
                        id: id.to_owned(),
                        statement: statement.to_owned(),
                        evidence_refs: list(&get("evidence")),
                        method: get("method"),
                        admissibility: Admissibility::from_str(&get("admissibility"))?,
                    };
                    o.validate()?;
                    out.observations.push(o);
                }
                "inference" => {
                    let i = Inference {
                        id: id.to_owned(),
                        kind: InferenceKind::from_str(&get("kind"))?,
                        statement: statement.to_owned(),
                        premise_refs: list(&get("premises")),
                        claim_ref: get("claim"),
                    };
                    i.validate()?;
                    out.inferences.push(i);
                }
                "judgment" => {
                    let j = Judgment {
                        id: id.to_owned(),
                        kind: get("kind"),
                        statement: statement.to_owned(),
                        actor: get("actor"),
                        acting_role: get("acting role"),
                        meaning: get("meaning"),
                        basis_refs: list(&get("basis")),
                        authority: JudgmentAuthority::parse(&get("authority"))?,
                        limitations: super::records::list(&get("limitations")),
                    };
                    j.validate()?;
                    out.judgments.push(j);
                }
                other => {
                    return Err(EpistemicError::UnknownRecordClass {
                        id: id.to_owned(),
                        class: other.to_owned(),
                    });
                }
            }
            Ok(())
        }

        for line in source.lines() {
            let trimmed = line.trim();
            if let Some(heading) = trimmed.strip_prefix("## ") {
                flush(&mut out, &id, &statement, &fields)?;
                id.clear();
                fields.clear();
                in_section = heading.trim().eq_ignore_ascii_case("Evidence");
                continue;
            }
            if !in_section {
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("### ") {
                flush(&mut out, &id, &statement, &fields)?;
                fields.clear();
                let (head, tail) = rest
                    .split_once(" — ")
                    .or_else(|| rest.split_once(" - "))
                    .unwrap_or((rest, ""));
                id = head.trim().to_owned();
                statement = tail.trim().to_owned();
                continue;
            }
            if let Some((k, v)) = bullet(trimmed) {
                fields.insert(k, v);
            }
        }
        flush(&mut out, &id, &statement, &fields)?;
        out.validate_references()?;
        Ok(out)
    }
}
