// SPDX-License-Identifier: Apache-2.0
//! The roadmap record: one per program, beside the SAS (OW-ADR-0023).
//!
//! The SAS says what the system is; the roadmap says in what order it is
//! built — sequencing, dependency, priority and phase (SAS §6.3). It is a
//! Warrant's milestones graph one level up: its milestones are the
//! program's phases and its stages are Warrants.
//!
//! What it deliberately does NOT hold: a member list, or a status. A
//! Warrant joins a phase through its own `[[roadmap]]` ref, and a phase is
//! achieved when its `exit` Warrant resolves satisfied — both computed from
//! the records, the way currency is (OW-ADR-0022). A second place to write
//! either fact would be the one that drifts.
//!
//! On disk (`docs/roadmap/` by default):
//!
//! ```text
//! roadmap.toml            oh.war/roadmap/v1 — uuid, program, prefix, atoms
//! atoms/10-intent.md      authored: why the program, the outcome of the whole
//! atoms/20-phases.yaml    authored, typed: tiers and phases
//! revisions/<n>.toml      oh.war/roadmap-revision/v1 — accepted revisions
//! ```
//!
//! The phases atom is read by the restricted structured reader
//! (OW-ADR-0003), the same one milestones use: no YAML library, no implicit
//! typing, every value a quoted scalar or a flow list of them.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::sas::{SasAcceptance, SasRevisionState};
use crate::structured::{self, StructuredValue};

pub const ROADMAP_SCHEMA: &str = "oh.war/roadmap/v1";
pub const PHASES_SCHEMA: &str = "oh.war/roadmap-phases/v1";
pub const ROADMAP_REVISION_SCHEMA: &str = "oh.war/roadmap-revision/v1";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RoadmapError {
    #[error("the phases atom could not be read: {0}")]
    Structured(String),
    #[error("the phases atom declares schema {found:?}; expected {PHASES_SCHEMA:?}")]
    Schema { found: String },
    #[error("phase {index} has no `id`")]
    MissingId { index: usize },
    #[error("phase id {id:?} is not `<PREFIX>-PHASE-<N>` with the roadmap's prefix {prefix:?}")]
    BadId { id: String, prefix: String },
    #[error("phase {id} is declared twice; a phase id is its identity")]
    Duplicate { id: String },
    #[error("phase {id} depends on {missing}, which the roadmap does not declare")]
    UnknownDependency { id: String, missing: String },
    #[error(
        "phase dependencies form a cycle through {through}; a roadmap orders work, and a cycle orders nothing"
    )]
    Cycle { through: String },
    #[error("phase {id} names priority {priority:?}, which is not one of the declared tiers")]
    UnknownTier { id: String, priority: String },
}

/// `roadmap.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoadmapManifest {
    pub schema: String,
    pub uuid: String,
    pub program: String,
    /// The `<PREFIX>` of every `roadmap://<PREFIX>-PHASE-<N>` ref.
    pub prefix: String,
    #[serde(default)]
    pub atoms: Vec<RoadmapAtomRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoadmapAtomRef {
    pub ordinal: u32,
    pub role: String,
    pub path: String,
}

/// One release tier: a phase's priority names one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tier {
    pub id: String,
    pub title: String,
}

/// One phase: a milestone of the program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Phase {
    /// `<PREFIX>-PHASE-<N>`.
    pub id: String,
    /// `N`, parsed from the id.
    pub number: u8,
    pub title: String,
    pub outcome: String,
    /// The criterion, as text. Carried, never evaluated: achievement is the
    /// `exit` Warrant's recorded resolution.
    pub exit: String,
    pub depends_on: Vec<String>,
    /// A tier id, when the roadmap declares tiers.
    pub priority: Option<String>,
    /// Slugs placed in this phase that no Warrant carries yet.
    pub open: Vec<String>,
}

/// The phases atom, parsed and validated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Phases {
    pub tiers: Vec<Tier>,
    pub phases: Vec<Phase>,
}

impl Phases {
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Phase> {
        self.phases.iter().find(|p| p.id == id)
    }

    #[must_use]
    pub fn by_number(&self, n: u8) -> Option<&Phase> {
        self.phases.iter().find(|p| p.number == n)
    }

