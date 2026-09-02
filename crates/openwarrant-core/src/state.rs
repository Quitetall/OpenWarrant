// SPDX-License-Identifier: AGPL-3.0-or-later
//! The state model (SAS §24, RQ-032).
//!
//! §24 opens with "State SHALL be decomposed into independent dimensions", and
//! the decomposition is the whole point. A single status enum cannot express
//! §24.6's truthful combinations — "resolved, satisfied, but superseded" and
//! "resolved, satisfied, but annulled" are different facts about the same work,
//! and collapsing them loses the distinction between *this was replaced* and
//! *you may no longer rely on this*.
//!
//! §24.6 states the reason directly: "The original outcome remains because it
//! records what was concluded at the time. Standing records whether the
//! organization still permits reliance upon it."
//!
//! # Derived versus recorded
//!
//! There is nowhere to store a transition yet — the local journal is
//! OW-WAR-0031. Until it exists, a [`WarrantState`] carries [`Provenance`]
//! saying whether it was RECORDED or DERIVED from the record's shape. A derived
//! state presented as a recorded one would be a false claim about provenance,
//! which is the class of error this whole system exists to prevent.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StateError {
    #[error("unknown {dimension} {found:?}; expected one of {known}")]
    Unknown {
        dimension: &'static str,
        found: String,
        known: String,
    },
    #[error(
        "illegal transition {from} → {to}: {reason}. §24.7 defines the core \
         transitions and an illegal one fails closed rather than clamping to the \
         nearest legal state"
    )]
    IllegalTransition {
        from: Phase,
        to: Phase,
        reason: &'static str,
    },
    #[error("untruthful combination: {reason} (SAS §24.6)")]
    UntruthfulCombination { reason: String },
}

