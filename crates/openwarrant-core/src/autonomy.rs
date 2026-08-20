// SPDX-License-Identifier: AGPL-3.0-or-later
//! The autonomy envelope (SAS §30) and amendment records (§31).
//!
//! # What decides whether an executor may act alone
//!
//! §30 sorts every change into three bands: a local choice needing no revision,
//! an auto-authorized revision that a policy already decided, and a manual
//! revision needing a human and usually an ADR. The bands are closed lists, and
//! the classification is deliberately NOT a judgement call at execution time —
//! §30.4 sets the default:
//!
//! ```yaml
//! on_ambiguity: block_and_propose
//! ```
//!
//! followed by *"An executor SHALL NOT improvise normative semantics."*
//! [`classify`] returns `None` for anything it does not recognise, and
//! [`AmbiguityBehavior::default`] is `block_and_propose`, so an unrecognised
//! change blocks rather than falling into the most permissive band. A classifier
//! that guessed would be an executor improvising.
//!
//! # §31's amendment record
//!
//! Twelve fields, all required, and one sentence that constrains what an
//! amendment can mean: *"An amendment SHALL NOT retroactively reinterpret prior
//! execution."* [`AmendmentRecord::validate`] enforces the fields;
//! [`ArtifactAdmissibility`] is where the sentence bites, because deciding that
//! already-produced artifacts remain admissible is a decision that must be
//! recorded rather than assumed.

use serde::{Deserialize, Serialize};

use crate::vocab::vocabulary;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AutonomyError {
    #[error("unknown {vocabulary} {found:?}; SAS defines {known}")]
    UnknownTerm {
        vocabulary: &'static str,
        found: String,
        known: String,
    },
    #[error(
        "change {change:?} is not in any §30 band. §30.4 sets on_ambiguity: \
         block_and_propose, and an executor SHALL NOT improvise normative \
         semantics — so this blocks and is proposed, rather than being guessed into \
         the most convenient band"
    )]
    Unclassifiable { change: String },
    #[error(
        "amendment {id:?} omits {field}, which §31 requires of every revision after \
         authorization"
    )]
    AmendmentIncomplete { id: String, field: &'static str },
    #[error(
        "amendment {id:?} is classed {band} but carries no governing ADR or policy. \
         §30.3 requires manual authorization and usually an ADR; §30.2's \
         auto-authorization is only meaningful because a policy made the decision \
         in advance"
    )]
    AmendmentWithoutAuthority { id: String, band: AutonomyBand },
    #[error(
        "amendment {id:?} declares prior artifacts inadmissible AND requires no \
         re-preflight. If earlier work is no longer admissible the contract changed \
         materially, and §32 readiness has to be re-established"
    )]
    InadmissibleWithoutRepreflight { id: String },
}

vocabulary!(
    /// Which §30 band a change falls into.
    AutonomyBand, "autonomy band", AutonomyError, {
        LocalChoice => "local_choice",
        AutoAuthorizedRevision => "auto_authorized_revision",
        ManualRevision => "manual_revision",
    }
);

impl AutonomyBand {
    /// Whether a contract revision is created at all (§30.1 creates none).
    #[must_use]
    pub const fn creates_revision(self) -> bool {
        matches!(self, Self::AutoAuthorizedRevision | Self::ManualRevision)
    }

    /// Whether a human must authorize (§30.3).
    #[must_use]
    pub const fn needs_human_authorization(self) -> bool {
        matches!(self, Self::ManualRevision)
    }
}

vocabulary!(
    /// §30.4's behaviour when a change does not classify.
    AmbiguityBehavior, "ambiguity behavior", AutonomyError, {
        BlockAndPropose => "block_and_propose",
        Proceed => "proceed",
    }
);

impl Default for AmbiguityBehavior {
    /// §30.4: *"The default is `on_ambiguity: block_and_propose`."*
    fn default() -> Self {
        Self::BlockAndPropose
    }
}

/// §30.1's local choices, verbatim.
pub const LOCAL_CHOICES: [&str; 6] = [
    "private module organization",
    "private symbol naming",
    "equivalent internal algorithm preserving declared invariants",
    "additional non-mutating tests",
    "use of an already authorized tool",
    "diagnostic instrumentation removed before submission",
];

/// §30.2's auto-authorizable revisions, verbatim.
pub const AUTO_AUTHORIZED: [&str; 6] = [
    "adding read-only context",
    "adding a stricter gate",
    "increasing timeout within a ceiling",
    "adding a development-only dependency from an approved source",
    "attaching prior failure evidence to a repair attempt",
    "clarifying wording without semantic change",
];

/// §30.3's manual-revision triggers, verbatim.
pub const MANUAL_REVISION: [&str; 11] = [
    "completion-claim change",
    "scope expansion",
    "public interface change",
    "security or safety boundary change",
    "new production dependency",
    "gate weakening",
    "pass-threshold change",
    "new external side effect",
    "material budget change",
    "release or regulatory impact",
    "accepted residual risk",
];

