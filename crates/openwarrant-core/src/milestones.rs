// SPDX-License-Identifier: AGPL-3.0-or-later
//! Milestones and stages (SAS §23), executor kinds (§23.4), named typed ports
//! (§23.5), and responsibility tiers (§26).
//!
//! §23.6 keeps milestones and stages distinct and this module refuses to blur
//! them: a milestone is an ACCEPTANCE CHECKPOINT and carries no executor; a
//! stage is a DISPATCHABLE EXECUTION NODE and carries no obligations. Wiring an
//! executor onto a milestone, or obligations onto a stage, is a parse error.
//!
//! # What this module does NOT do
//!
//! It says nothing about whether a milestone is MET. That needs the state model
//! (OW-WAR-0008) and gate runs (OW-WAR-0020). A milestone here is a well-formed
//! declaration, and any caller inferring completion from a parse is wrong.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::structured::{self, StructuredError, StructuredValue};

/// The only milestones schema this build understands.
pub const MILESTONES_SCHEMA: &str = "oh.war/milestones/v1";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MilestoneError {
    #[error(transparent)]
    Structured(#[from] StructuredError),
    #[error("unknown milestones schema {found:?}; this build understands {expected:?}")]
    UnknownSchema { found: String, expected: String },
    #[error("the atom declares no `schema`")]
    MissingSchema,
    #[error("{kind} {index} declares no `id`")]
    MissingId { kind: &'static str, index: usize },
    #[error("duplicate {kind} id {id:?}")]
    DuplicateId { kind: &'static str, id: String },
    #[error("milestone {milestone:?} references stage {stage:?}, which is not declared")]
    DanglingStageRef { milestone: String, stage: String },
    #[error("milestone {milestone:?} depends on {dependency:?}, which is not declared")]
    DanglingDependency {
        milestone: String,
        dependency: String,
    },
    #[error("milestone dependency cycle: {path}")]
    DependencyCycle { path: String },
    #[error(
        "milestone {id:?} declares {field:?}; that belongs to a stage. \
         §23.6 keeps milestones and stages distinct"
    )]
    MilestoneHasStageField { id: String, field: String },
    #[error(
        "stage {id:?} declares {field:?}; that belongs to a milestone. \
         §23.6 keeps milestones and stages distinct"
    )]
    StageHasMilestoneField { id: String, field: String },
    #[error("unknown executor kind {found:?} on stage {id:?}; expected one of {known}")]
    UnknownExecutorKind {
        id: String,
        found: String,
        known: String,
    },
    #[error("unknown responsibility tier {found:?} on stage {id:?}; expected T1, T2, T3, or T4")]
    UnknownTier { id: String, found: String },
    #[error("stage {id:?} declares port {port:?} with no type; §23.5 requires ports to be typed")]
    UntypedPort { id: String, port: String },
}

/// Who executes a stage (SAS §23.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutorKind {
    Human,
    Agent,
    Service,
    Katana,
    Blut,
    Laboratory,
}

impl ExecutorKind {
    pub const ALL: [Self; 6] = [
        Self::Human,
        Self::Agent,
        Self::Service,
        Self::Katana,
        Self::Blut,
        Self::Laboratory,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Agent => "agent",
            Self::Service => "service",
            Self::Katana => "katana",
            Self::Blut => "blut",
            Self::Laboratory => "laboratory",
        }
    }
}

impl std::fmt::Display for ExecutorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Executor responsibility tier (SAS §26).
///
/// §26.5: tier and executor kind are ORTHOGONAL. A T1 stage may be executed by
/// any kind of actor, and nothing here may infer one from the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ResponsibilityTier {
    /// Constitutional or mission-critical (§26.1).
    T1,
    /// Implementation judgment (§26.2).
    T2,
    /// Minor implementation detail (§26.3).
    T3,
    /// Mechanical (§26.4).
    T4,
}

impl ResponsibilityTier {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::T1 => "T1",
            Self::T2 => "T2",
            Self::T3 => "T3",
            Self::T4 => "T4",
        }
    }
}

