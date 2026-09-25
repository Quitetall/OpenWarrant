// SPDX-License-Identifier: Apache-2.0
//! Atom roles, jurisdiction classes, and composition profiles (SAS §13, §16).

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
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
    #[error(
        "unknown profile {found:?}; this program admits {known} (SAS §16.3; a profile \
         beyond the two core ones is defined in profiles/<name>.toml)"
    )]
    UnknownProfile { found: String, known: String },
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

/// One of the two profiles §16.3 defines, which every other profile extends.
///
/// This is the only closed set left: §16.3 names exactly these two and the
/// roles each requires. Everything else is a [`ProfileDefinition`] in a
/// [`ProfileRegistry`], added as data (§2.2: "SHALL NOT require redesign").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CoreProfile {
    Delivery,
    Decision,
}

impl CoreProfile {
    /// Both core profiles.
    pub const ALL: [Self; 2] = [Self::Delivery, Self::Decision];

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

    fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.as_str() == name)
    }
}

impl fmt::Display for CoreProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A composition profile (SAS §16.3): a name a [`ProfileRegistry`] validated.
///
/// Not an enum (OW-WAR-0140, U-002 option A). A profile is its name and the
/// core profile it extends; the roles it adds live in its
/// [`ProfileDefinition`]. Holding a `Profile` means some registry resolved
/// the name. [`FromStr`] resolves against [`ProfileRegistry::builtin`], so
/// `"experiment"` is still refused there, and a program's own definitions
/// are admitted only through [`ProfileRegistry::resolve`] on the registry
/// that read them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Profile {
    name: Cow<'static, str>,
    core: CoreProfile,
}

// `Profile::Delivery` and `Profile::Decision` were enum variants until
// OW-WAR-0140. Kept as associated constants under the same names, so every
// caller that names a core profile reads as it did.
#[allow(non_upper_case_globals)]
impl Profile {
    /// The core delivery profile.
    pub const Delivery: Self = Self {
        name: Cow::Borrowed("delivery"),
        core: CoreProfile::Delivery,
    };
    /// The core decision profile.
    pub const Decision: Self = Self {
        name: Cow::Borrowed("decision"),
        core: CoreProfile::Decision,
    };
}

