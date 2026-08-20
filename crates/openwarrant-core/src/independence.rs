// SPDX-License-Identifier: AGPL-3.0-or-later
//! Verifier independence (SAS §46, RQ-053).
//!
//! # What independence is for
//!
//! §46.2: a blind verifier receives the authorized contract, artifacts, gates,
//! evidence, and required context — and *"does not receive persuasive performer
//! narrative or private reasoning."*
//!
//! That is the whole point. A verifier that reads the performer's account of why
//! the work is correct is being persuaded rather than shown, and
//! [`BlindVerifierInput`] therefore has no field for a narrative to arrive in.
//! [`admissible_inputs`] and [`INADMISSIBLE_INPUTS`] are the two halves of §46.2,
//! and [`Independence::is_blind`] requires both narrative and rationale blindness
//! together — transcript-blind alone still lets a rationale document through.
//!
//! # OW-ADR-0004's `independence: none`
//!
//! One person may hold several roles (§27.4), and this project's Warrants are
//! authored and verified by the same actor. [`Independence::none`] is that state
//! named rather than omitted, and [`Independence::meets`] refuses to report it as
//! satisfying `controlled` or `high_assurance`. §27.4's own words: *"Role
//! separation by one person is not organizational independence."*

use serde::{Deserialize, Serialize};

use crate::vocab::vocabulary;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IndependenceError {
    #[error("unknown {vocabulary} {found:?}; SAS defines {known}")]
    UnknownTerm {
        vocabulary: &'static str,
        found: String,
        known: String,
    },
    #[error(
        "{assurance} assurance requires {required}, and the declared independence \
         does not provide it: {missing}. §27.4 — role separation by one person is \
         not organizational independence"
    )]
    InsufficientIndependence {
        assurance: String,
        required: IndependenceMinimum,
        missing: String,
    },
    #[error(
        "the verifier can modify {asset}. A verifier able to change what it \
         measures is not measuring it — §32.5 requires that the performer cannot \
         modify protected gate assets, and the same holds of the verifier"
    )]
    VerifierCanModify { asset: &'static str },
    #[error(
        "a blind verifier was given {input}, which §46.2 excludes: it receives the \
         contract, artifacts, gates, evidence, and required context, and NOT \
         persuasive performer narrative or private reasoning"
    )]
    InadmissibleInput { input: String },
}

vocabulary!(
    /// §46.3's minimum per assurance level.
    IndependenceMinimum, "independence minimum", IndependenceError, {
        VerifierControlledExecution => "verifier_controlled_execution",
        BlindProcessOrAgentReview => "blind_process_or_agent_review",
        IndependentAccountableControl => "independent_accountable_control",
    }
);

impl IndependenceMinimum {
    /// §46.3's table.
    #[must_use]
    pub fn for_assurance(level: &str) -> Self {
        match level {
            "controlled" => Self::BlindProcessOrAgentReview,
            "high" | "high_assurance" => Self::IndependentAccountableControl,
            _ => Self::VerifierControlledExecution,
        }
    }
}

/// §46.1's nine independence dimensions.
///
/// Booleans rather than a summary score, because §46.3's minimums are about
/// *which* separations hold. A single "independence level" field would let a
/// verifier that shares a workspace claim the same standing as one that does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Independence {
    pub performer_transcript_blind: bool,
    pub performer_rationale_blind: bool,
    pub separate_writable_workspace: bool,
    pub cannot_modify_subject_artifacts: bool,
    pub cannot_modify_gate_definition: bool,
    pub cannot_modify_gate_fixtures: bool,
    pub separate_context_compilation: bool,
    pub distinct_model_required: bool,
    pub distinct_human_required: bool,
}

impl Default for Independence {
    /// §46.1's own defaults: the two `distinct_*` dimensions are `false` there,
    /// and everything else `true`.
    fn default() -> Self {
        Self {
            performer_transcript_blind: true,
            performer_rationale_blind: true,
            separate_writable_workspace: true,
            cannot_modify_subject_artifacts: true,
            cannot_modify_gate_definition: true,
            cannot_modify_gate_fixtures: true,
            separate_context_compilation: true,
            distinct_model_required: false,
            distinct_human_required: false,
        }
    }
}