impl FromStr for ResponsibilityTier {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "T1" => Ok(Self::T1),
            "T2" => Ok(Self::T2),
            "T3" => Ok(Self::T3),
            "T4" => Ok(Self::T4),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for ResponsibilityTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A named typed port on a stage (SAS §23.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Port {
    pub name: String,
    pub type_name: String,
}

/// An acceptance checkpoint (SAS §23.1). Carries no executor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stage_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub obligation_refs: Vec<String>,
}

/// A dispatchable execution node (SAS §23.3). Carries no obligations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub executor_kind: ExecutorKind,
    pub responsibility_tier: ResponsibilityTier,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<Port>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<Port>,
    /// The name this stage's EXECUTOR knows it by (§49.2).
    ///
    /// A WAR stage id (`STAGE-002`) is an identifier inside this Warrant. It is
    /// not what BLUT, Katana or a laboratory calls the thing that runs. The
    /// adapter used the id as the BLUT stage name, which meant every lowering
    /// named `STAGE-NNN` and was refused for naming a stage no cookbook has —
    /// a refusal that looked like the pinned-registry rule working, when in fact
    /// the question "which executor stage is this?" had never been asked.
    ///
    /// Deliberately executor-NEUTRAL. `katana` and `laboratory` stages have the
    /// same problem and get the same field, rather than each seam growing a
    /// private one.
    ///
    /// `None` is honest for a stage nobody has bound yet; the adapter refuses to
    /// lower it rather than guessing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executor_ref: Option<String>,
}

/// A validated milestone graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MilestoneGraph {
    pub milestones: Vec<Milestone>,
    pub stages: Vec<Stage>,
}

impl MilestoneGraph {
    /// Milestones referencing no stage. Not an error — a checkpoint may be
    /// satisfied by obligations alone — but worth reporting.
    #[must_use]
    pub fn milestones_without_stages(&self) -> Vec<&str> {
        self.milestones
            .iter()
            .filter(|m| m.stage_refs.is_empty())
            .map(|m| m.id.as_str())
            .collect()
    }

    /// Stages no milestone references. Work nobody is waiting on.
    #[must_use]
    pub fn unreferenced_stages(&self) -> Vec<&str> {
        let referenced: BTreeSet<&str> = self
            .milestones
            .iter()
            .flat_map(|m| m.stage_refs.iter().map(String::as_str))
            .collect();
        self.stages
            .iter()
            .filter(|s| !referenced.contains(s.id.as_str()))
            .map(|s| s.id.as_str())
            .collect()
    }

    /// Every obligation identifier referenced by any milestone.
    #[must_use]
    pub fn referenced_obligations(&self) -> BTreeSet<&str> {
        self.milestones
            .iter()
            .flat_map(|m| m.obligation_refs.iter().map(String::as_str))
            .collect()
    }
}

fn scalar(record: &BTreeMap<String, StructuredValue>, key: &str) -> Option<String> {
    record
        .get(key)
        .and_then(StructuredValue::as_scalar)
        .map(str::to_owned)
}

fn list(record: &BTreeMap<String, StructuredValue>, key: &str) -> Vec<String> {
    record
        .get(key)
        .and_then(StructuredValue::as_list)
        .map(<[String]>::to_vec)
        .unwrap_or_default()
}

