// SPDX-License-Identifier: Apache-2.0
//! Kernel states and declared refinements (OW-WAR-0148 M4; OW-ADR-0031).
//!
//! Two halves, kept apart:
//!
//! - **Fixed states** ([`FixedState`]) are derived here from facts other
//!   builders already read, and only where the record's type selects the
//!   capability that produces them:
//!   - a Warrant: `draft` (neither authorized nor resolved), `authorized`
//!     (its authorization binds the contract as it compiles now — §56.1
//!     requirement 1's own reading), `resolved` (a §56.2 record binding the
//!     current contract), `superseded` (OW-ADR-0022 currency);
//!   - an obligation: `verified` (an admissible independent verdict on record
//!     establishes it — the reading `war sign resolve` makes);
//!   - a ticket and its items: `open`, `in_progress`, `done`, from the
//!     checklist and the claims `war claim` holds;
//!   - a roadmap phase: `achieved` (its exit Warrant's resolution is
//!     recorded) and `accepted` (the roadmap record is accepted).
//! - **Declared states** come from a profile's `[[states]]` and are entered
//!   by `war plan state`, an authored event in the journal of the Warrant or
//!   ticket that owns the record (`state.entered`). One holds while its
//!   fixed parent holds — for an item's `in_progress`, under the same claim
//!   it was entered under — and reads `lapsed` once the parent stops. Of two
//!   declared states refining the same parent, the later entry is the one on
//!   record.
//!
//! Nothing that evaluates a §56.1 requirement, a capability gate, a
//! signature or a verdict reads anything in this module. A declared state is
//! a qualifier a team can see; it is never a way to read more than its
//! parent.

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use openwarrant_core::Capabilities;
use openwarrant_core::kernel_state::{DeclaredState, FixedState, StateKind};
use serde::Serialize;

use crate::corpus::Corpus;
use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};
use crate::ticket::{self, Ticket, claim::Claim};

/// The journal event a declared state is entered by.
pub const ENTERED: &str = "state.entered";

type Claims = BTreeMap<String, Option<Claim>>;

/// A fixed state holding for a record now. `episode` names what it holds
/// under, where that can change while the state still reads the same: an
/// item's `in_progress` holds under one claim, and a re-claim is a new
/// episode.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Holding {
    pub record: String,
    pub state: FixedState,
    pub episode: Option<String>,
}

/// A declared state on record for a record, held or lapsed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Declared {
    pub record: String,
    pub state: String,
    pub refines: String,
    pub lapsed: bool,
    pub entered_by: String,
    pub entered_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// One `state.entered` event, as the journal holds it.
#[derive(Debug, Clone)]
struct Entry {
    record: String,
    state: String,
    refines: String,
    episode: Option<String>,
    note: Option<String>,
    actor: String,
    at: String,
    id: String,
}

fn entries_in(dir: &Utf8Path) -> Vec<Entry> {
    // Cheap first: most journals hold no declared state, and a ticket list
    // reads every ticket's.
    let Ok(text) = crate::vfs::read_to_string(dir.join(crate::journal_cmd::FILE)) else {
        return Vec::new();
    };
    if !text.contains(ENTERED) {
        return Vec::new();
    }
    let Ok(journal) = crate::journal_cmd::parse(&text) else {
        return Vec::new();
    };
    journal
        .events
        .iter()
        .filter(|e| e.event_type == ENTERED)
        .filter_map(|e| {
            let p: serde_json::Value = serde_json::from_str(&e.payload).ok()?;
            let s = |k: &str| p.get(k).and_then(|v| v.as_str()).map(str::to_owned);
            Some(Entry {
                record: s("record")?,
                state: s("state")?,
                refines: s("refines")?,
                episode: s("episode"),
                note: s("note"),
                actor: e.actor_ref.clone(),
                at: e.occurred_at.clone(),
                id: e.id.clone(),
            })
        })
        .collect()
}

/// The declared states on record in one journal, evaluated against what
/// holds now. `declared` is the owning profile's `[[states]]`: an entry the
/// profile no longer declares (or now declares refining something else)
/// reads lapsed.
fn evaluate(entries: Vec<Entry>, holding: &[Holding], declared: &[DeclaredState]) -> Vec<Declared> {
    // The latest entry per (record, parent), FORMAT-1.0 §11.2: the later
    // `occurred_at`, and of two at one time the greater `id` — never the
    // order of the lines.
    let mut latest: BTreeMap<(String, String), Entry> = BTreeMap::new();
    for e in entries {
        let key = (e.record.clone(), e.refines.clone());
        let wins = latest
            .get(&key)
            .is_none_or(|old| (e.at.as_str(), e.id.as_str()) > (old.at.as_str(), old.id.as_str()));
        if wins {
            latest.insert(key, e);
        }
    }
    latest
        .into_values()
        .map(|e| {
            let still_declared = declared
                .iter()
                .any(|d| d.name == e.state && d.refines.as_str() == e.refines);
            let parent_holds = holding.iter().any(|h| {
                h.record == e.record && h.state.as_str() == e.refines && h.episode == e.episode
            });
            Declared {
                lapsed: !(still_declared && parent_holds),
                record: e.record,
                state: e.state,
                refines: e.refines,
                entered_by: e.actor,
                entered_at: e.at,
                note: e.note,
            }
        })
        .collect()
}

