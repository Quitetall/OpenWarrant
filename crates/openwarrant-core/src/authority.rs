// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actor authority — who may exercise which role (SAS §27).
//!
//! # Two vocabularies share the word "role"
//!
//! [`crate::role`] names ATOM roles: what a piece of a Warrant is *for*
//! (`intent`, `basis`, `assurance`). This module names ACTOR roles: what a
//! person, agent or service is *entitled to do* (`performer`, `verifier`,
//! `authorizer`, `resolver`).
//!
//! They are unrelated vocabularies that collide on one English word, and
//! conflating them is how a system ends up letting a document's `assurance`
//! section authorize itself. They are kept in separate modules with separate
//! types so the compiler refuses the confusion.
//!
//! # §27.2 is a list of things that cannot be argued around
//!
//! An agent SHALL NOT authorize its own proposed WAR, accept organizational
//! residual risk, or resolve its own delivery. [`RoleAssignment::may_resolve`]
//! and [`RoleAssignment::may_authorize`] return a typed refusal rather than a
//! bool, so a caller that ignores the reason cannot accidentally read "no" as
//! "not applicable".
//!
//! # §27.3 is the ONLY automated-resolution path, and it has five conditions
//!
//! A separately identified policy service MAY resolve a basic mechanical WAR.
//! All five conditions in §27.3 are conjunctive and are checked here
//! individually, because the tempting shortcut — "it is basic, so a machine may
//! close it" — is four conditions short and would let a service resolve a
//! Warrant whose obligations were never mechanical.
//!
//! Condition one, "policy explicitly allows it", is deliberately NOT a property
//! of the actor: it is repository policy that a human sets once. An actor that
//! could grant itself the policy would be self-authorizing by another name.

use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub use crate::contract::ActorKind;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AuthorityError {
    #[error(
        "unknown actor role {found:?}; known roles are {known}. An unknown role fails \
         closed — authority is never inferred from a name nobody recognises"
    )]
    UnknownRole { found: String, known: String },
    #[error("a role assignment must name the actor it assigns (§27.4)")]
    MissingActor,
    #[error("a role assignment must record who assigned it (§27.4)")]
    MissingAssigner,
    #[error("a role assignment must grant at least one role")]
    NoRoles,
    #[error(
        "actor {actor:?} does not hold the {role} role, so it may not act in it \
         (§27.4: the system records the role ACTUALLY exercised)"
    )]
    RoleNotHeld { actor: String, role: ActorRole },
    #[error(
        "actor {actor:?} performed this work, so it may not {act} it \
         (§27.2: an agent SHALL NOT resolve its own delivery, and §51.2 forbids a \
         performer's own report satisfying an independent gate)"
    )]
    SelfAct { actor: String, act: &'static str },
    #[error(
        "actor {actor:?} is an agent, and an agent SHALL NOT {act} (§27.2). \
         §27.3 admits ONE exception — a separately identified policy service \
         resolving a basic mechanical Warrant — and it does not apply here: {why}"
    )]
    AgentProhibited {
        actor: String,
        act: &'static str,
        why: String,
    },
    #[error(
        "repository policy does not permit automated resolution (§27.3 condition 1). \
         This is set by a human in `openwarrant.toml`; an actor that could grant \
         itself this policy would be self-authorizing under another name"
    )]
    PolicyForbids,
}

/// What an actor is entitled to do (§27.1, §27.2).
///
/// Every variant maps to an act §27 names explicitly. There is no `admin` or
/// `owner` catch-all: a role that means "everything" defeats the point of
/// recording which one was exercised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorRole {
    /// Executes stages and produces artifacts (§27.1).
    Performer,
    /// Reviews artifacts independently (§46).
    Verifier,
    /// Authorizes a proposed contract revision (§28.4).
    Authorizer,
    /// Records a resolution (§56).
    Resolver,
    /// Accepts organizational residual risk (§58).
    RiskAcceptor,
    /// Records a judgment (§42). An agent may only *recommend* one (§27.1).
    Judge,
}

