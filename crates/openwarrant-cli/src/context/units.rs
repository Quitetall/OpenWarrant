// SPDX-License-Identifier: Apache-2.0
//! What `war context` answers when a packet of records cannot: a long
//! document, one of its units, a topic, a code path, or a target too wide
//! for its budget. A refusal leaves an agent reading whole files, the worst
//! outcome for its tokens; this returns the units that fit, ranked, with
//! every cut declared (section 12.2: nothing is silently omitted).
//!
//! This is `oh.war/unit-context/v1`, beside the record packet of section 10
//! and not a change to it: the rule that an explicit budget the roots do not
//! fit is refused stands, and [`degrade`] leaves that refusal alone.
//!
//! Deterministic, local and model-free. A unit is given whole or not at all.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::{Graph, Options, RULE_OVER_BUDGET, RULE_TARGET_UNKNOWN, Refusal, normalize_path};
use crate::corpus::Corpus;
use openwarrant_core::instruction::section_id;
use openwarrant_core::units;

pub const SCHEMA: &str = "oh.war/unit-context/v1";

/// The share of the budget the list of units may take.
const INDEX_SHARE: usize = 4;
/// Topic and path answers list at most this many units.
const MOST_RANKED: usize = 40;

/// One unit, as `--json` gives it.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnitEntry {
    pub id: String,
    pub title: String,
    pub source: String,
    pub line: usize,
    /// The bytes of its text.
    pub bytes: usize,
    pub revision: String,
    /// `full` or `summary`.
    pub tier: String,
    /// Its text, for `full`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// The packet.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnitPacket {
    pub schema: String,
    pub target: String,
    /// `document`, `unit`, `topic`, `path` or `records`.
    pub kind: String,
    pub budget: Option<usize>,
    /// Every unit listed, best first.
    pub units: Vec<UnitEntry>,
    /// What was left out and how to get it. Never empty when anything was.
    pub cut: Vec<String>,
    /// The whole size of the files these units come from.
    pub source_bytes: usize,
    /// The size of what this packet returned.
    pub returned_bytes: usize,
    /// `markdown`, as the terminal shows it.
    #[serde(skip)]
    pub markdown: String,
}

/// A unit that could be returned.
#[derive(Debug, Clone)]
struct Candidate {
    id: String,
    title: String,
    source: String,
    line: usize,
    /// `[start, end)` in `source`, so overlapping units are never both given.
    span: [usize; 2],
    text: String,
}

impl Candidate {
    fn overlaps(&self, other: &Self) -> bool {
        self.source == other.source && self.span[0] < other.span[1] && other.span[0] < self.span[1]
    }
}

/// The units of the file `rel`, or `None` when it cannot be read.
fn file_units(corpus: &Corpus, rel: &str) -> Option<Vec<(units::Unit, Candidate)>> {
    let text = crate::vfs::read_to_string(corpus.repo().root.join(rel)).ok()?;
    Some(
        units::scan(&text)
            .into_iter()
            .map(|u| {
                let c = Candidate {
                    id: section_id(rel, &u.id),
                    title: u.title.clone(),
                    source: rel.to_owned(),
                    line: u.line,
                    span: [u.start, u.end],
                    text: text[u.start..u.end].to_owned(),
                };
                (u, c)
            })
            .collect(),
    )
}

/// A document in order: each `##`, or its `###`s when it alone would take
/// the budget.
fn document(found: Vec<(units::Unit, Candidate)>, budget: Option<usize>) -> Vec<Candidate> {
    let too_big: BTreeSet<String> = found
        .iter()
        .filter(|(u, c)| {
            u.level == 2
                && budget.is_some_and(|b| c.text.len() > b)
                && found
                    .iter()
                    .any(|(n, _)| n.parent.as_deref() == Some(&u.id))
        })
        .map(|(u, _)| u.id.clone())
        .collect();
    found
        .into_iter()
        .filter(|(u, _)| match u.level {
            3 => u.parent.as_ref().is_some_and(|p| too_big.contains(p)),
            _ => true,
        })
        .map(|(_, c)| c)
        .collect()
}

