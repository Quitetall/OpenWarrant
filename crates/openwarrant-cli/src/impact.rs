// SPDX-License-Identifier: Apache-2.0
//! `war impact <record>` — what a change to one record affects
//! (OW-WAR-0148 M3; OW-ADR-0031), as `oh.war/impact/v1`.
//!
//! Read from the compiled model (`oh.war/model/v1`) and nothing else but
//! the documents' text, so it says nothing a builder does not derive:
//!
//! - **affected** — every record that reaches the subject through incoming
//!   relations, transitively, each with the edge it was reached by and its
//!   depth. A namespaced relation (`x.mentions`) is carried and inert: it is
//!   listed under `inert` and never walked.
//! - **documents** — the Warrants, tickets, record atoms and instruction
//!   files that hold the subject or an affected record, and the Warrants and
//!   tickets whose text names one of them by id (a Warrant's basis naming
//!   `REQ-pr1`, or citing `md:CLAUDE.md#testing`, M16).
//! - **evaluations** — each obligation that `evaluates` the subject or an
//!   affected record, with its verdict as recorded, the revision its
//!   relation pins and the target's revision now. A verdict bound to another
//!   revision stays recorded and reads `stale`; one bound to none reads
//!   `unbound` (nobody can say which bytes it judged). Nothing is cleared.
//! - **phases** — the roadmap phases the affected Warrants are placed in,
//!   whose progress is recomputed from them, with their achievement now.
//! - **projections** — the generated views known to include an affected
//!   Warrant or phase: the Warrant's own `generated/`, the corpus-wide
//!   status and overview, the roadmap view; and (M6) exactly the declared
//!   document projections that select the subject (its bytes reach them),
//!   each with the affected records it also selects.
//!
//! An id that is not a record of the model is refused by name
//! (`impact.unknown-record`). Nothing is written.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::corpus::Corpus;
use crate::diagnostic::{Diagnostic, Report};
use crate::model::{Model, Relation};
use crate::repo::RepoError;

pub const SCHEMA: &str = "oh.war/impact/v1";

/// What a change to one record affects.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Impact {
    /// `oh.war/impact/v1`.
    pub schema: String,
    /// The model's `basis_digest` this was read from.
    pub basis_digest: String,
    pub record: Subject,
    /// Sorted by (depth, id).
    pub affected: Vec<Affected>,
    /// Namespaced relations into the subject or an affected record: carried,
    /// never walked. Sorted.
    pub inert: Vec<Relation>,
    /// Sorted by id.
    pub documents: Vec<Document>,
    /// Sorted by (obligation, target).
    pub evaluations: Vec<Evaluation>,
    /// Sorted by id.
    pub phases: Vec<Phase>,
    /// Sorted by path.
    pub projections: Vec<Projection>,
}

/// The record asked about.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub source: String,
    pub revision: String,
}

/// A record reached through incoming relations.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Affected {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub source: String,
    pub revision: String,
    /// The edge it was first reached by: `via.to` is the subject or an
    /// affected record nearer to it.
    pub via: Relation,
    /// 1 for a direct relation to the subject.
    pub depth: u32,
}

/// A document that holds or names the subject or an affected record.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    /// A Warrant's alias, a ticket's id, or a record atom's path.
    pub id: String,
    /// `warrant`, `ticket` or `records`.
    pub kind: String,
    /// Why: `holds <id>`, `declares <id>`, `names <id> in <file>`.
    pub because: Vec<String>,
}

/// An obligation's verdict on an affected record.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evaluation {
    /// `<alias>/OBL-…`.
    pub obligation: String,
    /// The record it evaluates.
    pub target: String,
    /// The disposition as the corpus reads it now, if it has one. A verdict
    /// whose reviewed subject no longer matches reads `unknown` here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<String>,
    /// The disposition the stored verification record holds, as written,
    /// whether or not it still counts for current work.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recorded: Option<String>,
    /// The revision the `evaluates` relation pins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_revision: Option<String>,
    /// The target's revision now.
    pub current_revision: String,
    /// `current` (bound to this revision), `stale` (bound to another) or
    /// `unbound` (pins none).
    pub reads: String,
}

/// A roadmap phase whose progress an affected Warrant feeds.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Phase {
    pub id: String,
    /// The affected Warrants placed in it.
    pub via: Vec<String>,
    /// Its achievement as the corpus derives it now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub achieved: Option<String>,
}

/// A generated view known to include an affected Warrant or phase.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    /// Repository-relative.
    pub path: String,
    /// `warrant` (that Warrant's own view), `corpus` (every Warrant's),
    /// `roadmap` (every phase's) or `declared` (a document's projection that
    /// selects the subject, OW-WAR-0148 M6).
    pub scope: String,
    /// The affected records it includes.
    pub selects: Vec<String>,
}

