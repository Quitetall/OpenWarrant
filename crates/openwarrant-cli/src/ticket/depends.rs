// SPDX-License-Identifier: Apache-2.0
//! What a ticket waits on beyond its own repository (t-3c16c W6).
//!
//! `x.depends_on` names items and clauses, here (`CL-handle`) or in a named
//! repository (`hpr:t-abc`, `hpr:CL-handle`). It is an extension, listed in
//! `requires:` (FORMAT-1.0 7.3), because the target grammar of `depends_on`
//! waits for a signing sitting.
//!
//! A dependency in another repository is met when the *pin* holds it done.
//! What `origin` has done and this side has not pinned is the state the whole
//! workstream exists to name: *done upstream, not pinned here*. It is what
//! separates two release cadences. Each state is one line, said where the item
//! is listed, and nothing but a met dependency lets an item into `ready`.
//!
//! | State | Ready | Ticket at done | Warrant or Order at done |
//! |---|---|---|---|
//! | met | yes | closes | closes |
//! | open, undecided, done upstream | no | warns, closes | refused |
//! | UNKNOWN | no | warns, closes | UNKNOWN, stays open |
//!
//! Nothing here fetches. The cost falls only on a ticket that declares one.

use std::collections::BTreeMap;

use serde::Serialize;

use super::{Store, Ticket, checks, live};
use crate::interfaces;
use crate::repo::Repository;
use crate::repos;
use crate::work::Work;

/// The extension's key.
pub const KEY: &str = "x.depends_on";

/// Where one dependency stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Met,
    /// Not done, at the pin or upstream.
    Open,
    /// A clause not decided yet.
    Undecided,
    /// `origin` holds it; the pin does not.
    Upstream,
    /// It could not be read.
    Unknown,
}

/// One dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dep {
    pub target: String,
    pub state: State,
    /// What a reader needs, in one line.
    pub line: String,
}

/// Every ticket's dependencies that are not met, by ticket.
pub type Held = BTreeMap<String, Vec<Dep>>;

/// What a repository at one commit says of an id.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Found {
    Done,
    Open(String),
    Clause(interfaces::Clause),
    Absent,
}

impl Found {
    /// Done, or a clause settled enough to build against: a `tbr` is decided.
    fn settled(&self) -> bool {
        match self {
            Self::Done => true,
            Self::Clause(c) => !c.is_tbd(),
            Self::Open(_) | Self::Absent => false,
        }
    }
}

fn look(repo: &Repository, id: &str) -> Found {
    if let Some(c) = interfaces::clause(repo, id) {
        return Found::Clause(c);
    }
    let work = Work::read(repo);
    match work.items().iter().find(|i| i.id == id) {
        Some(i) if i.is_done() => Found::Done,
        Some(i) => Found::Open(i.state.clone()),
        None => Found::Absent,
    }
}

fn dep(target: &str, state: State, what: String) -> Dep {
    Dep {
        target: target.to_owned(),
        state,
        line: format!("{target}: {what}"),
    }
}

/// What the pin does not settle, from what this machine has fetched.
fn beyond_pin(target: &str, id: &str, located: &repos::Located, at_pin: &Found) -> Dep {
    let pin = repos::short(&located.pin.commit);
    let tip = repos::upstream_tip(&located.git_dir)
        .filter(|tip| !repos::holds(&located.git_dir, &located.pin.commit, tip));
    let upstream = tip
        .as_ref()
        .and_then(|tip| repos::read_at(located, tip, None, |r| look(r, id)).ok());
    match (at_pin, upstream.as_ref()) {
        (_, Some(found)) if found.settled() => {
            let word = if matches!(found, Found::Clause(_)) {
                "agreed"
            } else {
                "done"
            };
            let tip = tip.as_deref().map_or("?", repos::short);
            dep(
                target,
                State::Upstream,
                format!(
                    "{word} upstream, not pinned here (pin {pin}, upstream {tip}); move the pin"
                ),
            )
        }
        (Found::Clause(c), _) => dep(target, State::Undecided, c.standing()),
        (Found::Open(state), _) => dep(target, State::Open, format!("{state} at the pin {pin}")),
        (Found::Absent, Some(Found::Clause(c))) => dep(target, State::Undecided, c.standing()),
        (Found::Absent, Some(Found::Open(state))) => {
            dep(target, State::Open, format!("{state} upstream"))
        }
        (Found::Done | Found::Absent, _) => dep(
            target,
            State::Unknown,
            format!("UNKNOWN: no {id} at the pin {pin} or upstream"),
        ),
    }
}

fn foreign(root: &camino::Utf8Path, target: &str, name: &str, id: &str) -> Dep {
    let located = match repos::locate(root, name) {
        Ok(l) => l,
        Err(why) => return dep(target, State::Unknown, format!("UNKNOWN: {why}")),
    };
    let commit = located.pin.commit.clone();
    match repos::read_at(&located, &commit, None, |r| look(r, id)) {
        Ok(found) if found.settled() => Dep {
            target: target.to_owned(),
            state: State::Met,
            line: String::new(),
        },
        Ok(found) => beyond_pin(target, id, &located, &found),
        Err(why) => dep(target, State::Unknown, format!("UNKNOWN: {why}")),
    }
}

fn local(store: &Store, target: &str) -> Dep {
    let repo = match Repository::discover(Some(store.root.clone())) {
        Ok(r) => r,
        Err(why) => return dep(target, State::Unknown, format!("UNKNOWN: {why}")),
    };
    match look(&repo, target) {
        found if found.settled() => Dep {
            target: target.to_owned(),
            state: State::Met,
            line: String::new(),
        },
        Found::Clause(c) => dep(target, State::Undecided, c.standing()),
        Found::Open(state) => dep(target, State::Open, state),
        _ => dep(
            target,
            State::Unknown,
            "UNKNOWN: nothing here has that id".to_owned(),
        ),
    }
}

