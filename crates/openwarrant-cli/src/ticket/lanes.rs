// SPDX-License-Identifier: Apache-2.0
//! Lanes (OW-ADR-0036): ordered paths of work that do not collide.
//!
//! A lane is a work item marked `war.lane: true`. Its `paths` are what it
//! owns; its items are `part_of` it and run in `depends_on` order, one
//! runner from start to end. A lane that `depends_on` another runs after it
//! and may share its files; two lanes running at the same time may not
//! (`lane.overlap`). `war done` on a lane item refuses a change outside the
//! lane's paths since the claim's base revision (`lane.outside-paths`),
//! before any write; `war lane widen` is the recorded way out.
//!
//! `war.lane` is critical (FORMAT-1.0 7.3): a lane document lists it in
//! `requires`. `war.for` (who the work suits) refuses nothing.

use std::collections::BTreeSet;

use camino::Utf8Path;
use openwarrant_core::format::FieldValue;
use openwarrant_core::ticket::Blocker;

use super::{CreateArgs, Outcome, Store, Ticket, authoring, scopes};
use crate::repo::RepoError;

/// The extension key that makes an item a lane.
pub const LANE_KEY: &str = "war.lane";
/// Who the work suits: an agent or a role.
pub const FOR_KEY: &str = "war.for";
/// The journal event of a widening.
pub const WIDENED: &str = "lane.widened";
/// Two lanes running at the same time whose paths overlap.
pub const OVERLAP: &str = "lane.overlap";
/// A lane item's work changed a file outside its lane's paths.
pub const OUTSIDE: &str = "lane.outside-paths";

/// The frontmatter of `t`, parsed alone; `None` for a document with none.
fn front(t: &Ticket) -> Option<openwarrant_core::format::Document> {
    if !t.format {
        return None;
    }
    let text = &t.checklist_text;
    let rest = text.strip_prefix("---\n")?;
    let end = "---\n".len() + rest.find("\n---\n")? + "\n---\n".len();
    Some(openwarrant_core::format::parse(&text.as_bytes()[..end]))
}

fn list(v: Option<&FieldValue>) -> Vec<String> {
    match v {
        Some(FieldValue::One(s)) => vec![s.clone()],
        Some(FieldValue::Many(v)) => v.clone(),
        None => Vec::new(),
    }
}

/// Whether `t` is a lane.
#[must_use]
pub fn is_lane(t: &Ticket) -> bool {
    t.format
        && t.checklist_text.contains(LANE_KEY)
        && front(t)
            .is_some_and(|d| matches!(d.ext.get(LANE_KEY), Some(FieldValue::One(v)) if v == "true"))
}

/// `war.for` of `t` itself.
#[must_use]
pub fn for_of(t: &Ticket) -> Option<String> {
    if !t.checklist_text.contains(FOR_KEY) {
        return None;
    }
    match front(t)?.ext.get(FOR_KEY) {
        Some(FieldValue::One(v)) if !v.trim().is_empty() => Some(v.trim().to_owned()),
        _ => None,
    }
}

/// One lane, as read.
#[derive(Debug, Clone)]
pub struct Lane<'a> {
    pub t: &'a Ticket,
    pub paths: Vec<String>,
    /// The lanes it runs after (its `depends_on` naming lanes), by id.
    pub after: Vec<String>,
    pub for_who: Option<String>,
}

impl Lane<'_> {
    #[must_use]
    pub fn id(&self) -> &str {
        self.t.id()
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.t.manifest.title
    }
}

/// Every lane in `tickets`, in id order.
#[must_use]
pub fn all(tickets: &[Ticket]) -> Vec<Lane<'_>> {
    let ids: BTreeSet<&str> = tickets
        .iter()
        .filter(|t| is_lane(t))
        .map(Ticket::id)
        .collect();
    let mut lanes: Vec<Lane<'_>> = tickets
        .iter()
        .filter(|t| ids.contains(t.id()))
        .map(|t| {
            let doc = front(t);
            Lane {
                t,
                paths: doc
                    .as_ref()
                    .map(|d| list(d.fields.get("paths")))
                    .unwrap_or_default(),
                after: t
                    .depends_on
                    .iter()
                    .filter_map(|b| match b {
                        Blocker::Ticket { ticket } if ids.contains(ticket.as_str()) => {
                            Some(ticket.clone())
                        }
                        _ => None,
                    })
                    .collect(),
                for_who: for_of(t),
            }
        })
        .collect();
    lanes.sort_by(|a, b| a.id().cmp(b.id()));
    lanes
}