/// Every word of `text` that could be an id: runs of letters, digits, `-`
/// and `_`.
fn tokens(text: &str) -> BTreeSet<&str> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .filter(|t| !t.is_empty())
        .collect()
}

/// The ids of `named` that `text` names: a record id as a word, an
/// instruction section (`md:CLAUDE.md#testing`, M16) as a citation.
fn mentioned<'a>(text: &str, named: &BTreeSet<&'a str>) -> Vec<&'a str> {
    let words = tokens(text);
    let cited = openwarrant_core::instruction::citations(text);
    named
        .iter()
        .copied()
        .filter(|i| {
            if i.starts_with(openwarrant_core::instruction::ID_PREFIX) {
                cited.iter().any(|c| c == i)
            } else {
                words.contains(i)
            }
        })
        .collect()
}

/// The impact of a change to `id`, or `None` when `id` is no record of the
/// model (and the report says so by name).
pub fn build(corpus: &Corpus, id: &str) -> Result<(Report, Option<Impact>), RepoError> {
    let model = crate::model::build(corpus)?;
    let mut report = Report::default();
    let Some(subject) = model.records.iter().find(|r| r.id == id) else {
        report.push(Diagnostic::error(
            "impact.unknown-record",
            id.to_owned(),
            format!(
                "{id} is not a record of this corpus ({} records); `war model --json` lists \
                 every record id",
                model.records.len()
            ),
        ));
        return Ok((report, None));
    };
    let impact = walk(corpus, &model, subject);
    for e in impact.evaluations.iter().filter(|e| e.reads == "stale") {
        report.push(Diagnostic::warn(
            "impact.evaluation-stale",
            e.obligation.clone(),
            format!(
                "{} evaluated {} at {}; it is {} now. The verdict ({}) stays recorded, bound to \
                 the revision it judged; for current work it reads {}",
                e.obligation,
                e.target,
                e.bound_revision.as_deref().unwrap_or_default(),
                e.current_revision,
                e.recorded.as_deref().unwrap_or("none"),
                e.verdict.as_deref().unwrap_or("none")
            ),
        ));
    }
    report.push(Diagnostic::pass("impact.walked", summary(&impact)));
    Ok((report, Some(impact)))
}

/// The disposition `<alias>/<OBL-id>`'s stored verification record holds,
/// read as written. `None` when there is no record or it cannot be read.
fn recorded_verdict(corpus: &Corpus, obligation: &str) -> Option<String> {
    let (alias, id) = obligation.split_once('/')?;
    let repo = corpus.repo();
    let dir = repo.warrant_dir(alias).ok()?;
    let set = repo.load_verifications(&dir).ok()?;
    set.records
        .iter()
        .find(|r| r.obligation == id)
        .map(|r| r.disposition.to_string())
}

