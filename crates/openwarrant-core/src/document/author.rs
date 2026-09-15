// SPDX-License-Identifier: Apache-2.0
//! Pure, bounded source authoring. Returning bytes never saves, signs or executes them.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// A unit's exact Markdown, including its heading and terminal LF (or CRLF).
#[derive(Clone, Debug)]
pub struct AuthoredUnit {
    pub id: String,
    pub kind: UnitKind,
    pub text: String,
}

/// Typed TOML fields. A list preserves caller order and makes duplicate keys detectable.
/// Required fields (including schema, identity and revision) are always explicit.
#[derive(Clone, Debug)]
pub struct DocumentFields {
    pub metadata: Vec<(String, MetadataValue)>,
    pub units: Vec<AuthoredUnit>,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum LineEnding {
    #[default]
    Lf,
    CrLf,
}
impl LineEnding {
    fn text(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }
}

/// Presentation affects framing only; authored unit text is never normalized.
#[derive(Clone, Copy, Debug, Default)]
pub struct AuthorOptions {
    pub line_ending: LineEnding,
    pub wrap_metadata: bool,
}

/// Explicit edits; identity and revision never change implicitly. Metadata title
/// edits also update the first visible heading in RC.3. Unit edits preserve IDs.
#[derive(Clone, Debug)]
pub enum DocumentEdit {
    Metadata {
        key: String,
        value: Option<MetadataValue>,
    },
    Unit {
        id: String,
        text: String,
    },
}

struct Output {
    text: String,
    limit: usize,
}
impl Output {
    fn new(limit: usize) -> Self {
        Self {
            text: String::new(),
            limit,
        }
    }
    fn push(&mut self, text: &str) -> Result<(), Diagnostic> {
        if text.len() > self.limit.saturating_sub(self.text.len()) {
            return Err(error("resource-limit", "Authoring output limit exceeded"));
        }
        self.text.push_str(text);
        Ok(())
    }
}
fn error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(code, message, 0..0)
}

// JSON basic string escapes are also TOML 1.0 basic string escapes. Stream
// characters so escaping cannot allocate an unbounded intermediate string.
fn quoted(out: &mut Output, text: &str) -> Result<(), Diagnostic> {
    out.push("\"")?;
    for ch in text.chars() {
        match ch {
            '"' => out.push("\\\"")?,
            '\\' => out.push("\\\\")?,
            '\n' => out.push("\\n")?,
            '\r' => out.push("\\r")?,
            '\t' => out.push("\\t")?,
            c if c.is_control() => out.push(&format!("\\u{:04X}", c as u32))?,
            c => {
                out.push(c.encode_utf8(&mut [0; 4]))?;
            }
        }
    }
    out.push("\"")
}
fn value(out: &mut Output, v: &MetadataValue, depth: usize, max: usize) -> Result<(), Diagnostic> {
    if depth > max {
        return Err(error("resource-limit", "Metadata depth exceeded"));
    }
    match v {
        MetadataValue::String(s) => quoted(out, s),
        MetadataValue::Integer(i) if (0..=9_007_199_254_740_991).contains(i) => {
            out.push(&i.to_string())
        }
        MetadataValue::Boolean(b) => out.push(if *b { "true" } else { "false" }),
        MetadataValue::Array(items) => {
            out.push("[")?;
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(", ")?;
                }
                value(out, item, depth + 1, max)?;
            }
            out.push("]")
        }
        MetadataValue::Table(fields) => {
            out.push("{ ")?;
            for (i, (key, item)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push(", ")?;
                }
                quoted(out, key)?;
                out.push(" = ")?;
                value(out, item, depth + 1, max)?;
            }
            out.push(" }")
        }
        _ => Err(error("source-invalid", "Unsupported metadata value")),
    }
}
fn metadata(
    fields: &[(String, MetadataValue)],
    newline: &str,
    limits: ParseLimits,
) -> Result<String, Diagnostic> {
    if limits.source_bytes == 0
        || limits.metadata_bytes == 0
        || limits.units == 0
        || !(1..=128).contains(&limits.metadata_depth)
    {
        return Err(error("resource-limit", "Invalid authoring limits"));
    }
    let mut seen = BTreeSet::new();
    let mut out = Output::new(limits.metadata_bytes.min(limits.source_bytes));
    for (key, item) in fields {
        if !seen.insert(key) {
            return Err(error("field-duplicate", format!("Duplicate field {key}")));
        }
        quoted(&mut out, key)?;
        out.push(" = ")?;
        value(&mut out, item, 1, limits.metadata_depth)?;
        out.push(newline)?;
    }
    Ok(out.text)
}
fn valid(bytes: &[u8], dialect: Dialect, limits: ParseLimits) -> Result<Document<'_>, Diagnostic> {
    let doc = parse_document(bytes, dialect, limits)?;
    let report = validate_document(&doc, &ValidationOptions::default());
    if report.validity != Validity::Valid {
        return Err(report
            .diagnostics
            .into_iter()
            .find(|d| d.severity == Severity::Error)
            .unwrap_or_else(|| error("source-invalid", "Invalid document")));
    }
    Ok(doc)
}
fn exact_units(doc: &Document<'_>, units: &[AuthoredUnit]) -> Result<(), Diagnostic> {
    if doc.units().len() != units.len()
        || !doc.units().iter().zip(units).all(|(actual, expected)| {
            actual.id == expected.id
                && actual.kind == expected.kind
                && doc.source[actual.span.clone()] == expected.text
        })
    {
        return Err(error(
            "unit-injection",
            "Authored text changed unit boundaries or identities",
        ));
    }
    Ok(())
}