/// The lane `query` names: its id, or its name (case aside).
pub fn find<'a, 'b>(lanes: &'b [Lane<'a>], query: &str) -> Result<&'b Lane<'a>, Box<Outcome>> {
    let q = query.trim();
    if let Some(l) = lanes.iter().find(|l| l.id() == q) {
        return Ok(l);
    }
    let named: Vec<&Lane<'_>> = lanes
        .iter()
        .filter(|l| l.name().eq_ignore_ascii_case(q))
        .collect();
    match named.as_slice() {
        [one] => Ok(one),
        [] => Err(Box::new(Outcome::refused(
            "lane.unknown",
            String::new(),
            format!(
                "no lane `{q}`; lanes: {}",
                if lanes.is_empty() {
                    "none (`war lane new <name> --paths <glob>`)".to_owned()
                } else {
                    lanes
                        .iter()
                        .map(|l| format!("{} ({})", l.name(), l.id()))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            ),
        ))),
        many => Err(Box::new(Outcome::refused(
            "lane.ambiguous",
            String::new(),
            format!(
                "`{q}` names {} lanes: {}; use the id",
                many.len(),
                many.iter().map(|l| l.id()).collect::<Vec<_>>().join(", ")
            ),
        ))),
    }
}

/// The lane `t` is an item of, when it is one.
#[must_use]
pub fn lane_of<'a, 'b>(lanes: &'b [Lane<'a>], t: &Ticket) -> Option<&'b Lane<'a>> {
    let parent = t.manifest.part_of.as_deref()?;
    lanes.iter().find(|l| l.id() == parent)
}

/// `members` in run order: each after the siblings it `depends_on`, ties by
/// creation then id. A cycle among them keeps the rest in that tie order.
#[must_use]
pub fn ordered<'a>(mut members: Vec<&'a Ticket>) -> Vec<&'a Ticket> {
    members.sort_by(|a, b| {
        (a.manifest.created_at.as_str(), a.id()).cmp(&(b.manifest.created_at.as_str(), b.id()))
    });
    let ids: BTreeSet<&str> = members.iter().map(|t| t.id()).collect();
    let before = |t: &Ticket| -> Vec<String> {
        t.depends_on
            .iter()
            .filter_map(|b| match b {
                Blocker::Ticket { ticket } | Blocker::ItemOf { ticket, .. }
                    if ids.contains(ticket.as_str()) && ticket != t.id() =>
                {
                    Some(ticket.clone())
                }
                _ => None,
            })
            .collect()
    };
    let mut placed: Vec<&'a Ticket> = Vec::with_capacity(members.len());
    let mut done: BTreeSet<String> = BTreeSet::new();
    while placed.len() < members.len() {
        let next = members
            .iter()
            .find(|m| !done.contains(m.id()) && before(m).iter().all(|b| done.contains(b)))
            .or_else(|| members.iter().find(|m| !done.contains(m.id())));
        let Some(m) = next else { break };
        done.insert(m.id().to_owned());
        placed.push(m);
    }
    placed
}

/// A lane is done when it has items and every one is done.
#[must_use]
pub fn done(members: &[&Ticket]) -> bool {
    !members.is_empty() && members.iter().all(|m| m.is_done())
}

