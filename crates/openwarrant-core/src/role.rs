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

/// What a document's type makes apply to it (OW-ADR-0031): a closed set the
/// kernel implements. A profile SELECTS capabilities; it cannot define one,
/// and a capability never supplies an act — `verification` means "this
/// cannot read verified without an independent response", not "verified".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Capability {
    /// Parse, schema, required roles: today's structural `war check`.
    Structure,
    /// Relations resolve: parents, roadmap, traceability.
    Links,
    /// Claim, release and done on work items.
    Claims,
    /// A revision is in force only once a human accepts it.
    Acceptance,
    /// Gate runs and receipts bound to a subject.
    Evidence,
    /// Obligations settled by an independent verifier (§51.2, RQ-053).
    Verification,
    /// A human signs the contract before work counts.
    Authorization,
    /// A human closes it against settled obligations (§56.1).
    Resolution,
    /// A milestone graph of dispatchable stages, and §56.1 requirement 12's
    /// runtime receipts.
    Stages,
}

impl Capability {
    /// The closed set, in declaration order.
    pub const ALL: [Self; 9] = [
        Self::Structure,
        Self::Links,
        Self::Claims,
        Self::Acceptance,
        Self::Evidence,
        Self::Verification,
        Self::Authorization,
        Self::Resolution,
        Self::Stages,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Structure => "structure",
            Self::Links => "links",
            Self::Claims => "claims",
            Self::Acceptance => "acceptance",
            Self::Evidence => "evidence",
            Self::Verification => "verification",
            Self::Authorization => "authorization",
            Self::Resolution => "resolution",
            Self::Stages => "stages",
        }
    }

    /// A capability by name, or `None` for a name outside the closed set.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == name)
    }

    /// What this capability needs beside it (OW-ADR-0031).
    #[must_use]
    pub const fn prerequisites(self) -> &'static [Self] {
        match self {
            Self::Structure => &[],
            Self::Links
            | Self::Claims
            | Self::Acceptance
            | Self::Evidence
            | Self::Authorization
            | Self::Stages => &[Self::Structure],
            Self::Verification => &[Self::Evidence],
            Self::Resolution => &[Self::Verification, Self::Authorization],
        }
    }

    const fn bit(self) -> u16 {
        1 << (self as u16)
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A set of [`Capability`], as a profile selects them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Capabilities(u16);

impl Capabilities {
    /// Every capability: the delivery profile's set.
    pub const ALL: Self = Self::of(&Capability::ALL);
    /// The working form's set (`form = "working"`, a ticket).
    pub const WORKING: Self =
        Self::of(&[Capability::Structure, Capability::Links, Capability::Claims]);

    /// The set holding exactly `caps`.
    #[must_use]
    pub const fn of(caps: &[Capability]) -> Self {
        let mut bits = 0u16;
        let mut i = 0;
        while i < caps.len() {
            bits |= caps[i].bit();
            i += 1;
        }
        Self(bits)
    }

    #[must_use]
    pub const fn has(self, cap: Capability) -> bool {
        self.0 & cap.bit() != 0
    }

    #[must_use]
    pub const fn without(self, cap: Capability) -> Self {
        Self(self.0 & !cap.bit())
    }

    /// Whether every capability in `self` is also in `other`.
    #[must_use]
    pub const fn is_subset_of(self, other: Self) -> bool {
        self.0 & !other.0 == 0
    }

    /// The capabilities held, in the closed set's order.
    pub fn iter(self) -> impl Iterator<Item = Capability> {
        Capability::ALL.into_iter().filter(move |c| self.has(*c))
    }

    /// The capabilities of the closed set this one lacks.
    pub fn absent(self) -> impl Iterator<Item = Capability> {
        Capability::ALL.into_iter().filter(move |c| !self.has(*c))
    }

    /// The first selected capability whose prerequisite is not selected.
    #[must_use]
    pub fn missing_prerequisite(self) -> Option<(Capability, Capability)> {
        self.iter().find_map(|c| {
            c.prerequisites()
                .iter()
                .find(|p| !self.has(**p))
                .map(|p| (c, *p))
        })
    }
}

impl fmt::Display for Capabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> = self.iter().map(Capability::as_str).collect();
        f.write_str(&names.join(", "))
    }
}

/// What a kind is, as data (OW-ADR-0031): its capabilities, and the per-kind
/// behaviour that was once chosen by matching a profile's name. A core
/// profile's is fixed here; its file may restate it and may not change it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindData {
    pub capabilities: Capabilities,
    /// The `profile_outcome` a `satisfied` resolution records (§56.2);
    /// `None` records the common outcome's own word.
    pub satisfied_outcome: Option<String>,
    /// §56.3: the profile carries a falsifiable claim, so a resolution of it
    /// may record `falsified`.
    pub falsifiable_claims: bool,
    /// OW-ADR-0029: a standing authorization class may cover this profile.
    pub standing_coverage: bool,
}