// ---- fixed states: tickets ---------------------------------------------------

/// The fixed states holding for a ticket and its items. Nothing without
/// `claims`: open, in progress and done are what claiming and finishing
/// produce. FORMAT-1.0 §11.1 adds, beside them: `claimed` for an item or
/// the ticket a live claim names (a claim whose lease ran out names
/// nothing; a lock that does not parse still holds, fail closed), and
/// `blocked` for an open item an item it waits on is not done — judged over
/// `tickets` when given (a blocker in another ticket is looked up there),
/// and not judged without them.
pub fn ticket_holding(
    caps: Capabilities,
    t: &Ticket,
    tickets: Option<&[Ticket]>,
    claims: &Claims,
) -> Vec<Holding> {
    let mut out = Vec::new();
    if !caps.has(openwarrant_core::Capability::Claims) {
        return out;
    }
    let now = ticket::now_secs();
    let live = |c: Option<&Claim>| c.is_none_or(|c| !c.lease_expired(now));
    let tid = t.id();
    out.push(Holding {
        record: tid.to_owned(),
        state: match ticket::phase_of(t, tickets, claims) {
            ticket::TicketState::Open => FixedState::Open,
            ticket::TicketState::InProgress => FixedState::InProgress,
            ticket::TicketState::Done => FixedState::Done,
        },
        episode: None,
    });
    let whole = ticket::claim_on(claims, tid, None);
    if !t.is_done()
        && let Some((_, c)) = whole
        && live(c)
    {
        out.push(Holding {
            record: tid.to_owned(),
            state: FixedState::Claimed,
            episode: c.map(|c| c.claim.clone()),
        });
    }
    for item in &t.checklist.items {
        let Some(iid) = &item.id else { continue };
        let record = format!("{tid}/{iid}");
        let held = ticket::claim_on(claims, tid, Some(iid)).or(whole);
        let (state, episode) = match (item.done, held) {
            (true, _) => (FixedState::Done, None),
            (false, Some((_, c))) => (FixedState::InProgress, c.map(|c| c.claim.clone())),
            (false, None) => (FixedState::Open, None),
        };
        if !item.done
            && let Some((_, c)) = held
            && live(c)
        {
            out.push(Holding {
                record: record.clone(),
                state: FixedState::Claimed,
                episode: episode.clone(),
            });
        }
        if !item.done
            && let Some(all) = tickets
            && !ticket::waiting_on(all, t, item).is_empty()
        {
            out.push(Holding {
                record: record.clone(),
                state: FixedState::Blocked,
                episode: None,
            });
        }
        out.push(Holding {
            record,
            state,
            episode,
        });
    }
    out
}

/// The declared states on record for a ticket and its items.
#[must_use]
pub fn ticket_declared(
    store: &ticket::Store,
    t: &Ticket,
    tickets: Option<&[Ticket]>,
    claims: &Claims,
) -> Vec<Declared> {
    let entries = entries_in(&t.dir);
    if entries.is_empty() {
        return Vec::new();
    }
    let holding = ticket_holding(store.definition.capabilities(), t, tickets, claims);
    evaluate(entries, &holding, &store.definition.states)
}

/// ` [in_review]` (or ` [in_review, lapsed]`) for a record that has a
/// declared state on record, empty otherwise: what `war show` appends.
#[must_use]
pub fn annotate(declared: &[Declared], record: &str) -> String {
    declared
        .iter()
        .filter(|d| d.record == record)
        .map(|d| {
            if d.lapsed {
                format!(" [{}, lapsed]", d.state)
            } else {
                format!(" [{}]", d.state)
            }
        })
        .collect()
}

// ---- fixed states: Warrants and phases -----------------------------------

/// The fixed states holding for every Warrant and obligation of the corpus,
/// each only where its kind selects the capability that produces it.
pub fn warrant_holding(corpus: &Corpus) -> Result<Vec<Holding>, RepoError> {
    use openwarrant_core::Capability as C;
    let repo = corpus.repo();
    let status = corpus.status()?;
    let mut out = Vec::new();
    let mut push = |record: String, state: FixedState| {
        out.push(Holding {
            record,
            state,
            episode: None,
        });
    };
    for w in &status.warrants {
        if w.validity != openwarrant_core::Validity::Valid {
            continue;
        }
        let Some(one) = corpus.entry(&w.alias).and_then(crate::corpus::Entry::ok) else {
            continue;
        };
        let caps = one.capabilities(&repo.profiles);
        let resolved = caps.has(C::Resolution)
            && w.resolution
                .as_ref()
                .is_some_and(|r| r.binds_current_contract);
        let authorized = caps.has(C::Authorization)
            && w.checks
                .as_ref()
                .is_some_and(|c| c.exact_authorized_contract_revision);
        if caps.has(C::Structure) && !authorized && !resolved {
            push(w.alias.clone(), FixedState::Draft);
        }
        if authorized {
            push(w.alias.clone(), FixedState::Authorized);
        }
        if resolved {
            push(w.alias.clone(), FixedState::Resolved);
        }
        if caps.has(C::Links)
            && matches!(
                corpus.currencies().of(&w.alias),
                crate::relations::Derived::Superseded { .. }
            )
        {
            push(w.alias.clone(), FixedState::Superseded);
        }
        if caps.has(C::Verification) {
            for o in &w.obligations {
                if o.disposition == "established"
                    && o.verifier.is_some()
                    && o.inadmissible_because.is_none()
                {
                    push(format!("{}/{}", w.alias, o.id), FixedState::Verified);
                }
            }
        }
    }
    Ok(out)
}

