// SPDX-License-Identifier: Apache-2.0
//! What a person or an arriving agent reads: `war view prime`, `war show`,
//! `war view tickets`. Plain Markdown, in the words of the work — no rule names,
//! no section numbers.

use std::collections::BTreeMap;

use serde::Serialize;

use openwarrant_core::ticks::Level;

use openwarrant_core::ticks::Checks;

use super::ladder::{TickCounts, TickView};
use super::{
    Outcome, Store, Target, Ticket, TicketState, claim, claim_on, now_secs, open_blockers, resolve,
    rfc3339, state_of,
};
use crate::repo::RepoError;

/// `90s` → `1m`, `7500s` → `2h 5m`, `200000s` → `2d 7h`.
#[must_use]
pub fn ago(secs: u64) -> String {
    match secs {
        0..60 => format!("{secs}s"),
        60..3600 => format!("{}m", secs / 60),
        3600..86_400 => {
            let m = (secs % 3600) / 60;
            if m == 0 {
                format!("{}h", secs / 3600)
            } else {
                format!("{}h {m}m", secs / 3600)
            }
        }
        _ => {
            let h = (secs % 86_400) / 3600;
            if h == 0 {
                format!("{}d", secs / 86_400)
            } else {
                format!("{}d {h}h", secs / 86_400)
            }
        }
    }
}

/// The intent's text without its title line and without its notes.
#[must_use]
pub fn description(intent: &str) -> String {
    let mut lines = intent.lines().peekable();
    if lines.peek().is_some_and(|l| l.starts_with("# ")) {
        lines.next();
    }
    let body: Vec<&str> = lines
        .take_while(|l| l.trim_end() != super::NOTES_HEADING)
        .collect();
    body.join("\n").trim().to_owned()
}

/// The dated notes, oldest first, each with its continuation lines.
#[must_use]
pub fn notes(intent: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut inside = false;
    for line in intent.lines() {
        if line.starts_with("## ") || line.starts_with("# ") {
            inside = line.trim_end() == super::NOTES_HEADING;
            continue;
        }
        if !inside {
            continue;
        }
        if let Some(rest) = line.strip_prefix("- ") {
            out.push(rest.to_owned());
        } else if let (Some(last), true) = (out.last_mut(), line.starts_with("  ")) {
            last.push('\n');
            last.push_str(line);
        }
    }
    out
}

/// The date the ticket's last item was ticked, from the checklist itself.
fn done_on(t: &Ticket) -> Option<chrono::NaiveDate> {
    t.checklist
        .items
        .iter()
        .filter_map(|i| i.done_on.as_deref())
        .filter_map(|d| chrono::NaiveDate::parse_from_str(d.get(..10)?, "%Y-%m-%d").ok())
        .max()
}

fn today() -> chrono::NaiveDate {
    chrono::DateTime::from_timestamp(i64::try_from(now_secs()).unwrap_or(0), 0)
        .map_or(chrono::NaiveDate::MIN, |d| d.date_naive())
}

type Claims = BTreeMap<String, Option<claim::Claim>>;

/// The annotation after an open item: who holds it, or what it waits on.
fn annotation(
    store: &Store,
    tickets: &[Ticket],
    t: &Ticket,
    item: &openwarrant_core::ticket::Item,
    claims: &Claims,
    now: u64,
    checks: &Checks,
) -> String {
    let mut notes = Vec::new();
    let held = item
        .id
        .as_deref()
        .and_then(|id| claim_on(claims, t.id(), Some(id)))
        .or_else(|| claim_on(claims, t.id(), None));
    if let Some((_, c)) = held {
        notes.push(match c {
            Some(c) => {
                let stale = if c.lease_expired(now) {
                    ", lease ran out"
                } else if c.age(now) > store.ttl_secs {
                    ", stale"
                } else {
                    ""
                };
                format!("claimed by {} ({} ago{stale})", c.actor, ago(c.age(now)))
            }
            None => "claimed (unreadable lock)".to_owned(),
        });
    }
    let waiting = open_blockers(tickets, t, item);
    if !waiting.is_empty() {
        notes.push(format!("waits on {}", waiting.join(", ")));
    }
    if item.id.is_none() {
        notes.push("no id yet".to_owned());
    }
    // OW-WAR-0148 M13: what the item must show when it ticks, when that is
    // more than a claim.
    if let Some(n) = minimum_note(store, t, item, checks) {
        notes.push(n);
    }
    if notes.is_empty() {
        String::new()
    } else {
        format!(" — {}", notes.join("; "))
    }
}

/// `ticks at observed or above: `war done t-x/i-y --check`` for an open item
/// whose minimum is above claimed; `None` for every other.
fn minimum_note(
    store: &Store,
    t: &Ticket,
    item: &openwarrant_core::ticket::Item,
    checks: &Checks,
) -> Option<String> {
    let id = item.id.as_deref()?;
    let (min, _) = super::ladder::minimum(store, t, checks, Some(id));
    let milestone = checks.milestone(id).is_some();
    if min == Level::Claimed && !milestone {
        return None;
    }
    let target = format!("{}/{id}", t.id());
    Some(format!(
        "{}ticks at {} or above: {}",
        if milestone { "milestone; " } else { "" },
        min.as_str(),
        super::ladder::command_for(min, &target)
    ))
}