/// What `t` declares it depends on.
#[must_use]
pub fn targets(t: &Ticket) -> Vec<String> {
    if !t.format || !t.checklist_text.contains(KEY) {
        return Vec::new();
    }
    checks::declared(t.checklist_text.as_bytes(), KEY)
}

fn each(store: &Store, wanted: &[String]) -> Vec<Dep> {
    wanted
        .iter()
        .map(|target| match repos::classify(&store.root, target) {
            repos::Target::Local => local(store, target),
            repos::Target::Undeclared(name) => dep(
                target,
                State::Unknown,
                format!("UNKNOWN: {name} is not declared in [repos]"),
            ),
            repos::Target::Named(name, id) => foreign(&store.root, target, name, id),
        })
        .collect()
}

/// What a claim compares of `id` in a repository at one commit: the digest of
/// a clause's own text, or of an item's title and state; and the path that
/// holds it. `None`: it has no such clause or item.
#[must_use]
pub fn watched_text(repo: &Repository, id: &str) -> Option<(String, String)> {
    if let Some(c) = interfaces::clause(repo, id) {
        return Some((c.digest, c.source));
    }
    let work = Work::read(repo);
    let item = work.items().iter().find(|i| i.id == id)?;
    let text = format!("{}\n{}\n{}\n", item.id, item.title, item.state);
    Some((
        openwarrant_core::format::sha256(text.as_bytes()),
        item.place.clone(),
    ))
}

/// The clauses and items of named repositories `t` declares or cites
/// (`hpr:CL-step`, `hpr:t-abc`), as `war start` watches them: by what the pin
/// holds. One that cannot be read now is not watched: `war done` says so
/// through [`of`].
#[must_use]
pub fn watches(store: &Store, t: &Ticket) -> Vec<live::Watched> {
    let Ok(all) = repos::declared(&store.root) else {
        return Vec::new();
    };
    if all.is_empty() {
        return Vec::new();
    }
    let mut wanted = targets(t);
    // Any id the interface declares: a word is looked up, never shaped.
    for word in t
        .checklist_text
        .split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, ':' | '-' | '_' | '.')))
        .map(|w| w.trim_end_matches('.'))
    {
        let named = matches!(repos::classify(&store.root, word), repos::Target::Named(..));
        if named && !wanted.iter().any(|w| w == word) {
            wanted.push(word.to_owned());
        }
    }
    let mut out = Vec::new();
    for target in wanted {
        let repos::Target::Named(name, id) = repos::classify(&store.root, &target) else {
            continue;
        };
        let Ok((located, Some((digest, _)))) =
            repos::through_pin(&store.root, name, None, |r| watched_text(r, id))
        else {
            continue;
        };
        out.push(live::Watched {
            source: target.clone(),
            reach: live::PIN.to_owned(),
            because: target.clone(),
            known: vec![digest.clone()],
            file: digest,
            commit: Some(located.pin.commit),
        });
    }
    out
}

/// Every dependency of `t`, met or not.
#[must_use]
pub fn of(store: &Store, t: &Ticket) -> Vec<Dep> {
    let wanted = targets(t);
    if wanted.is_empty() {
        return Vec::new();
    }
    each(store, &wanted)
}

/// The tickets of `tickets` with a dependency not met, and those dependencies.
/// Open tickets only. A ticket that declares none costs a substring search.
#[must_use]
pub fn held(store: &Store, tickets: &[Ticket]) -> Held {
    let mut out = Held::new();
    for t in tickets.iter().filter(|t| !t.is_done()) {
        let waiting: Vec<Dep> = of(store, t)
            .into_iter()
            .filter(|d| d.state != State::Met)
            .collect();
        if !waiting.is_empty() {
            out.insert(t.id().to_owned(), waiting);
        }
    }
    out
}

/// The one line for what holds a ticket: its first, and how many more.
#[must_use]
pub fn one_line(waiting: &[Dep]) -> Option<String> {
    let first = waiting.first()?;
    Some(if waiting.len() > 1 {
        format!("{} (+{})", first.line, waiting.len() - 1)
    } else {
        first.line.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(line: &str) -> Dep {
        Dep {
            target: "t".to_owned(),
            state: State::Open,
            line: line.to_owned(),
        }
    }

    #[test]
    fn one_dependency_is_its_line_and_more_are_counted() {
        assert_eq!(one_line(&[]), None);
        assert_eq!(one_line(&[d("a")]).as_deref(), Some("a"));
        assert_eq!(
            one_line(&[d("a"), d("b"), d("c")]).as_deref(),
            Some("a (+2)")
        );
    }

    #[test]
    fn a_tbr_clause_is_decided_and_a_tbd_one_is_not() {
        let clause = |status: &str| {
            Found::Clause(interfaces::Clause {
                id: "CL-a".to_owned(),
                title: String::new(),
                status: status.to_owned(),
                verify: String::new(),
                check: None,
                owner: None,
                due: None,
                source: String::new(),
                line: 1,
                digest: String::new(),
            })
        };
        assert!(clause("agreed").settled());
        assert!(clause("tbr").settled());
        assert!(!clause("tbd").settled());
        assert!(!Found::Open("open".to_owned()).settled());
        assert!(!Found::Absent.settled());
    }
}