/// Parse and validate a milestones atom.
pub fn parse(source: &str) -> Result<MilestoneGraph, MilestoneError> {
    let doc = structured::parse(source)?;

    match doc.scalar("schema") {
        None => return Err(MilestoneError::MissingSchema),
        Some(s) if s != MILESTONES_SCHEMA => {
            return Err(MilestoneError::UnknownSchema {
                found: s.to_owned(),
                expected: MILESTONES_SCHEMA.to_owned(),
            });
        }
        Some(_) => {}
    }

    // §23.6: fields that belong to the other kind are refused, not ignored.
    const STAGE_ONLY: [&str; 5] = [
        "executor_kind",
        "responsibility_tier",
        "inputs",
        "outputs",
        "executor_ref",
    ];
    const MILESTONE_ONLY: [&str; 3] = ["depends_on", "stage_refs", "obligation_refs"];

    let mut milestones = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, record) in doc.records("milestones").unwrap_or(&[]).iter().enumerate() {
        let id = scalar(record, "id").ok_or(MilestoneError::MissingId {
            kind: "milestone",
            index,
        })?;
        if !seen.insert(id.clone()) {
            return Err(MilestoneError::DuplicateId {
                kind: "milestone",
                id,
            });
        }
        if let Some(field) = STAGE_ONLY.iter().find(|f| record.contains_key(**f)) {
            return Err(MilestoneError::MilestoneHasStageField {
                id,
                field: (*field).to_owned(),
            });
        }
        milestones.push(Milestone {
            title: scalar(record, "title"),
            depends_on: list(record, "depends_on"),
            stage_refs: list(record, "stage_refs"),
            obligation_refs: list(record, "obligation_refs"),
            id,
        });
    }

    let mut stages = Vec::new();
    let mut seen_stages = BTreeSet::new();
    for (index, record) in doc.records("stages").unwrap_or(&[]).iter().enumerate() {
        let id = scalar(record, "id").ok_or(MilestoneError::MissingId {
            kind: "stage",
            index,
        })?;
        if !seen_stages.insert(id.clone()) {
            return Err(MilestoneError::DuplicateId { kind: "stage", id });
        }
        if let Some(field) = MILESTONE_ONLY.iter().find(|f| record.contains_key(**f)) {
            return Err(MilestoneError::StageHasMilestoneField {
                id,
                field: (*field).to_owned(),
            });
        }

        let raw_kind = scalar(record, "executor_kind").unwrap_or_default();
        let executor_kind = ExecutorKind::ALL
            .into_iter()
            .find(|k| k.as_str() == raw_kind)
            .ok_or_else(|| MilestoneError::UnknownExecutorKind {
                id: id.clone(),
                found: raw_kind.clone(),
                known: ExecutorKind::ALL
                    .iter()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            })?;

        let raw_tier = scalar(record, "responsibility_tier").unwrap_or_default();
        let responsibility_tier =
            ResponsibilityTier::from_str(&raw_tier).map_err(|()| MilestoneError::UnknownTier {
                id: id.clone(),
                found: raw_tier.clone(),
            })?;

        // Ports are `name:type` pairs; §23.5 requires them typed.
        let ports = |key: &str| -> Result<Vec<Port>, MilestoneError> {
            list(record, key)
                .into_iter()
                .map(|entry| {
                    entry
                        .split_once(':')
                        .map(|(n, t)| Port {
                            name: n.trim().to_owned(),
                            type_name: t.trim().to_owned(),
                        })
                        .filter(|p| !p.name.is_empty() && !p.type_name.is_empty())
                        .ok_or_else(|| MilestoneError::UntypedPort {
                            id: id.clone(),
                            port: entry.clone(),
                        })
                })
                .collect()
        };

        stages.push(Stage {
            title: scalar(record, "title"),
            executor_kind,
            responsibility_tier,
            inputs: ports("inputs")?,
            outputs: ports("outputs")?,
            executor_ref: scalar(record, "executor_ref").filter(|v| !v.trim().is_empty()),
            id,
        });
    }

    let graph = MilestoneGraph { milestones, stages };
    validate_references(&graph)?;
    validate_acyclic(&graph)?;
    Ok(graph)
}

/// Dangling `stage_refs` and `depends_on` fail closed: a plan referencing work
/// nobody defined is a plan with a hole in it.
fn validate_references(graph: &MilestoneGraph) -> Result<(), MilestoneError> {
    let stage_ids: BTreeSet<&str> = graph.stages.iter().map(|s| s.id.as_str()).collect();
    let milestone_ids: BTreeSet<&str> = graph.milestones.iter().map(|m| m.id.as_str()).collect();

    for m in &graph.milestones {
        for stage in &m.stage_refs {
            if !stage_ids.contains(stage.as_str()) {
                return Err(MilestoneError::DanglingStageRef {
                    milestone: m.id.clone(),
                    stage: stage.clone(),
                });
            }
        }
        for dep in &m.depends_on {
            if !milestone_ids.contains(dep.as_str()) {
                return Err(MilestoneError::DanglingDependency {
                    milestone: m.id.clone(),
                    dependency: dep.clone(),
                });
            }
        }
    }
    Ok(())
}