/// The unit `slug` of `found`: alone when it fits, else its `###`s after it.
fn one_unit(
    found: Vec<(units::Unit, Candidate)>,
    slug: &str,
    budget: Option<usize>,
) -> Option<Vec<Candidate>> {
    let at = found.iter().find(|(u, _)| u.id == slug)?;
    let fits = budget.is_none_or(|b| at.1.text.len() <= b);
    let top = at.1.clone();
    if fits {
        return Some(vec![top]);
    }
    let mut out: Vec<Candidate> = found
        .into_iter()
        .filter(|(u, _)| u.parent.as_deref() == Some(slug))
        .map(|(_, c)| c)
        .collect();
    out.insert(0, top);
    Some(out)
}

/// The lowercase words of `text`.
fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
}

/// The words of a query that carry meaning.
fn query_terms(query: &str) -> Vec<String> {
    const STOP: [&str; 24] = [
        "a", "an", "and", "are", "as", "at", "be", "by", "do", "for", "from", "how", "in", "is",
        "it", "of", "on", "or", "our", "the", "this", "to", "what", "with",
    ];
    let mut seen = BTreeSet::new();
    words(query)
        .filter(|w| !STOP.contains(&w.as_str()) && seen.insert(w.clone()))
        .collect()
}

/// `pool` ranked for `query`, best first, dropping any unit that misses more
/// than half its terms. Titles count triple and the whole phrase counts
/// again; `damped` units (closed work) count for less. Ties go by id.
fn rank(pool: Vec<Candidate>, query: &str, damped: &BTreeSet<String>) -> Vec<Candidate> {
    let terms = query_terms(query);
    if terms.is_empty() || pool.is_empty() {
        return Vec::new();
    }
    let phrase = query.trim().to_lowercase();
    let lengths: Vec<usize> = pool.iter().map(|c| c.text.len().max(1)).collect();
    let average = lengths.iter().sum::<usize>() as f64 / lengths.len() as f64;
    let counts: Vec<BTreeMap<&str, usize>> = {
        let wanted: BTreeSet<&str> = terms.iter().map(String::as_str).collect();
        pool.iter()
            .map(|c| {
                let mut of = BTreeMap::new();
                for w in words(&c.text) {
                    if let Some(t) = wanted.get(w.as_str()) {
                        *of.entry(*t).or_insert(0) += 1;
                    }
                }
                of
            })
            .collect()
    };
    let n = pool.len() as f64;
    let weight: BTreeMap<&str, f64> = terms
        .iter()
        .map(|t| {
            let df = counts.iter().filter(|c| c.contains_key(t.as_str())).count() as f64;
            (t.as_str(), (1.0 + n / (df + 1.0)).ln())
        })
        .collect();
    let needed = terms.len().div_ceil(2);
    let mut scored: Vec<(f64, Candidate)> = pool
        .into_iter()
        .zip(counts.iter().zip(lengths))
        .filter_map(|(c, (count, length))| {
            let title: BTreeSet<String> = words(&c.title).collect();
            let hits = terms
                .iter()
                .filter(|t| count.contains_key(t.as_str()) || title.contains(*t))
                .count();
            if hits < needed {
                return None;
            }
            let norm = 0.25 + 0.75 * length as f64 / average;
            let mut score: f64 = terms
                .iter()
                .map(|t| {
                    let tf = count.get(t.as_str()).copied().unwrap_or(0) as f64;
                    let body = tf / (tf + 1.2 * norm);
                    let boost = if title.contains(t) { 3.0 } else { 0.0 };
                    weight[t.as_str()] * (body + boost)
                })
                .sum();
            score *= hits as f64 / terms.len() as f64;
            if terms.len() > 1 {
                if c.title.to_lowercase().contains(&phrase) {
                    score *= 3.0;
                } else if c.text.to_lowercase().contains(&phrase) {
                    score *= 1.5;
                }
            }
            if damped.contains(&c.source) {
                score *= 0.5;
            }
            Some((score, c))
        })
        .collect();
    scored.sort_by(|(a, x), (b, y)| b.total_cmp(a).then_with(|| x.id.cmp(&y.id)));
    scored.into_iter().map(|(_, c)| c).collect()
}

/// A work document's state that means the work is over.
fn closed(state: Option<&str>) -> bool {
    state.is_some_and(|s| {
        matches!(
            s,
            "done" | "closed" | "resolved" | "cancelled" | "canceled" | "superseded" | "rejected"
        )
    })
}