/// Author a new RC.3 source, validating the complete candidate before returning it.
/// Unsupported optional extensions are preserved; validity grants no assurance.
pub fn author_document(
    fields: &DocumentFields,
    options: AuthorOptions,
    limits: ParseLimits,
) -> Result<Vec<u8>, Diagnostic> {
    let nl = options.line_ending.text();
    let meta = metadata(&fields.metadata, nl, limits)?;
    if fields.units.len() > limits.units {
        return Err(error("resource-limit", "Unit limit exceeded"));
    }
    let mut out = Output::new(limits.source_bytes);
    for unit in &fields.units {
        if !unit.text.ends_with('\n') {
            return Err(error(
                "source-invalid",
                "Unit text must end with LF or CRLF",
            ));
        }
        out.push("<!-- ow:unit ")?;
        out.push(&unit.id)?;
        out.push(match unit.kind {
            UnitKind::Binding => " binding -->",
            UnitKind::Background => " background -->",
        })?;
        out.push(nl)?;
        out.push(&unit.text)?;
    }
    out.push("<!-- ow:metadata -->")?;
    out.push(nl)?;
    if options.wrap_metadata {
        out.push("<details>")?;
        out.push(nl)?;
        out.push("<summary>OpenWarrant metadata</summary>")?;
        out.push(nl)?;
        out.push(nl)?;
    }
    out.push("```toml")?;
    out.push(nl)?;
    out.push(&meta)?;
    out.push("```")?;
    out.push(nl)?;
    if options.wrap_metadata {
        out.push(nl)?;
        out.push("</details>")?;
        out.push(nl)?;
    }
    out.push("<!-- /ow:metadata -->")?;
    out.push(nl)?;
    exact_units(
        &valid(out.text.as_bytes(), Dialect::Rc3, limits)?,
        &fields.units,
    )?;
    Ok(out.text.into_bytes())
}