impl Independence {
    /// No independence at all — one actor, every role.
    ///
    /// OW-ADR-0004 requires this to be RECORDED rather than omitted. An absent
    /// independence field reads as unexamined; `none` reads as examined and
    /// absent, and `meets` refuses to let it satisfy anything above `basic`.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            performer_transcript_blind: false,
            performer_rationale_blind: false,
            separate_writable_workspace: false,
            cannot_modify_subject_artifacts: false,
            cannot_modify_gate_definition: false,
            cannot_modify_gate_fixtures: false,
            separate_context_compilation: false,
            distinct_model_required: false,
            distinct_human_required: false,
        }
    }

    /// §46.2 — blindness requires BOTH narrative and reasoning to be withheld.
    ///
    /// Transcript-blind alone is not blind: a rationale document is exactly the
    /// "private reasoning" §46.2 excludes, and it persuades just as well without
    /// the transcript around it.
    #[must_use]
    pub const fn is_blind(self) -> bool {
        self.performer_transcript_blind && self.performer_rationale_blind
    }

    /// Whether the verifier is unable to change what it measures.
    #[must_use]
    pub const fn cannot_alter_the_measurement(self) -> bool {
        self.cannot_modify_subject_artifacts
            && self.cannot_modify_gate_definition
            && self.cannot_modify_gate_fixtures
    }

    /// Whether execution is genuinely verifier-controlled (§46.3, `basic`).
    #[must_use]
    pub const fn is_verifier_controlled(self) -> bool {
        self.separate_writable_workspace && self.cannot_alter_the_measurement()
    }

    /// Which dimensions are missing for a given minimum, named.
    #[must_use]
    pub fn missing_for(self, minimum: IndependenceMinimum) -> Vec<&'static str> {
        let mut missing = Vec::new();
        let mut want = |ok: bool, name: &'static str| {
            if !ok {
                missing.push(name);
            }
        };
        // Every level needs the verifier unable to alter the measurement.
        want(
            self.cannot_modify_subject_artifacts,
            "cannot_modify_subject_artifacts",
        );
        want(
            self.cannot_modify_gate_definition,
            "cannot_modify_gate_definition",
        );
        want(
            self.cannot_modify_gate_fixtures,
            "cannot_modify_gate_fixtures",
        );
        want(
            self.separate_writable_workspace,
            "separate_writable_workspace",
        );

        if minimum >= IndependenceMinimum::BlindProcessOrAgentReview {
            want(
                self.performer_transcript_blind,
                "performer_transcript_blind",
            );
            want(self.performer_rationale_blind, "performer_rationale_blind");
            want(
                self.separate_context_compilation,
                "separate_context_compilation",
            );
        }
        if minimum == IndependenceMinimum::IndependentAccountableControl {
            // §46.3's highest row wants an independent accountable person or an
            // equivalent control. A distinct model is not a distinct person, and
            // both are recorded separately for that reason.
            want(self.distinct_human_required, "distinct_human_required");
        }
        missing
    }

    /// §46.3 — whether this independence meets a level's minimum.
    pub fn meets(self, assurance: &str) -> Result<(), IndependenceError> {
        let required = IndependenceMinimum::for_assurance(assurance);
        let missing = self.missing_for(required);
        if missing.is_empty() {
            Ok(())
        } else {
            Err(IndependenceError::InsufficientIndependence {
                assurance: assurance.to_owned(),
                required,
                missing: missing.join(", "),
            })
        }
    }
}

/// §46.2's admissible inputs to a blind verifier.
pub const ADMISSIBLE_INPUTS: [&str; 5] = [
    "authorized contract",
    "artifacts",
    "gates",
    "evidence",
    "required context",
];

/// What §46.2 excludes, named so that exclusion is checkable rather than assumed.
pub const INADMISSIBLE_INPUTS: [&str; 4] = [
    "performer narrative",
    "performer private reasoning",
    "performer transcript",
    "performer submission rationale",
];

