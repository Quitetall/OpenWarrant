// SPDX-License-Identifier: Apache-2.0
//! Two corpus projections for the progress platform (slice D2), compiled by
//! `war compile` and drift-checked by `war check --generated`:
//!
//! - `CORPUS_TIMELINE.json` (`oh.war/corpus-timeline/v1`): every journal
//!   event of every Warrant, sorted by `(occurred_at, warrant, id)`, with a
//!   per-day histogram. Read from committed `journal.jsonl` files only, so it
//!   reproduces from a fresh clone.
//! - `CORPUS_PENDING.json` (`oh.war/corpus-pending/v1`): the human acts that
//!   await a signature, each with the command that performs it. Derived from
//!   the same records `war next` reads; tracked inputs only.
//!
//! Both are canonical JSON (RFC 8785), so two compilations of one tree are
//! byte-identical.

use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use serde::Serialize;

use crate::repo::{RepoError, Repository};

pub const TIMELINE_SCHEMA: &str = "oh.war/corpus-timeline/v1";
pub const PENDING_SCHEMA: &str = "oh.war/corpus-pending/v1";

#[derive(Debug, Clone, Serialize)]
pub struct TimelineEvent {
    pub occurred_at: String,
    pub warrant: String,
    pub id: String,
    pub event_type: String,
    pub class: String,
    pub actor_ref: String,
    /// The event's payload parsed as JSON when it is JSON; otherwise the
    /// string as recorded. Never a panic on a malformed payload.
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct Day {
    pub date: String,
    pub events: usize,
    pub by_type: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Timeline {
    pub schema: String,
    pub events: Vec<TimelineEvent>,
    pub days: Vec<Day>,
    pub by_type: BTreeMap<String, usize>,
    pub warrants: usize,
    /// Events whose `occurred_at` is not an RFC 3339 UTC timestamp. They are
    /// kept (a record is never dropped from its own history) and bucketed
    /// under the day `malformed-timestamp`; this count says how many.
    pub malformed_timestamps: usize,
}

pub fn build_timeline(repo: &Repository) -> Result<Timeline, RepoError> {
    let mut events = Vec::new();
    let mut warrants = 0usize;
    for dir in repo.warrant_dirs()? {
        let Ok(loaded) = repo.load_warrant(&dir) else {
            continue;
        };
        let alias = loaded.alias();
        let Ok(journal) = crate::journal_cmd::load(&dir) else {
            continue;
        };
        warrants += 1;
        for e in &journal.events {
            let payload = serde_json::from_str::<serde_json::Value>(&e.payload)
                .unwrap_or_else(|_| serde_json::Value::String(e.payload.clone()));
            events.push(TimelineEvent {
                occurred_at: e.occurred_at.clone(),
                warrant: alias.clone(),
                id: e.id.clone(),
                event_type: e.event_type.clone(),
                // The record's own serialisation of the class, never its
                // Debug name.
                class: serde_json::to_value(e.class)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default(),
                actor_ref: e.actor_ref.clone(),
                payload,
            });
        }
    }
    events.sort_by(|a, b| {
        (&a.occurred_at, &a.warrant, &a.id).cmp(&(&b.occurred_at, &b.warrant, &b.id))
    });
    let mut by_type: BTreeMap<String, usize> = BTreeMap::new();
    let mut days: BTreeMap<String, Day> = BTreeMap::new();
    let mut malformed_timestamps = 0usize;
    for e in &events {
        *by_type.entry(e.event_type.clone()).or_insert(0) += 1;
        let date = if openwarrant_core::timestamp::validate_rfc3339_utc(&e.occurred_at).is_ok() {
            e.occurred_at[..10].to_owned()
        } else {
            malformed_timestamps += 1;
            "malformed-timestamp".to_owned()
        };
        let day = days.entry(date.clone()).or_insert_with(|| Day {
            date,
            events: 0,
            by_type: BTreeMap::new(),
        });
        day.events += 1;
        *day.by_type.entry(e.event_type.clone()).or_insert(0) += 1;
    }
    Ok(Timeline {
        schema: TIMELINE_SCHEMA.to_owned(),
        events,
        days: days.into_values().collect(),
        by_type,
        warrants,
        malformed_timestamps,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct PendingAct {
    pub warrant: String,
    /// `authorize` | `resolve` | `correct` | `accept`.
    pub action: String,
    /// `war sign <target>`, verbatim.
    pub command: String,
    pub why: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PendingSet {
    pub schema: String,
    pub acts: Vec<PendingAct>,
    pub count: usize,
}

pub fn build_pending(repo: &Repository) -> Result<PendingSet, RepoError> {
    let next = crate::next::run(repo)?;
    let mut acts: Vec<PendingAct> = next
        .actions
        .iter()
        .filter(|a| a.actor == crate::next::Actor::Human)
        .map(|a| PendingAct {
            warrant: a.warrant.clone(),
            action: a.action.clone(),
            command: a.command.clone(),
            why: a.why.clone(),
        })
        .collect();
    acts.sort_by(|a, b| (&a.warrant, &a.action).cmp(&(&b.warrant, &b.action)));
    Ok(PendingSet {
        schema: PENDING_SCHEMA.to_owned(),
        count: acts.len(),
        acts,
    })
}

fn canonical<T: Serialize>(v: &T, what: &str) -> Result<String, RepoError> {
    serde_jcs::to_string(v)
        .map(|s| s + "\n")
        .map_err(|e| RepoError::Message(format!("could not canonicalise {what}: {e}")))
}

/// The timeline projection, with its path.
pub fn corpus_timeline_json(repo: &Repository) -> Result<(Utf8PathBuf, String), RepoError> {
    let path = repo
        .root
        .join(&repo.config.paths.warrants)
        .join("generated")
        .join("CORPUS_TIMELINE.json");
    Ok((
        path,
        canonical(&build_timeline(repo)?, "CORPUS_TIMELINE.json")?,
    ))
}

/// The pending projection, with its path.
pub fn corpus_pending_json(repo: &Repository) -> Result<(Utf8PathBuf, String), RepoError> {
    let path = repo
        .root
        .join(&repo.config.paths.warrants)
        .join("generated")
        .join("CORPUS_PENDING.json");
    Ok((
        path,
        canonical(&build_pending(repo)?, "CORPUS_PENDING.json")?,
    ))
}
