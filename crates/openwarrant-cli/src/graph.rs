// SPDX-License-Identifier: Apache-2.0
//! The work graph (OW-WAR-0148 M15): one dependency graph over every kind of
//! work the model holds, and the set that can start now.
//!
//! `war plan frontier` answers for the stages of directory Warrants and `war
//! view ready` for the items of light ones. This is the one graph both are
//! slices of, built from `war plan model`'s records and relations, so a
//! scheduler, a person and an external tool all read one answer:
//!
//! - **Nodes.** Every item of a light Warrant (`t-x/i-y`); every light
//!   Warrant (`t-x`), which is the work itself when it has no items and is
//!   done when its last item is otherwise; every stage of a directory Warrant
//!   (`NS-WAR-0001/STAGE-001`); and every authored record a `depends_on` or
//!   an item's `implements` reaches (`REQ-pr1`).
//! - **Edges.** `a → b` reads "a waits on b". The model's `depends_on`
//!   relations (an item's `after`, a record atom's relation lines); a light
//!   Warrant waits on its items; a stage waits on every stage of each
//!   milestone its own milestone `depends_on`; an item that `implements` a
//!   record waits on what that record `depends_on`.
//! - **Done.** An item or light Warrant when its checklist says so; a stage
//!   when a submission is recorded (`war plan frontier`); a record when at
//!   least one item implements it and every one that does is done. A record
//!   nothing implements is never done, so what waits on it waits, reported
//!   (`graph.record-unimplemented`): fail closed, as a milestone without
//!   obligations is.
//! - **Ready.** Not done, runnable (an item, a light Warrant with no items
//!   and no child Warrants, an agent stage the frontier lists open), nobody
//!   holds it, and every node it waits on is done. A claim whose lease ran
//!   out holds nothing (M11).
//! - **Cycles.** A strongly connected set of two or more nodes, or a node
//!   that waits on itself, is a cycle, and every node in it is blocked by
//!   name (`graph.cycle`). A scheduler refuses to start while one exists:
//!   nothing in a cycle can ever be ready, and what waits on it would wait
//!   for ever without saying why.
//!
//! External tools never own this graph. An adapter is handed one node at a
//! time and answers for that node; what is ready next is decided here.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::corpus::Corpus;
use crate::diagnostic::{Diagnostic, Report};
use crate::repo::RepoError;
use crate::ticket::{Store, claim};

pub const SCHEMA: &str = "oh.war/work-graph/v1";

/// What a node is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// An item of a light Warrant.
    Item,
    /// A light Warrant as a whole.
    Warrant,
    /// A stage of a directory Warrant.
    Stage,
    /// An authored record other work implements or waits on.
    Record,
}

impl Kind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Item => "item",
            Self::Warrant => "warrant",
            Self::Stage => "stage",
            Self::Record => "record",
        }
    }
}

/// Where a node stands, derived and never written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Done,
    /// Runnable, unheld, and everything it waits on is done.
    Ready,
    /// Someone holds it: a live claim, or a dispatched stage.
    Running,
    /// Something it waits on is not done.
    Waiting,
    /// Nothing the graph can do moves it: a cycle, a blocking question, a
    /// person's decision.
    Blocked,
}

impl State {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Waiting => "waiting",
            Self::Blocked => "blocked",
        }
    }
}

/// One node of the work graph.
#[derive(Debug, Clone, Serialize)]
pub struct Node {
    /// Its global record id: `t-x/i-y`, `t-x`, `NS-WAR-0001/STAGE-001`, `REQ-pr1`.
    pub id: String,
    pub kind: Kind,
    /// The Warrant that holds it (a light Warrant's id or a directory
    /// Warrant's alias); absent for a record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warrant: Option<String>,
    /// The Warrant's title (a record's id).
    pub title: String,
    /// The item's text, a stage's title; empty for a Warrant or record.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
    /// The light Warrant's type (`bug`, `feature`, ...); `stage` for a stage.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub work_type: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    pub priority: u8,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub created_at: String,
    /// The Warrant's `due` date (`YYYY-MM-DD`), when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    /// Whether an executor can run it: an item, a light Warrant that is its
    /// own work, an agent stage.
    pub runnable: bool,
    pub state: State,
    /// What it waits on that is not done, or why it is blocked.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub waits_on: Vec<String>,
    /// Who holds it, when someone does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder: Option<String>,
}