/// Whether one of `a` and `b` runs after the other, through `after`.
#[must_use]
pub fn in_sequence(lanes: &[Lane<'_>], a: &str, b: &str) -> bool {
    a != b && (reaches(lanes, a, b) || reaches(lanes, b, a))
}

/// The first overlap of two path lists, said; `None` when disjoint. An
/// overlap that cannot be ruled out is one.
#[must_use]
pub fn overlap(a: &[String], b: &[String]) -> Option<String> {
    for x in a {
        for y in b {
            match scopes::overlaps(x, y) {
                Ok(false) => {}
                Ok(true) => return Some(format!("{x} overlaps {y}")),
                Err(why) => {
                    return Some(format!("{x} and {y}: overlap cannot be ruled out: {why}"));
                }
            }
        }
    }
    None
}

/// Every pair of lanes running at the same time (neither after the other,
/// neither done) whose paths overlap: `(a, b, why)`, ids in order.
#[must_use]
pub fn overlaps(tickets: &[Ticket], lanes: &[Lane<'_>]) -> Vec<(String, String, String)> {
    let live: Vec<&Lane<'_>> = lanes
        .iter()
        .filter(|l| !done(&super::children_of(tickets, l.id())))
        .collect();
    let mut out = Vec::new();
    for (i, a) in live.iter().enumerate() {
        for b in &live[i + 1..] {
            if in_sequence(lanes, a.id(), b.id()) {
                continue;
            }
            if let Some(why) = overlap(&a.paths, &b.paths) {
                out.push((a.id().to_owned(), b.id().to_owned(), why));
            }
        }
    }
    out
}

/// Whether the concrete repository path `path` matches `glob`: `**` crosses
/// directories (`**/` also matches none), `*` and `?` stay within one.
#[must_use]
pub fn matches(glob: &str, path: &str) -> bool {
    fn go(g: &[u8], p: &[u8]) -> bool {
        match g {
            [] => p.is_empty(),
            [b'*', b'*', b'/', rest @ ..] => {
                go(rest, p) || (0..p.len()).any(|i| p[i] == b'/' && go(rest, &p[i + 1..]))
            }
            [b'*', b'*', rest @ ..] => (0..=p.len()).any(|i| go(rest, &p[i..])),
            [b'*', rest @ ..] => (0..=p.len())
                .take_while(|&i| i == 0 || p[i - 1] != b'/')
                .any(|i| go(rest, &p[i..])),
            [b'?', rest @ ..] => p.first().is_some_and(|c| *c != b'/') && go(rest, &p[1..]),
            [c, rest @ ..] => p.first() == Some(c) && go(rest, &p[1..]),
        }
    }
    go(glob.as_bytes(), path.as_bytes())
}

/// Whether the item glob `g` provably lies inside one of `lane`: the same
/// glob, a literal path one matches, or under a literal `<dir>/**`. Anything
/// else cannot be shown inside, so it is not.
#[must_use]
pub fn inside(g: &str, lane: &[String]) -> bool {
    let wild = |s: &str| s.contains(['*', '?', '[', '{']);
    lane.iter().any(|l| {
        if l == g || l == "**" || (!wild(g) && matches(l, g)) {
            return true;
        }
        l.strip_suffix("/**")
            .is_some_and(|dir| !wild(dir) && g.starts_with(&format!("{dir}/")))
    })
}

/// The files outside `paths` among `changed`, leaving out what is exempt.
#[must_use]
pub fn outside(paths: &[String], changed: &[String], exempt: &[String]) -> Vec<String> {
    let mut out: Vec<String> = changed
        .iter()
        .filter(|f| !exempt.iter().any(|e| f.starts_with(e.as_str())))
        .filter(|f| !paths.iter().any(|g| matches(g, f)))
        .cloned()
        .collect();
    out.sort();
    out.dedup();
    out
}

fn normalized(store: &Store, paths: &[String]) -> Result<Vec<String>, String> {
    let mut out = paths
        .iter()
        .map(|p| scopes::normal(store, p))
        .collect::<Result<Vec<_>, _>>()?;
    out.sort();
    out.dedup();
    Ok(out)
}

/// The lane's paths as a claim holds them (a directory is `dir/**`).
fn lane_paths(store: &Store, lane: &Lane<'_>) -> Vec<String> {
    normalized(store, &lane.paths).unwrap_or_else(|_| lane.paths.clone())
}

// ---- readiness ---------------------------------------------------------

/// What an item of a lane waits on beyond itself: each lane its lane runs
/// after that is not done, by name. Nothing for an item outside a lane.
pub(super) fn waits<'a>(
    find: &dyn Fn(&str) -> Option<&'a Ticket>,
    members: &dyn Fn(&str) -> Vec<&'a Ticket>,
    t: &Ticket,
) -> Vec<String> {
    let Some(parent) = t.manifest.part_of.as_deref().and_then(find) else {
        return Vec::new();
    };
    if !is_lane(parent) {
        return Vec::new();
    }
    parent
        .depends_on
        .iter()
        .filter_map(|b| match b {
            Blocker::Ticket { ticket } => {
                let target = find(ticket)?;
                let finished = if is_lane(target) {
                    done(&members(ticket))
                } else {
                    target.is_done()
                };
                (!finished).then(|| format!("lane {} ({ticket})", target.manifest.title))
            }
            _ => None,
        })
        .collect()
}

/// Whether a `depends_on` on the record `tid` is met when `tid` is a lane:
/// `Some(done)` for a lane, `None` for anything else.
pub(super) fn lane_done<'a>(
    find: &dyn Fn(&str) -> Option<&'a Ticket>,
    members: &dyn Fn(&str) -> Vec<&'a Ticket>,
    tid: &str,
) -> Option<bool> {
    let t = find(tid)?;
    is_lane(t).then(|| done(&members(tid)))
}

/// A lane is done when it has items and every one is: derived on each read
/// of every ticket, never written, so an item added later opens it again.
pub(super) fn settle(tickets: &mut [Ticket]) {
    let finished: Vec<usize> = (0..tickets.len())
        .filter(|&i| {
            let t = &tickets[i];
            t.checklist.items.is_empty()
                && !t.done_whole
                && is_lane(t)
                && done(&super::children_of(tickets, t.id()))
        })
        .collect();
    for i in finished {
        tickets[i].done_whole = true;
    }
}

