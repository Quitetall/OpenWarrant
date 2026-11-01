// SPDX-License-Identifier: Apache-2.0
//! Complete task packets over an explicit model and source loader. No model call.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Graph, Refusal};

pub const SCHEMA: &str = "oh.war/task-context/v1";
const MISSING: &str = "context.required-missing";

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceEntry {
    pub id: String,
    pub source: String,
    pub revision: String,
    pub payload_sha256: String,
    pub coverage: String,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub source: String,
    pub line: usize,
    pub revision: String,
    pub state: String,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Projection {
    /// Required source text appears once here; citations are metadata only.
    pub markdown: String,
    pub records: Vec<Citation>,
    pub diagnostics: Vec<super::Finding>,
    pub applicability: String,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPacket {
    pub schema: String,
    pub target: String,
    pub selection_digest: String,
    pub projection: Projection,
    pub source_manifest: Vec<SourceEntry>,
    pub supplemental_sources: BTreeMap<String, String>,
    pub qualification_established: bool,
}

fn missing(message: impl Into<String>) -> Refusal {
    Refusal {
        rule: MISSING,
        message: message.into(),
        needed: None,
    }
}

/// Required constraints/dependencies are transitive. Containment never drags
/// unrelated sibling Warrants into a packet through their common parent.
pub fn compile(
    graph: &Graph,
    target: &str,
    mut source: impl FnMut(&str) -> Result<String, String>,
) -> Result<TaskPacket, Refusal> {
    let selected = required_ids(graph, target)?;
    let mut manifest = Vec::new();
    let mut supplemental: BTreeMap<String, String> = BTreeMap::new();
    for id in &selected {
        let unit = graph
            .units
            .get(id)
            .ok_or_else(|| missing(format!("{target}: required record {id} is unavailable")))?;
        let (payload, coverage) = if let Some(text) = &unit.text {
            (
                text.clone(),
                if unit.kind == "x.journal-notes" {
                    "derived-journal-notes"
                } else {
                    "record"
                },
            )
        } else {
            let text = if let Some(text) = supplemental.get(&unit.source) {
                text.clone()
            } else {
                let text = source(&unit.source).map_err(|why| {
                    missing(format!(
                        "{target}: required source {} for {id} is unavailable: {why}",
                        unit.source
                    ))
                })?;
                supplemental.insert(unit.source.clone(), text.clone());
                text
            };
            (text, "whole-source")
        };
        if payload.is_empty() {
            return Err(missing(format!(
                "{target}: required text for {id} is empty"
            )));
        }
        manifest.push(SourceEntry {
            id: id.clone(),
            source: unit.source.clone(),
            revision: unit.revision.clone(),
            payload_sha256: openwarrant_core::format::sha256(payload.as_bytes()),
            coverage: coverage.into(),
        });
    }
    let packet = super::compile_selected(graph, target, &selected)?;
    let projection=Projection {
        markdown:packet.markdown,
        records:packet.records.into_iter().map(|r|Citation{id:r.id,kind:r.kind,source:r.source,line:r.line,revision:r.revision,state:r.state}).collect(),
        diagnostics:packet.diagnostics,
        applicability:"conservative: include declared constraints and their dependencies when stage applicability is not known".into(),
    };
    let binding = serde_jcs::to_vec(
        &serde_json::json!({"projection":packet.selection_digest,"sources":manifest}),
    )
    .map_err(|e| missing(e.to_string()))?;
    let mut digest = SCHEMA.as_bytes().to_vec();
    digest.push(b'\n');
    digest.extend(binding);
    Ok(TaskPacket {
        schema: SCHEMA.into(),
        target: target.into(),
        selection_digest: openwarrant_core::format::sha256(&digest),
        projection,
        source_manifest: manifest,
        supplemental_sources: supplemental,
        qualification_established: false,
    })
}

fn required_ids(
    graph: &Graph,
    target: &str,
) -> Result<std::collections::BTreeSet<String>, Refusal> {
    let seeds = super::resolve(graph, target)?;
    let mut selected = seeds.zero;
    selected.extend(seeds.roots);
    selected.extend(seeds.one.into_keys());
    loop {
        let before = selected.len();
        let current: Vec<_> = selected.iter().cloned().collect();
        for id in current {
            let unit = graph
                .units
                .get(&id)
                .ok_or_else(|| missing(format!("{target}: required record {id} is unavailable")))?;
            if let Some(parent) = &unit.document {
                selected.insert(parent.clone());
                for shared in graph.units.values().filter(|u| {
                    u.document.as_ref() == Some(parent)
                        && !matches!(u.kind.as_str(), "stage" | "milestone" | "item")
                }) {
                    selected.insert(shared.id.clone());
                }
            }
            if unit.work {
                selected.extend(
                    graph
                        .units
                        .values()
                        .filter(|u| u.document.as_deref() == Some(&id))
                        .map(|u| u.id.clone()),
                );
            }
        }
        for edge in &graph.edges {
            if selected.contains(&edge.from)
                && matches!(
                    edge.kind.as_str(),
                    "depends_on" | "implements" | "x.repairs"
                )
            {
                selected.insert(edge.to.clone());
            }
            if selected.contains(&edge.to)
                && matches!(edge.kind.as_str(), "constrains" | "evaluates")
            {
                selected.insert(edge.from.clone());
            }
        }
        if selected.len() == before {
            break;
        }
    }
    Ok(selected)
}

/// Filesystem adapter. A provider can use `compile` with its own exact sources.
pub fn repository(
    repo: &crate::repo::Repository,
    target: &str,
) -> Result<TaskPacket, crate::repo::RepoError> {
    let corpus = crate::corpus::Corpus::new(repo);
    let model = crate::model::build(&corpus)?;
    let mut graph = super::adapter::graph(&corpus, &model);
    for name in ["AGENTS.md", "CLAUDE.md", "CONTEXT.md"] {
        let original = repo.root.join(name);
        if !crate::vfs::exists(&original) {
            continue;
        }
        let canonical = original.canonicalize_utf8().map_err(|e| {
            crate::repo::RepoError::Message(format!("context.required-missing: {name}: {e}"))
        })?;
        let root = repo
            .root
            .canonicalize_utf8()
            .map_err(|e| crate::repo::RepoError::Message(e.to_string()))?;
        let relative=canonical.strip_prefix(&root).map_err(|_|crate::repo::RepoError::Message(format!("context.required-missing: {name} resolves outside this repository; supply it through the trusted context provider")))?;
        let text = crate::vfs::read_regular_to_string(root.as_std_path(), canonical.as_std_path())
            .map_err(|e| {
                crate::repo::RepoError::Message(format!("context.required-missing: {name}: {e}"))
            })?;
        let id = format!("host:{name}");
        graph.add(super::Unit {
            id: id.clone(),
            kind: "instruction".into(),
            source: relative.to_string(),
            line: 1,
            span: [0, text.len()],
            revision: openwarrant_core::format::sha256(text.as_bytes()),
            text: Some(text),
            ..Default::default()
        });
        let seeds = super::resolve(&graph, target)
            .map_err(|e| crate::repo::RepoError::Message(e.to_string()))?;
        for root in seeds.roots {
            graph.edges.push(super::Edge {
                from: id.clone(),
                kind: "constrains".into(),
                to: root,
            });
        }
    }
    let selected =
        required_ids(&graph, target).map_err(|e| crate::repo::RepoError::Message(e.to_string()))?;
    let works: Vec<_> = selected
        .iter()
        .filter_map(|id| graph.units.get(id))
        .filter(|u| u.work)
        .map(|u| (u.id.clone(), u.source.clone()))
        .collect();
    for (alias, document) in works {
        let Some(journal) = super::adapter::evidence_journal(&alias, &document) else {
            continue;
        };
        let path = repo.root.join(&journal);
        if !crate::vfs::exists(&path) {
            continue;
        }
        let notes = (|| -> Result<Option<(String, String)>, String> {
            let raw = crate::vfs::read_anchored_limited(
                repo.root.as_std_path(),
                std::path::Path::new(&journal),
                8 * 1024 * 1024,
            )
            .map_err(|e| e.to_string())?;
            let text = std::str::from_utf8(&raw).map_err(|e| e.to_string())?;
            let parsed = crate::journal_cmd::parse(text).map_err(|e| e.to_string())?;
            let mut projection = String::new();
            for event in parsed
                .events
                .iter()
                .filter(|e| crate::journal_cmd::work_type(&e.event_type) == "work.note_added")
            {
                let Ok(payload) = serde_json::from_str::<serde_json::Value>(&event.payload) else {
                    continue;
                };
                let Some(note) = payload["note"].as_str() else {
                    continue;
                };
                let Some(note_target) = payload["target"].as_str() else {
                    continue;
                };
                if note_target != alias && !selected.contains(note_target) {
                    continue;
                }
                if projection.is_empty() {
                    projection.push_str("# Recorded task notes\n\nDeterministic projection of recorded experience; it grants no authority.\n\n");
                }
                projection.push_str(&format!(
                    "## {} · {} · {}\n\n{}\n\n",
                    event.id, event.actor_ref, event.occurred_at, note
                ));
            }
            Ok((!projection.is_empty())
                .then(|| (openwarrant_core::format::sha256(&raw), projection)))
        })();
        match notes {
            Ok(Some((revision, text))) => {
                let id = format!("runtime-notes:{alias}:{revision}");
                if graph.units.contains_key(&id) {
                    return Err(crate::repo::RepoError::Message("context.required-missing: runtime note projection ID collides with an authored record".into()));
                }
                graph.add(super::Unit {
                    id,
                    kind: "x.journal-notes".into(),
                    title: Some("Recorded task notes".into()),
                    document: Some(alias),
                    source: journal,
                    line: 1,
                    revision,
                    text: Some(text),
                    ..Default::default()
                });
            }
            Ok(None) => {}
            Err(message) => graph.diagnostics.push(super::Finding {
                rule: "context.notes-unavailable".into(),
                record: alias,
                message,
            }),
        }
    }
    compile(&graph, target, |path| {
        crate::vfs::read_regular_to_string(
            repo.root.as_std_path(),
            repo.root.join(path).as_std_path(),
        )
        .map_err(|e| e.to_string())
    })
    .map_err(|e| crate::repo::RepoError::Message(e.to_string()))
}