impl ActorRole {
    const ALL: [Self; 6] = [
        Self::Performer,
        Self::Verifier,
        Self::Authorizer,
        Self::Resolver,
        Self::RiskAcceptor,
        Self::Judge,
    ];

    #[must_use]
    pub fn known() -> String {
        Self::ALL
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl fmt::Display for ActorRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Performer => "performer",
            Self::Verifier => "verifier",
            Self::Authorizer => "authorizer",
            Self::Resolver => "resolver",
            Self::RiskAcceptor => "risk_acceptor",
            Self::Judge => "judge",
        })
    }
}

impl FromStr for ActorRole {
    type Err = AuthorityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "performer" => Ok(Self::Performer),
            "verifier" => Ok(Self::Verifier),
            "authorizer" => Ok(Self::Authorizer),
            "resolver" => Ok(Self::Resolver),
            "risk_acceptor" => Ok(Self::RiskAcceptor),
            "judge" => Ok(Self::Judge),
            other => Err(AuthorityError::UnknownRole {
                found: other.to_owned(),
                known: Self::known(),
            }),
        }
    }
}

/// The §27.3 conditions that are properties of the WORK rather than the actor.
///
/// Passed in by the caller because none of them can be read off a role
/// assignment: whether every obligation is mechanical is a fact about the
/// Warrant, and an actor that could assert it about itself would be deciding
/// its own eligibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyResolutionContext<'a> {
    /// §27.3 condition 1 — repository policy, set by a human.
    pub policy_allows: bool,
    /// The Warrant's assurance level. §27.3 admits `basic` only.
    pub assurance_level: &'a str,
    /// §27.3 condition 2 — every obligation is mechanically decidable.
    pub all_obligations_mechanical: bool,
    /// §27.3 condition 3 — no residual-risk judgment is required.
    pub residual_risk_judgment_required: bool,
}

