// SPDX-License-Identifier: AGPL-3.0-or-later
//! Verification records — who checked an obligation, and how independent they were (SAS §46, §38.5).
//!
//! # Why independence moved out of the repository config
//!
//! `openwarrant.toml` carries one `[independence]` block for the whole
//! repository, and `war resolve` reads it to answer §56.1's requirement 10. That
//! is unsound, and the unsoundness is the dangerous direction: a single global
//! block set to `true` makes EVERY Warrant claim independence, including ones
//! nothing independent ever looked at.
//!
//! §46.1's dimensions are properties of a particular verifier examining a
//! particular claim. `separate_writable_workspace` is not a fact about a
//! repository; it is a fact about the actor who produced a verdict. So a
//! verdict carries its own [`Verifier`], and admissibility is decided from that
//! rather than from a project-wide assertion of good intentions.
//!
//! The global block remains meaningful as a DEFAULT for verifications that do
//! not state their own — but a default that claims more independence than a
//! verification actually had is refused by [`Verification::admissible_for`].

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::independence::{Independence, IndependenceError};
use crate::obligation::Disposition;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum VerificationError {
    #[error("verification of {obligation} names no verifier")]
    NoVerifier { obligation: String },
    #[error(
        "verification of {obligation} was produced by {actor:?}, which is also the performer. \
         §51.2 forbids self-completion and RQ-053 forbids a performer's report from \
         satisfying an independent gate."
    )]
    SelfVerification { obligation: String, actor: String },
    #[error(
        "verification of {obligation} by {actor:?} is not independent enough for {assurance} \
         assurance: {source}"
    )]
    InsufficientIndependence {
        obligation: String,
        actor: String,
        assurance: String,
        /// Boxed: five inline `String`s made the `Err` variant large enough that
        /// clippy's `result_large_err` fires, and every caller inspects this
        /// only on the error path where one allocation costs nothing.
        #[source]
        source: Box<IndependenceError>,
    },
    #[error(
        "verification of {obligation} cites no evidence. §40.7 forbids a judgment \
         standing in for the observation it was supposed to rest on."
    )]
    NoEvidence { obligation: String },
}

/// What kind of actor produced a verdict (§27.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    Human,
    Agent,
    Service,
}

impl ActorKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Agent => "agent",
            Self::Service => "service",
        }
    }
}

impl fmt::Display for ActorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Who performed a verification, and the independence they actually had.
///
/// `independence` is recorded AS OBSERVED for this verifier, not copied from a
/// project default. An agent reviewer that never sees the performer's transcript
/// genuinely has `performer_transcript_blind`; asserting it for a verifier that
/// did would be a false record, and this type is where that lie would have to be
/// written down to take effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verifier {
    /// Stable identifier for the actor, e.g. `lamu:mimo-v2.5-pro`.
    pub actor: String,
    pub kind: ActorKind,
    /// The model identity, where the actor is a model. Recorded separately from
    /// `actor` so `distinct_model_required` can be checked against something
    /// more specific than a label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub independence: Independence,
}

impl Verifier {
    /// Whether this verifier is a different actor from `performer`.
    #[must_use]
    pub fn is_distinct_from(&self, performer: &str) -> bool {
        self.actor != performer
    }
}

/// One obligation, verified.
///
/// This is the record §38.5 needs before a disposition means anything, and the
/// record §56.1's requirements 5 and 10 read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verification {
    pub obligation: String,
    pub verifier: Verifier,
    pub disposition: Disposition,
    /// What was examined. A reference to a Gate Run receipt, an artifact digest,
    /// or a named measurement — never a restatement of the claim.
    pub evidence: String,
    /// The performer whose work this verifies, so self-verification is
    /// detectable rather than merely discouraged.
    pub performer: String,
}

impl Verification {
    /// Whether this verification may be admitted for a Warrant at `assurance`.
    ///
    /// Fail-closed in three ways, each of which has a specific failure it
    /// prevents:
    ///
    /// 1. **No evidence** — a disposition with nothing behind it is a judgment
    ///    substituted for an observation (§40.7).
    /// 2. **Self-verification** — the performer grading their own work is the
    ///    single most likely way a false PASS enters this system (§51.2).
    /// 3. **Insufficient independence** — §46.3's minimum for the level.
    pub fn admissible_for(&self, assurance: &str) -> Result<(), VerificationError> {
        if self.verifier.actor.trim().is_empty() {
            return Err(VerificationError::NoVerifier {
                obligation: self.obligation.clone(),
            });
        }
        if self.evidence.trim().is_empty() {
            return Err(VerificationError::NoEvidence {
                obligation: self.obligation.clone(),
            });
        }
        if !self.verifier.is_distinct_from(&self.performer) {
            return Err(VerificationError::SelfVerification {
                obligation: self.obligation.clone(),
                actor: self.verifier.actor.clone(),
            });
        }
        self.verifier
            .independence
            .meets(assurance)
            .map_err(|source| VerificationError::InsufficientIndependence {
                obligation: self.obligation.clone(),
                actor: self.verifier.actor.clone(),
                assurance: assurance.to_owned(),
                source: Box::new(source),
            })
    }
}

