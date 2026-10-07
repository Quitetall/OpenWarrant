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
//!     establishes it — the reading `war resolve` makes);
//!   - a ticket and its items: `open`, `in_progress`, `done`, from the
//!     checklist and the claims `war claim` holds;
//!   - a roadmap phase: `achieved` (its exit Warrant's resolution is
//!     recorded) and `accepted` (the roadmap record is accepted).
//! - **Declared states** come from a profile's `[[states]]` and are entered
//!   by `war state`, an authored event in the journal of the Warrant or
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
            })
        })
        .collect()
}

/// The declared states on record in one journal, evaluated against what
/// holds now. `declared` is the owning profile's `[[states]]`: an entry the
/// profile no longer declares (or now declares refining something else)
/// reads lapsed.
fn evaluate(entries: Vec<Entry>, holding: &[Holding], declared: &[DeclaredState]) -> Vec<Declared> {
    // The latest entry per (record, parent): journal order is append order.
    let mut latest: BTreeMap<(String, String), Entry> = BTreeMap::new();
    for e in entries {
        latest.insert((e.record.clone(), e.refines.clone()), e);
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
/// produce.
pub fn ticket_holding(caps: Capabilities, t: &Ticket, claims: &Claims) -> Vec<Holding> {
    let mut out = Vec::new();
    if !caps.has(openwarrant_core::Capability::Claims) {
        return out;
    }
    let tid = t.id();
    out.push(Holding {
        record: tid.to_owned(),
        state: match ticket::state_of(t, claims) {
            ticket::TicketState::Open => FixedState::Open,
            ticket::TicketState::InProgress => FixedState::InProgress,
            ticket::TicketState::Done => FixedState::Done,
        },
        episode: None,
    });
    for item in &t.checklist.items {
        let Some(iid) = &item.id else { continue };
        let held = ticket::claim_on(claims, tid, Some(iid))
            .or_else(|| ticket::claim_on(claims, tid, None));
        let (state, episode) = match (item.done, held) {
            (true, _) => (FixedState::Done, None),
            (false, Some((_, c))) => (FixedState::InProgress, c.map(|c| c.claim.clone())),
            (false, None) => (FixedState::Open, None),
        };
        out.push(Holding {
            record: format!("{tid}/{iid}"),
            state,
            episode,
        });
    }
    out
}

/// The declared states on record for a ticket and its items.
#[must_use]
pub fn ticket_declared(store: &ticket::Store, t: &Ticket, claims: &Claims) -> Vec<Declared> {
    let entries = entries_in(&t.dir);
    if entries.is_empty() {
        return Vec::new();
    }
    let holding = ticket_holding(store.definition.capabilities(), t, claims);
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

/// `achieved` and `accepted` for roadmap phases.
fn phase_holding(corpus: &Corpus) -> Vec<Holding> {
    let mut out = Vec::new();
    if let Ok(status) = corpus.status() {
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
    if let Ok(Some(rm)) = corpus.roadmap()
        && rm.accepted().is_some()
    {
        for p in &rm.phases.phases {
            out.push(Holding {
                record: p.id.clone(),
                state: FixedState::Accepted,
                episode: None,
            });
        }
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
    if let Ok((tickets, _)) = corpus.tickets()
        && let Ok(store) = ticket::Store::open(corpus.repo(), None)
    {
        let claims = store.claims().unwrap_or_default();
        let caps = store.definition.capabilities();
        for t in tickets {
            out.extend(ticket_holding(caps, t, &claims).iter().map(fixed_state));
            for d in ticket_declared(&store, t, &claims) {
                out.insert(declared_state(&d));
            }
        }
    }
    Ok(out)
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

/// What `war state` acts on, once resolved.
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
            holding: ticket_holding(store.definition.capabilities(), t, &claims),
            on_record: ticket_declared(&store, t, &claims),
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
                "{query} is not a record a journal owns: `war state` takes a Warrant \
                 (`NS-WAR-0001`), one of its records (`NS-WAR-0001/OBL-001`), a ticket or an \
                 item. `war model` lists record ids"
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

/// `war state <record> <name> [--note]`: enter a declared state, as an
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