/// Classify a change against §30's three lists.
///
/// Returns `None` when the change matches nothing. That is the important case:
/// §30.4 says an unclassified change blocks and is proposed, so this deliberately
/// does no fuzzy matching. A classifier that guessed "probably a local choice"
/// would be an executor improvising normative semantics, which §30.4 forbids in
/// so many words.
#[must_use]
pub fn classify(change: &str) -> Option<AutonomyBand> {
    let c = change.trim().to_lowercase();
    if MANUAL_REVISION.iter().any(|m| *m == c) {
        return Some(AutonomyBand::ManualRevision);
    }
    if AUTO_AUTHORIZED.iter().any(|m| *m == c) {
        return Some(AutonomyBand::AutoAuthorizedRevision);
    }
    if LOCAL_CHOICES.iter().any(|m| *m == c) {
        return Some(AutonomyBand::LocalChoice);
    }
    None
}

/// Classify, or fail in the direction §30.4 requires.
pub fn classify_or_block(change: &str) -> Result<AutonomyBand, AutonomyError> {
    classify(change).ok_or_else(|| AutonomyError::Unclassifiable {
        change: change.to_owned(),
    })
}

/// What an amendment decides about work already produced (§31).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactAdmissibility {
    /// Prior artifacts stand. §31: an amendment SHALL NOT retroactively
    /// reinterpret prior execution, so this is the neutral answer.
    RemainAdmissible,
    /// Prior artifacts no longer support the amended contract. This is not a
    /// reinterpretation of what they showed — it is a statement that the contract
    /// they were produced against is no longer the contract.
    Inadmissible,
}

/// §31's amendment record. Twelve fields, none optional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmendmentRecord {
    pub id: String,
    pub band: AutonomyBand,
    /// A structured diff, not prose. §31 says "structured semantic diff".
    #[serde(default)]
    pub semantic_diff: Vec<SemanticChange>,
    pub reason: String,
    pub governing_adr_or_policy: String,
    #[serde(default)]
    pub affected_stages: Vec<String>,
    #[serde(default)]
    pub affected_milestones: Vec<String>,
    #[serde(default)]
    pub affected_attempts: Vec<String>,
    #[serde(default)]
    pub affected_gate_runs: Vec<String>,
    pub artifact_admissibility: ArtifactAdmissibility,
    pub restart_or_repair_instruction: String,
    pub re_preflight_required: bool,
    pub authorizer: String,
    pub effective_time: String,
}

/// One entry in §31's structured semantic diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticChange {
    /// Which §28.5 contract element moved.
    pub element: crate::contract::ContractElement,
    pub before: String,
    pub after: String,
}

