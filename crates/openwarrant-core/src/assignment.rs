// SPDX-License-Identifier: Apache-2.0
//! Review assignment — who, among those the register already permits, takes
//! a Warrant's `verify` and `resolve` acts (OW-WAR-0137).
//!
//! # An assignment narrows; it never grants
//!
//! `docs/authority/roles.toml` is the only source of who may sign anything
//! (§27.4). An assignment is a per-Warrant `assignment.toml` that picks a
//! subset of those actors for an act. Every function here computes an
//! INTERSECTION with what the register already allows: an actor named here
//! who does not hold the role is refused by name, never added. If code ever
//! lets an assignment put an actor on an eligible list the register did not,
//! that is a defect, and [`narrow`] is where it would have to be written.
//!
//! # The authorizer signs who reviews
//!
//! The assignment's [`Assignment::digest`] is listed in the authorization
//! request and echoed by the signed response, the way the deliverable set is
//! (OW-ADR-0021). [`standing`] compares the file as it stands with the digest
//! the authorization recorded, so an edit — or a removal, which would WIDEN
//! who may sign — after authorization is a named finding, not a new rule.
//!
//! # Where this file lives
//!
//! Pure: values in, findings out, no I/O. It is compiled into the CLI crate
//! through a `#[path]` module in `authorize.rs` because OW-WAR-0137's declared
//! set does not include `openwarrant-core/src/lib.rs`; the imports name
//! `openwarrant_core::` for that reason. Registering it in core is a one-line
//! change for the Warrant that owns core's `lib.rs`.

use std::collections::BTreeSet;
use std::fmt;

use openwarrant_core::authority::{ActorKind, ActorRole, AuthorityRegister};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The record family an `assignment.toml` declares.
pub const SCHEMA: &str = "oh.war/assignment/v1";
/// The file, beside the Warrant's `manifest.toml`.
pub const FILE: &str = "assignment.toml";

/// The two acts an assignment may name. Authorization is deliberately absent:
/// the authorizer is who signs the assignment, so an assignment of the
/// authorizer would be signed by the actor it chooses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Act {
    Verify,
    Resolve,
}

impl Act {
    #[must_use]
    pub fn role(self) -> ActorRole {
        match self {
            Self::Verify => ActorRole::Verifier,
            Self::Resolve => ActorRole::Resolver,
        }
    }

    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Verify => "verify",
            Self::Resolve => "resolve",
        }
    }
}

/// `oh.war/assignment/v1`: per act, one or more actors. Unknown keys are
/// refused so that `authorize = [...]` or a misspelt act fails closed rather
/// than reading as "no assignment".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assignment {
    pub schema: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verify: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolve: Vec<String>,
}

impl Assignment {
    /// Parse and check the envelope. A file that names no actor for any act
    /// is refused: it would read as an assignment and narrow nothing.
    pub fn parse(text: &str) -> Result<Self, String> {
        let a: Self = toml::from_str(text).map_err(|e| e.to_string())?;
        if a.schema != SCHEMA {
            return Err(format!(
                "schema {:?} is not {SCHEMA:?}; an assignment of another shape is refused, \
                 not guessed at",
                a.schema
            ));
        }
        if a.verify.is_empty() && a.resolve.is_empty() {
            return Err("names no actor for `verify` or `resolve`".to_owned());
        }
        for (act, list) in [(Act::Verify, &a.verify), (Act::Resolve, &a.resolve)] {
            if list.iter().any(|n| n.trim().is_empty()) {
                return Err(format!("`{}` names an empty actor", act.word()));
            }
            let unique: BTreeSet<&String> = list.iter().collect();
            if unique.len() != list.len() {
                return Err(format!("`{}` names an actor twice", act.word()));
            }
        }
        Ok(a)
    }

    /// The actors assigned to `act`; empty when the act is not assigned.
    #[must_use]
    pub fn actors(&self, act: Act) -> &[String] {
        match act {
            Act::Verify => &self.verify,
            Act::Resolve => &self.resolve,
        }
    }