/// Whether the ready row of `ticket` suits `who`: its own `war.for`, else
/// its lane's.
#[must_use]
pub fn suits(tickets: &[Ticket], ticket: &str, who: &str) -> bool {
    let Some(t) = tickets.iter().find(|t| t.id() == ticket) else {
        return false;
    };
    let own = for_of(t).or_else(|| {
        let parent = t.manifest.part_of.as_deref()?;
        tickets
            .iter()
            .find(|p| p.id() == parent && is_lane(p))
            .and_then(for_of)
    });
    own.is_some_and(|f| f.eq_ignore_ascii_case(who.trim()))
}

// ---- commands ------------------------------------------------------------

/// `war lane new <name> --paths <glob>… [--for <who>] [--after <lane>…]`.
pub fn new(
    store: &Store,
    name: &str,
    paths: &[String],
    for_who: Option<&str>,
    after: &[String],
) -> Result<Outcome, RepoError> {
    let name = name.trim();
    if name.is_empty() {
        return Ok(Outcome::refused(
            "lane.name-empty",
            String::new(),
            "a lane needs a name",
        ));
    }
    let paths = match normalized(store, paths) {
        Ok(p) if !p.is_empty() => p,
        Ok(_) => {
            return Ok(Outcome::refused(
                "lane.paths-empty",
                String::new(),
                "a lane owns paths: `--paths <glob>…`",
            ));
        }
        Err(why) => return Ok(Outcome::refused("lane.paths-invalid", String::new(), why)),
    };
    let _admission = scopes::AdmissionLock::acquire(store)?;
    let (tickets, _) = store.load_all()?;
    let lanes = all(&tickets);
    if let Some(l) = lanes.iter().find(|l| l.name().eq_ignore_ascii_case(name)) {
        return Ok(Outcome::refused(
            "lane.exists",
            store.rel(&l.t.intent_path),
            format!("lane `{name}` is {}", l.id()),
        ));
    }
    let mut after_ids = Vec::new();
    for q in after {
        match find(&lanes, q) {
            Ok(l) => after_ids.push(l.id().to_owned()),
            Err(refusal) => return Ok(*refusal),
        }
    }
    // Its ancestors run before it; every other lane not done runs beside it.
    let ancestor = |id: &str| after_ids.iter().any(|a| reaches(&lanes, a, id));
    for l in &lanes {
        if ancestor(l.id()) || done(&super::children_of(&tickets, l.id())) {
            continue;
        }
        if let Some(why) = overlap(&paths, &lane_paths(store, l)) {
            return Ok(Outcome::refused(
                OVERLAP,
                store.rel(&l.t.intent_path),
                format!(
                    "lane `{name}` would overlap lane `{}` ({}), which runs at the same time: {why}. Use disjoint paths, or `--after {}`",
                    l.name(),
                    l.id(),
                    l.name()
                ),
            ));
        }
    }
    let args = CreateArgs {
        title: name.to_owned(),
        summary: None,
        items: Vec::new(),
        body: None,
        priority: None,
        kind: None,
        labels: Vec::new(),
        part_of: None,
        issue: None,
        scope: authoring::Scope {
            paths: paths.clone(),
            after: after_ids.clone(),
            lane: true,
            for_who: for_who.map(str::to_owned).filter(|s| !s.trim().is_empty()),
            ..authoring::Scope::default()
        },
        tests: Vec::new(),
        force: true,
    };
    let mut out = super::create(store, &args)?;
    if out.is_refused() {
        return Ok(out);
    }
    let id = out.result["id"].as_str().unwrap_or_default().to_owned();
    out.human = format!(
        "lane {name} {id}  paths {}{}\nnext: `war create \"...\" --lane {id}`",
        paths.join(", "),
        if after_ids.is_empty() {
            String::new()
        } else {
            format!("  after {}", after_ids.join(", "))
        }
    );
    out.result["lane"] =
        serde_json::json!({"name": name, "paths": paths, "after": after_ids, "for": for_who});
    Ok(out)
}

