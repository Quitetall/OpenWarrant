// SPDX-License-Identifier: Apache-2.0
//! `war context <target>` — the context compiler, and the index it serves
//! (OpenWarrant Format 1.0, `docs/format/FORMAT-1.0.md` section 10).
//!
//! An agent asks for a target (a Warrant or ticket, an item, a record, a
//! file) and receives the records that target needs, each cited by id,
//! type, `path:line` and revision, inside a byte budget. Whatever does not
//! fit is listed by its one-line summary (`[S]`) so the agent can ask for it
//! the same way. The output is a pure function of the tracked documents:
//! identical arguments and bytes give identical output bytes.
//!
//! # The seam
//!
//! The compiler reads a [`Graph`]: [`Unit`]s (a record with its title,
//! summary, location, revision, primary state and exact text) and the
//! relations between them. [`adapter::graph`] builds it from
//! `oh.war/model/v1` alone: each unit is a model record with its title,
//! document, line, revision and `content_digest`, and its text is the
//! record's own span of its source as the model gives it.
//!
//! # What is pure here
//!
//! [`compile`] (roots, walk, order, budget, rendering) and [`index`] take
//! the graph and their arguments and nothing else: no clock, no
//! environment, no claim locks, no directory order.

pub mod adapter;
pub mod task;

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

/// The schema of `--json` output.
pub const SCHEMA: &str = "oh.war/context/v1";
/// The index `war admin compile` writes (section 10.2).
pub const INDEX_PATH: &str = "docs/generated/llms.txt";
/// Default budget, in bytes (section 10.3).
pub const DEFAULT_BUDGET: usize = 16_000;
/// Default walk depth.
pub const DEFAULT_DEPTH: usize = 3;
/// The walk this compiler uses: section 10.3's table, unchanged.
pub const WALK: &str = "core";
/// The format version the index and packets declare.
pub const FORMAT: &str = "openwarrant/1.0";

/// The refusal rules (section 11.2).
pub const RULE_TARGET_UNKNOWN: &str = "context.target-unknown";
pub const RULE_OVER_BUDGET: &str = "context.over-budget";
/// A relation whose target names no record: skipped by the walk, reported.
pub const RULE_TARGET_UNREAD: &str = "format.target-unknown";

// ---- The seam -------------------------------------------------------------

/// One record as the compiler reads it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, serde::Deserialize)]
pub struct Unit {
    /// Global id (`OW-WAR-0146/OBL-001`, `t-0d61d/i-de0e`, `REQ-pr1`).
    pub id: String,
    /// Its type (`warrant`, `section`, `obligation`, `item`, …).
    pub kind: String,
    /// One-line title; `None` for a legacy record that has none.
    pub title: Option<String>,
    /// A work document's one-line `summary` (section 10.1), where authored.
    pub summary: Option<String>,
    /// Whether it is a work document (a Warrant or ticket): an alias target
    /// resolves to it, and its units join it at distance 0.
    pub work: bool,
    /// The global id of the work document holding it, if any.
    pub document: Option<String>,
    /// The global id of the section it sits in, if any (an item's
    /// `S-checklist`).
    pub section: Option<String>,
    /// `[start, end]` byte offsets of its span in `source`.
    pub span: [usize; 2],
    /// Repository-relative path, `/`-separated.
    pub source: String,
    /// 1-based line of its heading (or first line).
    pub line: usize,
    /// `sha256:<hex>`.
    pub revision: String,
    /// `sha256:<hex>` over meaning (section 9), when the model carries it.
    pub content_digest: Option<String>,
    /// The exact bytes of its span; `None` when the span is not known, in
    /// which case the record is only ever given by its summary.
    pub text: Option<String>,
    /// Its primary state (section 10.2), `None` for `-`.
    pub state: Option<String>,
    /// A work document's `paths` globs (section 3.3).
    pub paths: Vec<String>,
    /// A work document's UUIDv7, so `war://<uuid>` resolves to it.
    pub uuid: Option<String>,
    /// Whether an editable preset wraps its text in `rec` markers (section
    /// 10.5): a unit of a Format 1.0 document that is not the document's
    /// own record (its frontmatter and preamble).
    pub editable: bool,
}

/// One relation, `{from, kind, to}`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, serde::Deserialize)]
pub struct Edge {
    pub from: String,
    pub kind: String,
    pub to: String,
}

/// What the compiler reads.
#[derive(Debug, Clone, Default, Serialize, serde::Deserialize)]
pub struct Graph {
    /// The repository's name, for the index heading.
    pub name: String,
    pub units: BTreeMap<String, Unit>,
    pub edges: Vec<Edge>,
    /// Findings the adapter made while reading (sorted, deduplicated on use).
    pub diagnostics: Vec<Finding>,
    /// What a preset that reads state read besides the documents (section
    /// 10.4); `None` for a graph of the documents alone.
    pub tracking: Option<Tracking>,
}

/// The journals, claims and time a preset that reads state answers at
/// (section 10.4). The adapter fills it; the compiler only renders it, so
/// [`compile`] stays a function of its graph.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct Tracking {
    /// The time, as given (`--at`) or read from the clock, RFC 3339.
    pub at: String,
    /// The `basis_digest` rule over `[path, sha256 of the file's bytes]` for
    /// every journal and every claim file, sorted by path: `sha256:<64 hex>`.
    pub journal_digest: String,
    /// An item's `resume` state line, without its newline: `state <primary>[
    /// · holder <actor> · lease <until>][ · by <actor> · <date> · <level>]`.
    pub state_lines: BTreeMap<String, String>,
    /// A work document's candidates of section 11.5, in order: what `next`
    /// would offer of it.
    pub next: BTreeMap<String, Vec<String>>,
}

impl Graph {
    /// Add a unit; a later unit with the same id does not replace the first.
    pub fn add(&mut self, unit: Unit) {
        self.units.entry(unit.id.clone()).or_insert(unit);
    }
}

/// A diagnostic, `{rule, record, message}`.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, serde::Deserialize)]
pub struct Finding {
    pub rule: String,
    pub record: String,
    pub message: String,
}

// ---- Arguments, refusals, output -----------------------------------------