/// `from` waits on `to`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    /// Where the edge comes from: `depends_on`, `item` (a Warrant waits on
    /// its items), `milestone` (a stage on an earlier milestone's stages),
    /// `implements` (an item on what its record depends on).
    pub via: &'static str,
}

/// The work graph.
#[derive(Debug, Clone, Serialize)]
pub struct Graph {
    pub schema: &'static str,
    /// Sorted by id.
    pub nodes: Vec<Node>,
    /// Sorted.
    pub edges: Vec<Edge>,
    /// Each cycle as the ids in it, smallest first; empty when the graph is
    /// acyclic.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cycles: Vec<Vec<String>>,
}

impl Graph {
    #[must_use]
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes
            .binary_search_by(|n| n.id.as_str().cmp(id))
            .ok()
            .map(|i| &self.nodes[i])
    }

    fn node_mut(&mut self, id: &str) -> Option<&mut Node> {
        self.nodes
            .binary_search_by(|n| n.id.as_str().cmp(id))
            .ok()
            .map(|i| &mut self.nodes[i])
    }

    /// The nodes `id` waits on.
    pub fn deps<'a>(&'a self, id: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.edges
            .iter()
            .filter(move |e| e.from == id)
            .map(|e| e.to.as_str())
    }

    /// The nodes that wait on `id`.
    pub fn dependents<'a>(&'a self, id: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.edges
            .iter()
            .filter(move |e| e.to == id)
            .map(|e| e.from.as_str())
    }

    /// The ready nodes, in id order; a scheduler orders them.
    #[must_use]
    pub fn ready(&self) -> Vec<&Node> {
        self.nodes
            .iter()
            .filter(|n| n.state == State::Ready)
            .collect()
    }

    /// How many nodes are in each state.
    #[must_use]
    pub fn counts(&self) -> BTreeMap<&'static str, usize> {
        let mut out = BTreeMap::new();
        for n in &self.nodes {
            *out.entry(n.state.as_str()).or_insert(0) += 1;
        }
        out
    }

    /// Mark `id` blocked for the reason given (a scheduler's own finding:
    /// attempts used up, a person's decision asked for). Its dependents are
    /// recomputed by [`Graph::settle`].
    pub fn block(&mut self, id: &str, why: String) {
        if let Some(n) = self.node_mut(id)
            && n.state != State::Done
        {
            n.state = State::Blocked;
            n.waits_on = vec![why];
        }
    }

    /// Mark `id` done (a scheduler landed it) and settle what waits on it.
    pub fn mark_done(&mut self, id: &str) {
        if let Some(n) = self.node_mut(id) {
            n.state = State::Done;
            n.waits_on.clear();
            n.holder = None;
        }
        self.settle();
    }

    /// Mark `id` held by `who`.
    pub fn mark_running(&mut self, id: &str, who: &str) {
        if let Some(n) = self.node_mut(id)
            && matches!(n.state, State::Ready | State::Waiting)
        {
            n.state = State::Running;
            n.holder = Some(who.to_owned());
        }
    }

    /// Give `id` back: held by nobody, and ready again if everything it
    /// waits on is done.
    pub fn release(&mut self, id: &str) {
        if let Some(n) = self.node_mut(id)
            && n.state == State::Running
        {
            n.state = State::Waiting;
            n.holder = None;
        }
        self.settle();
    }

    /// Recompute Ready and Waiting from the edges, and the done state of
    /// every derived node (a Warrant with items, a record), until nothing
    /// moves. Done, Running and Blocked nodes keep their state, except that a
    /// derived node becomes Done once what it is made of is.
    pub fn settle(&mut self) {
        loop {
            let done: BTreeSet<String> = self
                .nodes
                .iter()
                .filter(|n| n.state == State::Done)
                .map(|n| n.id.clone())
                .collect();
            let ids: BTreeSet<String> = self.nodes.iter().map(|n| n.id.clone()).collect();
            let mut deps: BTreeMap<&str, Vec<&Edge>> = BTreeMap::new();
            for e in &self.edges {
                deps.entry(e.from.as_str()).or_default().push(e);
            }
            let mut changes: Vec<(String, State, Vec<String>)> = Vec::new();
            for n in &self.nodes {
                if matches!(n.state, State::Done | State::Running | State::Blocked) {
                    continue;
                }
                let mine = deps.get(n.id.as_str()).cloned().unwrap_or_default();
                let open: Vec<String> = mine
                    .iter()
                    .filter(|e| !done.contains(&e.to))
                    .map(|e| {
                        if ids.contains(&e.to) {
                            e.to.clone()
                        } else {
                            format!("{} (unknown)", e.to)
                        }
                    })
                    .collect();
                let derived = matches!(n.kind, Kind::Record)
                    || (n.kind == Kind::Warrant && !n.runnable && !mine.is_empty());
                let next = if derived {
                    // Made of what it waits on: done when all of it is, and
                    // never done when it is made of nothing (fail closed).
                    let made_of: Vec<&&Edge> = mine
                        .iter()
                        .filter(|e| matches!(e.via, "item" | "implemented_by"))
                        .collect();
                    if !made_of.is_empty() && open.is_empty() {
                        State::Done
                    } else {
                        State::Waiting
                    }
                } else if !n.runnable {
                    State::Waiting
                } else if open.is_empty() {
                    State::Ready
                } else {
                    State::Waiting
                };
                if next != n.state || (next == State::Waiting && open != n.waits_on) {
                    changes.push((n.id.clone(), next, open));
                }
            }
            if changes.is_empty() {
                return;
            }
            for (id, state, open) in changes {
                if let Some(n) = self.node_mut(&id) {
                    n.state = state;
                    n.waits_on = if state == State::Waiting {
                        open
                    } else {
                        Vec::new()
                    };
                }
            }
        }
    }
}

