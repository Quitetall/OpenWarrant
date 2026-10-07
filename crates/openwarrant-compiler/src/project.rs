// SPDX-License-Identifier: Apache-2.0
//! Projection targets (OW-WAR-0148 M6; OW-ADR-0031): one set of records,
//! many documents.
//!
//! [`render`] is a pure function of a selection of the compiled model (an
//! [`Input`]: records with their bodies, and relations) and a declared
//! [`ProjectionDef`] (`openwarrant_core::projection`), to bytes and a trace.
//! It reads no repository and writes nothing, so `war render`, `war compile`,
//! `war impact` and `war host` all call the same function.
//!
//! # Selection
//!
//! From the subject's seeds (a document's roots, or every record of its
//! area, or one record), walk the projection's `select.through` relations,
//! admitting only records whose type `select.types` names, to `select.depth`
//! steps. Each section shows the admitted records of its types, in authored
//! order (source, line, id).
//!
//! # What a rendering depends on, exactly
//!
//! A rendering **selects** a record when the record's bytes reach the output:
//! it is shown, or it authored an incoming relation an annotation shows.
//! Every rendering ends with a provenance table naming each selected record
//! and its revision, so a change to any selected record changes the bytes,
//! and a change to any other record does not. A record named only by id at
//! the far end of an outgoing relation is **mentioned**, not selected: its
//! id is part of the relation's author's bytes. An incoming relation is
//! shown only from records the selection admits, so nothing outside the
//! selection reaches the output. `war impact` lists exactly the declared
//! projections that select a record.
//!
//! # Trace
//!
//! Every output line has one origin: the record it renders, the document
//! declaration (its title), or the template (the document type's profile:
//! headings, intros, table headers). The JSON form ([`Projection`]) carries
//! the lines as runs, each with its origin's id and revision.
//!
//! # Budget
//!
//! A projection may declare `max_bytes` (a caller may override it). A
//! rendering over budget is refused by name, [`RenderError::OverBudget`],
//! with the records that cost the most; it is never truncated.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write as _;

use openwarrant_core::projection::{
    Block, Column, Direction, DocumentProfile, ListStyle, Of, ProjectionDef, Renderer, Show, Step,
};
use serde::{Deserialize, Serialize};

/// The JSON form of a rendering.
pub const SCHEMA: &str = "oh.war/projection/v1";

/// One record of the selection's input: a model record with its body.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    /// Repository-relative path of the file that holds it.
    pub source: String,
    /// 1-based line of its heading, where it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    pub revision: String,
    /// Its text as authored: a record atom's span without its heading and
    /// relation lines; an obligation's statement; an item's line.
    #[serde(default)]
    pub body: String,
    /// Named fields: an obligation's `scope`, `evidence`, `verdict`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

/// One relation of the input, as the model carries it.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub from: String,
    pub kind: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_revision: Option<String>,
}

/// What a projection selects from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Input {
    pub records: Vec<Node>,
    pub relations: Vec<Edge>,
}

/// What is rendered: a declared document, an area, or one record.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    /// `password-reset/prd`, `password-reset` or `REQ-pr1`.
    pub id: String,
    /// `document`, `area` or `record`.
    pub kind: String,
    pub title: String,
    /// The records the walk starts from.
    pub seeds: Vec<String>,
    /// The declaration's revision, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// The declaration's path, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// A record the rendering selects.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selected {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub revision: String,
    pub source: String,
    /// Output bytes rendered from it.
    pub bytes: usize,
}

/// One run of output lines and where they came from.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trace {
    /// 1-based, inclusive.
    pub start_line: usize,
    pub end_line: usize,
    /// `record`, `document` or `template`.
    pub origin: String,
    /// The record id, the document id, or the document type's name.
    pub id: String,
    /// Its revision; absent for a subject with no declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
}

/// The template a rendering came from: the document type's profile file.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Template {
    /// The document type.
    pub profile: String,
    /// `sha256:` of its profile file.
    pub revision: String,
}

/// A rendering, as `war render --json` and `war host` return it
/// (`oh.war/projection/v1`).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    /// `oh.war/projection/v1`.
    pub schema: String,
    pub projection: String,
    /// `markdown` or `json`.
    pub renderer: String,
    pub subject: Subject,
    pub template: Template,
    /// `sha256:` of `content`.
    pub digest: String,
    /// `content`'s length in bytes.
    pub bytes: usize,
    /// The budget it was rendered under, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<usize>,
    /// Every record whose bytes reach the output, in authored order.
    pub selects: Vec<Selected>,
    /// Records named only by id, sorted.
    pub mentions: Vec<String>,
    /// Every line, in runs.
    pub trace: Vec<Trace>,
    /// The rendered bytes.
    pub content: String,
}

impl Projection {
    /// Whether the rendering selects `id`.
    #[must_use]
    pub fn selects(&self, id: &str) -> bool {
        self.selects.iter().any(|s| s.id == id)
    }
}

/// Why nothing was rendered.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RenderError {
    #[error(
        "projection {projection} of {subject}: root {root} is not a record of the model; \
         `war model --json` lists every record id"
    )]
    UnknownRoot {
        projection: String,
        subject: String,
        root: String,
    },
    #[error(
        "projection {projection} of {subject} is {bytes} bytes, over its budget of {max_bytes} \
         bytes; nothing is truncated. The records that cost the most: {largest}. Narrow the \
         document's roots, or raise `max_bytes`"
    )]
    OverBudget {
        projection: String,
        subject: String,
        bytes: usize,
        max_bytes: usize,
        largest: String,
    },
}