/// Whether `from` runs after `to`, directly or through other lanes.
fn reaches(lanes: &[Lane<'_>], from: &str, to: &str) -> bool {
    let mut seen = BTreeSet::new();
    let mut queue = vec![from.to_owned()];
    while let Some(id) = queue.pop() {
        if id == to {
            return true;
        }
        if !seen.insert(id.clone()) {
            continue;
        }
        if let Some(l) = lanes.iter().find(|l| l.id() == id) {
            queue.extend(l.after.iter().cloned());
        }
    }
    false
}

/// `war create … --lane <lane>`: the item goes last in the lane. Its own
/// paths, if any, must sit inside the lane's. `Err` is the refusal.
pub fn place(
    store: &Store,
    query: &str,
    args: &mut CreateArgs,
) -> Result<Result<(), Outcome>, RepoError> {
    let (tickets, _) = store.load_all()?;
    let lanes = all(&tickets);
    let lane = match find(&lanes, query) {
        Ok(l) => l,
        Err(refusal) => return Ok(Err(*refusal)),
    };
    if args.part_of.as_deref().is_some_and(|p| p != lane.id()) {
        return Ok(Err(Outcome::refused(
            "lane.part-of",
            String::new(),
            "an item in a lane is part of the lane; drop `--part-of`",
        )));
    }
    let owned = lane_paths(store, lane);
    let own = match normalized(store, &args.scope.paths) {
        Ok(p) => p,
        Err(why) => {
            return Ok(Err(Outcome::refused(
                "lane.paths-invalid",
                String::new(),
                why,
            )));
        }
    };
    if let Some(g) = own.iter().find(|g| !inside(g, &owned)) {
        return Ok(Err(Outcome::refused(
            "lane.item-outside",
            store.rel(&lane.t.intent_path),
            format!(
                "{g} is not inside lane `{}` ({}); widen the lane first (`war lane widen {} {g}`)",
                lane.name(),
                owned.join(", "),
                lane.name()
            ),
        )));
    }
    let members = ordered(super::children_of(&tickets, lane.id()));
    args.part_of = Some(lane.id().to_owned());
    if let Some(last) = members.last() {
        args.scope.after.push(last.id().to_owned());
    }
    Ok(Ok(()))
}

/// `war start --lane <lane>`: the lane's next ready item, claimed with the
/// lane's paths, and its brief. Holding one of its items, that one again.
pub fn start(store: &Store, query: &str) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let lanes = all(&tickets);
    let lane = match find(&lanes, query) {
        Ok(l) => l,
        Err(refusal) => return Ok(*refusal),
    };
    let members = ordered(super::children_of(&tickets, lane.id()));
    let name = lane.name().to_owned();
    let paths = lane_paths(store, lane);
    let held = super::held_by_me(store)?;
    let next = if let Some(m) = members
        .iter()
        .find(|m| !m.is_done() && held.iter().any(|h| h == m.id()))
    {
        m.id().to_owned()
    } else {
        let Some(m) = members.iter().find(|m| !m.is_done()) else {
            let said = if members.is_empty() {
                format!("lane {name} has no items: `war create \"...\" --lane {name}`")
            } else {
                format!("lane {name} is done")
            };
            return Ok(Outcome::ok(
                said,
                serde_json::json!({"schema": "oh.war/ticket-start/v1", "ticket": null, "lane": lane.id()}),
            ));
        };
        let rows = super::ready_rows(store, &tickets)?;
        if !rows.iter().any(|r| r.ticket == m.id()) {
            let lookup = super::Lookup::of(&tickets);
            let mut why: Vec<String> = match m.checklist.items.iter().find(|i| !i.done) {
                Some(item) => lookup.waiting_on(m, item),
                None => {
                    let by = super::first_by_id(&tickets);
                    super::document_blockers_by(
                        &|id| by.get(id).copied(),
                        &|id| super::children_of(&tickets, id),
                        m,
                    )
                }
            };
            if why.is_empty() {
                why.push(
                    "claimed, or its paths conflict with a live claim (`war view prime`)".into(),
                );
            }
            return Ok(Outcome::refused(
                "lane.waiting",
                store.rel(&m.intent_path),
                format!(
                    "lane {name}: next is {} ({}), not ready: waits on {}",
                    m.id(),
                    m.manifest.title,
                    why.join(", ")
                ),
            ));
        }
        let claimed = super::claim_scoped(store, m.id(), false, &paths)?;
        if claimed.is_refused() {
            return Ok(claimed);
        }
        m.id().to_owned()
    };
    let mut out = super::start_cmd(store, Some(&next))?;
    if !out.is_refused() {
        out.human = format!("lane {name}: {}\n{}", paths.join(", "), out.human);
        out.result["lane"] = serde_json::json!({"id": lane.id(), "name": name, "paths": paths});
    }
    Ok(out)
}