/// What a view puts beside a done item: its level, who verified or signed
/// it, and the minimum when it is short of it. `(claimed)` never reads as
/// checked.
pub(crate) fn tick_marker(v: &TickView) -> String {
    let mut inner = v.level.as_str().to_owned();
    if let Some(by) = &v.by {
        inner.push_str(&match v.level {
            Level::Independent => format!(": verified by {by}"),
            Level::Signed => format!(": signed off by {by}"),
            _ => String::new(),
        });
    }
    if !v.meets_minimum {
        inner.push_str(&format!("; needs {}", v.minimum.as_str()));
    }
    format!("({inner})")
}

fn open_item_line(
    store: &Store,
    tickets: &[Ticket],
    t: &Ticket,
    item: &openwarrant_core::ticket::Item,
    claims: &Claims,
    now: u64,
    checks: &Checks,
) -> String {
    format!(
        "- [ ] {}{}{}\n",
        item.text,
        item.id
            .as_ref()
            .map(|i| format!(" ({i})"))
            .unwrap_or_default(),
        annotation(store, tickets, t, item, claims, now, checks)
    )
}

/// A ticket's summary row, for `--json`.
#[derive(Debug, Serialize)]
struct Row {
    id: String,
    title: String,
    state: TicketState,
    done: usize,
    total: usize,
    priority: u8,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    done_on: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    promoted_to: Option<String>,
    claims: Vec<claim::Claim>,
    dir: String,
    /// OW-WAR-0148 M4: declared states on record for the ticket or its
    /// items, held or lapsed. Absent where there are none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    declared_states: Vec<crate::states::Declared>,
    /// OW-WAR-0148 M5: the ticket's type, labels, epic and issue, each
    /// absent when the ticket has none.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    labels: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    part_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    issue: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    issue_url: Option<String>,
    /// For an epic: its tickets (those `part_of` it), done and in all.
    #[serde(skip_serializing_if = "Option::is_none")]
    tickets: Option<Progress>,
    /// OW-WAR-0148 M13: how many of its ticks stand at each level of the
    /// ladder. Absent while nothing is ticked.
    #[serde(skip_serializing_if = "Option::is_none")]
    ticks: Option<TickCounts>,
}

/// `done` of `total`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
}

/// An epic's progress over its tickets, or `None` when nothing is part of it.
fn epic_progress(all: &[Ticket], t: &Ticket, claims: &Claims) -> Option<Progress> {
    let kids = super::children_of(all, t.id());
    (!kids.is_empty()).then(|| Progress {
        done: kids
            .iter()
            .filter(|k| state_of(k, claims) == TicketState::Done)
            .count(),
        total: kids.len(),
    })
}

fn row(store: &Store, t: &Ticket, claims: &Claims) -> Row {
    let (done, total) = t.checklist.progress();
    let prefix = format!("{}--", t.id());
    Row {
        id: t.id().to_owned(),
        title: t.manifest.title.clone(),
        state: state_of(t, claims),
        done,
        total,
        priority: t.manifest.priority,
        created_at: t.manifest.created_at.clone(),
        done_on: done_on(t).map(|d| d.to_string()),
        promoted_to: t.manifest.promoted_to.clone(),
        claims: claims
            .iter()
            .filter(|(k, _)| **k == claim::lock_name(t.id(), None) || k.starts_with(&prefix))
            .filter_map(|(_, c)| c.clone())
            .collect(),
        dir: store.rel(&t.dir),
        declared_states: crate::states::ticket_declared(store, t, claims),
        kind: t.manifest.kind.clone(),
        labels: t.manifest.labels.clone(),
        part_of: t.manifest.part_of.clone(),
        issue: t.manifest.issue,
        issue_url: t.manifest.issue_url.clone(),
        tickets: None,
        ticks: {
            let counts = super::ladder::ticks_of(&super::ladder::Reader::new(store), t).counts;
            (counts.total() > 0).then_some(counts)
        },
    }
}

/// The row with its epic progress, read over `all`.
fn row_in(store: &Store, all: &[Ticket], t: &Ticket, claims: &Claims) -> Row {
    let mut r = row(store, t, claims);
    r.tickets = epic_progress(all, t, claims);
    r
}

/// The bracketed tail of a `war view tickets` line for M5's fields, empty for a
/// ticket that has none: `  [bug; backend, auth]  [in t-1a2b]  [epic: 1/3]
/// [#12]`.
fn fields_tail(r: &Row, with_parent: bool) -> String {
    let mut out = String::new();
    let mut what = Vec::new();
    if let Some(k) = &r.kind {
        what.push(k.clone());
    }
    if !r.labels.is_empty() {
        what.push(r.labels.join(", "));
    }
    if !what.is_empty() {
        out.push_str(&format!("  [{}]", what.join("; ")));
    }
    if let Some(p) = r.part_of.as_ref().filter(|_| with_parent) {
        out.push_str(&format!("  [in {p}]"));
    }
    if let Some(p) = r.tickets {
        out.push_str(&format!("  [epic: {}/{} done]", p.done, p.total));
    }
    if let Some(n) = r.issue {
        out.push_str(&format!("  [#{n}]"));
    }
    out
}

// ---- filters and search (OW-WAR-0148 M5) -----------------------------------

/// `war view tickets --type/--label/--state/--text/--search/--epic`. Every filter
/// given must hold; none given lists every ticket, as before.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub kind: Option<String>,
    /// Every label named must be on the ticket.
    pub labels: Vec<String>,
    /// `open`, `in_progress`, `done`, or a state the ticket profile declares
    /// (`in_review`), held by the ticket or one of its items.
    pub state: Option<String>,
    /// A phrase, matched case-insensitively anywhere in the ticket's text.
    pub text: Option<String>,
    /// Words, each of which must begin a word somewhere in the ticket's text.
    pub search: Option<String>,
    /// Only the tickets part of this one (an epic).
    pub epic: Option<String>,
}