fn walk(corpus: &Corpus, model: &Model, subject: &crate::model::Record) -> Impact {
    let repo = corpus.repo();
    let records: BTreeMap<&str, &crate::model::Record> =
        model.records.iter().map(|r| (r.id.as_str(), r)).collect();
    let mut incoming: BTreeMap<&str, Vec<&Relation>> = BTreeMap::new();
    for r in &model.relations {
        incoming.entry(r.to.as_str()).or_default().push(r);
    }

    // ---- affected: incoming relations, transitively, inert ones not walked.
    let mut seen: BTreeSet<&str> = BTreeSet::from([subject.id.as_str()]);
    let mut affected = Vec::new();
    let mut inert = BTreeSet::new();
    let mut queue = VecDeque::from([(subject.id.as_str(), 0u32)]);
    while let Some((node, depth)) = queue.pop_front() {
        for rel in incoming.get(node).into_iter().flatten() {
            if openwarrant_core::relation::RelationKind::parse(&rel.kind)
                .is_ok_and(|k| k.is_inert())
            {
                inert.insert((*rel).clone());
                continue;
            }
            if !seen.insert(rel.from.as_str()) {
                continue;
            }
            queue.push_back((rel.from.as_str(), depth + 1));
            let (kind, source, revision) = records.get(rel.from.as_str()).map_or_else(
                || (String::new(), String::new(), String::new()),
                |r| (r.kind.clone(), r.source.clone(), r.revision.clone()),
            );
            affected.push(Affected {
                id: rel.from.clone(),
                kind,
                source,
                revision,
                via: (*rel).clone(),
                depth: depth + 1,
            });
        }
    }
    affected.sort_by(|a, b| (a.depth, &a.id).cmp(&(b.depth, &b.id)));
    let reached: BTreeSet<&str> = seen.clone();

    // ---- documents: who holds each reached record, and who names one.
    let mut documents: BTreeMap<String, Document> = BTreeMap::new();
    let mut because = |id: &str, kind: &str, why: String| {
        let d = documents.entry(id.to_owned()).or_insert_with(|| Document {
            id: id.to_owned(),
            kind: kind.to_owned(),
            because: Vec::new(),
        });
        if !d.because.contains(&why) {
            d.because.push(why);
        }
    };
    let authored = corpus.records();
    let adapted = corpus.adapters();
    for id in &reached {
        if let Some(r) = authored.get(id) {
            because(&r.source, "records", format!("declares {id}"));
        } else if let Some(r) = authored.instruction(id) {
            // M16: the instruction file that holds the section.
            because(&r.source, "instruction", format!("declares {id}"));
        } else if let Some((kind, source)) = adapted.source_of(id) {
            // M10: a folder read in place declares it.
            because(&source, kind, format!("declares {id}"));
        } else if let Some((head, _)) = id.split_once('/')
            && let Some(holder) = records.get(head)
        {
            because(head, &holder.kind, format!("holds {id}"));
        }
    }
    let named: BTreeSet<&str> = reached
        .iter()
        .copied()
        .filter(|i| !i.contains('/') || i.starts_with(openwarrant_core::instruction::ID_PREFIX))
        .collect();
    if let Ok(entries) = corpus.entries() {
        for e in entries {
            let Some((basis, alias)) = e.ok().and_then(|one| {
                Some((
                    one.basis.as_ref()?,
                    one.validated.as_ref()?.alias.to_string(),
                ))
            }) else {
                continue;
            };
            for atom in &basis.atoms {
                let text = String::from_utf8_lossy(&atom.bytes);
                for id in mentioned(&text, &named).into_iter().filter(|i| *i != alias) {
                    because(&alias, "warrant", format!("names {id} in {}", atom.source));
                }
            }
        }
    }
    if let Ok((tickets, _)) = corpus.tickets() {
        for t in tickets {
            for (text, path) in [
                (&t.intent, &t.intent_path),
                (&t.checklist_text, &t.checklist_path),
            ] {
                for id in mentioned(text, &named).into_iter().filter(|i| *i != t.id()) {
                    because(
                        t.id(),
                        "ticket",
                        format!(
                            "names {id} in {}",
                            path.strip_prefix(&t.dir).unwrap_or(path)
                        ),
                    );
                }
            }
        }
    }
    let mut documents: Vec<Document> = documents.into_values().collect();
    for d in &mut documents {
        d.because.sort();
    }

    // ---- evaluations of anything reached.
    let state = |record: &str, kind: &str| -> Option<String> {
        model
            .states
            .iter()
            .find(|s| s.record == record && s.kind == kind)
            .map(|s| s.value.clone())
    };
    let mut evaluations: Vec<Evaluation> = model
        .relations
        .iter()
        .filter(|r| r.kind == "evaluates" && reached.contains(r.to.as_str()))
        .map(|r| {
            let current = records
                .get(r.to.as_str())
                .map(|t| t.revision.clone())
                .unwrap_or_default();
            let reads = match &r.to_revision {
                None => "unbound",
                Some(pin) if *pin == current => "current",
                Some(_) => "stale",
            };
            Evaluation {
                obligation: r.from.clone(),
                target: r.to.clone(),
                verdict: state(&r.from, "disposition"),
                recorded: recorded_verdict(corpus, &r.from),
                bound_revision: r.to_revision.clone(),
                current_revision: current,
                reads: reads.to_owned(),
            }
        })
        .collect();
    evaluations.sort_by(|a, b| (&a.obligation, &a.target).cmp(&(&b.obligation, &b.target)));

    // ---- phases the affected Warrants feed.
    let warrants: BTreeSet<&str> = documents
        .iter()
        .filter(|d| d.kind == "warrant")
        .map(|d| d.id.as_str())
        .chain(
            reached
                .iter()
                .copied()
                .filter(|i| records.get(i).is_some_and(|r| r.kind == "warrant")),
        )
        .collect();
    let mut phases: BTreeMap<String, Phase> = BTreeMap::new();
    for r in model
        .relations
        .iter()
        .filter(|r| r.kind == "roadmap" && warrants.contains(r.from.as_str()))
    {
        let p = phases.entry(r.to.clone()).or_insert_with(|| Phase {
            id: r.to.clone(),
            via: Vec::new(),
            achieved: state(&r.to, "achieved"),
        });
        if !p.via.contains(&r.from) {
            p.via.push(r.from.clone());
        }
    }
    let phases: Vec<Phase> = phases.into_values().collect();

    // ---- projections known to include them.
    let mut projections: BTreeMap<String, Projection> = BTreeMap::new();
    let mut include = |path: camino::Utf8PathBuf, scope: &str, id: &str| {
        if !crate::vfs::is_file(&path) {
            return;
        }
        let p = projections
            .entry(repo.relative(&path))
            .or_insert_with(|| Projection {
                path: repo.relative(&path),
                scope: scope.to_owned(),
                selects: Vec::new(),
            });
        if !p.selects.iter().any(|s| s == id) {
            p.selects.push(id.to_owned());
        }
    };
    let corpus_views = repo
        .root
        .join(&repo.config.paths.warrants)
        .join("generated");
    for w in &warrants {
        if let Some(e) = corpus.entry(w)
            && let Ok(rd) = crate::vfs::read_dir(e.dir.join("generated"))
        {
            let mut files: Vec<camino::Utf8PathBuf> = rd
                .filter_map(Result::ok)
                .filter_map(|x| camino::Utf8PathBuf::from_path_buf(x.path()).ok())
                .collect();
            files.sort();
            for f in files {
                include(f, "warrant", w);
            }
        }
        for name in [
            "CORPUS_STATUS.md",
            "CORPUS_STATUS.json",
            "CORPUS_STATUS.html",
            "WARRANT_OVERVIEW.md",
        ] {
            include(corpus_views.join(name), "corpus", w);
        }
    }
    for p in &phases {
        include(
            crate::roadmap_cmd::dir(repo).join("view.json"),
            "roadmap",
            &p.id,
        );
    }
    // OW-WAR-0148 M6: the declared projections that select the subject.
    for p in crate::render_cmd::selecting(corpus, model, &subject.id, &reached) {
        projections.insert(p.path.clone(), p);
    }
    let projections: Vec<Projection> = projections.into_values().collect();

    Impact {
        schema: SCHEMA.to_owned(),
        basis_digest: model.basis_digest.clone(),
        record: Subject {
            id: subject.id.clone(),
            kind: subject.kind.clone(),
            source: subject.source.clone(),
            revision: subject.revision.clone(),
        },
        affected,
        inert: inert.into_iter().collect(),
        documents,
        evaluations,
        phases,
        projections,
    }
}