/// `war lane widen <lane> <glob>`: the glob joins the lane's paths, and
/// `lane.widened` is journalled with its actor. Refused, nothing written,
/// when the lane would then overlap a lane running at the same time.
pub fn widen(store: &Store, query: &str, glob: &str) -> Result<Outcome, RepoError> {
    let g = match scopes::normal(store, glob) {
        Ok(g) => g,
        Err(why) => return Ok(Outcome::refused("lane.paths-invalid", String::new(), why)),
    };
    let _admission = scopes::AdmissionLock::acquire(store)?;
    let (tickets, _) = store.load_all()?;
    let lanes = all(&tickets);
    let lane = match find(&lanes, query) {
        Ok(l) => l,
        Err(refusal) => return Ok(*refusal),
    };
    let name = lane.name().to_owned();
    if lane.paths.contains(&g) || lane_paths(store, lane).contains(&g) {
        return Ok(Outcome::ok(
            format!("lane {name} already holds {g}"),
            serde_json::json!({"schema": "oh.war/lane-widened/v1", "lane": lane.id(), "glob": g, "widened": false}),
        ));
    }
    for other in &lanes {
        if other.id() == lane.id()
            || in_sequence(&lanes, lane.id(), other.id())
            || done(&super::children_of(&tickets, other.id()))
        {
            continue;
        }
        if let Some(why) = overlap(std::slice::from_ref(&g), &lane_paths(store, other)) {
            return Ok(Outcome::refused(
                OVERLAP,
                store.rel(&other.t.intent_path),
                format!(
                    "widening lane {name} by {g} would overlap lane {} ({}), which runs at the same time: {why}; nothing was written",
                    other.name(),
                    other.id()
                ),
            ));
        }
    }
    let mut paths = lane.paths.clone();
    paths.push(g.clone());
    let mut changes = serde_json::Map::new();
    changes.insert("paths".into(), serde_json::json!(paths));
    let edited = authoring::edit(store, lane.t, &changes, None)?;
    if edited.is_refused() {
        return Ok(edited);
    }
    store.journal(
        lane.t,
        WIDENED,
        &serde_json::json!({"lane": lane.id(), "glob": g, "paths": paths}),
    )?;
    Ok(Outcome::ok(
        format!("lane {name}: + {g} (journalled {WIDENED})"),
        serde_json::json!({"schema": "oh.war/lane-widened/v1", "lane": lane.id(), "glob": g, "paths": paths, "widened": true}),
    ))
}

/// The files changed in `workspace` since `base`, committed or not,
/// tracked or untracked; `Err` when git could not say.
fn changed(workspace: &Utf8Path, base: &str) -> Result<Vec<String>, String> {
    let git = |args: &[&str]| -> Result<Vec<String>, String> {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(workspace)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stderr(std::process::Stdio::null())
            .output()
            .map_err(|e| format!("git: {e}"))?;
        if !out.status.success() {
            return Err(format!("`git {}` failed", args.join(" ")));
        }
        Ok(out
            .stdout
            .split(|b| *b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect())
    };
    let mut files = git(&["diff", "--name-only", "--no-renames", "-z", base])?;
    files.extend(git(&["ls-files", "--others", "--exclude-standard", "-z"])?);
    Ok(files)
}

/// `war done` on a lane item (before any write): a refusal naming each file
/// changed outside the lane's paths since the claim's base revision, or
/// `UNKNOWN` when that cannot be established. `None`: not a lane item, or
/// nothing outside.
pub(super) fn outside_refusal(
    store: &Store,
    tickets: &[Ticket],
    t: &Ticket,
    what: &str,
) -> Result<Option<Outcome>, RepoError> {
    if t.manifest.part_of.is_none() {
        return Ok(None);
    }
    let lanes = all(tickets);
    let Some(lane) = lane_of(&lanes, t) else {
        return Ok(None);
    };
    let name = lane.name().to_owned();
    let file = store.rel(&t.intent_path);
    let coordination = store
        .claims()?
        .values()
        .flatten()
        .filter(|c| c.ticket == t.id() && super::owns_claim(store, c))
        .find_map(|c| c.coordination.as_deref().cloned());
    let Some((base, workspace)) = coordination.and_then(|c| {
        let base = c.base_revision?;
        Some((base, camino::Utf8PathBuf::from(c.workspace)))
    }) else {
        return Ok(Some(Outcome::unknown(
            OUTSIDE,
            file,
            format!(
                "{what} is in lane {name}; no claim of yours records the revision it started from, so a change outside the lane's paths cannot be ruled out. Take it with `war start --lane {name}`"
            ),
        )));
    };
    let files = match changed(&workspace, &base) {
        Ok(f) => f,
        Err(why) => {
            return Ok(Some(Outcome::unknown(
                OUTSIDE,
                file,
                format!(
                    "{what}: what changed since {} is UNKNOWN: {why}",
                    &base[..base.len().min(12)]
                ),
            )));
        }
    };
    // The records `war` itself writes are never the lane's work.
    let mut exempt = vec![
        ".openwarrant/".to_owned(),
        format!("{}/", crate::subject::WORK_ROOT),
        format!("{}/", store.rel(&store.dir)),
    ];
    exempt.extend(tickets.iter().map(|x| store.rel(&x.intent_path)));
    let paths = lane_paths(store, lane);
    let out = outside(&paths, &files, &exempt);
    if out.is_empty() {
        return Ok(None);
    }
    Ok(Some(Outcome::refused(
        OUTSIDE,
        file,
        format!(
            "{what} is in lane {name} ({}); changed outside it since {}: {}. Undo those, or `war lane widen {name} <glob>` (journalled); nothing was written",
            paths.join(", "),
            &base[..base.len().min(12)],
            out.join(", ")
        ),
    )))
}

/// `war lane held`: the paths the edit guard allows, for the lanes whose
/// items this actor holds (and the records `war` writes); none when it
/// holds no lane item.
pub fn held(store: &Store) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let lanes = all(&tickets);
    let mine = super::held_by_me(store)?;
    let mut holding: Vec<serde_json::Value> = Vec::new();
    let mut paths: Vec<String> = Vec::new();
    for id in &mine {
        let Some(t) = tickets.iter().find(|t| t.id() == id) else {
            continue;
        };
        if let Some(lane) = lane_of(&lanes, t) {
            let p = lane_paths(store, lane);
            holding.push(
                serde_json::json!({"lane": lane.id(), "name": lane.name(), "item": id, "paths": p}),
            );
            paths.extend(p);
        }
    }
    if !paths.is_empty() {
        paths.extend([
            ".openwarrant/**".to_owned(),
            format!("{}/**", crate::subject::WORK_ROOT),
            format!("{}/**", store.rel(&store.dir)),
        ]);
    }
    paths.sort();
    paths.dedup();
    let human = if holding.is_empty() {
        "no lane item held".to_owned()
    } else {
        paths.join("\n")
    };
    Ok(Outcome::ok(
        human,
        serde_json::json!({"schema": "oh.war/lane-held/v1", "held": holding, "paths": paths}),
    ))
}