/// Every unit a query may reach: the indexed documents' units, then the
/// sections of the work documents the walk leaves out, and the sources of
/// closed work.
fn search_pool(corpus: &Corpus, graph: &Graph) -> (Vec<Candidate>, BTreeSet<String>) {
    let repo = corpus.repo();
    let (globs, _) = crate::doc_index::globs(repo);
    let walked = crate::doc_index::walk(repo, &globs);
    let mut pool = Vec::new();
    for rel in walked.iter().filter(|p| p.ends_with(".md")) {
        if let Some(found) = file_units(corpus, rel) {
            pool.extend(found.into_iter().map(|(_, c)| c));
        }
    }
    let mut done = BTreeSet::new();
    for u in graph.units.values() {
        if u.work && closed(u.state.as_deref()) {
            done.insert(u.source.clone());
        }
        if walked.contains(&u.source) || u.kind == "item" || u.work {
            continue;
        }
        let Some(text) = &u.text else { continue };
        pool.push(Candidate {
            id: u.id.clone(),
            title: u.title.clone().unwrap_or_else(|| u.id.clone()),
            source: u.source.clone(),
            line: u.line,
            span: u.span,
            text: text.clone(),
        });
    }
    (pool, done)
}

/// The records a wide target reached: open work before closed, its own
/// records before the units inside them, then by id, newest first.
fn wide(graph: &Graph, target: &str) -> Vec<Candidate> {
    let Ok(seeds) = super::resolve(graph, target) else {
        return Vec::new();
    };
    let mut ids: Vec<(u8, &str)> = seeds
        .zero
        .iter()
        .chain(seeds.roots.iter())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter_map(|id| {
            let u = graph.units.get(id)?;
            let owner = u.document.as_ref().and_then(|d| graph.units.get(d));
            let over = closed(owner.unwrap_or(u).state.as_deref());
            Some((
                u8::from(over) * 2 + u8::from(!seeds.roots.contains(id)),
                id.as_str(),
            ))
        })
        .collect();
    ids.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(a.1)));
    ids.into_iter()
        .filter_map(|(_, id)| {
            let u = &graph.units[id];
            Some(Candidate {
                id: u.id.clone(),
                title: u.title.clone().unwrap_or_else(|| u.id.clone()),
                source: u.source.clone(),
                line: u.line,
                span: u.span,
                text: u.text.clone()?,
            })
        })
        .collect()
}

/// Whether `target` reads as an id a user typed, not words or a path: a
/// failed lookup of one stays a refusal.
fn id_shaped(target: &str) -> bool {
    let t = target.trim();
    if t.starts_with("war://") || t.chars().any(char::is_whitespace) {
        return t.starts_with("war://");
    }
    let dashed = t.contains('-') || t.contains('/');
    dashed && t.chars().any(|c| c.is_ascii_digit()) && !t.contains('.')
}

