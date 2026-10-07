// SPDX-License-Identifier: Apache-2.0
//! Estimates learned from the journal, and the critical path (OW-WAR-0148
//! M15; decision 9).
//!
//! # What is learned
//!
//! Every light Warrant's journal records when each item was claimed
//! (`ticket.claimed`, `ticket.claim_reclaimed`, `ticket.claim_stolen`) and
//! when it was ticked (`ticket.item_done`). One sample is the time from the
//! last claim on an item (or on its whole Warrant) to its tick, labelled with
//! the Warrant's type and labels. A tick with no claim before it (an older
//! journal, a hand edit) is no sample, and is counted as skipped rather than
//! guessed.
//!
//! # How a node is estimated
//!
//! The median of the most specific group of samples that has any, in this
//! order: the same type and at least one shared label; at least one shared
//! label; the same type; every sample. With no history at all, the prior
//! (`[go] prior_secs`, thirty minutes by default). Each estimate says which group it
//! came from and how many samples it rests on, so a number from one sample
//! never reads like one from fifty.
//!
//! # The critical path
//!
//! A node's remaining length is its own estimate plus the longest remaining
//! length among the nodes that wait on it. The critical path starts at the
//! open node with the longest remaining length that waits on nothing open,
//! and follows the longest dependent each step. A scheduler runs it first.
//!
//! # Deadlines
//!
//! A light Warrant may carry `due = "YYYY-MM-DD"`. A node's effective due
//! date is the earliest of its own and of every node that waits on it,
//! directly or not: work a deadline depends on inherits the deadline. Among
//! the nodes ready now, one with an effective due date goes before one
//! without, the earlier date first; a date never makes a node ready, so it
//! never reorders what waits on what.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::diagnostic::{Diagnostic, Report};
use crate::graph::{Graph, Kind, Node, State};
use crate::repo::RepoError;
use crate::ticket::{Store, Ticket};

pub const SCHEMA: &str = "oh.war/estimate/v1";

/// With no history at all: thirty minutes a node.
pub const DEFAULT_PRIOR_SECS: u64 = 30 * 60;

/// One learned duration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Sample {
    /// `t-x/i-y`, or `t-x` for a Warrant with no items.
    pub target: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub work_type: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    pub seconds: u64,
}

/// Every sample in the corpus, and how many ticks had no claim before them.
#[derive(Debug, Clone, Default, Serialize)]
pub struct History {
    pub samples: Vec<Sample>,
    /// Ticks with no claim on record before them: not samples.
    pub unclaimed_ticks: usize,
    /// The prior, in seconds.
    pub prior_secs: u64,
}

/// Where an estimate came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// Same type, a shared label.
    TypeLabel,
    /// A shared label.
    Label,
    /// Same type.
    Type,
    /// Every sample.
    All,
    /// No history: the prior.
    Prior,
    /// A Warrant with items, or a record: made of other nodes, no work of
    /// its own.
    Derived,
}

impl Source {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TypeLabel => "type+label",
            Self::Label => "label",
            Self::Type => "type",
            Self::All => "all",
            Self::Prior => "prior",
            Self::Derived => "derived",
        }
    }
}

/// One node's estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Estimate {
    pub seconds: u64,
    pub source: Source,
    pub samples: usize,
}

fn parse_time(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s.trim())
        .ok()
        .map(|d| d.timestamp())
}

/// Learn from every light Warrant's journal. An unreadable journal is
/// reported and contributes nothing; it is never read as empty history.
pub fn learn(tickets: &[Ticket], prior_secs: u64, report: &mut Report) -> History {
    let mut history = History {
        prior_secs,
        ..History::default()
    };
    for t in tickets {
        let journal = match crate::journal_cmd::load(&t.dir) {
            Ok(j) => j,
            Err(e) => {
                report.push(Diagnostic::unknown(
                    "estimate.journal-unreadable",
                    t.dir.join(crate::journal_cmd::FILE).to_string(),
                    format!(
                        "{}: its journal cannot be read, so its history is not in the \
                         estimates: {e}",
                        t.id()
                    ),
                ));
                continue;
            }
        };
        let mut claimed: BTreeMap<String, i64> = BTreeMap::new();
        for e in &journal.events {
            let payload: serde_json::Value =
                serde_json::from_str(&e.payload).unwrap_or(serde_json::Value::Null);
            let target = payload
                .get("target")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let Some(at) = parse_time(&e.occurred_at) else {
                continue;
            };
            match e.event_type.as_str() {
                crate::ticket::event::CLAIMED
                | crate::ticket::event::CLAIM_RECLAIMED
                | crate::ticket::event::CLAIM_STOLEN => {
                    if !target.is_empty() {
                        claimed.insert(target, at);
                    }
                }
                crate::ticket::event::ITEM_DONE => {
                    let since = claimed
                        .get(&target)
                        .or_else(|| claimed.get(t.id()))
                        .copied();
                    match since {
                        Some(since) if at >= since => {
                            history.samples.push(Sample {
                                target,
                                work_type: t.manifest.kind.clone(),
                                labels: t.manifest.labels.clone(),
                                seconds: u64::try_from(at - since).unwrap_or(0),
                            });
                        }
                        _ => history.unclaimed_ticks += 1,
                    }
                }
                _ => {}
            }
        }
    }
    history
}