/// The built-in presets (section 10.4), in the table's order.
pub const PRESETS: [&str; 5] = ["agent", "agent.min", "human", "review", "resume"];
/// The preset without `--preset`.
pub const DEFAULT_PRESET: &str = "agent";

/// The walk steps of section 10.3, in order: what the walk named `core` follows.
pub const CORE_STEPS: [&str; 8] = [
    "out:implements",
    "out:depends_on",
    "in:constrains",
    "out:selected_over",
    "in:evaluates",
    "out:part_of",
    "in:part_of",
    "in:implements",
];

/// A preset (section 10.4): a named set of choices for the compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    pub name: String,
    /// `core` for one that walks the core steps; else its own name.
    pub walk: String,
    /// The steps it follows, in order, by name ([`CORE_STEPS`], and
    /// `out:evaluates`).
    pub steps: Vec<&'static str>,
    pub depth: usize,
    /// Bytes; `None` for a preset without a budget (every record `[F]`).
    pub budget: Option<usize>,
    /// Record types it leaves out.
    pub omit_types: Vec<String>,
    /// Field names it cuts from the text it gives (the line and its
    /// continuations).
    pub omit_fields: Vec<String>,
    /// Its records are wrapped in `rec` markers (section 10.5).
    pub editable: bool,
    /// It reads the journals and claims of section 11, at a time.
    pub reads_state: bool,
    /// `human`: the work document's own units first, in document order.
    pub own_first: bool,
    /// `resume`: the target's record and items, a state line under each
    /// item, and `## Next`.
    pub resume: bool,
}

impl Preset {
    /// A built-in preset by name.
    #[must_use]
    pub fn builtin(name: &str) -> Option<Self> {
        let core = Self {
            name: name.to_owned(),
            walk: WALK.to_owned(),
            steps: CORE_STEPS.to_vec(),
            depth: DEFAULT_DEPTH,
            budget: Some(DEFAULT_BUDGET),
            omit_types: Vec::new(),
            omit_fields: Vec::new(),
            editable: false,
            reads_state: false,
            own_first: false,
            resume: false,
        };
        Some(match name {
            "agent" => core,
            "agent.min" => Self {
                walk: name.to_owned(),
                steps: vec!["out:implements", "in:constrains", "in:evaluates"],
                depth: 1,
                budget: Some(4000),
                omit_types: vec!["option".to_owned(), "consequence".to_owned()],
                omit_fields: vec!["rationale".to_owned()],
                ..core
            },
            "human" => Self {
                budget: None,
                editable: true,
                own_first: true,
                ..core
            },
            "review" => Self {
                walk: name.to_owned(),
                steps: vec!["in:evaluates", "out:evaluates"],
                depth: 2,
                budget: None,
                reads_state: true,
                ..core
            },
            "resume" => Self {
                depth: 1,
                budget: None,
                reads_state: true,
                resume: true,
                ..core
            },
            _ => return None,
        })
    }

    /// A preset a profile declares (`[[presets]]`, section 10.4), by name:
    /// its steps (the core walk's when it names none), depth (3), budget
    /// (16000, or `none`), what it leaves out, whether it is editable and
    /// whether it reads state. It walks under its own name unless its
    /// steps are the core ones. `Err` names a step no walk knows.
    pub fn declared(
        name: &str,
        profiles: &openwarrant_core::format_profile::Profiles,
    ) -> Option<Result<Self, String>> {
        use openwarrant_core::format_profile::Budget;
        let p = profiles
            .found()
            .flat_map(|p| p.presets.iter())
            .find(|p| p.name == name)?;
        let mut steps = Vec::new();
        for s in &p.steps {
            match step(s) {
                Some(found) => steps.push(found.0),
                None => {
                    return Some(Err(format!(
                        "preset `{name}` names the step `{s}`, which no walk follows (the steps \
                         are {} and out:evaluates)",
                        CORE_STEPS.join(", ")
                    )));
                }
            }
        }
        if steps.is_empty() {
            steps = CORE_STEPS.to_vec();
        }
        let walk = if steps == CORE_STEPS {
            WALK.to_owned()
        } else {
            name.to_owned()
        };
        Some(Ok(Self {
            name: name.to_owned(),
            walk,
            steps,
            depth: p
                .depth
                .and_then(|d| usize::try_from(d).ok())
                .unwrap_or(DEFAULT_DEPTH),
            budget: match p.budget {
                Some(Budget::None) => None,
                Some(Budget::Bytes(b)) => Some(usize::try_from(b).unwrap_or(usize::MAX)),
                None => Some(DEFAULT_BUDGET),
            },
            omit_types: p.omit_types.clone(),
            omit_fields: p.omit_fields.clone(),
            editable: p.editable,
            reads_state: p.reads_state,
            own_first: false,
            resume: false,
        }))
    }
}

/// `war context` arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Bytes (`--tokens n` is `4n`); `None` for no budget.
    pub budget: Option<usize>,
    pub depth: usize,
    /// Refuse rather than give any record by summary only.
    pub strict: bool,
    /// The preset; its budget and depth are the defaults of the two above.
    pub preset: Preset,
}

impl Default for Options {
    fn default() -> Self {
        Self::of(Preset::builtin(DEFAULT_PRESET).unwrap_or_else(|| unreachable!()))
    }
}

impl Options {
    /// A preset's own choices.
    #[must_use]
    pub fn of(preset: Preset) -> Self {
        Self {
            budget: preset.budget,
            depth: preset.depth,
            strict: false,
            preset,
        }
    }

    /// `preset` with `--budget` / `--tokens` (section 10.3: `--tokens n`
    /// means `--budget 4n`) and `--depth` in place of its own, where given.
    #[must_use]
    pub fn with(
        preset: Preset,
        budget: Option<usize>,
        tokens: Option<usize>,
        depth: Option<usize>,
    ) -> Self {
        let mut o = Self::of(preset);
        if let Some(b) = budget.or_else(|| tokens.map(|t| t.saturating_mul(4))) {
            o.budget = Some(b);
        }
        if let Some(d) = depth {
            o.depth = d;
        }
        o
    }