    /// Phases in dependency order (a phase after everything it depends on),
    /// ties by number. Validation has already refused a cycle.
    #[must_use]
    pub fn in_dependency_order(&self) -> Vec<&Phase> {
        let mut placed: BTreeSet<&str> = BTreeSet::new();
        let mut out = Vec::new();
        while out.len() < self.phases.len() {
            let before = out.len();
            for p in &self.phases {
                if !placed.contains(p.id.as_str())
                    && p.depends_on.iter().all(|d| placed.contains(d.as_str()))
                {
                    placed.insert(&p.id);
                    out.push(p);
                }
            }
            if out.len() == before {
                break; // unreachable after validation; never loop forever
            }
        }
        out
    }
}

fn scalar(rec: &BTreeMap<String, StructuredValue>, key: &str) -> Option<String> {
    rec.get(key)
        .and_then(StructuredValue::as_scalar)
        .map(str::to_owned)
}

fn list(rec: &BTreeMap<String, StructuredValue>, key: &str) -> Vec<String> {
    rec.get(key)
        .and_then(StructuredValue::as_list)
        .map(<[String]>::to_vec)
        .unwrap_or_default()
}

/// The `N` of `<prefix>-PHASE-<N>`, under the same rule `RoadmapRef` uses:
/// decimal, no leading zero.
fn phase_number(id: &str, prefix: &str) -> Option<u8> {
    let n = id.strip_prefix(prefix)?.strip_prefix("-PHASE-")?;
    if n.is_empty() || (n.len() > 1 && n.starts_with('0')) || !n.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    n.parse().ok()
}

/// Parse and validate a phases atom for a roadmap whose prefix is `prefix`.
pub fn parse_phases(source: &str, prefix: &str) -> Result<Phases, RoadmapError> {
    let doc = structured::parse(source).map_err(|e| RoadmapError::Structured(e.to_string()))?;
    match doc.scalar("schema") {
        Some(PHASES_SCHEMA) => {}
        other => {
            return Err(RoadmapError::Schema {
                found: other.unwrap_or("").to_owned(),
            });
        }
    }
    let tiers: Vec<Tier> = doc
        .records("tiers")
        .unwrap_or_default()
        .iter()
        .filter_map(|r| {
            Some(Tier {
                id: scalar(r, "id")?,
                title: scalar(r, "title").unwrap_or_default(),
            })
        })
        .collect();
    let mut phases = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, r) in doc.records("phases").unwrap_or_default().iter().enumerate() {
        let id = scalar(r, "id").ok_or(RoadmapError::MissingId { index })?;
        let number = phase_number(&id, prefix).ok_or_else(|| RoadmapError::BadId {
            id: id.clone(),
            prefix: prefix.to_owned(),
        })?;
        if !seen.insert(id.clone()) {
            return Err(RoadmapError::Duplicate { id });
        }
        let priority = scalar(r, "priority");
        if let Some(p) = &priority
            && !tiers.is_empty()
            && !tiers.iter().any(|t| &t.id == p)
        {
            return Err(RoadmapError::UnknownTier {
                id,
                priority: p.clone(),
            });
        }
        phases.push(Phase {
            number,
            title: scalar(r, "title").unwrap_or_default(),
            outcome: scalar(r, "outcome").unwrap_or_default(),
            exit: scalar(r, "exit").unwrap_or_default(),
            depends_on: list(r, "depends_on"),
            priority,
            open: list(r, "open"),
            id,
        });
    }
    for p in &phases {
        for d in &p.depends_on {
            if !seen.contains(d) {
                return Err(RoadmapError::UnknownDependency {
                    id: p.id.clone(),
                    missing: d.clone(),
                });
            }
        }
    }
    let parsed = Phases { tiers, phases };
    if parsed.in_dependency_order().len() != parsed.phases.len() {
        let placed: BTreeSet<&str> = parsed
            .in_dependency_order()
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        let through = parsed
            .phases
            .iter()
            .filter(|p| !placed.contains(p.id.as_str()))
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(RoadmapError::Cycle { through });
    }
    Ok(parsed)
}

/// One revision of the roadmap: the digest of its manifest and atoms, and
/// — once a human accepts it — who, as what, meaning what.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoadmapRevision {
    pub schema: String,
    pub revision: u32,
    /// sha256 over the manifest and every atom, as `roadmap_digest` computes.
    pub sha256: String,
    pub state: SasRevisionState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predecessor: Option<u32>,
    /// The phases as they stood — id → title — so two revisions can be
    /// compared from records alone.
    #[serde(default)]
    pub phases: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<SasAcceptance>,
}