impl CoreProfile {
    /// The core profile's kind data (OW-ADR-0031). `delivery` selects every
    /// capability; `decision` every one but `stages`, so §56.1 requirement 12
    /// reads "not applicable" for a decision instead of unmet forever.
    #[must_use]
    pub fn kind_data(self) -> KindData {
        match self {
            Self::Delivery => KindData {
                capabilities: Capabilities::ALL,
                satisfied_outcome: Some("delivered".to_owned()),
                falsifiable_claims: false,
                standing_coverage: true,
            },
            Self::Decision => KindData {
                capabilities: Capabilities::ALL.without(Capability::Stages),
                satisfied_outcome: None,
                falsifiable_claims: false,
                standing_coverage: false,
            },
        }
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
    /// `form = "working"`: the core roles a record of this profile carries
    /// BEFORE it is promoted into its core profile. `None` is the contract
    /// form every profile had before the ticket loop: a Warrant of it
    /// requires every core role §16.3 names.
    ///
    /// A working-form record is not a Warrant of the contract corpus. It
    /// lives outside `docs/warrants/`, is never compiled, authorized,
    /// verified or resolved under this profile, and
    /// [`crate::Manifest::validate_in`] refuses a Warrant manifest that names
    /// it (`WorkingFormProfile`). The only way from a working record into the
    /// contract layer is promotion into the core profile, which then requires
    /// every core role as before. So a working form loosens nothing a
    /// signature, a verification or a resolution reads.
    pub working_core_roles: Option<Vec<AtomRole>>,
    /// Its capabilities and the per-kind behaviour once chosen by name
    /// (OW-ADR-0031). A core profile's is [`CoreProfile::kind_data`]; an
    /// extension's defaults to its core's (the working form's set for
    /// `form = "working"`) and may narrow it, never widen it.
    pub kind: KindData,
    /// The record types this profile composes and the core relation kinds
    /// its records may use and must use (`[records]`, `[relations]`;
    /// OW-WAR-0148 M3). Empty when the file declares none, and always for a
    /// built-in core profile: the kernel knows no profile nouns. A core
    /// file's declaration is program data, not kind data: it loosens nothing
    /// an act reads.
    pub vocabulary: crate::relation::Vocabulary,
    /// OW-WAR-0148 M4: `[[states]]`, the declared refinements of fixed
    /// kernel states this profile's records may enter (`war state`). Empty
    /// when the file declares none, and always for a built-in core profile.
    /// Program data, not kind data: a declared state satisfies no check and
    /// no gate.
    pub states: Vec<crate::kernel_state::DeclaredState>,
    /// OW-WAR-0148 M5: `[fields]`, the values a working-form record's
    /// optional fields may take (a ticket's `type` and `labels`). Empty when
    /// the file declares none. Program data: it selects no capability and
    /// loosens nothing an act reads.
    pub fields: FieldsDecl,
    /// OW-WAR-0148 M13: `[ticks]`, the least a tick of this type must show
    /// on the ladder (claimed < observed < independent < signed), for every
    /// item, for every milestone, and for the milestones of a record of a
    /// given `type`. Empty when the file declares none: every minimum is
    /// `claimed`. Program data: it raises what `war done` asks for and
    /// loosens nothing an act reads.
    pub ticks: crate::ticks::TicksDecl,
}

/// `[fields]` of a working-form profile (OW-WAR-0148 M5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldsDecl {
    /// The types a record may have. A type outside them is refused; with
    /// none declared, a record has no type.
    pub types: Vec<String>,
    /// The labels this program names. Open unless `labels_closed`.
    pub labels: Vec<String>,
    /// Only the declared labels are admitted; any other is refused by rule
    /// (`ticket.label-unknown`).
    pub labels_closed: bool,
}

impl FieldsDecl {
    /// Why `kind` is refused as a type, if it is.
    #[must_use]
    pub fn refuse_type(&self, kind: &str) -> Option<String> {
        if self.types.iter().any(|t| t == kind) {
            return None;
        }
        Some(if self.types.is_empty() {
            format!("type {kind:?}: the ticket profile declares no types ([fields] types)")
        } else {
            format!(
                "type {kind:?} is not one the ticket profile declares: {}",
                self.types.join(", ")
            )
        })
    }

    /// Why `label` is refused, if it is: only when the set is closed.
    #[must_use]
    pub fn refuse_label(&self, label: &str) -> Option<String> {
        (self.labels_closed && !self.labels.iter().any(|l| l == label)).then(|| {
            format!(
                "label {label:?} is not in the ticket profile's closed label set: {}",
                self.labels.join(", ")
            )
        })
    }
}

impl ProfileDefinition {
    /// The capabilities this kind selects.
    #[must_use]
    pub const fn capabilities(&self) -> Capabilities {
        self.kind.capabilities
    }

    /// Whether this is a working-form profile (`form = "working"`).
    #[must_use]
    pub const fn is_working_form(&self) -> bool {
        self.working_core_roles.is_some()
    }

