// SPDX-License-Identifier: AGPL-3.0-or-later
//! Atom roles, jurisdiction classes, and composition profiles (SAS §13, §16).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RoleError {
    #[error(
        "unknown atom role {found:?}; known roles are {known}. An unknown REQUIRED role \
         fails closed (SAS §16.4) — if this is an optional extension role, it must be \
         namespaced (contain a '.')"
    )]
    UnknownRole { found: String, known: String },
    #[error("unknown jurisdiction {found:?}; expected authored, bound, or generated (SAS §13)")]
    UnknownJurisdiction { found: String },
    #[error("unknown profile {found:?}; expected delivery or decision (SAS §16.3)")]
    UnknownProfile { found: String },
}

/// Who may change an atom (SAS §13).
///
/// The tri-state is the seam the whole jurisdiction law rests on, and collapsing
/// any two of them removes a guarantee:
///
/// - `authored` — a file in this repository, edited directly (§13.1).
/// - `bound` — owned by another authority and edited only there (§13.2). It may
///   be READ here; it may not be written here.
/// - `generated` — a projection, never edited at all (§13.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Jurisdiction {
    Authored,
    Bound,
    Generated,
}

impl Jurisdiction {
    /// Whether a direct edit to this atom in this repository is permitted.
    ///
    /// Exhaustive on purpose: a new class must be classified here before it
    /// compiles, because "may I write this?" is the question the class exists
    /// to answer.
    #[must_use]
    pub const fn is_directly_editable(self) -> bool {
        match self {
            Self::Authored => true,
            Self::Bound | Self::Generated => false,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Authored => "authored",
            Self::Bound => "bound",
            Self::Generated => "generated",
        }
    }
}

impl FromStr for Jurisdiction {
    type Err = RoleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "authored" => Ok(Self::Authored),
            "bound" => Ok(Self::Bound),
            "generated" => Ok(Self::Generated),
            other => Err(RoleError::UnknownJurisdiction {
                found: other.to_owned(),
            }),
        }
    }
}

impl fmt::Display for Jurisdiction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The core atom roles of SAS §16.1, in canonical order.
///
/// A note on naming, because the SAS uses two words for one thing and it would
/// otherwise look like a bug: §16.1's table names the *section* at ordinal 30
/// `decisions`, while §16.2's and §61's examples give the *atom* `role = "adr"`.
/// Both are right — a Decisions section is composed of ADR-role atoms — so the
/// role here is `Adr` and [`AtomRole::section_name`] reports `decisions`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AtomRole {
    Control,
    Intent,
    Basis,
    Adr,
    WorkOrder,
    Milestones,
    Execution,
    Assurance,
    Resolution,
    Validation,
    RelationsAndIntegrity,
}

impl AtomRole {
    /// Every core role, in the canonical order of §16.1.
    pub const ALL: [Self; 11] = [
        Self::Control,
        Self::Intent,
        Self::Basis,
        Self::Adr,
        Self::WorkOrder,
        Self::Milestones,
        Self::Execution,
        Self::Assurance,
        Self::Resolution,
        Self::Validation,
        Self::RelationsAndIntegrity,
    ];

