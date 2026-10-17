// SPDX-License-Identifier: Apache-2.0
//! Format 1.0 profile data (`docs/format/FORMAT-1.0.md` sections 7.4 to
//! 7.6): what a kind of document is, as a TOML file `profiles/<name>.toml`
//! of schema `oh.war/profile/v1`.
//!
//! # Two readers of one directory
//!
//! `profiles/` also holds the files [`crate::role::ProfileRegistry`] reads:
//! the contract and working forms of a Warrant profile, and the document
//! types of OW-WAR-0148. Those files read exactly as before, and they never
//! govern a Format 1.0 document. A file is a Format 1.0 profile
//! ([`is_format_profile`]) when it declares `format` (7.5: `format =
//! "openwarrant/1.0"`); the legacy registry skips such a file, and this
//! module reads it with every key closed (`deny_unknown_fields`). A file
//! without `format` is a legacy profile (section 14).
//!
//! # Built-ins
//!
//! The profiles of section 7.6 need no file ([`builtin`]). They are
//! defined by TOML files compiled into this crate and read by the same
//! parser, so a built-in and a repository's file have one shape. A file
//! named like a built-in replaces it only by `extends` (7.5).
//!
//! # Refusals
//!
//! A profile file with an unknown key, a capability outside the closed set
//! or without its prerequisite, a `refines` naming no fixed state, or a
//! preset named like a built-in one is refused (`profile.invalid`); a
//! document naming it reads as `format.profile-unknown` (7.5).
//!
//! Pure: a parse over bytes.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::ticks::Level;

/// The schema a profile file declares.
pub const SCHEMA: &str = "oh.war/profile/v1";

/// 7.6: the closed set of capabilities.
pub const CAPABILITIES: [&str; 10] = [
    "structure",
    "links",
    "claims",
    "checks",
    "stages",
    "acceptance",
    "evidence",
    "verification",
    "authorization",
    "resolution",
];

/// 7.6: a capability's prerequisites, `(capability, needs)`, besides
/// `structure`, which every other capability needs.
pub const PREREQUISITES: [(&str, &str); 3] = [
    ("verification", "evidence"),
    ("resolution", "authorization"),
    ("checks", "claims"),
];

/// 11.1: the fixed states a declared state may refine.
pub const FIXED_STATES: [&str; 13] = [
    "draft",
    "open",
    "in_progress",
    "done",
    "blocked",
    "claimed",
    "superseded",
    "failing",
    "accepted",
    "partially_accepted",
    "authorized",
    "verified",
    "resolved",
];

/// 10.4: the built-in presets, which no profile may redefine.
pub const BUILTIN_PRESETS: [&str; 5] = ["agent", "agent.min", "human", "review", "resume"];

/// The built-in profiles of 7.6, as `(name, file bytes)`: byte-identical
/// copies of the normative files at `schemas/format/1.0/profiles/` (#295).
/// A unit test holds each copy to its file whenever the file is present.
const BUILTIN: [(&str, &str); 11] = [
    ("adr", include_str!("format_profiles/adr.toml")),
    (
        "architecture",
        include_str!("format_profiles/architecture.toml"),
    ),
    ("decision", include_str!("format_profiles/decision.toml")),
    ("delivery", include_str!("format_profiles/delivery.toml")),
    ("incident", include_str!("format_profiles/incident.toml")),
    ("prd", include_str!("format_profiles/prd.toml")),
    ("release", include_str!("format_profiles/release.toml")),
    ("roadmap", include_str!("format_profiles/roadmap.toml")),
    ("spec", include_str!("format_profiles/spec.toml")),
    ("test-plan", include_str!("format_profiles/test-plan.toml")),
    ("ticket", include_str!("format_profiles/ticket.toml")),
];

/// The names of the built-in profiles.
#[must_use]
pub fn builtin_names() -> Vec<&'static str> {
    BUILTIN.iter().map(|(n, _)| *n).collect()
}

/// The built-in profile named `name`, if there is one.
#[must_use]
pub fn builtin(name: &str) -> Option<Profile> {
    let (_, text) = BUILTIN.iter().find(|(n, _)| *n == name)?;
    // A built-in that does not parse is a defect of this crate; the unit
    // tests read every one.
    parse(&format!("{name}.toml"), text.as_bytes(), &|_| None).ok()
}

/// A profile's form (7.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// Its documents are work documents (4.1).
    Work,
    /// Its documents are record documents or specifications (4.4, 4.5).
    Document,
}

/// A field value's shape (7.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Text,
    Line,
    Int,
    Number,
    Bool,
    Id,
    Ids,
    Gate,
    Date,
    Enum,
}

impl Shape {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "text" => Self::Text,
            "line" => Self::Line,
            "int" => Self::Int,
            "number" => Self::Number,
            "bool" => Self::Bool,
            "id" => Self::Id,
            "ids" => Self::Ids,
            "gate" => Self::Gate,
            "date" => Self::Date,
            "enum" => Self::Enum,
            _ => return None,
        })
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Line => "line",
            Self::Int => "int",
            Self::Number => "number",
            Self::Bool => "bool",
            Self::Id => "id",
            Self::Ids => "ids",
            Self::Gate => "gate",
            Self::Date => "date",
            Self::Enum => "enum",
        }
    }
}

