// SPDX-License-Identifier: Apache-2.0
//! The adapter: `oh.war/model/v1` as the compiler's [`Graph`].
//!
//! Every unit is a model record, read as the model gives it: its id, type,
//! title, document, source, line, revision and `content_digest`, its
//! `fields` (`summary`, `paths`, `uuid`) and its primary state. The model
//! lowers the legacy encodings (Format 1.0 section 12) itself, so a
//! directory Warrant's atoms are already its sections and its milestones
//! already records here. The only bytes read are each record's own span of
//! its source, `[start, end]` as the model gives it, for the `[F]` tier.
//!
//! Two things the model does not state, derived from what it does:
//! - a legacy Warrant's `paths` are its deliverables' `target_ref` fields
//!   (a Format 1.0 work document authors `paths` itself);
//! - a record with no title (a records/v1 record) shows its first body line
//!   that is not one of its own relation lines.
//!
//! Only tracked inputs are read: the model, and the files its records cite.
//! Claim locks, journals' clocks, the ledger and the environment are never
//! consulted here.

use std::collections::BTreeMap;

use crate::corpus::Corpus;
use crate::model::{FieldValue, Model, Record, State};

use super::{Edge, Finding, Graph, Unit, cut_title};

/// The files records cite, read once each.
struct Files<'a> {
    root: &'a camino::Utf8Path,
    cache: BTreeMap<String, Option<Vec<u8>>>,
}

impl Files<'_> {
    /// The bytes `[start, end)` of `rel`, when the span is non-empty and
    /// inside the file.
    fn span(&mut self, rel: &str, [start, end]: [usize; 2]) -> Option<String> {
        let root = self.root;
        let bytes = self
            .cache
            .entry(rel.to_owned())
            .or_insert_with(|| crate::vfs::read(root.join(rel)).ok())
            .as_deref()?;
        (start < end)
            .then(|| bytes.get(start..end))
            .flatten()
            .map(|b| String::from_utf8_lossy(b).into_owned())
    }
}

/// The primary state of each record (FORMAT-1.0 section 11.1, as the index
/// and packets show it): [`crate::states::primary_state`] over its states.
/// A record whose primary state is `-` has no entry.
fn primary_states(model: &Model, states: &[State]) -> BTreeMap<String, String> {
    let mut of: BTreeMap<&str, Vec<&State>> = BTreeMap::new();
    for s in states {
        of.entry(s.record.as_str()).or_default().push(s);
    }
    model
        .records
        .iter()
        .filter_map(|r| {
            let p = crate::states::primary_state(&r.kind, of.get(r.id.as_str())?);
            (p != "-").then(|| (r.id.clone(), p))
        })
        .collect()
}

