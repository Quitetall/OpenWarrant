// SPDX-License-Identifier: Apache-2.0
//! OW-WAR-0070: a pull-only view, never an authority or completion record.
pub mod classify;
use crate::{
    journal_cmd, questions,
    repo::{RepoError, Repository},
    sign::{self, Pending},
};
use chrono::{DateTime, FixedOffset};
use classify::{HumanAct, NextAct, Warrant};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Write as _};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub alias: String,
    pub title: String,
    pub state: openwarrant_core::Phase,
    pub awaited_act: HumanAct,
    pub waiting_since: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inbox {
    pub api_version: String,
    pub namespace: String,
    pub generated_at: String,
    pub items: Vec<Item>,
}
fn timestamp(s: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(s).ok()
}

// Existing tolerant readers treat some filesystem errors as absence. Check
// their containers before calling them; reuse those readers for record syntax.
fn check_path(path: &camino::Utf8Path, directory: bool) -> Result<(), RepoError> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(RepoError::Io {
                context: format!("could not inspect {path}"),
                source,
            });
        }
        Ok(_) => {}
    }
    let metadata = std::fs::metadata(path).map_err(|source| RepoError::Io {
        context: format!("could not inspect {path}"),
        source,
    })?;
    if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
        return Err(RepoError::Message(format!(
            "{path}: wrong record container type"
        )));
    }
    if directory {
        for entry in std::fs::read_dir(path).map_err(|source| RepoError::Io {
            context: format!("could not read {path}"),
            source,
        })? {
            entry.map_err(|source| RepoError::Io {
                context: format!("could not read entry in {path}"),
                source,
            })?;
        }
    }
    Ok(())
}