impl AmendmentRecord {
    pub fn validate(&self) -> Result<(), AutonomyError> {
        let required: [(&'static str, &str); 6] = [
            ("reason", &self.reason),
            ("governing_adr_or_policy", &self.governing_adr_or_policy),
            (
                "restart_or_repair_instruction",
                &self.restart_or_repair_instruction,
            ),
            ("authorizer", &self.authorizer),
            ("effective_time", &self.effective_time),
            ("id", &self.id),
        ];
        for (field, value) in required {
            if value.trim().is_empty() {
                return Err(AutonomyError::AmendmentIncomplete {
                    id: self.id.clone(),
                    field,
                });
            }
        }
        if self.semantic_diff.is_empty() {
            return Err(AutonomyError::AmendmentIncomplete {
                id: self.id.clone(),
                field: "semantic_diff",
            });
        }
        // §30.2 and §30.3 — a revision without a named authority is a change
        // nobody authorized.
        if self.band.creates_revision() && self.governing_adr_or_policy.trim().is_empty() {
            return Err(AutonomyError::AmendmentWithoutAuthority {
                id: self.id.clone(),
                band: self.band,
            });
        }
        // If earlier work no longer holds, readiness has to be re-established.
        if self.artifact_admissibility == ArtifactAdmissibility::Inadmissible
            && !self.re_preflight_required
        {
            return Err(AutonomyError::InadmissibleWithoutRepreflight {
                id: self.id.clone(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::ContractElement;
    use std::str::FromStr;

    fn amendment() -> AmendmentRecord {
        AmendmentRecord {
            id: "AM-001".into(),
            band: AutonomyBand::ManualRevision,
            semantic_diff: vec![SemanticChange {
                element: ContractElement::Scope,
                before: "one crate".into(),
                after: "two crates".into(),
            }],
            reason: "the second crate was discovered to be in scope".into(),
            governing_adr_or_policy: "adr://OW-ADR-0004".into(),
            affected_stages: vec!["STAGE-001".into()],
            affected_milestones: vec!["M1".into()],
            affected_attempts: vec![],
            affected_gate_runs: vec![],
            artifact_admissibility: ArtifactAdmissibility::RemainAdmissible,
            restart_or_repair_instruction: "continue; no rework required".into(),
            re_preflight_required: true,
            authorizer: "QuiteTall".into(),
            effective_time: "2026-08-20T00:00:00Z".into(),
        }
    }

    /// §30's three lists, transcribed. If the SAS gains a trigger and this is not
    /// updated, the new trigger falls to `None` and blocks — which is the correct
    /// direction to fail.
    #[test]
    fn the_three_bands_match_the_sas() {
        assert_eq!(LOCAL_CHOICES.len(), 6);
        assert_eq!(AUTO_AUTHORIZED.len(), 6);
        assert_eq!(MANUAL_REVISION.len(), 11);
        assert_eq!(
            classify("private symbol naming"),
            Some(AutonomyBand::LocalChoice)
        );
        assert_eq!(
            classify("adding a stricter gate"),
            Some(AutonomyBand::AutoAuthorizedRevision)
        );
        assert_eq!(
            classify("gate weakening"),
            Some(AutonomyBand::ManualRevision)
        );
    }

    /// §30.4 — an unrecognised change blocks. This is the whole point.
    #[test]
    fn an_unrecognised_change_blocks_rather_than_being_guessed() {
        for change in [
            "adding a slightly weaker gate",
            "renaming a public function",
            "a small refactor",
            "",
        ] {
            assert_eq!(classify(change), None, "{change:?} was classified");
            assert!(
                matches!(
                    classify_or_block(change),
                    Err(AutonomyError::Unclassifiable { .. })
                ),
                "{change:?} did not block"
            );
        }
    }

    /// The default must be the safe one. A `Default` that proceeded would make
    /// every unconfigured executor the permissive kind.
    #[test]
    fn the_ambiguity_default_is_block_and_propose() {
        assert_eq!(
            AmbiguityBehavior::default(),
            AmbiguityBehavior::BlockAndPropose
        );
    }

    /// Adjacent-sounding changes must not collapse into one band: "adding a
    /// stricter gate" is auto-authorized, "gate weakening" is manual.
    #[test]
    fn stricter_and_weaker_gates_are_different_bands() {
        assert_eq!(
            classify("adding a stricter gate"),
            Some(AutonomyBand::AutoAuthorizedRevision)
        );
        assert_eq!(
            classify("gate weakening"),
            Some(AutonomyBand::ManualRevision)
        );
        assert!(AutonomyBand::ManualRevision.needs_human_authorization());
        assert!(!AutonomyBand::AutoAuthorizedRevision.needs_human_authorization());
    }

    /// §30.1 — a local choice creates no revision; the other two do.
    #[test]
    fn only_local_choices_avoid_a_revision() {
        assert!(!AutonomyBand::LocalChoice.creates_revision());
        assert!(AutonomyBand::AutoAuthorizedRevision.creates_revision());
        assert!(AutonomyBand::ManualRevision.creates_revision());
    }

    #[test]
    fn a_complete_amendment_validates() {
        assert_eq!(amendment().validate(), Ok(()));
    }

    #[test]
    fn each_missing_required_field_is_named() {
        for (name, blank) in [
            (
                "reason",
                (|a: &mut AmendmentRecord| a.reason.clear()) as fn(&mut AmendmentRecord),
            ),
            ("governing_adr_or_policy", |a| {
                a.governing_adr_or_policy.clear()
            }),
            ("restart_or_repair_instruction", |a| {
                a.restart_or_repair_instruction.clear()
            }),
            ("authorizer", |a| a.authorizer.clear()),
            ("effective_time", |a| a.effective_time.clear()),
            ("semantic_diff", |a| a.semantic_diff.clear()),
        ] {
            let mut a = amendment();
            blank(&mut a);
            match a.validate() {
                Err(AutonomyError::AmendmentIncomplete { field, .. }) => {
                    assert_eq!(field, name);
                }
                Err(AutonomyError::AmendmentWithoutAuthority { .. })
                    if name == "governing_adr_or_policy" => {}
                other => panic!("blanking {name} was accepted: {other:?}"),
            }
        }
    }

    /// §31 — an amendment that voids prior work has changed the contract
    /// materially, so readiness must be re-established.
    #[test]
    fn declaring_prior_artifacts_inadmissible_forces_a_re_preflight() {
        let mut a = amendment();
        a.artifact_admissibility = ArtifactAdmissibility::Inadmissible;
        a.re_preflight_required = false;
        assert!(matches!(
            a.validate(),
            Err(AutonomyError::InadmissibleWithoutRepreflight { .. })
        ));

        a.re_preflight_required = true;
        assert_eq!(a.validate(), Ok(()));
    }

    #[test]
    fn vocabularies_round_trip() {
        for &b in AutonomyBand::ALL {
            assert_eq!(AutonomyBand::from_str(b.as_str()), Ok(b));
        }
        for &b in AmbiguityBehavior::ALL {
            assert_eq!(AmbiguityBehavior::from_str(b.as_str()), Ok(b));
        }
        assert!(AutonomyBand::from_str("probably_fine").is_err());
    }

    #[test]
    fn an_amendment_round_trips_through_json() {
        let a = amendment();
        let s = serde_json::to_string(&a).expect("serialize");
        assert_eq!(
            serde_json::from_str::<AmendmentRecord>(&s).expect("deserialize"),
            a
        );
    }
}