    /// The role name as written in a manifest.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Control => "control",
            Self::Intent => "intent",
            Self::Basis => "basis",
            Self::Adr => "adr",
            Self::WorkOrder => "work_order",
            Self::Milestones => "milestones",
            Self::Execution => "execution",
            Self::Assurance => "assurance",
            Self::Resolution => "resolution",
            Self::Validation => "validation",
            Self::RelationsAndIntegrity => "relations_and_integrity",
        }
    }

    /// The name of the section this role composes into (§16.1, §18).
    #[must_use]
    pub const fn section_name(self) -> &'static str {
        match self {
            Self::Adr => "decisions",
            other => other.as_str(),
        }
    }

    /// The canonical ordinal from §16.1's table.
    #[must_use]
    pub const fn canonical_ordinal(self) -> u32 {
        match self {
            Self::Control => 0,
            Self::Intent => 10,
            Self::Basis => 20,
            Self::Adr => 30,
            Self::WorkOrder => 40,
            Self::Milestones => 45,
            Self::Execution => 50,
            Self::Assurance => 60,
            Self::Resolution => 70,
            Self::Validation => 80,
            Self::RelationsAndIntegrity => 90,
        }
    }

    /// The jurisdiction §16.1 expects for this role.
    ///
    /// `None` where the SAS lists more than one ("authored + bound",
    /// "authored definitions + generated state"), because asserting a single
    /// class there would be inventing a rule the specification declined to make.
    #[must_use]
    pub const fn typical_jurisdiction(self) -> Option<Jurisdiction> {
        match self {
            Self::Intent | Self::WorkOrder => Some(Jurisdiction::Authored),
            Self::Adr => Some(Jurisdiction::Bound),
            Self::Execution | Self::RelationsAndIntegrity => Some(Jurisdiction::Generated),
            // control: generated/bound · basis: authored + bound
            // milestones: authored definitions + generated state
            // assurance: authored obligations + generated proof
            // resolution: generated/bound · validation: authored/bound
            _ => None,
        }
    }

    /// Whether this role is produced by the compiler rather than authored.
    ///
    /// These never appear as atom entries in a manifest: a manifest declares
    /// sources, and a generated section has none.
    #[must_use]
    pub const fn is_compiler_produced(self) -> bool {
        matches!(self, Self::Control | Self::RelationsAndIntegrity)
    }
}

impl FromStr for AtomRole {
    type Err = RoleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|role| role.as_str() == s)
            .ok_or_else(|| RoleError::UnknownRole {
                found: s.to_owned(),
                known: Self::ALL
                    .iter()
                    .map(|r| r.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            })
    }
}

impl fmt::Display for AtomRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A composition profile (SAS §16.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    Delivery,
    Decision,
}

impl Profile {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Delivery => "delivery",
            Self::Decision => "decision",
        }
    }

    /// Every role §16.3 requires for this profile, including compiler-produced
    /// ones.
    #[must_use]
    pub fn required_roles(self) -> Vec<AtomRole> {
        match self {
            Self::Delivery => vec![
                AtomRole::Control,
                AtomRole::Intent,
                AtomRole::Basis,
                AtomRole::WorkOrder,
                AtomRole::Milestones,
                AtomRole::Assurance,
                AtomRole::RelationsAndIntegrity,
            ],
            Self::Decision => vec![
                AtomRole::Control,
                AtomRole::Intent,
                AtomRole::Basis,
                AtomRole::Adr,
                AtomRole::Assurance,
                AtomRole::RelationsAndIntegrity,
            ],
        }
    }

    /// The roles an AUTHOR must supply in the manifest.
    ///
    /// This is [`Self::required_roles`] minus the compiler-produced ones, and
    /// the distinction is load-bearing: §16.3 lists `control` and
    /// `relations_and_integrity` as required for both profiles, but a manifest
    /// that declared them would be claiming to author a projection. Validating
    /// a manifest against the full set would reject every correct manifest ever
    /// written.
    #[must_use]
    pub fn required_authored_roles(self) -> Vec<AtomRole> {
        self.required_roles()
            .into_iter()
            .filter(|role| !role.is_compiler_produced())
            .collect()
    }
}

