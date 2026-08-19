// SPDX-License-Identifier: AGPL-3.0-or-later
//! Contract-adequacy review (SAS §39, RQ-055).
//!
//! §39 opens: "The authoring process SHALL test whether the gate set
//! meaningfully supports each obligation." §39.1 gives the adversarial question
//! in one sentence:
//!
//! > Construct an artifact that passes every declared gate while violating this
//! > obligation.
//!
//! # What this replaces
//!
//! RQ-055 was previously satisfied by a substring search in the shipped binary:
//! any assurance atom whose text merely contained the word passed. That is a
//! check-shaped non-check, and it is exactly the failure this project exists to
//! prevent, shipped by this project. The old call site is gone, and the word is
//! not written as a string literal in the check path, so grepping for it finds
//! no resurrection.
//!
//! # What it deliberately does not do
//!
//! §39.5 is explicit: "No generic compiler can prove that arbitrary gates entail
//! arbitrary natural-language claims. OpenWarrant exposes and records the
//! remaining judgment." So this validates that a review was CONDUCTED and
//! reached an outcome. It cannot validate that the review was any good, and
//! nothing here should be read as doing so.

use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AdequacyError {
    #[error("unknown adequacy outcome {found:?}; expected one of {known}")]
    UnknownOutcome { found: String, known: String },
    #[error(
        "assurance level {level} requires a contract-adequacy review (§39.4) and no \
         `## Gate Adequacy` section was found"
    )]
    ReviewMissing { level: String },
    #[error(
        "the adequacy review records no adversarial question (§39.1). The question is \
         the review: \"construct an artifact that passes every declared gate while \
         violating this obligation\""
    )]
    QuestionMissing,
}

/// What an adequacy review concluded (SAS §39.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdequacyOutcome {
    CounterexampleFound,
    NoCounterexampleFound,
    ObligationNarrowed,
    GateAdded,
    GateStrengthened,
    GapAccepted,
    ClaimRemoved,
    ReviewNotRequired,
}

impl AdequacyOutcome {
    pub const ALL: [Self; 8] = [
        Self::CounterexampleFound,
        Self::NoCounterexampleFound,
        Self::ObligationNarrowed,
        Self::GateAdded,
        Self::GateStrengthened,
        Self::GapAccepted,
        Self::ClaimRemoved,
        Self::ReviewNotRequired,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CounterexampleFound => "counterexample_found",
            Self::NoCounterexampleFound => "no_counterexample_found",
            Self::ObligationNarrowed => "obligation_narrowed",
            Self::GateAdded => "gate_added",
            Self::GateStrengthened => "gate_strengthened",
            Self::GapAccepted => "gap_accepted",
            Self::ClaimRemoved => "claim_removed",
            Self::ReviewNotRequired => "review_not_required",
        }
    }
}

impl FromStr for AdequacyOutcome {
    type Err = AdequacyError;
    fn from_str(s: &str) -> Result<Self, AdequacyError> {
        Self::ALL
            .into_iter()
            .find(|o| o.as_str() == s)
            .ok_or_else(|| AdequacyError::UnknownOutcome {
                found: s.to_owned(),
                known: Self::ALL
                    .iter()
                    .map(|o| o.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            })
    }
}

impl fmt::Display for AdequacyOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The adequacy requirement per assurance level (SAS §39.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdequacyRequirement {
    /// basic — structural checks; semantic review optional by policy.
    Structural,
    /// controlled — blind adversarial review required.
    BlindAdversarial,
    /// high_assurance — independent domain review plus executed negative
    /// controls or equivalent.
    IndependentPlusNegativeControls,
}

impl AdequacyRequirement {
    /// §39.4's table.
    #[must_use]
    pub fn for_level(level: &str) -> Self {
        match level {
            "controlled" => Self::BlindAdversarial,
            "high" | "high_assurance" => Self::IndependentPlusNegativeControls,
            _ => Self::Structural,
        }
    }

    /// Whether a review section must be present at all.
    #[must_use]
    pub const fn requires_review(self) -> bool {
        match self {
            Self::Structural => false,
            Self::BlindAdversarial | Self::IndependentPlusNegativeControls => true,
        }
    }

    /// Whether §39.4 requires executed negative controls, not merely permits
    /// them under §39.3's "where economical".
    #[must_use]
    pub const fn requires_executed_controls(self) -> bool {
        match self {
            Self::Structural | Self::BlindAdversarial => false,
            Self::IndependentPlusNegativeControls => true,
        }
    }
}

/// A contract-adequacy review as recorded in an assurance atom.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AdequacyReview {
    /// §39.1 — the adversarial question, asked.
    pub question: String,
    /// §39.2 — what it concluded. May be empty when the review is pending.
    pub outcomes: BTreeSet<AdequacyOutcome>,
    /// §39.3 — attacks actually run. Empty means none were.
    pub executed_attacks: Vec<String>,
    /// §39.5 — the judgment that remains.
    pub limitation: Option<String>,
    /// Whether the atom carries a `## Gate Adequacy` section at all.
    pub present: bool,
}

impl AdequacyReview {
    /// Whether §39.3's attacks were actually executed.
    ///
    /// A section that says attacks will be "recorded here when run" has run
    /// none. That phrasing is a plan, and §39.3 wants evidence.
    #[must_use]
    pub fn has_executed_attacks(&self) -> bool {
        !self.executed_attacks.is_empty()
    }