impl Filter {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.kind.is_none()
            && self.labels.is_empty()
            && self.state.is_none()
            && self.text.is_none()
            && self.search.is_none()
            && self.epic.is_none()
    }
}

/// Everything `--text` and `--search` read: the title, the whole intent
/// (description and notes), and each item's text and done note.
fn haystack(t: &Ticket) -> String {
    let mut h = String::with_capacity(t.intent.len() + t.checklist_text.len() + 64);
    h.push_str(&t.manifest.title);
    h.push('\n');
    h.push_str(&t.intent);
    for item in &t.checklist.items {
        h.push('\n');
        h.push_str(&item.text);
        if let Some(n) = &item.note {
            h.push('\n');
            h.push_str(n);
        }
    }
    h.to_lowercase()
}

/// The words of `text`, lowercase: maximal runs of letters and digits.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// A fixed state's name as a filter takes it: `in_progress`, `in-progress`
/// and `in progress` are one.
fn fixed_state(name: &str) -> Option<TicketState> {
    match name.trim().to_lowercase().replace(['-', ' '], "_").as_str() {
        "open" => Some(TicketState::Open),
        "in_progress" => Some(TicketState::InProgress),
        "done" => Some(TicketState::Done),
        _ => None,
    }
}

fn sorted<'a>(tickets: &'a [Ticket], claims: &Claims) -> Vec<&'a Ticket> {
    let rank = |s: TicketState| match s {
        TicketState::InProgress => 0,
        TicketState::Open => 1,
        TicketState::Done => 2,
    };
    let mut out: Vec<&Ticket> = tickets.iter().collect();
    out.sort_by(|a, b| {
        (
            rank(state_of(a, claims)),
            a.manifest.priority,
            &a.manifest.created_at,
            a.id(),
        )
            .cmp(&(
                rank(state_of(b, claims)),
                b.manifest.priority,
                &b.manifest.created_at,
                b.id(),
            ))
    });
    out
}

/// Every Warrant beside the light ones, for the one list (OW-WAR-0148
/// M10): the directory and read-in-place rows, the warnings from reading
/// them, and the words `--type` accepts for them. Empty: the light
/// Warrants alone, as before M10 (the board reads that).
#[derive(Debug, Clone, Default)]
pub struct Others {
    pub rows: Vec<crate::warrants::Row>,
    pub faults: Vec<crate::diagnostic::Diagnostic>,
    /// Profile names and read-in-place kinds a row may carry.
    pub types: Vec<String>,
}

impl Others {
    /// Every Warrant of `repo` that is not in the light encoding.
    pub fn of(repo: &crate::repo::Repository) -> Result<Self, RepoError> {
        let (rows, faults) = crate::warrants::other_rows(repo)?;
        Ok(Self {
            rows,
            faults,
            types: crate::warrants::type_words(repo),
        })
    }
}

/// The light Warrants alone, every one, its state and progress.
pub fn tickets(store: &Store) -> Result<Outcome, RepoError> {
    tickets_filtered(store, &Filter::default())
}

/// The light Warrants alone, filtered.
pub fn tickets_filtered(store: &Store, filter: &Filter) -> Result<Outcome, RepoError> {
    list(store, &Others::default(), filter)
}