fn one(r: &Record, key: &str) -> Option<String> {
    r.fields
        .get(key)
        .and_then(FieldValue::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn many(r: &Record, key: &str) -> Option<Vec<String>> {
    match r.fields.get(key)? {
        FieldValue::One(s) if s.is_empty() => None,
        FieldValue::One(s) => Some(vec![s.clone()]),
        FieldValue::Many(v) => Some(v.clone()),
    }
}

/// A title for a record whose heading has none: its first non-blank body
/// line that is not one of its own relation lines (`<kind> <target>`).
fn body_title(r: &Record, relations: &[(&str, &str)]) -> Option<String> {
    r.body
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .find(|l| {
            !relations.iter().any(|(kind, to)| {
                l.strip_prefix(kind)
                    .is_some_and(|rest| rest.starts_with(' ') && rest.contains(to))
            })
        })
        .map(cut_title)
}

/// The graph of the corpus, its states those of the documents alone
/// ([`crate::states::document_states`]): what the index and every preset
/// that does not read state are bound to (section 10.3's purity rule).
#[must_use]
pub fn graph(corpus: &Corpus, model: &Model) -> Graph {
    let states = crate::states::document_states(corpus, &model.states);
    graph_with(corpus, model, &states)
}

/// The graph of the corpus with every state of `model`, journals and
/// claims included: what a preset that reads state shows (section 10.4).
#[must_use]
pub fn graph_reading_state(corpus: &Corpus, model: &Model) -> Graph {
    graph_with(corpus, model, &model.states)
}

fn graph_with(corpus: &Corpus, model: &Model, states: &[State]) -> Graph {
    let repo = corpus.repo();
    let mut files = Files {
        root: &repo.root,
        cache: BTreeMap::new(),
    };
    let states = primary_states(model, states);
    // Section 10.5: a unit of a Format 1.0 document is editable through a
    // package; the document's own record (its frontmatter and preamble,
    // span from byte 0) is not, nor is anything of a legacy encoding.
    let format_sources: std::collections::BTreeSet<&str> = corpus
        .format_docs()
        .docs
        .iter()
        .map(|d| d.source.as_str())
        .collect();
    let mut g = Graph {
        name: repo.config.project.name.clone(),
        ..Graph::default()
    };
    // A legacy Warrant's `paths`: the files its deliverables declare.
    let mut deliverable_paths: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for r in model.records.iter().filter(|r| r.kind == "deliverable") {
        if let (Some(doc), Some(t)) = (&r.document, one(r, "target_ref")) {
            deliverable_paths.entry(doc.as_str()).or_default().push(t);
        }
    }
    let mut outgoing: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
    for rel in &model.relations {
        outgoing
            .entry(rel.from.as_str())
            .or_default()
            .push((rel.kind.as_str(), rel.to.as_str()));
    }
    for r in &model.records {
        let work = matches!(r.kind.as_str(), "warrant" | "ticket");
        let title = r.title.clone().filter(|t| !t.is_empty()).or_else(|| {
            body_title(
                r,
                outgoing.get(r.id.as_str()).map_or(&[][..], Vec::as_slice),
            )
        });
        let u = Unit {
            id: r.id.clone(),
            kind: r.kind.clone(),
            title,
            // A document's own record carries the document's `summary`
            // (section 10.2's `> ` line): a work document's, and a
            // `doc:<path>` record's.
            summary: (work || r.id.starts_with("doc:"))
                .then(|| one(r, "summary"))
                .flatten(),
            work,
            document: r.document.clone(),
            section: r.section.clone(),
            span: r.span,
            source: r.source.clone(),
            line: r.line.max(1),
            revision: r.revision.clone(),
            content_digest: Some(r.content_digest.clone()).filter(|d| !d.is_empty()),
            text: files.span(&r.source, r.span),
            state: states.get(r.id.as_str()).cloned(),
            paths: if work {
                many(r, "paths")
                    .or_else(|| deliverable_paths.get(r.id.as_str()).cloned())
                    .unwrap_or_default()
            } else {
                Vec::new()
            },
            uuid: work.then(|| one(r, "uuid")).flatten(),
            editable: !work
                && r.span[0] > 0
                && r.span[0] < r.span[1]
                && format_sources.contains(r.source.as_str()),
        };
        if u.text.is_none() {
            g.diagnostics.push(Finding {
                rule: "context.span-unknown".to_owned(),
                record: r.id.clone(),
                message: format!(
                    "{}: the model gives this {} no span in {}; it is given by its summary only",
                    r.id, r.kind, r.source
                ),
            });
        }
        g.add(u);
    }
    // A `war://<uuid>` target stays as authored in the model; the walk
    // follows it to the work document with that uuid.
    let by_uuid: BTreeMap<String, String> = g
        .units
        .values()
        .filter_map(|u| Some((format!("war://{}", u.uuid.as_ref()?), u.id.clone())))
        .collect();
    g.edges = model
        .relations
        .iter()
        .map(|r| Edge {
            from: r.from.clone(),
            kind: r.kind.clone(),
            to: by_uuid.get(&r.to).cloned().unwrap_or_else(|| r.to.clone()),
        })
        .collect();
    g.edges.sort();
    g.edges.dedup();
    g.diagnostics.sort();
    g.diagnostics.dedup();
    g
}

/// What a preset that reads state reads besides the documents (section
/// 10.4), at `now` (Unix seconds; `at` is its RFC 3339 text as the header
/// shows it): the digest of every journal and claim file, each item's
/// `resume` state line, and each work document's candidates of section
/// 11.5. The claims and their leases are judged at `now`.
#[must_use]
pub fn tracking(corpus: &Corpus, graph: &Graph, at: &str, now: u64) -> super::Tracking {
    let repo = corpus.repo();
    let mut out = super::Tracking {
        at: at.to_owned(),
        ..super::Tracking::default()
    };
    // Every journal and every claim file, by repository path.
    let mut files: std::collections::BTreeSet<String> = graph
        .units
        .values()
        .filter(|u| u.work)
        .filter_map(|u| evidence_journal(&u.id, &u.source))
        .collect();
    let store = crate::ticket::Store::open(repo, None).ok();
    if let Some(store) = &store {
        let dirs = std::iter::once(&store.claims_dir).chain(store.legacy_claims_dirs());
        for dir in dirs {
            for path in crate::ticket::claim::lock_files(dir) {
                files.insert(relative_to(&repo.root, &path));
            }
        }
    }
    let pairs: Vec<(String, String)> = files
        .into_iter()
        .filter_map(|rel| {
            let bytes = crate::vfs::read(repo.root.join(&rel)).ok()?;
            Some((
                rel,
                format!("sha256:{}", openwarrant_compiler::sha256_hex(&bytes)),
            ))
        })
        .collect();
    out.journal_digest =
        super::selection_digest(pairs.iter().map(|(a, b)| (a.as_str(), b.as_str())));
    let Some(store) = store else {
        return out;
    };
    let Ok((tickets, _)) = store.load_all() else {
        return out;
    };
    let claims = store.claims().unwrap_or_default();
    let reader = crate::ticket::ladder::Reader::new(&store);
    for t in &tickets {
        let ticks = crate::ticket::ladder::ticks_of(&reader, t);
        let whole = crate::ticket::claim_on(&claims, t.id(), None);
        for item in &t.checklist.items {
            let Some(iid) = &item.id else { continue };
            let id = format!("{}/{iid}", t.id());
            let Some(state) = graph.units.get(&id).and_then(|u| u.state.clone()) else {
                continue;
            };
            let base = state.split('/').next().unwrap_or_default();
            let mut line = format!("state {state}");
            if base == "claimed"
                && let Some((_, Some(c))) =
                    crate::ticket::claim_on(&claims, t.id(), Some(iid)).or(whole)
            {
                let until = c.lease_until_unix.map_or_else(
                    || "-".to_owned(),
                    crate::gate_cmd::receipt::rfc3339_from_secs,
                );
                line.push_str(&format!(" · holder {} · lease {until}", c.actor));
            }
            if base == "done" {
                let by = item.done_by.as_deref().unwrap_or("-");
                let date = item
                    .done_on
                    .as_deref()
                    .map_or("-", |d| openwarrant_core::ticks::split_level(d).0);
                let level = ticks.items.get(iid).map_or("claimed", |v| v.level.as_str());
                line.push_str(&format!(" · by {by} · {date} · {level}"));
            }
            out.state_lines.insert(id, line);
        }
    }
    if let Ok(rows) = crate::ticket::ready_rows_at(&store, &tickets, claims, now) {
        for row in rows {
            out.next
                .entry(row.ticket.clone())
                .or_default()
                .push(row.target());
        }
    }
    out
}

/// A work document's journal (section 11.2): `journal.jsonl` in its
/// evidence directory, `<dir>/<alias>/` beside a one-file document, the
/// document's own directory otherwise.
pub(super) fn evidence_journal(alias: &str, source: &str) -> Option<String> {
    let (dir, file) = source.rsplit_once('/').unwrap_or(("", source));
    let join = |d: &str| {
        if d.is_empty() {
            format!("{alias}/journal.jsonl")
        } else {
            format!("{d}/{alias}/journal.jsonl")
        }
    };
    if file == format!("{alias}.md") {
        return Some(join(dir));
    }
    let parts: Vec<&str> = source.split('/').collect();
    let at = parts.iter().position(|p| *p == alias)?;
    Some(format!("{}/journal.jsonl", parts[..=at].join("/")))
}

/// `path` from the repository root, `/`-separated; a path outside it is
/// kept as it is.
fn relative_to(root: &camino::Utf8Path, path: &camino::Utf8Path) -> String {
    path.strip_prefix(root).map_or_else(
        |_| path.as_str().to_owned(),
        |p| p.as_str().replace('\\', "/"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(body: &str) -> Record {
        serde_json::from_value(serde_json::json!({
            "id": "CON-pr1", "type": "constraint", "source": "docs/records/a.md",
            "revision": "sha256:00", "body": body
        }))
        .unwrap()
    }

    #[test]
    fn an_untitled_record_shows_its_first_body_line_that_is_not_a_relation() {
        let r = record("\nconstrains REQ-pr1\n  Tokens expire in 15 minutes.\nmore\n");
        assert_eq!(
            body_title(&r, &[("constrains", "REQ-pr1")]).as_deref(),
            Some("Tokens expire in 15 minutes.")
        );
        // Without the relation known, the relation line is the first line.
        assert_eq!(body_title(&r, &[]).as_deref(), Some("constrains REQ-pr1"));
        assert_eq!(body_title(&record("\n\n"), &[]), None);
    }

    #[test]
    fn a_span_outside_its_file_or_empty_is_no_text() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!("war-context-span-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("f.md"), "abcdef").unwrap();
        let mut files = Files {
            root: &root,
            cache: BTreeMap::new(),
        };
        assert_eq!(files.span("f.md", [1, 4]).as_deref(), Some("bcd"));
        assert_eq!(files.span("f.md", [0, 0]), None);
        assert_eq!(files.span("f.md", [2, 99]), None);
        assert_eq!(files.span("missing.md", [0, 1]), None);
        let _ = std::fs::remove_dir_all(&root);
    }
}