/// One declared field: `<name> = { required, shape, values }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDecl {
    pub required: bool,
    pub shape: Shape,
    /// For `enum`: the admitted values.
    pub values: Vec<String>,
}

impl FieldDecl {
    /// Why `value` does not have this field's shape, if it does not.
    #[must_use]
    pub fn refuse(&self, value: &str) -> Option<String> {
        let ok = match self.shape {
            Shape::Text => true,
            Shape::Line => !value.contains('\n'),
            Shape::Int => is_int(value),
            Shape::Number => is_number(value),
            Shape::Bool => matches!(value, "true" | "false"),
            Shape::Id => crate::format::parse_target(value.trim()).is_some(),
            Shape::Ids => value
                .split(", ")
                .all(|t| crate::format::parse_target(t.trim()).is_some()),
            Shape::Gate => value.split(", ").all(|g| is_gate_uri(g.trim())),
            Shape::Date => is_date(value),
            Shape::Enum => self.values.iter().any(|v| v == value),
        };
        (!ok).then(|| match self.shape {
            Shape::Enum => format!("{value:?} is not one of {}", self.values.join(", ")),
            Shape::Line => "a `line` field holds one line".to_owned(),
            Shape::Int => format!("{value:?} is not an integer"),
            Shape::Number => format!("{value:?} is not a number"),
            Shape::Bool => format!("{value:?} is not `true` or `false`"),
            Shape::Id => format!("{value:?} is not an id (section 5)"),
            Shape::Ids => format!("{value:?} is not a comma-separated list of ids (section 5)"),
            Shape::Gate => format!("{value:?} is not `gate://<id>@<version>`, comma-separated"),
            Shape::Date => format!("{value:?} is not a date YYYY-MM-DD"),
            Shape::Text => String::new(),
        })
    }
}

fn is_int(v: &str) -> bool {
    let d = v.strip_prefix('-').unwrap_or(v);
    !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit())
}

/// 7.5 `number`: an `int`, optionally followed by `.` and digits.
#[must_use]
pub fn is_number(v: &str) -> bool {
    match v.split_once('.') {
        None => is_int(v),
        Some((i, f)) => is_int(i) && !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()),
    }
}

/// Appendix A `date`: `YYYY-MM-DD`, digits only.
#[must_use]
pub fn is_date(v: &str) -> bool {
    let b = v.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// Appendix A `gate-uri`: `gate://<gate-id>@<semver>`.
#[must_use]
pub fn is_gate_uri(v: &str) -> bool {
    let Some(rest) = v.strip_prefix("gate://") else {
        return false;
    };
    let Some((id, version)) = rest.split_once('@') else {
        return false;
    };
    let label = |s: &str| {
        let mut c = s.chars();
        c.next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            && c.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    };
    let core = version.split_once('-').map_or(version, |(c, _)| c);
    let nums: Vec<&str> = core.split('.').collect();
    id.split('.').all(label)
        && nums.len() == 3
        && nums
            .iter()
            .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// One `[[states]]` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDecl {
    pub name: String,
    pub refines: String,
    pub note: Option<String>,
}

/// `[ticks]`: the minimum tick level (11.4).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ticks {
    pub item: Option<Level>,
    pub milestone: Option<Level>,
    pub types: BTreeMap<String, Level>,
}

/// A preset's budget: bytes, or `"none"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Budget {
    Bytes(u64),
    None,
}

/// One `[[presets]]` entry (10.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    pub name: String,
    pub steps: Vec<String>,
    pub depth: Option<u32>,
    pub budget: Option<Budget>,
    pub omit_types: Vec<String>,
    pub omit_fields: Vec<String>,
    pub editable: bool,
    pub reads_state: bool,
}

/// One profile, its `extends` already applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    pub extends: Option<String>,
    /// Every profile it extends, nearest last: `bugfix` extending `ticket`
    /// has `["ticket"]`.
    pub ancestors: Vec<String>,
    pub form: Form,
    pub note: Option<String>,
    /// From the closed set of 7.6, in its order.
    pub capabilities: Vec<&'static str>,
    pub frontmatter_required: Vec<String>,
    pub frontmatter_optional: Vec<String>,
    pub sections_required: Vec<String>,
    pub sections_optional: Vec<String>,
    pub sections_closed: bool,
    pub record_types: Vec<String>,
    /// `[records] closed = true`: an undotted type neither the core nor
    /// `record_types` names is refused (`format.key-unknown`).
    pub records_closed: bool,
    /// Per record type, per field.
    pub fields: BTreeMap<String, BTreeMap<String, FieldDecl>>,
    pub relations_allow: Vec<String>,
    pub relations_require: Vec<[String; 3]>,
    pub states: Vec<StateDecl>,
    pub ticks: Ticks,
    pub presets: Vec<Preset>,
}

impl Profile {
    /// Whether it selects `capability`.
    #[must_use]
    pub fn has(&self, capability: &str) -> bool {
        self.capabilities.contains(&capability)
    }

    /// The undotted frontmatter keys it adds to those of 3.3.
    pub fn frontmatter_keys(&self) -> impl Iterator<Item = &str> {
        self.frontmatter_required
            .iter()
            .chain(&self.frontmatter_optional)
            .map(String::as_str)
    }

