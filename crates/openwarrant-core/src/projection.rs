// SPDX-License-Identifier: Apache-2.0
//! Document types and their projections, as data (OW-WAR-0148 M6;
//! OW-ADR-0031).
//!
//! # The document form
//!
//! A profile file with `form = "document"` is a **document type**: it
//! composes records (`[records] types`, `[relations] allow`, as any profile
//! does) and declares how to project them (`[[projections]]`). It is not a
//! Warrant profile. Nothing of it is authorized, verified or resolved, and a
//! Warrant manifest cannot name it: its capabilities are chosen from
//! `structure` and `links`, which are its default, and any other capability
//! is refused (`profile.capabilities`). A document is a view of records whose
//! authority, where they have any, lives in the Warrants that govern them.
//!
//! ```toml
//! schema = "oh.war/profile/v1"
//! name = "prd"
//! form = "document"
//!
//! [records]
//! types = ["outcome", "requirement"]
//!
//! [relations]
//! allow = ["implements"]
//!
//! [[projections]]
//! name = "prd"
//! title = "{title}: product requirements"
//! [projections.select]
//! types = ["outcome", "requirement"]
//!
//! [[projections.sections]]
//! heading = "Requirements"
//! block = "list"
//! types = ["requirement"]
//! annotate = ["out:implements"]
//! ```
//!
//! # A projection
//!
//! A named, pure function from a selection of the compiled model to bytes
//! (`openwarrant_compiler::project` renders it). It declares:
//!
//! - **select** — the record types it may show and the relations it walks
//!   from its seeds (a document's roots, or every record of its area):
//!   `"in:evaluates"` walks incoming `evaluates` edges, `"out:implements"`
//!   outgoing ones, a bare kind both ways, and `"in:implements:interface"`
//!   only to records of type `interface`. `depth` bounds the walk.
//! - **sections**, in order, each a block of the template vocabulary:
//!   `heading` (a heading and its intro), `list` (a record list, bullets or
//!   blocks), `table` (records × fields), `tree` (records with their
//!   children through named relations) and `quote` (each record's body,
//!   quoted).
//! - **renderer** — `markdown` or `json`; **max_bytes** — a budget the
//!   rendering must fit, or it is refused by name, never truncated.
//!
//! Refused when the profile is read, `profile.projection`: a projection that
//! names a record type its profile does not declare (the kernel's
//! `obligation` and `item` excepted), a relation kind its profile does not
//! allow (a namespaced kind is inert and may not be walked), a section type
//! the selection does not admit, or a block it does not know.
//!
//! Pure: values, parses and validation (§79.1).

use std::collections::BTreeSet;

use serde::Deserialize;

use crate::relation::{CoreKind, KERNEL_RECORD_TYPES, RelationKind, Vocabulary};
use crate::role::{Capabilities, Capability};

/// The `form` a document type declares.
pub const DOCUMENT_FORM: &str = "document";

/// The capabilities a document type may select, and its default.
pub const DOCUMENT_CAPABILITIES: Capabilities =
    Capabilities::of(&[Capability::Structure, Capability::Links]);

/// A document type: `profiles/<name>.toml` with `form = "document"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentProfile {
    pub name: String,
    pub note: Option<String>,
    pub capabilities: Capabilities,
    pub vocabulary: Vocabulary,
    pub projections: Vec<ProjectionDef>,
    /// `sha256:<hex>` of the file's bytes: what the template lines of every
    /// rendering trace to.
    pub digest: String,
}

/// Which way a step walks a relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Direction {
    /// From the record to the relation's target.
    Out,
    /// From the relation's source to the record.
    In,
    /// Either.
    Both,
}

/// One relation a projection walks or shows: `[in:|out:]<kind>[:<type>]`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Step {
    pub kind: CoreKind,
    pub direction: Direction,
    /// Only to (or from) records of this type.
    pub only: Option<String>,
}

