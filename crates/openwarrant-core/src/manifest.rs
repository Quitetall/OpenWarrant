// SPDX-License-Identifier: AGPL-3.0-or-later
//! The WAR manifest — composition and relations (SAS §61).
//!
//! §61.1: "The manifest defines composition and relations. It does not duplicate
//! the full semantic content of atoms." Everything substantive lives in the atoms
//! this points at.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::identity::{IdentityError, LocalAlias, WarUuid};
use crate::role::{AtomRole, Profile, RoleError, is_namespaced_extension_role};

/// The only manifest schema this build understands.
pub const MANIFEST_SCHEMA: &str = "oh.war/manifest/v1";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error(
        "unknown manifest schema {found:?}; this build understands {expected:?} \
         (SAS §69.3 — a breaking protocol change is not silently accepted)"
    )]
    UnknownSchema { found: String, expected: String },
    #[error("title is empty")]
    TitleEmpty,
    #[error(transparent)]
    Identity(#[from] IdentityError),
    #[error(transparent)]
    Role(#[from] RoleError),
    #[error("unknown assurance level {found:?}; expected basic, controlled, or high (SAS §25)")]
    UnknownAssuranceLevel { found: String },
    #[error(
        "duplicate atom ordinal {ordinal} (SAS §61.2 — ordinals are unique within a composition)"
    )]
    DuplicateOrdinal { ordinal: u32 },
    #[error(
        "profile {profile} requires a {role} atom and the manifest declares none \
         (SAS §16.3, §91.2 test 7 — required atom omission fails closed)"
    )]
    MissingRequiredRole { profile: Profile, role: AtomRole },
    #[error(
        "atom at ordinal {ordinal} declares role {role:?}, which is neither a core role \
         nor a namespaced optional extension (SAS §16.4 — unknown required roles fail closed)"
    )]
    UnknownRequiredRole { ordinal: u32, role: String },
    #[error(
        "atom at ordinal {ordinal} declares a compiler-produced role {role}; a manifest \
         declares SOURCES, and a generated section has none (SAS §13.3)"
    )]
    GeneratedRoleDeclared { ordinal: u32, role: AtomRole },
    #[error("atom at ordinal {ordinal} declares neither `path` nor `ref`")]
    AtomWithoutSource { ordinal: u32 },
    #[error("atom at ordinal {ordinal} declares both `path` and `ref`; exactly one is the source")]
    AtomWithBothSources { ordinal: u32 },
    #[error(
        "enterprise_id {found:?} was set locally. Only Knowledge Fabric may allocate one \
         (SAS §12.4, §91.3 test 20 — an official identifier cannot be fabricated locally)"
    )]
    FabricatedEnterpriseId { found: String },
    #[error(
        "parent {index} references {reference:?} but declares no contract_revision (SAS §20.2)"
    )]
    ParentWithoutRevision { index: usize, reference: String },
}

/// Assurance level (SAS §25).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssuranceLevel {
    Basic,
    Controlled,
    High,
}

impl AssuranceLevel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Controlled => "controlled",
            Self::High => "high",
        }
    }

    /// Whether §39.4 requires a contract-adequacy review at this level.
    #[must_use]
    pub const fn requires_adequacy_review(self) -> bool {
        match self {
            Self::Basic => false,
            Self::Controlled | Self::High => true,
        }
    }
}

impl FromStr for AssuranceLevel {
    type Err = ManifestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "basic" => Ok(Self::Basic),
            "controlled" => Ok(Self::Controlled),
            "high" => Ok(Self::High),
            other => Err(ManifestError::UnknownAssuranceLevel {
                found: other.to_owned(),
            }),
        }
    }
}

impl std::fmt::Display for AssuranceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One atom entry in a composition (SAS §61).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtomEntry {
    pub ordinal: u32,
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    #[serde(default = "AtomEntry::default_required")]
    pub required: bool,
}

impl AtomEntry {
    fn default_required() -> bool {
        true
    }

    /// The parsed core role, or `None` for a namespaced optional extension.
    pub fn core_role(&self) -> Result<Option<AtomRole>, ManifestError> {
        match AtomRole::from_str(&self.role) {
            Ok(role) => Ok(Some(role)),
            Err(_) if is_namespaced_extension_role(&self.role) && !self.required => Ok(None),
            Err(_) => Err(ManifestError::UnknownRequiredRole {
                ordinal: self.ordinal,
                role: self.role.clone(),
            }),
        }
    }
}