impl RenderError {
    /// The rule it is reported under.
    #[must_use]
    pub const fn rule(&self) -> &'static str {
        match self {
            Self::UnknownRoot { .. } => "projection.root-unknown",
            Self::OverBudget { .. } => "projection.over-budget",
        }
    }
}

// ---- The graph.

struct Graph<'a> {
    nodes: BTreeMap<&'a str, &'a Node>,
    out: BTreeMap<&'a str, Vec<&'a Edge>>,
    inc: BTreeMap<&'a str, Vec<&'a Edge>>,
}

impl<'a> Graph<'a> {
    fn new(input: &'a Input) -> Self {
        let mut out: BTreeMap<&str, Vec<&Edge>> = BTreeMap::new();
        let mut inc: BTreeMap<&str, Vec<&Edge>> = BTreeMap::new();
        for e in &input.relations {
            out.entry(e.from.as_str()).or_default().push(e);
            inc.entry(e.to.as_str()).or_default().push(e);
        }
        for v in out.values_mut().chain(inc.values_mut()) {
            v.sort_by(|a, b| (&a.from, &a.kind, &a.to).cmp(&(&b.from, &b.kind, &b.to)));
            v.dedup_by(|a, b| a.from == b.from && a.kind == b.kind && a.to == b.to);
        }
        Self {
            nodes: input.records.iter().map(|n| (n.id.as_str(), n)).collect(),
            out,
            inc,
        }
    }

    /// `(other end, edge)` for each relation of `step` at `id`.
    fn along(&self, id: &str, step: &Step) -> Vec<(&'a str, &'a Edge)> {
        let kind = step.kind.as_str();
        let mut found = Vec::new();
        if step.direction != Direction::In {
            for e in self.out.get(id).into_iter().flatten() {
                if e.kind == kind {
                    found.push((e.to.as_str(), *e));
                }
            }
        }
        if step.direction != Direction::Out {
            for e in self.inc.get(id).into_iter().flatten() {
                if e.kind == kind {
                    found.push((e.from.as_str(), *e));
                }
            }
        }
        if let Some(only) = &step.only {
            found.retain(|(other, _)| self.nodes.get(other).is_some_and(|n| &n.kind == only));
        }
        found
    }
}

fn order_key(n: &Node) -> (&str, usize, &str) {
    (n.source.as_str(), n.line.unwrap_or(0), n.id.as_str())
}

/// The records `def` selects from `seeds`, in authored order.
fn select<'a>(graph: &Graph<'a>, def: &ProjectionDef, seeds: &[&'a str]) -> BTreeSet<&'a str> {
    let types: BTreeSet<&str> = def.select.types.iter().map(String::as_str).collect();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut selected = BTreeSet::new();
    let mut queue = VecDeque::new();
    for s in seeds {
        if seen.insert(*s) {
            if graph
                .nodes
                .get(s)
                .is_some_and(|n| types.contains(n.kind.as_str()))
            {
                selected.insert(*s);
            }
            queue.push_back((*s, 0u32));
        }
    }
    while let Some((id, depth)) = queue.pop_front() {
        if def.select.depth.is_some_and(|d| depth >= d) {
            continue;
        }
        for step in &def.select.through {
            for (other, _) in graph.along(id, step) {
                let Some(n) = graph.nodes.get(other) else {
                    continue;
                };
                if !types.contains(n.kind.as_str()) || !seen.insert(n.id.as_str()) {
                    continue;
                }
                selected.insert(n.id.as_str());
                queue.push_back((n.id.as_str(), depth + 1));
            }
        }
    }
    selected
}

// ---- Text.

fn paragraphs(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur: Vec<&str> = Vec::new();
    for line in body.lines() {
        if line.trim().is_empty() {
            if !cur.is_empty() {
                out.push(cur.join("\n"));
                cur.clear();
            }
        } else {
            cur.push(line.trim_end());
        }
    }
    if !cur.is_empty() {
        out.push(cur.join("\n"));
    }
    out
}

/// The first paragraph, on one line.
fn summary(n: &Node) -> String {
    paragraphs(&n.body)
        .first()
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}

/// Headings inside a body, demoted `by` levels (fence-aware), so a record's
/// own `## Notes` stays under the heading the projection gives it.
fn demote(body: &str, by: usize) -> Vec<String> {
    let mut fence: Option<String> = None;
    body.lines()
        .map(|l| {
            let t = l.trim_start();
            if let Some(f) = &fence {
                if t.starts_with(f.as_str()) {
                    fence = None;
                }
                return l.to_owned();
            }
            if t.starts_with("```") || t.starts_with("~~~") {
                fence = Some(t[..3].to_owned());
                return l.to_owned();
            }
            let hashes = l.chars().take_while(|c| *c == '#').count();
            if hashes > 0 && l[hashes..].starts_with(' ') {
                let level = (hashes + by).min(6);
                format!("{}{}", "#".repeat(level), &l[hashes..])
            } else {
                l.to_owned()
            }
        })
        .collect()
}

fn cell(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "\\|")
}