/// `war view warrants` (`war view tickets`, `war view ls`): every Warrant, whatever its
/// encoding, with its type, state and progress. The light ones first, in
/// the order work takes them (`result.tickets`, as before M10); then the
/// directory ones and the ones read in place (`result.warrants`).
///
/// Filters: exactly the Warrants every given filter admits, in the same
/// order and form. `--type` takes a light Warrant's type (`bug`) or any
/// Warrant's profile (`delivery`, `ticket`, `openspec`); `--state` the
/// fixed three, a declared state, or a phase a journal records (`draft`,
/// `authorized`, `resolved`). Labels and epics are the light encoding's.
/// A filter that can match nothing by construction is refused by name.
pub fn list(store: &Store, others: &Others, filter: &Filter) -> Result<Outcome, RepoError> {
    let (tickets, faults) = store.load_all()?;
    let claims = store.claims()?;
    // Refusals first: a filter that can match nothing by construction is a
    // typo, never an empty answer.
    if let Some(k) = &filter.kind
        && k != openwarrant_core::ticket::TICKET_PROFILE
        && !others.types.iter().any(|t| t == k)
        && let Some(why) = store.definition.fields.refuse_type(k)
    {
        let mut why = why;
        if !others.types.is_empty() {
            why.push_str(&format!(
                "; nor is it a Warrant type here: {}",
                others.types.join(", ")
            ));
        }
        return Ok(super::Outcome::refused(
            "ticket.filter-type-unknown",
            "profiles/ticket.toml",
            why,
        ));
    }
    for l in &filter.labels {
        if let Some(why) = store.definition.fields.refuse_label(l) {
            return Ok(super::Outcome::refused(
                "ticket.label-unknown",
                "profiles/ticket.toml",
                why,
            ));
        }
    }
    // A state a light Warrant can be in (fixed or declared), or a phase a
    // directory Warrant's journal records.
    enum Wanted {
        Fixed(TicketState),
        Declared(String),
        Phase(String),
    }
    let wanted_state = match &filter.state {
        None => None,
        Some(s) => match fixed_state(s) {
            Some(f) => Some(Wanted::Fixed(f)),
            None if store.definition.states.iter().any(|d| d.name == s.trim()) => {
                Some(Wanted::Declared(s.trim().to_owned()))
            }
            None if crate::warrants::PHASES.contains(&s.trim()) => {
                Some(Wanted::Phase(s.trim().to_owned()))
            }
            None => {
                let mut known = vec![
                    "open".to_owned(),
                    "in_progress".to_owned(),
                    "done".to_owned(),
                ];
                known.extend(store.definition.states.iter().map(|d| d.name.clone()));
                return Ok(super::Outcome::refused(
                    "ticket.filter-state-unknown",
                    "profiles/ticket.toml",
                    format!(
                        "state {s:?} is neither a fixed state nor one the ticket profile declares: {}; \
                         nor a phase a Warrant's journal records: {}",
                        known.join(", "),
                        crate::warrants::PHASES.join(", ")
                    ),
                ));
            }
        },
    };
    let epic = match &filter.epic {
        None => None,
        Some(q) => match resolve(&tickets, q) {
            Ok(Target::Ticket(n)) => Some(tickets[n].id().to_owned()),
            Ok(Target::Item(..)) => {
                return Ok(super::Outcome::refused(
                    "ticket.unknown",
                    String::new(),
                    format!("`--epic {q}` names an item; an epic is a Warrant"),
                ));
            }
            Err(d) => return Ok(super::Outcome::from_diagnostic(d)),
        },
    };
    let text = filter.text.as_ref().map(|t| t.trim().to_lowercase());
    let search = filter.search.as_deref().map(words).unwrap_or_default();
    let matches_text = |hay: &str| -> bool {
        if let Some(phrase) = &text
            && !hay.contains(phrase.as_str())
        {
            return false;
        }
        if !search.is_empty() {
            let have = words(hay);
            if !search
                .iter()
                .all(|q| have.iter().any(|w| w.starts_with(q.as_str())))
            {
                return false;
            }
        }
        true
    };
    let admits = |t: &Ticket, r: &Row| -> bool {
        if let Some(k) = &filter.kind
            && k != openwarrant_core::ticket::TICKET_PROFILE
            && r.kind.as_ref() != Some(k)
        {
            return false;
        }
        if !filter.labels.iter().all(|l| r.labels.contains(l)) {
            return false;
        }
        let state_holds = match &wanted_state {
            None => true,
            Some(Wanted::Fixed(f)) => r.state == *f,
            Some(Wanted::Declared(declared)) => r
                .declared_states
                .iter()
                .any(|d| !d.lapsed && &d.state == declared),
            Some(Wanted::Phase(_)) => false,
        };
        if !state_holds {
            return false;
        }
        if let Some(e) = &epic
            && r.part_of.as_ref() != Some(e)
        {
            return false;
        }
        if text.is_some() || !search.is_empty() {
            return matches_text(&haystack(t));
        }
        true
    };
    // A Warrant in another encoding: its type, its state (or phase), and
    // its title. Labels and epics are the light encoding's alone.
    let admits_other = |o: &crate::warrants::Row| -> bool {
        if let Some(k) = &filter.kind
            && &o.profile != k
        {
            return false;
        }
        if !filter.labels.is_empty() || epic.is_some() {
            return false;
        }
        let state_holds = match &wanted_state {
            None => true,
            Some(Wanted::Fixed(f)) => o.fixed_state() == Some(fixed_word(*f)),
            Some(Wanted::Declared(_)) => false,
            Some(Wanted::Phase(p)) => &o.state == p,
        };
        if !state_holds {
            return false;
        }
        if text.is_some() || !search.is_empty() {
            return matches_text(&format!("{}\n{}", o.id, o.title).to_lowercase());
        }
        true
    };
    let mut human = String::new();
    if tickets.is_empty() && others.rows.is_empty() {
        human.push_str("no Warrants yet: `war create \"what this work accomplishes\"`");
    }
    let order = sorted(&tickets, &claims);
    let listed: Vec<&crate::warrants::Row> = others
        .rows
        .iter()
        .filter(|o| filter.is_empty() || admits_other(o))
        .collect();
    let width = order
        .iter()
        .map(|t| t.id().len())
        .chain(listed.iter().map(|o| o.id.len()))
        .max()
        .unwrap_or(6);
    let mut rows = Vec::new();
    for t in order {
        let r = row_in(store, &tickets, t, &claims);
        if !filter.is_empty() && !admits(t, &r) {
            continue;
        }
        let holders: Vec<String> = r.claims.iter().map(|c| c.actor.clone()).collect();
        let held: Vec<String> = r
            .declared_states
            .iter()
            .filter(|d| !d.lapsed)
            .map(|d| {
                let short = d
                    .record
                    .strip_prefix(&format!("{}/", r.id))
                    .unwrap_or(&d.record);
                format!("{short} {}", d.state)
            })
            .collect();
        human.push_str(&format!(
            "{:<width$}  {:<11}  {:>5}  p{}  {}{}{}{}\n",
            r.id,
            r.state.as_str(),
            format!("{}/{}", r.done, r.total),
            r.priority,
            r.title,
            if holders.is_empty() {
                String::new()
            } else {
                format!("  [claimed: {}]", holders.join(", "))
            },
            r.promoted_to
                .as_ref()
                .map(|w| format!("  [promoted: {w}]"))
                .unwrap_or_default(),
            if held.is_empty() {
                String::new()
            } else {
                format!("  [{}]", held.join(", "))
            },
        ));
        // OW-WAR-0148 M5: inserted before the newline, empty for a ticket
        // with none of the new fields, so an existing ticket's line is as it
        // was.
        let tail = fields_tail(&r, true);
        if !tail.is_empty() {
            human.pop();
            human.push_str(&tail);
            human.push('\n');
        }
        // OW-WAR-0148 M13: how its ticks were earned, at the line's end; a
        // ticket with nothing ticked reads as it did.
        if let Some(c) = &r.ticks {
            human.pop();
            human.push_str(&format!("  [ticks: {}]", c.describe()));
            human.push('\n');
        }
        rows.push(r);
    }
    // M10: the Warrants in the other encodings, after the light ones, each
    // with its type; a directory Warrant's state is its journal's phase.
    for o in &listed {
        let progress = match (o.done, o.total) {
            (Some(d), Some(n)) => format!("{d}/{n}"),
            _ => "-".to_owned(),
        };
        let kind = if o.encoding == "directory" {
            o.profile.clone()
        } else {
            format!("{}, read in place", o.profile)
        };
        human.push_str(&format!(
            "{:<width$}  {:<11}  {:>5}  --  {}  [{kind}]\n",
            o.id,
            o.state.replace('_', " "),
            progress,
            o.title,
        ));
    }
    let any = !tickets.is_empty() || !others.rows.is_empty();
    if !filter.is_empty() && rows.is_empty() && listed.is_empty() && any {
        human.push_str("no Warrant matches");
    }
    let mut result = serde_json::json!({"schema": "oh.war/ticket-list/v1", "tickets": rows});
    if !others.rows.is_empty() {
        result["warrants"] = serde_json::to_value(&listed).unwrap_or_default();
    }
    if !filter.is_empty() {
        result["filter"] = serde_json::json!({
            "type": filter.kind,
            "labels": filter.labels,
            "state": filter.state,
            "text": filter.text,
            "search": filter.search,
            "epic": epic,
        });
    }
    let mut out = Outcome::ok(human.trim_end().to_owned(), result);
    for f in faults {
        out.report.push(crate::diagnostic::Diagnostic::warn(
            f.rule.clone(),
            f.file.clone().unwrap_or_default(),
            format!("skipped: {}", f.message),
        ));
    }
    for f in &others.faults {
        out.report.push(f.clone());
    }
    Ok(out)
}