/// `achieved` and `accepted` for roadmap phases, each only where the
/// `roadmap` type selects the capability that produces it (OW-WAR-0148
/// M18): `links` for `achieved`, `acceptance` for `accepted`. Beside them,
/// the store types' own records: the roadmap and the specification
/// `accepted` while their bytes are an accepted revision's, and an ADR
/// another supersedes `superseded`.
fn phase_holding(corpus: &Corpus) -> Vec<Holding> {
    use openwarrant_core::Capability as C;
    use openwarrant_core::projection::Store;
    let repo = corpus.repo();
    let roadmap = crate::types::caps(repo, Store::Roadmap);
    let mut out = Vec::new();
    if roadmap.has(C::Links)
        && let Ok(status) = corpus.status()
    {
        for o in &status.objectives {
            if let (Some(r), openwarrant_core::status::Achieved::Recorded) =
                (&o.roadmap_ref, &o.achieved)
            {
                out.push(Holding {
                    record: format!("{}-PHASE-{}", r.prefix, r.phase),
                    state: FixedState::Achieved,
                    episode: None,
                });
            }
        }
    }
    if roadmap.has(C::Acceptance)
        && let Ok(Some(rm)) = corpus.roadmap()
        && rm.accepted().is_some()
    {
        for p in &rm.phases.phases {
            out.push(Holding {
                record: p.id.clone(),
                state: FixedState::Accepted,
                episode: None,
            });
        }
        if rm.is_accepted() {
            out.push(Holding {
                record: crate::model::roadmap_id(rm),
                state: FixedState::Accepted,
                episode: None,
            });
        }
    }
    if let Some(spec) = crate::model::spec_accepted(corpus) {
        out.push(Holding {
            record: spec,
            state: FixedState::Accepted,
            episode: None,
        });
    }
    for adr in crate::model::adrs_superseded(corpus) {
        out.push(Holding {
            record: adr,
            state: FixedState::Superseded,
            episode: None,
        });
    }
    out
}

/// The declared states on record in one Warrant's journal.
fn warrant_declared(corpus: &Corpus, dir: &Utf8Path, holding: &[Holding]) -> Vec<Declared> {
    let entries = entries_in(dir);
    if entries.is_empty() {
        return Vec::new();
    }
    let declared = corpus
        .entry_at(dir)
        .and_then(crate::corpus::Entry::ok)
        .and_then(|one| one.validated.as_ref())
        .and_then(|v| corpus.repo().profiles.definition(&v.profile))
        .map(|d| d.states.clone())
        .unwrap_or_default();
    evaluate(entries, holding, &declared)
}

// ---- fixed states: checks (FORMAT-1.0 §11.1, §12) -------------------------

/// The payload of a `check.run` event (FORMAT-1.0 §11.2, §12), as read.
/// The checks runner that records it is a later step; this is its shape.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct CheckRun {
    pub check: String,
    pub run: String,
    /// `PASS`, `FAIL` or `UNKNOWN` (read without regard to case).
    pub verdict: String,
    pub revision: String,
    /// `[id, revision]` of each record or item the check evaluates, at the
    /// run (draft 3, revision 1).
    #[serde(default)]
    pub subjects: Vec<(String, String)>,
    #[serde(default)]
    pub commit: Option<String>,
    #[serde(default)]
    pub output_sha256: Option<String>,
    #[serde(default)]
    pub value: Option<f64>,
}

/// The payload of a `trigger.fired` event (FORMAT-1.0 §12), as read.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct TriggerFired {
    pub check: String,
    pub trigger: String,
    pub subject: String,
}

/// The last `check.run` of each check in a journal, by the check's id as
/// the payload names it: the latest `occurred_at`, the greater `id`
/// breaking a tie (FORMAT-1.0 §11.2). A payload that is not a run's shape
/// is no run.
#[must_use]
pub fn last_runs(journal: &openwarrant_core::journal::Journal) -> BTreeMap<String, CheckRun> {
    last_runs_where(journal, &|_| true)
}

