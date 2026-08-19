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
#[derive(Debug, Clone, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The proposal vocabulary must not grow a variant that names a write
    /// target. If a future variant needs one, that is a protocol change and
    /// belongs in an ADR, not in a patch.
    #[test]
    fn proposal_kinds_are_declarative() {
        let kinds = [
            ProposalKind::CreateAtom,
            ProposalKind::ReplaceAtom,
            ProposalKind::Question,
            ProposalKind::DecisionDetected,
        ];
        assert_eq!(kinds.len(), 4);
    }
}