/// A fixed state as `Row::fixed_state` names it.
const fn fixed_word(s: TicketState) -> &'static str {
    match s {
        TicketState::Open => "open",
        TicketState::InProgress => "in_progress",
        TicketState::Done => "done",
    }
}

/// `war show <ticket>`: the ticket as a person reads it, with who holds what.
pub fn show(store: &Store, query: &str) -> Result<Outcome, RepoError> {
    let (tickets, _) = store.load_all()?;
    let index = match resolve(&tickets, query) {
        Ok(Target::Ticket(n) | Target::Item(n, _)) => n,
        Err(d) => return Ok(Outcome::from_diagnostic(d)),
    };
    let t = &tickets[index];
    let claims = store.claims()?;
    let now = now_secs();
    let r = row_in(store, &tickets, t, &claims);
    // OW-WAR-0148 M13: every tick's level, and the Warrant's parts.
    let reader = super::ladder::Reader::new(store);
    let report = super::ladder::ticks_of(&reader, t);
    let mut md = format!("# {} — {}\n\n", t.id(), t.manifest.title);
    let declared = r.declared_states.clone();
    let ann = |record: Option<String>| {
        record.map_or_else(String::new, |rid| crate::states::annotate(&declared, &rid))
    };
    md.push_str(&format!(
        "{}{} · {}/{} done · priority {} · created {} by {} · `{}/`\n",
        r.state.as_str(),
        ann(Some(t.id().to_owned())),
        r.done,
        r.total,
        r.priority,
        &t.manifest.created_at[..t.manifest.created_at.len().min(10)],
        t.manifest.created_by,
        r.dir
    ));
    if let Some(w) = &t.manifest.promoted_to {
        md.push_str(&format!("Promoted to Warrant {w} for sign-off.\n"));
    }
    // OW-WAR-0148 M5: one line for the new fields, only when there are any.
    let mut facts = Vec::new();
    if let Some(k) = &t.manifest.kind {
        facts.push(format!("type {k}"));
    }
    if !t.manifest.labels.is_empty() {
        facts.push(format!("labels {}", t.manifest.labels.join(", ")));
    }
    if let Some(p) = &t.manifest.part_of {
        let title = tickets.iter().find(|x| x.id() == p).map_or_else(
            || " (unknown)".to_owned(),
            |x| format!(" ({})", x.manifest.title),
        );
        facts.push(format!("part of {p}{title}"));
    }
    // OW-WAR-0148 M15: a due date, when there is one.
    if let Some(d) = &t.manifest.due {
        facts.push(format!("due {d}"));
    }
    if let Some(n) = t.manifest.issue {
        facts.push(match &t.manifest.issue_url {
            Some(u) => format!("GitHub issue [#{n}]({u})"),
            None => format!("GitHub issue #{n}"),
        });
    }
    if !facts.is_empty() {
        md.push_str(&facts.join(" · "));
        md.push('\n');
    }
    let body = description(&t.intent);
    if !body.is_empty() {
        md.push('\n');
        md.push_str(&body);
        md.push('\n');
    }
    md.push_str("\n## Checklist\n\n");
    if t.checklist.items.is_empty() && r.tickets.is_some() {
        md.push_str("(no items of its own: the work is its Warrants, below)\n");
    } else if t.checklist.items.is_empty() {
        md.push_str("(no items: the Warrant is the work — `war add` breaks it down)\n");
    }
    for item in &t.checklist.items {
        if item.done {
            let view = report.items.get(&super::ladder::item_key(item));
            let on = item
                .done_on
                .as_deref()
                .map(|d| openwarrant_core::ticks::split_level(d).0);
            md.push_str(&format!(
                "- [x] {}{}{}{}{}{}\n",
                view.map(|v| format!("{} ", tick_marker(v)))
                    .unwrap_or_default(),
                item.text,
                item.id
                    .as_ref()
                    .map(|i| format!(" ({i})"))
                    .unwrap_or_default(),
                match (&item.done_by, on) {
                    (Some(by), Some(on)) => format!(" — done by {by}, {on}"),
                    (Some(by), None) => format!(" — done by {by}"),
                    _ => String::new(),
                } + &item
                    .note
                    .as_ref()
                    .map(|n| format!(": {n}"))
                    .unwrap_or_default(),
                view.and_then(|v| v.unbacked.as_ref().map(|why| (v.written, why)))
                    .map(|(w, why)| format!(
                        " — its [{}] marker is not believed: {why}",
                        w.map_or("?", Level::as_str)
                    ))
                    .unwrap_or_default(),
                ann(item.id.as_ref().map(|i| format!("{}/{i}", t.id())))
            ));
        } else {
            let line = open_item_line(store, &tickets, t, item, &claims, now, &report.checks);
            md.push_str(line.trim_end_matches('\n'));
            md.push_str(&ann(item.id.as_ref().map(|i| format!("{}/{i}", t.id()))));
            md.push('\n');
        }
    }
    let standings = if report.checks.kpis.is_empty() {
        Vec::new()
    } else {
        super::ladder::standings(&report.checks, &super::ladder::backing(t).kpi_runs)
    };
    if !report.checks.is_empty() {
        md.push_str(&checks_section(t, &report, &standings));
    }
    if let Some(c) = &r.ticks {
        md.push_str(&format!(
            "\nTicks: {}. claimed < observed < independent < signed; a claimed tick is the \
             performer's word, nothing checked it.\n",
            c.describe()
        ));
    }
    // OW-WAR-0148 M5: an epic lists its tickets, each with its state and
    // progress, and the epic's progress over them.
    let kids = super::children_of(&tickets, t.id());
    let mut children = Vec::new();
    if let Some(p) = r.tickets {
        md.push_str(&format!(
            "\n## Warrants in it ({}/{} done)\n\n",
            p.done, p.total
        ));
        for k in sorted_refs(kids, &claims) {
            let kr = row_in(store, &tickets, k, &claims);
            md.push_str(&format!(
                "- [{}] {} — {} · {} · {}/{} done{}\n",
                if kr.state == TicketState::Done {
                    'x'
                } else {
                    ' '
                },
                kr.id,
                kr.title,
                kr.state.as_str(),
                kr.done,
                kr.total,
                fields_tail(&kr, false)
            ));
            children.push(kr);
        }
    }
    let all = notes(&t.intent);
    if !all.is_empty() {
        md.push_str("\n## Notes\n\n");
        for n in &all {
            md.push_str(&format!("- {n}\n"));
        }
    }
    // M11: each revision `--if-rev` takes, the ticket's and every item's.
    let items: Vec<serde_json::Value> = t
        .checklist
        .items
        .iter()
        .map(|item| {
            let mut v = serde_json::to_value(item).unwrap_or_default();
            if let Some(o) = v.as_object_mut() {
                o.insert(
                    "revision".to_owned(),
                    serde_json::json!(super::item_revision(&t.checklist_text, item)),
                );
                // OW-WAR-0148 M13: the date without its level marker, and
                // the tick (or, open, the minimum it must reach).
                if let Some(on) = &item.done_on {
                    o.insert(
                        "done_on".to_owned(),
                        serde_json::json!(openwarrant_core::ticks::split_level(on).0),
                    );
                }
                if let Some(view) = report.items.get(&super::ladder::item_key(item)) {
                    o.insert("tick".to_owned(), serde_json::json!(view));
                    o.insert(
                        "tick_marker".to_owned(),
                        serde_json::json!(tick_marker(view)),
                    );
                } else if let Some(id) = item.id.as_deref() {
                    let (min, _) = super::ladder::minimum(store, t, &report.checks, Some(id));
                    if min > Level::Claimed || report.checks.milestone(id).is_some() {
                        o.insert("minimum".to_owned(), serde_json::json!(min));
                    }
                }
                if let Some(id) = item.id.as_deref()
                    && report.checks.milestone(id).is_some()
                {
                    o.insert("milestone".to_owned(), serde_json::json!(true));
                }
            }
            v
        })
        .collect();
    let mut result = serde_json::json!({
        "schema": "oh.war/ticket-show/v1",
        "ticket": r,
        "checks": if report.checks.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::json!({
                "tests": report.checks.tests,
                "kpis": standings,
                "milestones": report.checks.milestones,
                "faults": report.checks.faults,
            })
        },
        "revision": t.revision,
        "description": body,
        "items": items,
        "notes": all,
        "markdown": md,
    });
    if !children.is_empty() {
        result["tickets"] = serde_json::to_value(&children).unwrap_or_default();
    }
    Ok(Outcome::ok(md.trim_end().to_owned(), result))
}