/// A phase-level diff between two revisions' phase maps.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub retitled: Vec<(String, String, String)>,
}

impl PhaseDiff {
    #[must_use]
    pub fn between(before: &BTreeMap<String, String>, after: &BTreeMap<String, String>) -> Self {
        let mut d = Self::default();
        for (id, t) in after {
            match before.get(id) {
                None => d.added.push(format!("{id} {t}")),
                Some(b) if b != t => d.retitled.push((id.clone(), b.clone(), t.clone())),
                Some(_) => {}
            }
        }
        for (id, t) in before {
            if !after.contains_key(id) {
                d.removed.push(format!("{id} {t}"));
            }
        }
        d
    }

    #[must_use]
    pub fn summary(&self) -> String {
        if self.added.is_empty() && self.removed.is_empty() && self.retitled.is_empty() {
            return "no phase added, removed or retitled (exits, order, priority or open work may differ)".to_owned();
        }
        let mut parts = Vec::new();
        if !self.added.is_empty() {
            parts.push(format!("+{} ({})", self.added.len(), self.added.join("; ")));
        }
        if !self.removed.is_empty() {
            parts.push(format!(
                "−{} ({})",
                self.removed.len(),
                self.removed.join("; ")
            ));
        }
        for (id, b, a) in &self.retitled {
            parts.push(format!("~{id}: {b:?} → {a:?}"));
        }
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"schema: "oh.war/roadmap-phases/v1"

tiers:
  - id: "1"
    title: "Library"
  - id: "2"
    title: "Integration"

phases:
  - id: "OW-PHASE-0"
    title: "Telemetry"
    exit: "numbers exist"
    depends_on: []
    priority: "2"
    open: ["friction-baseline"]
  - id: "OW-PHASE-1"
    title: "Compiler"
    exit: "we use it"
    depends_on: ["OW-PHASE-0"]
    priority: "1"
"#;

    #[test]
    fn a_sound_atom_parses_in_dependency_order() {
        let p = parse_phases(GOOD, "OW").unwrap();
        assert_eq!(p.phases.len(), 2);
        assert_eq!(p.by_number(1).unwrap().depends_on, ["OW-PHASE-0"]);
        assert_eq!(p.get("OW-PHASE-0").unwrap().open, ["friction-baseline"]);
        let order: Vec<&str> = p
            .in_dependency_order()
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(order, ["OW-PHASE-0", "OW-PHASE-1"]);
    }

    #[test]
    fn it_refuses_by_name() {
        let dup = GOOD.replace("OW-PHASE-1\"\n    title", "OW-PHASE-0\"\n    title");
        assert!(matches!(
            parse_phases(&dup, "OW"),
            Err(RoadmapError::Duplicate { .. })
        ));
        let unknown = GOOD.replace(
            "depends_on: [\"OW-PHASE-0\"]",
            "depends_on: [\"OW-PHASE-7\"]",
        );
        assert!(matches!(
            parse_phases(&unknown, "OW"),
            Err(RoadmapError::UnknownDependency { .. })
        ));
        let cycle = GOOD.replace("depends_on: []", "depends_on: [\"OW-PHASE-1\"]");
        assert!(matches!(
            parse_phases(&cycle, "OW"),
            Err(RoadmapError::Cycle { .. })
        ));
        assert!(matches!(
            parse_phases(GOOD, "XX"),
            Err(RoadmapError::BadId { .. })
        ));
        let tier = GOOD.replace("priority: \"2\"", "priority: \"9\"");
        assert!(matches!(
            parse_phases(&tier, "OW"),
            Err(RoadmapError::UnknownTier { .. })
        ));
        let padded = GOOD.replace("OW-PHASE-1\"", "OW-PHASE-01\"");
        assert!(matches!(
            parse_phases(&padded, "OW"),
            Err(RoadmapError::BadId { .. })
        ));
    }

    #[test]
    fn the_diff_names_what_moved() {
        let a: BTreeMap<String, String> =
            [("P-0".into(), "A".into()), ("P-1".into(), "B".into())].into();
        let b: BTreeMap<String, String> =
            [("P-1".into(), "B2".into()), ("P-2".into(), "C".into())].into();
        let d = PhaseDiff::between(&a, &b);
        assert_eq!(d.added, ["P-2 C"]);
        assert_eq!(d.removed, ["P-0 A"]);
        assert_eq!(d.retitled, [("P-1".into(), "B".into(), "B2".into())]);
    }
}