/// [`last_runs`] over the runs `keep` admits (a check's current revision's).
pub fn last_runs_where(
    journal: &openwarrant_core::journal::Journal,
    keep: &dyn Fn(&CheckRun) -> bool,
) -> BTreeMap<String, CheckRun> {
    let mut last: BTreeMap<String, ((String, String), CheckRun)> = BTreeMap::new();
    for e in &journal.events {
        if e.event_type != ticket::event::CHECK_RUN {
            continue;
        }
        let Ok(run) = serde_json::from_str::<CheckRun>(&e.payload) else {
            continue;
        };
        if !keep(&run) {
            continue;
        }
        let at = (e.occurred_at.clone(), e.id.clone());
        if last.get(&run.check).is_none_or(|(prev, _)| at > *prev) {
            last.insert(run.check.clone(), (at, run));
        }
    }
    last.into_iter().map(|(k, (_, r))| (k, r)).collect()
}

/// `failing` (FORMAT-1.0 §11.1): each check or kpi record of a Format 1.0
/// work document whose last recorded run on its current revision
/// (`check.run`, in the document's journal) failed. With no run on record a check has no such state: it is
/// absent, never assumed passing or failing.
fn failing(corpus: &Corpus) -> Vec<Holding> {
    let repo = corpus.repo();
    let mut out = Vec::new();
    for d in corpus.format_docs().docs.iter().filter(|d| d.admitted()) {
        let Some(alias) = d.doc.alias.as_deref() else {
            continue;
        };
        // Each check with its current revision: a run on an older one says
        // nothing about it now (§11.1, §12).
        let checks: BTreeMap<&str, String> = d
            .doc
            .units
            .iter()
            .filter(|u| matches!(u.record_type.as_str(), "check" | "kpi"))
            .map(|u| {
                (
                    u.id.as_str(),
                    openwarrant_core::format::sha256(
                        d.bytes.get(u.start..u.end).unwrap_or_default(),
                    ),
                )
            })
            .collect();
        if checks.is_empty() {
            continue;
        }
        let source = repo.root.join(&d.source);
        let dir = match source.file_name() {
            Some("warrant.md" | "ticket.md") => source.parent().map(Utf8Path::to_owned),
            _ => source.parent().map(|p| p.join(alias)),
        };
        let Some(dir) = dir else { continue };
        let Ok(journal) = crate::journal_cmd::load(&dir) else {
            continue;
        };
        let current = |r: &CheckRun| {
            let local = r.check.rsplit('/').next().unwrap_or(&r.check);
            checks.get(local).is_some_and(|rev| *rev == r.revision)
        };
        let runs = last_runs_where(&journal, &current);
        for c in checks.keys() {
            let global = format!("{alias}/{c}");
            let run = runs.get(*c).or_else(|| runs.get(&global));
            if run.is_some_and(|r| r.verdict.eq_ignore_ascii_case("fail")) {
                out.push(Holding {
                    record: global,
                    state: FixedState::Failing,
                    episode: None,
                });
            }
        }
    }
    out
}

// ---- the primary state (FORMAT-1.0 §11.1, §10.2) ----------------------------

/// A work document's phases, latest last: the one shown is the latest
/// that holds.
const PHASES: [&str; 6] = [
    "draft",
    "open",
    "in_progress",
    "done",
    "authorized",
    "resolved",
];

/// The primary state a record shows in the index and in packets
/// (FORMAT-1.0 §11.1, draft 3 revision 1): the first that holds of
/// `superseded`; the record's `phase` (a work document, `warrant` or
/// `ticket`: the latest of `draft`, `open`, `in_progress`, `done` or an
/// Appendix C phase that holds); its `checklist` state (an item: `done`,
/// `claimed`, `blocked` or `open`, in that order); its `check` state
/// (`failing`); its `disposition`; its `acceptance` (`accepted`,
/// `partially_accepted`); else `-`. A declared state that holds is shown
/// after its parent: `in_progress/in_review`.
///
/// `states` are the model's states of this one record (`oh.war/model/v1`
/// `states[]`): kernel states (`computed`, `authenticated`, `declared`)
/// and the builders' (`phase`, `checklist`, `disposition`, ...).
#[must_use]
pub fn primary_state(record_type: &str, states: &[&crate::model::State]) -> String {
    let holds = |v: &str| {
        states
            .iter()
            .any(|s| s.value == v && !s.lapsed && s.kind != StateKind::Declared.as_str())
    };
    let first = |vs: &[&str]| vs.iter().find(|v| holds(v)).map(|v| (*v).to_owned());
    let work = matches!(record_type, "warrant" | "ticket");
    let base: Option<String> = first(&["superseded"])
        .or_else(|| {
            work.then(|| {
                PHASES
                    .iter()
                    .rev()
                    .find(|p| holds(p))
                    .map(|p| (*p).to_owned())
            })
            .flatten()
        })
        .or_else(|| {
            (record_type == "item")
                .then(|| first(&["done", "claimed", "blocked", "open"]))
                .flatten()
        })
        .or_else(|| first(&["failing"]))
        .or_else(|| {
            states
                .iter()
                .find(|s| s.kind == "disposition")
                .map(|s| s.value.clone())
        })
        .or_else(|| first(&["accepted", "partially_accepted"]));
    let Some(base) = base else {
        return "-".to_owned();
    };
    // A declared state refining a state that holds follows it.
    match states.iter().find(|s| {
        s.kind == StateKind::Declared.as_str()
            && !s.lapsed
            && s.refines.as_deref().is_some_and(&holds)
    }) {
        Some(d) => format!("{base}/{}", d.value),
        None => base,
    }
}

