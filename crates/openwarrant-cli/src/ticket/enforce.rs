// SPDX-License-Identifier: Apache-2.0
//! What `war done` holds an item to, by level (t-3c16c W3, step 6).
//!
//! The level is [`crate::work::ticket_level`], derived here as everywhere and
//! stored nowhere. What the claim was given, has seen and declared is
//! [`super::live::standing`].
//!
//! | Level | A change it has not seen | Permissions widened since `war start` |
//! |---|---|---|
//! | Ticket | warns, and closes | nothing |
//! | Warrant | refused until seen; then its trigger runs | flagged and journalled |
//! | Order | as a Warrant | refused until a new signed revision exists |
//!
//! An Order is held to one thing more: what a person signed must still be
//! what stands. An Order nobody has signed yet is signed at its end, over
//! whatever it says then, so amending it takes nothing; a change to one that
//! is signed takes a new signature, which only a person can make.
//!
//! What cannot be read stays unknown (Law 15). A Ticket warns and closes,
//! since nothing but the agent's word decides it. A Warrant or an Order
//! answers `UNKNOWN` and stays open, and an item whose level cannot be read
//! could be either. A claim with nothing kept, because it was taken without
//! `war start`, was never promised a comparison: it warns, and says so.
//!
//! Every refusal comes before the first write of `war done`, and no refusal
//! here writes a record. The rule ids are stable; FORMAT-1.0 does not name
//! them yet, so they are not under `work.`.

use openwarrant_core::ticks;

use super::live::{self, Standing};
use super::{Outcome, Store, Ticket, ladder};
use crate::diagnostic::Diagnostic;
use crate::official;
use crate::preset::Policy;
use crate::repo::Repository;
use crate::work::{self, Level};

/// A change the claim has not seen: refused on a Warrant or an Order; on a
/// Ticket, the warning of the same name.
pub const UNSEEN: &str = "done.unseen";
/// A change it has seen, in a file its workspace does not hold: a trigger
/// run here would say nothing about the new state.
pub const BEHIND: &str = "done.behind";
/// A Warrant's permissions widened since `war start`: a warning.
pub const WIDENED: &str = "done.widened";
/// An Order's permissions widened, with no signed revision over them.
pub const ORDER_WIDENED: &str = "done.order-widened";
/// What was signed of an Order is not what stands.
pub const ORDER_AMENDED: &str = "done.order-amended";
/// The level could not be read: `UNKNOWN`.
pub const LEVEL_UNKNOWN: &str = "done.level-unknown";
/// Where the claim stands could not be read: `UNKNOWN` on a Warrant or an
/// Order; on a Ticket, the warning of the same name.
pub const SEEN_UNKNOWN: &str = "done.seen-unknown";
/// A Warrant or an Order worked without `war start`: a warning.
pub const UNRECORDED: &str = "done.unrecorded";

/// The journal event of a widening that was let through.
pub const WIDENED_EVENT: &str = "work.scope_widened";

/// What `war done` carries past the gate.
#[derive(Debug, Default)]
pub struct Gate {
    warnings: Vec<Diagnostic>,
    /// Permissions widened on a Warrant, by name.
    widened: Vec<String>,
    /// A change was seen during this claim: the trigger runs, whatever the
    /// caller asked for.
    pub rerun: bool,
}

fn checks_file(store: &Store, t: &Ticket) -> String {
    store.rel(&if t.format {
        t.checklist_path.clone()
    } else {
        t.dir.join(ticks::CHECKS_FILE)
    })
}

/// `docs/plan.md`, or `docs/plan.md (+2)`, and where the first of them is.
fn sources<'a>(mut names: impl Iterator<Item = &'a str>, count: usize) -> String {
    let first = names.next().unwrap_or_default();
    if count > 1 {
        format!("{first} (+{})", count - 1)
    } else {
        first.to_owned()
    }
}