/// `war check`: every pair of lanes running at the same time whose paths
/// overlap (`lane.overlap`), and every lane item whose own paths are not
/// inside its lane's (`lane.item-outside`). A pass when there are lanes
/// and neither is found.
pub fn check(store: &Store, tickets: &[Ticket], report: &mut crate::diagnostic::Report) {
    use crate::diagnostic::Diagnostic;
    let lanes = all(tickets);
    if lanes.is_empty() {
        return;
    }
    let mut faults = 0;
    let named = |id: &str| {
        lanes
            .iter()
            .find(|l| l.id() == id)
            .map_or_else(|| id.to_owned(), |l| format!("{} ({id})", l.name()))
    };
    for (a, b, why) in overlaps(tickets, &lanes) {
        faults += 1;
        let file = lanes
            .iter()
            .find(|l| l.id() == b)
            .map(|l| store.rel(&l.t.intent_path))
            .unwrap_or_default();
        report.push(Diagnostic::error(
            OVERLAP,
            file,
            format!(
                "lanes {} and {} run at the same time and overlap: {why}. Make them disjoint, or put one after the other (`depends_on`)",
                named(&a),
                named(&b)
            ),
        ));
    }
    for t in tickets {
        let Some(lane) = lane_of(&lanes, t) else {
            continue;
        };
        let doc = front(t);
        let own = doc
            .as_ref()
            .map(|d| list(d.fields.get("paths")))
            .unwrap_or_default();
        let owned = lane_paths(store, lane);
        let own = normalized(store, &own).unwrap_or(own);
        if let Some(g) = own.iter().find(|g| !inside(g, &owned)) {
            faults += 1;
            report.push(Diagnostic::error(
                "lane.item-outside",
                store.rel(&t.intent_path),
                format!(
                    "{}: {g} is not inside lane {} ({})",
                    t.id(),
                    lane.name(),
                    owned.join(", ")
                ),
            ));
        }
    }
    if faults == 0 {
        report.push(Diagnostic::pass(
            "lane.disjoint",
            format!(
                "{} lane(s): none running at the same time overlaps another",
                lanes.len()
            ),
        ));
    }
}

// ---- views -----------------------------------------------------------------

/// One lane in `war plan frontier` and `war status`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Row {
    pub lane: String,
    pub name: String,
    pub paths: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<String>,
    #[serde(rename = "for", skip_serializing_if = "Option::is_none")]
    pub for_who: Option<String>,
    /// `done`, `empty`, or the next item's id.
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    pub items: usize,
    pub done: usize,
}

/// An item outside every lane that waits on two or more lanes (or their
/// items): a gate, a tag, a release.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Join {
    pub id: String,
    pub title: String,
    pub lanes: Vec<String>,
    pub done: bool,
}