impl Profile {
    /// The core profile itself, as a `Profile`.
    #[must_use]
    pub const fn from_core(core: CoreProfile) -> Self {
        match core {
            CoreProfile::Delivery => Self::Delivery,
            CoreProfile::Decision => Self::Decision,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// The core profile this one is, or extends.
    #[must_use]
    pub const fn core(&self) -> CoreProfile {
        self.core
    }

    /// Whether this is one of §16.3's two profiles rather than an extension.
    #[must_use]
    pub fn is_core(&self) -> bool {
        self.name == self.core.as_str()
    }

    /// Every CORE role §16.3 requires for this profile, including
    /// compiler-produced ones. An extending profile requires its core
    /// profile's roles; the namespaced roles it adds are in its
    /// [`ProfileDefinition`].
    #[must_use]
    pub fn required_roles(&self) -> Vec<AtomRole> {
        self.core.required_roles()
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
    pub fn required_authored_roles(&self) -> Vec<AtomRole> {
        self.required_roles()
            .into_iter()
            .filter(|role| !role.is_compiler_produced())
            .collect()
    }
}

impl FromStr for Profile {
    type Err = RoleError;

    /// Resolve against the built-in registry: the two core profiles only.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ProfileRegistry::builtin().resolve(s)
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The schema a profile definition file declares.
pub const PROFILE_SCHEMA: &str = "oh.war/profile/v1";

/// A namespaced role a profile requires, with what `war new` writes for it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredExtensionRole {
    /// The namespaced role, e.g. `contractor.terms`.
    pub role: String,
    /// The ordinal `war new` gives its atom.
    pub ordinal: u32,
    /// The atom's file name under `atoms/`.
    pub file: String,
    /// The atom body `war new` writes below the frontmatter.
    pub stub: String,
}

/// One profile, as data: `profiles/<name>.toml`.
///
/// A core profile's definition restates §16.3 and may not differ from it. An
/// extension names the core profile it `extends` and the namespaced roles it
/// requires, all in its own namespace. No field reaches the core types: the
/// roles it requires are composition entries like any other atom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileDefinition {
    pub name: String,
    /// The core profile this is, or extends.
    pub core: CoreProfile,
    /// `None` for a core profile.
    pub extends: Option<CoreProfile>,
    /// §22.3: "until that profile is approved". `false` says the definition
    /// is a technical mechanism nobody with the authority has approved.
    /// Always `true` for the two core profiles.
    pub approved: bool,
    pub required_extension_roles: Vec<RequiredExtensionRole>,
    /// The role whose atom names the acceptance authority, if any.
    pub acceptance_role: Option<String>,
    /// Roles whose atoms carry references rather than copies (§22.3).
    pub reference_roles: Vec<String>,
    /// `sha256:<hex>` of the definition's bytes, when read from a file. The
    /// pin a Warrant composed against it can be checked by (A-003).
    pub digest: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileFile {
    schema: String,
    name: String,
    #[serde(default)]
    core: bool,
    #[serde(default)]
    extends: Option<String>,
    #[serde(default)]
    approved: Option<bool>,
    /// Prose for a reader; carries no meaning.
    #[serde(default)]
    #[allow(dead_code)]
    note: Option<String>,
    #[serde(default)]
    required_roles: Option<Vec<String>>,
    #[serde(default)]
    requires: Vec<RequiredExtensionRole>,
    #[serde(default)]
    acceptance_role: Option<String>,
    #[serde(default)]
    reference_roles: Vec<String>,
}

/// Why a profile definition was refused. Every refusal names the file.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProfileError {
    #[error("{file}: not a profile definition: {detail}")]
    Parse { file: String, detail: String },
    #[error("{file}: schema {found:?}; this build reads {PROFILE_SCHEMA:?}")]
    Schema { file: String, found: String },
    #[error("{file}: names profile {name:?}; a definition's name is its file stem")]
    NameMismatch { file: String, name: String },
    #[error(
        "{file}: profile name {name:?} is not a lowercase word ([a-z][a-z0-9_-]*); a \
         profile name is inside every digest"
    )]
    BadName { file: String, name: String },
    #[error(
        "{file}: redefines core profile {name}: {detail}. §16.3 fixes the core profiles; a \
         program restates them and may not change them"
    )]
    CoreRedefined {
        file: String,
        name: String,
        detail: String,
    },
    #[error(
        "{file}: profile {name} must extend a core profile (delivery or decision); found {found:?}"
    )]
    BadExtends {
        file: String,
        name: String,
        found: String,
    },
    #[error(
        "{file}: profile {name} must say whether it is approved (`approved = true|false`, \
         §22.3)"
    )]
    ApprovalUnstated { file: String, name: String },
    #[error(
        "{file}: profile {name} may require only roles in its own namespace `{name}.*`, not \
         {role:?} (§16.4)"
    )]
    ForeignRole {
        file: String,
        name: String,
        role: String,
    },
    #[error("{file}: profile {name} names {role:?} as {what}, and does not require it")]
    RoleNotRequired {
        file: String,
        name: String,
        role: String,
        what: &'static str,
    },
    #[error(
        "{file}: profile {name} lists {role:?} or ordinal {ordinal} twice, or gives it a \
         core role's ordinal (§16.1)"
    )]
    Duplicate {
        file: String,
        name: String,
        role: String,
        ordinal: u32,
    },
}

/// The profiles a program admits (SAS §16.3, §2.2).
///
/// Starts from the two core profiles, whose roles are §16.3's and which
/// nothing can change, and admits a program's own definitions as data. The
/// core reads no file (§79.1): the caller hands over each definition's file
/// name and bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRegistry {
    definitions: BTreeMap<String, ProfileDefinition>,
}

impl Default for ProfileRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

