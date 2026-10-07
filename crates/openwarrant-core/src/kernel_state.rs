// SPDX-License-Identifier: Apache-2.0
//! States: a fixed kernel set plus declared refinements (OW-WAR-0148 M4;
//! OW-ADR-0031).
//!
//! # The fixed set
//!
//! `draft, open, in_progress, done, accepted, superseded, authorized,
//! verified, resolved, achieved`. Each is either **computed** from facts the
//! kernel already reads (a checklist, a claim, a relation, an exit Warrant's
//! resolution) or **authenticated**: it holds only on the record of a human
//! act or an independent verifier's ingested response. No profile adds a
//! fixed state, and none is ever entered by hand.
//!
//! Each fixed state is produced by one capability ([`FixedState::capability`]):
//! a record's type reaches it only when its profile selects that capability.
//! Each sits in one dimension of SAS §24 / RQ-032's decomposition
//! ([`FixedState::facet`]): the kernel states name moments in that
//! decomposition, they do not replace it. Several can hold at once, as the
//! dimensions do (an authorized Warrant can also be superseded).
//!
//! # Declared states
//!
//! A profile may declare `[[states]] name = "in_review", refines =
//! "in_progress"`. A declared state is a qualifier on its fixed parent:
//!
//! - it is entered by an authored, journaled event (`war state`);
//! - it holds only while its parent holds, and lapses when the parent stops;
//! - refining an authenticated state, it can be entered only while that state
//!   already holds — `signed_off` refines `verified`, and is never a way to
//!   read verified without the verifier;
//! - it never satisfies a §56.1 resolution check or a capability gate. Nothing
//!   that reads a check or a gate reads a declared state.

use std::fmt;

use crate::role::{Capabilities, Capability};

/// Computed, authenticated, or declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StateKind {
    /// Derived from facts the kernel reads.
    Computed,
    /// Holds only on the record of a human act or an ingested verdict.
    Authenticated,
    /// A profile's refinement of a fixed state, entered by an authored event.
    Declared,
}

impl StateKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Computed => "computed",
            Self::Authenticated => "authenticated",
            Self::Declared => "declared",
        }
    }
}

impl fmt::Display for StateKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One of the ten kernel states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FixedState {
    Draft,
    Open,
    InProgress,
    Done,
    Accepted,
    Superseded,
    Authorized,
    Verified,
    Resolved,
    Achieved,
}

impl FixedState {
    /// The closed set, in the order the plan names it.
    pub const ALL: [Self; 10] = [
        Self::Draft,
        Self::Open,
        Self::InProgress,
        Self::Done,
        Self::Accepted,
        Self::Superseded,
        Self::Authorized,
        Self::Verified,
        Self::Resolved,
        Self::Achieved,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Accepted => "accepted",
            Self::Superseded => "superseded",
            Self::Authorized => "authorized",
            Self::Verified => "verified",
            Self::Resolved => "resolved",
            Self::Achieved => "achieved",
        }
    }

    /// A fixed state by name, or `None` for a name outside the set.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.as_str() == name)
    }

    /// Computed or authenticated.
    #[must_use]
    pub const fn kind(self) -> StateKind {
        match self {
            Self::Accepted | Self::Authorized | Self::Verified | Self::Resolved => {
                StateKind::Authenticated
            }
            Self::Draft
            | Self::Open
            | Self::InProgress
            | Self::Done
            | Self::Superseded
            | Self::Achieved => StateKind::Computed,
        }
    }

    /// The capability that produces it: a type whose profile does not select
    /// it never reaches this state.
    #[must_use]
    pub const fn capability(self) -> Capability {
        match self {
            Self::Draft => Capability::Structure,
            Self::Open | Self::InProgress | Self::Done => Capability::Claims,
            Self::Accepted => Capability::Acceptance,
            Self::Superseded | Self::Achieved => Capability::Links,
            Self::Authorized => Capability::Authorization,
            Self::Verified => Capability::Verification,
            Self::Resolved => Capability::Resolution,
        }
    }

    /// The SAS §24 / RQ-032 dimension it is a moment of: `phase`,
    /// `outcome`, `currency` or `standing`. (`condition` — clear, blocked,
    /// paused — has no kernel state: blocking is a relation, not a state.)
    #[must_use]
    pub const fn facet(self) -> &'static str {
        match self {
            Self::Draft
            | Self::Open
            | Self::InProgress
            | Self::Done
            | Self::Authorized
            | Self::Resolved => "phase",
            Self::Verified | Self::Achieved => "outcome",
            Self::Superseded => "currency",
            Self::Accepted => "standing",
        }
    }
}

impl fmt::Display for FixedState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A profile's `[[states]]` entry, checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredState {
    pub name: String,
    pub refines: FixedState,
}

/// Why a `[[states]]` declaration was refused, each under its own rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarationError {
    /// `profile.state-refines-unknown`, `profile.state-collides`,
    /// `profile.state-unreachable` or `profile.state-invalid`.
    pub rule: &'static str,
    pub detail: String,
}