/// Every lane and every join.
#[must_use]
pub fn rows(tickets: &[Ticket]) -> (Vec<Row>, Vec<Join>) {
    let lanes = all(tickets);
    let rows = lanes
        .iter()
        .map(|l| {
            let members = ordered(super::children_of(tickets, l.id()));
            let next = members
                .iter()
                .find(|m| !m.is_done())
                .map(|m| m.id().to_owned());
            let finished = members.iter().filter(|m| m.is_done()).count();
            Row {
                lane: l.id().to_owned(),
                name: l.name().to_owned(),
                paths: l.paths.clone(),
                after: l.after.clone(),
                for_who: l.for_who.clone(),
                state: if members.is_empty() {
                    "empty".into()
                } else if next.is_none() {
                    "done".into()
                } else {
                    "open".into()
                },
                next,
                items: members.len(),
                done: finished,
            }
        })
        .collect();
    let lane_ids: BTreeSet<&str> = lanes.iter().map(Lane::id).collect();
    let lane_of_id = |id: &str| -> Option<String> {
        if lane_ids.contains(id) {
            return Some(id.to_owned());
        }
        let t = tickets.iter().find(|t| t.id() == id)?;
        t.manifest
            .part_of
            .as_deref()
            .filter(|p| lane_ids.contains(p))
            .map(str::to_owned)
    };
    let mut joins = Vec::new();
    for t in tickets {
        if t.manifest
            .part_of
            .as_deref()
            .is_some_and(|p| lane_ids.contains(p))
            || lane_ids.contains(t.id())
        {
            continue;
        }
        let mut targets: Vec<String> = t
            .depends_on
            .iter()
            .filter_map(|b| match b {
                Blocker::Ticket { ticket } | Blocker::ItemOf { ticket, .. } => Some(ticket.clone()),
                Blocker::Item { .. } => None,
            })
            .collect();
        targets.extend(super::depends::targets(t));
        let mut joined: Vec<String> = targets
            .iter()
            .filter_map(|x| lane_of_id(x.split('/').next().unwrap_or(x)))
            .collect();
        joined.sort();
        joined.dedup();
        if joined.len() >= 2 {
            joins.push(Join {
                id: t.id().to_owned(),
                title: t.manifest.title.clone(),
                lanes: joined,
                done: t.is_done(),
            });
        }
    }
    (rows, joins)
}

/// The lines `war status` and `war plan frontier` print for them.
#[must_use]
pub fn lines(rows: &[Row], joins: &[Join]) -> Vec<String> {
    let mut out = Vec::new();
    for r in rows {
        let mut line = format!("lane {} {}  {}/{}", r.name, r.lane, r.done, r.items);
        match &r.next {
            Some(n) => line.push_str(&format!("  next {n}  `war start --lane {}`", r.name)),
            None => line.push_str(&format!("  {}", r.state)),
        }
        if !r.after.is_empty() {
            line.push_str(&format!("  after {}", r.after.join(", ")));
        }
        if let Some(f) = &r.for_who {
            line.push_str(&format!("  for {f}"));
        }
        out.push(line);
    }
    for j in joins.iter().filter(|j| !j.done) {
        out.push(format!(
            "join {} {}  waits on {}",
            j.id,
            j.title,
            j.lanes.join(", ")
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_glob_matches_within_and_across_directories() {
        assert!(matches("src/**", "src/a/b.rs"));
        assert!(matches("src/**", "src/a.rs"));
        assert!(matches("**/x.rs", "x.rs"));
        assert!(matches("**/x.rs", "a/b/x.rs"));
        assert!(matches("src/*.rs", "src/a.rs"));
        assert!(!matches("src/*.rs", "src/a/b.rs"));
        assert!(matches("docs/?.md", "docs/a.md"));
        assert!(!matches("docs/?.md", "docs/ab.md"));
        assert!(!matches("src/**", "srcx/a.rs"));
        assert!(!matches("src/a.rs", "src/a.rsx"));
    }

    #[test]
    fn a_file_outside_every_path_is_named_and_records_are_exempt() {
        let paths = vec!["src/**".to_owned(), "README.md".to_owned()];
        let changed = vec![
            "src/a.rs".to_owned(),
            "README.md".to_owned(),
            "lib/b.rs".to_owned(),
            ".openwarrant/state/x".to_owned(),
            "docs/tickets/t-1.md".to_owned(),
            "lib/b.rs".to_owned(),
        ];
        let exempt = vec![".openwarrant/".to_owned(), "docs/tickets/".to_owned()];
        assert_eq!(
            outside(&paths, &changed, &exempt),
            vec!["lib/b.rs".to_owned()]
        );
        assert!(outside(&paths, &changed[..2], &exempt).is_empty());
    }

    #[test]
    fn an_item_path_is_inside_only_when_it_provably_is() {
        let lane = vec!["src/api/**".to_owned(), "Cargo.toml".to_owned()];
        assert!(inside("src/api/**", &lane));
        assert!(inside("src/api/v1/**", &lane));
        assert!(inside("src/api/x.rs", &lane));
        assert!(inside("Cargo.toml", &lane));
        assert!(!inside("src/**", &lane));
        assert!(!inside("src/db/x.rs", &lane));
        assert!(!inside("**/*.rs", &lane));
    }

    #[test]
    fn overlapping_paths_are_said_and_disjoint_ones_are_not() {
        let a = vec!["src/api/**".to_owned()];
        let b = vec!["src/**".to_owned()];
        let c = vec!["docs/**".to_owned()];
        assert!(overlap(&a, &b).is_some());
        assert!(overlap(&a, &c).is_none());
    }
}