impl ProfileRegistry {
    /// The two core profiles and nothing else.
    #[must_use]
    pub fn builtin() -> Self {
        let definitions = CoreProfile::ALL
            .into_iter()
            .map(|core| {
                (
                    core.as_str().to_owned(),
                    ProfileDefinition {
                        name: core.as_str().to_owned(),
                        core,
                        extends: None,
                        approved: true,
                        required_extension_roles: Vec::new(),
                        acceptance_role: None,
                        reference_roles: Vec::new(),
                        digest: None,
                    },
                )
            })
            .collect();
        Self { definitions }
    }

    /// The built-in registry plus each `(file name, bytes)` definition.
    ///
    /// Fail-closed on the first bad file: a registry that skipped one would
    /// report that file's Warrants as having an unknown profile, which is a
    /// different fault from the one that happened.
    pub fn with_definitions<'a>(
        sources: impl IntoIterator<Item = (&'a str, &'a [u8])>,
    ) -> Result<Self, ProfileError> {
        let mut registry = Self::builtin();
        for (file, bytes) in sources {
            let definition = parse_definition(file, bytes)?;
            if definition.extends.is_some() {
                registry
                    .definitions
                    .insert(definition.name.clone(), definition);
            } else if let Some(core) = registry.definitions.get_mut(&definition.name) {
                // A core restatement: identical by construction, so it only
                // contributes the pin of the file it was read from.
                core.digest = definition.digest;
            }
        }
        Ok(registry)
    }

    /// Resolve a profile name, or refuse it as unknown (§16.3).
    pub fn resolve(&self, name: &str) -> Result<Profile, RoleError> {
        match self.definitions.get(name) {
            Some(definition) if definition.extends.is_none() => {
                Ok(Profile::from_core(definition.core))
            }
            Some(definition) => Ok(Profile {
                name: Cow::Owned(definition.name.clone()),
                core: definition.core,
            }),
            None => Err(RoleError::UnknownProfile {
                found: name.to_owned(),
                known: self.names().join(", "),
            }),
        }
    }

    /// The definition behind a resolved profile.
    #[must_use]
    pub fn definition(&self, profile: &Profile) -> Option<&ProfileDefinition> {
        self.definitions
            .get(profile.as_str())
            .filter(|d| d.core == profile.core())
    }

    /// Every profile name this registry admits, sorted.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        self.definitions.keys().map(String::as_str).collect()
    }

    /// Every definition, by name.
    pub fn definitions(&self) -> impl Iterator<Item = &ProfileDefinition> {
        self.definitions.values()
    }

    /// The namespaced roles `profile` requires, empty for a core profile.
    #[must_use]
    pub fn required_extension_roles(&self, profile: &Profile) -> Vec<&str> {
        self.definition(profile)
            .map(|d| {
                d.required_extension_roles
                    .iter()
                    .map(|r| r.role.as_str())
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn is_profile_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn parse_definition(file: &str, bytes: &[u8]) -> Result<ProfileDefinition, ProfileError> {
    use sha2::{Digest as _, Sha256};

    let owned = file.to_owned();
    let text = std::str::from_utf8(bytes).map_err(|e| ProfileError::Parse {
        file: owned.clone(),
        detail: e.to_string(),
    })?;
    let raw: ProfileFile = toml::from_str(text).map_err(|e| ProfileError::Parse {
        file: owned.clone(),
        detail: e.to_string(),
    })?;
    if raw.schema != PROFILE_SCHEMA {
        return Err(ProfileError::Schema {
            file: owned,
            found: raw.schema,
        });
    }
    let stem = file.strip_suffix(".toml").unwrap_or(file);
    let stem = stem.rsplit('/').next().unwrap_or(stem);
    if raw.name != stem {
        return Err(ProfileError::NameMismatch {
            file: owned,
            name: raw.name,
        });
    }
    if !is_profile_name(&raw.name) {
        return Err(ProfileError::BadName {
            file: owned,
            name: raw.name,
        });
    }
    let digest = Some(format!(
        "sha256:{}",
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    ));

    if let Some(core) = CoreProfile::parse(&raw.name) {
        let redefined = |detail: &str| ProfileError::CoreRedefined {
            file: owned.clone(),
            name: raw.name.clone(),
            detail: detail.to_owned(),
        };
        if !raw.core {
            return Err(redefined("it does not say `core = true`"));
        }
        if raw.extends.is_some() {
            return Err(redefined("a core profile extends nothing"));
        }
        if raw.approved == Some(false) {
            return Err(redefined("a core profile cannot be unapproved"));
        }
        if !raw.requires.is_empty()
            || raw.acceptance_role.is_some()
            || !raw.reference_roles.is_empty()
        {
            return Err(redefined("a core profile requires no namespaced role"));
        }
        let want: Vec<&str> = core.required_roles().iter().map(|r| r.as_str()).collect();
        let found: Vec<&str> = raw
            .required_roles
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(String::as_str)
            .collect();
        if found != want {
            return Err(redefined(&format!(
                "required_roles {found:?}, where §16.3 requires {want:?}"
            )));
        }
        return Ok(ProfileDefinition {
            name: raw.name,
            core,
            extends: None,
            approved: true,
            required_extension_roles: Vec::new(),
            acceptance_role: None,
            reference_roles: Vec::new(),
            digest,
        });
    }

    if raw.core || raw.required_roles.is_some() {
        return Err(ProfileError::BadExtends {
            file: owned,
            name: raw.name,
            found: "a core profile's fields (`core`, `required_roles`)".to_owned(),
        });
    }
    let extends = raw.extends.clone().unwrap_or_default();
    let Some(core) = CoreProfile::parse(&extends) else {
        return Err(ProfileError::BadExtends {
            file: owned,
            name: raw.name,
            found: extends,
        });
    };
    let Some(approved) = raw.approved else {
        return Err(ProfileError::ApprovalUnstated {
            file: owned,
            name: raw.name,
        });
    };
    let namespace = format!("{}.", raw.name);
    let mut roles = BTreeSet::new();
    let mut ordinals = BTreeSet::new();
    for required in &raw.requires {
        if !required.role.starts_with(&namespace) || !is_namespaced_extension_role(&required.role) {
            return Err(ProfileError::ForeignRole {
                file: owned,
                name: raw.name.clone(),
                role: required.role.clone(),
            });
        }
        let core_ordinal = AtomRole::ALL
            .iter()
            .any(|r| r.canonical_ordinal() == required.ordinal);
        if !roles.insert(required.role.as_str())
            || !ordinals.insert(required.ordinal)
            || core_ordinal
        {
            return Err(ProfileError::Duplicate {
                file: owned,
                name: raw.name.clone(),
                role: required.role.clone(),
                ordinal: required.ordinal,
            });
        }
    }
    let named = raw
        .acceptance_role
        .iter()
        .map(|r| (r, "its acceptance role"))
        .chain(raw.reference_roles.iter().map(|r| (r, "a reference role")));
    for (role, what) in named {
        if !roles.contains(role.as_str()) {
            return Err(ProfileError::RoleNotRequired {
                file: owned,
                name: raw.name.clone(),
                role: role.clone(),
                what,
            });
        }
    }
    Ok(ProfileDefinition {
        name: raw.name,
        core,
        extends: Some(core),
        approved,
        required_extension_roles: raw.requires,
        acceptance_role: raw.acceptance_role,
        reference_roles: raw.reference_roles,
        digest,
    })
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
        assert!(ProfileRegistry::builtin().resolve("contractor").is_err());
    }

    /// OW-WAR-0140 OBL-001: the two core profiles as data are exactly §16.3,
    /// and reading them changes nothing a Warrant composes against.
    #[test]
    fn the_core_profile_files_restate_the_sas() {
        let delivery = include_bytes!("../../../profiles/delivery.toml");
        let decision = include_bytes!("../../../profiles/decision.toml");
        let registry = ProfileRegistry::with_definitions([
            ("profiles/delivery.toml", delivery.as_slice()),
            ("profiles/decision.toml", decision.as_slice()),
        ])
        .expect("the committed definitions parse");
        assert_eq!(registry.names(), vec!["decision", "delivery"]);
        assert_eq!(registry.resolve("delivery"), Ok(Profile::Delivery));
        assert_eq!(registry.resolve("decision"), Ok(Profile::Decision));
        for core in CoreProfile::ALL {
            let d = registry
                .definition(&Profile::from_core(core))
                .expect("defined");
            assert!(d.required_extension_roles.is_empty());
            assert!(
                d.digest
                    .as_deref()
                    .is_some_and(|x| x.starts_with("sha256:"))
            );
        }
        assert!(registry.resolve("experiment").is_err());
    }

    #[test]
    fn a_core_profile_cannot_be_redefined() {
        let loosened = br#"
schema = "oh.war/profile/v1"
name = "delivery"
core = true
required_roles = ["control", "intent", "relations_and_integrity"]
"#;
        assert!(matches!(
            ProfileRegistry::with_definitions([("profiles/delivery.toml", loosened.as_slice())]),
            Err(ProfileError::CoreRedefined { .. })
        ));
    }

    const EXT: &str = r#"
schema = "oh.war/profile/v1"
name = "lab"
extends = "delivery"
approved = false
acceptance_role = "lab.acceptance"
reference_roles = ["lab.terms"]

[[requires]]
role = "lab.acceptance"
ordinal = 71
file = "71-lab-acceptance.md"
stub = "x"

[[requires]]
role = "lab.terms"
ordinal = 72
file = "72-lab-terms.md"
stub = "y"
"#;

    #[test]
    fn an_extension_resolves_to_its_core_profile() {
        let registry = ProfileRegistry::with_definitions([("profiles/lab.toml", EXT.as_bytes())])
            .expect("parses");
        let lab = registry.resolve("lab").expect("admitted");
        assert_eq!(lab.as_str(), "lab");
        assert_eq!(lab.core(), CoreProfile::Delivery);
        assert!(!lab.is_core());
        assert_eq!(lab.required_roles(), Profile::Delivery.required_roles());
        assert_eq!(
            registry.required_extension_roles(&lab),
            vec!["lab.acceptance", "lab.terms"]
        );
        let d = registry.definition(&lab).expect("defined");
        assert!(!d.approved);
        assert_eq!(d.acceptance_role.as_deref(), Some("lab.acceptance"));
    }

    #[test]
    fn an_extension_is_refused_on_every_malformation() {
        let refuse = |file: &str, text: String| {
            ProfileRegistry::with_definitions([(file, text.as_bytes())]).expect_err(&text)
        };
        assert!(matches!(
            refuse("profiles/other.toml", EXT.to_owned()),
            ProfileError::NameMismatch { .. }
        ));
        assert!(matches!(
            refuse(
                "profiles/lab.toml",
                EXT.replace("extends = \"delivery\"", "extends = \"lab\"")
            ),
            ProfileError::BadExtends { .. }
        ));
        assert!(matches!(
            refuse("profiles/lab.toml", EXT.replace("approved = false\n", "")),
            ProfileError::ApprovalUnstated { .. }
        ));
        assert!(matches!(
            refuse(
                "profiles/lab.toml",
                EXT.replace("role = \"lab.terms\"", "role = \"contractor.terms\"")
            ),
            ProfileError::ForeignRole { .. }
        ));
        assert!(matches!(
            refuse(
                "profiles/lab.toml",
                EXT.replace("ordinal = 72", "ordinal = 71")
            ),
            ProfileError::Duplicate { .. }
        ));
        assert!(matches!(
            refuse(
                "profiles/lab.toml",
                EXT.replace("ordinal = 72", "ordinal = 60")
            ),
            ProfileError::Duplicate { .. }
        ));
        assert!(matches!(
            refuse(
                "profiles/lab.toml",
                EXT.replace(
                    "reference_roles = [\"lab.terms\"]",
                    "reference_roles = [\"lab.other\"]"
                )
            ),
            ProfileError::RoleNotRequired { .. }
        ));
        assert!(matches!(
            refuse("profiles/lab.toml", format!("{EXT}\nsurprise = 1\n")),
            ProfileError::Parse { .. }
        ));
    }
}