fn is_word(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Check a profile's `(name, refines)` declarations against its
/// capabilities. Refused: a `refines` outside the fixed set; a name that is a
/// fixed state's (a declared state can never be mistaken for one); a name
/// that is not a lowercase word, or is declared twice; and a refinement of a
/// fixed state the profile's capabilities never reach, which could never be
/// entered.
pub fn declare(
    entries: &[(String, String)],
    capabilities: Capabilities,
) -> Result<Vec<DeclaredState>, DeclarationError> {
    let mut out: Vec<DeclaredState> = Vec::new();
    for (name, refines) in entries {
        if FixedState::parse(name).is_some() {
            return Err(DeclarationError {
                rule: "profile.state-collides",
                detail: format!(
                    "declared state `{name}` is a fixed kernel state's name; a fixed state is \
                     computed or authenticated, and a profile can refine one, never redeclare it"
                ),
            });
        }
        if !is_word(name) {
            return Err(DeclarationError {
                rule: "profile.state-invalid",
                detail: format!("declared state {name:?} is not a lowercase word ([a-z][a-z0-9_-]*)"),
            });
        }
        if out.iter().any(|d| d.name == *name) {
            return Err(DeclarationError {
                rule: "profile.state-invalid",
                detail: format!("declared state `{name}` is declared twice"),
            });
        }
        let Some(parent) = FixedState::parse(refines) else {
            return Err(DeclarationError {
                rule: "profile.state-refines-unknown",
                detail: format!(
                    "declared state `{name}` refines {refines:?}, which is not a fixed state ({}); \
                     a declared state refines one of the kernel's",
                    FixedState::ALL.map(FixedState::as_str).join(", ")
                ),
            });
        };
        if !capabilities.has(parent.capability()) {
            return Err(DeclarationError {
                rule: "profile.state-unreachable",
                detail: format!(
                    "declared state `{name}` refines `{parent}`, which only the `{}` capability \
                     produces, and the profile does not select it: it could never be entered",
                    parent.capability()
                ),
            });
        }
        out.push(DeclaredState {
            name: name.clone(),
            refines: parent,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decl(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
            .collect()
    }

    #[test]
    fn every_fixed_state_has_a_kind_a_capability_and_a_facet() {
        for s in FixedState::ALL {
            assert_eq!(FixedState::parse(s.as_str()), Some(s));
            assert_ne!(s.kind(), StateKind::Declared);
            assert!(["phase", "outcome", "currency", "standing"].contains(&s.facet()));
        }
        assert_eq!(FixedState::Verified.kind(), StateKind::Authenticated);
        assert_eq!(FixedState::InProgress.kind(), StateKind::Computed);
    }

    #[test]
    fn a_refinement_is_admitted() {
        let d = declare(&decl(&[("in_review", "in_progress")]), Capabilities::WORKING).unwrap();
        assert_eq!(d[0].refines, FixedState::InProgress);
    }

    #[test]
    fn refusals_are_named() {
        let rule = |pairs: &[(&str, &str)], caps| declare(&decl(pairs), caps).unwrap_err().rule;
        assert_eq!(
            rule(&[("in_review", "reviewing")], Capabilities::ALL),
            "profile.state-refines-unknown"
        );
        assert_eq!(
            rule(&[("verified", "in_progress")], Capabilities::ALL),
            "profile.state-collides"
        );
        assert_eq!(
            rule(&[("signed_off", "verified")], Capabilities::WORKING),
            "profile.state-unreachable"
        );
        assert_eq!(
            rule(&[("a", "done"), ("a", "open")], Capabilities::ALL),
            "profile.state-invalid"
        );
        assert_eq!(rule(&[("In Review", "done")], Capabilities::ALL), "profile.state-invalid");
    }

    #[test]
    fn a_profile_file_carries_its_states() {
        use crate::role::ProfileRegistry;
        let lab = b"schema = \"oh.war/profile/v1\"\nname = \"lab\"\nextends = \"delivery\"\n\
            approved = false\n[[states]]\nname = \"signed_off\"\nrefines = \"verified\"\n";
        let r = ProfileRegistry::with_definitions([("profiles/lab.toml", lab.as_slice())]).unwrap();
        let p = r.resolve("lab").unwrap();
        let d = r.definition(&p).unwrap();
        assert_eq!(d.states[0].name, "signed_off");
        assert_eq!(d.states[0].refines, FixedState::Verified);

        let bad = b"schema = \"oh.war/profile/v1\"\nname = \"lab\"\nextends = \"delivery\"\n\
            approved = false\n[[states]]\nname = \"in_review\"\nrefines = \"reviewing\"\n";
        let e = ProfileRegistry::with_definitions([("profiles/lab.toml", bad.as_slice())])
            .unwrap_err();
        assert_eq!(e.rule(), "profile.state-refines-unknown");
        assert!(e.to_string().contains("\"reviewing\""), "{e}");
    }
}