impl Step {
    /// Parse `in:evaluates`, `out:implements`, `constrains`,
    /// `in:implements:interface`. A namespaced or unknown kind is refused.
    pub fn parse(text: &str) -> Result<Self, String> {
        let (direction, rest) = if let Some(r) = text.strip_prefix("in:") {
            (Direction::In, r)
        } else if let Some(r) = text.strip_prefix("out:") {
            (Direction::Out, r)
        } else {
            (Direction::Both, text)
        };
        let (kind, only) = match rest.split_once(':') {
            Some((k, t)) => (k, Some(t.to_owned())),
            None => (rest, None),
        };
        match RelationKind::parse(kind) {
            Ok(RelationKind::Core(core)) => Ok(Self {
                kind: core,
                direction,
                only,
            }),
            Ok(RelationKind::Namespaced(_)) => Err(format!(
                "`{text}` walks the namespaced kind `{kind}`; a namespaced relation is \
                 carried and inert, and a projection may not give it meaning"
            )),
            Err(e) => Err(format!("`{text}`: {e}")),
        }
    }

    /// How a reader says this relation, from the record's side:
    /// `out:implements` "implements", `in:implements` "implemented by".
    #[must_use]
    pub fn phrase(&self) -> &'static str {
        let out = self.direction != Direction::In;
        match (self.kind, out) {
            (CoreKind::PartOf, true) => "part of",
            (CoreKind::PartOf, false) => "includes",
            (CoreKind::DependsOn, true) => "depends on",
            (CoreKind::DependsOn, false) => "needed by",
            (CoreKind::Implements, true) => "implements",
            (CoreKind::Implements, false) => "implemented by",
            (CoreKind::Constrains, true) => "constrains",
            (CoreKind::Constrains, false) => "constrained by",
            (CoreKind::Evaluates, true) => "evaluates",
            (CoreKind::Evaluates, false) => "evaluated by",
            (CoreKind::Supersedes, true) => "supersedes",
            (CoreKind::Supersedes, false) => "superseded by",
            (CoreKind::Supports, true) => "supports",
            (CoreKind::Supports, false) => "supported by",
            (CoreKind::Refutes, true) => "refutes",
            (CoreKind::Refutes, false) => "refuted by",
            (CoreKind::TradesOffAgainst, _) => "trades off against",
            (CoreKind::Causes, true) => "causes",
            (CoreKind::Causes, false) => "caused by",
            (CoreKind::Qualifies, true) => "qualifies",
            (CoreKind::Qualifies, false) => "qualified by",
            (CoreKind::SelectedOver, true) => "chosen over",
            (CoreKind::SelectedOver, false) => "passed over for",
        }
    }
}

/// Markdown or JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Renderer {
    Markdown,
    Json,
}

impl Renderer {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Json => "json",
        }
    }

    /// The file extension `war compile` writes it under.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Markdown => "md",
            Self::Json => "json",
        }
    }
}

/// What a projection selects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Select {
    /// The record types it may show.
    pub types: Vec<String>,
    /// The relations it walks from its seeds.
    pub through: Vec<Step>,
    /// How many steps from a seed it walks; unbounded when absent.
    pub depth: Option<u32>,
}

/// How much of a record a block shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Show {
    /// Its first paragraph, on one line.
    Summary,
    /// Its whole body.
    Body,
}

/// A list's layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListStyle {
    /// `- **ID.** text`.
    Bullets,
    /// A `### ID` subsection per record.
    Blocks,
}

/// One column of a table, `<column>[=<label>]`: `summary=Requirement`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDef {
    pub column: Column,
    /// The header; the column's own name when absent.
    pub label: Option<String>,
}

/// What a table column shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Column {
    Id,
    Type,
    Summary,
    Revision,
    Source,
    /// `field:<name>` — a field the record carries (an obligation's
    /// `scope`, `evidence`, `verdict`).
    Field(String),
    /// A relation, as the ids at its other end.
    Relation(Step),
}