/// Judge `t` for `what`, an item of it or all of it. `Err` is the answer,
/// and nothing has been written.
pub fn judge(store: &Store, t: &Ticket, what: &str) -> Result<Gate, Box<Outcome>> {
    let level = work::ticket_level(store, t);
    let id = t.id();
    if level == Level::Unknown {
        return Err(Box::new(Outcome::unknown(
            LEVEL_UNKNOWN,
            checks_file(store, t),
            format!(
                "{what}: its level could not be read, so it may have a trigger or need a \
                 signature; nothing was written. Fix {}, then `war done {what}`",
                checks_file(store, t)
            ),
        )));
    }
    let doc = store.rel(&t.checklist_path);
    let standing = live::standing(store, id);
    if level == Level::Ticket {
        return Ok(ticket_gate(standing, what, &doc));
    }
    let mut gate = Gate::default();
    let mut widened = Vec::new();
    match standing {
        Standing::Unread(why) => {
            return Err(Box::new(Outcome::unknown(
                SEEN_UNKNOWN,
                doc,
                format!(
                    "{what} is a {}: whether a change reached it could not be read ({why}); \
                     nothing was written. Fix that, then `war done {what}`",
                    level.as_str()
                ),
            )));
        }
        Standing::Unrecorded => gate.warnings.push(Diagnostic::warn(
            UNRECORDED,
            doc.clone(),
            format!(
                "{what} is a {} claimed without `war start`, so nothing records what it was \
                 given: changes and widenings since are UNKNOWN",
                level.as_str()
            ),
        )),
        Standing::Read(given) => {
            if let Some(first) = given.unseen.first() {
                let names = given.unseen.iter().map(|n| n.source.as_str());
                return Err(Box::new(Outcome::refused(
                    UNSEEN,
                    first.source.clone(),
                    format!(
                        "{what} is a {} and {} changed {} since it started; nothing was \
                         written. Run `war start {id}`, then `war done {what}`",
                        level.as_str(),
                        sources(names, given.unseen.len()),
                        first.place,
                    ),
                )));
            }
            if let Some(first) = given.behind.first() {
                let names = given.behind.iter().map(|t| t.source.as_str());
                return Err(Box::new(Outcome::refused(
                    BEHIND,
                    first.source.clone(),
                    format!(
                        "{what}: {} changed {} and this workspace does not hold it, so its \
                         trigger would run on the old state; nothing was written. Take the \
                         change in here, then `war done {what}`",
                        sources(names, given.behind.len()),
                        first.place,
                    ),
                )));
            }
            gate.rerun = !given.kept.seen.is_empty();
            let now = crate::go::packet::read_allowed(t);
            let (Some(was), Ok(now)) = (&given.kept.declared, &now) else {
                let why = now
                    .err()
                    .unwrap_or_else(|| "what it declared at `war start` was not kept".to_owned());
                return Err(Box::new(Outcome::unknown(
                    SEEN_UNKNOWN,
                    doc,
                    format!(
                        "{what}: whether its permissions widened could not be read ({why}); \
                         nothing was written. Fix that, or `war start {id}`, then `war done \
                         {what}`"
                    ),
                )));
            };
            widened = live::widened(was, now);
        }
    }
    if level == Level::Order {
        order_gate(store, t, what, &widened)?;
    } else if !widened.is_empty() {
        gate.warnings.push(Diagnostic::warn(
            WIDENED,
            doc,
            format!(
                "{what}: permissions widened after work started: {}; recorded in its journal",
                widened.join(", ")
            ),
        ));
        gate.widened = widened;
    }
    Ok(gate)
}