/// `## Checks` in `war show`: each test, each KPI with its latest, best and
/// target, each milestone with its minimum and where its tick stands.
fn checks_section(
    t: &Ticket,
    report: &super::ladder::TickReport,
    standings: &[super::ladder::KpiStanding],
) -> String {
    let mut md = String::from("\n## Checks\n\n");
    for test in &report.checks.tests {
        md.push_str(&format!(
            "- test {}: `{}`{}\n",
            test.name,
            test.cmd,
            test.item
                .as_ref()
                .map(|i| format!(" (for {i})"))
                .unwrap_or_default()
        ));
    }
    for s in standings {
        md.push_str(&format!(
            "- KPI {} (`{}`)\n",
            super::ladder::describe_standing(s),
            s.cmd
        ));
    }
    for m in &report.checks.milestones {
        let item = t.item(&m.item);
        let text = item.map_or("(no such item)", |i| i.text.as_str());
        let at = match (item, report.items.get(&m.item)) {
            (Some(_), Some(v)) => format!("ticked {}", tick_marker(v)),
            (Some(_), None) => "open".to_owned(),
            (None, _) => "not in the checklist".to_owned(),
        };
        md.push_str(&format!(
            "- milestone {} {text}: min {}; {at}\n",
            m.item,
            m.min.map_or("claimed", Level::as_str)
        ));
    }
    for f in &report.checks.faults {
        md.push_str(&format!(
            "- line {} of {}: {} ({})\n",
            f.line,
            openwarrant_core::ticks::CHECKS_FILE,
            f.message,
            f.rule
        ));
    }
    md
}