/// A group of a tree's children: the records at the other end of one
/// relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Children {
    pub relation: Step,
    /// Its label; the relation's phrase when absent.
    pub label: Option<String>,
    /// Said when a record has none.
    pub empty: Option<String>,
}

/// One block of the template vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// The section's heading and intro, and nothing else.
    Heading,
    List {
        style: ListStyle,
        show: Show,
        annotate: Vec<Step>,
        /// Each record's type, source and revision under it.
        provenance: bool,
        /// Fields to list under each record.
        fields: Vec<String>,
    },
    Table {
        columns: Vec<ColumnDef>,
    },
    Tree {
        children: Vec<Children>,
        show: Show,
        annotate: Vec<Step>,
        /// Fields to list under each child.
        fields: Vec<String>,
    },
    Quote,
}

/// Which of the selected records a section draws from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Of {
    All,
    /// Only the seeds (a document's roots).
    Roots,
    /// Everything but the seeds.
    Rest,
}

/// One section of a projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub heading: String,
    pub intro: Option<String>,
    pub types: Vec<String>,
    pub of: Of,
    pub block: Block,
    /// Said when the section selects nothing.
    pub empty: Option<String>,
}

/// A declared projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionDef {
    pub name: String,
    /// `{title}` is the document's title (or the root record's id).
    pub title: String,
    pub intro: Option<String>,
    pub renderer: Renderer,
    pub select: Select,
    pub sections: Vec<Section>,
    /// The heading of the provenance table every rendering ends with.
    pub sources: String,
    /// The budget a rendering must fit, in bytes.
    pub max_bytes: Option<usize>,
}