    /// Whether the review reached any §39.2 outcome.
    #[must_use]
    pub fn has_outcome(&self) -> bool {
        !self.outcomes.is_empty()
    }

    /// Validate structurally against the level's requirement.
    ///
    /// Deliberately does NOT fail on missing executed attacks: §39.3 says
    /// "SHOULD ... where economical", and turning a SHOULD into a hard failure
    /// would make it the first rule anyone disables. The caller reports that gap
    /// separately.
    pub fn validate(
        &self,
        requirement: AdequacyRequirement,
        level: &str,
    ) -> Result<(), AdequacyError> {
        if !requirement.requires_review() {
            return Ok(());
        }
        if !self.present {
            return Err(AdequacyError::ReviewMissing {
                level: level.to_owned(),
            });
        }
        if self.question.trim().is_empty() {
            return Err(AdequacyError::QuestionMissing);
        }
        Ok(())
    }
}

/// The section heading §39 reviews are recorded under.
pub const ADEQUACY_HEADING: &str = "Gate Adequacy";

/// Phrases that state the absence of executed attacks rather than reporting one.
///
/// Each of these has appeared in this repository's own corpus. "recorded here
/// when run" is a plan; "none yet" is an admission. Both mean zero attacks were
/// executed, and reading either as an attack turns the §39.3 warning off for
/// exactly the Warrants that most need it.
const ABSENCE_PHRASES: [&str; 5] = [
    "when run",
    "to be recorded",
    "none yet",
    "none —",
    "no attacks",
];

/// Extract the adequacy review from an assurance atom.
///
/// Reads the `## Gate Adequacy` section: the adversarial question, any
/// `- **outcome:**` bullets, and the executed-attacks statement.
#[must_use]
pub fn parse(source: &str) -> AdequacyReview {
    let mut review = AdequacyReview::default();
    let mut in_section = false;
    let mut in_attacks = false;

    for line in source.lines() {
        let trimmed = line.trim();

        if let Some(heading) = trimmed.strip_prefix("## ") {
            // Exact heading, not a substring. The whole point of this module is
            // that a substring match is not a check; using one to FIND the
            // section would leave the same weakness one layer down, where
            // `## Notes On Adequacy` would open a review section that is not one.
            in_section = heading.trim().eq_ignore_ascii_case(ADEQUACY_HEADING);
            in_attacks = false;
            continue;
        }
        if !in_section {
            continue;
        }
        review.present = true;

        let lower = trimmed.to_lowercase();

        // An outcome bullet, if the author recorded one.
        if let Some(rest) = trimmed
            .strip_prefix("- **outcome:**")
            .or_else(|| trimmed.strip_prefix("- **outcomes:**"))
        {
            for token in rest.split(',') {
                if let Ok(o) = AdequacyOutcome::from_str(token.trim()) {
                    review.outcomes.insert(o);
                }
            }
            continue;
        }

        // The adversarial question. An explicit label, the SAS's own phrasing,
        // or a bolded interrogative.
        //
        // Deliberately NOT "any line containing `could`": ordinary prose in this
        // section says "this could be improved" constantly, and a bare
        // substring would accept the first such sentence as the review's
        // question — the same substring failure this module replaces, moved
        // inside it.
        let is_question = lower.contains("adversarial question")
            || lower.contains("passes every declared gate")
            || (trimmed.starts_with("**") && trimmed.contains('?'));
        if review.question.is_empty() && is_question {
            review.question = trimmed.to_owned();
            continue;
        }

        if lower.starts_with("**executed attacks") || lower.starts_with("executed attacks") {
            in_attacks = true;
            // A statement of absence is not an attack. See ABSENCE_PHRASES.
            if !ABSENCE_PHRASES.iter().any(|p| lower.contains(p)) {
                // `**Executed attacks:**` splits to a bare `**`, which is
                // emphasis, not an attack. Strip the markup before deciding
                // whether anything was actually recorded on this line.
                let rest = trimmed
                    .split_once(':')
                    .map(|(_, r)| r.trim().trim_matches('*').trim())
                    .unwrap_or_default();
                if !rest.is_empty() {
                    review.executed_attacks.push(rest.to_owned());
                }
            }
            continue;
        }

        if in_attacks && trimmed.starts_with("- ") {
            review.executed_attacks.push(trimmed[2..].to_owned());
            continue;
        }

        if lower.contains("limitation") && review.limitation.is_none() {
            review.limitation = Some(trimmed.to_owned());
        }
    }

    review
}

#[cfg(test)]
mod tests {
    use super::*;