/// Render `candidates` within `budget`: the list of units, then whole texts
/// in order, each only if it fits and overlaps nothing already given.
fn render(
    corpus: &Corpus,
    target: &str,
    kind: &str,
    candidates: Vec<Candidate>,
    budget: Option<usize>,
    notes: Vec<String>,
) -> UnitPacket {
    let listed = budget.map_or(candidates.len(), |b| {
        let mut room = b / INDEX_SHARE;
        candidates
            .iter()
            .take_while(|c| {
                let line = c.id.len() + c.title.len() + c.source.len() + 24;
                let fits = line <= room;
                room = room.saturating_sub(line);
                fits
            })
            .count()
            .max(1)
    });
    let unlisted = candidates.len().saturating_sub(listed);
    let mut sources: BTreeSet<&str> = BTreeSet::new();
    for c in &candidates {
        sources.insert(&c.source);
    }
    let source_bytes = sources
        .iter()
        .filter_map(|s| crate::vfs::metadata(corpus.repo().root.join(s)).ok())
        .map(|m| usize::try_from(m.len()).unwrap_or(usize::MAX))
        .sum();
    let head_room = 240 + target.len();
    let list_bytes: usize = candidates[..listed]
        .iter()
        .map(|c| c.id.len() + c.title.len() + c.source.len() + 24)
        .sum();
    let mut left = budget.map(|b| b.saturating_sub(head_room + list_bytes + 400));
    let mut given: Vec<usize> = Vec::new();
    for (i, c) in candidates.iter().enumerate().take(listed) {
        let block = c.text.len() + c.id.len() + c.source.len() + 32;
        if left.is_some_and(|l| block > l) {
            continue;
        }
        if given.iter().any(|g| candidates[*g].overlaps(c)) {
            continue;
        }
        if let Some(l) = &mut left {
            *l -= block;
        }
        given.push(i);
    }
    let units: Vec<UnitEntry> = candidates[..listed]
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let full = given.contains(&i);
            UnitEntry {
                id: c.id.clone(),
                title: c.title.clone(),
                source: c.source.clone(),
                line: c.line,
                bytes: c.text.len(),
                revision: openwarrant_core::instruction::digest(c.text.as_bytes()),
                tier: if full { "full" } else { "summary" }.to_owned(),
                text: full.then(|| c.text.clone()),
            }
        })
        .collect();
    let summary_only = listed - given.len();
    let mut cut = notes;
    if summary_only > 0 {
        cut.push(format!(
            "{summary_only} unit(s) by title only: `war context '<id>'` gives one in full"
        ));
    }
    if unlisted > 0 {
        cut.push(format!(
            "{unlisted} more unit(s) not listed: `--full` gives everything"
        ));
    }
    let mut body = format!(
        "# Context: {target}\n<!-- {SCHEMA} · {kind} · budget {} · {} of {} unit(s) in full -->\n\n## Units\n",
        budget.map_or_else(|| "none".to_owned(), |b| b.to_string()),
        given.len(),
        candidates.len(),
    );
    for u in &units {
        let mark = if u.text.is_some() { "[F]" } else { "[S]" };
        body.push_str(&format!(
            "{mark} {} · {} · {}:{} · {} bytes\n",
            u.id, u.title, u.source, u.line, u.bytes
        ));
    }
    if !cut.is_empty() {
        body.push_str("\n## Cut\n");
        for c in &cut {
            body.push_str(&format!("- {c}\n"));
        }
    }
    if !given.is_empty() {
        body.push_str("\n## Text\n");
    }
    for u in units.iter().filter(|u| u.text.is_some()) {
        let text = u.text.as_deref().unwrap_or_default();
        body.push_str(&format!("> {} · {}:{}\n{text}", u.id, u.source, u.line));
        if !text.ends_with('\n') {
            body.push('\n');
        }
        body.push('\n');
    }
    let shown = body.len();
    body.push_str(&format!(
        "<!-- returned {shown} bytes of {source_bytes} in its sources -->\n"
    ));
    UnitPacket {
        schema: SCHEMA.to_owned(),
        target: target.to_owned(),
        kind: kind.to_owned(),
        budget,
        returned_bytes: shown,
        source_bytes,
        units,
        cut,
        markdown: body,
    }
}