/// A SAS requirement this Warrant implements (SAS §34.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Implements {
    pub r#ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contribution: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoadmapRef {
    pub r#ref: String,
}

/// A parent relation (SAS §20.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParentRef {
    pub r#ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_revision: Option<u32>,
    /// §20.2 also requires a `contract_digest`. It is optional here because
    /// digesting does not exist until OW-WAR-0003, and a placeholder digest that
    /// looks real is worse than an absent one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_digest: Option<String>,
}

/// A WAR manifest (SAS §61).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: String,
    pub uuid: String,
    pub local_alias: String,
    #[serde(default)]
    pub enterprise_id: String,
    pub title: String,
    pub profile: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assurance_level: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implements: Vec<Implements>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roadmap: Vec<RoadmapRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parents: Vec<ParentRef>,
    #[serde(default)]
    pub atoms: Vec<AtomEntry>,
}

/// A manifest whose invariants have been checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedManifest {
    pub uuid: WarUuid,
    pub alias: LocalAlias,
    pub profile: Profile,
    pub assurance_level: AssuranceLevel,
    pub raw: Manifest,
}

impl Manifest {
    /// Validate every invariant this crate can check without touching the
    /// filesystem. Path existence is the caller's job (§79.1).
    ///
    /// `namespace`, when supplied, additionally requires the alias to belong to
    /// this repository.
    pub fn validate(&self, namespace: Option<&str>) -> Result<ValidatedManifest, ManifestError> {
        if self.schema != MANIFEST_SCHEMA {
            return Err(ManifestError::UnknownSchema {
                found: self.schema.clone(),
                expected: MANIFEST_SCHEMA.to_owned(),
            });
        }
        if self.title.trim().is_empty() {
            return Err(ManifestError::TitleEmpty);
        }

        let uuid = WarUuid::from_str(&self.uuid)?;
        let alias = match namespace {
            Some(ns) => LocalAlias::parse_in(&self.local_alias, ns)?,
            None => LocalAlias::parse(&self.local_alias)?,
        };

        // §91.3 test 20. An empty string means "not yet allocated", which is
        // the only value a repository may write for itself.
        if !self.enterprise_id.trim().is_empty() {
            return Err(ManifestError::FabricatedEnterpriseId {
                found: self.enterprise_id.clone(),
            });
        }

        let profile = Profile::from_str(&self.profile)?;
        let assurance_level = match &self.assurance_level {
            Some(level) => AssuranceLevel::from_str(level)?,
            None => AssuranceLevel::Basic,
        };

        // §91.2 test 8: duplicate ordinals fail.
        let mut ordinals = BTreeSet::new();
        for atom in &self.atoms {
            if !ordinals.insert(atom.ordinal) {
                return Err(ManifestError::DuplicateOrdinal {
                    ordinal: atom.ordinal,
                });
            }
        }

        // §91.2 test 9 (via core_role), plus source-shape checks.
        let mut present_roles = BTreeSet::new();
        for atom in &self.atoms {
            match (&atom.path, &atom.r#ref) {
                (None, None) => {
                    return Err(ManifestError::AtomWithoutSource {
                        ordinal: atom.ordinal,
                    });
                }
                (Some(_), Some(_)) => {
                    return Err(ManifestError::AtomWithBothSources {
                        ordinal: atom.ordinal,
                    });
                }
                _ => {}
            }
            if let Some(role) = atom.core_role()? {
                if role.is_compiler_produced() {
                    return Err(ManifestError::GeneratedRoleDeclared {
                        ordinal: atom.ordinal,
                        role,
                    });
                }
                present_roles.insert(role);
            }
        }

        // §91.2 test 7: a required atom omission fails closed.
        for role in profile.required_authored_roles() {
            if !present_roles.contains(&role) {
                return Err(ManifestError::MissingRequiredRole { profile, role });
            }
        }

        for (index, parent) in self.parents.iter().enumerate() {
            if parent.contract_revision.is_none() {
                return Err(ManifestError::ParentWithoutRevision {
                    index,
                    reference: parent.r#ref.clone(),
                });
            }
        }

        Ok(ValidatedManifest {
            uuid,
            alias,
            profile,
            assurance_level,
            raw: self.clone(),
        })
    }
}

/// A composition cycle found across a corpus (SAS §91.2 test 12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentCycle {
    /// Aliases on the cycle, in traversal order.
    pub path: Vec<String>,
}