/// What to build the graph from, besides the corpus: the claims now held,
/// read through the ticket store (M11's shared lock set).
pub fn build(corpus: &Corpus, store: &Store) -> Result<(Report, Graph), RepoError> {
    let model = crate::model::build(corpus)?;
    build_from(corpus, store, &model)
}

/// [`build`] over a model already built.
pub fn build_from(
    corpus: &Corpus,
    store: &Store,
    model: &crate::model::Model,
) -> Result<(Report, Graph), RepoError> {
    let mut report = Report::default();
    let (tickets, faults) = corpus.tickets()?;
    for f in faults {
        report.push(Diagnostic::warn(
            f.rule.clone(),
            f.file.clone().unwrap_or_default(),
            format!("not in the work graph: {}", f.message),
        ));
    }
    let claims = store.claims()?;
    let now = crate::ticket::now_secs();
    let record_types: BTreeMap<&str, &str> = model
        .records
        .iter()
        .map(|r| (r.id.as_str(), r.kind.as_str()))
        .collect();

    let mut nodes: BTreeMap<String, Node> = BTreeMap::new();
    let mut edges: BTreeSet<Edge> = BTreeSet::new();

    // ---- light Warrants and their items, as the ticket store reads them.
    let held = |ticket: &str, item: Option<&str>| -> Option<String> {
        let (_, c) = crate::ticket::claim_on(&claims, ticket, item)?;
        match c {
            // A lock nobody can read still holds (fail closed).
            None => Some("somebody (the lock does not parse)".to_owned()),
            Some(c) if c.lease_expired(now) => None,
            Some(c) => Some(c.actor.clone()),
        }
    };
    for t in tickets {
        let tid = t.id().to_owned();
        let base = |kind: Kind, id: String, text: String, runnable: bool| Node {
            id,
            kind,
            warrant: Some(tid.clone()),
            title: t.manifest.title.clone(),
            text,
            work_type: t.manifest.kind.clone(),
            labels: t.manifest.labels.clone(),
            priority: t.manifest.priority,
            created_at: t.manifest.created_at.clone(),
            due: t.manifest.due.clone(),
            runnable,
            state: State::Waiting,
            waits_on: Vec::new(),
            holder: None,
        };
        let whole_holder = held(&tid, None);
        if t.checklist.items.is_empty() {
            // The Warrant is its own work, unless it is an epic worked
            // through its child Warrants (as `war view ready` reads it).
            let children = crate::ticket::children_of(tickets, &tid);
            let mut n = base(
                Kind::Warrant,
                tid.clone(),
                String::new(),
                children.is_empty(),
            );
            if t.checklist.is_done() {
                n.state = State::Done;
            } else if let Some(h) = whole_holder {
                n.state = State::Running;
                n.holder = Some(h);
            }
            nodes.insert(tid.clone(), n);
            continue;
        }
        let mut whole = base(Kind::Warrant, tid.clone(), String::new(), false);
        if t.checklist.is_done() {
            whole.state = State::Done;
        }
        nodes.insert(tid.clone(), whole);
        for item in &t.checklist.items {
            // A line with no id yet is named by the next write; until then it
            // cannot be addressed, so it is not a node.
            let Some(iid) = &item.id else { continue };
            let id = format!("{tid}/{iid}");
            edges.insert(Edge {
                from: tid.clone(),
                to: id.clone(),
                via: "item",
            });
            let mut n = base(Kind::Item, id.clone(), item.text.clone(), true);
            if item.done {
                n.state = State::Done;
            } else if let Some(h) = held(&tid, Some(iid)).or_else(|| whole_holder.clone()) {
                n.state = State::Running;
                n.holder = Some(h);
            }
            nodes.insert(id, n);
        }
    }

    // ---- stages, as the frontier reads them.
    let (frontier_report, frontier) = corpus.frontier()?;
    for d in &frontier_report.diagnostics {
        if d.severity != crate::diagnostic::Severity::Pass {
            report.push(d.clone());
        }
    }
    let mut stage_milestones: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in &frontier.rows {
        let id = format!("{}/{}", row.warrant, row.stage);
        let runnable = row.executor_kind == "agent";
        let (state, waits_on, holder) = match row.state {
            crate::frontier::StageState::Done => (State::Done, Vec::new(), None),
            crate::frontier::StageState::Claimed => (
                State::Running,
                Vec::new(),
                Some("a dispatch with no submission yet".to_owned()),
            ),
            crate::frontier::StageState::Blocked => {
                // A milestone wait is an edge (below); a blocking question is
                // a person's, and holds the stage whatever the edges say.
                let questions: Vec<String> = row
                    .waiting_on
                    .iter()
                    .filter(|w| w.starts_with("Q-"))
                    .map(|q| format!("question {q} (a person answers it)"))
                    .collect();
                if questions.is_empty() {
                    (State::Waiting, row.waiting_on.clone(), None)
                } else {
                    (State::Blocked, questions, None)
                }
            }
            crate::frontier::StageState::Open if runnable => (State::Ready, Vec::new(), None),
            crate::frontier::StageState::Open => (State::Waiting, Vec::new(), None),
        };
        for m in row.milestone.split('+') {
            stage_milestones
                .entry(format!("{}/{m}", row.warrant))
                .or_default()
                .push(id.clone());
        }
        nodes.insert(
            id.clone(),
            Node {
                id,
                kind: Kind::Stage,
                warrant: Some(row.warrant.clone()),
                title: row.title.clone(),
                text: row.title.clone(),
                work_type: Some("stage".to_owned()),
                labels: Vec::new(),
                priority: openwarrant_core::ticket::DEFAULT_PRIORITY,
                created_at: String::new(),
                due: None,
                runnable,
                state,
                waits_on,
                holder,
            },
        );
    }
    // A stage waits on every stage of each milestone its milestone waits on.
    // The frontier decides when that milestone is complete (its obligations
    // established); the edge is what ordering and cycles read.
    for e in corpus.entries()? {
        let Some(one) = e.ok() else { continue };
        let Some(alias) = one.validated.as_ref().map(|v| v.alias.to_string()) else {
            continue;
        };
        let Some(text) = one.basis.as_ref().and_then(|b| {
            b.atoms
                .iter()
                .find(|a| a.role == "milestones")
                .and_then(|a| String::from_utf8(a.bytes.clone()).ok())
        }) else {
            continue;
        };
        let Ok(g) = openwarrant_core::milestones::parse(&text) else {
            continue;
        };
        for m in &g.milestones {
            for s in &m.stage_refs {
                let from = format!("{alias}/{s}");
                if !nodes.contains_key(&from) {
                    continue;
                }
                for d in &m.depends_on {
                    for to in stage_milestones
                        .get(&format!("{alias}/{d}"))
                        .into_iter()
                        .flatten()
                    {
                        if *to != from {
                            edges.insert(Edge {
                                from: from.clone(),
                                to: to.clone(),
                                via: "milestone",
                            });
                        }
                    }
                }
            }
        }
    }

    // ---- the model's depends_on and implements, and the records they reach.
    let mut implemented: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut record_deps: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for r in &model.relations {
        match r.kind.as_str() {
            "depends_on" => {
                edges.insert(Edge {
                    from: r.from.clone(),
                    to: r.to.clone(),
                    via: "depends_on",
                });
                record_deps
                    .entry(r.from.clone())
                    .or_default()
                    .push(r.to.clone());
            }
            "implements" => {
                implemented
                    .entry(r.to.clone())
                    .or_default()
                    .push(r.from.clone());
            }
            _ => {}
        }
    }
    let work: BTreeSet<String> = nodes.keys().cloned().collect();
    let is_work = |id: &str| work.contains(id);
    let mut records: BTreeSet<String> = BTreeSet::new();
    for e in &edges {
        for id in [&e.from, &e.to] {
            if !is_work(id) && record_types.contains_key(id.as_str()) {
                records.insert(id.clone());
            }
        }
    }
    for (record, by) in &implemented {
        if by.iter().any(|b| is_work(b)) && record_types.contains_key(record.as_str()) {
            records.insert(record.clone());
        }
    }
    for id in &records {
        // A record is made of the work that implements it.
        let by: Vec<&String> = implemented
            .get(id)
            .map(|v| v.iter().filter(|b| is_work(b)).collect())
            .unwrap_or_default();
        for b in &by {
            edges.insert(Edge {
                from: id.clone(),
                to: (*b).clone(),
                via: "implemented_by",
            });
            // ...and the work waits on what the record waits on.
            for d in record_deps.get(id).into_iter().flatten() {
                edges.insert(Edge {
                    from: (*b).clone(),
                    to: d.clone(),
                    via: "implements",
                });
            }
        }
        if by.is_empty() {
            report.push(Diagnostic::warn(
                "graph.record-unimplemented",
                id.clone(),
                format!(
                    "{id} is reached by a depends_on, and no item implements it, so it never \
                     reads done and what waits on it waits. Add an item that implements it \
                     (`war create \"...\" --implements {id}`)"
                ),
            ));
        }
        nodes.insert(
            id.clone(),
            Node {
                id: id.clone(),
                kind: Kind::Record,
                warrant: None,
                title: id.clone(),
                text: String::new(),
                work_type: record_types.get(id.as_str()).map(|t| (*t).to_owned()),
                labels: Vec::new(),
                priority: openwarrant_core::ticket::DEFAULT_PRIORITY,
                created_at: String::new(),
                due: None,
                runnable: false,
                state: State::Waiting,
                waits_on: Vec::new(),
                holder: None,
            },
        );
    }
    // Only edges between work this graph knows, or from work to something
    // unknown (which blocks, named, as `war claim` reads it).
    let edges: Vec<Edge> = edges
        .into_iter()
        .filter(|e| nodes.contains_key(&e.from))
        .collect();

    let mut graph = Graph {
        schema: SCHEMA,
        nodes: nodes.into_values().collect(),
        edges,
        cycles: Vec::new(),
    };
    graph.cycles = cycles(&graph);
    for c in graph.cycles.clone() {
        let ring = format!("{} → {}", c.join(" → "), c[0]);
        report.push(Diagnostic::error(
            "graph.cycle",
            c[0].clone(),
            format!(
                "{} wait on each other: {ring}. Nothing in a cycle can ever start; remove one \
                 of the `after` edges (edit the checklist line, or the record's relation line)",
                c.join(", ")
            ),
        ));
        for id in &c {
            graph.block(id, format!("cycle: {ring}"));
        }
    }
    graph.settle();
    Ok((report, graph))
}