pub fn run(repo: &Repository) -> Result<Inbox, RepoError> {
    let generated_at = crate::gate_cmd::receipt::now_rfc3339_public();
    let now = timestamp(&generated_at)
        .ok_or_else(|| RepoError::Message("system time is not RFC 3339".into()))?;
    // Some aggregate readers tolerate damaged records. The inbox must not turn
    // that damage into an apparently empty list, so load its inputs strictly.
    check_path(&repo.warrants_dir(), true)?;
    check_path(&repo.root.join("docs/authority/roles.toml"), false)?;
    let mut inputs = BTreeMap::new();
    for dir in repo.warrant_dirs()? {
        let one = repo.load_warrant(&dir)?;
        if one.validated.is_none()
            || one.basis.is_none()
            || one
                .report
                .diagnostics
                .iter()
                .any(|d| d.severity == crate::diagnostic::Severity::Error)
        {
            return Err(RepoError::Message(format!(
                "{}: Warrant could not be loaded",
                one.alias()
            )));
        }
        for file in [
            "authorization.toml",
            "resolution.toml",
            "deliverables.toml",
            "judgments.toml",
            "rationale.toml",
            journal_cmd::FILE,
        ] {
            check_path(&dir.join(file), false)?;
        }
        for directory in [questions::DIR, "verifications", "corrections"] {
            check_path(&dir.join(directory), true)?;
        }
        repo.load_authorization(&dir)?;
        repo.load_resolution(&dir)?;
        for failures in [
            repo.load_verifications(&dir)?.failures,
            repo.load_deliverables(&dir)?.failures,
            repo.load_corrections(&dir)?.failures,
        ] {
            if let Some((path, message)) = failures.first() {
                return Err(RepoError::Message(format!("{path}: {message}")));
            }
        }
        repo.load_judgments(&dir)?;
        repo.load_rationale(&dir)?;
        let qs = questions::load(repo, &one.alias())?;
        if qs.iter().any(|q| q.warrant != one.alias()) {
            return Err(RepoError::Message(format!(
                "{}: question belongs to another Warrant",
                one.alias()
            )));
        }
        let journal = journal_cmd::load(&dir)?;
        if journal.events.iter().any(|e| {
            Some(e.warrant_uuid.as_str())
                != one
                    .validated
                    .as_ref()
                    .map(|v| v.uuid.to_string())
                    .as_deref()
        }) {
            return Err(RepoError::Message(format!(
                "{}: journal belongs to another Warrant",
                one.alias()
            )));
        }
        inputs.insert(one.alias(), (qs, journal));
    }
    let pending = sign::pending(repo)?;
    let status = crate::status::build(repo)?;
    let mut items = Vec::new();
    for w in status.warrants {
        let phase = w
            .state
            .ok_or_else(|| RepoError::Message(format!("{}: state unavailable", w.alias)))?
            .phase;
        let (qs, journal) = inputs.remove(&w.alias).ok_or_else(|| {
            RepoError::Message(format!("{}: records changed during inbox read", w.alias))
        })?;
        let pending = pending.iter().find_map(|p| match p {
            Pending::Authorize { alias, .. } if alias == &w.alias => Some(HumanAct::Authorize),
            Pending::Correct { alias, .. } if alias == &w.alias => Some(HumanAct::Correct),
            Pending::Resolve { alias, .. } if alias == &w.alias => Some(HumanAct::Resolve),
            _ => None,
        });
        let NextAct::Human(awaited_act) = classify::next_act(&Warrant {
            phase,
            pending,
            blocking_question: qs.iter().any(|q| q.is_open() && q.blocking),
        }) else {
            continue;
        };
        // Journal order is the recorded order. An invalid newest time must not
        // silently fall back to an older event. Receipts and dispatch reads do
        // not move a lifecycle transition or reset the wait age.
        let waiting_since = journal
            .events
            .iter()
            .rev()
            .find(|e| {
                matches!(
                    e.event_type.as_str(),
                    journal_cmd::DRAFT_CREATED
                        | journal_cmd::AUTHORIZATION_RECORDED
                        | journal_cmd::VERIFICATION_RECORDED
                        | journal_cmd::RESOLUTION_RECORDED
                        | journal_cmd::CORRECTION_RECORDED
                        | questions::EVENT_ASKED
                        | questions::EVENT_ANSWERED
                )
            })
            .and_then(|e| {
                timestamp(&e.occurred_at)
                    .filter(|t| t <= &now)
                    .map(|_| e.occurred_at.clone())
            });
        items.push(Item {
            alias: w.alias,
            title: w.title.unwrap_or_default(),
            state: phase,
            awaited_act,
            waiting_since,
        });
    }
    items.sort_by(|a, b| {
        match (
            a.waiting_since.as_deref().and_then(timestamp),
            b.waiting_since.as_deref().and_then(timestamp),
        ) {
            (Some(a_time), Some(b_time)) => a_time.cmp(&b_time).then(a.alias.cmp(&b.alias)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.alias.cmp(&b.alias),
        }
    });
    Ok(Inbox {
        api_version: "oh.war/inbox/v1".into(),
        namespace: repo.config.project.namespace.as_str().to_owned(),
        generated_at,
        items,
    })
}
fn cell(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
pub fn render(inbox: &Inbox) -> String {
    if inbox.items.is_empty() {
        return "Nothing waiting on a human.\n".into();
    }
    let mut text = "WARRANT\tTITLE\tSTATE\tAWAITED ACT\tAGE\n".to_owned();
    let now = timestamp(&inbox.generated_at);
    for item in &inbox.items {
        let age = item
            .waiting_since
            .as_deref()
            .and_then(timestamp)
            .zip(now)
            .map_or_else(
                || "unknown".into(),
                |(since, now)| format!("{}s", (now - since).num_seconds()),
            );
        let act = match item.awaited_act {
            HumanAct::Answer => "answer",
            HumanAct::Authorize => "authorize",
            HumanAct::Correct => "correct",
            HumanAct::Resolve => "resolve",
        };
        let _ = writeln!(
            text,
            "{}\t{}\t{}\t{}\t{}",
            cell(&item.alias),
            cell(&item.title),
            item.state,
            act,
            age
        );
    }
    text
}