/// Detect cycles in the parent graph of a corpus.
///
/// A Warrant that is transitively its own parent has no originating rationale —
/// §20.1 says a parent "preserves the originating context ... from which child
/// work is decomposed", and a cycle means the decomposition has no root.
/// Returns every distinct cycle found, not just the first.
#[must_use]
pub fn detect_parent_cycles(manifests: &[ValidatedManifest]) -> Vec<ParentCycle> {
    // Parent refs are `war://<uuid>`; index by uuid so lookups are exact.
    let by_uuid: BTreeMap<String, &ValidatedManifest> =
        manifests.iter().map(|m| (m.uuid.to_string(), m)).collect();

    let mut mark: BTreeMap<String, Mark> = by_uuid
        .keys()
        .map(|uuid| (uuid.clone(), Mark::Unvisited))
        .collect();
    let mut cycles = Vec::new();
    let mut seen: BTreeSet<Vec<String>> = BTreeSet::new();
    let mut path: Vec<String> = Vec::new();

    // Visit in a deterministic order so the reported cycle path does not depend
    // on the order manifests happened to be loaded in.
    for uuid in by_uuid.keys() {
        visit(uuid, &by_uuid, &mut mark, &mut path, &mut cycles, &mut seen);
    }

    cycles
}

/// Three-colour DFS marking (Cormen et al.): the standard way to find a back
/// edge, which is exactly what a cycle is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    Unvisited,
    InProgress,
    Done,
}