/// One actor and the roles it holds (§27.4).
///
/// §27.4 permits one person to exercise several roles and REQUIRES the system to
/// record the role actually exercised — so this records what is *held*, and the
/// `may_*` methods are asked about a specific act. Holding a role is not
/// exercising it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleAssignment {
    pub actor: String,
    pub actor_kind: ActorKind,
    /// A set, not a list: holding a role twice means nothing, and ordering it
    /// keeps the serialized record stable for digesting.
    pub roles: BTreeSet<ActorRole>,
    /// §27.4 — authority has a source. An assignment nobody granted is not an
    /// assignment.
    pub assigned_by: String,
    pub effective_time: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl RoleAssignment {
    /// Structural validity, independent of any act.
    pub fn validate(&self) -> Result<(), AuthorityError> {
        if self.actor.trim().is_empty() {
            return Err(AuthorityError::MissingActor);
        }
        if self.assigned_by.trim().is_empty() {
            return Err(AuthorityError::MissingAssigner);
        }
        if self.roles.is_empty() {
            return Err(AuthorityError::NoRoles);
        }
        Ok(())
    }

    #[must_use]
    pub fn holds(&self, role: ActorRole) -> bool {
        self.roles.contains(&role)
    }

    fn require(&self, role: ActorRole) -> Result<(), AuthorityError> {
        if self.holds(role) {
            Ok(())
        } else {
            Err(AuthorityError::RoleNotHeld {
                actor: self.actor.clone(),
                role,
            })
        }
    }

    /// §28.4 with §27.2 — may this actor authorize a revision proposed by
    /// `proposer`?
    ///
    /// An agent authorizing its OWN proposal is the prohibition §27.2 opens
    /// with. An agent authorizing someone else's is not addressed by §27.2 and
    /// is refused here anyway: §27.1 lists what an agent may do and authorizing
    /// is absent from it, so permitting it would be reading a permission into
    /// silence.
    pub fn may_authorize(&self, proposer: &str) -> Result<(), AuthorityError> {
        self.validate()?;
        self.require(ActorRole::Authorizer)?;
        if self.actor_kind == ActorKind::Agent {
            return Err(AuthorityError::AgentProhibited {
                actor: self.actor.clone(),
                act: "authorize a proposed WAR",
                why: "§27.3 covers resolution only, never authorization".to_owned(),
            });
        }
        if self.actor == proposer {
            return Err(AuthorityError::SelfAct {
                actor: self.actor.clone(),
                act: "authorize",
            });
        }
        Ok(())
    }

    /// §58 with §27.2 — may this actor accept organizational residual risk?
    ///
    /// Flatly closed to agents. §27.2 names it in the prohibited list and §27.3
    /// grants no exception, so unlike resolution there is no conditional path.
    pub fn may_accept_residual_risk(&self) -> Result<(), AuthorityError> {
        self.validate()?;
        self.require(ActorRole::RiskAcceptor)?;
        if self.actor_kind == ActorKind::Agent {
            return Err(AuthorityError::AgentProhibited {
                actor: self.actor.clone(),
                act: "accept organizational residual risk",
                why: "§27.3's exception is for resolution of basic mechanical work, \
                      and risk acceptance is neither"
                    .to_owned(),
            });
        }
        Ok(())
    }

    /// §56 with §27.2 and §27.3 — may this actor resolve `performer`'s delivery?
    ///
    /// The five §27.3 conditions are checked one at a time and reported by name.
    /// A single combined bool would make "a service tried to close a controlled
    /// Warrant" and "a service tried to close a Warrant with a judgement in it"
    /// report identically, and the difference is the whole point of the clause.
    ///
    /// Condition 5 — "the meaning of resolution is explicit" — is not checked
    /// here because it is a property of the resolution RECORD, not of the actor;
    /// [`crate::resolution`] holds it. Splitting it out is deliberate: a role
    /// assignment cannot promise what a future record will say.
    pub fn may_resolve(
        &self,
        performer: &str,
        context: PolicyResolutionContext<'_>,
    ) -> Result<(), AuthorityError> {
        self.validate()?;
        self.require(ActorRole::Resolver)?;

        // §27.3 condition 4 applies to every actor kind, not only agents:
        // "performer and resolver identities are distinct". A human resolving
        // their own delivery is §27.2's "resolve its own delivery" as well.
        if self.actor == performer {
            return Err(AuthorityError::SelfAct {
                actor: self.actor.clone(),
                act: "resolve",
            });
        }

        if self.actor_kind == ActorKind::Human {
            return Ok(());
        }

        // §27.3's exception names "a separately identified policy service", not
        // an agent. §27.1 lists what an agent may do and resolution is absent,
        // so an agent resolving ANYONE's delivery is refused — reading a
        // permission into that silence is exactly the move §27.2 exists to stop.
        if self.actor_kind == ActorKind::Agent {
            return Err(AuthorityError::AgentProhibited {
                actor: self.actor.clone(),
                act: "resolve a delivery",
                why: "§27.3's exception is granted to a separately identified policy \
                      service, and this actor is an agent"
                    .to_owned(),
            });
        }

        // Everything below is §27.3's narrow exception, reached only by a policy
        // service, for which resolution is prohibited by default and permitted
        // only when all of these hold.
        if !context.policy_allows {
            return Err(AuthorityError::PolicyForbids);
        }
        if context.assurance_level != "basic" {
            return Err(AuthorityError::AgentProhibited {
                actor: self.actor.clone(),
                act: "resolve a delivery",
                why: format!(
                    "§27.3 admits basic Warrants only; this one is {}",
                    context.assurance_level
                ),
            });
        }
        if !context.all_obligations_mechanical {
            return Err(AuthorityError::AgentProhibited {
                actor: self.actor.clone(),
                act: "resolve a delivery",
                why: "§27.3 requires every obligation to be mechanical; at least one is not"
                    .to_owned(),
            });
        }
        if context.residual_risk_judgment_required {
            return Err(AuthorityError::AgentProhibited {
                actor: self.actor.clone(),
                act: "resolve a delivery",
                why: "§27.3 requires that no residual-risk judgment be needed; one is".to_owned(),
            });
        }
        Ok(())
    }
}