    /// The roles a working-form record must carry: its core roles, then its
    /// own namespaced roles. Empty for a contract-form profile, whose roles
    /// a manifest supplies and [`crate::Manifest::validate_in`] checks.
    #[must_use]
    pub fn working_roles(&self) -> Vec<String> {
        let Some(core) = &self.working_core_roles else {
            return Vec::new();
        };
        core.iter()
            .map(|r| r.as_str().to_owned())
            .chain(self.required_extension_roles.iter().map(|r| r.role.clone()))
            .collect()
    }
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
    /// `"contract"` (the default) or `"working"`.
    #[serde(default)]
    form: Option<String>,
    /// For `form = "working"` only: the core roles a working record carries.
    #[serde(default)]
    core_roles: Option<Vec<String>>,
    /// OW-ADR-0031: the capabilities this kind selects, from the closed set.
    /// Absent: the core profile's, or the working form's for `form =
    /// "working"`.
    #[serde(default)]
    capabilities: Option<Vec<String>>,
    /// The `profile_outcome` word a `satisfied` resolution records.
    #[serde(default)]
    satisfied_outcome: Option<String>,
    /// §56.3: whether the kind carries a falsifiable claim.
    #[serde(default)]
    falsifiable_claims: Option<bool>,
    /// OW-ADR-0029: whether a standing class may cover the kind.
    #[serde(default)]
    standing_coverage: Option<bool>,
    /// OW-WAR-0148 M3: `[records] types = [...]`, the record types (profile
    /// nouns) a record atom governed by this profile may hold.
    #[serde(default)]
    records: Option<RecordsTable>,
    /// OW-WAR-0148 M3: `[relations] allow = [...]` (core kinds) and
    /// `require = [[from_type, kind, to_type], ...]`.
    #[serde(default)]
    relations: Option<RelationsTable>,
    /// OW-WAR-0148 M4: `[[states]] name = "...", refines = "<fixed state>"`.
    #[serde(default)]
    states: Vec<StateEntry>,
    /// OW-WAR-0148 M5: `[fields] types = [...]`, `labels = [...]`,
    /// `labels_closed = bool`. A working form only.
    #[serde(default)]
    fields: Option<FieldsTable>,
    /// OW-WAR-0148 M13: `[ticks] item = "...", milestone = "..."` and
    /// `[ticks.types] <type> = "<level>"`.
    #[serde(default)]
    ticks: Option<TicksTable>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct TicksTable {
    #[serde(default)]
    item: Option<String>,
    #[serde(default)]
    milestone: Option<String>,
    #[serde(default)]
    types: BTreeMap<String, String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldsTable {
    #[serde(default)]
    types: Vec<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    labels_closed: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordsTable {
    #[serde(default)]
    types: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RelationsTable {
    #[serde(default)]
    allow: Vec<String>,
    #[serde(default)]
    require: Vec<[String; 3]>,
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
        "{file}: profile {name}: {detail}. A working form (`form = \"working\"`) names the \
         authored core roles of the profile it extends that a record carries before \
         promotion (`core_roles`); a contract form names none"
    )]
    BadForm {
        file: String,
        name: String,
        detail: String,
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
    #[error(
        "{file}: profile {name} names capability {found:?}, which is not one of the closed \
         set ({known}). A profile selects capabilities; it cannot define one (OW-ADR-0031)"
    )]
    UnknownCapability {
        file: String,
        name: String,
        found: String,
        known: String,
    },
    #[error(
        "{file}: profile {name} selects `{capability}` without `{needs}`, which it needs \
         (OW-ADR-0031)"
    )]
    CapabilityPrerequisite {
        file: String,
        name: String,
        capability: Capability,
        needs: Capability,
    },
    #[error("{file}: profile {name}: {detail} (OW-ADR-0031)")]
    BadCapabilities {
        file: String,
        name: String,
        detail: String,
    },
    #[error("{file}: profile {name}: [records]/[relations]: {detail} (OW-ADR-0031)")]
    BadVocabulary {
        file: String,
        name: String,
        detail: String,
    },
    #[error("{file}: profile {name}: [[states]]: {detail} (OW-ADR-0031)")]
    BadState {
        file: String,
        name: String,
        rule: &'static str,
        detail: String,
    },
    /// OW-WAR-0148 M5: `[fields]` refused.
    #[error("{file}: profile {name}: [fields]: {detail}")]
    BadFields {
        file: String,
        name: String,
        detail: String,
    },
    /// OW-WAR-0148 M13: `[ticks]` refused.
    #[error("{file}: profile {name}: [ticks]: {detail}")]
    BadTicks {
        file: String,
        name: String,
        detail: String,
    },
    /// OW-WAR-0148 M6: a document type (`form = "document"`) refused, under
    /// the rule [`crate::projection`] names.
    #[error("{file}: document type {name}: {detail} (OW-ADR-0031)")]
    BadDocument {
        file: String,
        name: String,
        rule: &'static str,
        detail: String,
    },
}