/// Three-colour DFS over `depends_on` — the same algorithm as the Warrant parent
/// graph and the ADR supersession graph, deliberately, so all three agree.
fn validate_acyclic(graph: &MilestoneGraph) -> Result<(), MilestoneError> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Unvisited,
        InProgress,
        Done,
    }

    let deps: BTreeMap<&str, &[String]> = graph
        .milestones
        .iter()
        .map(|m| (m.id.as_str(), m.depends_on.as_slice()))
        .collect();
    let mut mark: BTreeMap<&str, Mark> = deps.keys().map(|k| (*k, Mark::Unvisited)).collect();

    fn visit<'a>(
        id: &'a str,
        deps: &BTreeMap<&'a str, &'a [String]>,
        mark: &mut BTreeMap<&'a str, Mark>,
        path: &mut Vec<&'a str>,
    ) -> Result<(), MilestoneError> {
        match mark.get(id) {
            None | Some(Mark::Done) => return Ok(()),
            Some(Mark::InProgress) => {
                let start = path.iter().position(|p| *p == id).unwrap_or(0);
                let mut cycle: Vec<&str> = path[start..].to_vec();
                cycle.push(id);
                return Err(MilestoneError::DependencyCycle {
                    path: cycle.join(" → "),
                });
            }
            Some(Mark::Unvisited) => {}
        }
        mark.insert(id, Mark::InProgress);
        path.push(id);
        if let Some(children) = deps.get(id) {
            for child in children.iter() {
                // Borrow the key already owned by `deps` so lifetimes line up.
                if let Some((key, _)) = deps.get_key_value(child.as_str()) {
                    visit(key, deps, mark, path)?;
                }
            }
        }
        path.pop();
        mark.insert(id, Mark::Done);
        Ok(())
    }

    let ids: Vec<&str> = deps.keys().copied().collect();
    let mut path = Vec::new();
    for id in ids {
        visit(id, &deps, &mut mark, &mut path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"schema: "oh.war/milestones/v1"

milestones:
  - id: "M1"
    title: "First"
    stage_refs: ["STAGE-001"]
    obligation_refs: ["OBL-001"]
  - id: "M2"
    title: "Second"
    depends_on: ["M1"]
    stage_refs: ["STAGE-002"]

stages:
  - id: "STAGE-001"
    title: "Do"
    executor_kind: "human"
    responsibility_tier: "T2"
  - id: "STAGE-002"
    title: "Check"
    executor_kind: "service"
    responsibility_tier: "T1"
"#;

    #[test]
    fn parses_a_valid_graph() {
        let g = parse(GOOD).expect("valid");
        assert_eq!(g.milestones.len(), 2);
        assert_eq!(g.stages.len(), 2);
        assert_eq!(g.stages[0].executor_kind, ExecutorKind::Human);
        assert_eq!(g.stages[1].responsibility_tier, ResponsibilityTier::T1);
        assert_eq!(
            g.referenced_obligations().into_iter().collect::<Vec<_>>(),
            vec!["OBL-001"]
        );
    }

    /// The whole corpus is the acceptance test (OW-WAR-0007 OBL-003).
    #[test]
    fn every_milestone_graph_in_this_repository_validates() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/warrants");
        let mut checked = 0;
        for entry in std::fs::read_dir(dir).expect("warrants") {
            let path = entry
                .expect("entry")
                .path()
                .join("atoms/45-milestones.yaml");
            if !path.is_file() {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("readable");
            parse(&text).unwrap_or_else(|e| panic!("{} invalid: {e}", path.display()));
            checked += 1;
        }
        assert!(
            checked >= 40,
            "expected the whole corpus, checked {checked}"
        );
    }

    // --- planted violations (OW-WAR-0007 OBL-002) ---

    #[test]
    fn a_dangling_stage_ref_is_refused() {
        let bad = GOOD.replace(
            "\"STAGE-001\"]\n    obligation",
            "\"STAGE-999\"]\n    obligation",
        );
        assert_eq!(
            parse(&bad),
            Err(MilestoneError::DanglingStageRef {
                milestone: "M1".to_owned(),
                stage: "STAGE-999".to_owned()
            })
        );
    }

    #[test]
    fn a_dangling_dependency_is_refused() {
        let bad = GOOD.replace("depends_on: [\"M1\"]", "depends_on: [\"M99\"]");
        assert_eq!(
            parse(&bad),
            Err(MilestoneError::DanglingDependency {
                milestone: "M2".to_owned(),
                dependency: "M99".to_owned()
            })
        );
    }

    #[test]
    fn duplicate_ids_are_refused() {
        let bad = GOOD.replace("- id: \"M2\"", "- id: \"M1\"");
        assert_eq!(
            parse(&bad),
            Err(MilestoneError::DuplicateId {
                kind: "milestone",
                id: "M1".to_owned()
            })
        );
        let bad = GOOD.replace("- id: \"STAGE-002\"", "- id: \"STAGE-001\"");
        assert!(matches!(
            parse(&bad),
            Err(MilestoneError::DuplicateId { kind: "stage", .. })
        ));
    }

    #[test]
    fn a_dependency_cycle_is_refused() {
        let bad = GOOD.replace(
            "  - id: \"M1\"\n    title: \"First\"",
            "  - id: \"M1\"\n    depends_on: [\"M2\"]\n    title: \"First\"",
        );
        match parse(&bad) {
            Err(MilestoneError::DependencyCycle { path }) => {
                assert!(path.contains("M1") && path.contains("M2"), "got {path}");
            }
            other => panic!("expected a cycle, got {other:?}"),
        }
    }

    /// §23.6: a milestone is not a stage.
    #[test]
    fn a_milestone_carrying_a_stage_field_is_refused() {
        let bad = GOOD.replace(
            "  - id: \"M1\"\n    title: \"First\"",
            "  - id: \"M1\"\n    executor_kind: \"human\"\n    title: \"First\"",
        );
        assert_eq!(
            parse(&bad),
            Err(MilestoneError::MilestoneHasStageField {
                id: "M1".to_owned(),
                field: "executor_kind".to_owned()
            })
        );
    }

    #[test]
    fn a_stage_carrying_a_milestone_field_is_refused() {
        let bad = GOOD.replace(
            "  - id: \"STAGE-001\"\n    title: \"Do\"",
            "  - id: \"STAGE-001\"\n    obligation_refs: [\"OBL-001\"]\n    title: \"Do\"",
        );
        assert_eq!(
            parse(&bad),
            Err(MilestoneError::StageHasMilestoneField {
                id: "STAGE-001".to_owned(),
                field: "obligation_refs".to_owned()
            })
        );
    }

    #[test]
    fn unknown_executor_kinds_and_tiers_are_refused() {
        let bad = GOOD.replace("executor_kind: \"human\"", "executor_kind: \"wizard\"");
        assert!(matches!(
            parse(&bad),
            Err(MilestoneError::UnknownExecutorKind { .. })
        ));
        let bad = GOOD.replace("responsibility_tier: \"T2\"", "responsibility_tier: \"T9\"");
        assert!(matches!(
            parse(&bad),
            Err(MilestoneError::UnknownTier { .. })
        ));
    }

    #[test]
    fn an_unknown_schema_is_refused() {
        let bad = GOOD.replace("oh.war/milestones/v1", "oh.war/milestones/v2");
        assert!(matches!(
            parse(&bad),
            Err(MilestoneError::UnknownSchema { .. })
        ));
    }

    /// §23.5: a port without a type is refused.
    #[test]
    fn an_untyped_port_is_refused() {
        let bad = GOOD.replace(
            "    executor_kind: \"human\"",
            "    inputs: [\"corpus\"]\n    executor_kind: \"human\"",
        );
        assert_eq!(
            parse(&bad),
            Err(MilestoneError::UntypedPort {
                id: "STAGE-001".to_owned(),
                port: "corpus".to_owned()
            })
        );
    }

    #[test]
    fn typed_ports_parse() {
        let good = GOOD.replace(
            "    executor_kind: \"human\"",
            "    inputs: [\"corpus:war/corpus\"]\n    outputs: [\"report:war/report\"]\n    executor_kind: \"human\"",
        );
        let g = parse(&good).expect("valid");
        assert_eq!(g.stages[0].inputs[0].name, "corpus");
        assert_eq!(g.stages[0].inputs[0].type_name, "war/corpus");
        assert_eq!(g.stages[0].outputs[0].name, "report");
    }

    /// §26.5: tier and executor kind are orthogonal — neither may be inferred
    /// from the other.
    #[test]
    fn tier_and_executor_kind_are_orthogonal() {
        let g = parse(GOOD).expect("valid");
        let human_t2 = &g.stages[0];
        let service_t1 = &g.stages[1];
        assert_eq!(human_t2.executor_kind, ExecutorKind::Human);
        assert_eq!(human_t2.responsibility_tier, ResponsibilityTier::T2);
        assert_eq!(service_t1.executor_kind, ExecutorKind::Service);
        assert_eq!(service_t1.responsibility_tier, ResponsibilityTier::T1);
    }
}