/// The independence an out-of-process, read-only reviewing model actually has.
///
/// Every `true` here is a claim about how such a reviewer is invoked, and each
/// is true for the same structural reason: the reviewer is a separate process
/// that receives a diff and returns text. It cannot write to the workspace, so
/// it cannot alter artifacts, gate definitions, or fixtures; it never receives
/// the performer's conversation or reasoning, so it is blind to both; it builds
/// its own context from what it is handed.
///
/// `distinct_human_required` is `false` and must stay false: a different model
/// is not a different person, and §46.3's highest row wants an accountable
/// human. That is why `high` assurance still cannot be met this way, and the
/// separation of those two flags is exactly what stops a model review from being
/// mistaken for human accountability.
#[must_use]
pub fn read_only_reviewer_independence() -> Independence {
    Independence {
        performer_transcript_blind: true,
        performer_rationale_blind: true,
        separate_writable_workspace: true,
        cannot_modify_subject_artifacts: true,
        cannot_modify_gate_definition: true,
        cannot_modify_gate_fixtures: true,
        separate_context_compilation: true,
        distinct_model_required: true,
        distinct_human_required: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verifier(actor: &str) -> Verifier {
        Verifier {
            actor: actor.to_owned(),
            kind: ActorKind::Agent,
            model: Some("mimo-v2.5-pro".to_owned()),
            independence: read_only_reviewer_independence(),
        }
    }

    fn verification(actor: &str, performer: &str) -> Verification {
        Verification {
            obligation: "OBL-001".to_owned(),
            verifier: verifier(actor),
            disposition: Disposition::Established,
            evidence: "gate-run://abc123".to_owned(),
            performer: performer.to_owned(),
        }
    }

    #[test]
    fn a_read_only_reviewer_meets_basic_and_controlled() {
        let v = verification("lamu:mimo-v2.5-pro", "claude");
        assert_eq!(v.admissible_for("basic"), Ok(()));
        assert_eq!(v.admissible_for("controlled"), Ok(()));
    }

    /// §46.3's highest row wants an accountable human. A distinct model is not
    /// one, and this is the test that stops the two being conflated.
    #[test]
    fn a_model_reviewer_does_not_meet_high_assurance() {
        let v = verification("lamu:mimo-v2.5-pro", "claude");
        let err = v.admissible_for("high").expect_err("must refuse");
        match err {
            VerificationError::InsufficientIndependence { source, .. } => {
                assert!(
                    source.to_string().contains("distinct_human_required"),
                    "the missing dimension must be named: {source}"
                );
            }
            other => panic!("wrong error: {other:?}"),
        }
    }

    /// The single most likely route to a false PASS.
    #[test]
    fn self_verification_is_refused() {
        let v = verification("claude", "claude");
        assert_eq!(
            v.admissible_for("basic"),
            Err(VerificationError::SelfVerification {
                obligation: "OBL-001".to_owned(),
                actor: "claude".to_owned()
            })
        );
    }

    /// §40.7: a judgment may not stand in for the observation it should rest on.
    #[test]
    fn a_verdict_without_evidence_is_refused() {
        let mut v = verification("lamu:mimo-v2.5-pro", "claude");
        v.evidence = "   ".to_owned();
        assert_eq!(
            v.admissible_for("basic"),
            Err(VerificationError::NoEvidence {
                obligation: "OBL-001".to_owned()
            })
        );
    }

    #[test]
    fn an_unnamed_verifier_is_refused() {
        let v = verification("", "claude");
        assert_eq!(
            v.admissible_for("basic"),
            Err(VerificationError::NoVerifier {
                obligation: "OBL-001".to_owned()
            })
        );
    }

    /// A verifier that CAN write to the workspace fails even `basic`, because
    /// every level requires it to be unable to alter the measurement.
    #[test]
    fn a_verifier_that_could_alter_the_measurement_is_refused_at_every_level() {
        let mut v = verification("lamu:mimo-v2.5-pro", "claude");
        v.verifier.independence.cannot_modify_gate_fixtures = false;
        for level in ["basic", "controlled", "high"] {
            assert!(
                v.admissible_for(level).is_err(),
                "{level} must refuse a verifier that can edit the fixtures"
            );
        }
    }

    /// Refuted and not_established are admissible verdicts — admissibility is
    /// about whether the VERIFICATION counts, not about whether it was positive.
    /// Conflating them would mean only good news could be recorded.
    #[test]
    fn a_negative_verdict_is_still_admissible() {
        for disposition in [Disposition::Refuted, Disposition::NotEstablished] {
            let mut v = verification("lamu:mimo-v2.5-pro", "claude");
            v.disposition = disposition;
            assert_eq!(
                v.admissible_for("controlled"),
                Ok(()),
                "{disposition} must be recordable"
            );
        }
    }

    /// The reviewer profile must not silently claim human accountability.
    #[test]
    fn the_reviewer_profile_claims_eight_of_nine_dimensions() {
        let i = read_only_reviewer_independence();
        assert!(!i.distinct_human_required, "a model is not a person");
        for (ok, name) in [
            (i.performer_transcript_blind, "performer_transcript_blind"),
            (i.performer_rationale_blind, "performer_rationale_blind"),
            (i.separate_writable_workspace, "separate_writable_workspace"),
            (
                i.cannot_modify_subject_artifacts,
                "cannot_modify_subject_artifacts",
            ),
            (
                i.cannot_modify_gate_definition,
                "cannot_modify_gate_definition",
            ),
            (i.cannot_modify_gate_fixtures, "cannot_modify_gate_fixtures"),
            (
                i.separate_context_compilation,
                "separate_context_compilation",
            ),
            (i.distinct_model_required, "distinct_model_required"),
        ] {
            assert!(
                ok,
                "{name} must hold for an out-of-process read-only reviewer"
            );
        }
    }
}