/// Every cycle: Tarjan's strongly connected components of two or more nodes,
/// and every node with an edge to itself, each sorted, smallest first.
#[must_use]
pub fn cycles(graph: &Graph) -> Vec<Vec<String>> {
    let index_of: BTreeMap<&str, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); graph.nodes.len()];
    let mut self_loops = BTreeSet::new();
    for e in &graph.edges {
        let (Some(&a), Some(&b)) = (index_of.get(e.from.as_str()), index_of.get(e.to.as_str()))
        else {
            continue;
        };
        if a == b {
            self_loops.insert(a);
        }
        adj[a].push(b);
    }
    // Iterative Tarjan, so a long chain cannot overflow the stack.
    let n = graph.nodes.len();
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut next = 0usize;
    let mut out: Vec<Vec<String>> = Vec::new();
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        let mut work: Vec<(usize, usize)> = vec![(root, 0)];
        while let Some(&mut (v, ref mut i)) = work.last_mut() {
            if *i == 0 && index[v] == usize::MAX {
                index[v] = next;
                low[v] = next;
                next += 1;
                stack.push(v);
                on_stack[v] = true;
            }
            if *i < adj[v].len() {
                let w = adj[v][*i];
                *i += 1;
                if index[w] == usize::MAX {
                    work.push((w, 0));
                } else if on_stack[w] {
                    low[v] = low[v].min(index[w]);
                }
                continue;
            }
            work.pop();
            if let Some(&(parent, _)) = work.last() {
                low[parent] = low[parent].min(low[v]);
            }
            if low[v] == index[v] {
                let mut scc = Vec::new();
                while let Some(w) = stack.pop() {
                    on_stack[w] = false;
                    scc.push(w);
                    if w == v {
                        break;
                    }
                }
                if scc.len() > 1 || self_loops.contains(&v) {
                    let mut ids: Vec<String> =
                        scc.iter().map(|&k| graph.nodes[k].id.clone()).collect();
                    ids.sort();
                    out.push(ids);
                }
            }
        }
    }
    out.sort();
    out
}