    /// Computed like the deliverable-set digest: JCS over the sorted
    /// `[act, actor]` pairs, sha256, `sha256:` prefixed. Comments, key order
    /// and list order do not move it; a changed actor does.
    #[must_use]
    pub fn digest(&self) -> String {
        let mut pairs: Vec<[&str; 2]> = self
            .verify
            .iter()
            .map(|a| ["verify", a.as_str()])
            .chain(self.resolve.iter().map(|a| ["resolve", a.as_str()]))
            .collect();
        pairs.sort_unstable();
        let jcs = serde_jcs::to_string(&pairs).unwrap_or_default();
        let hash = Sha256::digest(jcs.as_bytes());
        let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
        format!("sha256:{hex}")
    }
}

/// Why an assignment cannot be signed, by rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule: &'static str,
    pub message: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.rule, self.message)
    }
}

/// Validate every assigned actor against the register, before a signature.
///
/// Order per actor, strongest statement first: an agent named as resolver is
/// refused by KIND (granting it the role would not help, §27.2); the performer
/// named as its own verifier or resolver is refused as a self-act (§51.2); then
/// the role must be held. Every finding is returned, not only the first.
#[must_use]
pub fn validate(a: &Assignment, register: &AuthorityRegister, performer: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for act in [Act::Verify, Act::Resolve] {
        for actor in a.actors(act) {
            let entry = register.actor(actor);
            if act == Act::Resolve && entry.is_some_and(|e| e.actor_kind == ActorKind::Agent) {
                out.push(Finding {
                    rule: "assignment.agent-prohibited",
                    message: format!(
                        "{actor:?} is an agent and is named to resolve. §27.2: an agent SHALL \
                         NOT resolve a delivery, and an assignment cannot change that — granting \
                         the role would not either"
                    ),
                });
                continue;
            }
            if actor == performer {
                out.push(Finding {
                    rule: if act == Act::Verify {
                        "assignment.self-verify"
                    } else {
                        "assignment.self-act"
                    },
                    message: format!(
                        "{actor:?} is this Warrant's performer and is named to {} it (§51.2, \
                         RQ-053: the performer does not clear its own work)",
                        act.word()
                    ),
                });
                continue;
            }
            if !entry.is_some_and(|e| e.holds(act.role())) {
                out.push(Finding {
                    rule: "assignment.role-missing",
                    message: format!(
                        "{actor:?} is named to {} and {} in docs/authority/roles.toml. An \
                         assignment narrows who may act; it never grants a role",
                        act.word(),
                        if entry.is_some() {
                            format!("does not hold `{}`", act.role())
                        } else {
                            "has no entry".to_owned()
                        }
                    ),
                });
            }
        }
    }
    out
}

/// Where a Warrant's assignment stands against its authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Standing {
    /// No file and no recorded digest: today's behavior, unchanged.
    None,
    /// The file's digest is the one the authorization recorded.
    Signed(Assignment),
    /// A file exists and no authorization has been recorded yet: a proposal.
    Unsigned(Assignment),
    /// The authorization recorded one digest and the tree holds another — an
    /// edit, an addition after signing, or a removal (which would widen).
    Moved {
        recorded: Option<String>,
        current: Option<String>,
    },
    /// The file exists and does not parse.
    Malformed(String),
}

impl Standing {
    /// The narrowing in force for `act`: `None` when the act is not narrowed
    /// at all, `Some(list)` when only `list` may act, and `Err` when nobody
    /// may act until a human fixes the record.
    pub fn for_act(&self, act: Act) -> Result<Option<&[String]>, Finding> {
        match self {
            Self::None => Ok(None),
            Self::Signed(a) => Ok(Some(a.actors(act)).filter(|l| !l.is_empty())),
            Self::Unsigned(_) => Err(Finding {
                rule: "assignment.unsigned",
                message: format!(
                    "{FILE} exists and no authorization has signed it. An assignment takes \
                     effect when the authorizer signs its digest; authorize first"
                ),
            }),
            Self::Moved { recorded, current } => Err(Finding {
                rule: "assignment.moved",
                message: format!(
                    "{FILE} is {} and the authorization signed {}. Who reviews is what the \
                     authorizer signed: restore the file, or amend and re-authorize (§31). \
                     Nobody may {} until then",
                    current.as_deref().unwrap_or("absent"),
                    recorded.as_deref().unwrap_or("no assignment"),
                    act.word()
                ),
            }),
            Self::Malformed(why) => Err(Finding {
                rule: "assignment.malformed",
                message: format!("{FILE} does not parse: {why}"),
            }),
        }
    }
}