    /// The budget from `--budget` / `--tokens` (section 10.3: `--tokens n`
    /// means `--budget 4n`), the default when neither is given.
    #[must_use]
    pub fn budget_of(budget: Option<usize>, tokens: Option<usize>) -> usize {
        budget
            .or_else(|| tokens.map(|t| t.saturating_mul(4)))
            .unwrap_or(DEFAULT_BUDGET)
    }
}

/// A refusal: nothing is emitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub rule: &'static str,
    pub message: String,
    /// For `context.over-budget`: the bytes the refused output needs.
    pub needed: Option<usize>,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.rule, self.message)
    }
}

/// How a record reached the selection.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Via {
    pub step: String,
    pub from: String,
}

/// One selected record, as `--json` gives it.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PacketRecord {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    /// The title as the index shows it.
    pub title: String,
    /// The primary state, `-` when none.
    pub state: String,
    pub source: String,
    pub line: usize,
    pub revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_digest: Option<String>,
    pub distance: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<Via>,
    /// `full` or `summary`.
    pub tier: String,
    /// The exact span, for `full`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// `oh.war/context/v1`.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Packet {
    pub schema: String,
    pub target: String,
    /// Bytes; `null` for a preset without a budget.
    pub budget: Option<usize>,
    /// UTF-8 bytes of the Markdown rendering.
    pub bytes: usize,
    pub depth: usize,
    /// The preset it was compiled under (section 10.4).
    pub preset: String,
    pub walk: String,
    pub selection_digest: String,
    /// A preset that reads state: the digest of the journals and claim
    /// files it read (section 10.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal: Option<String>,
    /// A preset that reads state: the time it answered at.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    pub records: Vec<PacketRecord>,
    pub diagnostics: Vec<Finding>,
    /// The Markdown rendering itself; not serialized (it is the default
    /// output, and `bytes` is its size).
    #[serde(skip)]
    pub markdown: String,
}

// ---- Shapes ----------------------------------------------------------------

/// An item id: `i-` and 3 to 16 lowercase hex digits (section 5).
#[must_use]
pub fn is_item_id(s: &str) -> bool {
    openwarrant_core::ticket::is_item_id(s)
}

/// The section id slug (section 3.4): ASCII letters lowercased, digits
/// kept, every other run replaced by one `-`, `-` trimmed at both ends.
#[must_use]
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if dash && !out.is_empty() {
                out.push('-');
            }
            dash = false;
            out.push(c.to_ascii_lowercase());
        } else {
            dash = true;
        }
    }
    out
}

/// Section 10.3's type rank.
#[must_use]
pub fn type_rank(kind: &str) -> usize {
    const RANK: [&str; 18] = [
        "warrant",
        "ticket",
        "section",
        "item",
        "requirement",
        "outcome",
        "constraint",
        "decision",
        "interface",
        "term",
        "option",
        "consequence",
        "obligation",
        "check",
        "kpi",
        "milestone",
        "stage",
        "instruction",
    ];
    match kind {
        "warrant" | "ticket" => 0,
        other => RANK.iter().position(|k| *k == other).unwrap_or(RANK.len()),
    }
}

/// A title for a record that has none (section 10.2): its first non-blank
/// body line, cut at 120 bytes on a character boundary and ended with `…`
/// when cut.
#[must_use]
pub fn cut_title(line: &str) -> String {
    let line = line.trim();
    if line.len() <= 120 {
        return line.to_owned();
    }
    let mut end = 120;
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &line[..end])
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Its title, else `-`. A record whose heading has none is given its first
/// body line as its title by the adapter, from the model's `body`.
fn shown_title(u: &Unit) -> String {
    u.title
        .as_deref()
        .map(one_line)
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "-".to_owned())
}

fn shown_state(u: &Unit) -> String {
    u.state.clone().unwrap_or_else(|| "-".to_owned())
}

/// `<id> · <type> · <state> · <title> · <source>:<line>`.
fn summary_line(u: &Unit) -> String {
    format!(
        "{} · {} · {} · {} · {}:{}",
        u.id,
        u.kind,
        shown_state(u),
        shown_title(u),
        u.source,
        u.line
    )
}

fn short_revision(rev: &str) -> String {
    let hex = rev.strip_prefix("sha256:").unwrap_or(rev);
    format!("sha256:{}", hex.get(..12).unwrap_or(hex))
}

/// `sha256:` over the RFC 8785 bytes of `[[id, revision], …]` in id order
/// (section 8's `basis_digest` rule).
#[must_use]
pub fn selection_digest<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    let mut pairs: Vec<(&str, &str)> = pairs.into_iter().collect();
    pairs.sort_unstable();
    let bytes = serde_jcs::to_string(&pairs).unwrap_or_default();
    format!(
        "sha256:{}",
        openwarrant_compiler::sha256_hex(bytes.as_bytes())
    )
}

// ---- Glob ------------------------------------------------------------------

/// `pattern` matches `path`: `*` any run within a segment, `?` one
/// character within a segment, `**` any run of segments; anything else
/// literal. A pattern ending in `/` matches everything under it.
#[must_use]
pub fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern = pattern.trim_start_matches("./");
    let path = path.trim_start_matches("./");
    if let Some(dir) = pattern.strip_suffix('/') {
        return path.starts_with(&format!("{dir}/"));
    }
    let p: Vec<&str> = pattern.split('/').collect();
    let s: Vec<&str> = path.split('/').collect();
    segs(&p, &s)
}

fn segs(p: &[&str], s: &[&str]) -> bool {
    match p.split_first() {
        None => s.is_empty(),
        Some((&"**", rest)) => (0..=s.len()).any(|i| segs(rest, &s[i..])),
        Some((first, rest)) => s
            .split_first()
            .is_some_and(|(h, t)| seg(first.as_bytes(), h.as_bytes()) && segs(rest, t)),
    }
}

fn seg(p: &[u8], s: &[u8]) -> bool {
    match p.split_first() {
        None => s.is_empty(),
        Some((b'*', rest)) => (0..=s.len()).any(|i| seg(rest, &s[i..])),
        Some((b'?', rest)) => !s.is_empty() && seg(rest, &s[1..]),
        Some((c, rest)) => s.first() == Some(c) && seg(rest, &s[1..]),
    }
}

// ---- Roots -----------------------------------------------------------------