/// `war plan frontier --all`: one line per node, with what it waits on.
#[must_use]
pub fn render(g: &Graph) -> String {
    let counts = g.counts();
    let n = |s: &str| counts.get(s).copied().unwrap_or(0);
    let mut out = format!(
        "work graph: {} ready · {} running · {} waiting · {} blocked · {} done\n",
        n("ready"),
        n("running"),
        n("waiting"),
        n("blocked"),
        n("done")
    );
    for c in &g.cycles {
        out.push_str(&format!(
            "cycle: {} → {}\n",
            c.join(" → "),
            c.first().map_or("", String::as_str)
        ));
    }
    out.push('\n');
    for node in &g.nodes {
        let what = if node.text.is_empty() || node.text == node.title {
            node.title.clone()
        } else {
            format!("{}: {}", node.title, node.text)
        };
        out.push_str(&format!(
            "  {:<8} {:<28} {:<8} {}{}{}\n",
            node.state.as_str(),
            node.id,
            node.kind.as_str(),
            what,
            node.holder
                .as_ref()
                .map(|h| format!("  [held by {h}]"))
                .unwrap_or_default(),
            if node.waits_on.is_empty() {
                String::new()
            } else {
                format!("  (waits on {})", node.waits_on.join(", "))
            }
        ));
    }
    out
}

