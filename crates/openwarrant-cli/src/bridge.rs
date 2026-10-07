// SPDX-License-Identifier: Apache-2.0
//! `war admin bridge claude-tasks` (OW-WAR-0148 M13): a harness's own task list
//! feeds the tick ladder instead of competing with it.
//!
//! # What it reads
//!
//! Claude Code keeps a task list per list id under `~/.claude/tasks/<id>/`
//! (`CLAUDE_CODE_TASK_LIST_ID` names a shared one), and its tasks are
//! pending, in progress or completed; a `TaskCompleted` hook receives
//! `task_id`, `task_subject` and, when there is one, `task_description`
//! (code.claude.com/docs/en/hooks, "TaskCompleted"; /docs/en/interactive-mode,
//! "Task list"; /docs/en/agent-teams, "Assign and claim tasks"). The layout of
//! a task file inside that directory is not documented, so this reads the
//! documented vocabulary and nothing else: each `*.json` there (or `--file`)
//! holding a task, an array of tasks or `{"tasks": [...]}`, each task an
//! object whose id is `id` or `task_id`, whose subject is `subject` or
//! `task_subject`, whose description is `description` or `task_description`,
//! and whose status is `status` or `task_status`. A file that is not that is
//! named UNKNOWN and skipped, never guessed at.
//!
//! # What it does
//!
//! A completed task whose subject or description names exactly one item of
//! this repository (`t-x/i-y`, or an `i-y` only one ticket has) proposes
//! `war done <item> --check` when the item has something to check, and
//! `war done <item>` (claimed) when it has nothing. Without `--apply` it
//! prints what it would do and writes nothing; with it, it claims the item
//! when nobody holds it and ticks it through the same command, with every
//! refusal that command has. A tick it could not make is said, by rule, and
//! never fails the bridge: the bridge never stands between an agent and its
//! work. A task naming nothing, or two things, does nothing.

use camino::Utf8PathBuf;
use openwarrant_core::ticks::Level;
use serde::Serialize;

use crate::diagnostic::Diagnostic;
use crate::repo::RepoError;
use crate::ticket::{self, Outcome, Store, Target};

/// Where the tasks come from.
#[derive(Debug, Clone)]
pub enum Source {
    /// `~/.claude/tasks/$CLAUDE_CODE_TASK_LIST_ID/`.
    Default,
    Dir(Utf8PathBuf),
    File(Utf8PathBuf),
    /// A `TaskCompleted` hook's input: a file, or `-` for stdin.
    Event(String),
}

/// One task as read.
#[derive(Debug, Clone, Serialize)]
pub struct Task {
    pub id: String,
    pub subject: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub status: String,
    /// The file it came from.
    pub source: String,
}

/// What the bridge decided for one task.
#[derive(Debug, Clone, Serialize)]
pub struct Proposal {
    pub task: Task,
    /// The item it names, resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// The command it would run, or ran.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Why nothing is done, when nothing is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nothing: Option<String>,
    /// With --apply: what the command answered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied: Option<String>,
}

fn field(v: &serde_json::Value, names: &[&str]) -> Option<String> {
    names.iter().find_map(|n| match v.get(*n) {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        _ => None,
    })
}

/// The tasks in one JSON document, or why it holds none we can read.
fn tasks_in(text: &str, source: &str) -> Result<Vec<Task>, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    let list = match &value {
        serde_json::Value::Array(a) => a.clone(),
        serde_json::Value::Object(o) => match o.get("tasks") {
            Some(serde_json::Value::Array(a)) => a.clone(),
            _ => vec![value.clone()],
        },
        _ => return Err("neither a task, a list of tasks, nor {\"tasks\": [...]}".to_owned()),
    };
    let mut out = Vec::new();
    for t in &list {
        let (Some(id), Some(subject)) = (
            field(t, &["id", "task_id"]),
            field(t, &["subject", "task_subject"]),
        ) else {
            return Err(
                "a task without an id (`id` or `task_id`) and a subject (`subject` or \
                 `task_subject`)"
                    .to_owned(),
            );
        };
        out.push(Task {
            id,
            subject,
            description: field(t, &["description", "task_description"]).unwrap_or_default(),
            status: field(t, &["status", "task_status"]).unwrap_or_default(),
            source: source.to_owned(),
        });
    }
    Ok(out)
}