impl ProfileError {
    /// The rule a refusal is reported under. Every refusal that is not about
    /// capabilities keeps the one rule it always had.
    #[must_use]
    pub const fn rule(&self) -> &'static str {
        match self {
            Self::UnknownCapability { .. } => "profile.capability-unknown",
            Self::CapabilityPrerequisite { .. } => "profile.capability-prerequisite",
            Self::BadCapabilities { .. } => "profile.capabilities",
            Self::BadVocabulary { .. } => "profile.records",
            Self::BadFields { .. } => "profile.fields",
            Self::BadTicks { .. } => "profile.ticks",
            Self::BadState { rule, .. } => rule,
            Self::BadDocument { rule, .. } => rule,
            _ => "profile.invalid",
        }
    }
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
    /// OW-WAR-0148 M6: the document types (`form = "document"`), by name.
    /// Never Warrant profiles: [`Self::resolve`] does not know them.
    documents: BTreeMap<String, crate::projection::DocumentProfile>,
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
                        working_core_roles: None,
                        kind: core.kind_data(),
                        vocabulary: crate::relation::Vocabulary::default(),
                        states: Vec::new(),
                        fields: FieldsDecl::default(),
                        ticks: crate::ticks::TicksDecl::default(),
                    },
                )
            })
            .collect();
        Self {
            definitions,
            documents: BTreeMap::new(),
        }
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
            // OW-WAR-0148 M6: a document type is read by its own parser.
            if let Some(document) = parse_document_file(file, bytes, &registry)? {
                registry.documents.insert(document.name.clone(), document);
                continue;
            }
            let definition = parse_definition(file, bytes)?;
            if definition.extends.is_some() {
                registry
                    .definitions
                    .insert(definition.name.clone(), definition);
            } else if let Some(core) = registry.definitions.get_mut(&definition.name) {
                // A core restatement: identical in its kind data by
                // construction, so it contributes the pin of the file it was
                // read from, and the record vocabulary the program declares
                // (program data, not kind data).
                core.digest = definition.digest;
                core.vocabulary = definition.vocabulary;
                core.states = definition.states;
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

    /// Whether `profile` is a working form (a ticket, OW-WAR-0147): a record
    /// of it lives outside the contract corpus until it is promoted.
    #[must_use]
    pub fn is_working_form(&self, profile: &Profile) -> bool {
        self.definition(profile)
            .is_some_and(ProfileDefinition::is_working_form)
    }

    /// The kind data behind `profile` (OW-ADR-0031): its definition's, or its
    /// core profile's when the registry holds no definition by that name.
    #[must_use]
    pub fn kind(&self, profile: &Profile) -> KindData {
        self.definition(profile)
            .map_or_else(|| profile.core().kind_data(), |d| d.kind.clone())
    }

    /// The capabilities `profile` selects.
    #[must_use]
    pub fn capabilities(&self, profile: &Profile) -> Capabilities {
        self.definition(profile).map_or_else(
            || profile.core().kind_data().capabilities,
            ProfileDefinition::capabilities,
        )
    }

    /// The record vocabulary of the profile named `name`, if the registry
    /// admits one by that name (OW-WAR-0148 M3).
    #[must_use]
    pub fn vocabulary(&self, name: &str) -> Option<&crate::relation::Vocabulary> {
        self.definitions
            .get(name)
            .map(|d| &d.vocabulary)
            .or_else(|| self.documents.get(name).map(|d| &d.vocabulary))
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

// ---- OW-WAR-0148 M6: document types (`form = "document"`) and their
// projections. Additive: a file without `form = "document"` is read by
// `parse_definition` exactly as before, and a document type is never a
// Warrant profile (`resolve` does not know it).

impl ProfileRegistry {
    /// Every document type, by name.
    pub fn documents(&self) -> impl Iterator<Item = &crate::projection::DocumentProfile> {
        self.documents.values()
    }

    /// The document type named `name`.
    #[must_use]
    pub fn document(&self, name: &str) -> Option<&crate::projection::DocumentProfile> {
        self.documents.get(name)
    }

    /// The projection named `name` and the document type that declares it.
    /// Projection names are unique across a program's document types.
    #[must_use]
    pub fn projection(
        &self,
        name: &str,
    ) -> Option<(
        &crate::projection::DocumentProfile,
        &crate::projection::ProjectionDef,
    )> {
        self.documents.values().find_map(|d| {
            d.projections
                .iter()
                .find(|p| p.name == name)
                .map(|p| (d, p))
        })
    }
}

/// A document type, when `bytes` declares `form = "document"`; `None` for
/// every other profile file. Its name is its file stem, as for every
/// profile, and its projections' names are unique across the registry.
fn parse_document_file(
    file: &str,
    bytes: &[u8],
    registry: &ProfileRegistry,
) -> Result<Option<crate::projection::DocumentProfile>, ProfileError> {
    use sha2::{Digest as _, Sha256};
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Ok(None);
    };
    if !crate::projection::is_document_form(text) {
        return Ok(None);
    }
    let stem = file.strip_suffix(".toml").unwrap_or(file);
    let stem = stem.rsplit('/').next().unwrap_or(stem);
    let digest = format!(
        "sha256:{}",
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let document =
        crate::projection::parse_document(text, digest).map_err(|e| ProfileError::BadDocument {
            file: file.to_owned(),
            name: stem.to_owned(),
            rule: e.rule,
            detail: e.detail,
        })?;
    if document.name != stem {
        return Err(ProfileError::NameMismatch {
            file: file.to_owned(),
            name: document.name,
        });
    }
    for p in &document.projections {
        if let Some((other, _)) = registry.projection(&p.name) {
            return Err(ProfileError::BadDocument {
                file: file.to_owned(),
                name: document.name.clone(),
                rule: "profile.projection",
                detail: format!(
                    "projection `{}` is already declared by document type {}; a projection's \
                     name is unique across the program",
                    p.name, other.name
                ),
            });
        }
    }
    Ok(Some(document))
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
        if raw.form.is_some() || raw.core_roles.is_some() {
            return Err(redefined("a core profile has no working form"));
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
        let vocabulary = parse_vocabulary(&owned, &raw.name, &raw)?;
        // OW-ADR-0031: a core file may restate its kind data, never change it.
        let fixed = core.kind_data();
        let stated = parse_kind(&owned, &raw.name, &fixed, &raw)?;
        if stated != fixed {
            return Err(redefined(&format!(
                "its kind data (capabilities [{}], satisfied_outcome {:?}, \
                 falsifiable_claims {}, standing_coverage {}) differs from the core's \
                 (capabilities [{}], satisfied_outcome {:?}, falsifiable_claims {}, \
                 standing_coverage {})",
                stated.capabilities,
                stated.satisfied_outcome,
                stated.falsifiable_claims,
                stated.standing_coverage,
                fixed.capabilities,
                fixed.satisfied_outcome,
                fixed.falsifiable_claims,
                fixed.standing_coverage
            )));
        }
        let states = parse_states(&owned, &raw.name, fixed.capabilities, &raw)?;
        parse_fields(&owned, &raw.name, false, &raw)?;
        let ticks = parse_ticks(&owned, &raw.name, &FieldsDecl::default(), &raw)?;
        return Ok(ProfileDefinition {
            name: raw.name,
            core,
            extends: None,
            approved: true,
            required_extension_roles: Vec::new(),
            acceptance_role: None,
            reference_roles: Vec::new(),
            digest,
            working_core_roles: None,
            states,
            kind: fixed,
            vocabulary,
            fields: FieldsDecl::default(),
            ticks,
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
    let working_core_roles = parse_form(&owned, &raw.name, core, &raw)?;
    let vocabulary = parse_vocabulary(&owned, &raw.name, &raw)?;
    let base = if working_core_roles.is_some() {
        Capabilities::WORKING
    } else {
        core.kind_data().capabilities
    };
    let defaults = KindData {
        capabilities: base,
        satisfied_outcome: None,
        falsifiable_claims: false,
        standing_coverage: false,
    };
    let kind = parse_kind(&owned, &raw.name, &defaults, &raw)?;
    if !kind.capabilities.is_subset_of(base) {
        let beyond: Vec<&str> = kind
            .capabilities
            .iter()
            .filter(|c| !base.has(*c))
            .map(Capability::as_str)
            .collect();
        return Err(ProfileError::BadCapabilities {
            file: owned,
            name: raw.name,
            detail: format!(
                "selects [{}], which {} does not; an extension narrows its core's \
                 capabilities ([{base}]) and never widens them",
                beyond.join(", "),
                if working_core_roles.is_some() {
                    "a working form".to_owned()
                } else {
                    format!("`{core}`")
                }
            ),
        });
    }
    let states = parse_states(&owned, &raw.name, kind.capabilities, &raw)?;
    let fields = parse_fields(&owned, &raw.name, working_core_roles.is_some(), &raw)?;
    let ticks = parse_ticks(&owned, &raw.name, &fields, &raw)?;
    Ok(ProfileDefinition {
        name: raw.name,
        core,
        extends: Some(core),
        approved,
        required_extension_roles: raw.requires,
        acceptance_role: raw.acceptance_role,
        reference_roles: raw.reference_roles,
        digest,
        working_core_roles,
        states,
        kind,
        vocabulary,
        fields,
        ticks,
    })
}

// ---- OW-WAR-0148 M13: the minimum a tick must show ----------------------------

fn parse_ticks(
    file: &str,
    name: &str,
    fields: &FieldsDecl,
    raw: &ProfileFile,
) -> Result<crate::ticks::TicksDecl, ProfileError> {
    use crate::ticks::Level;
    let Some(table) = &raw.ticks else {
        return Ok(crate::ticks::TicksDecl::default());
    };
    let bad = |detail: String| ProfileError::BadTicks {
        file: file.to_owned(),
        name: name.to_owned(),
        detail,
    };
    let level = |what: &str, v: &str| -> Result<Level, ProfileError> {
        Level::parse(v).ok_or_else(|| {
            bad(format!(
                "{what} {v:?} is not a level: claimed, observed, independent or signed"
            ))
        })
    };
    let item = table
        .item
        .as_deref()
        .map(|v| level("item", v))
        .transpose()?;
    let milestone = table
        .milestone
        .as_deref()
        .map(|v| level("milestone", v))
        .transpose()?;
    let mut types = BTreeMap::new();
    for (kind, v) in &table.types {
        if !fields.types.iter().any(|t| t == kind) {
            return Err(bad(format!(
                "types.{kind}: the profile declares no such type ([fields] types: {})",
                if fields.types.is_empty() {
                    "none".to_owned()
                } else {
                    fields.types.join(", ")
                }
            )));
        }
        types.insert(kind.clone(), level(&format!("types.{kind}"), v)?);
    }
    Ok(crate::ticks::TicksDecl {
        item,
        milestone,
        types,
    })
}

// ---- OW-WAR-0148 M5: a working form's fields ---------------------------------

fn parse_fields(
    file: &str,
    name: &str,
    working: bool,
    raw: &ProfileFile,
) -> Result<FieldsDecl, ProfileError> {
    let Some(table) = &raw.fields else {
        return Ok(FieldsDecl::default());
    };
    let bad = |detail: String| ProfileError::BadFields {
        file: file.to_owned(),
        name: name.to_owned(),
        detail,
    };
    if !working {
        return Err(bad(
            "only a working form (`form = \"working\"`, a ticket) declares fields".to_owned(),
        ));
    }
    for (what, words) in [("type", &table.types), ("label", &table.labels)] {
        let mut seen = BTreeSet::new();
        for w in words {
            if !crate::ticket::is_field_word(w) {
                return Err(bad(format!(
                    "{what} {w:?} is not a word (lowercase, [a-z0-9_-], 1 to 40)"
                )));
            }
            if !seen.insert(w.as_str()) {
                return Err(bad(format!("{what} {w:?} is declared twice")));
            }
        }
    }
    if table.labels_closed && table.labels.is_empty() {
        return Err(bad(
            "labels_closed with no labels would refuse every label; declare the set".to_owned(),
        ));
    }
    Ok(FieldsDecl {
        types: table.types.clone(),
        labels: table.labels.clone(),
        labels_closed: table.labels_closed,
    })
}

// ---- OW-WAR-0148 M4: declared states ----------------------------------------

/// One `[[states]]` entry as written.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StateEntry {
    name: String,
    refines: String,
    /// Prose for a reader; carries no meaning.
    #[serde(default)]
    #[allow(dead_code)]
    note: Option<String>,
}

/// `[[states]]`: checked by [`crate::kernel_state::declare`] against the
/// capabilities the kind selects, each refusal under its own rule.
fn parse_states(
    file: &str,
    name: &str,
    capabilities: Capabilities,
    raw: &ProfileFile,
) -> Result<Vec<crate::kernel_state::DeclaredState>, ProfileError> {
    let entries: Vec<(String, String)> = raw
        .states
        .iter()
        .map(|s| (s.name.clone(), s.refines.clone()))
        .collect();
    crate::kernel_state::declare(&entries, capabilities).map_err(|e| ProfileError::BadState {
        file: file.to_owned(),
        name: name.to_owned(),
        rule: e.rule,
        detail: e.detail,
    })
}

// ---- end of declared states -------------------------------------------------

/// `[records]` and `[relations]` (OW-WAR-0148 M3): checked by
/// [`crate::relation::Vocabulary::declare`], refused `profile.records`.
fn parse_vocabulary(
    file: &str,
    name: &str,
    raw: &ProfileFile,
) -> Result<crate::relation::Vocabulary, ProfileError> {
    let none = RelationsTable::default();
    let types = raw
        .records
        .as_ref()
        .map(|r| r.types.as_slice())
        .unwrap_or_default();
    let relations = raw.relations.as_ref().unwrap_or(&none);
    crate::relation::Vocabulary::declare(types, &relations.allow, &relations.require).map_err(
        |detail| ProfileError::BadVocabulary {
            file: file.to_owned(),
            name: name.to_owned(),
            detail,
        },
    )
}

/// `capabilities`, `satisfied_outcome`, `falsifiable_claims` and
/// `standing_coverage` (OW-ADR-0031). An absent field is `defaults`'.
/// Refused: a name outside the closed set, one listed twice, none at all, a
/// capability without its prerequisite, and per-kind data whose capability
/// the kind does not select — an outcome word or a falsifiable claim without
/// `resolution`, standing coverage without `authorization`.
fn parse_kind(
    file: &str,
    name: &str,
    defaults: &KindData,
    raw: &ProfileFile,
) -> Result<KindData, ProfileError> {
    let bad = |detail: String| ProfileError::BadCapabilities {
        file: file.to_owned(),
        name: name.to_owned(),
        detail,
    };
    let capabilities = match raw.capabilities.as_deref() {
        None => defaults.capabilities,
        Some([]) => return Err(bad("`capabilities` selects nothing".to_owned())),
        Some(listed) => {
            let mut seen = Vec::new();
            for found in listed {
                let Some(c) = Capability::parse(found) else {
                    return Err(ProfileError::UnknownCapability {
                        file: file.to_owned(),
                        name: name.to_owned(),
                        found: found.clone(),
                        known: Capability::ALL.map(Capability::as_str).join(", "),
                    });
                };
                if seen.contains(&c) {
                    return Err(bad(format!("capability `{c}` is listed twice")));
                }
                seen.push(c);
            }
            Capabilities::of(&seen)
        }
    };
    if let Some((capability, needs)) = capabilities.missing_prerequisite() {
        return Err(ProfileError::CapabilityPrerequisite {
            file: file.to_owned(),
            name: name.to_owned(),
            capability,
            needs,
        });
    }
    if let Some(word) = raw.satisfied_outcome.as_deref()
        && !is_profile_name(word)
    {
        return Err(bad(format!(
            "satisfied_outcome {word:?} is not a lowercase word ([a-z][a-z0-9_-]*)"
        )));
    }
    let satisfied_outcome = raw
        .satisfied_outcome
        .clone()
        .or_else(|| defaults.satisfied_outcome.clone());
    let falsifiable_claims = raw
        .falsifiable_claims
        .unwrap_or(defaults.falsifiable_claims);
    let standing_coverage = raw.standing_coverage.unwrap_or(defaults.standing_coverage);
    let resolves = capabilities.has(Capability::Resolution);
    if !resolves && (satisfied_outcome.is_some() || falsifiable_claims) {
        return Err(bad(
            "`satisfied_outcome` and `falsifiable_claims` describe a resolution, and the \
             kind does not select `resolution`"
                .to_owned(),
        ));
    }
    if standing_coverage && !capabilities.has(Capability::Authorization) {
        return Err(bad(
            "`standing_coverage` stands in for one authorization, and the kind does not \
             select `authorization`"
                .to_owned(),
        ));
    }
    Ok(KindData {
        capabilities,
        satisfied_outcome,
        falsifiable_claims,
        standing_coverage,
    })
}

/// `form` and `core_roles`: `None` for the contract form, the working
/// form's core roles otherwise. A working form names at least one core role,
/// each an AUTHORED role of the profile it extends, once; it has no
/// acceptance or reference role, because nothing accepts a working record.
fn parse_form(
    file: &str,
    name: &str,
    core: CoreProfile,
    raw: &ProfileFile,
) -> Result<Option<Vec<AtomRole>>, ProfileError> {
    let bad = |detail: String| ProfileError::BadForm {
        file: file.to_owned(),
        name: name.to_owned(),
        detail,
    };
    match raw.form.as_deref() {
        None | Some("contract") => {
            if raw.core_roles.is_some() {
                return Err(bad("`core_roles` without `form = \"working\"`".to_owned()));
            }
            Ok(None)
        }
        Some("working") => {
            let listed = raw.core_roles.as_deref().unwrap_or_default();
            if listed.is_empty() {
                return Err(bad("a working form names at least one core role".to_owned()));
            }
            if raw.acceptance_role.is_some() || !raw.reference_roles.is_empty() {
                return Err(bad(
                    "a working form has no acceptance or reference role; those are the \
                     contract's"
                        .to_owned(),
                ));
            }
            let authored = Profile::from_core(core).required_authored_roles();
            let mut roles = Vec::new();
            for role in listed {
                let parsed = AtomRole::from_str(role)
                    .ok()
                    .filter(|r| authored.contains(r))
                    .ok_or_else(|| {
                        bad(format!(
                            "core role {role:?} is not an authored role of `{core}` ({})",
                            authored
                                .iter()
                                .map(|r| r.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))
                    })?;
                if roles.contains(&parsed) {
                    return Err(bad(format!("core role {role:?} is listed twice")));
                }
                roles.push(parsed);
            }
            Ok(Some(roles))
        }
        Some(other) => Err(bad(format!(
            "form {other:?}; the forms are \"contract\" and \"working\""
        ))),
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

    /// OW-ADR-0031: the core defaults, and the committed files restate them.
    #[test]
    fn core_kinds_select_their_capabilities() {
        let builtin = ProfileRegistry::builtin();
        assert_eq!(builtin.capabilities(&Profile::Delivery), Capabilities::ALL);
        let decision = builtin.capabilities(&Profile::Decision);
        assert!(!decision.has(Capability::Stages));
        assert_eq!(decision, Capabilities::ALL.without(Capability::Stages));
        assert_eq!(
            builtin
                .kind(&Profile::Delivery)
                .satisfied_outcome
                .as_deref(),
            Some("delivered")
        );
        assert!(builtin.kind(&Profile::Delivery).standing_coverage);
        assert!(!builtin.kind(&Profile::Decision).standing_coverage);
        let committed = ProfileRegistry::with_definitions([
            (
                "profiles/delivery.toml",
                include_bytes!("../../../profiles/delivery.toml").as_slice(),
            ),
            (
                "profiles/decision.toml",
                include_bytes!("../../../profiles/decision.toml").as_slice(),
            ),
            (
                "profiles/ticket.toml",
                include_bytes!("../../../profiles/ticket.toml").as_slice(),
            ),
        ])
        .expect("the committed files parse");
        assert_eq!(
            committed.kind(&Profile::Delivery),
            builtin.kind(&Profile::Delivery)
        );
        assert_eq!(
            committed.kind(&Profile::Decision),
            builtin.kind(&Profile::Decision)
        );
        let ticket = committed.resolve("ticket").expect("ticket");
        assert_eq!(committed.capabilities(&ticket), Capabilities::WORKING);
    }

    fn lab_with(extra: &str) -> Result<ProfileRegistry, ProfileError> {
        let text = format!(
            "schema = \"oh.war/profile/v1\"\nname = \"lab\"\nextends = \"delivery\"\n\
             approved = false\n{extra}"
        );
        ProfileRegistry::with_definitions([("profiles/lab.toml", text.as_bytes())])
    }

    /// OW-WAR-0148 M3: a profile declares its record types and relation
    /// kinds; a bad declaration is refused `profile.records`, and a core
    /// file's declaration is kept beside its fixed kind data.
    #[test]
    fn profiles_declare_their_record_vocabulary() {
        let r = lab_with(
            "[records]\ntypes = [\"outcome\", \"requirement\"]\n[relations]\nallow = \
             [\"implements\"]\nrequire = [[\"requirement\", \"implements\", \"outcome\"]]\n",
        )
        .expect("a declared vocabulary parses");
        let v = r.vocabulary("lab").expect("lab");
        assert!(v.has_type("requirement") && v.require.len() == 1);
        assert!(
            r.vocabulary("delivery")
                .is_some_and(crate::relation::Vocabulary::is_empty)
        );
        let err = lab_with("[records]\ntypes = [\"item\"]\n").expect_err("kernel type");
        assert_eq!(err.rule(), "profile.records");
        let err = lab_with("[relations]\nallow = [\"mentions\"]\n").expect_err("unknown kind");
        assert_eq!(err.rule(), "profile.records");
        let err = lab_with("[records]\nkinds = []\n").expect_err("unknown field");
        assert_eq!(err.rule(), "profile.invalid");
        let core = br#"schema = "oh.war/profile/v1"
name = "delivery"
core = true
required_roles = ["control", "intent", "basis", "work_order", "milestones", "assurance", "relations_and_integrity"]

[records]
types = ["risk"]
"#;
        let r = ProfileRegistry::with_definitions([("profiles/delivery.toml", core.as_slice())])
            .expect("a core file may declare a vocabulary");
        assert!(r.vocabulary("delivery").is_some_and(|v| v.has_type("risk")));
        assert_eq!(r.capabilities(&Profile::Delivery), Capabilities::ALL);
    }

    #[test]
    fn capabilities_are_a_closed_set_with_prerequisites() {
        let err = lab_with("capabilities = [\"structure\", \"verification\"]\n")
            .expect_err("verification needs evidence");
        assert_eq!(err.rule(), "profile.capability-prerequisite");
        assert!(matches!(
            err,
            ProfileError::CapabilityPrerequisite {
                capability: Capability::Verification,
                needs: Capability::Evidence,
                ..
            }
        ));
        let err = lab_with("capabilities = [\"structure\", \"telepathy\"]\n").expect_err("unknown");
        assert_eq!(err.rule(), "profile.capability-unknown");
        let err = lab_with("capabilities = [\"links\"]\n").expect_err("links needs structure");
        assert_eq!(err.rule(), "profile.capability-prerequisite");
        let err = lab_with("capabilities = []\n").expect_err("selects nothing");
        assert_eq!(err.rule(), "profile.capabilities");
        let err = lab_with("capabilities = [\"structure\", \"structure\"]\n").expect_err("twice");
        assert_eq!(err.rule(), "profile.capabilities");
        let err = lab_with("capabilities = [\"structure\"]\nsatisfied_outcome = \"done\"\n")
            .expect_err("an outcome word without resolution");
        assert_eq!(err.rule(), "profile.capabilities");
        let err = lab_with("capabilities = [\"structure\"]\nstanding_coverage = true\n")
            .expect_err("standing coverage without authorization");
        assert_eq!(err.rule(), "profile.capabilities");
        // Every refusal not about capabilities keeps its rule.
        assert_eq!(
            lab_with("form = \"nonsense\"\n")
                .expect_err("bad form")
                .rule(),
            "profile.invalid"
        );

        let narrowed = lab_with("capabilities = [\"structure\", \"links\", \"claims\"]\n")
            .expect("a narrowed extension");
        let lab = narrowed.resolve("lab").expect("lab");
        assert_eq!(narrowed.capabilities(&lab), Capabilities::WORKING);
        // An extension with no `capabilities` inherits its core's, and none
        // of the core's name-chosen behaviour.
        let plain = lab_with("").expect("plain");
        let lab = plain.resolve("lab").expect("lab");
        assert_eq!(plain.capabilities(&lab), Capabilities::ALL);
        assert_eq!(plain.kind(&lab).satisfied_outcome, None);
        assert!(!plain.kind(&lab).standing_coverage);
        let data = lab_with(
            "satisfied_outcome = \"concluded\"\nfalsifiable_claims = true\nstanding_coverage = true\n",
        )
        .expect("kind data");
        let lab = data.resolve("lab").expect("lab");
        let kind = data.kind(&lab);
        assert_eq!(kind.satisfied_outcome.as_deref(), Some("concluded"));
        assert!(kind.falsifiable_claims && kind.standing_coverage);
    }

    #[test]
    fn an_extension_narrows_and_never_widens() {
        let wide = "schema = \"oh.war/profile/v1\"\nname = \"memo\"\nextends = \"decision\"\n\
                    approved = false\ncapabilities = [\"structure\", \"stages\"]\n";
        let err = ProfileRegistry::with_definitions([("profiles/memo.toml", wide.as_bytes())])
            .expect_err("decision has no stages");
        assert_eq!(err.rule(), "profile.capabilities");
        let working = "schema = \"oh.war/profile/v1\"\nname = \"task\"\nextends = \"delivery\"\n\
                       approved = false\nform = \"working\"\ncore_roles = [\"intent\"]\n\
                       capabilities = [\"structure\", \"evidence\"]\n";
        let err = ProfileRegistry::with_definitions([("profiles/task.toml", working.as_bytes())])
            .expect_err("a working form never has evidence");
        assert_eq!(err.rule(), "profile.capabilities");
        let restated_wrong = "schema = \"oh.war/profile/v1\"\nname = \"decision\"\ncore = true\n\
            required_roles = [\"control\", \"intent\", \"basis\", \"adr\", \"assurance\", \
            \"relations_and_integrity\"]\ncapabilities = [\"structure\"]\n";
        assert!(matches!(
            ProfileRegistry::with_definitions([(
                "profiles/decision.toml",
                restated_wrong.as_bytes()
            )]),
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

    const WORKING: &str = r##"
schema = "oh.war/profile/v1"
name = "task"
extends = "delivery"
approved = false
form = "working"
core_roles = ["intent"]

[[requires]]
role = "task.checklist"
ordinal = 15
file = "15-checklist.md"
stub = "# Checklist"
"##;

    #[test]
    fn a_working_form_names_its_roles_and_is_known_as_one() {
        let registry =
            ProfileRegistry::with_definitions([("profiles/task.toml", WORKING.as_bytes())])
                .expect("parses");
        let task = registry.resolve("task").expect("admitted");
        assert!(registry.is_working_form(&task));
        assert_eq!(
            registry.definition(&task).expect("defined").working_roles(),
            vec!["intent".to_owned(), "task.checklist".to_owned()]
        );
        // A contract-form extension and the core profiles are not working forms.
        let lab = ProfileRegistry::with_definitions([("profiles/lab.toml", EXT.as_bytes())])
            .expect("parses");
        assert!(!lab.is_working_form(&lab.resolve("lab").expect("lab")));
        assert!(!registry.is_working_form(&Profile::Delivery));
        assert!(
            registry
                .definition(&Profile::Delivery)
                .expect("core")
                .working_roles()
                .is_empty()
        );
    }

    #[test]
    fn a_profile_declares_the_least_a_tick_must_show() {
        use crate::ticks::Level;
        let with = |ticks: &str| {
            format!("{WORKING}\n[fields]\ntypes = [\"bug\", \"chore\"]\n\n[ticks]\n{ticks}\n")
        };
        let registry = ProfileRegistry::with_definitions([(
            "profiles/task.toml",
            with("milestone = \"observed\"\n\n[ticks.types]\nbug = \"independent\"").as_bytes(),
        )])
        .expect("parses");
        let def = registry
            .definition(&registry.resolve("task").expect("task"))
            .expect("def");
        assert_eq!(def.ticks.item, None);
        assert_eq!(def.ticks.milestone, Some(Level::Observed));
        assert_eq!(def.ticks.types.get("bug"), Some(&Level::Independent));
        assert_eq!(
            def.ticks.minimum(Some("bug"), Some(None)).0,
            Level::Independent
        );
        // Absent: every minimum is claimed.
        let plain = ProfileRegistry::with_definitions([("profiles/task.toml", WORKING.as_bytes())])
            .expect("parses");
        let def = plain
            .definition(&plain.resolve("task").expect("task"))
            .expect("def");
        assert_eq!(def.ticks.minimum(None, Some(None)).0, Level::Claimed);
        for bad in [
            "item = \"verified\"",
            "milestone = \"done\"",
            "[ticks.types]\nepic = \"observed\"",
            "colour = \"red\"",
        ] {
            let e =
                ProfileRegistry::with_definitions([("profiles/task.toml", with(bad).as_bytes())])
                    .expect_err(bad);
            assert_eq!(
                e.rule(),
                if bad.starts_with("colour") {
                    "profile.invalid"
                } else {
                    "profile.ticks"
                },
                "{bad}: {e}"
            );
        }
    }

    #[test]
    fn a_working_form_declares_its_fields_and_nothing_else_may() {
        let with = |fields: &str| format!("{WORKING}\n[fields]\n{fields}\n");
        let registry = ProfileRegistry::with_definitions([(
            "profiles/task.toml",
            with("types = [\"bug\", \"chore\"]\nlabels = [\"ui\"]\nlabels_closed = true")
                .as_bytes(),
        )])
        .expect("parses");
        let task = registry.resolve("task").expect("admitted");
        let fields = &registry.definition(&task).expect("defined").fields;
        assert_eq!(fields.types, vec!["bug".to_owned(), "chore".to_owned()]);
        assert!(fields.refuse_type("bug").is_none());
        assert!(fields.refuse_type("epic").is_some());
        assert!(fields.refuse_label("ui").is_none());
        assert!(
            fields
                .refuse_label("api")
                .expect("closed")
                .contains("closed label set")
        );
        // Absent: no types, labels open.
        let plain = ProfileRegistry::with_definitions([("profiles/task.toml", WORKING.as_bytes())])
            .expect("parses");
        let def = plain
            .definition(&plain.resolve("task").expect("task"))
            .expect("def");
        assert!(
            def.fields.refuse_type("bug").is_some() && def.fields.refuse_label("any").is_none()
        );
        for bad in [
            "types = [\"Bug\"]",
            "labels = [\"two words\"]",
            "types = [\"bug\", \"bug\"]",
            "labels_closed = true",
            "colours = [\"red\"]",
        ] {
            let e =
                ProfileRegistry::with_definitions([("profiles/task.toml", with(bad).as_bytes())]);
            assert!(e.is_err(), "{bad}");
        }
        // A contract form has no fields.
        let e = ProfileRegistry::with_definitions([(
            "profiles/lab.toml",
            format!("{EXT}\n[fields]\ntypes = [\"bug\"]\n").as_bytes(),
        )])
        .expect_err("contract form");
        assert_eq!(e.rule(), "profile.fields");
    }

    #[test]
    fn a_working_form_is_refused_on_every_malformation() {
        let refuse = |text: String| {
            ProfileRegistry::with_definitions([("profiles/task.toml", text.as_bytes())])
                .expect_err(&text)
        };
        for (from, to) in [
            // No core roles at all.
            ("core_roles = [\"intent\"]", "core_roles = []"),
            // A compiler-produced role is not authored.
            ("core_roles = [\"intent\"]", "core_roles = [\"control\"]"),
            // `adr` is decision's, not delivery's.
            ("core_roles = [\"intent\"]", "core_roles = [\"adr\"]"),
            (
                "core_roles = [\"intent\"]",
                "core_roles = [\"intent\", \"intent\"]",
            ),
            ("form = \"working\"", "form = \"sketch\""),
            // core_roles belongs to the working form only.
            ("form = \"working\"\n", ""),
        ] {
            assert!(
                matches!(
                    refuse(WORKING.replace(from, to)),
                    ProfileError::BadForm { .. }
                ),
                "{from} -> {to}"
            );
        }
        // Nothing accepts a working record, so it has no acceptance role.
        assert!(matches!(
            refuse(WORKING.replace(
                "form = \"working\"",
                "form = \"working\"\nacceptance_role = \"task.checklist\""
            )),
            ProfileError::BadForm { .. }
        ));
        // A core profile has no working form.
        let core = b"schema = \"oh.war/profile/v1\"\nname = \"delivery\"\ncore = true\nform = \"working\"\nrequired_roles = [\"control\", \"intent\", \"basis\", \"work_order\", \"milestones\", \"assurance\", \"relations_and_integrity\"]\n";
        assert!(matches!(
            ProfileRegistry::with_definitions([("profiles/delivery.toml", core.as_slice())]),
            Err(ProfileError::CoreRedefined { .. })
        ));
    }
}
