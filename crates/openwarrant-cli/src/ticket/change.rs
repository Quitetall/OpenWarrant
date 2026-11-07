// SPDX-License-Identifier: Apache-2.0
//! A change to an interface (t-3c16c W6): a ticket of type `change-request`
//! naming its clauses, classed by its effect.
//!
//! | Class | Level | What closes it |
//! |---|---|---|
//! | `compatible` | Warrant | its trigger: the named clauses' checks, or `--test` |
//! | `breaking` | Order | one sign-off per party, by different people |
//!
//! A breaking change has one checklist item per party of the interface, each
//! a sign-off (the existing Order path: `war sign <id>/<item> --ssh-sign`), so
//! a change one side cannot undo alone takes both owners. The level is derived
//! as ever: `x.change` raises the ticket's minimum, and no record holds a level.
//! Nothing here signs: `war done` names what a person would sign.
//!
//! `x.change` and `x.clauses` are extensions listed in `requires:`
//! (FORMAT-1.0 7.3) until a signing sitting promotes them.

use serde::Serialize;

use super::{Ticket, checks};
use crate::interfaces::{self, Clause};
use crate::repo::Repository;
use crate::repos;

/// The ticket type.
pub const TYPE: &str = "change-request";
/// `breaking` or `compatible`.
pub const CHANGE_KEY: &str = "x.change";
/// The clauses it changes.
pub const CLAUSES_KEY: &str = "x.clauses";

/// A change request's effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    Breaking,
    Compatible,
}

/// The class `t` declares, for a change request.
#[must_use]
pub fn class_of(t: &Ticket) -> Option<Class> {
    if t.manifest.kind.as_deref() != Some(TYPE) || !t.format {
        return None;
    }
    match checks::declared(t.checklist_text.as_bytes(), CHANGE_KEY)
        .first()
        .map(String::as_str)
    {
        Some("breaking") => Some(Class::Breaking),
        Some("compatible") => Some(Class::Compatible),
        _ => None,
    }
}

/// The clauses `t` names.
#[must_use]
pub fn clauses_of(t: &Ticket) -> Vec<String> {
    if t.manifest.kind.as_deref() != Some(TYPE) || !t.format {
        return Vec::new();
    }
    checks::declared(t.checklist_text.as_bytes(), CLAUSES_KEY)
}

/// What a request adds to the ticket it makes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Prepared {
    /// One item per party, for a breaking change.
    pub items: Vec<String>,
    /// The named clauses' checks, for a compatible one.
    pub tests: Vec<String>,
}

/// A clause and its interface's parties, here or at a pin.
fn find(repo: &Repository, target: &str) -> Result<(Clause, Vec<String>), String> {
    let lookup = |r: &Repository, id: &str| {
        interfaces::all(r).into_iter().find_map(|i| {
            let clause = i.clauses.iter().find(|c| c.id == id).cloned();
            clause.map(|c| (c, i.parties.clone()))
        })
    };
    let found = match repos::classify(&repo.root, target) {
        repos::Target::Local => lookup(repo, target),
        repos::Target::Undeclared(name) => {
            return Err(format!("{name} is not declared in [repos]"));
        }
        repos::Target::Named(name, id) => {
            repos::through_pin(&repo.root, name, None, |r| lookup(r, id))?.1
        }
    };
    found.ok_or_else(|| format!("no clause {target}"))
}

/// Check what `war create --type change-request` was given, and say what it
/// adds. `Err` is the refusal, with its rule.
pub fn prepare(
    repo: &Repository,
    change: Option<&str>,
    clauses: &[String],
    tests: &[String],
) -> Result<Prepared, (&'static str, String)> {
    let class = match change {
        Some("breaking") => Class::Breaking,
        Some("compatible") => Class::Compatible,
        Some(other) => {
            return Err((
                "change.class",
                format!("--change {other:?}: it is `breaking` or `compatible`"),
            ));
        }
        None => {
            return Err((
                "change.class",
                "a change request is classed: `--change breaking` or `--change compatible`"
                    .to_owned(),
            ));
        }
    };
    if clauses.is_empty() {
        return Err((
            "change.clauses",
            "a change request names its clauses: `--clause CL-step`".to_owned(),
        ));
    }
    let mut out = Prepared::default();
    let mut parties: Vec<String> = Vec::new();
    for target in clauses {
        let (clause, of) = find(repo, target).map_err(|why| ("change.clauses", why))?;
        for p in of {
            if !parties.contains(&p) {
                parties.push(p);
            }
        }
        out.tests.extend(clause.check);
    }
    match class {
        Class::Breaking => {
            if parties.len() < 2 {
                return Err((
                    "change.parties",
                    format!(
                        "a breaking change takes both owners, and its interface names {}",
                        if parties.is_empty() {
                            "no party".to_owned()
                        } else {
                            parties.join(", ")
                        }
                    ),
                ));
            }
            out.items = parties.iter().map(|p| format!("accept: {p}")).collect();
            out.tests.clear();
        }
        Class::Compatible => {
            if out.tests.is_empty() && tests.is_empty() {
                return Err((
                    "change.no-trigger",
                    "a compatible change is a Warrant, closed by a trigger: name a clause with a \
                     `check`, or pass `--test`"
                        .to_owned(),
                ));
            }
        }
    }
    Ok(out)
}

