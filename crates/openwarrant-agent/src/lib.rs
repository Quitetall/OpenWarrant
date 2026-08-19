// SPDX-License-Identifier: AGPL-3.0-or-later
//! Agent-assisted planning protocol (SAS §74, §75).
//!
//! # Status: protocol surface only
//!
//! Phase 1 does not build the planner. This crate exists now so the boundary it
//! guards is visible from the first commit rather than negotiated later, and it
//! contains no adapter, no model provider, and no agent loop (§79.3).
//!
//! The boundary is §74.5: **an agent never mutates repository files directly.**
//! It returns a proposal; the proposal is validated against the schema and the
//! semantic rules; only then does the CLI write. Every capability in this crate
//! is shaped to make the alternative impossible to express — there is no type
//! here that carries a path to be written.
//!
//! `war plan` and the Katana drafter adapter arrive in Phase 2.

#![forbid(unsafe_code)]

/// What an agent is permitted to return from a planning turn (§74.2).
///
/// Modelled as data, never as an action. A proposal describes atoms it would
/// like to exist; it cannot describe a write, because §74.4 requires validation
/// before application and §74.5 forbids direct model mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProposalKind {
    /// Create a new authored atom in a role.
    CreateAtom,
    /// Replace the body of an existing authored atom.
    ReplaceAtom,
    /// Raise a question for the human rather than guessing (§74.6).
    Question,
    /// Flag that a normative decision was detected and needs an ADR (§74.7).
    DecisionDetected,
}

impl ProposalKind {
    /// Whether this variant describes only an intent, carrying no write target.
    ///
    /// The match is exhaustive on purpose. `ProposalKind` is `#[non_exhaustive]`
    /// for downstream crates, but inside this crate a new variant makes this
    /// function fail to compile until someone classifies it — which is the point.
    /// A variant that would answer `false` here is a protocol change and belongs
    /// in an ADR, not in a patch.
    #[must_use]
    pub const fn describes_only_intent(self) -> bool {
        match self {
            Self::CreateAtom | Self::ReplaceAtom | Self::Question | Self::DecisionDetected => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §74.5: an agent never mutates repository files directly.
    ///
    /// An earlier version of this test asserted only `kinds.len() == 4`, which
    /// would have permitted exactly the thing it claimed to guard: adding a
    /// `WriteFile` variant while deleting another keeps the count at four. The
    /// count proved nothing. This version routes every variant through an
    /// exhaustive classifier, so the guard cannot be satisfied by arithmetic.
    #[test]
    fn no_proposal_kind_names_a_write_target() {
        for kind in [
            ProposalKind::CreateAtom,
            ProposalKind::ReplaceAtom,
            ProposalKind::Question,
            ProposalKind::DecisionDetected,
        ] {
            assert!(
                kind.describes_only_intent(),
                "{kind:?} carries a write target; §74.5 forbids direct model mutation"
            );
        }
    }
}