/// Edit an existing valid source transactionally in memory. No-op returns exact
/// input bytes; only requested metadata/unit spans change. RC.2 stays RC.2.
/// Diagnostics on candidate validation index candidate bytes; pre-edit failures
/// index input bytes. No failure returns partial output, and no filesystem is used.
pub fn edit_document(
    doc: &Document<'_>,
    edits: &[DocumentEdit],
    limits: ParseLimits,
) -> Result<Vec<u8>, Diagnostic> {
    valid(doc.original(), doc.dialect(), limits)?;
    // Bound the entire borrowed edit batch before allocating copies. Per-field
    // limits alone let many individually small edits exhaust memory in aggregate.
    let mut input = Output::new(limits.source_bytes);
    for edit in edits {
        match edit {
            DocumentEdit::Metadata { key, value: item } => {
                quoted(&mut input, key)?;
                if let Some(item) = item {
                    value(&mut input, item, 1, limits.metadata_depth)?;
                }
            }
            DocumentEdit::Unit { id, text } => {
                quoted(&mut input, id)?;
                input.push(text)?;
            }
        }
    }
    drop(input);
    let mut fields: Vec<_> = doc
        .metadata()
        .as_table()
        .expect("parsed table")
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let mut units: Vec<_> = doc
        .units()
        .iter()
        .map(|u| AuthoredUnit {
            id: u.id.clone(),
            kind: u.kind,
            text: doc.source[u.span.clone()].into(),
        })
        .collect();
    let unit_indices: BTreeMap<_, _> = units
        .iter()
        .enumerate()
        .map(|(i, u)| (u.id.clone(), i))
        .collect();
    let mut seen = BTreeSet::new();
    let mut metadata_changed = false;
    for edit in edits {
        let (domain, key) = match edit {
            DocumentEdit::Metadata { key, .. } => ("metadata", key),
            DocumentEdit::Unit { id, .. } => ("unit", id),
        };
        if !seen.insert((domain, key)) {
            return Err(error(
                "field-duplicate",
                format!("Duplicate {domain} edit {key}"),
            ));
        }
        match edit {
            DocumentEdit::Metadata { key, value } => {
                if !validate::CORE_FIELDS.contains(&key.as_str()) {
                    return Err(error("source-invalid", format!("Unknown core field {key}")));
                }
                fields.retain(|(k, _)| k != key);
                if let Some(v) = value {
                    fields.push((key.clone(), v.clone()));
                }
                metadata_changed = true;
            }
            DocumentEdit::Unit { id, text } => {
                let index = unit_indices
                    .get(id)
                    .ok_or_else(|| error("unit-missing", format!("Unknown unit {id}")))?;
                let unit = &mut units[*index];
                if !text.ends_with('\n') {
                    return Err(error(
                        "source-invalid",
                        "Unit text must end with LF or CRLF",
                    ));
                }
                if text.len() > limits.source_bytes {
                    return Err(error("resource-limit", "Unit text limit exceeded"));
                }
                unit.text.clone_from(text);
            }
        }
    }
    if doc.dialect() == Dialect::Rc3 && seen.contains(&("metadata", &String::from("title"))) {
        let title = fields
            .iter()
            .find(|(k, _)| k == "title")
            .and_then(|(_, v)| v.as_str())
            .ok_or_else(|| error("source-invalid", "Title must be a string"))?;
        let first = units.first_mut().expect("validated document has a unit");
        // Explicit first-unit edits must agree; never silently overwrite caller text.
        if !seen.contains(&("unit", &first.id)) {
            let end = first.text.find('\n').expect("validated heading line");
            let nl = if first.text[..end].ends_with('\r') {
                "\r\n"
            } else {
                "\n"
            };
            let mut heading = Output::new(limits.source_bytes);
            heading.push("# ")?;
            heading.push(title)?;
            heading.push(nl)?;
            heading.push(&first.text[end + 1..])?;
            first.text = heading.text;
        }
    }
    let mut out = Output::new(limits.source_bytes);
    let mut replacements = Vec::new();
    if metadata_changed {
        let nl = if doc.source[doc.metadata_span()].contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        replacements.push((doc.metadata_span(), metadata(&fields, nl, limits)?));
    }
    for (unit, original) in units.iter().zip(doc.units()) {
        if unit.text != doc.source[original.span.clone()] {
            replacements.push((original.span.clone(), unit.text.clone()));
        }
    }
    replacements.sort_by_key(|(range, _)| range.start);
    let mut cursor = 0;
    for (range, text) in replacements {
        out.push(&doc.source[cursor..range.start])?;
        out.push(&text)?;
        cursor = range.end;
    }
    out.push(&doc.source[cursor..])?;
    exact_units(&valid(out.text.as_bytes(), doc.dialect(), limits)?, &units)?;
    Ok(out.text.into_bytes())
}