    /// The fields it declares for records of `record_type`.
    #[must_use]
    pub fn fields_of(&self, record_type: &str) -> Option<&BTreeMap<String, FieldDecl>> {
        self.fields.get(record_type)
    }
}

// ---- the file --------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    schema: String,
    format: String,
    name: String,
    #[serde(default)]
    extends: Option<String>,
    #[serde(default)]
    form: Option<String>,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    capabilities: Option<Vec<String>>,
    #[serde(default)]
    frontmatter: Option<RequiredOptional>,
    #[serde(default)]
    sections: Option<SectionsFile>,
    #[serde(default)]
    records: Option<RecordsFile>,
    #[serde(default)]
    fields: BTreeMap<String, BTreeMap<String, FieldFile>>,
    #[serde(default)]
    relations: Option<RelationsFile>,
    #[serde(default)]
    states: Vec<StateFile>,
    #[serde(default)]
    ticks: Option<TicksFile>,
    #[serde(default)]
    presets: Vec<PresetFile>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequiredOptional {
    #[serde(default)]
    required: Vec<String>,
    #[serde(default)]
    optional: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SectionsFile {
    #[serde(default)]
    required: Vec<String>,
    #[serde(default)]
    optional: Vec<String>,
    #[serde(default)]
    closed: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordsFile {
    #[serde(default)]
    types: Vec<String>,
    #[serde(default)]
    closed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldFile {
    #[serde(default)]
    required: bool,
    #[serde(default)]
    shape: Option<String>,
    #[serde(default)]
    values: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RelationsFile {
    #[serde(default)]
    allow: Vec<String>,
    #[serde(default)]
    require: Vec<[String; 3]>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StateFile {
    name: String,
    refines: String,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct TicksFile {
    #[serde(default)]
    item: Option<String>,
    #[serde(default)]
    milestone: Option<String>,
    #[serde(default)]
    types: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetFile {
    name: String,
    #[serde(default)]
    steps: Vec<String>,
    #[serde(default)]
    depth: Option<u32>,
    #[serde(default)]
    budget: Option<toml::Value>,
    #[serde(default)]
    omit_types: Vec<String>,
    #[serde(default)]
    omit_fields: Vec<String>,
    #[serde(default)]
    editable: bool,
    #[serde(default)]
    reads_state: bool,
}

/// Why a profile file was refused (`profile.invalid`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalid {
    pub file: String,
    pub detail: String,
}

impl std::fmt::Display for Invalid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.file, self.detail)
    }
}

/// The rule a refused profile file is reported under.
pub const INVALID_RULE: &str = "profile.invalid";

/// Whether `bytes` are a Format 1.0 profile file rather than one the legacy
/// profile registry reads (module docs). A file that is not TOML is not
/// one: the legacy reader refuses it as it always did.
#[must_use]
pub fn is_format_profile(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|t| toml::from_str::<toml::Table>(t).ok())
        .is_some_and(|t| t.contains_key("format"))
}

/// The keys of 7.5 that no legacy profile file has: a file with one of them
/// and no `format` key was written as a Format 1.0 profile and lacks the
/// key that makes it one. (`[fields.<type>]` is one too: a legacy
/// `[fields]` holds lists and flags, never a table per record type.)
const FORMAT_ONLY_KEYS: [&str; 3] = ["frontmatter", "sections", "presets"];

/// Whether `bytes` are a profile file written in the shape of 7.5 that does
/// not declare `format` (a `[sections]` or `[fields.<type>]` table, ...).
/// FORMAT-1.0 §7.5: "a profile file without that key is a legacy profile
/// (section 14) and never governs a Format 1.0 document". Such a file is
/// no legacy definition either, so the legacy registry does not read it
/// (and does not refuse the repository over it): it governs nothing, and a
/// document naming it reads `format.profile-unknown`.
#[must_use]
pub fn is_format_shaped_legacy(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|t| toml::from_str::<toml::Table>(t).ok())
        .is_some_and(|t| {
            let per_type_fields = t
                .get("fields")
                .and_then(toml::Value::as_table)
                .is_some_and(|f| f.values().any(toml::Value::is_table));
            !t.contains_key("format")
                && (per_type_fields || FORMAT_ONLY_KEYS.iter().any(|k| t.contains_key(*k)))
        })
}

fn is_word(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|c| c.is_ascii_lowercase())
        && c.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn is_name(s: &str) -> bool {
    is_word(s) || crate::format::is_dotted(s)
}

/// Parse one profile file. `file` is its path (its stem is its name);
/// `lookup` finds the profile an `extends` names, already parsed. A file
/// named like a built-in extends that built-in when `extends` names its
/// own name.
pub fn parse(
    file: &str,
    bytes: &[u8],
    lookup: &dyn Fn(&str) -> Option<Profile>,
) -> Result<Profile, Invalid> {
    let bad = |detail: String| Invalid {
        file: file.to_owned(),
        detail,
    };
    let text = std::str::from_utf8(bytes).map_err(|e| bad(format!("not UTF-8: {e}")))?;
    let raw: File =
        toml::from_str(text).map_err(|e: toml::de::Error| bad(e.message().to_owned()))?;
    if crate::format::parse_version(&raw.format).is_none_or(|(major, _)| major != 1) {
        return Err(bad(format!(
            "format {:?}; a Format 1.0 profile declares `format = \"openwarrant/1.0\"`",
            raw.format
        )));
    }
    if raw.schema != SCHEMA {
        return Err(bad(format!(
            "schema {:?}; a profile is {SCHEMA:?}",
            raw.schema
        )));
    }
    let stem = file.strip_suffix(".toml").unwrap_or(file);
    let stem = stem.rsplit('/').next().unwrap_or(stem);
    if raw.name != stem || !is_word(&raw.name) {
        return Err(bad(format!(
            "name {:?}: a profile's name is a lowercase word equal to its file stem ({stem:?})",
            raw.name
        )));
    }
    // 7.5: the base it starts from.
    let base = match raw.extends.as_deref() {
        None => None,
        Some(parent) => {
            let found = if parent == raw.name {
                builtin(parent)
            } else {
                lookup(parent)
            };
            Some(found.ok_or_else(|| {
                bad(format!(
                    "extends {parent:?}, which is no profile this reader can find"
                ))
            })?)
        }
    };
    let form = match (raw.form.as_deref(), &base) {
        (Some("work"), _) => Form::Work,
        (Some("document"), _) => Form::Document,
        (Some(other), _) => {
            return Err(bad(format!(
                "form {other:?}; a Format 1.0 profile's form is \"work\" or \"document\""
            )));
        }
        (None, Some(b)) => b.form,
        (None, None) => {
            return Err(bad(
                "no `form`: a profile is `form = \"work\"` or `form = \"document\"`".to_owned(),
            ));
        }
    };
    if let Some(b) = &base
        && b.form != form
    {
        return Err(bad(format!(
            "form differs from the form of {:?}, which it extends",
            b.name
        )));
    }
    let mut p = base.clone().unwrap_or(Profile {
        name: String::new(),
        extends: None,
        ancestors: Vec::new(),
        form,
        note: None,
        capabilities: Vec::new(),
        frontmatter_required: Vec::new(),
        frontmatter_optional: Vec::new(),
        sections_required: Vec::new(),
        sections_optional: Vec::new(),
        sections_closed: false,
        record_types: Vec::new(),
        records_closed: false,
        fields: BTreeMap::new(),
        relations_allow: Vec::new(),
        relations_require: Vec::new(),
        states: Vec::new(),
        ticks: Ticks::default(),
        presets: Vec::new(),
    });
    if let Some(b) = &base {
        p.ancestors.push(b.name.clone());
    }
    p.name.clone_from(&raw.name);
    p.extends.clone_from(&raw.extends);
    p.form = form;
    if raw.note.is_some() {
        p.note.clone_from(&raw.note);
    }

    // Capabilities: the closed set and its prerequisites (7.6). An
    // extension adds; it never removes what its base selects.
    let mut caps: BTreeSet<&'static str> = p.capabilities.iter().copied().collect();
    for c in raw.capabilities.iter().flatten() {
        let Some(known) = CAPABILITIES.iter().find(|k| *k == c) else {
            return Err(bad(format!(
                "capability {c:?} is not one of the closed set ({})",
                CAPABILITIES.join(", ")
            )));
        };
        caps.insert(known);
    }
    let structure = caps
        .iter()
        .find(|c| **c != "structure")
        .map(|c| (*c, "structure"));
    for (c, needs) in PREREQUISITES.into_iter().chain(structure) {
        if caps.contains(c) && !caps.contains(needs) {
            return Err(bad(format!(
                "capability `{c}` without `{needs}`, which it needs"
            )));
        }
    }
    p.capabilities = CAPABILITIES
        .iter()
        .copied()
        .filter(|c| caps.contains(c))
        .collect();

    // [frontmatter]: undotted keys beyond 3.3's.
    if let Some(fm) = &raw.frontmatter {
        for k in fm.required.iter().chain(&fm.optional) {
            if !is_word(k) || crate::format::CORE_KEYS.contains(&k.as_str()) {
                return Err(bad(format!(
                    "[frontmatter] {k:?}: a declared key is an undotted word that is not a key \
                     of section 3.3"
                )));
            }
        }
        merge_required(
            &mut p.frontmatter_required,
            &mut p.frontmatter_optional,
            &fm.required,
            &fm.optional,
        );
    }
    // [sections].
    if let Some(s) = &raw.sections {
        for name in s.required.iter().chain(&s.optional) {
            if crate::format::slug(name).is_empty() {
                return Err(bad(format!("[sections] {name:?} has an empty slug")));
            }
        }
        merge_required(
            &mut p.sections_required,
            &mut p.sections_optional,
            &s.required,
            &s.optional,
        );
        p.sections_closed |= s.closed;
    }
    // [records] types.
    if let Some(r) = &raw.records {
        for t in &r.types {
            if !is_name(t) {
                return Err(bad(format!(
                    "[records] type {t:?} is not a lowercase word or a dotted name"
                )));
            }
            if !p.record_types.contains(t) {
                p.record_types.push(t.clone());
            }
        }
        p.records_closed |= r.closed;
    }
    // [fields.<type>].
    for (ty, fields) in &raw.fields {
        if !is_name(ty) {
            return Err(bad(format!(
                "[fields.{ty}]: {ty:?} is not a record type name"
            )));
        }
        let into = p.fields.entry(ty.clone()).or_default();
        for (name, f) in fields {
            if !is_word(name) {
                return Err(bad(format!(
                    "[fields.{ty}] {name:?}: a declared field is an undotted word"
                )));
            }
            let shape = match f.shape.as_deref() {
                None => Shape::Text,
                Some(s) => Shape::parse(s).ok_or_else(|| {
                    bad(format!(
                        "[fields.{ty}] {name}: shape {s:?} is not text, line, int, number, date, id, \
                         ids, gate, enum or bool"
                    ))
                })?,
            };
            if (shape == Shape::Enum) == f.values.is_empty() {
                return Err(bad(format!(
                    "[fields.{ty}] {name}: `values` is given exactly when the shape is `enum`"
                )));
            }
            let required = f.required || into.get(name).is_some_and(|d| d.required);
            into.insert(
                name.clone(),
                FieldDecl {
                    required,
                    shape,
                    values: f.values.clone(),
                },
            );
        }
    }
    // [relations].
    if let Some(r) = &raw.relations {
        for k in &r.allow {
            if crate::relation::CoreKind::parse(k).is_none() {
                return Err(bad(format!(
                    "[relations] allow {k:?}: not a core relation kind (3.6)"
                )));
            }
            if !p.relations_allow.contains(k) {
                p.relations_allow.push(k.clone());
            }
        }
        for req in &r.require {
            if crate::relation::CoreKind::parse(&req[1]).is_none() {
                return Err(bad(format!(
                    "[relations] require {req:?}: {:?} is not a core relation kind",
                    req[1]
                )));
            }
            if !p.relations_require.contains(req) {
                p.relations_require.push(req.clone());
            }
        }
    }
    // [[states]].
    for s in &raw.states {
        if !is_word(&s.name) {
            return Err(bad(format!(
                "[[states]] {:?} is not a lowercase word",
                s.name
            )));
        }
        if !FIXED_STATES.contains(&s.refines.as_str()) {
            return Err(bad(format!(
                "[[states]] {}: refines {:?}, which is no fixed state ({})",
                s.name,
                s.refines,
                FIXED_STATES.join(", ")
            )));
        }
        if FIXED_STATES.contains(&s.name.as_str()) || p.states.iter().any(|d| d.name == s.name) {
            return Err(bad(format!(
                "[[states]] {} is declared twice, or is a fixed state",
                s.name
            )));
        }
        p.states.push(StateDecl {
            name: s.name.clone(),
            refines: s.refines.clone(),
            note: s.note.clone(),
        });
    }
    // [ticks].
    if let Some(t) = &raw.ticks {
        let level = |what: &str, v: &str| {
            Level::parse(v).ok_or_else(|| {
                bad(format!(
                    "[ticks] {what} {v:?} is not claimed, observed, independent or signed"
                ))
            })
        };
        if let Some(v) = &t.item {
            p.ticks.item = Some(level("item", v)?);
        }
        if let Some(v) = &t.milestone {
            p.ticks.milestone = Some(level("milestone", v)?);
        }
        for (ty, v) in &t.types {
            let l = level(&format!("types.{ty}"), v)?;
            p.ticks.types.insert(ty.clone(), l);
        }
    }
    // [[presets]].
    for pr in &raw.presets {
        if BUILTIN_PRESETS.contains(&pr.name.as_str()) {
            return Err(bad(format!(
                "[[presets]] {:?} is a built-in preset's name (10.4); a profile may not \
                 redefine one",
                pr.name
            )));
        }
        if !is_name(&pr.name) || p.presets.iter().any(|q| q.name == pr.name) {
            return Err(bad(format!(
                "[[presets]] {:?}: a preset's name is a word or dotted name, once",
                pr.name
            )));
        }
        let budget = match &pr.budget {
            None => None,
            Some(toml::Value::String(s)) if s == "none" => Some(Budget::None),
            Some(toml::Value::Integer(n)) if *n >= 0 => {
                Some(Budget::Bytes(u64::try_from(*n).unwrap_or_default()))
            }
            Some(other) => {
                return Err(bad(format!(
                    "[[presets]] {}: budget {other} is not a byte count or \"none\"",
                    pr.name
                )));
            }
        };
        p.presets.push(Preset {
            name: pr.name.clone(),
            steps: pr.steps.clone(),
            depth: pr.depth,
            budget,
            omit_types: pr.omit_types.clone(),
            omit_fields: pr.omit_fields.clone(),
            editable: pr.editable,
            reads_state: pr.reads_state,
        });
    }
    Ok(p)
}

/// Add `required` and `optional` to a declaration: a name required by
/// either is required (an extension narrows, never loosens).
fn merge_required(
    into_required: &mut Vec<String>,
    into_optional: &mut Vec<String>,
    required: &[String],
    optional: &[String],
) {
    for r in required {
        into_optional.retain(|o| o != r);
        if !into_required.contains(r) {
            into_required.push(r.clone());
        }
    }
    for o in optional {
        if !into_required.contains(o) && !into_optional.contains(o) {
            into_optional.push(o.clone());
        }
    }
}

/// A repository's profiles: its Format 1.0 profile files over the
/// built-ins.
#[derive(Debug, Clone, Default)]
pub struct Profiles {
    found: BTreeMap<String, Profile>,
    /// Names whose file was refused: their documents read
    /// `format.profile-unknown`, never a built-in of the same name.
    refused: BTreeSet<String>,
    /// Every refusal, by file.
    pub invalid: Vec<Invalid>,
}

/// What looking a profile up found.
#[derive(Debug, Clone, Copy)]
pub enum Lookup<'a> {
    Found(&'a Profile),
    /// A file of that name was refused (`profile.invalid`).
    Invalid,
    /// No file and no built-in of that name.
    Missing,
}

impl Profiles {
    /// The built-ins alone.
    #[must_use]
    pub fn builtins() -> Self {
        Self::load(std::iter::empty())
    }

    /// The built-ins, then each `(file, bytes)` that [`is_format_profile`]
    /// (any other file is the legacy registry's). A file may extend
    /// another file's profile; a cycle or a missing base is refused.
    pub fn load<'a>(files: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> Self {
        let mut out = Self::default();
        for name in builtin_names() {
            if let Some(p) = builtin(name) {
                out.found.insert(name.to_owned(), p);
            }
        }
        let pending: BTreeMap<String, (&str, &[u8])> = files
            .into_iter()
            .filter(|(_, b)| is_format_profile(b))
            .map(|(f, b)| {
                let s = f.strip_suffix(".toml").unwrap_or(f);
                (s.rsplit('/').next().unwrap_or(s).to_owned(), (f, b))
            })
            .collect();
        let mut done: BTreeMap<String, Result<Profile, Invalid>> = BTreeMap::new();
        for name in pending.keys() {
            resolve(name, &pending, &mut done, &mut Vec::new());
        }
        for (name, r) in done {
            match r {
                Ok(p) => {
                    out.found.insert(name, p);
                }
                Err(e) => {
                    out.found.remove(&name);
                    out.refused.insert(name);
                    out.invalid.push(e);
                }
            }
        }
        out
    }

    /// The profile named `name`.
    #[must_use]
    pub fn lookup(&self, name: &str) -> Lookup<'_> {
        if self.refused.contains(name) {
            return Lookup::Invalid;
        }
        self.found.get(name).map_or(Lookup::Missing, Lookup::Found)
    }

    /// Every profile read from a file, by name.
    pub fn found(&self) -> impl Iterator<Item = &Profile> {
        self.found.values()
    }
}