/// A Ticket closes on the agent's word: what it missed is only said.
fn ticket_gate(standing: Standing, what: &str, doc: &str) -> Gate {
    let mut gate = Gate::default();
    match standing {
        Standing::Unrecorded => {}
        Standing::Unread(why) => gate.warnings.push(Diagnostic::warn(
            SEEN_UNKNOWN,
            doc.to_owned(),
            format!("{what}: whether a change reached it is UNKNOWN ({why}); closing anyway"),
        )),
        Standing::Read(given) => {
            if let Some(first) = given.unseen.first() {
                let names = given.unseen.iter().map(|n| n.source.as_str());
                gate.warnings.push(Diagnostic::warn(
                    UNSEEN,
                    first.source.clone(),
                    format!(
                        "{what} closes with a change unseen: {} changed {}",
                        sources(names, given.unseen.len()),
                        first.place,
                    ),
                ));
            }
        }
    }
    gate
}

/// What a person would sign, for a refusal to carry.
fn revision_request(t: &Ticket, what: &str, widened: &[String]) -> serde_json::Value {
    serde_json::json!({
        "schema": "oh.war/order-revision-request/v1",
        "target": what,
        "command": format!("war sign approve {} --ssh-sign", t.id()),
        "digest": official::statement_digest(t),
        "statement": official::statement(t),
        "widened": widened,
    })
}

/// An Order: a widening needs a signature over it, and what was signed must
/// still stand.
fn order_gate(
    store: &Store,
    t: &Ticket,
    what: &str,
    widened: &[String],
) -> Result<(), Box<Outcome>> {
    let id = t.id();
    let doc = store.rel(&t.checklist_path);
    let repo = Repository::discover(Some(store.root.clone())).map_err(|e| {
        Box::new(Outcome::unknown(
            SEEN_UNKNOWN,
            doc.clone(),
            format!(
                "{what} is an Order: whether its signature stands could not be read ({e}); \
                 nothing was written"
            ),
        ))
    })?;
    let policy = Policy::read_or_default(&repo.root);
    let approval = official::approval(&repo, &policy, &official::light(t));
    let current = approval.as_ref().is_some_and(|a| a.counts);
    if !widened.is_empty() && !current {
        let mut refused = Outcome::refused(
            ORDER_WIDENED,
            doc,
            format!(
                "{what} is an Order and its permissions widened after work started: {}; nothing \
                 was written. It closes once a human signs a new revision: `war sign approve \
                 {id} --ssh-sign`",
                widened.join(", ")
            ),
        );
        refused.result = revision_request(t, what, widened);
        return Err(Box::new(refused));
    }
    if let Some(a) = approval.filter(|a| !a.counts) {
        let mut refused = Outcome::refused(
            ORDER_AMENDED,
            a.response.clone(),
            format!(
                "{what} is an Order and what was signed no longer stands; nothing was written. \
                 It closes once a human signs a new revision: `war sign approve {id} --ssh-sign`"
            ),
        );
        refused.result = revision_request(t, what, widened);
        refused.result["why"] = serde_json::json!(a.why);
        return Err(Box::new(refused));
    }
    let backing = ladder::backing(t);
    for item in &t.checklist.items {
        let key = ladder::item_key(item);
        if let Some(signed) = backing.signoff(&key)
            && signed.digest != ladder::statement_digest(t, &key, &item.text)
        {
            return Err(Box::new(Outcome::refused(
                ORDER_AMENDED,
                signed.response.clone(),
                format!(
                    "{what} is an Order and {id}/{key} changed after {} signed it off; nothing \
                     was written. It closes once a human signs it again: `war sign {id}/{key} \
                     --ssh-sign`",
                    signed.signer
                ),
            )));
        }
    }
    Ok(())
}

/// The tick landed: say what the gate let through, and journal a widening.
pub fn landed(
    store: &Store,
    t: &Ticket,
    what: &str,
    gate: Gate,
    out: &mut Outcome,
) -> Result<(), crate::repo::RepoError> {
    if !gate.widened.is_empty() {
        store.journal(
            t,
            WIDENED_EVENT,
            &serde_json::json!({"target": what, "widened": gate.widened}),
        )?;
        out.result["widened"] = serde_json::json!(gate.widened);
    }
    for w in gate.warnings {
        out.report.push(w);
    }
    Ok(())
}