// ---- the model -----------------------------------------------------------

fn fixed_state(h: &Holding) -> crate::model::State {
    crate::model::State {
        record: h.record.clone(),
        kind: h.state.kind().as_str().to_owned(),
        value: h.state.as_str().to_owned(),
        provenance: match h.state.kind() {
            StateKind::Authenticated => "recorded",
            _ => "computed",
        }
        .to_owned(),
        facet: Some(h.state.facet().to_owned()),
        refines: None,
        lapsed: false,
    }
}

fn declared_state(d: &Declared) -> crate::model::State {
    crate::model::State {
        record: d.record.clone(),
        kind: StateKind::Declared.as_str().to_owned(),
        value: d.state.clone(),
        provenance: "authored".to_owned(),
        facet: None,
        refines: Some(d.refines.clone()),
        lapsed: d.lapsed,
    }
}

/// Every kernel state of the corpus, for `oh.war/model/v1`.
pub fn model_states(corpus: &Corpus) -> Result<BTreeSet<crate::model::State>, RepoError> {
    let mut out = BTreeSet::new();
    let warrants = warrant_holding(corpus)?;
    for e in corpus.entries()? {
        for d in warrant_declared(corpus, &e.dir, &warrants) {
            out.insert(declared_state(&d));
        }
    }
    out.extend(warrants.iter().map(fixed_state));
    out.extend(phase_holding(corpus).iter().map(fixed_state));
    let store = ticket::Store::open(corpus.repo(), None).ok();
    let held = store.as_ref().map(|s| s.claims().unwrap_or_default());
    out.extend(
        format_warrant_phases(corpus, held.as_ref())
            .iter()
            .map(fixed_state),
    );
    if let Ok((tickets, _)) = corpus.tickets()
        && let Some(store) = store
    {
        let claims = held.unwrap_or_default();
        let caps = store.definition.capabilities();
        for t in tickets {
            out.extend(
                ticket_holding(caps, t, Some(tickets), &claims)
                    .iter()
                    .map(fixed_state),
            );
            for d in ticket_declared(&store, t, Some(tickets), &claims) {
                out.insert(declared_state(&d));
            }
        }
    }
    for h in failing(corpus) {
        out.insert(fixed_state(&h));
    }
    Ok(out)
}

/// FORMAT-1.0 §11.1's `phase` of each Format 1.0 Warrant (a ticket's is
/// [`ticket_holding`]'s): `done` when it has items and every box is `[x]`,
/// `in_progress` when one is, or a live claim in `claims` names the
/// Warrant or one of its items, else `open`. With no `claims` (the
/// documents alone, §10.3), no claim is held. A Warrant with no items is
/// `open` until claimed; one done as a whole is the tracker's (a journal
/// event, §11.2), not read here.
fn format_warrant_phases(corpus: &Corpus, claims: Option<&Claims>) -> Vec<Holding> {
    let now = ticket::now_secs();
    let live = |alias: &str, item: Option<&str>| {
        claims.is_some_and(|c| {
            ticket::claim_on(c, alias, item)
                .is_some_and(|(_, c)| c.is_none_or(|c| !c.lease_expired(now)))
        })
    };
    let mut out = Vec::new();
    for d in &corpus.format_docs().docs {
        if d.place != crate::format_doc::Place::Warrant || !d.admitted() {
            continue;
        }
        let Some(alias) = d.doc.alias.as_deref() else {
            continue;
        };
        let items: Vec<&openwarrant_core::format::Item> =
            d.doc.units.iter().flat_map(|u| u.items.iter()).collect();
        let started = items
            .iter()
            .any(|i| i.done || i.id.as_deref().is_some_and(|id| live(alias, Some(id))))
            || live(alias, None);
        let state = if !items.is_empty() && items.iter().all(|i| i.done) {
            FixedState::Done
        } else if started {
            FixedState::InProgress
        } else {
            FixedState::Open
        };
        out.push(Holding {
            record: alias.to_owned(),
            state,
            episode: None,
        });
    }
    out
}