fn short(rev: &str) -> &str {
    let hex = rev.strip_prefix("sha256:").unwrap_or(rev);
    &hex[..hex.len().min(12)]
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

// ---- Output with origins.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin<'a> {
    Template,
    Document,
    Record(&'a str),
}

struct Out<'a> {
    lines: Vec<(String, Origin<'a>)>,
}

impl<'a> Out<'a> {
    fn line(&mut self, text: impl Into<String>, origin: Origin<'a>) {
        self.lines.push((text.into(), origin));
    }
    fn blank(&mut self, origin: Origin<'a>) {
        if self.lines.last().is_some_and(|(l, _)| !l.is_empty()) {
            self.lines.push((String::new(), origin));
        }
    }
}

struct Ctx<'g, 'a> {
    graph: &'g Graph<'a>,
    selected: &'g BTreeSet<&'a str>,
    /// Records whose bytes reached the output.
    used: BTreeSet<&'a str>,
    mentioned: BTreeSet<&'a str>,
}

impl<'a> Ctx<'_, 'a> {
    /// The ids at the far end of `step` from `id` that a reader may be shown:
    /// every target of an outgoing relation (its author is `id`), and only
    /// selected sources of an incoming one.
    fn ends(&mut self, id: &'a str, step: &Step) -> Vec<(&'a str, &'a Edge)> {
        let mut ends = Vec::new();
        for (other, edge) in self.graph.along(id, step) {
            let authored_here = edge.from == id;
            if authored_here {
                ends.push((other, edge));
            } else if self.selected.contains(other) {
                // The edge is `other`'s bytes: it reaches the output.
                self.used.insert(other);
                ends.push((other, edge));
            }
        }
        ends.sort_by(|a, b| a.0.cmp(b.0));
        ends.dedup_by(|a, b| a.0 == b.0);
        ends
    }

    fn mention(&mut self, id: &'a str) {
        if !self.selected.contains(id) {
            self.mentioned.insert(id);
        }
    }

    /// `Implements OUT-pr1; constrained by CON-pr1, DEC-pr1.`, or nothing.
    fn annotation(&mut self, id: &'a str, steps: &[Step]) -> Option<String> {
        let mut parts = Vec::new();
        for step in steps {
            let ends = self.ends(id, step);
            if ends.is_empty() {
                continue;
            }
            for (other, _) in &ends {
                self.mention(other);
            }
            parts.push(format!(
                "{} {}",
                step.phrase(),
                ends.iter().map(|(o, _)| *o).collect::<Vec<_>>().join(", ")
            ));
        }
        (!parts.is_empty()).then(|| capitalize(&parts.join("; ")) + ".")
    }
}

fn field_label(f: &str) -> String {
    capitalize(&f.replace('_', " "))
}

/// A pin's reading, for an `evaluates` edge or any edge that pins one.
fn pin_reading(graph: &Graph<'_>, edge: &Edge) -> Option<String> {
    let current = graph
        .nodes
        .get(edge.to.as_str())
        .map(|n| n.revision.as_str());
    match (&edge.to_revision, current) {
        (Some(pin), Some(now)) if pin == now => {
            Some(format!("judged {} at its current revision", edge.to))
        }
        (Some(pin), _) => Some(format!(
            "judged {} at an earlier revision ({}): stale until judged again",
            edge.to,
            short(pin)
        )),
        (None, _) if edge.kind == "evaluates" => Some(format!(
            "pins no revision of {}: nobody can say which bytes it judged",
            edge.to
        )),
        (None, _) => None,
    }
}

// ---- Rendering.

fn title_of(def: &ProjectionDef, subject: &Subject) -> String {
    def.title.replace("{title}", &subject.title)
}

fn section_records<'a>(
    graph: &Graph<'a>,
    selected: &BTreeSet<&'a str>,
    seeds: &BTreeSet<&str>,
    types: &[String],
    of: Of,
) -> Vec<&'a Node> {
    let mut v: Vec<&Node> = selected
        .iter()
        .filter_map(|id| graph.nodes.get(id).copied())
        .filter(|n| types.contains(&n.kind))
        .filter(|n| match of {
            Of::All => true,
            Of::Roots => seeds.contains(n.id.as_str()),
            Of::Rest => !seeds.contains(n.id.as_str()),
        })
        .collect();
    v.sort_by(|a, b| order_key(a).cmp(&order_key(b)));
    v
}