#[must_use]
pub fn summary(i: &Impact) -> String {
    format!(
        "{}: {} affected record(s), {} document(s), {} evaluation(s) ({} stale), {} phase(s), \
         {} projection(s); {} inert relation(s) not walked",
        i.record.id,
        i.affected.len(),
        i.documents.len(),
        i.evaluations.len(),
        i.evaluations.iter().filter(|e| e.reads == "stale").count(),
        i.phases.len(),
        i.projections.len(),
        i.inert.len()
    )
}

fn short(rev: &str) -> &str {
    let hex = rev.strip_prefix("sha256:").unwrap_or(rev);
    &hex[..hex.len().min(12)]
}

/// The human rendering.
#[must_use]
pub fn render(i: &Impact) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    let _ = writeln!(
        s,
        "{} · {}  {}  {}",
        i.record.id, i.record.kind, i.record.source, i.record.revision
    );
    let _ = writeln!(s, "\naffected ({}):", i.affected.len());
    for a in &i.affected {
        let _ = writeln!(
            s,
            "  {} · {}  ({} {} {}, depth {})",
            a.id, a.kind, a.via.from, a.via.kind, a.via.to, a.depth
        );
    }
    let _ = writeln!(s, "\ndocuments ({}):", i.documents.len());
    for d in &i.documents {
        let _ = writeln!(s, "  {} ({}): {}", d.id, d.kind, d.because.join("; "));
    }
    let _ = writeln!(s, "\nevaluations ({}):", i.evaluations.len());
    for e in &i.evaluations {
        let bound = e
            .bound_revision
            .as_deref()
            .map_or_else(|| "no revision".to_owned(), |b| short(b).to_owned());
        let _ = writeln!(
            s,
            "  {} evaluates {}: verdict {} (recorded {}), bound to {bound}, reads {} ({} is {} now)",
            e.obligation,
            e.target,
            e.verdict.as_deref().unwrap_or("none"),
            e.recorded.as_deref().unwrap_or("none"),
            e.reads,
            e.target,
            short(&e.current_revision)
        );
    }
    let _ = writeln!(s, "\nphases ({}):", i.phases.len());
    for p in &i.phases {
        let _ = writeln!(
            s,
            "  {} via {}: progress recomputed (achieved: {})",
            p.id,
            p.via.join(", "),
            p.achieved.as_deref().unwrap_or("unknown")
        );
    }
    let _ = writeln!(s, "\nprojections ({}):", i.projections.len());
    for p in &i.projections {
        let _ = writeln!(s, "  {} ({}: {})", p.path, p.scope, p.selects.join(", "));
    }
    if !i.inert.is_empty() {
        let _ = writeln!(s, "\ninert, not walked ({}):", i.inert.len());
        for r in &i.inert {
            let _ = writeln!(s, "  {} {} {}", r.from, r.kind, r.to);
        }
    }
    s
}