/// Every role assignment in force, and the parse failures alongside them.
#[derive(Debug, Default, Clone)]
pub struct AuthorityRegister {
    pub assignments: Vec<RoleAssignment>,
}

impl AuthorityRegister {
    #[must_use]
    pub fn new(assignments: Vec<RoleAssignment>) -> Self {
        Self { assignments }
    }

    /// The assignment for one actor, if any.
    #[must_use]
    pub fn actor(&self, name: &str) -> Option<&RoleAssignment> {
        self.assignments.iter().find(|a| a.actor == name)
    }

    /// Every actor holding `role`.
    pub fn holders(&self, role: ActorRole) -> impl Iterator<Item = &RoleAssignment> {
        self.assignments.iter().filter(move |a| a.holds(role))
    }

    /// §56.1 requirement 13 — is there an actor who may actually resolve this?
    ///
    /// Returns the first admissible resolver. `None` means no assigned actor
    /// passes [`RoleAssignment::may_resolve`], which is the fail-closed answer:
    /// a Warrant with nobody entitled to close it is not closeable, and saying
    /// so is more useful than a bare `false`.
    #[must_use]
    pub fn eligible_resolver(
        &self,
        performer: &str,
        context: PolicyResolutionContext<'_>,
    ) -> Option<&RoleAssignment> {
        self.assignments
            .iter()
            .find(|a| a.may_resolve(performer, context).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assignment(actor: &str, kind: ActorKind, roles: &[ActorRole]) -> RoleAssignment {
        RoleAssignment {
            actor: actor.to_owned(),
            actor_kind: kind,
            roles: roles.iter().copied().collect(),
            assigned_by: "owner".to_owned(),
            effective_time: "2026-08-25T00:00:00Z".to_owned(),
            note: None,
        }
    }

    fn permissive() -> PolicyResolutionContext<'static> {
        PolicyResolutionContext {
            policy_allows: true,
            assurance_level: "basic",
            all_obligations_mechanical: true,
            residual_risk_judgment_required: false,
        }
    }

    #[test]
    fn an_unknown_role_fails_closed() {
        assert!(matches!(
            "superuser".parse::<ActorRole>(),
            Err(AuthorityError::UnknownRole { .. })
        ));
    }

    #[test]
    fn role_names_round_trip_through_display() {
        for role in ActorRole::ALL {
            assert_eq!(
                role.to_string().parse::<ActorRole>().expect("parses"),
                role,
                "Display and FromStr must agree, or a written record cannot be read back"
            );
        }
    }

    #[test]
    fn holding_a_role_is_required_before_exercising_it() {
        let a = assignment("brian", ActorKind::Human, &[ActorRole::Performer]);
        assert!(matches!(
            a.may_resolve("claude", permissive()),
            Err(AuthorityError::RoleNotHeld { .. })
        ));
    }

    #[test]
    fn a_human_may_not_resolve_their_own_delivery() {
        let a = assignment("brian", ActorKind::Human, &[ActorRole::Resolver]);
        assert!(
            matches!(
                a.may_resolve("brian", permissive()),
                Err(AuthorityError::SelfAct { .. })
            ),
            "§27.3 condition 4 is about identities, not actor kinds"
        );
    }

    #[test]
    fn a_human_resolver_needs_no_policy_grant() {
        let a = assignment("brian", ActorKind::Human, &[ActorRole::Resolver]);
        let mut ctx = permissive();
        ctx.policy_allows = false;
        ctx.assurance_level = "high";
        ctx.all_obligations_mechanical = false;
        assert!(
            a.may_resolve("claude", ctx).is_ok(),
            "§27.3 constrains the policy-service exception, not human authority"
        );
    }

    #[test]
    fn an_agent_may_not_authorize_even_someone_elses_proposal() {
        let a = assignment("claude", ActorKind::Agent, &[ActorRole::Authorizer]);
        assert!(matches!(
            a.may_authorize("someone-else"),
            Err(AuthorityError::AgentProhibited { .. })
        ));
    }

    #[test]
    fn a_human_may_not_authorize_their_own_proposal() {
        let a = assignment("brian", ActorKind::Human, &[ActorRole::Authorizer]);
        assert!(matches!(
            a.may_authorize("brian"),
            Err(AuthorityError::SelfAct { .. })
        ));
    }

    #[test]
    fn an_agent_may_never_accept_residual_risk() {
        let a = assignment("claude", ActorKind::Agent, &[ActorRole::RiskAcceptor]);
        assert!(matches!(
            a.may_accept_residual_risk(),
            Err(AuthorityError::AgentProhibited { .. })
        ));
    }

    /// §27.3's five conditions are conjunctive. Each is removed alone from an
    /// otherwise-passing context, and each alone must refuse — a test that only
    /// checked the all-false case would pass against an implementation that
    /// looked at one condition.
    #[test]
    fn each_policy_service_condition_refuses_on_its_own() {
        let service = assignment(
            "policy-service",
            ActorKind::PolicyService,
            &[ActorRole::Resolver],
        );
        assert!(
            service.may_resolve("claude", permissive()).is_ok(),
            "the baseline context must pass, or the removals below prove nothing"
        );

        let mut no_policy = permissive();
        no_policy.policy_allows = false;
        assert!(matches!(
            service.may_resolve("claude", no_policy),
            Err(AuthorityError::PolicyForbids)
        ));

        let mut controlled = permissive();
        controlled.assurance_level = "controlled";
        assert!(matches!(
            service.may_resolve("claude", controlled),
            Err(AuthorityError::AgentProhibited { .. })
        ));

        let mut judgemental = permissive();
        judgemental.all_obligations_mechanical = false;
        assert!(matches!(
            service.may_resolve("claude", judgemental),
            Err(AuthorityError::AgentProhibited { .. })
        ));

        let mut risky = permissive();
        risky.residual_risk_judgment_required = true;
        assert!(matches!(
            service.may_resolve("claude", risky),
            Err(AuthorityError::AgentProhibited { .. })
        ));

        assert!(
            matches!(
                service.may_resolve("policy-service", permissive()),
                Err(AuthorityError::SelfAct { .. })
            ),
            "condition 4: performer and resolver identities must be distinct"
        );
    }

    #[test]
    fn an_assignment_granting_nothing_is_invalid() {
        let mut a = assignment("brian", ActorKind::Human, &[]);
        assert!(matches!(a.validate(), Err(AuthorityError::NoRoles)));
        a.roles.insert(ActorRole::Resolver);
        a.assigned_by = "  ".to_owned();
        assert!(matches!(a.validate(), Err(AuthorityError::MissingAssigner)));
    }

    #[test]
    fn an_empty_register_has_no_eligible_resolver() {
        let register = AuthorityRegister::default();
        assert!(
            register.eligible_resolver("claude", permissive()).is_none(),
            "no assignments means nobody may close anything, not that anybody may"
        );
    }

    #[test]
    fn the_register_finds_only_admissible_resolvers() {
        let register = AuthorityRegister::new(vec![
            assignment("claude", ActorKind::Agent, &[ActorRole::Performer]),
            assignment("brian", ActorKind::Human, &[ActorRole::Resolver]),
        ]);
        assert_eq!(
            register
                .eligible_resolver("claude", permissive())
                .map(|a| a.actor.as_str()),
            Some("brian")
        );
        assert!(
            register.eligible_resolver("brian", permissive()).is_none(),
            "the only resolver performed the work, so nobody is eligible"
        );
    }
}