/// Step 1's result: the roots, the distance-0 set, and the records placed
/// at distance 1 by the target's shape.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Seeds {
    pub roots: Vec<String>,
    pub zero: BTreeSet<String>,
    pub one: BTreeMap<String, String>,
}

fn normalize_path(target: &str) -> String {
    let mut t = target.trim();
    while let Some(rest) = t.strip_prefix("./") {
        t = rest;
    }
    t.trim_end_matches('/').to_owned()
}

/// Resolve `target` to its seeds (section 10.3, step 1).
pub fn resolve(graph: &Graph, target: &str) -> Result<Seeds, Refusal> {
    let mut seeds = Seeds::default();
    let members = |doc: &str| -> Vec<String> {
        graph
            .units
            .values()
            .filter(|u| u.document.as_deref() == Some(doc))
            .map(|u| u.id.clone())
            .collect()
    };
    let work_root = |seeds: &mut Seeds, id: &str| {
        seeds.roots.push(id.to_owned());
        seeds.zero.insert(id.to_owned());
        seeds.zero.extend(members(id));
    };
    let id = target.trim();
    let unit = graph.units.get(id).or_else(|| {
        // A bare item id, when exactly one document has it.
        if is_item_id(id) {
            let mut hits = graph
                .units
                .values()
                .filter(|u| u.kind == "item" && u.id.rsplit('/').next() == Some(id));
            match (hits.next(), hits.next()) {
                (Some(u), None) => Some(u),
                _ => None,
            }
        } else {
            None
        }
    });
    let unit = unit.or_else(|| {
        id.strip_prefix("war://").and_then(|uuid| {
            graph
                .units
                .values()
                .find(|u| u.work && u.uuid.as_deref() == Some(uuid))
        })
    });
    if let Some(u) = unit {
        if u.work {
            work_root(&mut seeds, &u.id);
        } else {
            seeds.roots.push(u.id.clone());
            seeds.zero.insert(u.id.clone());
            if u.kind == "item"
                && let Some(doc) = &u.document
            {
                // The item, and its document's Intent at distance 1; a
                // document without one gives its own record instead.
                let intent = format!("{doc}/S-intent");
                if graph.units.contains_key(&intent) {
                    seeds.one.insert(intent, u.id.clone());
                } else if graph.units.contains_key(doc) {
                    seeds.one.insert(doc.clone(), u.id.clone());
                }
            }
        }
        return Ok(seeds);
    }
    // A path: a document's, else any path a work document's globs match.
    let path = normalize_path(id);
    let in_file: Vec<&Unit> = graph.units.values().filter(|u| u.source == path).collect();
    if !in_file.is_empty() {
        let docs: BTreeSet<String> = in_file
            .iter()
            .filter_map(|u| {
                if u.work {
                    Some(u.id.clone())
                } else {
                    u.document.clone()
                }
            })
            .filter(|d| graph.units.contains_key(d))
            .collect();
        if docs.is_empty() {
            for u in in_file {
                seeds.roots.push(u.id.clone());
                seeds.zero.insert(u.id.clone());
            }
        } else {
            for d in docs {
                work_root(&mut seeds, &d);
            }
        }
        return Ok(seeds);
    }
    let matched: Vec<String> = graph
        .units
        .values()
        .filter(|u| u.work && u.paths.iter().any(|g| glob_match(g, &path)))
        .map(|u| u.id.clone())
        .collect();
    if matched.is_empty() {
        return Err(Refusal {
            rule: RULE_TARGET_UNKNOWN,
            message: format!(
                "{target:?} is no record, item, Warrant or ticket here, no tracked document, and no path a Warrant's paths match; `docs/generated/llms.txt` lists every id"
            ),
            needed: None,
        });
    }
    for d in matched {
        work_root(&mut seeds, &d);
    }
    Ok(seeds)
}

// ---- Walk ------------------------------------------------------------------

/// Every step a preset may name: `(name, direction out?, kind, from-type)`.
/// Section 10.3's eight are [`CORE_STEPS`]; `review` adds `out:evaluates`
/// (section 10.4).
const STEPS: [(&str, bool, &str, Option<&str>); 9] = [
    ("out:implements", true, "implements", None),
    ("out:depends_on", true, "depends_on", None),
    ("in:constrains", false, "constrains", None),
    ("out:selected_over", true, "selected_over", None),
    ("in:evaluates", false, "evaluates", None),
    ("out:part_of", true, "part_of", None),
    ("in:part_of", false, "part_of", Some("consequence")),
    ("in:implements", false, "implements", Some("interface")),
    ("out:evaluates", true, "evaluates", None),
];

fn step(name: &str) -> Option<(&'static str, bool, &'static str, Option<&'static str>)> {
    STEPS.iter().find(|s| s.0 == name).copied()
}

/// The step that gives an item root its document's Intent (or the document
/// itself) at distance 1, outside the walk's eight (section 10.3, step 1).
pub const STEP_ITEM_INTENT: &str = "item-intent";

fn item_intent(item: &str) -> Via {
    Via {
        step: STEP_ITEM_INTENT.to_owned(),
        from: item.to_owned(),
    }
}

/// One record of the selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reached {
    pub id: String,
    pub distance: usize,
    pub via: Option<Via>,
}

/// Step 2: breadth-first from the seeds, to `depth`, over the core steps.
#[must_use]
pub fn walk(graph: &Graph, seeds: &Seeds, depth: usize) -> (Vec<Reached>, Vec<Finding>) {
    walk_over(graph, seeds, depth, &CORE_STEPS, &[])
}