/// Parent UUIDs of one manifest, in declaration order.
fn parent_uuids(m: &ValidatedManifest) -> Vec<String> {
    m.raw
        .parents
        .iter()
        .filter_map(|p| p.r#ref.strip_prefix("war://").map(str::to_owned))
        .collect()
}

/// Recursion depth is bounded by the corpus size — tens of Warrants, not a
/// data-dependent depth — so recursion is safe here and reads better than an
/// explicit stack carrying its own path.
fn visit(
    uuid: &str,
    by_uuid: &BTreeMap<String, &ValidatedManifest>,
    mark: &mut BTreeMap<String, Mark>,
    path: &mut Vec<String>,
    cycles: &mut Vec<ParentCycle>,
    seen: &mut BTreeSet<Vec<String>>,
) {
    match mark.get(uuid) {
        // A parent outside this corpus is absent from `mark`. It cannot close a
        // cycle here, and reporting one would make every child of an unloaded
        // Warrant look broken.
        None => return,
        Some(Mark::Done) => return,
        Some(Mark::InProgress) => {
            // Back edge: `uuid` is on the current path, so the cycle is the
            // path from its first occurrence onward.
            let start = path.iter().position(|u| u == uuid).unwrap_or(0);
            let mut aliases: Vec<String> = path[start..]
                .iter()
                .filter_map(|u| by_uuid.get(u).map(|m| m.alias.to_string()))
                .collect();
            if let Some(alias) = by_uuid.get(uuid).map(|m| m.alias.to_string()) {
                aliases.push(alias);
            }
            // Dedup key: the same cycle reached from two different roots is one
            // cycle, and reporting it twice would inflate the count.
            let mut key = aliases.clone();
            key.sort_unstable();
            key.dedup();
            if seen.insert(key) {
                cycles.push(ParentCycle { path: aliases });
            }
            return;
        }
        Some(Mark::Unvisited) => {}
    }

    mark.insert(uuid.to_owned(), Mark::InProgress);
    path.push(uuid.to_owned());
    if let Some(node) = by_uuid.get(uuid) {
        for parent in parent_uuids(node) {
            visit(&parent, by_uuid, mark, path, cycles, seen);
        }
    }
    path.pop();
    mark.insert(uuid.to_owned(), Mark::Done);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(ordinal: u32, role: &str) -> AtomEntry {
        AtomEntry {
            ordinal,
            role: role.to_owned(),
            path: Some(format!("atoms/{ordinal}-{role}.md")),
            r#ref: None,
            required: true,
        }
    }

    fn delivery() -> Manifest {
        Manifest {
            schema: MANIFEST_SCHEMA.to_owned(),
            uuid: "01a018db-19fc-7f2a-8e39-69730f255e33".to_owned(),
            local_alias: "OW-WAR-0001".to_owned(),
            enterprise_id: String::new(),
            title: "A delivery warrant".to_owned(),
            profile: "delivery".to_owned(),
            assurance_level: Some("basic".to_owned()),
            implements: vec![],
            roadmap: vec![],
            parents: vec![],
            atoms: vec![
                atom(10, "intent"),
                atom(20, "basis"),
                atom(40, "work_order"),
                atom(45, "milestones"),
                atom(60, "assurance"),
            ],
        }
    }

    #[test]
    fn a_well_formed_delivery_manifest_validates() {
        let v = delivery().validate(Some("OW")).expect("valid");
        assert_eq!(v.profile, Profile::Delivery);
        assert_eq!(v.assurance_level, AssuranceLevel::Basic);
        assert_eq!(v.alias.as_str(), "OW-WAR-0001");
    }

    #[test]
    fn assurance_level_defaults_to_basic_and_drives_adequacy() {
        let mut m = delivery();
        m.assurance_level = None;
        assert_eq!(
            m.validate(None).expect("valid").assurance_level,
            AssuranceLevel::Basic
        );
        assert!(!AssuranceLevel::Basic.requires_adequacy_review());
        assert!(AssuranceLevel::Controlled.requires_adequacy_review());
        assert!(AssuranceLevel::High.requires_adequacy_review());
    }

    // --- Planted violations (SAS §91.2). ---

    /// Test 7: required atom omission fails closed.
    #[test]
    fn missing_required_atom_is_refused() {
        let mut m = delivery();
        m.atoms.retain(|a| a.role != "milestones");
        assert_eq!(
            m.validate(None),
            Err(ManifestError::MissingRequiredRole {
                profile: Profile::Delivery,
                role: AtomRole::Milestones
            })
        );
    }

    /// Test 8: duplicate ordinal fails.
    #[test]
    fn duplicate_ordinal_is_refused() {
        let mut m = delivery();
        m.atoms.push(atom(20, "work_order"));
        assert_eq!(
            m.validate(None),
            Err(ManifestError::DuplicateOrdinal { ordinal: 20 })
        );
    }

    /// Test 9: unknown required role fails.
    #[test]
    fn unknown_required_role_is_refused() {
        let mut m = delivery();
        m.atoms.push(AtomEntry {
            ordinal: 55,
            role: "hypothesis".to_owned(),
            path: Some("atoms/55-hypothesis.md".to_owned()),
            r#ref: None,
            required: true,
        });
        assert_eq!(
            m.validate(None),
            Err(ManifestError::UnknownRequiredRole {
                ordinal: 55,
                role: "hypothesis".to_owned()
            })
        );
    }

    /// §16.4's other half: an unknown OPTIONAL NAMESPACED role is preserved,
    /// not refused. Collapsing these two behaviours is the bug this guards.
    #[test]
    fn unknown_optional_namespaced_role_is_preserved() {
        let mut m = delivery();
        m.atoms.push(AtomEntry {
            ordinal: 55,
            role: "lab.protocol".to_owned(),
            path: Some("atoms/55-lab.md".to_owned()),
            r#ref: None,
            required: false,
        });
        let v = m
            .validate(None)
            .expect("namespaced optional role is allowed");
        assert!(
            v.raw.atoms.iter().any(|a| a.role == "lab.protocol"),
            "the extension atom must survive validation"
        );
    }

    /// A namespaced role marked REQUIRED still fails: we cannot satisfy a
    /// requirement we do not understand.
    #[test]
    fn unknown_namespaced_role_marked_required_is_refused() {
        let mut m = delivery();
        m.atoms.push(AtomEntry {
            ordinal: 55,
            role: "lab.protocol".to_owned(),
            path: Some("atoms/55-lab.md".to_owned()),
            r#ref: None,
            required: true,
        });
        assert!(matches!(
            m.validate(None),
            Err(ManifestError::UnknownRequiredRole { .. })
        ));
    }

    #[test]
    fn declaring_a_generated_role_is_refused() {
        let mut m = delivery();
        m.atoms.push(atom(90, "relations_and_integrity"));
        assert_eq!(
            m.validate(None),
            Err(ManifestError::GeneratedRoleDeclared {
                ordinal: 90,
                role: AtomRole::RelationsAndIntegrity
            })
        );
    }

    /// §91.3 test 20: an enterprise ID cannot be fabricated locally.
    #[test]
    fn locally_set_enterprise_id_is_refused() {
        let mut m = delivery();
        m.enterprise_id = "OH-WAR-000042".to_owned();
        assert_eq!(
            m.validate(None),
            Err(ManifestError::FabricatedEnterpriseId {
                found: "OH-WAR-000042".to_owned()
            })
        );
    }

    #[test]
    fn unknown_schema_is_refused() {
        let mut m = delivery();
        m.schema = "oh.war/manifest/v2".to_owned();
        assert!(matches!(
            m.validate(None),
            Err(ManifestError::UnknownSchema { .. })
        ));
    }

    #[test]
    fn atom_source_shape_is_enforced() {
        let mut m = delivery();
        m.atoms[0].path = None;
        assert_eq!(
            m.validate(None),
            Err(ManifestError::AtomWithoutSource { ordinal: 10 })
        );

        let mut m = delivery();
        m.atoms[0].r#ref = Some("adr://something".to_owned());
        assert_eq!(
            m.validate(None),
            Err(ManifestError::AtomWithBothSources { ordinal: 10 })
        );
    }

    /// §20.2: a child must cite an exact parent contract revision.
    #[test]
    fn parent_without_contract_revision_is_refused() {
        let mut m = delivery();
        m.parents.push(ParentRef {
            r#ref: "war://01a018db-19fc-7f34-92db-54b2dca5446d".to_owned(),
            contract_revision: None,
            contract_digest: None,
        });
        assert!(matches!(
            m.validate(None),
            Err(ManifestError::ParentWithoutRevision { .. })
        ));
    }

    #[test]
    fn alias_from_another_namespace_is_refused() {
        let mut m = delivery();
        m.local_alias = "LIM-WAR-0001".to_owned();
        assert!(matches!(
            m.validate(Some("OW")),
            Err(ManifestError::Identity(
                IdentityError::AliasNamespaceMismatch { .. }
            ))
        ));
    }

    #[test]
    fn a_decision_profile_requires_an_adr_atom() {
        let mut m = delivery();
        m.profile = "decision".to_owned();
        // delivery atoms have no adr role
        assert_eq!(
            m.validate(None),
            Err(ManifestError::MissingRequiredRole {
                profile: Profile::Decision,
                role: AtomRole::Adr
            })
        );

        m.atoms.push(atom(30, "adr"));
        m.validate(None).expect("decision profile now satisfied");
    }

    // --- Composition cycles (SAS §91.2 test 12). ---

    fn with_parent(alias: &str, uuid: &str, parent_uuid: Option<&str>) -> ValidatedManifest {
        let mut m = delivery();
        m.local_alias = alias.to_owned();
        m.uuid = uuid.to_owned();
        if let Some(p) = parent_uuid {
            m.parents = vec![ParentRef {
                r#ref: format!("war://{p}"),
                contract_revision: Some(1),
                contract_digest: None,
            }];
        }
        m.validate(None).expect("valid")
    }

    const A: &str = "01a018db-19fc-7f2a-8e39-69730f255e33";
    const B: &str = "01a018db-19fc-7f34-92db-54b2dca5446d";
    const C: &str = "01a018db-19fc-72ba-87b3-c1bd1aec86a8";

    #[test]
    fn an_acyclic_corpus_has_no_cycles() {
        let corpus = vec![
            with_parent("OW-WAR-0001", A, None),
            with_parent("OW-WAR-0002", B, Some(A)),
            with_parent("OW-WAR-0003", C, Some(A)),
        ];
        assert_eq!(detect_parent_cycles(&corpus), vec![]);
    }

    #[test]
    fn a_direct_cycle_is_detected() {
        let corpus = vec![
            with_parent("OW-WAR-0001", A, Some(B)),
            with_parent("OW-WAR-0002", B, Some(A)),
        ];
        let cycles = detect_parent_cycles(&corpus);
        assert_eq!(cycles.len(), 1, "one distinct cycle, got {cycles:?}");
    }

    #[test]
    fn a_self_parent_is_detected() {
        let corpus = vec![with_parent("OW-WAR-0001", A, Some(A))];
        assert_eq!(detect_parent_cycles(&corpus).len(), 1);
    }

    #[test]
    fn a_longer_cycle_is_detected() {
        let corpus = vec![
            with_parent("OW-WAR-0001", A, Some(B)),
            with_parent("OW-WAR-0002", B, Some(C)),
            with_parent("OW-WAR-0003", C, Some(A)),
        ];
        assert_eq!(detect_parent_cycles(&corpus).len(), 1);
    }

    /// A parent outside the corpus is not a cycle and must not be reported as
    /// one — that would make every child of an unloaded Warrant look broken.
    #[test]
    fn an_external_parent_is_not_a_cycle() {
        let corpus = vec![with_parent("OW-WAR-0002", B, Some(A))];
        assert_eq!(detect_parent_cycles(&corpus), vec![]);
    }
}