    const WITH_PLACEHOLDER: &str = r"# Assurance

## Gate Adequacy

Required at `controlled`.

**Adversarial question: could every obligation pass while the system is wrong?**

Yes, in two ways.

**Executed attacks:** recorded here when run (§39.3).

## Residual Risk

Something.
";

    const WITH_ATTACKS: &str = r"# Assurance

## Gate Adequacy

**Adversarial question: could a malformed graph pass?**

- **outcome:** gate_added, no_counterexample_found

**Executed attacks:**
- planted a dangling stage_ref; refused by milestones.invalid
- planted a dependency cycle; refused by milestones.invalid
";

    #[test]
    fn a_placeholder_is_not_an_executed_attack() {
        let r = parse(WITH_PLACEHOLDER);
        assert!(r.present);
        assert!(!r.question.is_empty());
        assert!(
            !r.has_executed_attacks(),
            "'recorded here when run' is a plan, not evidence (§39.3)"
        );
        assert!(!r.has_outcome(), "no §39.2 outcome was recorded");
    }

    #[test]
    fn executed_attacks_and_outcomes_are_read() {
        let r = parse(WITH_ATTACKS);
        assert!(r.has_executed_attacks());
        assert_eq!(
            r.executed_attacks.len(),
            2,
            "the `**Executed attacks:**` label itself is not an attack: {:?}",
            r.executed_attacks
        );
        assert!(r.has_outcome());
        assert!(r.outcomes.contains(&AdequacyOutcome::GateAdded));
        assert!(r.outcomes.contains(&AdequacyOutcome::NoCounterexampleFound));
    }

    #[test]
    fn an_absent_section_is_absent() {
        let r = parse("# Assurance\n\n## Acceptance Obligations\n\nNothing.\n");
        assert!(!r.present);
    }

    /// Regression: OW-WAR-0023 writes `**Executed attacks:** none yet — …`.
    /// Reading that as an attack turned OFF the §39.3 warning for the one
    /// Warrant most honest about having run none. External review caught this
    /// on real corpus data after the check had already shipped in a commit.
    #[test]
    fn a_stated_absence_of_attacks_is_not_an_attack() {
        for body in [
            "**Executed attacks:** none yet — the capability model does not exist.",
            "**Executed attacks:** recorded here when run (§39.3).",
            "**Executed attacks:** none — nothing to attack.",
            "**Executed attacks:** no attacks have been run.",
            "**Executed attacks:** to be recorded.",
        ] {
            let r = parse(&format!(
                "## Gate Adequacy\n\n**Adversarial question: could this pass?**\n\n{body}\n"
            ));
            assert!(
                !r.has_executed_attacks(),
                "{body:?} states an absence; parsed {:?}",
                r.executed_attacks
            );
        }
    }

    /// The question detector must not accept ordinary prose. A bare `could`
    /// substring is the failure this module exists to replace, moved inside it.
    #[test]
    fn ordinary_prose_is_not_mistaken_for_the_adversarial_question() {
        let r = parse(
            "## Gate Adequacy\n\nThis section could be improved later. We could \
             also consider more controls.\n",
        );
        assert!(
            r.question.is_empty(),
            "accepted prose as the review's question: {:?}",
            r.question
        );
        assert_eq!(
            r.validate(AdequacyRequirement::BlindAdversarial, "controlled"),
            Err(AdequacyError::QuestionMissing)
        );
    }

    /// Both forms the corpus actually uses are still recognised.
    #[test]
    fn both_corpus_question_forms_are_recognised() {
        for q in [
            "**Adversarial question: could a Dispatch over-grant?**",
            "**Could this pass while adequacy is still theatre?**",
        ] {
            let r = parse(&format!("## Gate Adequacy\n\n{q}\n"));
            assert!(!r.question.is_empty(), "{q:?} not recognised");
        }
    }

    /// A heading that merely mentions the word does not open a review section.
    /// This is the substring failure that this module exists to replace, one
    /// layer down.
    #[test]
    fn a_heading_that_merely_mentions_the_word_is_not_a_review() {
        let r = parse(
            "# Assurance\n\n## Notes On Adequacy\n\nAdversarial question: could this pass?\n",
        );
        assert!(
            !r.present,
            "only the exact `## Gate Adequacy` heading opens a review section"
        );
    }