fn median(mut v: Vec<u64>) -> u64 {
    v.sort_unstable();
    let n = v.len();
    if n == 0 {
        return 0;
    }
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2
    }
}

impl History {
    /// The estimate for work of this type and these labels.
    #[must_use]
    pub fn estimate(&self, work_type: Option<&str>, labels: &[String]) -> Estimate {
        let shares = |s: &Sample| s.labels.iter().any(|l| labels.contains(l));
        let same = |s: &Sample| work_type.is_some() && s.work_type.as_deref() == work_type;
        let groups: [(Source, Box<dyn Fn(&Sample) -> bool + '_>); 4] = [
            (
                Source::TypeLabel,
                Box::new(|s: &Sample| same(s) && shares(s)),
            ),
            (Source::Label, Box::new(shares)),
            (Source::Type, Box::new(same)),
            (Source::All, Box::new(|_: &Sample| true)),
        ];
        for (source, admits) in groups {
            let v: Vec<u64> = self
                .samples
                .iter()
                .filter(|s| admits(s))
                .map(|s| s.seconds)
                .collect();
            if !v.is_empty() {
                return Estimate {
                    samples: v.len(),
                    seconds: median(v),
                    source,
                };
            }
        }
        Estimate {
            seconds: self.prior_secs,
            source: Source::Prior,
            samples: 0,
        }
    }

    /// The estimate for one node of the graph.
    #[must_use]
    pub fn of(&self, n: &Node) -> Estimate {
        let derived = matches!(n.kind, Kind::Record) || (n.kind == Kind::Warrant && !n.runnable);
        if derived || n.state == State::Done {
            return Estimate {
                seconds: 0,
                source: Source::Derived,
                samples: 0,
            };
        }
        self.estimate(n.work_type.as_deref(), &n.labels)
    }
}

/// One open node, as the schedule sees it.
#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub id: String,
    pub kind: Kind,
    pub state: State,
    pub estimate: Estimate,
    /// This node and the longest chain of work waiting on it, in seconds.
    pub remaining_secs: u64,
    /// The earliest due date of this node and everything waiting on it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    pub priority: u8,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
}