/// Compare the file with the authorization's record.
///
/// `current` is the parsed file (or its parse error), `None` when absent.
/// `authorized` is `None` when no authorization is recorded, `Some(None)`
/// when one is recorded without an assignment digest, `Some(Some(d))` when
/// it signed `d`.
#[must_use]
pub fn standing(
    current: Option<Result<Assignment, String>>,
    authorized: Option<Option<&str>>,
) -> Standing {
    match (current, authorized) {
        (Some(Err(why)), _) => Standing::Malformed(why),
        (None, None | Some(None)) => Standing::None,
        (Some(Ok(a)), None) => Standing::Unsigned(a),
        (Some(Ok(a)), Some(recorded)) => {
            let digest = a.digest();
            if recorded == Some(digest.as_str()) {
                Standing::Signed(a)
            } else {
                Standing::Moved {
                    recorded: recorded.map(str::to_owned),
                    current: Some(digest),
                }
            }
        }
        (None, Some(Some(recorded))) => Standing::Moved {
            recorded: Some(recorded.to_owned()),
            current: None,
        },
    }
}

/// The eligible list for an act after assignment, and who was dropped why.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Narrowed {
    /// Who may act now: `eligible ∩ assigned`, in `eligible`'s order.
    pub eligible: Vec<String>,
    /// Who the register alone would have allowed.
    pub before: Vec<String>,
    /// Assigned actors the register no longer permits (U-003). They are NOT
    /// replaced by anybody else.
    pub revoked: Vec<String>,
}