/// Step 2 over `steps`, in their order, never reaching a record of a type
/// in `omit` (a preset that leaves it out, section 10.4).
#[must_use]
pub fn walk_over(
    graph: &Graph,
    seeds: &Seeds,
    depth: usize,
    steps: &[&str],
    omit: &[String],
) -> (Vec<Reached>, Vec<Finding>) {
    let steps: Vec<_> = steps.iter().filter_map(|s| step(s)).collect();
    let omitted = |id: &str| graph.units.get(id).is_some_and(|u| omit.contains(&u.kind));
    let mut out_edges: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
    let mut in_edges: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
    for e in &graph.edges {
        out_edges
            .entry((e.from.as_str(), e.kind.as_str()))
            .or_default()
            .push(e.to.as_str());
        in_edges
            .entry((e.to.as_str(), e.kind.as_str()))
            .or_default()
            .push(e.from.as_str());
    }
    let mut findings = BTreeSet::new();
    let mut reached: BTreeMap<String, (usize, Option<Via>)> = BTreeMap::new();
    for id in &seeds.zero {
        reached.insert(id.clone(), (0, None));
    }
    let mut frontier: BTreeSet<String> = seeds.zero.clone();
    for level in 1..=depth {
        // to → (step index, from): the first by step order, then from id.
        let mut found: BTreeMap<String, (usize, String)> = BTreeMap::new();
        for from in &frontier {
            for (si, (_, out, kind, from_type)) in steps.iter().enumerate() {
                let next = if *out {
                    out_edges.get(&(from.as_str(), *kind))
                } else {
                    in_edges.get(&(from.as_str(), *kind))
                };
                for &to in next.into_iter().flatten() {
                    let Some(unit) = graph.units.get(to) else {
                        let (a, b) = if *out {
                            (from.as_str(), to)
                        } else {
                            (to, from.as_str())
                        };
                        findings.insert(Finding {
                            rule: RULE_TARGET_UNREAD.to_owned(),
                            record: a.to_owned(),
                            message: format!(
                                "{a} {kind} {b}: {to} is not a record here; the walk skipped it"
                            ),
                        });
                        continue;
                    };
                    if from_type.is_some_and(|t| unit.kind != t)
                        || reached.contains_key(to)
                        || omitted(to)
                    {
                        continue;
                    }
                    let cand = (si, from.clone());
                    found
                        .entry(to.to_owned())
                        .and_modify(|c| {
                            if cand < *c {
                                *c = cand.clone();
                            }
                        })
                        .or_insert(cand);
                }
            }
        }
        let mut next: BTreeSet<String> = BTreeSet::new();
        for (to, (si, from)) in found {
            reached.insert(
                to.clone(),
                (
                    level,
                    Some(Via {
                        step: steps[si].0.to_owned(),
                        from,
                    }),
                ),
            );
            next.insert(to);
        }
        if level == 1 {
            for (id, from) in &seeds.one {
                if !reached.contains_key(id) && !omitted(id) {
                    reached.insert(id.clone(), (1, Some(item_intent(from))));
                    next.insert(id.clone());
                }
            }
        }
        frontier = next;
        if frontier.is_empty() {
            break;
        }
    }
    if depth == 0 {
        for (id, from) in &seeds.one {
            reached
                .entry(id.clone())
                .or_insert((1, Some(item_intent(from))));
        }
    }
    let list = reached
        .into_iter()
        .filter(|(id, _)| !omitted(id))
        .map(|(id, (distance, via))| Reached { id, distance, via })
        .collect();
    (list, findings.into_iter().collect())
}

// ---- Compile ---------------------------------------------------------------

/// The work document a target names: the document itself, or the one an
/// item target sits in. `None` for any other target.
fn target_document(graph: &Graph, seeds: &Seeds) -> Option<String> {
    let [root] = seeds.roots.as_slice() else {
        return None;
    };
    let u = graph.units.get(root)?;
    if u.work {
        Some(u.id.clone())
    } else if u.kind == "item" {
        u.document.clone().filter(|d| graph.units.contains_key(d))
    } else {
        None
    }
}

/// Whether `u` is the work document `doc` or one of its units.
fn of_document(u: &Unit, doc: &str) -> bool {
    u.id == doc || u.document.as_deref() == Some(doc)
}