/// The documented default list directory, when its id is set.
fn default_dir() -> Option<Utf8PathBuf> {
    let id = std::env::var("CLAUDE_CODE_TASK_LIST_ID")
        .ok()
        .filter(|s| !s.trim().is_empty())?;
    let home = std::env::var("HOME").ok()?;
    Some(Utf8PathBuf::from(home).join(".claude/tasks").join(id))
}

/// Every item a text names: `t-x/i-y`, a bare `i-y`, or a bare `t-x` (a
/// Warrant with no items is its own one tick).
fn named_refs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for token in text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '/')) {
        let token = token.trim_matches(|c| c == '-' || c == '/');
        let ok = match token.split_once('/') {
            Some((t, i)) => {
                openwarrant_core::ticket::is_ticket_id(t) && openwarrant_core::ticket::is_item_id(i)
            }
            None => {
                openwarrant_core::ticket::is_item_id(token)
                    || openwarrant_core::ticket::is_ticket_id(token)
            }
        };
        if ok && !out.iter().any(|o| o == token) {
            out.push(token.to_owned());
        }
    }
    out
}

fn read_source(source: &Source) -> Result<(String, Vec<Task>, Vec<Diagnostic>), String> {
    let mut faults = Vec::new();
    match source {
        Source::Event(from) => {
            let text = if from == "-" {
                let mut s = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut s)
                    .map_err(|e| format!("could not read the hook input on stdin: {e}"))?;
                s
            } else {
                std::fs::read_to_string(from).map_err(|e| format!("could not read {from}: {e}"))?
            };
            let v: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| format!("the hook input is not JSON: {e}"))?;
            if let Some(event) = v.get("hook_event_name").and_then(|e| e.as_str())
                && event != "TaskCompleted"
            {
                return Ok((format!("a {event} event"), Vec::new(), faults));
            }
            let mut tasks = tasks_in(&text, "hook input")?;
            // The event is the completion itself.
            for t in &mut tasks {
                t.status = "completed".to_owned();
            }
            Ok(("a TaskCompleted hook".to_owned(), tasks, faults))
        }
        Source::File(path) => {
            let text =
                std::fs::read_to_string(path).map_err(|e| format!("could not read {path}: {e}"))?;
            let tasks = tasks_in(&text, path.as_str()).map_err(|why| format!("{path}: {why}"))?;
            Ok((path.to_string(), tasks, faults))
        }
        Source::Dir(_) | Source::Default => {
            let dir = match source {
                Source::Dir(d) => d.clone(),
                _ => default_dir().ok_or_else(|| {
                    "no task list named: Claude Code keeps one per list id under \
                     ~/.claude/tasks/<id>/; set CLAUDE_CODE_TASK_LIST_ID, or give --dir or --file"
                        .to_owned()
                })?,
            };
            let mut files: Vec<Utf8PathBuf> = std::fs::read_dir(&dir)
                .map_err(|e| format!("could not read {dir}: {e}"))?
                .filter_map(Result::ok)
                .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
                .filter(|p| p.extension() == Some("json"))
                .collect();
            files.sort();
            let mut tasks = Vec::new();
            for f in &files {
                match std::fs::read_to_string(f)
                    .map_err(|e| e.to_string())
                    .and_then(|t| tasks_in(&t, f.as_str()))
                {
                    Ok(mut t) => tasks.append(&mut t),
                    Err(why) => faults.push(Diagnostic::unknown(
                        "bridge.task-unreadable",
                        f.to_string(),
                        format!("UNKNOWN: {why}; skipped, nothing guessed from it"),
                    )),
                }
            }
            Ok((dir.to_string(), tasks, faults))
        }
    }
}