/// What to answer when [`super::compile`] refused `refused`: the units that
/// fit, or the refusal. A refusal stands for `--strict`, an explicit budget
/// the roots do not fit, a preset that resumes work, an id that names
/// nothing, and a query nothing matches.
///
/// # Errors
/// The refusal, when no units can answer.
pub fn degrade(
    corpus: &Corpus,
    graph: &Graph,
    target: &str,
    opts: &Options,
    refused: Refusal,
) -> Result<UnitPacket, Refusal> {
    let over = refused.rule == RULE_OVER_BUDGET;
    let unknown = refused.rule == RULE_TARGET_UNKNOWN;
    if opts.preset.resume || !(over || unknown) {
        return Err(refused);
    }
    if over && opts.explicit_budget {
        return Err(refused);
    }
    let budget = opts.budget;
    let root = &corpus.repo().root;
    let mut notes = Vec::new();
    let plain = target.trim().strip_prefix("md:").unwrap_or(target.trim());
    let (path, slug) = match plain.split_once('#') {
        Some((p, s)) => (normalize_path(p), Some(s)),
        None => (normalize_path(plain), None),
    };
    let markdown = path.ends_with(".md") && crate::vfs::is_file(root.join(&path));
    let (kind, candidates) = if markdown {
        let Some(found) = file_units(corpus, &path) else {
            return Err(refused);
        };
        match slug {
            Some(s) => {
                let known: Vec<String> = found.iter().map(|(u, _)| u.id.clone()).collect();
                let pool: Vec<Candidate> = found.iter().map(|(_, c)| c.clone()).collect();
                if let Some(c) = one_unit(found, s, budget) {
                    ("unit", c)
                } else {
                    let words = s.replace(['-', '_'], " ");
                    let mut near = rank(pool, &words, &BTreeSet::new());
                    // A title that says it is the answer; the rest only mention it.
                    if near.iter().any(|c| c.title.to_lowercase().contains(&words)) {
                        near.retain(|c| c.title.to_lowercase().contains(&words));
                    }
                    if near.is_empty() {
                        return Err(Refusal {
                            rule: RULE_TARGET_UNKNOWN,
                            message: format!(
                                "{path} has no unit `{s}`; its units: {}",
                                known
                                    .iter()
                                    .take(12)
                                    .cloned()
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                            needed: None,
                        });
                    }
                    notes.push(format!("{path} has no unit `{s}`: these match its words"));
                    ("unit", near)
                }
            }
            None => ("document", document(found, budget)),
        }
    } else if over {
        ("records", wide(graph, target))
    } else if id_shaped(target) {
        return Err(refused);
    } else {
        let is_path = target.contains('/')
            || crate::vfs::exists(root.join(&path))
            || path.rsplit_once('.').is_some_and(|(_, e)| e.len() <= 5);
        let (pool, closed_sources) = search_pool(corpus, graph);
        let mut ranked = if is_path {
            let mut found = rank_path(pool.clone(), graph, &path);
            let mut parent = path.as_str();
            while found.is_empty() {
                let Some((up, _)) = parent.trim_end_matches('/').rsplit_once('/') else {
                    break;
                };
                parent = up;
                // A top-level directory says nothing about one file.
                if !parent.contains('/') {
                    break;
                }
                found = rank_path(pool.clone(), graph, parent);
                if !found.is_empty() {
                    notes.push(format!("nothing names {path}: these name {parent}/"));
                }
            }
            found
        } else {
            rank(pool, target, &closed_sources)
        };
        ranked.truncate(MOST_RANKED);
        (if is_path { "path" } else { "topic" }, ranked)
    };
    if candidates.is_empty() {
        return Err(refused);
    }
    let packet = render(corpus, target.trim(), kind, candidates, budget, notes);
    if opts.strict && !packet.cut.is_empty() {
        return Err(Refusal {
            rule: RULE_OVER_BUDGET,
            message: format!("{}: --strict, and {}", packet.target, packet.cut.join("; ")),
            needed: Some(packet.units.iter().map(|u| u.bytes).sum()),
        });
    }
    Ok(packet)
}

/// The units that name `path` or the work that governs it, closest first: a
/// governing record's own units, the full path, then the file's name.
fn rank_path(pool: Vec<Candidate>, graph: &Graph, path: &str) -> Vec<Candidate> {
    let dir = path.trim_end_matches('/');
    // The last two segments: `mod.rs` alone would name half a Rust tree.
    let name = dir.rsplitn(3, '/').take(2).collect::<Vec<_>>();
    let name = name.into_iter().rev().collect::<Vec<_>>().join("/");
    let governed: BTreeSet<&str> = graph
        .units
        .values()
        .filter(|u| {
            u.paths
                .iter()
                .any(|p| p.trim_end_matches("/**") == dir || p.starts_with(&format!("{dir}/")))
        })
        .map(|u| u.source.as_str())
        .collect();
    let mut scored: Vec<(usize, Candidate)> = pool
        .into_iter()
        .filter_map(|c| {
            let full = c.text.matches(dir).count();
            let short = c.text.matches(&name).count();
            let held = governed.contains(c.source.as_str());
            let score = 1000 * usize::from(held) + 100 * full + short;
            (score > 0 && (held || full > 0 || short > 0)).then_some((score, c))
        })
        .collect();
    scored.sort_by(|(a, x), (b, y)| b.cmp(a).then_with(|| x.id.cmp(&y.id)));
    scored.into_iter().map(|(_, c)| c).collect()
}

/// The report for a packet: `context.emitted`, its cuts and its savings.
#[must_use]
pub fn report_of(p: &UnitPacket) -> crate::diagnostic::Report {
    use crate::diagnostic::Diagnostic;
    let mut report = crate::diagnostic::Report::default();
    let full = p.units.iter().filter(|u| u.text.is_some()).count();
    report.push(Diagnostic::pass(
        "context.emitted",
        format!(
            "{}: {} of {} unit(s) in full, {} bytes returned of {} in its sources",
            p.target,
            full,
            p.units.len(),
            p.returned_bytes,
            p.source_bytes,
        ),
    ));
    report
}