/// `Some(why)` when `signer` already signed another party's acceptance of the
/// breaking change `t`: both owners means two people.
#[must_use]
pub fn second_signature(t: &Ticket, item: &str, signer: &str) -> Option<String> {
    if class_of(t) != Some(Class::Breaking) {
        return None;
    }
    let backing = super::ladder::backing(t);
    t.checklist
        .items
        .iter()
        .map(super::ladder::item_key)
        .filter(|key| key != item)
        .find(|key| backing.signoff(key).is_some_and(|s| s.signer == signer))
        .map(|key| {
            format!(
                "{signer} signed {}/{key}; a breaking change takes a different person for each \
                 party",
                t.id()
            )
        })
}

/// One running claim a change to a clause reaches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reached {
    /// `here`, or the name of the repository.
    pub side: String,
    pub ticket: String,
    pub actor: String,
    pub clause: String,
    /// `depends_on`, `pin` or `manifest`.
    pub via: String,
}

/// The id a clause has wherever it is read: `CL-step` of `hpr:CL-step`.
fn bare(target: &str) -> &str {
    target.rsplit(':').next().unwrap_or(target)
}

/// The claims of the repository at `repo` that depend on `clause`.
fn claims_of(repo: &Repository, side: &str, clause: &str, out: &mut Vec<Reached>) {
    let Ok(store) = super::Store::open(repo, None) else {
        return;
    };
    let (tickets, _) = store.load_all().unwrap_or_default();
    let id = bare(clause);
    for (claim, kept) in super::live::held(&store) {
        let ticket = tickets.iter().find(|t| t.id() == claim.ticket);
        let declared =
            ticket.is_some_and(|t| super::depends::targets(t).iter().any(|d| bare(d) == id));
        let watched = kept.as_ref().is_some_and(|k| {
            k.watched
                .iter()
                .any(|w| w.reach == super::live::PIN && bare(&w.source) == id)
        });
        let given = kept.as_ref().is_some_and(|k| {
            k.source_manifest
                .iter()
                .any(|e| e.id == id || e.id.ends_with(&format!("/{id}")))
        });
        let via = if declared {
            "depends_on"
        } else if watched {
            "pin"
        } else if given {
            "manifest"
        } else {
            continue;
        };
        out.push(Reached {
            side: side.to_owned(),
            ticket: claim.ticket.clone(),
            actor: claim.actor.clone(),
            clause: clause.to_owned(),
            via: via.to_owned(),
        });
    }
}

/// What `record` (a change request, or a clause) reaches on both sides. The
/// second list names each side that could not be read: UNKNOWN, not nobody.
#[must_use]
pub fn across(repo: &Repository, record: &str) -> (Vec<Reached>, Vec<String>) {
    let clauses = super::Store::open(repo, None)
        .ok()
        .and_then(|s| s.load_all().ok())
        .and_then(|(tickets, _)| {
            tickets
                .iter()
                .find(|t| t.id() == record)
                .map(clauses_of)
                .filter(|c| !c.is_empty())
        })
        .unwrap_or_else(|| vec![record.to_owned()]);
    let mut out = Vec::new();
    let mut unread = Vec::new();
    for clause in &clauses {
        claims_of(repo, "here", clause, &mut out);
    }
    for name in repos::declared(&repo.root).unwrap_or_default().keys() {
        let Some(checkout) = repos::checkout(&repo.root, name) else {
            unread.push(format!("{name}: UNKNOWN, no checkout on this machine"));
            continue;
        };
        match Repository::open(checkout) {
            Ok(other) => {
                for clause in &clauses {
                    claims_of(&other, name, clause, &mut out);
                }
            }
            Err(e) => unread.push(format!("{name}: UNKNOWN, {e}")),
        }
    }
    out.sort_by(|a, b| (&a.side, &a.ticket, &a.clause).cmp(&(&b.side, &b.ticket, &b.clause)));
    out.dedup();
    (out, unread)
}

/// The lines `war plan impact` adds for what [`across`] found.
#[must_use]
pub fn render(reached: &[Reached], unread: &[String]) -> String {
    let mut s = String::new();
    if !reached.is_empty() || !unread.is_empty() {
        s.push_str("\nacross repositories:\n");
    }
    for r in reached {
        s.push_str(&format!(
            "  {}: {} ({}) reaches {} through {}\n",
            r.side, r.ticket, r.actor, r.clause, r.via
        ));
    }
    for u in unread {
        s.push_str(&format!("  {u}\n"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clause_is_the_same_clause_from_either_side() {
        assert_eq!(bare("hpr:CL-step"), "CL-step");
        assert_eq!(bare("CL-step"), "CL-step");
    }
}