/// Intersect. The result is a subset of `eligible` by construction: it is
/// `eligible` filtered, and nothing is ever pushed from `assigned`.
#[must_use]
pub fn narrow(eligible: &[String], assigned: &[String]) -> Narrowed {
    Narrowed {
        eligible: eligible
            .iter()
            .filter(|e| assigned.contains(e))
            .cloned()
            .collect(),
        before: eligible.to_vec(),
        revoked: assigned
            .iter()
            .filter(|a| !eligible.contains(a))
            .cloned()
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwarrant_core::authority::RoleAssignment;

    fn person(actor: &str, kind: ActorKind, roles: &[ActorRole]) -> RoleAssignment {
        RoleAssignment {
            actor: actor.to_owned(),
            actor_kind: kind,
            roles: roles.iter().copied().collect(),
            assigned_by: "owner".to_owned(),
            effective_time: "2026-01-01T00:00:00Z".to_owned(),
            note: None,
            ssh_principal: None,
        }
    }

    fn register() -> AuthorityRegister {
        AuthorityRegister::new(vec![
            person(
                "Ada",
                ActorKind::Human,
                &[ActorRole::Resolver, ActorRole::Verifier],
            ),
            person(
                "Ben",
                ActorKind::Human,
                &[ActorRole::Resolver, ActorRole::Verifier],
            ),
            person(
                "claude",
                ActorKind::Agent,
                &[
                    ActorRole::Performer,
                    ActorRole::Resolver,
                    ActorRole::Verifier,
                ],
            ),
        ])
    }

    fn assignment(verify: &[&str], resolve: &[&str]) -> Assignment {
        Assignment {
            schema: SCHEMA.to_owned(),
            verify: verify.iter().map(|s| (*s).to_owned()).collect(),
            resolve: resolve.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    fn rules(f: &[Finding]) -> Vec<&'static str> {
        f.iter().map(|f| f.rule).collect()
    }

    #[test]
    fn a_valid_assignment_has_no_findings() {
        assert!(validate(&assignment(&["Ada"], &["Ben"]), &register(), "claude").is_empty());
    }

    #[test]
    fn an_actor_without_the_role_is_refused_not_granted() {
        let f = validate(&assignment(&["Cy"], &[]), &register(), "claude");
        assert_eq!(rules(&f), ["assignment.role-missing"]);
    }

    #[test]
    fn an_agent_resolver_is_refused_by_kind_even_holding_the_role() {
        let f = validate(&assignment(&[], &["claude"]), &register(), "someone");
        assert_eq!(rules(&f), ["assignment.agent-prohibited"]);
    }

    #[test]
    fn the_performer_cannot_be_its_own_verifier() {
        let f = validate(&assignment(&["claude"], &[]), &register(), "claude");
        assert_eq!(rules(&f), ["assignment.self-verify"]);
    }

    #[test]
    fn narrowing_never_adds_anyone() {
        let eligible = vec!["Ada".to_owned(), "Ben".to_owned()];
        let n = narrow(&eligible, &["Ben".to_owned()]);
        assert_eq!(n.eligible, ["Ben"]);
        // Cy holds no role: nobody is eligible, and Cy is not added.
        let n = narrow(&eligible, &["Cy".to_owned()]);
        assert!(n.eligible.is_empty());
        assert_eq!(n.revoked, ["Cy"]);
        for assigned in [vec![], vec!["Cy".to_owned(), "Ada".to_owned()]] {
            let n = narrow(&eligible, &assigned);
            assert!(n.eligible.iter().all(|e| eligible.contains(e)));
        }
    }

    #[test]
    fn the_digest_ignores_order_and_moves_with_an_actor() {
        let a = assignment(&["Ada", "Ben"], &["Ben"]);
        let b = assignment(&["Ben", "Ada"], &["Ben"]);
        assert_eq!(a.digest(), b.digest());
        assert_ne!(a.digest(), assignment(&["Ada"], &["Ben"]).digest());
        // Moving an actor between acts moves the digest.
        assert_ne!(
            assignment(&["Ada"], &["Ben"]).digest(),
            assignment(&["Ben"], &["Ada"]).digest()
        );
    }

    #[test]
    fn standing_names_every_edit_after_signing() {
        let a = assignment(&["Ada"], &["Ben"]);
        let d = a.digest();
        assert_eq!(standing(None, None), Standing::None);
        assert_eq!(standing(None, Some(None)), Standing::None);
        assert_eq!(
            standing(Some(Ok(a.clone())), Some(Some(&d))),
            Standing::Signed(a.clone())
        );
        assert!(matches!(
            standing(Some(Ok(a.clone())), None),
            Standing::Unsigned(_)
        ));
        // Added after an authorization that signed none.
        assert!(matches!(
            standing(Some(Ok(a.clone())), Some(None)),
            Standing::Moved { recorded: None, .. }
        ));
        // Removed after signing: this would WIDEN, so it must not read as None.
        assert!(matches!(
            standing(None, Some(Some(&d))),
            Standing::Moved { current: None, .. }
        ));
        let edited = assignment(&["Ben"], &["Ben"]);
        assert!(matches!(
            standing(Some(Ok(edited)), Some(Some(&d))),
            Standing::Moved { .. }
        ));
    }

    #[test]
    fn a_moved_or_unsigned_assignment_lets_nobody_act() {
        let a = assignment(&["Ada"], &[]);
        let moved = standing(None, Some(Some(&a.digest())));
        assert_eq!(
            moved.for_act(Act::Resolve).unwrap_err().rule,
            "assignment.moved"
        );
        let unsigned = standing(Some(Ok(a.clone())), None);
        assert_eq!(
            unsigned.for_act(Act::Verify).unwrap_err().rule,
            "assignment.unsigned"
        );
        // Signed, but only verify assigned: resolve is not narrowed.
        let signed = standing(Some(Ok(a.clone())), Some(Some(&a.digest())));
        assert_eq!(signed.for_act(Act::Resolve), Ok(None));
        assert_eq!(
            signed.for_act(Act::Verify),
            Ok(Some(&["Ada".to_owned()][..]))
        );
    }

    #[test]
    fn parse_refuses_what_it_cannot_read_as_an_assignment() {
        assert!(Assignment::parse("schema = \"oh.war/assignment/v1\"\n").is_err());
        assert!(
            Assignment::parse("schema = \"oh.war/assignment/v1\"\nauthorize = [\"Ada\"]\n")
                .is_err()
        );
        assert!(
            Assignment::parse("schema = \"oh.war/assignment/v1\"\nverify = [\"Ada\", \"Ada\"]\n")
                .is_err()
        );
        assert!(
            Assignment::parse("schema = \"oh.war/assignment/v1\"\nresolve = [\"Ben\"]\n").is_ok()
        );
    }
}