impl FromStr for Profile {
    type Err = RoleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "delivery" => Ok(Self::Delivery),
            "decision" => Ok(Self::Decision),
            other => Err(RoleError::UnknownProfile {
                found: other.to_owned(),
            }),
        }
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whether an unrecognised role name is a namespaced optional extension (§16.4).
///
/// §16.4 draws a sharp line: "Unknown required roles SHALL fail closed. Unknown
/// optional namespaced roles SHALL be preserved." The two behaviours must not be
/// collapsed, so the namespace test lives here where both call sites can see it.
#[must_use]
pub fn is_namespaced_extension_role(name: &str) -> bool {
    match name.split_once('.') {
        Some((namespace, rest)) => !namespace.is_empty() && !rest.is_empty(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_round_trip_through_their_names() {
        for role in AtomRole::ALL {
            assert_eq!(
                AtomRole::from_str(role.as_str()),
                Ok(role),
                "{role} must parse from its own name"
            );
        }
    }

    #[test]
    fn canonical_ordinals_match_the_sas_table() {
        // SAS §16.1, transcribed. Pinned as an external expectation: reading
        // these back out of `canonical_ordinal` would assert only that the
        // function is consistent with itself.
        let expected = [
            ("control", 0),
            ("intent", 10),
            ("basis", 20),
            ("adr", 30),
            ("work_order", 40),
            ("milestones", 45),
            ("execution", 50),
            ("assurance", 60),
            ("resolution", 70),
            ("validation", 80),
            ("relations_and_integrity", 90),
        ];
        for (name, ordinal) in expected {
            let role = AtomRole::from_str(name).expect("known role");
            assert_eq!(role.canonical_ordinal(), ordinal, "{name}");
        }
    }

    #[test]
    fn ordinals_are_strictly_ascending_in_declaration_order() {
        let ordinals: Vec<u32> = AtomRole::ALL
            .iter()
            .map(|r| r.canonical_ordinal())
            .collect();
        let mut sorted = ordinals.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(ordinals, sorted, "ALL must be in ascending ordinal order");
    }

    #[test]
    fn the_decisions_section_is_composed_of_adr_atoms() {
        assert_eq!(AtomRole::Adr.as_str(), "adr");
        assert_eq!(AtomRole::Adr.section_name(), "decisions");
    }

    /// §16.4: an unknown required role fails closed.
    #[test]
    fn unknown_role_is_refused() {
        let err = AtomRole::from_str("hypothesis").expect_err("must refuse");
        match err {
            RoleError::UnknownRole { found, .. } => assert_eq!(found, "hypothesis"),
            other => panic!("wrong error: {other:?}"),
        }
    }

    #[test]
    fn namespaced_extension_roles_are_recognised_as_such() {
        assert!(is_namespaced_extension_role("lab.protocol"));
        assert!(is_namespaced_extension_role("x.contractor_terms"));
        assert!(!is_namespaced_extension_role("hypothesis"));
        assert!(!is_namespaced_extension_role(".leading"));
        assert!(!is_namespaced_extension_role("trailing."));
    }

    #[test]
    fn jurisdiction_write_permission_is_exhaustive() {
        assert!(Jurisdiction::Authored.is_directly_editable());
        assert!(!Jurisdiction::Bound.is_directly_editable());
        assert!(!Jurisdiction::Generated.is_directly_editable());
    }

    #[test]
    fn unknown_jurisdiction_is_refused() {
        assert_eq!(
            Jurisdiction::from_str("editable"),
            Err(RoleError::UnknownJurisdiction {
                found: "editable".to_owned()
            })
        );
    }

    /// §16.3, transcribed for both profiles.
    #[test]
    fn profile_required_roles_match_the_sas() {
        assert_eq!(
            Profile::Delivery.required_roles(),
            vec![
                AtomRole::Control,
                AtomRole::Intent,
                AtomRole::Basis,
                AtomRole::WorkOrder,
                AtomRole::Milestones,
                AtomRole::Assurance,
                AtomRole::RelationsAndIntegrity,
            ]
        );
        assert_eq!(
            Profile::Decision.required_roles(),
            vec![
                AtomRole::Control,
                AtomRole::Intent,
                AtomRole::Basis,
                AtomRole::Adr,
                AtomRole::Assurance,
                AtomRole::RelationsAndIntegrity,
            ]
        );
    }

    /// The authored subset must exclude exactly the compiler-produced roles.
    /// If this ever equals `required_roles`, every valid manifest starts failing.
    #[test]
    fn authored_roles_exclude_compiler_produced_ones() {
        for profile in [Profile::Delivery, Profile::Decision] {
            let authored = profile.required_authored_roles();
            assert!(
                !authored.contains(&AtomRole::Control),
                "{profile}: control is generated and cannot be authored"
            );
            assert!(
                !authored.contains(&AtomRole::RelationsAndIntegrity),
                "{profile}: relations_and_integrity is generated"
            );
            assert!(
                authored.len() < profile.required_roles().len(),
                "{profile}: the authored subset must be strictly smaller"
            );
        }
        assert_eq!(
            Profile::Delivery.required_authored_roles(),
            vec![
                AtomRole::Intent,
                AtomRole::Basis,
                AtomRole::WorkOrder,
                AtomRole::Milestones,
                AtomRole::Assurance,
            ]
        );
    }

    #[test]
    fn unknown_profile_is_refused() {
        assert!(Profile::from_str("experiment").is_err());
    }
}