/// The schedule: each open node's estimate and remaining length, the ready
/// set in the order a scheduler takes it, and the critical path.
#[derive(Debug, Clone, Serialize)]
pub struct Schedule {
    pub schema: &'static str,
    pub rows: Vec<Row>,
    /// The ready nodes, first to start first.
    pub order: Vec<String>,
    pub critical_path: Vec<String>,
    pub critical_secs: u64,
    pub history: HistorySummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct HistorySummary {
    pub samples: usize,
    pub unclaimed_ticks: usize,
    pub prior_secs: u64,
}

/// Each node's estimate, remaining length and effective due date.
#[must_use]
pub fn schedule(graph: &Graph, history: &History) -> Schedule {
    let est: BTreeMap<&str, Estimate> = graph
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), history.of(n)))
        .collect();
    let in_cycle: BTreeSet<&str> = graph.cycles.iter().flatten().map(String::as_str).collect();
    let mut dependents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for e in &graph.edges {
        dependents
            .entry(e.to.as_str())
            .or_default()
            .push(e.from.as_str());
    }
    // Remaining length and effective due, memoized over the acyclic part;
    // a node in a cycle contributes its own estimate and stops the walk.
    let mut remaining: BTreeMap<&str, u64> = BTreeMap::new();
    let mut due: BTreeMap<&str, Option<String>> = BTreeMap::new();
    for n in &graph.nodes {
        walk(
            n.id.as_str(),
            graph,
            &est,
            &dependents,
            &in_cycle,
            &mut remaining,
            &mut due,
        );
    }
    let open: Vec<&Node> = graph
        .nodes
        .iter()
        .filter(|n| n.state != State::Done)
        .collect();
    let rows: Vec<Row> = open
        .iter()
        .map(|n| Row {
            id: n.id.clone(),
            kind: n.kind,
            state: n.state,
            estimate: est[n.id.as_str()],
            remaining_secs: remaining.get(n.id.as_str()).copied().unwrap_or(0),
            due: due.get(n.id.as_str()).cloned().flatten(),
            priority: n.priority,
            title: n.title.clone(),
            text: n.text.clone(),
        })
        .collect();
    let by_id: BTreeMap<&str, &Row> = rows.iter().map(|r| (r.id.as_str(), r)).collect();
    let mut ready: Vec<&Row> = rows.iter().filter(|r| r.state == State::Ready).collect();
    ready.sort_by(|a, b| order_key(a, graph).cmp(&order_key(b, graph)));
    let order: Vec<String> = ready.iter().map(|r| r.id.clone()).collect();

    // The critical path: from the open node with the longest remaining
    // length among those waiting on nothing open, along the longest
    // dependent each step.
    let waits_on_open = |id: &str| -> bool {
        graph
            .deps(id)
            .any(|d| by_id.get(d).is_some_and(|r| r.state != State::Done))
    };
    let start = rows
        .iter()
        .filter(|r| !waits_on_open(&r.id) && est[r.id.as_str()].source != Source::Derived)
        .max_by(|a, b| {
            a.remaining_secs
                .cmp(&b.remaining_secs)
                .then_with(|| b.id.cmp(&a.id))
        });
    let mut critical_path = Vec::new();
    let mut seen = BTreeSet::new();
    let mut at = start.map(|r| r.id.as_str());
    while let Some(id) = at {
        if !seen.insert(id) {
            break;
        }
        if est[id].source != Source::Derived {
            critical_path.push(id.to_owned());
        }
        at = dependents
            .get(id)
            .into_iter()
            .flatten()
            .filter(|d| by_id.contains_key(**d) && !in_cycle.contains(**d))
            .max_by(|a, b| {
                remaining
                    .get(**a)
                    .cmp(&remaining.get(**b))
                    .then_with(|| b.cmp(a))
            })
            .copied();
    }
    let critical_secs = critical_path
        .iter()
        .map(|id| est[id.as_str()].seconds)
        .sum();
    Schedule {
        schema: SCHEMA,
        rows,
        order,
        critical_path,
        critical_secs,
        history: HistorySummary {
            samples: history.samples.len(),
            unclaimed_ticks: history.unclaimed_ticks,
            prior_secs: history.prior_secs,
        },
    }
}

/// The order a scheduler takes ready nodes in: an effective due date first
/// (earlier first), then the longest remaining length (the critical path),
/// then the Warrant's priority, then the oldest, then the id.
fn order_key<'a>(r: &'a Row, graph: &'a Graph) -> impl Ord + 'a {
    let created = graph.node(&r.id).map_or("", |n| n.created_at.as_str());
    (
        r.due.is_none(),
        r.due.clone(),
        std::cmp::Reverse(r.remaining_secs),
        r.priority,
        created,
        r.id.as_str(),
    )
}

fn walk<'a>(
    id: &'a str,
    graph: &'a Graph,
    est: &BTreeMap<&'a str, Estimate>,
    dependents: &BTreeMap<&'a str, Vec<&'a str>>,
    in_cycle: &BTreeSet<&'a str>,
    remaining: &mut BTreeMap<&'a str, u64>,
    due: &mut BTreeMap<&'a str, Option<String>>,
) -> (u64, Option<String>) {
    if let Some(r) = remaining.get(id) {
        return (*r, due.get(id).cloned().flatten());
    }
    let own_due = graph.node(id).and_then(|n| n.due.clone());
    let own = est.get(id).map_or(0, |e| e.seconds);
    if in_cycle.contains(id) {
        remaining.insert(id, own);
        due.insert(id, own_due.clone());
        return (own, own_due);
    }
    // Mark before descending: the graph outside cycles is acyclic, so this
    // only guards against a cycle the caller did not name.
    remaining.insert(id, own);
    let mut longest = 0u64;
    let mut earliest = own_due;
    for d in dependents.get(id).into_iter().flatten() {
        let (r, du) = walk(d, graph, est, dependents, in_cycle, remaining, due);
        longest = longest.max(r);
        earliest = match (earliest, du) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
    }
    let total = own + longest;
    remaining.insert(id, total);
    due.insert(id, earliest.clone());
    (total, earliest)
}