/// A claim's holder as one line, for a node that is running.
#[must_use]
pub fn holder_line(c: &claim::Claim) -> String {
    format!("{} since {}", c.actor, c.since)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, kind: Kind, runnable: bool, state: State) -> Node {
        Node {
            id: id.to_owned(),
            kind,
            warrant: None,
            title: id.to_owned(),
            text: String::new(),
            work_type: None,
            labels: Vec::new(),
            priority: 2,
            created_at: String::new(),
            due: None,
            runnable,
            state,
            waits_on: Vec::new(),
            holder: None,
        }
    }

    fn edge(from: &str, to: &str) -> Edge {
        Edge {
            from: from.to_owned(),
            to: to.to_owned(),
            via: "depends_on",
        }
    }

    fn graph(nodes: Vec<Node>, edges: Vec<Edge>) -> Graph {
        let mut nodes = nodes;
        nodes.sort_by(|a, b| a.id.cmp(&b.id));
        let mut g = Graph {
            schema: SCHEMA,
            nodes,
            edges,
            cycles: Vec::new(),
        };
        g.cycles = cycles(&g);
        g.settle();
        g
    }

    /// A diamond: d waits on b and c, which wait on a. Only a is ready, and
    /// each layer becomes ready when the one below it is done.
    #[test]
    fn a_diamond_opens_one_layer_at_a_time() {
        let mut g = graph(
            ["a", "b", "c", "d"]
                .iter()
                .map(|i| node(i, Kind::Item, true, State::Waiting))
                .collect(),
            vec![
                edge("b", "a"),
                edge("c", "a"),
                edge("d", "b"),
                edge("d", "c"),
            ],
        );
        assert!(g.cycles.is_empty());
        let ready = |g: &Graph| g.ready().iter().map(|n| n.id.clone()).collect::<Vec<_>>();
        assert_eq!(ready(&g), ["a"]);
        g.mark_running("a", "x");
        assert!(ready(&g).is_empty(), "a held is not ready");
        g.mark_done("a");
        assert_eq!(ready(&g), ["b", "c"]);
        g.mark_done("b");
        assert_eq!(ready(&g), ["c"], "d still waits on c");
        assert_eq!(g.node("d").unwrap().waits_on, ["c"]);
        g.mark_done("c");
        assert_eq!(ready(&g), ["d"]);
    }

    /// A two-node cycle and a self-loop are both cycles; a chain is not.
    #[test]
    fn cycles_are_found_and_chains_are_not() {
        let nodes = || {
            ["a", "b", "c", "s"]
                .iter()
                .map(|i| node(i, Kind::Item, true, State::Waiting))
                .collect::<Vec<_>>()
        };
        let g = graph(
            nodes(),
            vec![edge("a", "b"), edge("b", "a"), edge("s", "s")],
        );
        assert_eq!(
            g.cycles,
            vec![vec!["a".to_owned(), "b".to_owned()], vec!["s".to_owned()]]
        );
        let g = graph(nodes(), vec![edge("a", "b"), edge("b", "c")]);
        assert!(g.cycles.is_empty());
    }

    /// A record is done when the work implementing it is, and never when
    /// nothing implements it.
    #[test]
    fn a_record_is_made_of_its_implementers() {
        let mut g = graph(
            vec![
                node("R", Kind::Record, false, State::Waiting),
                node("E", Kind::Record, false, State::Waiting),
                node("i", Kind::Item, true, State::Waiting),
            ],
            vec![Edge {
                from: "R".into(),
                to: "i".into(),
                via: "implemented_by",
            }],
        );
        assert_eq!(g.node("R").unwrap().state, State::Waiting);
        assert_eq!(g.node("E").unwrap().state, State::Waiting);
        g.mark_done("i");
        assert_eq!(g.node("R").unwrap().state, State::Done);
        assert_eq!(
            g.node("E").unwrap().state,
            State::Waiting,
            "made of nothing"
        );
    }

    /// An edge to a node nobody declared blocks, named as unknown.
    #[test]
    fn an_unknown_dependency_waits_by_name() {
        let g = graph(
            vec![node("a", Kind::Item, true, State::Waiting)],
            vec![edge("a", "t-gone")],
        );
        assert_eq!(g.node("a").unwrap().state, State::Waiting);
        assert_eq!(g.node("a").unwrap().waits_on, ["t-gone (unknown)"]);
    }
}