    /// §39.4's table, transcribed.
    #[test]
    fn the_requirement_table_matches_the_sas() {
        assert_eq!(
            AdequacyRequirement::for_level("basic"),
            AdequacyRequirement::Structural
        );
        assert_eq!(
            AdequacyRequirement::for_level("controlled"),
            AdequacyRequirement::BlindAdversarial
        );
        assert_eq!(
            AdequacyRequirement::for_level("high"),
            AdequacyRequirement::IndependentPlusNegativeControls
        );
        assert!(!AdequacyRequirement::Structural.requires_review());
        assert!(AdequacyRequirement::BlindAdversarial.requires_review());
        // §39.4 requires executed negative controls only at high assurance.
        assert!(!AdequacyRequirement::BlindAdversarial.requires_executed_controls());
        assert!(AdequacyRequirement::IndependentPlusNegativeControls.requires_executed_controls());
    }

    /// A controlled Warrant with no review section fails (§39.4).
    #[test]
    fn a_missing_review_fails_at_controlled() {
        let r = parse("# Assurance\n\n## Acceptance Obligations\n\nx\n");
        assert_eq!(
            r.validate(AdequacyRequirement::BlindAdversarial, "controlled"),
            Err(AdequacyError::ReviewMissing {
                level: "controlled".to_owned()
            })
        );
        // basic does not require one.
        assert_eq!(r.validate(AdequacyRequirement::Structural, "basic"), Ok(()));
    }

    #[test]
    fn a_review_without_a_question_fails() {
        let r = AdequacyReview {
            present: true,
            question: String::new(),
            ..AdequacyReview::default()
        };
        assert_eq!(
            r.validate(AdequacyRequirement::BlindAdversarial, "controlled"),
            Err(AdequacyError::QuestionMissing)
        );
    }

    /// §39.3 is a SHOULD. Missing executed attacks is reported by the caller,
    /// not failed here — a SHOULD turned into a hard failure is the first rule
    /// anyone disables.
    #[test]
    fn missing_executed_attacks_does_not_fail_validation_at_controlled() {
        let r = parse(WITH_PLACEHOLDER);
        assert_eq!(
            r.validate(AdequacyRequirement::BlindAdversarial, "controlled"),
            Ok(()),
            "§39.3 says SHOULD, where economical"
        );
        assert!(
            !r.has_executed_attacks(),
            "but the gap is visible to the caller"
        );
    }

    /// §39.2's vocabulary, transcribed as an external expectation.
    #[test]
    fn outcome_vocabulary_matches_the_sas() {
        assert_eq!(
            AdequacyOutcome::ALL
                .iter()
                .map(|o| o.as_str())
                .collect::<Vec<_>>(),
            [
                "counterexample_found",
                "no_counterexample_found",
                "obligation_narrowed",
                "gate_added",
                "gate_strengthened",
                "gap_accepted",
                "claim_removed",
                "review_not_required",
            ]
        );
    }

    /// The corpus as it stands: every controlled Warrant has a review section,
    /// and the executed-attacks gap is real and measurable.
    #[test]
    fn the_real_corpus_review_state_is_what_is_claimed() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/warrants");
        let mut controlled = 0;
        let mut with_attacks = 0;
        let mut missing_section = Vec::new();
        for entry in std::fs::read_dir(dir).expect("warrants") {
            let d = entry.expect("entry").path();
            let manifest = d.join("manifest.toml");
            let assurance = d.join("atoms/60-assurance.md");
            if !manifest.is_file() || !assurance.is_file() {
                continue;
            }
            let m = std::fs::read_to_string(&manifest).expect("readable");
            let level = m
                .lines()
                .find(|l| l.starts_with("assurance_level"))
                .and_then(|l| l.split('"').nth(1))
                .unwrap_or("basic")
                .to_owned();
            if AdequacyRequirement::for_level(&level).requires_review() {
                controlled += 1;
                let r = parse(&std::fs::read_to_string(&assurance).expect("readable"));
                if !r.present {
                    missing_section.push(d.file_name().unwrap().to_string_lossy().into_owned());
                }
                if r.has_executed_attacks() {
                    with_attacks += 1;
                }
            }
        }
        assert!(
            controlled >= 14,
            "expected the controlled Warrants, saw {controlled}"
        );
        assert!(
            missing_section.is_empty(),
            "controlled Warrants with no adequacy section: {missing_section:?}"
        );
        // Recorded rather than asserted as good: most reviews have executed no
        // attacks. That is the honest state and the reason OW-WAR-0018 exists.
        assert!(
            with_attacks < controlled,
            "if every controlled review had executed attacks this assertion should be \
             updated deliberately rather than silently passing"
        );
    }
}