/// `war admin bridge claude-tasks [--dir|--file|--event] [--apply]`.
pub fn claude_tasks(store: &Store, source: &Source, apply: bool) -> Result<Outcome, RepoError> {
    let (from, tasks, faults) = match read_source(source) {
        Ok(v) => v,
        Err(why) => {
            return Ok(Outcome::refused(
                "bridge.no-tasks",
                String::new(),
                format!("{why}; nothing was read and nothing was written"),
            ));
        }
    };
    let mut proposals = Vec::new();
    let mut report = crate::diagnostic::Report::default();
    for d in faults {
        report.push(d);
    }
    for task in tasks {
        let p = propose(store, task, apply, &mut report)?;
        proposals.push(p);
    }
    let acting = proposals.iter().filter(|p| p.command.is_some()).count();
    let mut human = format!(
        "{} task(s) from {from}: {acting} to tick, {} with nothing to do",
        proposals.len(),
        proposals.len() - acting
    );
    for p in &proposals {
        human.push_str(&format!(
            "\n  {} {:?} ({}): {}",
            p.task.id,
            p.task.subject,
            if p.task.status.is_empty() {
                "no status"
            } else {
                p.task.status.as_str()
            },
            match (&p.command, &p.nothing, &p.applied) {
                (Some(c), _, Some(a)) => format!("{c} -> {a}"),
                (Some(c), _, None) => format!("would run {c}"),
                (None, Some(n), _) => n.clone(),
                (None, None, _) => "nothing".to_owned(),
            }
        ));
    }
    if !apply && acting > 0 {
        human.push_str("\nnothing was written; --apply ticks them");
    }
    Ok(Outcome {
        report,
        human,
        result: serde_json::json!({
            "schema": "oh.war/bridge-claude-tasks/v1",
            "source": from,
            "applied": apply,
            "tasks": proposals,
        }),
    })
}