/// Build the `Unknown` error for a dimension.
fn unknown<T>(dimension: &'static str, found: &str, all: &[T]) -> StateError
where
    T: fmt::Display,
{
    StateError::Unknown {
        dimension,
        found: found.to_owned(),
        known: all
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

macro_rules! dimension {
    ($name:ident, $label:literal, { $($variant:ident => $text:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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
            type Err = StateError;
            fn from_str(s: &str) -> Result<Self, StateError> {
                Self::ALL
                    .iter()
                    .copied()
                    .find(|v| v.as_str() == s)
                    .ok_or_else(|| unknown($label, s, Self::ALL))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

dimension!(Phase, "phase", {
    Draft => "draft",
    Proposed => "proposed",
    Authorized => "authorized",
    Ready => "ready",
    Executing => "executing",
    Verifying => "verifying",
    Resolved => "resolved",
});

dimension!(ExecutionCondition, "execution condition", {
    Clear => "clear",
    Blocked => "blocked",
    Paused => "paused",
});

dimension!(CommonOutcome, "outcome", {
    None => "none",
    Satisfied => "satisfied",
    NotSatisfied => "not_satisfied",
    Falsified => "falsified",
    Rejected => "rejected",
    Withdrawn => "withdrawn",
    Cancelled => "cancelled",
    Inconclusive => "inconclusive",
});

dimension!(Currency, "currency", {
    Current => "current",
    Superseded => "superseded",
    Deprecated => "deprecated",
});

dimension!(ResolutionStanding, "resolution standing", {
    Valid => "valid",
    Disputed => "disputed",
    Annulled => "annulled",
});

/// Whether a state was read from a record or inferred from the record's shape.
///
/// Everything is `Derived` until the local journal exists (OW-WAR-0031).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provenance {
    /// Read from a journalled transition.
    Recorded,
    /// Inferred from the record's shape because nothing stores transitions yet.
    Derived,
}

impl fmt::Display for Provenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Recorded => "recorded",
            Self::Derived => "derived",
        })
    }
}

/// The five independent dimensions of §24, plus how we came to know them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarrantState {
    pub phase: Phase,
    pub condition: ExecutionCondition,
    pub outcome: CommonOutcome,
    pub currency: Currency,
    pub standing: ResolutionStanding,
    pub provenance: Provenance,
}

impl WarrantState {
    /// A newly drafted Warrant.
    #[must_use]
    pub const fn draft(provenance: Provenance) -> Self {
        Self {
            phase: Phase::Draft,
            condition: ExecutionCondition::Clear,
            outcome: CommonOutcome::None,
            currency: Currency::Current,
            standing: ResolutionStanding::Valid,
            provenance,
        }
    }

    /// A Warrant whose §56.2 resolution is on record and binds the current
    /// contract. `outcome` is the record's, never inferred.
    #[must_use]
    pub const fn resolved_recorded(outcome: CommonOutcome) -> Self {
        Self {
            phase: Phase::Resolved,
            condition: ExecutionCondition::Clear,
            outcome,
            currency: Currency::Current,
            standing: ResolutionStanding::Valid,
            provenance: Provenance::Recorded,
        }
    }

    /// Check §24.6's truthfulness rules.
    ///
    /// These are the combinations the SAS says are *incoherent*, not merely
    /// unusual. Anything §24.6 shows as valid must pass.
    pub fn validate(&self) -> Result<(), StateError> {
        // An outcome is what verification concluded, so it cannot exist before
        // there is a conclusion.
        if self.phase != Phase::Resolved && self.outcome != CommonOutcome::None {
            return Err(StateError::UntruthfulCombination {
                reason: format!(
                    "phase is {} but outcome is {}; an outcome records what was \
                     concluded, and nothing is concluded before resolution",
                    self.phase, self.outcome
                ),
            });
        }
        if self.phase == Phase::Resolved && self.outcome == CommonOutcome::None {
            return Err(StateError::UntruthfulCombination {
                reason: "phase is resolved but outcome is none; resolving is \
                         concluding something"
                    .to_owned(),
            });
        }
        // Standing is about reliance on a resolution, so it presupposes one.
        if self.phase != Phase::Resolved && self.standing != ResolutionStanding::Valid {
            return Err(StateError::UntruthfulCombination {
                reason: format!(
                    "standing is {} but the Warrant is not resolved; standing records \
                     whether reliance on a RESOLUTION is still permitted",
                    self.standing
                ),
            });
        }
        Ok(())
    }

    /// Apply a phase transition, enforcing §24.7.
    pub fn transition_to(&self, to: Phase) -> Result<Self, StateError> {
        let legal = matches!(
            (self.phase, to),
            (Phase::Draft, Phase::Proposed)
                | (Phase::Proposed, Phase::Authorized)
                | (Phase::Authorized, Phase::Ready)
                | (Phase::Ready, Phase::Executing)
                | (Phase::Executing, Phase::Verifying)
                | (Phase::Verifying, Phase::Resolved)
                // §24.8: a material amendment returns to authorized from any
                // pre-resolution phase.
                | (Phase::Ready, Phase::Authorized)
                | (Phase::Executing, Phase::Authorized)
                | (Phase::Verifying, Phase::Authorized)
        );
        if !legal {
            return Err(StateError::IllegalTransition {
                from: self.phase,
                to,
                reason: if self.phase == Phase::Resolved {
                    "a resolved Warrant does not re-enter the lifecycle; \
                     post-resolution events change standing or currency (§24.9)"
                } else {
                    "not a core transition in §24.7"
                },
            });
        }
        let mut next = *self;
        next.phase = to;
        // Returning to authorized clears any outcome that was in flight.
        if to == Phase::Authorized {
            next.outcome = CommonOutcome::None;
        }
        next.validate()?;
        Ok(next)
    }

    /// Block the Warrant. §24.7: blocking OVERLAYS the phase.
    ///
    /// §91.6 test 37 — blocking does not erase phase. Modelling `blocked` as a
    /// phase rather than a condition is the mistake this method exists to make
    /// impossible.
    #[must_use]
    pub const fn blocked(&self) -> Self {
        let mut next = *self;
        next.condition = ExecutionCondition::Blocked;
        next
    }

    /// Clear a block, returning to the same phase (§24.7).
    #[must_use]
    pub const fn unblocked(&self) -> Self {
        let mut next = *self;
        next.condition = ExecutionCondition::Clear;
        next
    }

    /// §24.8: a material amendment returns to authorized and requires
    /// re-preflight.
    pub fn material_amendment(&self) -> Result<Self, StateError> {
        if self.phase == Phase::Resolved {
            return Err(StateError::IllegalTransition {
                from: self.phase,
                to: Phase::Authorized,
                reason: "a resolved Warrant is amended by superseding it, not by \
                         returning it to authorized (§21, §56.6)",
            });
        }
        self.transition_to(Phase::Authorized)
    }

    /// §24.9: supersession changes currency and does NOT erase the outcome.
    #[must_use]
    pub const fn superseded(&self) -> Self {
        let mut next = *self;
        next.currency = Currency::Superseded;
        next
    }

    /// §24.9: annulment changes standing only.
    ///
    /// §91.6 test 41 — the historical outcome survives. What changes is whether
    /// the organization still permits reliance on it.
    #[must_use]
    pub const fn annulled(&self) -> Self {
        let mut next = *self;
        next.standing = ResolutionStanding::Annulled;
        next
    }

    /// §24.9: a dispute preserves the original resolution (§91.6 test 40).
    #[must_use]
    pub const fn disputed(&self) -> Self {
        let mut next = *self;
        next.standing = ResolutionStanding::Disputed;
        next
    }

    /// Whether this Warrant may still be relied upon for new work.
    #[must_use]
    pub const fn is_current_and_valid(&self) -> bool {
        matches!(self.currency, Currency::Current)
            && matches!(self.standing, ResolutionStanding::Valid)
    }
}

impl fmt::Display for WarrantState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}/{}/{}/{} ({})",
            self.phase, self.condition, self.outcome, self.currency, self.standing, self.provenance
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The vocabularies transcribed from §24.1–§24.5, as external expectations.
    #[test]
    fn vocabularies_match_the_sas() {
        assert_eq!(
            Phase::ALL.iter().map(|p| p.as_str()).collect::<Vec<_>>(),
            [
                "draft",
                "proposed",
                "authorized",
                "ready",
                "executing",
                "verifying",
                "resolved"
            ]
        );
        assert_eq!(
            ExecutionCondition::ALL
                .iter()
                .map(|c| c.as_str())
                .collect::<Vec<_>>(),
            ["clear", "blocked", "paused"]
        );
        assert_eq!(
            CommonOutcome::ALL
                .iter()
                .map(|o| o.as_str())
                .collect::<Vec<_>>(),
            [
                "none",
                "satisfied",
                "not_satisfied",
                "falsified",
                "rejected",
                "withdrawn",
                "cancelled",
                "inconclusive"
            ]
        );
        assert_eq!(
            Currency::ALL.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
            ["current", "superseded", "deprecated"]
        );
        assert_eq!(
            ResolutionStanding::ALL
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
            ["valid", "disputed", "annulled"]
        );
    }

    /// RQ-032: the dimensions are INDEPENDENT. None may be computed from another.
    #[test]
    fn the_dimensions_are_independent() {
        let base = WarrantState {
            phase: Phase::Resolved,
            condition: ExecutionCondition::Clear,
            outcome: CommonOutcome::Satisfied,
            currency: Currency::Current,
            standing: ResolutionStanding::Valid,
            provenance: Provenance::Derived,
        };
        // Same phase and outcome, three different currency/standing pairs — all
        // truthful per §24.6, and no two are the same state.
        let superseded = base.superseded();
        let disputed = base.disputed();
        let annulled = base.annulled();
        for s in [superseded, disputed, annulled] {
            assert_eq!(s.phase, base.phase, "phase must not move");
            assert_eq!(s.outcome, base.outcome, "outcome must not move");
            assert_eq!(s.validate(), Ok(()));
        }
        assert_ne!(superseded, disputed);
        assert_ne!(disputed, annulled);
    }

    /// §24.6's three worked examples must all validate.
    #[test]
    fn the_sas_truthful_combinations_validate() {
        let resolved = |currency, standing| WarrantState {
            phase: Phase::Resolved,
            condition: ExecutionCondition::Clear,
            outcome: CommonOutcome::Satisfied,
            currency,
            standing,
            provenance: Provenance::Derived,
        };
        // "Completed and later replaced"
        assert_eq!(
            resolved(Currency::Superseded, ResolutionStanding::Valid).validate(),
            Ok(())
        );
        // "Accepted and later challenged"
        assert_eq!(
            resolved(Currency::Current, ResolutionStanding::Disputed).validate(),
            Ok(())
        );
        // "Resolution invalidated"
        assert_eq!(
            resolved(Currency::Current, ResolutionStanding::Annulled).validate(),
            Ok(())
        );
    }

    #[test]
    fn the_core_transition_chain_is_legal() {
        let mut s = WarrantState::draft(Provenance::Derived);
        for to in [
            Phase::Proposed,
            Phase::Authorized,
            Phase::Ready,
            Phase::Executing,
            Phase::Verifying,
        ] {
            s = s.transition_to(to).expect("legal transition");
        }
        s.outcome = CommonOutcome::Satisfied;
        let resolved = WarrantState {
            phase: Phase::Verifying,
            ..s
        }
        .transition_to(Phase::Resolved);
        assert!(resolved.is_ok(), "verifying → resolved is legal");
    }

    // --- planted violations (§91.6) ---

    /// §91.6 test 36: an illegal transition fails closed.
    #[test]
    fn illegal_transitions_are_refused() {
        let draft = WarrantState::draft(Provenance::Derived);
        for to in [
            Phase::Ready,
            Phase::Executing,
            Phase::Verifying,
            Phase::Resolved,
        ] {
            assert!(
                draft.transition_to(to).is_err(),
                "draft → {to} must be refused, not clamped"
            );
        }
    }

    /// §91.6 test 37: blocking does not erase phase.
    #[test]
    fn blocking_does_not_erase_phase() {
        let executing = WarrantState {
            phase: Phase::Executing,
            ..WarrantState::draft(Provenance::Derived)
        };
        let blocked = executing.blocked();
        assert_eq!(blocked.phase, Phase::Executing, "phase survives blocking");
        assert_eq!(blocked.condition, ExecutionCondition::Blocked);
        assert_eq!(blocked.unblocked().phase, Phase::Executing);
    }

    /// §91.6 test 38: a material amendment returns to authorized.
    #[test]
    fn material_amendment_returns_to_authorized() {
        let executing = WarrantState {
            phase: Phase::Executing,
            ..WarrantState::draft(Provenance::Derived)
        };
        let amended = executing.material_amendment().expect("legal");
        assert_eq!(amended.phase, Phase::Authorized);
    }

    /// §91.6 test 41: annulment changes standing, not the historical outcome.
    #[test]
    fn annulment_does_not_erase_the_outcome() {
        let resolved = WarrantState {
            phase: Phase::Resolved,
            outcome: CommonOutcome::Satisfied,
            ..WarrantState::draft(Provenance::Derived)
        };
        let annulled = resolved.annulled();
        assert_eq!(
            annulled.outcome,
            CommonOutcome::Satisfied,
            "the original outcome records what was concluded at the time"
        );
        assert_eq!(annulled.standing, ResolutionStanding::Annulled);
        assert!(!annulled.is_current_and_valid());
    }

    #[test]
    fn a_resolved_warrant_does_not_re_enter_the_lifecycle() {
        let resolved = WarrantState {
            phase: Phase::Resolved,
            outcome: CommonOutcome::Satisfied,
            ..WarrantState::draft(Provenance::Derived)
        };
        assert!(resolved.transition_to(Phase::Executing).is_err());
        assert!(resolved.material_amendment().is_err());
    }

    #[test]
    fn untruthful_combinations_are_refused() {
        // An outcome before resolution.
        let early = WarrantState {
            phase: Phase::Executing,
            outcome: CommonOutcome::Satisfied,
            ..WarrantState::draft(Provenance::Derived)
        };
        assert!(matches!(
            early.validate(),
            Err(StateError::UntruthfulCombination { .. })
        ));

        // Resolved with no outcome.
        let empty = WarrantState {
            phase: Phase::Resolved,
            ..WarrantState::draft(Provenance::Derived)
        };
        assert!(empty.validate().is_err());

        // Standing without a resolution.
        let standing = WarrantState {
            phase: Phase::Draft,
            standing: ResolutionStanding::Disputed,
            ..WarrantState::draft(Provenance::Derived)
        };
        assert!(standing.validate().is_err());
    }

    #[test]
    fn unknown_values_are_refused_by_dimension() {
        assert!(matches!(
            Phase::from_str("finished"),
            Err(StateError::Unknown {
                dimension: "phase",
                ..
            })
        ));
        assert!(matches!(
            Currency::from_str("stale"),
            Err(StateError::Unknown {
                dimension: "currency",
                ..
            })
        ));
    }

    /// Provenance must be visible; a derived state must not read as recorded.
    #[test]
    fn provenance_is_carried_and_displayed() {
        let derived = WarrantState::draft(Provenance::Derived);
        assert!(derived.to_string().contains("derived"));
        let recorded = WarrantState::draft(Provenance::Recorded);
        assert!(recorded.to_string().contains("recorded"));
        assert_ne!(derived, recorded, "provenance is part of the state");
    }
}