/// `tickets` in `war view tickets`' order.
fn sorted_refs<'a>(mut tickets: Vec<&'a Ticket>, claims: &Claims) -> Vec<&'a Ticket> {
    let rank = |s: TicketState| match s {
        TicketState::InProgress => 0,
        TicketState::Open => 1,
        TicketState::Done => 2,
    };
    tickets.sort_by(|a, b| {
        (
            rank(state_of(a, claims)),
            a.manifest.priority,
            &a.manifest.created_at,
            a.id(),
        )
            .cmp(&(
                rank(state_of(b, claims)),
                b.manifest.priority,
                &b.manifest.created_at,
                b.id(),
            ))
    });
    tickets
}

/// `war view prime [<ticket>]`: what an arriving agent (or person) reads first.
/// Open tickets with only their remaining items, who holds which claim, recent
/// notes, and done tickets compacted: a done ticket older than
/// `[tickets] compact_after_days` is one line.
pub fn prime(store: &Store, only: Option<&str>) -> Result<Outcome, RepoError> {
    let (all, _) = store.load_all()?;
    let claims = store.claims()?;
    let now = now_secs();
    if let Some(q) = only {
        let index = match resolve(&all, q) {
            Ok(Target::Ticket(n) | Target::Item(n, _)) => n,
            Err(d) => return Ok(Outcome::from_diagnostic(d)),
        };
        return Ok(prime_one(store, &all, &all[index], &claims, now));
    }
    let today = today();
    let order = sorted(&all, &claims);
    let (open, done): (Vec<&Ticket>, Vec<&Ticket>) = order
        .into_iter()
        .partition(|t| state_of(t, &claims) != TicketState::Done);
    let recent_cut = today - chrono::Days::new(store.compact_days);
    let (recent, old): (Vec<&Ticket>, Vec<&Ticket>) = done
        .into_iter()
        .partition(|t| done_on(t).is_some_and(|d| d >= recent_cut));
    let in_progress = open
        .iter()
        .filter(|t| state_of(t, &claims) == TicketState::InProgress)
        .count();

    let mut md = format!("# Warrants — {}\n\n", store.project);
    md.push_str(&format!(
        "_Read this first. From `{}/` at {}: {} open ({in_progress} in progress), {} done._\n\n",
        store.rel(&store.dir),
        rfc3339(now)[..16].replace('T', " "),
        open.len(),
        recent.len() + old.len()
    ));
    md.push_str(
        "How to work: `war next` lists what can start now; `war claim <id>` takes one; do it; \
         `war done <id> --note \"what you did\"` ticks it. Leave anything the next person needs \
         with `war note <warrant> \"...\"`. No step needs a signature or anyone's approval.\n",
    );

    let mut held: Vec<String> = Vec::new();
    for (name, c) in &claims {
        let Some(c) = c else {
            held.push(format!("- `{name}` — unreadable lock"));
            continue;
        };
        let title = all
            .iter()
            .find(|t| t.id() == c.ticket)
            .map(|t| match &c.item {
                Some(i) => t
                    .item(i)
                    .map_or_else(|| t.manifest.title.clone(), |x| x.text.clone()),
                None => format!("{} (whole Warrant)", t.manifest.title),
            })
            .unwrap_or_default();
        let stale = if c.lease_expired(now) {
            " — its lease ran out: `war claim` takes it"
        } else if c.age(now) > store.ttl_secs {
            " — stale: `war claim --steal` may take it"
        } else {
            ""
        };
        held.push(format!(
            "- {} — {title} — {}, {} ago{stale}",
            c.target(),
            c.actor,
            ago(c.age(now))
        ));
    }
    if !held.is_empty() {
        md.push_str("\n## Claimed now\n\n");
        md.push_str(&held.join("\n"));
        md.push('\n');
    }

    md.push_str("\n## Open\n");
    if open.is_empty() {
        md.push_str("\nNothing open. `war create \"...\"` starts a Warrant.\n");
    }
    for t in &open {
        let (d, n) = t.checklist.progress();
        md.push_str(&format!(
            "\n### {} — {}\n\n{} · {d}/{n} done · priority {}\n",
            t.id(),
            t.manifest.title,
            state_of(t, &claims).as_str(),
            t.manifest.priority
        ));
        let body = description(&t.intent);
        if let Some(first) = body.split("\n\n").next().filter(|p| !p.trim().is_empty()) {
            md.push('\n');
            md.push_str(first.trim());
            md.push('\n');
        }
        let checks = super::ladder::checks_of(t);
        let remaining: Vec<String> = t
            .checklist
            .items
            .iter()
            .filter(|i| !i.done)
            .map(|i| open_item_line(store, &all, t, i, &claims, now, &checks))
            .collect();
        if remaining.is_empty() {
            md.push_str("\nNo items yet: the Warrant is the work.\n");
        } else {
            md.push_str("\nRemaining:\n\n");
            md.push_str(&remaining.concat());
        }
        let ns = notes(&t.intent);
        if !ns.is_empty() {
            md.push_str("\nRecent notes:\n\n");
            for n in ns.iter().rev().take(3).rev() {
                md.push_str(&format!("- {n}\n"));
            }
        }
    }

    if !recent.is_empty() {
        md.push_str(&format!(
            "\n## Done in the last {} days\n",
            store.compact_days
        ));
        for t in &recent {
            let (_, n) = t.checklist.progress();
            md.push_str(&format!(
                "\n- **{} — {}** (done {}, {n} item(s))",
                t.id(),
                t.manifest.title,
                done_on(t).map(|d| d.to_string()).unwrap_or_default()
            ));
            if let Some(last) = notes(&t.intent).last() {
                md.push_str(&format!("\n  last note: {}", last.replace('\n', " ")));
            }
            md.push('\n');
        }
    }
    if !old.is_empty() {
        md.push_str("\n## Done earlier\n\n");
        for t in &old {
            let (_, n) = t.checklist.progress();
            md.push_str(&format!(
                "- {} — {} ({}, {n} item(s))\n",
                t.id(),
                t.manifest.title,
                done_on(t).map_or_else(|| "done".to_owned(), |d| format!("done {d}"))
            ));
        }
    }
    let rows: Vec<Row> = open.iter().map(|t| row(store, t, &claims)).collect();
    Ok(Outcome::ok(
        md.trim_end().to_owned(),
        serde_json::json!({
            "schema": "oh.war/ticket-prime/v1",
            "open": rows,
            "done_recent": recent.iter().map(|t| t.id()).collect::<Vec<_>>(),
            "done_compacted": old.iter().map(|t| t.id()).collect::<Vec<_>>(),
            "claims": claims.values().flatten().collect::<Vec<_>>(),
            "markdown": md,
        }),
    ))
}