/// `agent.min`: `text` without the fields named in `omit`, each line and
/// its continuations, from the record's attribute block (the lines after
/// its heading up to the first blank one). A section and an item have no
/// fields to cut.
fn without_fields(u: &Unit, text: &str, omit: &[String]) -> String {
    if omit.is_empty() || matches!(u.kind.as_str(), "section" | "item") {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut lines = text.split_inclusive('\n');
    if let Some(first) = lines.next() {
        out.push_str(first);
    }
    let (mut attrs, mut skipping) = (true, false);
    for raw in lines {
        let line = raw.trim_end_matches(['\n', '\r']);
        if attrs && line.trim().is_empty() {
            attrs = false;
        }
        if attrs {
            if skipping && line.starts_with("  ") {
                continue;
            }
            skipping = false;
            let name = line.split_once(':').map(|(n, _)| n);
            if name.is_some_and(|n| is_field_name(n) && omit.iter().any(|o| o == n)) {
                skipping = true;
                continue;
            }
        }
        out.push_str(raw);
    }
    out
}

/// A field name (section 3.6): a lowercase letter, then lowercase letters,
/// digits and `_`.
fn is_field_name(n: &str) -> bool {
    let mut c = n.chars();
    c.next().is_some_and(|f| f.is_ascii_lowercase())
        && c.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

/// `war context <target>`: the packet, or a refusal with nothing emitted.
pub fn compile(graph: &Graph, target: &str, opts: &Options) -> Result<Packet, Refusal> {
    compile_seeds(graph, target, opts, resolve(graph, target)?)
}

/// Render an explicit required selection through the same compiler as `context`.
/// Callers determine applicability; this function never truncates required text.
pub fn compile_selected(
    graph: &Graph,
    target: &str,
    ids: &BTreeSet<String>,
) -> Result<Packet, Refusal> {
    let mut seeds = resolve(graph, target)?;
    seeds.zero = ids.clone();
    seeds.one.clear();
    seeds.roots = ids.iter().cloned().collect();
    let mut preset = Preset::builtin("agent").expect("built-in agent preset");
    preset.name = "task.required".into();
    preset.walk = "task.required".into();
    preset.budget = None;
    preset.depth = 0;
    compile_seeds(graph, target, &Options::of(preset), seeds)
}

fn compile_seeds(
    graph: &Graph,
    target: &str,
    opts: &Options,
    mut seeds: Seeds,
) -> Result<Packet, Refusal> {
    let preset = &opts.preset;
    let own = target_document(graph, &seeds);
    if preset.resume {
        // The target's record and items (section 10.4).
        let Some(doc) = &own else {
            return Err(Refusal {
                rule: RULE_TARGET_UNKNOWN,
                message: format!(
                    "{target:?}: preset `resume` picks up a Warrant or ticket, or an item of one; this target is neither"
                ),
                needed: None,
            });
        };
        seeds.zero = graph
            .units
            .values()
            .filter(|u| u.id == *doc || (u.kind == "item" && u.document.as_deref() == Some(doc)))
            .map(|u| u.id.clone())
            .collect();
    }
    let (mut reached, mut findings) =
        walk_over(graph, &seeds, opts.depth, &preset.steps, &preset.omit_types);
    // Step 3: distance, type rank, source, line (then id, so ties are fixed).
    // `human` gives the work document's own units first, in document order.
    let key = |r: &Reached| {
        let u = &graph.units[&r.id];
        if preset.own_first
            && let Some(doc) = &own
            && of_document(u, doc)
        {
            return (0, 0, 0, "", u.line, u.id.as_str());
        }
        (
            1,
            r.distance,
            type_rank(&u.kind),
            u.source.as_str(),
            u.line,
            u.id.as_str(),
        )
    };
    reached.sort_by(|a, b| key(a).cmp(&key(b)));
    let shown_target = if graph.units.contains_key(target.trim()) {
        target.trim().to_owned()
    } else {
        normalize_path(target)
    };
    let digest = selection_digest(
        reached
            .iter()
            .map(|r| (r.id.as_str(), graph.units[&r.id].revision.as_str())),
    );
    let tracking = preset
        .reads_state
        .then_some(graph.tracking.as_ref())
        .flatten();
    let mut head = format!(
        "# Context: {shown_target}\n<!-- {SCHEMA} · budget {} · depth {} · preset {} (walk {}) · selection {digest}",
        opts.budget
            .map_or_else(|| "none".to_owned(), |b| b.to_string()),
        opts.depth,
        preset.name,
        preset.walk,
    );
    if let Some(t) = tracking {
        head.push_str(&format!(
            " · journal {} · at {}",
            short_revision(&t.journal_digest),
            t.at
        ));
    }
    head.push_str(" -->\n\n## Index\n");
    let index_at = head.len();
    for r in &reached {
        head.push_str("[F] ");
        head.push_str(&summary_line(&graph.units[&r.id]));
        head.push('\n');
    }
    head.push_str("\n## Records\n");
    let text_of = |u: &Unit| -> Option<String> {
        let mut text = without_fields(u, u.text.as_deref()?, &preset.omit_fields);
        if !text.ends_with('\n') {
            text.push('\n');
        }
        Some(text)
    };
    let block = |u: &Unit| -> Option<String> {
        let text = text_of(u)?;
        let mut b = format!(
            "> {} · {} · {}:{} · {}\n",
            u.id,
            u.kind,
            u.source,
            u.line,
            short_revision(&u.revision)
        );
        if preset.resume
            && let Some(line) = tracking.and_then(|t| t.state_lines.get(&u.id))
        {
            b.push_str(line);
            b.push('\n');
        }
        if preset.editable && u.editable {
            b.push_str(&format!(
                "<!-- rec {} {}:{} {} -->\n{text}<!-- /rec -->\n",
                u.id, u.source, u.line, u.revision
            ));
        } else {
            b.push_str(&text);
        }
        b.push('\n');
        Some(b)
    };
    let base = head.len();
    let roots: BTreeSet<&str> = seeds.roots.iter().map(String::as_str).collect();
    let roots_bytes: usize = reached
        .iter()
        .filter(|r| roots.contains(r.id.as_str()))
        .filter_map(|r| block(&graph.units[&r.id]))
        .map(|b| b.len())
        .sum();
    if let Some(budget) = opts.budget
        && (base > budget || base + roots_bytes > budget)
    {
        let needed = base + roots_bytes;
        return Err(Refusal {
            rule: RULE_OVER_BUDGET,
            message: format!(
                "{shown_target}: the header and index ({base} bytes) and the roots' full text ({roots_bytes} bytes) need {needed} bytes; the budget is {budget}. Raise --budget (or --tokens), or ask for a narrower target"
            ),
            needed: Some(needed),
        });
    }
    // Step 5: whole records in order until the next would not fit.
    let mut body = String::new();
    let mut full: BTreeSet<String> = BTreeSet::new();
    let mut stopped = false;
    let mut all_bytes = base;
    for r in &reached {
        let u = &graph.units[&r.id];
        // An item whose section is already given in full is there, in its
        // line: it is `[F]` and costs nothing more (section 10.3). An item
        // that is the target has its own entry.
        if !roots.contains(r.id.as_str()) && inside_full(graph, u, &full) {
            full.insert(r.id.clone());
            continue;
        }
        let Some(b) = block(u) else { continue };
        all_bytes += b.len();
        if !stopped
            && opts
                .budget
                .is_none_or(|budget| base + body.len() + b.len() <= budget)
        {
            body.push_str(&b);
            full.insert(r.id.clone());
        } else {
            stopped = true;
        }
    }
    if opts.strict && full.len() < reached.len() {
        let summary_only = reached.len() - full.len();
        return Err(Refusal {
            rule: RULE_OVER_BUDGET,
            message: format!(
                "{shown_target}: --strict, and {summary_only} of {} record(s) would be given by summary only; every record in full needs {all_bytes} bytes against a budget of {}{}",
                reached.len(),
                opts.budget
                    .map_or_else(|| "none".to_owned(), |b| b.to_string()),
                if reached.iter().any(|r| graph.units[&r.id].text.is_none()) {
                    " (a record whose span is not known is never given in full)"
                } else {
                    ""
                }
            ),
            needed: Some(all_bytes),
        });
    }
    // Mark the index: `[F]` for what follows in full, `[S]` for the rest.
    let mut markdown = String::with_capacity(base + body.len());
    markdown.push_str(&head[..index_at]);
    for r in &reached {
        markdown.push_str(if full.contains(&r.id) { "[F] " } else { "[S] " });
        markdown.push_str(&summary_line(&graph.units[&r.id]));
        markdown.push('\n');
    }
    markdown.push_str("\n## Records\n");
    debug_assert_eq!(markdown.len(), base);
    markdown.push_str(&body);
    if preset.resume
        && let Some(doc) = &own
    {
        // Up to three of the target's candidates, in the order of 11.5.
        markdown.push_str("## Next\n");
        for id in tracking
            .and_then(|t| t.next.get(doc))
            .into_iter()
            .flatten()
            .take(3)
        {
            let title = graph
                .units
                .get(id)
                .map_or_else(|| "-".to_owned(), shown_title);
            markdown.push_str(&format!("- {id} · {title}\n"));
        }
        markdown.push('\n');
    }
    findings.extend(
        graph
            .diagnostics
            .iter()
            .filter(|f| reached.iter().any(|r| r.id == f.record))
            .cloned(),
    );
    findings.sort();
    findings.dedup();
    let records = reached
        .iter()
        .map(|r| {
            let u = &graph.units[&r.id];
            let is_full = full.contains(&r.id);
            PacketRecord {
                id: u.id.clone(),
                kind: u.kind.clone(),
                title: shown_title(u),
                state: shown_state(u),
                source: u.source.clone(),
                line: u.line,
                revision: u.revision.clone(),
                content_digest: u.content_digest.clone(),
                distance: r.distance,
                via: r.via.clone(),
                tier: if is_full { "full" } else { "summary" }.to_owned(),
                text: if is_full {
                    u.text
                        .as_deref()
                        .map(|t| without_fields(u, t, &preset.omit_fields))
                } else {
                    None
                },
            }
        })
        .collect();
    Ok(Packet {
        schema: SCHEMA.to_owned(),
        target: shown_target,
        budget: opts.budget,
        bytes: markdown.len(),
        depth: opts.depth,
        preset: preset.name.clone(),
        walk: preset.walk.clone(),
        selection_digest: digest,
        journal: tracking.map(|t| t.journal_digest.clone()),
        at: tracking.map(|t| t.at.clone()),
        records,
        diagnostics: findings,
        markdown,
    })
}

/// Whether `u` is an item whose section is already given in full and whose
/// span lies inside that section's, so its text is already in the packet.
fn inside_full(graph: &Graph, u: &Unit, full: &BTreeSet<String>) -> bool {
    u.kind == "item"
        && u.text.is_some()
        && u.section.as_ref().is_some_and(|s| {
            full.contains(s)
                && graph.units.get(s).is_some_and(|sec| {
                    sec.source == u.source && sec.span[0] <= u.span[0] && u.span[1] <= sec.span[1]
                })
        })
}

// ---- Index -----------------------------------------------------------------

/// The index (section 10.2), `docs/generated/llms.txt`: each work document
/// by its own record, every other document by each of its records.
#[must_use]
pub fn index(graph: &Graph) -> String {
    let mut out = index_header(graph);
    for block in index_blocks(graph).values() {
        out.push_str(block);
    }
    out
}

fn index_header(graph: &Graph) -> String {
    format!(
        "# {}\n\n> OpenWarrant index ({FORMAT}). Generated; do not edit. Read any line's record with `war context <id>`.\n",
        graph.name
    )
}

/// Each source's block of the index, keyed by the source, in the index's
/// order: `\n## <source>\n> <summary>\n` and one line per record.
fn index_blocks(graph: &Graph) -> BTreeMap<String, String> {
    // A work document is one line: its own record. Every unit inside it is
    // one `war context <alias>` away, so listing them would only repeat it.
    let mut by_source: BTreeMap<&str, Vec<&Unit>> = BTreeMap::new();
    for u in graph.units.values().filter(|u| u.document.is_none()) {
        by_source.entry(u.source.as_str()).or_default().push(u);
    }
    let mut blocks = BTreeMap::new();
    for (source, mut units) in by_source {
        units.sort_by(|a, b| (a.line, &a.id).cmp(&(b.line, &b.id)));
        let summary = units
            .iter()
            .find(|u| u.work)
            .map(|u| {
                u.summary
                    .as_deref()
                    .or(u.title.as_deref())
                    .map_or_else(|| "-".to_owned(), one_line)
            })
            .or_else(|| {
                // A non-work document's own record (`doc:<path>`) carries
                // the document's `summary`.
                units
                    .iter()
                    .find(|u| u.id.starts_with("doc:"))
                    .and_then(|u| u.summary.as_deref())
                    .map(one_line)
            })
            .unwrap_or_else(|| "-".to_owned());
        let mut out = format!("\n## {source}\n> {summary}\n");
        for u in units {
            out.push_str("- ");
            out.push_str(&summary_line(u));
            out.push('\n');
        }
        blocks.insert(source.to_owned(), out);
    }
    blocks
}

/// An index file split back into its header and its blocks by source; `None`
/// unless the pieces reassemble to exactly `text`, sources ascending.
fn split_index(text: &str) -> Option<(String, BTreeMap<String, String>)> {
    let mut starts: Vec<usize> = text.match_indices("\n## ").map(|(i, _)| i).collect();
    starts.push(text.len());
    let header = text.get(..starts[0])?.to_owned();
    let mut blocks = BTreeMap::new();
    let mut last: Option<String> = None;
    for w in starts.windows(2) {
        let block = &text[w[0]..w[1]];
        let source = block.get(4..)?.split('\n').next()?.to_owned();
        if last.as_ref().is_some_and(|l| *l >= source) {
            return None;
        }
        last = Some(source.clone());
        blocks.insert(source, block.to_owned());
    }
    let again: String = std::iter::once(header.as_str())
        .chain(blocks.values().map(String::as_str))
        .collect();
    (again == text).then_some((header, blocks))
}

// ---- The corpus side ------------------------------------------------------

/// The graph of a repository: its model, through the adapter.
pub fn graph_of(corpus: &crate::corpus::Corpus) -> Result<Graph, crate::repo::RepoError> {
    let model = crate::model::build(corpus)?;
    Ok(adapter::graph(corpus, &model))
}

/// The index file `war admin compile` writes and `war check --generated`
/// compares: its path and its bytes.
pub fn index_file(
    corpus: &crate::corpus::Corpus,
) -> Result<(camino::Utf8PathBuf, String), crate::repo::RepoError> {
    Ok((
        corpus.repo().root.join(INDEX_PATH),
        index(&graph_of(corpus)?),
    ))
}

/// Rewrite the tickets' blocks of the index, and with `record_atoms` the
/// record atoms' too, when the repository keeps one and they moved: a
/// ticket act that lists, retitles or changes the state of a ticket, or
/// writes record atoms (`war create --records`), moves exactly those
/// blocks, and ticket work must not leave `war check --generated` reporting
/// drift (t-ca38). Only those blocks are rebuilt, from
/// [`crate::model::build_tickets`], so a ticket verb never pays for the
/// Warrants' passes; each is the block a full build writes. An index
/// that does not split cleanly into blocks is refused, not rebuilt. A
/// repository that never compiled an index gets none. Whether it was written.
pub fn refresh_index(
    repo: &crate::repo::Repository,
    record_atoms: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    let path = repo.root.join(INDEX_PATH);
    if !path.is_file() {
        return Ok(false);
    }
    let current = crate::vfs::read_to_string(&path)?;
    let Some((header, mut blocks)) = split_index(&current) else {
        return Err(format!("{INDEX_PATH} does not split into its blocks").into());
    };
    let corpus = crate::corpus::Corpus::new(repo);
    let model = crate::model::build_tickets(&corpus, record_atoms)?;
    blocks.extend(index_blocks(&adapter::graph(&corpus, &model)));
    let contents: String = std::iter::once(header)
        .chain(blocks.into_values())
        .collect();
    let before = crate::compile::atomic::prestate(&path)?;
    if before == crate::compile::atomic::Prestate::of(contents.as_bytes()) {
        return Ok(false);
    }
    crate::compile::atomic::write_if(&path, &contents, &before)?;
    Ok(true)
}

/// A target as the compiler reads it: an absolute path inside the
/// repository becomes repository-relative, so the packet never carries a
/// machine's path; anything else is unchanged.
#[must_use]
pub fn relative_target(repo: &crate::repo::Repository, target: &str) -> String {
    let p = camino::Utf8Path::new(target);
    if p.is_absolute()
        && let Ok(rel) = p.strip_prefix(&repo.root)
    {
        return rel.as_str().to_owned();
    }
    target.to_owned()
}

/// The report a packet or refusal answers with under `--json` (and over
/// MCP): the refusal as an error by its rule, the adapter's and walk's
/// findings as warnings.
#[must_use]
pub fn report_of(result: &Result<Packet, Refusal>) -> crate::diagnostic::Report {
    use crate::diagnostic::Diagnostic;
    let mut report = crate::diagnostic::Report::default();
    match result {
        Ok(p) => {
            for f in &p.diagnostics {
                report.push(Diagnostic::warn(
                    f.rule.clone(),
                    f.record.clone(),
                    f.message.clone(),
                ));
            }
            let full = p.records.iter().filter(|r| r.tier == "full").count();
            report.push(Diagnostic::pass(
                "context.emitted",
                format!(
                    "{}: {} record(s), {full} in full, {} bytes of {}",
                    p.target,
                    p.records.len(),
                    p.bytes,
                    p.budget
                        .map_or_else(|| "no budget".to_owned(), |b| b.to_string())
                ),
            ));
        }
        Err(r) => report.push(Diagnostic::error(r.rule, String::new(), r.message.clone())),
    }
    report
}

/// `war context`: the Markdown on stdout (exactly the packet's bytes), or
/// under `--json` the `oh.war/report/v1` envelope whose `result` is the
/// `oh.war/context/v1` packet. A refusal prints nothing on stdout in
/// Markdown mode, names its rule on stderr, and exits 2.
///
/// A preset that reads state reads the model with its journals and claims,
/// at `at` (`(RFC 3339 text, Unix seconds)`; the caller has set the time
/// claims are judged at), and the files of [`adapter::tracking`].
pub fn answer(
    mode: crate::output::Mode,
    corpus: &crate::corpus::Corpus,
    target: &str,
    opts: &Options,
    at: Option<(String, u64)>,
) -> Result<u8, crate::repo::RepoError> {
    use std::io::Write as _;
    let graph = match (&at, opts.preset.reads_state) {
        (Some((text, secs)), true) => {
            let model = crate::model::build(corpus)?;
            let mut g = adapter::graph_reading_state(corpus, &model);
            g.tracking = Some(adapter::tracking(corpus, &g, text, *secs));
            g
        }
        _ => graph_of(corpus)?,
    };
    let result = compile(&graph, target, opts);
    match mode {
        crate::output::Mode::Human => match result {
            Ok(p) => {
                let mut out = std::io::stdout().lock();
                let _ = out.write_all(p.markdown.as_bytes());
                let _ = out.flush();
                Ok(crate::EXIT_OK)
            }
            Err(r) => {
                eprintln!("{r}");
                Ok(crate::EXIT_NOT_READY)
            }
        },
        crate::output::Mode::Json => {
            let report = report_of(&result);
            let value = result.ok().map(|p| crate::output::value(&p));
            Ok(crate::output::finish(mode, "context", &report, value))
        }
    }
}

/// `war context --index`: the index of section 10.2 on stdout, exactly the
/// bytes `war admin compile` writes to [`INDEX_PATH`]; under `--json` the
/// envelope whose `result` is `{path, text}`.
pub fn answer_index(
    mode: crate::output::Mode,
    corpus: &crate::corpus::Corpus,
) -> Result<u8, crate::repo::RepoError> {
    use std::io::Write as _;
    let text = index(&graph_of(corpus)?);
    match mode {
        crate::output::Mode::Human => {
            let mut out = std::io::stdout().lock();
            let _ = out.write_all(text.as_bytes());
            let _ = out.flush();
            Ok(crate::EXIT_OK)
        }
        crate::output::Mode::Json => {
            let mut report = crate::diagnostic::Report::default();
            report.push(crate::diagnostic::Diagnostic::pass(
                "context.index",
                format!("{INDEX_PATH}: {} bytes", text.len()),
            ));
            let value = serde_json::json!({"path": INDEX_PATH, "text": text});
            Ok(crate::output::finish(mode, "context", &report, Some(value)))
        }
    }
}

#[cfg(test)]
mod tests;