type Pending<'a> = BTreeMap<String, (&'a str, &'a [u8])>;

/// The profile `name`: a pending file's, parsed once over its base, else a
/// built-in. A name already on `stack` is a cycle and resolves to nothing,
/// so the file that closes it is refused for a base it cannot find.
fn resolve(
    name: &str,
    pending: &Pending<'_>,
    done: &mut BTreeMap<String, Result<Profile, Invalid>>,
    stack: &mut Vec<String>,
) -> Option<Profile> {
    if let Some(r) = done.get(name) {
        return r.as_ref().ok().cloned();
    }
    let Some(&(file, bytes)) = pending.get(name) else {
        return builtin(name);
    };
    if stack.iter().any(|s| s == name) {
        return None;
    }
    stack.push(name.to_owned());
    let parent = std::str::from_utf8(bytes)
        .ok()
        .and_then(|t| toml::from_str::<toml::Table>(t).ok())
        .and_then(|t| t.get("extends").and_then(|v| v.as_str()).map(str::to_owned));
    let base = parent
        .as_deref()
        .filter(|p| *p != name)
        .and_then(|p| resolve(p, pending, done, stack));
    stack.pop();
    let found = |n: &str| {
        (parent.as_deref() == Some(n))
            .then(|| base.clone())
            .flatten()
    };
    let parsed = parse(file, bytes, &found).and_then(|p| {
        // 7.5: a file named like a built-in replaces it only by `extends`.
        if builtin(&p.name).is_some() && p.extends.is_none() {
            Err(Invalid {
                file: file.to_owned(),
                detail: format!(
                    "names built-in profile {:?} without `extends`; a file of a built-in's \
                     name replaces it only by extending it",
                    p.name
                ),
            })
        } else {
            Ok(p)
        }
    });
    let out = parsed.as_ref().ok().cloned();
    done.insert(name.to_owned(), parsed);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> Option<Profile> {
        None
    }

    #[test]
    fn every_builtin_parses_and_matches_section_7_6() {
        for name in builtin_names() {
            let p = builtin(name).unwrap_or_else(|| panic!("{name} does not parse"));
            assert_eq!(p.name, name);
        }
        let caps = |n: &str| builtin(n).map(|p| p.capabilities).unwrap_or_default();
        assert_eq!(caps("ticket"), ["structure", "links", "claims", "checks"]);
        assert_eq!(caps("delivery"), CAPABILITIES);
        assert!(!caps("decision").contains(&"stages"));
        assert_eq!(caps("decision").len(), 9);
        assert_eq!(caps("spec"), ["structure", "links", "acceptance"]);
        assert_eq!(caps("adr"), ["structure", "links", "acceptance"]);
        assert_eq!(caps("roadmap"), ["structure", "links", "claims"]);
        for n in ["release", "incident", "prd", "architecture", "test-plan"] {
            assert_eq!(caps(n), ["structure", "links"], "{n}");
        }
        let d = builtin("delivery").unwrap();
        assert_eq!(
            d.sections_required,
            ["Intent", "Basis", "Work order", "Assurance"]
        );
        assert_eq!(builtin("ticket").unwrap().sections_required, ["Checklist"]);
        assert_eq!(
            builtin("adr").unwrap().sections_required,
            ["Context", "Decision", "Consequences"]
        );
        let spec = builtin("spec").unwrap();
        assert_eq!(spec.form, Form::Document);
        assert!(spec.fields_of("requirement").unwrap()["statement"].required);
    }

    #[test]
    fn a_full_profile_file_reads() {
        let text = br#"schema = "oh.war/profile/v1"
format = "openwarrant/1.0"
name = "runbook"
form = "document"
note = "How to operate a service."
capabilities = ["structure", "links", "claims", "checks"]
[frontmatter]
required = ["service"]
optional = ["owner"]
[sections]
required = ["Steps"]
optional = ["Rollback"]
closed = true
[records]
types = ["step"]
[fields.step]
command = { required = true, shape = "line" }
timeout = { shape = "int" }
risk = { shape = "enum", values = ["low", "high"] }
[relations]
allow = ["depends_on"]
require = [["step", "depends_on", "step"]]
[[states]]
name = "in_review"
refines = "in_progress"
[ticks]
item = "observed"
[[presets]]
name = "oncall"
steps = ["out:depends_on"]
depth = 2
budget = "none"
omit_fields = ["risk"]
reads_state = true
"#;
        let p = parse("profiles/runbook.toml", text, &none).unwrap();
        assert_eq!(p.form, Form::Document);
        assert_eq!(p.frontmatter_required, ["service"]);
        assert!(p.sections_closed);
        assert_eq!(p.fields_of("step").unwrap()["timeout"].shape, Shape::Int);
        assert_eq!(p.ticks.item, Some(Level::Observed));
        assert_eq!(p.presets[0].budget, Some(Budget::None));
        assert!(is_format_profile(text));
    }

    #[test]
    fn each_refusal_is_profile_invalid() {
        let head = "schema = \"oh.war/profile/v1\"\nformat = \"openwarrant/1.0\"\nname = \"x\"\nform = \"work\"\n";
        let cases: &[(&str, &str)] = &[
            ("unknown key", "colour = \"red\"\n"),
            (
                "unknown capability",
                "capabilities = [\"structure\", \"magic\"]\n",
            ),
            (
                "checks without claims",
                "capabilities = [\"structure\", \"checks\"]\n",
            ),
            (
                "verification without evidence",
                "capabilities = [\"verification\"]\n",
            ),
            (
                "refines no fixed state",
                "[[states]]\nname = \"waiting\"\nrefines = \"limbo\"\n",
            ),
            ("built-in preset", "[[presets]]\nname = \"agent.min\"\n"),
            ("bad shape", "[fields.step]\nrun = { shape = \"blob\" }\n"),
            (
                "enum without values",
                "[fields.step]\nrun = { shape = \"enum\" }\n",
            ),
            ("bad tick", "[ticks]\nitem = \"vibes\"\n"),
            ("unknown table key", "[sections]\nwanted = [\"A\"]\n"),
            ("links without structure", "capabilities = [\"links\"]\n"),
            ("extends nothing", "extends = \"nowhere\"\n"),
            ("form other than the base's", "extends = \"adr\"\n"),
            (
                "closed records with a bad type",
                "[records]\ntypes = [\"Bad Type\"]\nclosed = true\n",
            ),
        ];
        for (what, tail) in cases {
            let text = format!("{head}{tail}");
            assert!(
                parse("profiles/x.toml", text.as_bytes(), &none).is_err(),
                "{what} was accepted"
            );
        }
        // The accepting side: the same file without the fault.
        assert!(parse("profiles/x.toml", head.as_bytes(), &none).is_ok());
        assert!(
            parse(
                "profiles/x.toml",
                format!("{head}capabilities = [\"structure\", \"claims\", \"checks\"]\n")
                    .as_bytes(),
                &none
            )
            .is_ok()
        );
        // The name is the file stem.
        assert!(parse("profiles/y.toml", head.as_bytes(), &none).is_err());
    }

    #[test]
    fn extends_adds_and_narrows() {
        let files: Vec<(&str, &[u8])> = vec![
            (
                "profiles/bugfix.toml",
                b"schema = \"oh.war/profile/v1\"\nformat = \"openwarrant/1.0\"\nname = \"bugfix\"\nextends = \"ticket\"\n\
[sections]\nrequired = [\"Reproduction\"]\n",
            ),
            (
                "profiles/hotfix.toml",
                b"schema = \"oh.war/profile/v1\"\nformat = \"openwarrant/1.0\"\nname = \"hotfix\"\nextends = \"bugfix\"\n\
[frontmatter]\nrequired = [\"due\"]\n",
            ),
            // A same-named file without `extends` is refused, and the
            // built-in is not used in its place.
            (
                "profiles/adr.toml",
                b"schema = \"oh.war/profile/v1\"\nformat = \"openwarrant/1.0\"\nname = \"adr\"\nform = \"document\"\n\
[sections]\nrequired = [\"Context\"]\n",
            ),
            // A cycle.
            (
                "profiles/a.toml",
                b"schema = \"oh.war/profile/v1\"\nformat = \"openwarrant/1.0\"\nname = \"a\"\nextends = \"b\"\n[sections]\n",
            ),
            (
                "profiles/b.toml",
                b"schema = \"oh.war/profile/v1\"\nformat = \"openwarrant/1.0\"\nname = \"b\"\nextends = \"a\"\n[sections]\n",
            ),
        ];
        let ps = Profiles::load(files);
        let Lookup::Found(h) = ps.lookup("hotfix") else {
            panic!("hotfix: {:?}", ps.invalid)
        };
        assert_eq!(h.sections_required, ["Checklist", "Reproduction"]);
        assert_eq!(h.frontmatter_required, ["due"]);
        assert!(!h.frontmatter_optional.contains(&"due".to_owned()));
        assert_eq!(h.form, Form::Work);
        assert!(matches!(ps.lookup("adr"), Lookup::Invalid));
        assert!(matches!(ps.lookup("a"), Lookup::Invalid));
        assert!(matches!(ps.lookup("b"), Lookup::Invalid));
        assert!(matches!(ps.lookup("nothing"), Lookup::Missing));
        assert!(matches!(ps.lookup("delivery"), Lookup::Found(_)));
        assert_eq!(ps.invalid.len(), 3);
    }

    #[test]
    fn each_builtin_is_the_normative_file_when_it_is_present() {
        // 7.6: the files at schemas/format/1.0/profiles/ are normative. The
        // copies compiled in here are those bytes; once the files are in the
        // repository, a copy that drifts fails here.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../schemas/format/1.0/profiles");
        for (name, text) in BUILTIN {
            let path = dir.join(format!("{name}.toml"));
            if let Ok(normative) = std::fs::read_to_string(&path) {
                assert_eq!(
                    text,
                    normative,
                    "{name}: the built-in differs from {}",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn the_format_key_is_the_discriminator() {
        // 7.5: a file without `format` is legacy, whatever else it says.
        assert!(!is_format_profile(
            b"schema = \"oh.war/profile/v1\"\nname = \"x\"\nform = \"work\"\n[sections]\n"
        ));
        assert!(is_format_profile(
            b"schema = \"oh.war/profile/v1\"\nformat = \"openwarrant/1.0\"\nname = \"x\"\n"
        ));
        // A `format` naming another major is refused, not ignored.
        assert!(
            parse(
                "profiles/x.toml",
                b"schema = \"oh.war/profile/v1\"\nformat = \"openwarrant/2.0\"\nname = \"x\"\nform = \"work\"\n",
                &none
            )
            .is_err()
        );
        for name in builtin_names() {
            assert!(
                is_format_profile(
                    BUILTIN
                        .iter()
                        .find(|(n, _)| *n == name)
                        .unwrap()
                        .1
                        .as_bytes()
                ),
                "{name}"
            );
        }
    }

    #[test]
    fn legacy_profile_files_are_not_format_profiles() {
        for text in [
            include_str!("../../../profiles/delivery.toml"),
            include_str!("../../../profiles/ticket.toml"),
            include_str!("../../../profiles/spec.toml"),
            include_str!("../../../profiles/prd.toml"),
            include_str!("../../../profiles/roadmap.toml"),
            include_str!("../../../profiles/contractor.toml"),
        ] {
            assert!(!is_format_profile(text.as_bytes()), "{text}");
        }
    }

    #[test]
    fn shapes() {
        let d = |shape, values: &[&str]| FieldDecl {
            required: false,
            shape,
            values: values.iter().map(|s| (*s).to_owned()).collect(),
        };
        assert!(d(Shape::Int, &[]).refuse("600").is_none());
        assert!(d(Shape::Int, &[]).refuse("ten").is_some());
        assert!(d(Shape::Date, &[]).refuse("2026-10-15").is_none());
        assert!(d(Shape::Date, &[]).refuse("15/10/2026").is_some());
        assert!(d(Shape::Line, &[]).refuse("a\nb").is_some());
        assert!(
            d(Shape::Gate, &[])
                .refuse("gate://ops.plants@1.1.0")
                .is_none()
        );
        assert!(d(Shape::Gate, &[]).refuse("gate://ops.plants").is_some());
        assert!(d(Shape::Ids, &[]).refuse("REQ-1, t-0d61d").is_none());
        assert!(d(Shape::Ids, &[]).refuse("REQ-1, the rule").is_some());
        assert!(d(Shape::Enum, &["a", "b"]).refuse("b").is_none());
        assert!(d(Shape::Enum, &["a", "b"]).refuse("c").is_some());
        assert!(d(Shape::Number, &[]).refuse("120.5").is_none());
        assert!(d(Shape::Number, &[]).refuse("-3").is_none());
        assert!(d(Shape::Number, &[]).refuse("1.").is_some());
        assert!(d(Shape::Number, &[]).refuse("high").is_some());
        assert!(d(Shape::Bool, &[]).refuse("false").is_none());
        assert!(d(Shape::Bool, &[]).refuse("yes").is_some());
    }
}