/// `war view prime <ticket>`: one ticket in full, remaining items only.
fn prime_one(store: &Store, all: &[Ticket], t: &Ticket, claims: &Claims, now: u64) -> Outcome {
    let (d, n) = t.checklist.progress();
    let checks = super::ladder::checks_of(t);
    let mut md = format!("# {} — {}\n\n", t.id(), t.manifest.title);
    md.push_str(&format!(
        "{} · {d}/{n} done · priority {} · `{}/`\n",
        state_of(t, claims).as_str(),
        t.manifest.priority,
        store.rel(&t.dir)
    ));
    let body = description(&t.intent);
    if !body.is_empty() {
        md.push('\n');
        md.push_str(&body);
        md.push('\n');
    }
    let remaining: Vec<String> = t
        .checklist
        .items
        .iter()
        .filter(|i| !i.done)
        .map(|i| open_item_line(store, all, t, i, claims, now, &checks))
        .collect();
    md.push_str("\n## Remaining\n\n");
    if remaining.is_empty() && n == 0 {
        md.push_str("No items: the Warrant is the work (`war claim` it whole, `war done` it).\n");
    } else if remaining.is_empty() {
        md.push_str("Nothing: every item is done.\n");
    } else {
        md.push_str(&remaining.concat());
    }
    if d > 0 {
        md.push_str(&format!(
            "\n{d} item(s) done; `war show {}` lists them.\n",
            t.id()
        ));
    }
    let ns = notes(&t.intent);
    if !ns.is_empty() {
        md.push_str("\n## Notes\n\n");
        for n in &ns {
            md.push_str(&format!("- {n}\n"));
        }
    }
    Outcome::ok(
        md.trim_end().to_owned(),
        serde_json::json!({
            "schema": "oh.war/ticket-prime/v1",
            "ticket": row(store, t, claims),
            "markdown": md,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ages_read_as_a_person_would_say_them() {
        assert_eq!(ago(5), "5s");
        assert_eq!(ago(90), "1m");
        assert_eq!(ago(7200), "2h");
        assert_eq!(ago(7500), "2h 5m");
        assert_eq!(ago(200_000), "2d 7h");
    }

    #[test]
    fn a_description_is_the_intent_without_title_or_notes() {
        let intent = "# Title\n\nWhy this matters.\n\nMore context.\n\n## Notes\n\n- **2026-09-25 10:00 UTC, claude:** chose A\n  because B\n- **2026-09-25 11:00 UTC, bob:** ok\n";
        assert_eq!(description(intent), "Why this matters.\n\nMore context.");
        assert_eq!(
            notes(intent),
            vec![
                "**2026-09-25 10:00 UTC, claude:** chose A\n  because B".to_owned(),
                "**2026-09-25 11:00 UTC, bob:** ok".to_owned()
            ]
        );
    }
}