// ---- The file form.

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentFile {
    schema: String,
    name: String,
    form: String,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    capabilities: Option<Vec<String>>,
    #[serde(default)]
    records: Option<RecordsTable>,
    #[serde(default)]
    relations: Option<RelationsTable>,
    #[serde(default)]
    projections: Vec<ProjectionFile>,
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectionFile {
    name: String,
    title: String,
    #[serde(default)]
    intro: Option<String>,
    #[serde(default)]
    renderer: Option<String>,
    select: SelectFile,
    #[serde(default)]
    sections: Vec<SectionFile>,
    #[serde(default)]
    sources: Option<String>,
    #[serde(default)]
    max_bytes: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectFile {
    types: Vec<String>,
    #[serde(default)]
    through: Vec<String>,
    #[serde(default)]
    depth: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChildrenFile {
    relation: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    empty: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SectionFile {
    heading: String,
    block: String,
    #[serde(default)]
    intro: Option<String>,
    #[serde(default)]
    types: Vec<String>,
    #[serde(default)]
    of: Option<String>,
    #[serde(default)]
    empty: Option<String>,
    #[serde(default)]
    style: Option<String>,
    #[serde(default)]
    show: Option<String>,
    #[serde(default)]
    annotate: Vec<String>,
    #[serde(default)]
    provenance: bool,
    #[serde(default)]
    fields: Vec<String>,
    #[serde(default)]
    columns: Vec<String>,
    #[serde(default)]
    children: Vec<ChildrenFile>,
}

/// Why a document type was refused: the rule, and the detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentError {
    /// `profile.invalid`, `profile.capabilities`, `profile.records` or
    /// `profile.projection`.
    pub rule: &'static str,
    pub detail: String,
}

fn invalid(detail: impl Into<String>) -> DocumentError {
    DocumentError {
        rule: "profile.invalid",
        detail: detail.into(),
    }
}

fn refused(projection: &str, detail: impl std::fmt::Display) -> DocumentError {
    DocumentError {
        rule: "profile.projection",
        detail: format!("projection `{projection}`: {detail}"),
    }
}

/// Whether `text` declares `form = "document"`. A file that does not parse
/// as TOML is not one; the contract-form parser reports it.
#[must_use]
pub fn is_document_form(text: &str) -> bool {
    toml::from_str::<toml::Value>(text)
        .is_ok_and(|v| v.get("form").and_then(toml::Value::as_str) == Some(DOCUMENT_FORM))
}

fn is_word(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|c| c.is_ascii_lowercase())
        && c.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Parse a document type. `name` is checked against the file by the caller,
/// as for every profile; `digest` is the file's.
pub fn parse_document(text: &str, digest: String) -> Result<DocumentProfile, DocumentError> {
    let raw: DocumentFile = toml::from_str(text).map_err(|e| invalid(e.to_string()))?;
    if raw.form != DOCUMENT_FORM {
        return Err(invalid(format!("form {:?} is not \"document\"", raw.form)));
    }
    if raw.schema != crate::role::PROFILE_SCHEMA {
        return Err(invalid(format!(
            "schema {:?}; this build reads {:?}",
            raw.schema,
            crate::role::PROFILE_SCHEMA
        )));
    }
    if !is_word(&raw.name)
        || crate::role::CoreProfile::ALL
            .iter()
            .any(|c| c.as_str() == raw.name)
    {
        return Err(invalid(format!(
            "profile name {:?} is not a lowercase word, or is a core profile's",
            raw.name
        )));
    }
    let capabilities = match raw.capabilities.as_deref() {
        None => DOCUMENT_CAPABILITIES,
        Some(listed) => {
            let mut seen = Vec::new();
            for found in listed {
                let Some(c) = Capability::parse(found) else {
                    return Err(DocumentError {
                        rule: "profile.capability-unknown",
                        detail: format!(
                            "capability {found:?} is not one of the closed set ({})",
                            Capability::ALL.map(Capability::as_str).join(", ")
                        ),
                    });
                };
                if !DOCUMENT_CAPABILITIES.has(c) {
                    return Err(DocumentError {
                        rule: "profile.capabilities",
                        detail: format!(
                            "a document type selects from [{DOCUMENT_CAPABILITIES}], and \
                             `{c}` is not among them. A document is a view of records: \
                             nothing of it is claimed, accepted, authorized, verified or \
                             resolved. Work that needs `{c}` is a Warrant's"
                        ),
                    });
                }
                if seen.contains(&c) {
                    return Err(DocumentError {
                        rule: "profile.capabilities",
                        detail: format!("capability `{c}` is listed twice"),
                    });
                }
                seen.push(c);
            }
            let caps = Capabilities::of(&seen);
            if let Some((capability, needs)) = caps.missing_prerequisite() {
                return Err(DocumentError {
                    rule: "profile.capability-prerequisite",
                    detail: format!("selects `{capability}` without `{needs}`, which it needs"),
                });
            }
            caps
        }
    };
    let none = RelationsTable::default();
    let types = raw
        .records
        .as_ref()
        .map(|r| r.types.as_slice())
        .unwrap_or_default();
    let relations = raw.relations.as_ref().unwrap_or(&none);
    let vocabulary =
        Vocabulary::declare(types, &relations.allow, &relations.require).map_err(|detail| {
            DocumentError {
                rule: "profile.records",
                detail: format!("[records]/[relations]: {detail}"),
            }
        })?;
    let mut projections = Vec::new();
    let mut names = BTreeSet::new();
    for p in raw.projections {
        let def = projection(p, &vocabulary)?;
        if !names.insert(def.name.clone()) {
            return Err(refused(&def.name, "declared twice"));
        }
        projections.push(def);
    }
    Ok(DocumentProfile {
        name: raw.name,
        note: raw.note,
        capabilities,
        vocabulary,
        projections,
        digest,
    })
}

fn declared_type(vocabulary: &Vocabulary, ty: &str) -> bool {
    vocabulary.has_type(ty) || KERNEL_RECORD_TYPES.contains(&ty)
}

fn step(name: &str, vocabulary: &Vocabulary, text: &str) -> Result<Step, DocumentError> {
    let s = Step::parse(text).map_err(|e| refused(name, e))?;
    if !vocabulary.allow.contains(&s.kind) {
        return Err(refused(
            name,
            format!(
                "`{text}` names the relation kind `{}`, which this profile does not allow \
                 ([relations] allow = [{}])",
                s.kind,
                vocabulary
                    .allow
                    .iter()
                    .map(|k| format!("\"{k}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }
    if let Some(t) = &s.only
        && !declared_type(vocabulary, t)
    {
        return Err(undeclared(name, vocabulary, t));
    }
    Ok(s)
}

fn undeclared(name: &str, vocabulary: &Vocabulary, ty: &str) -> DocumentError {
    refused(
        name,
        format!(
            "record type `{ty}` is not declared by this profile ([records] types = [{}]; the \
             kernel's `obligation` and `item` need no declaration)",
            vocabulary
                .types
                .iter()
                .map(|t| format!("\"{t}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    )
}

fn show(name: &str, s: Option<&str>) -> Result<Show, DocumentError> {
    match s {
        None | Some("body") => Ok(Show::Body),
        Some("summary") => Ok(Show::Summary),
        Some(other) => Err(refused(
            name,
            format!("show {other:?} is not `summary` or `body`"),
        )),
    }
}

fn projection(p: ProjectionFile, vocabulary: &Vocabulary) -> Result<ProjectionDef, DocumentError> {
    let name = p.name.clone();
    if !is_word(&name) {
        return Err(refused(
            &name,
            "a projection name is a lowercase word ([a-z][a-z0-9_-]*)",
        ));
    }
    let renderer = match p.renderer.as_deref() {
        None | Some("markdown") => Renderer::Markdown,
        Some("json") => Renderer::Json,
        Some(other) => {
            return Err(refused(
                &name,
                format!("renderer {other:?} is not `markdown` or `json`"),
            ));
        }
    };
    if p.select.types.is_empty() {
        return Err(refused(&name, "`select.types` selects nothing"));
    }
    for t in &p.select.types {
        if !declared_type(vocabulary, t) {
            return Err(undeclared(&name, vocabulary, t));
        }
    }
    let through = p
        .select
        .through
        .iter()
        .map(|t| step(&name, vocabulary, t))
        .collect::<Result<Vec<_>, _>>()?;
    if p.max_bytes == Some(0) {
        return Err(refused(&name, "`max_bytes` is 0; nothing fits"));
    }
    if p.sections.is_empty() {
        return Err(refused(&name, "declares no section"));
    }
    let selected: BTreeSet<&str> = p.select.types.iter().map(String::as_str).collect();
    let mut sections = Vec::new();
    for s in p.sections {
        for t in &s.types {
            if !declared_type(vocabulary, t) {
                return Err(undeclared(&name, vocabulary, t));
            }
            if !selected.contains(t.as_str()) {
                return Err(refused(
                    &name,
                    format!(
                        "section {:?} shows type `{t}`, which `select.types` does not select",
                        s.heading
                    ),
                ));
            }
        }
        let of = match s.of.as_deref() {
            None | Some("all") => Of::All,
            Some("roots") => Of::Roots,
            Some("rest") => Of::Rest,
            Some(other) => {
                return Err(refused(
                    &name,
                    format!(
                        "section {:?}: of {other:?} is not `all`, `roots` or `rest`",
                        s.heading
                    ),
                ));
            }
        };
        let annotate = s
            .annotate
            .iter()
            .map(|t| step(&name, vocabulary, t))
            .collect::<Result<Vec<_>, _>>()?;
        let block = match s.block.as_str() {
            "heading" => Block::Heading,
            "quote" => Block::Quote,
            "list" => Block::List {
                style: match s.style.as_deref() {
                    None | Some("bullets") => ListStyle::Bullets,
                    Some("blocks") => ListStyle::Blocks,
                    Some(other) => {
                        return Err(refused(
                            &name,
                            format!("style {other:?} is not `bullets` or `blocks`"),
                        ));
                    }
                },
                show: show(&name, s.show.as_deref())?,
                annotate,
                provenance: s.provenance,
                fields: s.fields.clone(),
            },
            "table" => {
                if s.columns.is_empty() {
                    return Err(refused(
                        &name,
                        format!("section {:?}: a table names its columns", s.heading),
                    ));
                }
                let mut columns = Vec::new();
                for spec in &s.columns {
                    let (c, label) = match spec.split_once('=') {
                        Some((c, l)) => (c, Some(l.to_owned())),
                        None => (spec.as_str(), None),
                    };
                    let column = match c {
                        "id" => Column::Id,
                        "type" => Column::Type,
                        "summary" => Column::Summary,
                        "revision" => Column::Revision,
                        "source" => Column::Source,
                        other => {
                            if let Some(f) = other.strip_prefix("field:") {
                                if !is_word(f) {
                                    return Err(refused(
                                        &name,
                                        format!("column {other:?}: a field is a lowercase word"),
                                    ));
                                }
                                Column::Field(f.to_owned())
                            } else if other.starts_with("in:") || other.starts_with("out:") {
                                Column::Relation(step(&name, vocabulary, other)?)
                            } else {
                                return Err(refused(
                                    &name,
                                    format!(
                                        "column {other:?} is not id, type, summary, revision, \
                                         source, field:<name>, in:<kind> or out:<kind>"
                                    ),
                                ));
                            }
                        }
                    };
                    columns.push(ColumnDef { column, label });
                }
                Block::Table { columns }
            }
            "tree" => {
                if s.children.is_empty() {
                    return Err(refused(
                        &name,
                        format!(
                            "section {:?}: a tree names its children's relations",
                            s.heading
                        ),
                    ));
                }
                let mut children = Vec::new();
                for c in &s.children {
                    let relation = step(&name, vocabulary, &c.relation)?;
                    if relation.direction == Direction::Both {
                        return Err(refused(
                            &name,
                            format!(
                                "child relation {:?} names no direction; a tree's children are \
                                 `in:<kind>` or `out:<kind>`",
                                c.relation
                            ),
                        ));
                    }
                    children.push(Children {
                        relation,
                        label: c.label.clone(),
                        empty: c.empty.clone(),
                    });
                }
                Block::Tree {
                    children,
                    show: show(&name, s.show.as_deref())?,
                    annotate,
                    fields: s.fields.clone(),
                }
            }
            other => {
                return Err(refused(
                    &name,
                    format!(
                        "section {:?}: block {other:?} is not heading, list, table, tree or quote",
                        s.heading
                    ),
                ));
            }
        };
        if !matches!(block, Block::Heading) && s.types.is_empty() {
            return Err(refused(
                &name,
                format!("section {:?} shows no record type", s.heading),
            ));
        }
        sections.push(Section {
            heading: s.heading,
            intro: s.intro,
            types: s.types,
            of,
            block,
            empty: s.empty,
        });
    }
    Ok(ProjectionDef {
        name,
        title: p.title,
        intro: p.intro,
        renderer,
        select: Select {
            types: p.select.types,
            through,
            depth: p.select.depth,
        },
        sections,
        sources: p.sources.unwrap_or_else(|| "Sources".to_owned()),
        max_bytes: p.max_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"
schema = "oh.war/profile/v1"
name = "prd"
form = "document"

[records]
types = ["outcome", "requirement"]

[relations]
allow = ["implements", "evaluates"]

[[projections]]
name = "prd"
title = "{title}: requirements"
[projections.select]
types = ["outcome", "requirement", "obligation"]
through = ["in:evaluates"]

[[projections.sections]]
heading = "Requirements"
block = "list"
types = ["requirement"]
annotate = ["out:implements"]

[[projections.sections]]
heading = "Checks"
block = "tree"
types = ["requirement"]
children = [{ relation = "in:evaluates", label = "Evaluated by" }]
"#;

    fn parse(text: &str) -> Result<DocumentProfile, DocumentError> {
        parse_document(text, "sha256:00".to_owned())
    }

    #[test]
    fn a_document_type_parses_with_its_projections() {
        assert!(is_document_form(DOC));
        assert!(!is_document_form("name = \"x\"\nform = \"working\"\n"));
        let d = parse(DOC).unwrap();
        assert_eq!(d.capabilities, DOCUMENT_CAPABILITIES);
        assert_eq!(d.projections.len(), 1);
        let p = &d.projections[0];
        assert_eq!(p.renderer, Renderer::Markdown);
        assert_eq!(p.select.through[0].direction, Direction::In);
        assert_eq!(p.sources, "Sources");
        assert!(matches!(p.sections[1].block, Block::Tree { .. }));
    }

    #[test]
    fn undeclared_types_and_kinds_are_refused_by_rule() {
        for (from, to, why) in [
            (
                "types = [\"outcome\", \"requirement\", \"obligation\"]",
                "types = [\"outcome\", \"risk\"]",
                "record type `risk` is not declared",
            ),
            (
                "through = [\"in:evaluates\"]",
                "through = [\"in:supersedes\"]",
                "`supersedes`, which this profile does not allow",
            ),
            (
                "through = [\"in:evaluates\"]",
                "through = [\"in:x.mentions\"]",
                "namespaced",
            ),
            (
                "through = [\"in:evaluates\"]",
                "through = [\"in:mentions\"]",
                "not a relation kind",
            ),
            (
                "annotate = [\"out:implements\"]",
                "annotate = [\"out:depends_on\"]",
                "`depends_on`, which this profile does not allow",
            ),
            (
                "block = \"list\"",
                "block = \"chart\"",
                "is not heading, list, table, tree or quote",
            ),
            (
                "relation = \"in:evaluates\"",
                "relation = \"evaluates\"",
                "names no direction",
            ),
        ] {
            let bad = DOC.replacen(from, to, 1);
            assert_ne!(bad, DOC, "{from}");
            let e = parse(&bad).unwrap_err();
            assert_eq!(e.rule, "profile.projection", "{e:?}");
            assert!(e.detail.contains(why), "{}", e.detail);
        }
        // A section showing a type the selection does not admit.
        let bad = DOC.replace(
            "types = [\"outcome\", \"requirement\", \"obligation\"]",
            "types = [\"outcome\", \"obligation\"]",
        );
        assert!(parse(&bad).unwrap_err().detail.contains("does not select"));
    }

    #[test]
    fn a_document_type_selects_no_authority() {
        for cap in [
            "authorization",
            "verification",
            "resolution",
            "claims",
            "acceptance",
        ] {
            let bad = DOC.replace(
                "form = \"document\"\n",
                &format!("form = \"document\"\ncapabilities = [\"structure\", \"{cap}\"]\n"),
            );
            let e = parse(&bad).unwrap_err();
            assert_eq!(e.rule, "profile.capabilities", "{cap}");
        }
        let ok = DOC.replace(
            "form = \"document\"\n",
            "form = \"document\"\ncapabilities = [\"structure\"]\n",
        );
        assert_eq!(
            parse(&ok).unwrap().capabilities,
            Capabilities::of(&[Capability::Structure])
        );
        let bad = DOC.replace(
            "form = \"document\"\n",
            "form = \"document\"\ncapabilities = [\"links\"]\n",
        );
        assert_eq!(
            parse(&bad).unwrap_err().rule,
            "profile.capability-prerequisite"
        );
    }

    #[test]
    fn steps_parse_with_direction_and_type() {
        let s = Step::parse("in:implements:interface").unwrap();
        assert_eq!(s.kind, CoreKind::Implements);
        assert_eq!(s.direction, Direction::In);
        assert_eq!(s.only.as_deref(), Some("interface"));
        assert_eq!(s.phrase(), "implemented by");
        assert_eq!(
            Step::parse("constrains").unwrap().direction,
            Direction::Both
        );
    }
}