/// The states the documents alone give (FORMAT-1.0 §10.3's purity rule:
/// no claim lock, journal or clock), from `states`, every state of the
/// model ([`model_states`] and the builders'). What the index and a preset
/// that does not read state show (§10.2, §10.4).
///
/// A ticket's and its items' fixed states are judged again with no claim
/// held and no journal read (a document done as a whole by a journal event
/// is not done here); a declared state (entered by a journal event) and a
/// check's `failing` (its recorded run) are left out. Everything else the
/// model states is read from documents and records of acts and is kept.
#[must_use]
pub fn document_states(
    corpus: &Corpus,
    states: &[crate::model::State],
) -> Vec<crate::model::State> {
    let mut tickets: Vec<Ticket> = corpus
        .tickets()
        .map(|(t, _)| t.to_vec())
        .unwrap_or_default();
    for t in &mut tickets {
        t.done_whole = false;
    }
    let ticket_ids: BTreeSet<&str> = tickets.iter().map(Ticket::id).collect();
    let of_ticket = |record: &str| {
        ticket_ids.contains(record)
            || record
                .split_once('/')
                .is_some_and(|(t, _)| ticket_ids.contains(t))
    };
    let failing = FixedState::Failing.as_str();
    let mut out: Vec<crate::model::State> = states
        .iter()
        .filter(|s| s.kind != StateKind::Declared.as_str() && s.value != failing)
        .filter(|s| !of_ticket(&s.record) || s.kind == "checklist")
        .cloned()
        .collect();
    out.extend(format_warrant_phases(corpus, None).iter().map(fixed_state));
    if let Ok(store) = ticket::Store::open(corpus.repo(), None) {
        let caps = store.definition.capabilities();
        let none = Claims::new();
        for t in &tickets {
            out.extend(
                ticket_holding(caps, t, Some(&tickets), &none)
                    .iter()
                    .map(fixed_state),
            );
        }
    }
    out.sort();
    out.dedup();
    out
}

// ---- `war show <alias>` --------------------------------------------------

/// The declared states on record for a Warrant and its records, as a short
/// section for `war show`; `None` when its journal holds none, so a Warrant
/// without one renders exactly as before.
pub fn warrant_section(repo: &Repository, alias: &str) -> Option<(String, Vec<Declared>)> {
    let dir = repo.warrant_dir(alias).ok()?;
    if entries_in(&dir).is_empty() {
        return None;
    }
    let corpus = Corpus::new(repo);
    let holding = warrant_holding(&corpus).ok()?;
    let declared = warrant_declared(&corpus, &dir, &holding);
    if declared.is_empty() {
        return None;
    }
    let mut md = String::from("\n## Declared states\n\n");
    for d in &declared {
        md.push_str(&format!(
            "- {}: {} (refines {}{}) — entered by {} at {}{}\n",
            d.record,
            d.state,
            d.refines,
            if d.lapsed { "; lapsed" } else { "" },
            d.entered_by,
            d.entered_at,
            d.note
                .as_ref()
                .map(|n| format!(": {n}"))
                .unwrap_or_default()
        ));
    }
    Some((md, declared))
}

// ---- `war state` -----------------------------------------------------------

/// What `war plan state` acts on, once resolved.
struct Subject {
    record: String,
    /// `ticket t-x` or `Warrant NS-WAR-0001`, for messages.
    owner: String,
    profile: String,
    journal_dir: camino::Utf8PathBuf,
    uuid: String,
    actor: String,
    declared: Vec<DeclaredState>,
    holding: Vec<Holding>,
    on_record: Vec<Declared>,
}

fn refused(rule: &str, file: &str, message: String) -> ticket::Outcome {
    let mut report = Report::default();
    report.push(Diagnostic::error(rule, file.to_owned(), message.clone()));
    ticket::Outcome {
        report,
        human: message,
        result: serde_json::Value::Null,
    }
}

fn subject(
    repo: &Repository,
    query: &str,
    actor: Option<&str>,
) -> Result<Result<Subject, ticket::Outcome>, RepoError> {
    if ticket::is_ticket_ref(query) {
        let store = ticket::Store::open(repo, actor)?;
        let (tickets, _) = store.load_all()?;
        let (n, record) = match ticket::resolve(&tickets, query) {
            Ok(ticket::Target::Ticket(n)) => (n, tickets[n].id().to_owned()),
            Ok(ticket::Target::Item(n, i)) => (n, format!("{}/{i}", tickets[n].id())),
            Err(d) => {
                return Ok(Err(refused(
                    "state.record-unknown",
                    "",
                    format!("{query}: {}", d.message),
                )));
            }
        };
        let t = &tickets[n];
        let claims = store.claims()?;
        return Ok(Ok(Subject {
            owner: format!("ticket {}", t.id()),
            profile: store.definition.name.clone(),
            journal_dir: t.dir.clone(),
            uuid: t.manifest.uuid.clone(),
            actor: store.actor.clone(),
            declared: store.definition.states.clone(),
            holding: ticket_holding(store.definition.capabilities(), t, Some(&tickets), &claims),
            on_record: ticket_declared(&store, t, Some(&tickets), &claims),
            record,
        }));
    }
    let alias = query.split('/').next().unwrap_or(query);
    let corpus = Corpus::new(repo);
    let unknown = || {
        refused(
            "state.record-unknown",
            "",
            format!(
                "{query} is not a record a journal owns: `war plan state` takes a Warrant \
                 (`NS-WAR-0001`), one of its records (`NS-WAR-0001/OBL-001`), a ticket or an \
                 item. `war plan model` lists record ids"
            ),
        )
    };
    let Some(entry) = corpus.entry(alias) else {
        return Ok(Err(unknown()));
    };
    let Some(validated) = entry.ok().and_then(|one| one.validated.as_ref()) else {
        return Ok(Err(refused(
            "state.record-unknown",
            &repo.relative(&entry.dir),
            format!("{alias}: the manifest did not validate, so it holds no state"),
        )));
    };
    let model = crate::model::build(&corpus)?;
    if !model.records.iter().any(|r| r.id == query) {
        return Ok(Err(unknown()));
    }
    let holding = warrant_holding(&corpus)?;
    let on_record = warrant_declared(&corpus, &entry.dir, &holding);
    let actor = actor
        .map(str::to_owned)
        .or_else(|| std::env::var("OPENWARRANT_ACTOR").ok())
        .map(|a| openwarrant_core::ticket::one_line(&a))
        .filter(|a| !a.is_empty())
        .unwrap_or_else(|| repo.performer());
    Ok(Ok(Subject {
        record: query.to_owned(),
        owner: format!("Warrant {alias}"),
        profile: validated.profile.to_string(),
        journal_dir: entry.dir.clone(),
        uuid: validated.uuid.to_string(),
        actor,
        declared: repo
            .profiles
            .definition(&validated.profile)
            .map(|d| d.states.clone())
            .unwrap_or_default(),
        holding,
        on_record,
    }))
}