fn propose(
    store: &Store,
    task: Task,
    apply: bool,
    report: &mut crate::diagnostic::Report,
) -> Result<Proposal, RepoError> {
    let mut p = Proposal {
        task,
        target: None,
        command: None,
        nothing: None,
        applied: None,
    };
    let completed = matches!(p.task.status.as_str(), "completed" | "complete" | "done");
    let refs = named_refs(&format!("{}\n{}", p.task.subject, p.task.description));
    let (tickets, _) = store.load_all()?;
    let mut resolved: Vec<(usize, Option<String>, String)> = Vec::new();
    for r in &refs {
        if let Ok(target) = ticket::resolve(&tickets, r) {
            let (n, item) = match target {
                Target::Ticket(n) => (n, None),
                Target::Item(n, i) => (n, Some(i)),
            };
            let name = item.as_ref().map_or_else(
                || tickets[n].id().to_owned(),
                |i| format!("{}/{i}", tickets[n].id()),
            );
            if !resolved.iter().any(|(_, _, x)| *x == name) {
                resolved.push((n, item, name));
            }
        }
    }
    let [(n, item, name)] = resolved.as_slice() else {
        p.nothing = Some(if resolved.is_empty() {
            "names no item of this repository; nothing".to_owned()
        } else {
            format!(
                "names {} items ({}); a task ticks one, so nothing",
                resolved.len(),
                resolved
                    .iter()
                    .map(|(_, _, x)| x.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        });
        return Ok(p);
    };
    p.target = Some(name.clone());
    if !completed {
        p.nothing = Some(format!(
            "not completed in Claude Code ({}); nothing to tick yet",
            if p.task.status.is_empty() {
                "no status"
            } else {
                p.task.status.as_str()
            }
        ));
        return Ok(p);
    }
    let t = &tickets[*n];
    let checks = ticket::ladder::checks_of(t);
    let (tests, kpis) = checks.for_item(item.as_deref());
    let can_check = !tests.is_empty() || kpis.iter().any(|k| k.gates());
    let done_item = item.as_ref().and_then(|i| t.item(i)).filter(|i| i.done);
    if let Some(it) = done_item {
        let reader = ticket::ladder::Reader::new(store);
        let level =
            ticket::ladder::view_of(&reader, t, it, &checks, &ticket::ladder::backing(t)).level;
        if level >= Level::Observed || !can_check {
            p.nothing = Some(format!("already ticked at {}; nothing", level.as_str()));
            return Ok(p);
        }
    } else if item.is_none() && t.checklist.is_done() {
        p.nothing = Some("already done; nothing".to_owned());
        return Ok(p);
    }
    let reach = if can_check {
        Level::Observed
    } else {
        Level::Claimed
    };
    let (minimum, _) = ticket::ladder::minimum(store, t, &checks, item.as_deref());
    if done_item.is_none() && reach < minimum {
        p.nothing = Some(format!(
            "ticks at {} or above, which a completed task cannot show: {}",
            minimum.as_str(),
            ticket::ladder::command_for(minimum, name)
        ));
        return Ok(p);
    }
    let command = if can_check {
        format!("war done {name} --check")
    } else {
        format!("war done {name}")
    };
    p.command = Some(command);
    if !apply {
        return Ok(p);
    }
    // Claim it when nobody holds it, so the tick goes through `war done`'s
    // own refusals; one somebody else holds is theirs, and nothing is done.
    if done_item.is_none() {
        let claims = store.claims()?;
        let held = ticket::claim_on(&claims, t.id(), item.as_deref())
            .or_else(|| ticket::claim_on(&claims, t.id(), None))
            .and_then(|(_, c)| c.map(|c| c.actor.clone()));
        match held {
            Some(actor) if actor != store.actor => {
                p.applied = Some(format!("held by {actor}; not ticked"));
                report.push(Diagnostic::warn(
                    "bridge.not-ticked",
                    name.clone(),
                    format!("{name} is claimed by {actor}; the bridge ticks nothing it holds"),
                ));
                return Ok(p);
            }
            Some(_) => {}
            None => {
                let claimed = ticket::claim_cmd(store, name, false)?;
                if claimed.is_refused() {
                    p.applied = Some(format!("not claimed: {}", first_message(&claimed)));
                    note_refusal(report, name, &claimed);
                    return Ok(p);
                }
            }
        }
    }
    let outcome = ticket::done_with(store, name, None, None, can_check)?;
    if outcome.is_refused() || !outcome.report.is_ready() {
        p.applied = Some(format!("not ticked: {}", first_message(&outcome)));
        note_refusal(report, name, &outcome);
    } else {
        p.applied = Some(outcome.human.lines().take(2).collect::<Vec<_>>().join("; "));
    }
    Ok(p)
}

fn first_message(o: &Outcome) -> String {
    o.report
        .diagnostics
        .iter()
        .find(|d| {
            matches!(
                d.severity,
                crate::diagnostic::Severity::Error | crate::diagnostic::Severity::Unknown
            )
        })
        .map_or_else(
            || o.human.clone(),
            |d| format!("{} ({})", d.message, d.rule),
        )
}

/// A tick the bridge could not make is said, as a warning: the bridge never
/// fails because a check did.
fn note_refusal(report: &mut crate::diagnostic::Report, name: &str, o: &Outcome) {
    report.push(Diagnostic::warn(
        "bridge.not-ticked",
        name.to_owned(),
        format!("{name}: {}", first_message(o)),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_task_names_items_by_their_ids_only() {
        assert_eq!(
            named_refs("Fix the parser (t-1a2b/i-9c01), see i-77be."),
            vec!["t-1a2b/i-9c01".to_owned(), "i-77be".to_owned()]
        );
        assert!(named_refs("Implement user authentication").is_empty());
        assert!(named_refs("multi-step work in t-x").is_empty());
        assert_eq!(named_refs("t-3f2a"), vec!["t-3f2a".to_owned()]);
    }

    #[test]
    fn the_documented_task_vocabulary_reads_and_nothing_else() {
        let hook = r#"{"session_id":"s","hook_event_name":"TaskCompleted","task_id":"task-001","task_subject":"Implement t-1a2b/i-9c01","task_description":"Add login"}"#;
        let t = tasks_in(hook, "hook").expect("reads");
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].id, "task-001");
        assert_eq!(t[0].description, "Add login");
        let list = r#"{"tasks":[{"id":"1","subject":"a","status":"completed"},{"id":2,"subject":"b","status":"pending"}]}"#;
        let t = tasks_in(list, "f").expect("reads");
        assert_eq!(t.len(), 2);
        assert_eq!(t[1].id, "2");
        assert!(tasks_in(r#"{"name":"no subject"}"#, "f").is_err());
        assert!(tasks_in("not json", "f").is_err());
    }
}