/// `1h 05m`, `25m`, `40s`.
#[must_use]
pub fn human(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{h}h {m:02}m")
    } else if m > 0 {
        format!("{m}m")
    } else {
        format!("{s}s")
    }
}

/// `war plan estimate`: every open node's estimate, the order the ready
/// ones would start in, and the critical path.
pub fn run(
    corpus: &crate::corpus::Corpus,
    store: &Store,
    prior_secs: u64,
) -> Result<(Report, Schedule, String), RepoError> {
    let (mut report, graph) = crate::graph::build(corpus, store)?;
    let (tickets, _) = corpus.tickets()?;
    let history = learn(tickets, prior_secs, &mut report);
    let schedule = schedule(&graph, &history);
    let mut human_out = String::new();
    human_out.push_str(&format!(
        "estimates from {} sample(s) of claim to done{}; with none, {} a node\n",
        schedule.history.samples,
        if schedule.history.unclaimed_ticks > 0 {
            format!(
                " ({} tick(s) with no claim before them are not samples)",
                schedule.history.unclaimed_ticks
            )
        } else {
            String::new()
        },
        human(prior_secs)
    ));
    for r in schedule
        .rows
        .iter()
        .filter(|r| r.estimate.source != Source::Derived)
    {
        let order = schedule
            .order
            .iter()
            .position(|o| *o == r.id)
            .map_or_else(|| "  ".to_owned(), |p| format!("{:>2}", p + 1));
        human_out.push_str(&format!(
            "{order} {:<8} {:<26} {:>7} ({}, n={})  path {:>7}{}  {}\n",
            r.state.as_str(),
            r.id,
            human(r.estimate.seconds),
            r.estimate.source.as_str(),
            r.estimate.samples,
            human(r.remaining_secs),
            r.due
                .as_ref()
                .map(|d| format!("  due {d}"))
                .unwrap_or_default(),
            if r.text.is_empty() { &r.title } else { &r.text },
        ));
    }
    if schedule.critical_path.is_empty() {
        human_out.push_str("critical path: none (nothing open)\n");
    } else {
        human_out.push_str(&format!(
            "critical path ({}): {}\n",
            human(schedule.critical_secs),
            schedule.critical_path.join(" → ")
        ));
    }
    report.note(format!(
        "{} open node(s), {} ready; critical path {} over {} node(s)",
        schedule.rows.len(),
        schedule.order.len(),
        human(schedule.critical_secs),
        schedule.critical_path.len()
    ));
    Ok((report, schedule, human_out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(t: Option<&str>, labels: &[&str], secs: u64) -> Sample {
        Sample {
            target: "t-x/i-y".into(),
            work_type: t.map(str::to_owned),
            labels: labels.iter().map(|l| (*l).to_owned()).collect(),
            seconds: secs,
        }
    }

    /// The most specific group with any samples decides, by its median, and
    /// the estimate says which group and how many.
    #[test]
    fn the_most_specific_group_with_history_decides() {
        let h = History {
            samples: vec![
                sample(Some("bug"), &["ui"], 600),
                sample(Some("bug"), &["ui"], 1200),
                sample(Some("bug"), &[], 3000),
                sample(Some("feature"), &["api"], 7200),
            ],
            unclaimed_ticks: 0,
            prior_secs: DEFAULT_PRIOR_SECS,
        };
        let e = h.estimate(Some("bug"), &["ui".into()]);
        assert_eq!(
            (e.seconds, e.source, e.samples),
            (900, Source::TypeLabel, 2)
        );
        let e = h.estimate(Some("chore"), &["api".into()]);
        assert_eq!((e.seconds, e.source), (7200, Source::Label));
        let e = h.estimate(Some("bug"), &["docs".into()]);
        assert_eq!((e.seconds, e.source, e.samples), (1200, Source::Type, 3));
        let e = h.estimate(Some("chore"), &[]);
        assert_eq!((e.source, e.samples), (Source::All, 4));
        let empty = History {
            prior_secs: 1800,
            ..History::default()
        };
        assert_eq!(
            empty.estimate(Some("bug"), &[]),
            Estimate {
                seconds: 1800,
                source: Source::Prior,
                samples: 0
            }
        );
    }

    #[test]
    fn durations_read_as_people_say_them() {
        assert_eq!(human(40), "40s");
        assert_eq!(human(25 * 60), "25m");
        assert_eq!(human(3900), "1h 05m");
    }
}