/// What a blind verifier is handed (§46.2).
///
/// There is deliberately no `narrative` field. A struct that could carry one
/// would need a rule forbidding its use; a struct that cannot carry one needs no
/// rule. [`Self::validate`] exists only for inputs arriving by name from outside
/// the type system.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BlindVerifierInput {
    pub authorized_contract_digest: String,
    #[serde(default)]
    pub artifact_refs: Vec<String>,
    #[serde(default)]
    pub gate_binding_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub required_context_refs: Vec<String>,
}

impl BlindVerifierInput {
    /// Refuse anything §46.2 excludes, when inputs are named dynamically.
    pub fn validate_supplied(supplied: &[String]) -> Result<(), IndependenceError> {
        for input in supplied {
            let lower = input.to_lowercase();
            if INADMISSIBLE_INPUTS.iter().any(|bad| lower.contains(bad)) {
                return Err(IndependenceError::InadmissibleInput {
                    input: input.clone(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    /// §46.3's table, transcribed.
    #[test]
    fn the_minimums_match_the_sas() {
        assert_eq!(
            IndependenceMinimum::for_assurance("basic"),
            IndependenceMinimum::VerifierControlledExecution
        );
        assert_eq!(
            IndependenceMinimum::for_assurance("controlled"),
            IndependenceMinimum::BlindProcessOrAgentReview
        );
        assert_eq!(
            IndependenceMinimum::for_assurance("high_assurance"),
            IndependenceMinimum::IndependentAccountableControl
        );
        assert_eq!(
            IndependenceMinimum::for_assurance("high"),
            IndependenceMinimum::IndependentAccountableControl
        );
    }

    /// §46.1's declared defaults, transcribed field by field.
    #[test]
    fn the_dimensions_match_the_sas_defaults() {
        let d = Independence::default();
        assert!(d.performer_transcript_blind);
        assert!(d.performer_rationale_blind);
        assert!(d.separate_writable_workspace);
        assert!(d.cannot_modify_subject_artifacts);
        assert!(d.cannot_modify_gate_definition);
        assert!(d.cannot_modify_gate_fixtures);
        assert!(d.separate_context_compilation);
        assert!(!d.distinct_model_required, "§46.1 shows false");
        assert!(!d.distinct_human_required, "§46.1 shows false");
    }

    /// OW-ADR-0004's case: one actor, every role. It must not satisfy anything
    /// above `basic`, and it must not satisfy `basic` either — this project's
    /// own Warrants are not verifier-controlled.
    #[test]
    fn independence_none_satisfies_nothing() {
        let none = Independence::none();
        for level in ["basic", "controlled", "high_assurance"] {
            let err = none.meets(level).unwrap_err();
            assert!(
                matches!(err, IndependenceError::InsufficientIndependence { .. }),
                "{level}: {err}"
            );
            assert!(
                err.to_string().contains("not organizational independence"),
                "the error should say why: {err}"
            );
        }
    }

    /// §46.2 — transcript-blind alone is not blind. A rationale document is the
    /// "private reasoning" the section excludes and persuades just as well.
    #[test]
    fn transcript_blindness_alone_is_not_blindness() {
        let half = Independence {
            performer_transcript_blind: true,
            performer_rationale_blind: false,
            ..Independence::default()
        };
        assert!(!half.is_blind(), "rationale still reached the verifier");
        let missing = half.missing_for(IndependenceMinimum::BlindProcessOrAgentReview);
        assert!(
            missing.contains(&"performer_rationale_blind"),
            "{missing:?}"
        );

        // ...and the same the other way round.
        let other_half = Independence {
            performer_transcript_blind: false,
            performer_rationale_blind: true,
            ..Independence::default()
        };
        assert!(!other_half.is_blind());
    }

    /// A verifier that can edit the gate is not measuring anything.
    #[test]
    fn a_verifier_that_can_alter_the_measurement_fails_every_level() {
        for (name, mut i) in [
            ("subject artifacts", Independence::default()),
            ("gate definition", Independence::default()),
            ("gate fixtures", Independence::default()),
        ] {
            match name {
                "subject artifacts" => i.cannot_modify_subject_artifacts = false,
                "gate definition" => i.cannot_modify_gate_definition = false,
                _ => i.cannot_modify_gate_fixtures = false,
            }
            assert!(!i.cannot_alter_the_measurement(), "{name}");
            for level in ["basic", "controlled", "high_assurance"] {
                assert!(i.meets(level).is_err(), "{name} was permitted at {level}");
            }
        }
    }

    /// The SAS defaults satisfy `basic` and `controlled`, but NOT
    /// `high_assurance` — because `distinct_human_required` defaults to false and
    /// §46.3's top row wants an independent accountable person.
    #[test]
    fn the_defaults_meet_controlled_but_not_high_assurance() {
        let d = Independence::default();
        assert_eq!(d.meets("basic"), Ok(()));
        assert_eq!(d.meets("controlled"), Ok(()));

        let err = d.meets("high_assurance").unwrap_err();
        assert!(err.to_string().contains("distinct_human_required"), "{err}");
    }

    /// A distinct model is not a distinct person. §46.3's highest row asks for an
    /// accountable person or an equivalent control, and conflating the two would
    /// let a second model stand in for a human reviewer.
    #[test]
    fn a_distinct_model_does_not_substitute_for_a_distinct_human() {
        let model_only = Independence {
            distinct_model_required: true,
            distinct_human_required: false,
            ..Independence::default()
        };
        assert!(
            model_only.meets("high_assurance").is_err(),
            "a second model was accepted as an independent accountable person"
        );

        let with_human = Independence {
            distinct_human_required: true,
            ..model_only
        };
        assert_eq!(with_human.meets("high_assurance"), Ok(()));
    }

    /// §46.2's two lists.
    #[test]
    fn the_blind_verifier_input_lists_match_the_sas() {
        assert_eq!(
            ADMISSIBLE_INPUTS,
            [
                "authorized contract",
                "artifacts",
                "gates",
                "evidence",
                "required context",
            ]
        );
        // The type itself has no field a narrative could arrive in.
        let input = BlindVerifierInput::default();
        let json = serde_json::to_string(&input).expect("serialize");
        for excluded in ["narrative", "rationale", "transcript", "reasoning"] {
            assert!(
                !json.contains(excluded),
                "BlindVerifierInput has a field named {excluded}"
            );
        }
    }

    #[test]
    fn persuasive_input_is_refused_by_name() {
        for bad in [
            "performer narrative",
            "Performer Private Reasoning",
            "the performer transcript from attempt 3",
        ] {
            assert!(
                matches!(
                    BlindVerifierInput::validate_supplied(&[bad.to_owned()]),
                    Err(IndependenceError::InadmissibleInput { .. })
                ),
                "{bad:?} was admitted"
            );
        }
        assert_eq!(
            BlindVerifierInput::validate_supplied(&[
                "authorized contract".into(),
                "evidence".into()
            ]),
            Ok(())
        );
    }

    #[test]
    fn missing_dimensions_are_named_not_counted() {
        let bare = Independence::none();
        let missing = bare.missing_for(IndependenceMinimum::IndependentAccountableControl);
        for expected in [
            "cannot_modify_subject_artifacts",
            "cannot_modify_gate_definition",
            "cannot_modify_gate_fixtures",
            "separate_writable_workspace",
            "performer_transcript_blind",
            "performer_rationale_blind",
            "separate_context_compilation",
            "distinct_human_required",
        ] {
            assert!(missing.contains(&expected), "{expected} not reported");
        }
    }

    #[test]
    fn vocabularies_round_trip() {
        for &m in IndependenceMinimum::ALL {
            assert_eq!(IndependenceMinimum::from_str(m.as_str()), Ok(m));
        }
        assert!(IndependenceMinimum::from_str("pretty_independent").is_err());
    }
}
