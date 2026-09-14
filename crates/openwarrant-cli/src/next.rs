// SPDX-License-Identifier: Apache-2.0
//! `war next` — what should happen next in this repository, and whose act it
//! is.
//!
//! Two lists exist already: `war sign --list` (the human's acts) and the
//! corpus projection's `next_actionable` (stages an agent could execute).
//! Neither answers the question an agent asks first: "of everything pending,
//! which is MINE?" This merges them into one ordered list where every action
//! names its actor, and it is asserted over the whole table that no action
//! whose actor is `agent` is a signing act. An agent that reads this cannot be
//! told to authorize, resolve, accept or correct; it can only be told that a
//! human must, and how.

use serde::Serialize;

use crate::repo::{RepoError, Repository};
use crate::sign::{self, Pending};

pub const SCHEMA: &str = "oh.war/next/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    Human,
    Agent,
}

#[derive(Debug, Clone, Serialize)]
pub struct Action {
    pub actor: Actor,
    pub warrant: String,
    /// A short verb phrase: `authorize`, `resolve`, `correct`, `accept`,
    /// `check`, `deliver`, `verify`, `execute`.
    pub action: String,
    /// The command that does it, verbatim.
    pub command: String,
    pub why: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Next {
    pub schema: &'static str,
    pub actions: Vec<Action>,
    /// Why the list is empty, when it is. Never silently empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nothing: Option<String>,
}

/// The pure part: given the pending human acts and the corpus projection,
/// derive the table. Separated from I/O so the "an agent never signs"
/// invariant can be asserted on synthetic inputs.
#[must_use]
pub fn derive(pending: &[Pending], status: &openwarrant_core::status::CorpusStatus) -> Next {
    let mut actions = Vec::new();
    // Human acts first: they unblock the most.
    for p in pending {
        let (warrant, action, target, why) = match p {
            Pending::Authorize {
                alias,
                revision,
                amendment,
                ..
            } => (
                alias.clone(),
                "authorize",
                alias.clone(),
                format!(
                    "revision {revision} awaits authorization{}",
                    amendment
                        .as_ref()
                        .map(|a| format!(" under {}", a.id))
                        .unwrap_or_default()
                ),
            ),
            Pending::Resolve { alias, request, .. } => (
                alias.clone(),
                "resolve",
                alias.clone(),
                if request.would_resolve_satisfied == Some(true) {
                    "the thirteen are met and §38.6 would resolve satisfied".to_owned()
                } else {
                    format!(
                        "the thirteen are met; {} obligation(s) unestablished, so the outcome must be named",
                        request.unestablished.len()
                    )
                },
            ),
            Pending::Accept { version, .. } => (
                format!("SAS {version}"),
                "accept",
                version.clone(),
                "a proposed SAS revision awaits acceptance".to_owned(),
            ),
            Pending::Correct {
                alias,
                deliverable_id,
                request,
            } => (
                alias.clone(),
                "correct",
                format!("{alias}/{deliverable_id}"),
                format!("{} drifted after resolution", request.target_ref),
            ),
        };
        actions.push(Action {
            actor: Actor::Human,
            warrant,
            action: action.to_owned(),
            command: format!("war sign {target}"),
            why,
        });
    }
    // Agent acts: whatever stands between a Warrant and the human's next act.
    for w in &status.warrants {
        use openwarrant_core::status::WarrantRung as R;
        let already_human = actions
            .iter()
            .any(|a| a.actor == Actor::Human && a.warrant == w.alias);
        match w.rung {
            R::Invalid => actions.push(Action {
                actor: Actor::Agent,
                warrant: w.alias.clone(),
                action: "check".to_owned(),
                command: format!("war check {}", w.alias),
                why: "the manifest does not validate".to_owned(),
            }),
            // A pending human act on this Warrant is the unblocker; an agent
            // action beside it would be noise, so Draft is only an agent's when
            // no human act is pending.
            R::Draft if !already_human => {
                let unmet = w.unmet.join("; ");
                actions.push(Action {
                    actor: Actor::Agent,
                    warrant: w.alias.clone(),
                    action: "deliver".to_owned(),
                    command: format!("war evidence record {a} && war verify {a}", a = w.alias),
                    why: if unmet.is_empty() {
                        "not every §56.1 requirement is met".to_owned()
                    } else {
                        format!("unmet: {unmet}")
                    },
                });
            }
            R::ReadyToResolve if w.would_resolve_satisfied != Some(true) => {
                actions.push(Action {
                    actor: Actor::Agent,
                    warrant: w.alias.clone(),
                    action: "verify".to_owned(),
                    command: format!("war verify {}", w.alias),
                    why: "obligations remain unestablished; a blind verifier must dispose of them"
                        .to_owned(),
                });
            }
            _ => {}
        }
    }
    for s in &status.next_actionable {
        actions.push(Action {
            actor: Actor::Agent,
            warrant: s.warrant.clone(),
            action: "execute".to_owned(),
            command: format!("war dispatch {} {}", s.warrant, s.stage),
            why: format!("{} / {}: {}", s.milestone, s.stage, s.why),
        });
    }
    // `assert!`, not `debug_assert!`: this invariant is the point of the
    // command and must hold in a release-built test binary too.
    assert!(
        actions
            .iter()
            .all(|a| a.actor != Actor::Agent || !a.command.starts_with("war sign")),
        "an agent is never handed a signing act"
    );
    let nothing = if actions.is_empty() {
        Some(
            status
                .nothing_actionable
                .as_ref()
                .map(|n| format!("{n:?}"))
                .unwrap_or_else(|| {
                    "nothing awaits a signature and no stage is actionable".to_owned()
                }),
        )
    } else {
        None
    };
    Next {
        schema: SCHEMA,
        actions,
        nothing,
    }
}

pub fn run(repo: &Repository) -> Result<Next, RepoError> {
    let pending = sign::pending(repo)?;
    let status = crate::status::build(repo)?;
    Ok(derive(&pending, &status))
}

#[must_use]
pub fn render(n: &Next) -> String {
    let mut s = String::new();
    for a in &n.actions {
        s.push_str(&format!(
            "{:<6} {:<12} {:<10} {}\n{:>6} {}\n",
            match a.actor {
                Actor::Human => "HUMAN",
                Actor::Agent => "agent",
            },
            a.warrant,
            a.action,
            a.command,
            "",
            a.why
        ));
    }
    if let Some(why) = &n.nothing {
        s.push_str(&format!("nothing to do: {why}\n"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_agent_is_never_handed_a_signing_act_and_humans_come_first() {
        // The real corpus is the fixture: every kind of pending act exists in
        // it or in its history, and the invariant must hold over all of it.
        let root = camino::Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize_utf8()
            .unwrap();
        let repo = Repository::open(root).unwrap();
        let n = run(&repo).unwrap();
        assert!(!n.actions.is_empty());
        for a in &n.actions {
            if a.actor == Actor::Agent {
                assert!(
                    !a.command.starts_with("war sign"),
                    "agent handed a signature: {a:?}"
                );
                assert!(
                    !["authorize", "resolve", "accept", "correct"].contains(&a.action.as_str())
                );
            } else {
                assert!(a.command.starts_with("war sign"), "{a:?}");
            }
        }
        let first_agent = n.actions.iter().position(|a| a.actor == Actor::Agent);
        let last_human = n.actions.iter().rposition(|a| a.actor == Actor::Human);
        if let (Some(fa), Some(lh)) = (first_agent, last_human) {
            assert!(lh < fa, "human acts are listed before agent acts");
        }
    }
}