/// `war plan state <record> <name> [--note]`: enter a declared state, as an
/// authored event in the owning journal. Refused, by rule and with nothing
/// written: a record no journal owns (`state.record-unknown`), a fixed
/// state's name (`state.fixed`), a name the record's profile does not
/// declare (`state.undeclared`), and a state whose fixed parent does not
/// hold for the record now (`state.parent-not-holding`) — for an
/// authenticated parent, the act or verdict behind it has to be on record
/// first.
pub fn enter(
    repo: &Repository,
    query: &str,
    name: &str,
    note: Option<&str>,
    actor: Option<&str>,
) -> Result<ticket::Outcome, RepoError> {
    let s = match subject(repo, query, actor)? {
        Ok(s) => s,
        Err(refusal) => return Ok(refusal),
    };
    let journal = repo.relative(&s.journal_dir.join(crate::journal_cmd::FILE));
    if let Some(fixed) = FixedState::parse(name) {
        return Ok(refused(
            "state.fixed",
            &journal,
            format!(
                "{}: `{fixed}` is a fixed kernel state, {}: nobody enters one by hand. A \
                 profile's `[[states]]` declares refinements that can be entered",
                s.record,
                match fixed.kind() {
                    StateKind::Authenticated =>
                        "authenticated — it holds only on the record of the act or verdict behind it",
                    _ => "computed from the records",
                }
            ),
        ));
    }
    let Some(d) = s.declared.iter().find(|d| d.name == name) else {
        let known: Vec<String> = s
            .declared
            .iter()
            .map(|d| format!("{} (refines {})", d.name, d.refines))
            .collect();
        return Ok(refused(
            "state.undeclared",
            &journal,
            format!(
                "{}: profile {} declares no state `{name}`; it declares {}. A declared state \
                 is a profile's `[[states]] name = \"...\", refines = \"<fixed state>\"`",
                s.record,
                s.profile,
                if known.is_empty() {
                    "none".to_owned()
                } else {
                    known.join(", ")
                }
            ),
        ));
    };
    let now: Vec<&Holding> = s.holding.iter().filter(|h| h.record == s.record).collect();
    let Some(parent) = now.iter().find(|h| h.state == d.refines) else {
        let reads = if now.is_empty() {
            "no fixed state holds for it".to_owned()
        } else {
            format!(
                "it reads {}",
                now.iter()
                    .map(|h| h.state.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let why = if d.refines.kind() == StateKind::Authenticated {
            format!(
                "`{}` is authenticated: `{name}` can be entered only while `{}` already holds, \
                 and a declared state never stands in for the act or verdict behind it",
                d.refines, d.refines
            )
        } else {
            format!("`{name}` holds only while `{}` does", d.refines)
        };
        return Ok(refused(
            "state.parent-not-holding",
            &journal,
            format!(
                "{}: `{name}` refines `{}`, which does not hold for it now ({reads}). {why}; \
                 nothing was written",
                s.record, d.refines
            ),
        ));
    };
    let result = |entered: bool| {
        serde_json::json!({
            "record": s.record,
            "state": name,
            "kind": StateKind::Declared.as_str(),
            "refines": d.refines.as_str(),
            "owner": s.owner,
            "journal": journal,
            "entered": entered,
        })
    };
    if s.on_record
        .iter()
        .any(|o| o.record == s.record && o.state == name && !o.lapsed)
    {
        let mut report = Report::default();
        let message = format!(
            "{}: already {name} (refines {}); nothing was written",
            s.record, d.refines
        );
        report.push(Diagnostic::pass("state.unchanged", message.clone()));
        return Ok(ticket::Outcome {
            report,
            human: message,
            result: result(false),
        });
    }
    let mut payload = serde_json::json!({
        "record": s.record,
        "state": name,
        "refines": d.refines.as_str(),
        "at": crate::gate_cmd::receipt::rfc3339_from_secs(ticket::now_secs()),
    });
    if let Some(e) = &parent.episode {
        payload["episode"] = serde_json::json!(e);
    }
    if let Some(n) = note.map(str::trim).filter(|n| !n.is_empty()) {
        payload["note"] = serde_json::json!(n);
    }
    crate::journal_cmd::record(
        &s.journal_dir,
        &s.uuid,
        ENTERED,
        &s.actor,
        &payload.to_string(),
    )?;
    let message = format!(
        "{}: {name} (declared; refines {}) — journaled in {journal}. It holds while {} does, \
         and satisfies no check and no gate",
        s.record, d.refines, d.refines
    );
    let mut report = Report::default();
    report.push(Diagnostic::pass("state.entered", message.clone()));
    Ok(ticket::Outcome {
        report,
        human: message,
        result: result(true),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(kind: &str, value: &str, refines: Option<&str>, lapsed: bool) -> crate::model::State {
        crate::model::State {
            record: "t-1/i-1".to_owned(),
            kind: kind.to_owned(),
            value: value.to_owned(),
            provenance: "computed".to_owned(),
            facet: None,
            refines: refines.map(str::to_owned),
            lapsed,
        }
    }

    /// FORMAT-1.0 §11.1: an item shows `done`, `claimed`, `blocked` or
    /// `open`, the first that holds; a declared state that holds follows
    /// it; a lapsed one does not.
    #[test]
    fn an_items_primary_state_is_the_first_that_holds() {
        let open = st("computed", "open", None, false);
        let blocked = st("computed", "blocked", None, false);
        let claimed = st("computed", "claimed", None, false);
        let in_progress = st("computed", "in_progress", None, false);
        let review = st("declared", "in_review", Some("in_progress"), false);
        let lapsed = st("declared", "in_review", Some("in_progress"), true);
        assert_eq!(primary_state("item", &[&open, &blocked]), "blocked");
        assert_eq!(primary_state("item", &[&open]), "open");
        assert_eq!(
            primary_state("item", &[&in_progress, &claimed, &review]),
            "claimed/in_review"
        );
        assert_eq!(
            primary_state("item", &[&in_progress, &claimed, &lapsed]),
            "claimed"
        );
        // A declared state never stands in for a fixed one.
        assert_eq!(primary_state("item", &[&review]), "-");
    }

    /// A work document shows its latest phase; `superseded` beats every
    /// phase; an obligation its disposition; anything else `-`.
    #[test]
    fn a_documents_primary_state_is_its_latest_phase() {
        let draft = st("computed", "draft", None, false);
        let authorized = st("authenticated", "authorized", None, false);
        let in_progress = st("computed", "in_progress", None, false);
        let superseded = st("computed", "superseded", None, false);
        let review = st("declared", "in_review", Some("in_progress"), false);
        assert_eq!(primary_state("warrant", &[&draft]), "draft");
        assert_eq!(
            primary_state("warrant", &[&draft, &authorized]),
            "authorized"
        );
        assert_eq!(
            primary_state("ticket", &[&in_progress, &review]),
            "in_progress/in_review"
        );
        assert_eq!(
            primary_state("warrant", &[&authorized, &superseded]),
            "superseded"
        );
        let disposition = st("disposition", "established", None, false);
        assert_eq!(primary_state("obligation", &[&disposition]), "established");
        assert_eq!(primary_state("requirement", &[&draft]), "-");
    }

    /// FORMAT-1.0 §11.1, §12: a check's last recorded run decides `failing`;
    /// a later pass clears it, and with no run there is nothing to read.
    #[test]
    fn the_last_check_run_decides() {
        let ev = |at: &str, verdict: &str| {
            crate::journal_cmd::event_in(
                crate::journal_cmd::Shape::V2,
                "u",
                ticket::event::CHECK_RUN,
                "war",
                at,
                &format!(
                    r#"{{"check":"CHK-1","run":"{at}","verdict":"{verdict}","revision":"sha256:0","subjects":[["t-1/i-1","sha256:1"]]}}"#
                ),
            )
            .unwrap()
        };
        let mut j = openwarrant_core::journal::Journal::default();
        assert!(last_runs(&j).is_empty());
        j.append(ev("2026-10-15T10:00:00Z", "FAIL")).unwrap();
        assert_eq!(last_runs(&j)["CHK-1"].verdict, "FAIL");
        assert_eq!(last_runs(&j)["CHK-1"].subjects[0].0, "t-1/i-1");
        j.append(ev("2026-10-15T11:00:00Z", "PASS")).unwrap();
        assert_eq!(last_runs(&j)["CHK-1"].verdict, "PASS");
        // A run on another revision of the check is not its last run now.
        assert!(last_runs_where(&j, &|r| r.revision == "sha256:other").is_empty());
        // A run whose time is earlier does not override a later one,
        // whatever line it is on.
        j.append(ev("2026-10-15T09:00:00Z", "FAIL")).unwrap();
        assert_eq!(last_runs(&j)["CHK-1"].verdict, "PASS");
    }
}
