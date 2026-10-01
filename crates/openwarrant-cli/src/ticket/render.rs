// SPDX-License-Identifier: Apache-2.0
//! What a person or an arriving agent reads: `war prime`, `war show`,
//! `war tickets`. Plain Markdown, in the words of the work — no rule names,
//! no section numbers.

use std::collections::BTreeMap;

use serde::Serialize;

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
                let stale = if c.age(now) > store.ttl_secs {
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
    if notes.is_empty() {
        String::new()
    } else {
        format!(" — {}", notes.join("; "))
    }
}

fn open_item_line(
    store: &Store,
    tickets: &[Ticket],
    t: &Ticket,
    item: &openwarrant_core::ticket::Item,
    claims: &Claims,
    now: u64,
) -> String {
    format!(
        "- [ ] {}{}{}\n",
        item.text,
        item.id
            .as_ref()
            .map(|i| format!(" ({i})"))
            .unwrap_or_default(),
        annotation(store, tickets, t, item, claims, now)
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

/// `war tickets` (`war ls`): every ticket, its state and progress.
pub fn tickets(store: &Store) -> Result<Outcome, RepoError> {
    let (tickets, faults) = store.load_all()?;
    let claims = store.claims()?;
    let mut human = String::new();
    if tickets.is_empty() {
        human.push_str("no tickets yet: `war create \"what this work accomplishes\"`");
    }
    let order = sorted(&tickets, &claims);
    let width = order.iter().map(|t| t.id().len()).max().unwrap_or(6);
    let mut rows = Vec::new();
    for t in order {
        let r = row(store, t, &claims);
        let holders: Vec<String> = r.claims.iter().map(|c| c.actor.clone()).collect();
        human.push_str(&format!(
            "{:<width$}  {:<11}  {:>5}  p{}  {}{}{}\n",
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
        ));
        rows.push(r);
    }
    let mut out = Outcome::ok(
        human.trim_end().to_owned(),
        serde_json::json!({"schema": "oh.war/ticket-list/v1", "tickets": rows}),
    );
    for f in faults {
        out.report.push(crate::diagnostic::Diagnostic::warn(
            f.rule.clone(),
            f.file.clone().unwrap_or_default(),
            format!("skipped: {}", f.message),
        ));
    }
    Ok(out)
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
    let r = row(store, t, &claims);
    let mut md = format!("# {} — {}\n\n", t.id(), t.manifest.title);
    md.push_str(&format!(
        "{} · {}/{} done · priority {} · created {} by {} · `{}/`\n",
        r.state.as_str(),
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
    let body = description(&t.intent);
    if !body.is_empty() {
        md.push('\n');
        md.push_str(&body);
        md.push('\n');
    }
    md.push_str("\n## Checklist\n\n");
    if t.checklist.items.is_empty() {
        md.push_str("(no items: the ticket is the work — `war add` breaks it down)\n");
    }
    for item in &t.checklist.items {
        if item.done {
            md.push_str(&format!(
                "- [x] {}{}{}\n",
                item.text,
                item.id
                    .as_ref()
                    .map(|i| format!(" ({i})"))
                    .unwrap_or_default(),
                match (&item.done_by, &item.done_on) {
                    (Some(by), Some(on)) => format!(" — done by {by}, {on}"),
                    (Some(by), None) => format!(" — done by {by}"),
                    _ => String::new(),
                } + &item
                    .note
                    .as_ref()
                    .map(|n| format!(": {n}"))
                    .unwrap_or_default()
            ));
        } else {
            md.push_str(&open_item_line(store, &tickets, t, item, &claims, now));
        }
    }
    let all = notes(&t.intent);
    if !all.is_empty() {
        md.push_str("\n## Notes\n\n");
        for n in &all {
            md.push_str(&format!("- {n}\n"));
        }
    }
    Ok(Outcome::ok(
        md.trim_end().to_owned(),
        serde_json::json!({
            "schema": "oh.war/ticket-show/v1",
            "ticket": r,
            "description": body,
            "items": t.checklist.items,
            "notes": all,
            "markdown": md,
        }),
    ))
}

/// `war prime [<ticket>]`: what an arriving agent (or person) reads first.
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

    let mut md = format!("# Tickets — {}\n\n", store.project);
    md.push_str(&format!(
        "_Read this first. From `{}/` at {}: {} open ({in_progress} in progress), {} done._\n\n",
        store.rel(&store.dir),
        rfc3339(now)[..16].replace('T', " "),
        open.len(),
        recent.len() + old.len()
    ));
    md.push_str(
        "How to work: `war ready` lists what can start now; `war claim <id>` takes one; do it; \
         `war done <id> --note \"what you did\"` ticks it. Leave anything the next person needs \
         with `war note <ticket> \"...\"`. No step needs a signature or anyone's approval.\n",
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
                None => format!("{} (whole ticket)", t.manifest.title),
            })
            .unwrap_or_default();
        let stale = if c.age(now) > store.ttl_secs {
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
        md.push_str("\nNothing open. `war create \"...\"` starts a ticket.\n");
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
        let remaining: Vec<String> = t
            .checklist
            .items
            .iter()
            .filter(|i| !i.done)
            .map(|i| open_item_line(store, &all, t, i, &claims, now))
            .collect();
        if remaining.is_empty() {
            md.push_str("\nNo items yet: the ticket is the work.\n");
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

/// `war prime <ticket>`: one ticket in full, remaining items only.
fn prime_one(store: &Store, all: &[Ticket], t: &Ticket, claims: &Claims, now: u64) -> Outcome {
    let (d, n) = t.checklist.progress();
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
        .map(|i| open_item_line(store, all, t, i, claims, now))
        .collect();
    md.push_str("\n## Remaining\n\n");
    if remaining.is_empty() && n == 0 {
        md.push_str("No items: the ticket is the work (`war claim` it whole, `war done` it).\n");
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