fn markdown<'a>(
    def: &ProjectionDef,
    subject: &Subject,
    graph: &Graph<'a>,
    ctx: &mut Ctx<'_, 'a>,
    seeds: &BTreeSet<&str>,
) -> Out<'a> {
    let mut o = Out { lines: Vec::new() };
    let title_origin = if subject.revision.is_some() {
        Origin::Document
    } else {
        Origin::Template
    };
    o.line(format!("# {}", title_of(def, subject)), title_origin);
    o.blank(Origin::Template);
    o.line(
        format!(
            "<!-- Generated by `war` from records (projection `{}`); edit the records, not \
             this file. -->",
            def.name
        ),
        Origin::Template,
    );
    if let Some(intro) = &def.intro {
        o.blank(Origin::Template);
        for l in intro.lines() {
            o.line(l, Origin::Template);
        }
    }
    for section in &def.sections {
        o.blank(Origin::Template);
        o.line(format!("## {}", section.heading), Origin::Template);
        if let Some(intro) = &section.intro {
            o.blank(Origin::Template);
            for l in intro.lines() {
                o.line(l, Origin::Template);
            }
        }
        if matches!(section.block, Block::Heading) {
            continue;
        }
        let records = section_records(graph, ctx.selected, seeds, &section.types, section.of);
        if records.is_empty() {
            o.blank(Origin::Template);
            o.line(
                format!("*{}*", section.empty.as_deref().unwrap_or("None recorded.")),
                Origin::Template,
            );
            continue;
        }
        match &section.block {
            Block::Heading => {}
            Block::Quote => {
                for n in records {
                    ctx.used.insert(n.id.as_str());
                    let r = Origin::Record(n.id.as_str());
                    o.blank(Origin::Template);
                    for l in n.body.trim().lines() {
                        let l = l.trim_end();
                        o.line(
                            if l.is_empty() {
                                ">".to_owned()
                            } else {
                                format!("> {l}")
                            },
                            r,
                        );
                    }
                    o.line(">", r);
                    o.line(format!("> — {}", n.id), r);
                }
            }
            Block::List {
                style,
                show,
                annotate,
                provenance,
                fields,
            } => {
                if *style == ListStyle::Bullets {
                    o.blank(Origin::Template);
                }
                for n in records {
                    ctx.used.insert(n.id.as_str());
                    let r = Origin::Record(n.id.as_str());
                    let note = ctx.annotation(n.id.as_str(), annotate);
                    match style {
                        ListStyle::Bullets => {
                            let paras = match show {
                                Show::Summary => vec![summary(n)],
                                Show::Body => paragraphs(&n.body),
                            };
                            let first = paras.first().map_or(String::new(), |p| {
                                p.split_whitespace().collect::<Vec<_>>().join(" ")
                            });
                            let single = paras.len() <= 1;
                            let mut head = format!("- **{}.** {first}", n.id);
                            if single && let Some(a) = &note {
                                head = format!("{} *{a}*", head.trim_end());
                            }
                            o.line(head.trim_end().to_owned(), r);
                            for p in paras.iter().skip(1) {
                                o.line("", r);
                                for l in p.lines() {
                                    o.line(format!("  {l}"), r);
                                }
                            }
                            if !single && let Some(a) = &note {
                                o.line("", r);
                                o.line(format!("  *{a}*"), r);
                            }
                            for f in fields {
                                if let Some(v) = n.fields.get(f) {
                                    o.line(format!("  - **{}:** {}", field_label(f), cell(v)), r);
                                }
                            }
                        }
                        ListStyle::Blocks => {
                            o.blank(Origin::Template);
                            o.line(format!("### {}", n.id), r);
                            if *provenance {
                                o.line("", r);
                                o.line(
                                    format!(
                                        "`{} · {}{} · revision {}`",
                                        n.kind,
                                        n.source,
                                        n.line.map(|l| format!(":{l}")).unwrap_or_default(),
                                        short(&n.revision)
                                    ),
                                    r,
                                );
                            }
                            let text = match show {
                                Show::Summary => vec![summary(n)],
                                Show::Body => demote(n.body.trim(), 2),
                            };
                            if !text.iter().all(String::is_empty) {
                                o.line("", r);
                                for l in text {
                                    o.line(l.trim_end().to_owned(), r);
                                }
                            }
                            if let Some(a) = note {
                                o.line("", r);
                                o.line(format!("*{a}*"), r);
                            }
                            let shown: Vec<&String> = fields
                                .iter()
                                .filter(|f| n.fields.contains_key(*f))
                                .collect();
                            if !shown.is_empty() {
                                o.line("", r);
                                for f in shown {
                                    o.line(
                                        format!("- **{}:** {}", field_label(f), cell(&n.fields[f])),
                                        r,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Block::Table { columns } => {
                o.blank(Origin::Template);
                let header: Vec<String> = columns
                    .iter()
                    .map(|c| {
                        c.label.clone().unwrap_or_else(|| match &c.column {
                            Column::Id => "ID".to_owned(),
                            Column::Type => "Type".to_owned(),
                            Column::Summary => "Summary".to_owned(),
                            Column::Revision => "Revision".to_owned(),
                            Column::Source => "Source".to_owned(),
                            Column::Field(f) => field_label(f),
                            Column::Relation(s) => capitalize(s.phrase()),
                        })
                    })
                    .collect();
                o.line(format!("| {} |", header.join(" | ")), Origin::Template);
                o.line(
                    format!("|{}", "---|".repeat(columns.len())),
                    Origin::Template,
                );
                for n in records {
                    ctx.used.insert(n.id.as_str());
                    let cells: Vec<String> = columns
                        .iter()
                        .map(|c| match &c.column {
                            Column::Id => n.id.clone(),
                            Column::Type => n.kind.clone(),
                            Column::Summary => cell(&summary(n)),
                            Column::Revision => format!("`{}`", short(&n.revision)),
                            Column::Source => cell(&n.source),
                            Column::Field(f) => {
                                n.fields.get(f).map_or_else(|| "—".to_owned(), |v| cell(v))
                            }
                            Column::Relation(s) => {
                                let ends = ctx.ends(n.id.as_str(), s);
                                for (other, _) in &ends {
                                    ctx.mention(other);
                                }
                                if ends.is_empty() {
                                    "—".to_owned()
                                } else {
                                    ends.iter().map(|(o, _)| *o).collect::<Vec<_>>().join(", ")
                                }
                            }
                        })
                        .collect();
                    o.line(
                        format!("| {} |", cells.join(" | ")),
                        Origin::Record(n.id.as_str()),
                    );
                }
            }
            Block::Tree {
                children,
                show,
                annotate,
                fields,
            } => {
                for n in records {
                    ctx.used.insert(n.id.as_str());
                    let r = Origin::Record(n.id.as_str());
                    o.blank(Origin::Template);
                    o.line(format!("### {}", n.id), r);
                    let text = match show {
                        Show::Summary => vec![summary(n)],
                        Show::Body => demote(n.body.trim(), 2),
                    };
                    if !text.iter().all(String::is_empty) {
                        o.line("", r);
                        for l in text {
                            o.line(l.trim_end().to_owned(), r);
                        }
                    }
                    if let Some(a) = ctx.annotation(n.id.as_str(), annotate) {
                        o.line("", r);
                        o.line(format!("*{a}*"), r);
                    }
                    for group in children {
                        let ends = ctx.ends(n.id.as_str(), &group.relation);
                        if ends.is_empty() && group.empty.is_none() {
                            continue;
                        }
                        let label = group
                            .label
                            .clone()
                            .unwrap_or_else(|| capitalize(group.relation.phrase()));
                        o.blank(Origin::Template);
                        o.line(format!("**{label}**"), Origin::Template);
                        o.blank(Origin::Template);
                        if ends.is_empty() {
                            o.line(
                                format!("*{}*", group.empty.as_deref().unwrap_or_default()),
                                Origin::Template,
                            );
                            continue;
                        }
                        for (other, edge) in ends {
                            match graph.nodes.get(other) {
                                Some(c) if ctx.selected.contains(other) => {
                                    ctx.used.insert(other);
                                    let co = Origin::Record(other);
                                    o.line(
                                        format!("- **{}.** {}", c.id, summary(c))
                                            .trim_end()
                                            .to_owned(),
                                        co,
                                    );
                                    for f in fields {
                                        if let Some(v) = c.fields.get(f) {
                                            o.line(
                                                format!("  - **{}:** {}", field_label(f), cell(v)),
                                                co,
                                            );
                                        }
                                    }
                                    // An `evaluates` edge, or any that pins a
                                    // revision: the pin is its author's bytes.
                                    if let Some(p) = pin_reading(graph, edge) {
                                        let author = Origin::Record(edge.from.as_str());
                                        o.line(format!("  - *{p}*"), author);
                                    }
                                }
                                _ => {
                                    ctx.mention(other);
                                    o.line(format!("- {other}"), r);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    sources(def, graph, ctx, &mut o);
    o
}

/// The provenance table every rendering ends with. It makes the bytes move
/// with every selected record's revision, and with nothing else.
fn sources<'a>(def: &ProjectionDef, graph: &Graph<'a>, ctx: &Ctx<'_, 'a>, o: &mut Out<'a>) {
    o.blank(Origin::Template);
    o.line(format!("## {}", def.sources), Origin::Template);
    o.blank(Origin::Template);
    let mut used: Vec<&Node> = ctx
        .used
        .iter()
        .filter_map(|id| graph.nodes.get(id).copied())
        .collect();
    used.sort_by(|a, b| order_key(a).cmp(&order_key(b)));
    if used.is_empty() {
        o.line("*No record is selected.*", Origin::Template);
        return;
    }
    o.line("| Record | Type | Revision | Source |", Origin::Template);
    o.line("|---|---|---|---|", Origin::Template);
    for n in used {
        o.line(
            format!(
                "| {} | {} | `{}` | {}{} |",
                n.id,
                n.kind,
                short(&n.revision),
                cell(&n.source),
                n.line.map(|l| format!(":{l}")).unwrap_or_default()
            ),
            Origin::Record(n.id.as_str()),
        );
    }
}

fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_default()
}

/// The JSON renderer: the same selection as data, one record per line so
/// each line traces to one record.
fn json<'a>(
    def: &ProjectionDef,
    subject: &Subject,
    graph: &Graph<'a>,
    ctx: &mut Ctx<'_, 'a>,
    seeds: &BTreeSet<&str>,
) -> Out<'a> {
    let mut o = Out { lines: Vec::new() };
    let title_origin = if subject.revision.is_some() {
        Origin::Document
    } else {
        Origin::Template
    };
    o.line("{", Origin::Template);
    o.line(
        format!("  \"projection\": {},", json_str(&def.name)),
        Origin::Template,
    );
    o.line(
        format!("  \"title\": {},", json_str(&title_of(def, subject))),
        title_origin,
    );
    o.line("  \"sections\": [", Origin::Template);
    let n_sections = def.sections.len();
    for (si, section) in def.sections.iter().enumerate() {
        o.line("    {", Origin::Template);
        o.line(
            format!("      \"heading\": {},", json_str(&section.heading)),
            Origin::Template,
        );
        o.line("      \"records\": [", Origin::Template);
        let records = if matches!(section.block, Block::Heading) {
            Vec::new()
        } else {
            section_records(graph, ctx.selected, seeds, &section.types, section.of)
        };
        let mut rows: Vec<(String, &'a str)> = Vec::new();
        for n in records {
            ctx.used.insert(n.id.as_str());
            let steps: Vec<&Step> = match &section.block {
                Block::List { annotate, .. } => annotate.iter().collect(),
                Block::Tree {
                    annotate, children, ..
                } => annotate
                    .iter()
                    .chain(children.iter().map(|c| &c.relation))
                    .collect(),
                Block::Table { columns } => columns
                    .iter()
                    .filter_map(|c| match &c.column {
                        Column::Relation(s) => Some(s),
                        _ => None,
                    })
                    .collect(),
                Block::Heading | Block::Quote => Vec::new(),
            };
            let mut relations = serde_json::Map::new();
            for s in steps {
                let ends = ctx.ends(n.id.as_str(), s);
                for (other, _) in &ends {
                    if ctx.selected.contains(other) {
                        ctx.used.insert(other);
                    } else {
                        ctx.mention(other);
                    }
                }
                let key = format!(
                    "{}{}",
                    match s.direction {
                        Direction::In => "in:",
                        Direction::Out => "out:",
                        Direction::Both => "",
                    },
                    s.kind
                );
                relations.insert(
                    key,
                    serde_json::Value::from(
                        ends.iter()
                            .map(|(o, _)| (*o).to_owned())
                            .collect::<Vec<_>>(),
                    ),
                );
            }
            let v = serde_json::json!({
                "id": n.id,
                "type": n.kind,
                "revision": n.revision,
                "source": n.source,
                "summary": summary(n),
                "body": n.body.trim(),
                "fields": n.fields,
                "relations": relations,
            });
            rows.push((serde_jcs::to_string(&v).unwrap_or_default(), n.id.as_str()));
        }
        let len = rows.len();
        for (i, (row, id)) in rows.into_iter().enumerate() {
            let comma = if i + 1 < len { "," } else { "" };
            o.line(format!("        {row}{comma}"), Origin::Record(id));
        }
        o.line("      ]", Origin::Template);
        o.line(
            if si + 1 < n_sections {
                "    },"
            } else {
                "    }"
            },
            Origin::Template,
        );
    }
    o.line("  ],", Origin::Template);
    o.line("  \"sources\": [", Origin::Template);
    let mut used: Vec<&Node> = ctx
        .used
        .iter()
        .filter_map(|id| graph.nodes.get(id).copied())
        .collect();
    used.sort_by(|a, b| order_key(a).cmp(&order_key(b)));
    let len = used.len();
    for (i, n) in used.into_iter().enumerate() {
        let v = serde_json::json!({"id": n.id, "type": n.kind, "revision": n.revision, "source": n.source});
        let comma = if i + 1 < len { "," } else { "" };
        o.line(
            format!(
                "    {}{comma}",
                serde_jcs::to_string(&v).unwrap_or_default()
            ),
            Origin::Record(n.id.as_str()),
        );
    }
    o.line("  ]", Origin::Template);
    o.line("}", Origin::Template);
    o
}

/// Render `def` of `subject` over `input`, without a budget: what `war
/// impact` reads to learn what a projection selects even when it would be
/// refused for its size.
pub fn render_unbounded(
    input: &Input,
    profile: &DocumentProfile,
    def: &ProjectionDef,
    subject: &Subject,
) -> Result<Projection, RenderError> {
    let graph = Graph::new(input);
    let mut seeds_v: Vec<&str> = Vec::new();
    for s in &subject.seeds {
        let Some((id, _)) = graph.nodes.get_key_value(s.as_str()) else {
            return Err(RenderError::UnknownRoot {
                projection: def.name.clone(),
                subject: subject.id.clone(),
                root: s.clone(),
            });
        };
        seeds_v.push(id);
    }
    let seeds: BTreeSet<&str> = seeds_v.iter().copied().collect();
    let selected = select(&graph, def, &seeds_v);
    let mut ctx = Ctx {
        graph: &graph,
        selected: &selected,
        used: BTreeSet::new(),
        mentioned: BTreeSet::new(),
    };
    let out = match def.renderer {
        Renderer::Markdown => markdown(def, subject, &graph, &mut ctx, &seeds),
        Renderer::Json => json(def, subject, &graph, &mut ctx, &seeds),
    };
    let mut lines = out.lines;
    while lines.last().is_some_and(|(l, _)| l.is_empty()) {
        lines.pop();
    }
    let mut content = String::new();
    let mut per_record: BTreeMap<&str, usize> = BTreeMap::new();
    let mut trace: Vec<Trace> = Vec::new();
    for (i, (text, origin)) in lines.iter().enumerate() {
        let _ = writeln!(content, "{text}");
        let (kind, id, revision) = match origin {
            Origin::Template => (
                "template",
                profile.name.clone(),
                Some(profile.digest.clone()),
            ),
            Origin::Document => ("document", subject.id.clone(), subject.revision.clone()),
            Origin::Record(id) => {
                *per_record.entry(id).or_default() += text.len() + 1;
                (
                    "record",
                    (*id).to_owned(),
                    graph.nodes.get(id).map(|n| n.revision.clone()),
                )
            }
        };
        match trace.last_mut() {
            Some(t) if t.origin == kind && t.id == id && t.revision == revision => {
                t.end_line = i + 1;
            }
            _ => trace.push(Trace {
                start_line: i + 1,
                end_line: i + 1,
                origin: kind.to_owned(),
                id,
                revision,
            }),
        }
    }
    let mut selects: Vec<&Node> = ctx
        .used
        .iter()
        .filter_map(|id| graph.nodes.get(id).copied())
        .collect();
    selects.sort_by(|a, b| order_key(a).cmp(&order_key(b)));
    let mentions: Vec<String> = ctx
        .mentioned
        .iter()
        .filter(|m| !ctx.used.contains(*m))
        .map(|m| (*m).to_owned())
        .collect();
    Ok(Projection {
        schema: SCHEMA.to_owned(),
        projection: def.name.clone(),
        renderer: def.renderer.as_str().to_owned(),
        subject: subject.clone(),
        template: Template {
            profile: profile.name.clone(),
            revision: profile.digest.clone(),
        },
        digest: format!("sha256:{}", crate::sha256_hex(content.as_bytes())),
        bytes: content.len(),
        max_bytes: None,
        selects: selects
            .into_iter()
            .map(|n| Selected {
                id: n.id.clone(),
                kind: n.kind.clone(),
                revision: n.revision.clone(),
                source: n.source.clone(),
                bytes: per_record.get(n.id.as_str()).copied().unwrap_or(0),
            })
            .collect(),
        mentions,
        trace,
        content,
    })
}

/// Render `def` of `subject` over `input`, within `max_bytes` (the
/// projection's own budget when `None`). Over budget is refused by name.
pub fn render(
    input: &Input,
    profile: &DocumentProfile,
    def: &ProjectionDef,
    subject: &Subject,
    max_bytes: Option<usize>,
) -> Result<Projection, RenderError> {
    let mut p = render_unbounded(input, profile, def, subject)?;
    let budget = max_bytes.or(def.max_bytes);
    p.max_bytes = budget;
    if let Some(max) = budget
        && p.bytes > max
    {
        let mut costs: Vec<&Selected> = p.selects.iter().collect();
        costs.sort_by(|a, b| b.bytes.cmp(&a.bytes).then(a.id.cmp(&b.id)));
        let largest = costs
            .iter()
            .take(3)
            .map(|s| format!("{} ({} bytes)", s.id, s.bytes))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(RenderError::OverBudget {
            projection: def.name.clone(),
            subject: subject.id.clone(),
            bytes: p.bytes,
            max_bytes: max,
            largest,
        });
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwarrant_core::projection::parse_document;

    const PROFILE: &str = r#"
schema = "oh.war/profile/v1"
name = "brief"
form = "document"

[records]
types = ["outcome", "requirement", "decision", "option"]

[relations]
allow = ["implements", "constrains", "selected_over", "evaluates"]

[[projections]]
name = "brief"
title = "{title}: brief"
[projections.select]
types = ["outcome", "requirement", "obligation", "decision", "option"]
through = ["in:evaluates", "out:selected_over"]

[[projections.sections]]
heading = "Outcomes"
block = "quote"
types = ["outcome"]

[[projections.sections]]
heading = "Requirements"
block = "list"
types = ["requirement"]
annotate = ["out:implements", "in:constrains"]

[[projections.sections]]
heading = "Coverage"
block = "table"
types = ["requirement"]
columns = ["id", "summary=Requirement", "in:evaluates=Evaluated by"]

[[projections.sections]]
heading = "Checks"
block = "tree"
types = ["requirement"]
fields = ["scope"]
children = [{ relation = "in:evaluates", label = "Evaluated by", empty = "Untested." }]

[[projections.sections]]
heading = "Decisions"
block = "tree"
types = ["decision"]
annotate = ["out:constrains"]
children = [{ relation = "out:selected_over", label = "Alternatives" }]
"#;

    fn node(id: &str, kind: &str, line: usize, body: &str) -> Node {
        Node {
            id: id.into(),
            kind: kind.into(),
            source: "docs/records/a/10-records.md".into(),
            line: Some(line),
            revision: format!("sha256:{}", crate::sha256_hex(body.as_bytes())),
            body: body.into(),
            fields: BTreeMap::new(),
        }
    }

    fn edge(from: &str, kind: &str, to: &str) -> Edge {
        Edge {
            from: from.into(),
            kind: kind.into(),
            to: to.into(),
            to_revision: None,
        }
    }

    fn input() -> Input {
        let mut obl = node(
            "W-1/OBL-001",
            "obligation",
            0,
            "A token at minute 16 is refused.",
        );
        obl.source = "docs/warrants/W-1/atoms/60-assurance.md".into();
        obl.line = None;
        obl.fields
            .insert("scope".into(), "the reset endpoint".into());
        Input {
            records: vec![
                node("OUT-1", "outcome", 1, "Users regain access."),
                node(
                    "REQ-1",
                    "requirement",
                    5,
                    "Tokens expire in 15 minutes.\n\nSecond paragraph.",
                ),
                node("REQ-2", "requirement", 9, "Tokens are single-use."),
                node("DEC-1", "decision", 12, "Signed tokens."),
                node("OPT-1", "option", 15, "Opaque tokens."),
                obl,
            ],
            relations: vec![
                edge("REQ-1", "implements", "OUT-1"),
                edge("DEC-1", "constrains", "REQ-1"),
                edge("DEC-1", "selected_over", "OPT-1"),
                edge("W-1/OBL-001", "evaluates", "REQ-1"),
            ],
        }
    }

    fn subject(seeds: &[&str]) -> Subject {
        Subject {
            id: "a/brief".into(),
            kind: "document".into(),
            title: "Reset".into(),
            seeds: seeds.iter().map(|s| (*s).to_owned()).collect(),
            revision: Some("sha256:doc".into()),
            source: Some("docs/records/a/documents.toml".into()),
        }
    }

    fn profile() -> DocumentProfile {
        parse_document(PROFILE, "sha256:tpl".into()).unwrap()
    }

    fn area() -> Vec<&'static str> {
        vec!["OUT-1", "REQ-1", "REQ-2", "DEC-1"]
    }

    #[test]
    fn renders_every_block_and_traces_every_line() {
        let p = profile();
        let r = render(&input(), &p, &p.projections[0], &subject(&area()), None).unwrap();
        let c = &r.content;
        assert!(c.starts_with("# Reset: brief\n"), "{c}");
        assert!(c.contains("> Users regain access.\n>\n> — OUT-1\n"), "{c}");
        assert!(
            c.contains("- **REQ-1.** Tokens expire in 15 minutes."),
            "{c}"
        );
        assert!(
            c.contains("  *Implements OUT-1; constrained by DEC-1.*"),
            "{c}"
        );
        assert!(
            c.contains("| REQ-1 | Tokens expire in 15 minutes. | W-1/OBL-001 |"),
            "{c}"
        );
        assert!(c.contains("- **W-1/OBL-001.** A token at minute 16 is refused.\n  - **Scope:** the reset endpoint\n  - *pins no revision of REQ-1"), "{c}");
        assert!(c.contains("*Untested.*"), "{c}");
        assert!(
            c.contains("**Alternatives**\n\n- **OPT-1.** Opaque tokens."),
            "{c}"
        );
        // Every line is traced, in order, with no gap.
        let lines = c.lines().count();
        let mut next = 1;
        for t in &r.trace {
            assert_eq!(t.start_line, next);
            assert!(t.end_line >= t.start_line);
            next = t.end_line + 1;
        }
        assert_eq!(next, lines + 1);
        let req = r.trace.iter().find(|t| t.id == "REQ-1").unwrap();
        assert_eq!(req.origin, "record");
        assert_eq!(
            req.revision.as_deref(),
            Some(input().records[1].revision.as_str())
        );
        assert_eq!(r.trace[0].origin, "document");
        // REQ-1 is constrained by DEC-1 and implements OUT-1: both selected.
        let ids: Vec<&str> = r.selects.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(
            ids,
            ["OUT-1", "REQ-1", "REQ-2", "DEC-1", "OPT-1", "W-1/OBL-001"]
        );
        assert!(r.mentions.is_empty(), "{:?}", r.mentions);
        assert!(c.ends_with("|\n"));
    }

    #[test]
    fn a_change_moves_exactly_the_renderings_that_select_it() {
        let p = profile();
        let def = &p.projections[0];
        // Only the decisions, seeded at DEC-1: REQ-1 is mentioned, not selected.
        let narrow = subject(&["DEC-1"]);
        let base = render(&input(), &p, def, &narrow, None).unwrap();
        assert!(!base.selects("REQ-1"));
        assert_eq!(base.mentions, ["REQ-1"]);
        let mut changed = input();
        changed.records[1].body = "Tokens expire in 10 minutes.".into();
        changed.records[1].revision = "sha256:moved".into();
        let after = render(&changed, &p, def, &narrow, None).unwrap();
        assert_eq!(base.content, after.content, "REQ-1 is not selected here");
        let wide = render(&input(), &p, def, &subject(&area()), None).unwrap();
        let wide_after = render(&changed, &p, def, &subject(&area()), None).unwrap();
        assert!(wide.selects("REQ-1"));
        assert_ne!(wide.content, wide_after.content);
        // A revision change alone (no visible text) still moves the bytes.
        let mut quiet = input();
        quiet.records[1].revision = "sha256:quiet".into();
        let q = render(&quiet, &p, def, &subject(&area()), None).unwrap();
        assert_ne!(wide.content, q.content);
    }

    #[test]
    fn over_budget_is_refused_by_name_and_an_unknown_root_too() {
        let p = profile();
        let def = &p.projections[0];
        let full = render(&input(), &p, def, &subject(&area()), None).unwrap();
        let e = render(&input(), &p, def, &subject(&area()), Some(full.bytes - 1)).unwrap_err();
        assert_eq!(e.rule(), "projection.over-budget");
        assert!(e.to_string().contains("nothing is truncated"), "{e}");
        let ok = render(&input(), &p, def, &subject(&area()), Some(full.bytes)).unwrap();
        assert_eq!(ok.max_bytes, Some(full.bytes));
        let e = render(&input(), &p, def, &subject(&["NOPE-1"]), None).unwrap_err();
        assert_eq!(e.rule(), "projection.root-unknown");
    }

    #[test]
    fn the_json_renderer_carries_the_same_selection_one_record_per_line() {
        let text = PROFILE.replace(
            "title = \"{title}: brief\"",
            "title = \"{title}: brief\"\nrenderer = \"json\"",
        );
        let p = parse_document(&text, "sha256:tpl".into()).unwrap();
        let r = render(&input(), &p, &p.projections[0], &subject(&area()), None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&r.content).unwrap();
        assert_eq!(v["title"], "Reset: brief");
        assert_eq!(v["sections"][1]["records"][0]["id"], "REQ-1");
        assert_eq!(
            v["sections"][1]["records"][0]["relations"]["out:implements"][0],
            "OUT-1"
        );
        for t in r.trace.iter().filter(|t| t.origin == "record") {
            for l in t.start_line..=t.end_line {
                let line = r.content.lines().nth(l - 1).unwrap();
                assert!(line.contains(&format!("\"id\":\"{}\"", t.id)), "{line}");
            }
        }
    }
}
