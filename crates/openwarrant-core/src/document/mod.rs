// SPDX-License-Identifier: Apache-2.0
//! Pure RC.3 document parsing and explicit RC.2 compatibility.
//!
//! Parsing preserves original bytes and separates source syntax from document
//! validity. Neither operation resolves external context or grants permission.

use std::ops::Range;

pub use toml10::Value as MetadataValue;

mod metadata;
mod references;
mod scan;
mod validate;
pub use validate::{
    Evaluation, SemanticSupport, ValidationOptions, ValidationReport, Validity, validate_document,
};

/// Explicit source dialect. Legacy Markdown is never guessed to be RC.2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dialect {
    Rc2,
    Rc3,
}

impl Dialect {
    pub fn schema(self) -> &'static str {
        match self {
            Self::Rc2 => "oh.war/document/1.0.0-rc.2",
            Self::Rc3 => "oh.war/document/1.0.0-rc.3",
        }
    }
}

/// Positive, inclusive resource bounds checked before parsing or collecting units.
#[derive(Clone, Copy, Debug)]
pub struct ParseLimits {
    pub source_bytes: usize,
    pub metadata_bytes: usize,
    pub units: usize,
    /// Conservative key/container depth bound (1..=128); table-path segments
    /// count twice because prior array tables may add an implicit level.
    pub metadata_depth: usize,
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            source_bytes: 8 * 1024 * 1024,
            metadata_bytes: 1024 * 1024,
            units: 65_536,
            metadata_depth: 64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A diagnostic range always indexes the caller's original UTF-8 source bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub span: Range<usize>,
}

impl Diagnostic {
    fn error(code: &'static str, message: impl Into<String>, span: Range<usize>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitKind {
    Binding,
    Background,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unit {
    pub id: String,
    pub kind: UnitKind,
    pub span: Range<usize>,
}

/// Borrowed original bytes plus independently parsed metadata and source map.
/// Construction is private so callers cannot substitute fabricated unit ranges.
#[derive(Debug)]
pub struct Document<'a> {
    source: &'a str,
    dialect: Dialect,
    metadata: MetadataValue,
    metadata_span: Range<usize>,
    units: Vec<Unit>,
}

impl<'a> Document<'a> {
    pub fn original(&self) -> &'a [u8] {
        self.source.as_bytes()
    }
    pub fn dialect(&self) -> Dialect {
        self.dialect
    }
    pub fn metadata(&self) -> &MetadataValue {
        &self.metadata
    }
    pub fn metadata_span(&self) -> Range<usize> {
        self.metadata_span.clone()
    }
    pub fn units(&self) -> &[Unit] {
        &self.units
    }
    pub fn unit(&self, id: &str) -> Option<&'a str> {
        self.units
            .iter()
            .find(|unit| unit.id == id)
            .map(|unit| &self.source[unit.span.clone()])
    }
}

/// Parse one explicitly selected document dialect without I/O or normalization.
pub fn parse_document(
    bytes: &[u8],
    dialect: Dialect,
    limits: ParseLimits,
) -> Result<Document<'_>, Diagnostic> {
    if limits.source_bytes == 0
        || limits.metadata_bytes == 0
        || limits.units == 0
        || limits.metadata_depth == 0
        || limits.metadata_depth > 128
    {
        return Err(Diagnostic::error(
            "resource-limit",
            "Limits must be positive; metadata depth must not exceed 128",
            0..0,
        ));
    }
    if bytes.len() > limits.source_bytes {
        return Err(Diagnostic::error(
            "resource-limit",
            "Source byte limit exceeded",
            0..0,
        ));
    }
    let source = std::str::from_utf8(bytes).map_err(|error| {
        Diagnostic::error(
            "source-invalid",
            "Invalid UTF-8",
            error.valid_up_to()..error.valid_up_to() + error.error_len().unwrap_or(1),
        )
    })?;
    if source.starts_with('\u{feff}') {
        return Err(Diagnostic::error(
            "source-invalid",
            "BOM is forbidden",
            0..source.len().min(3),
        ));
    }
    if let Some(offset) = source.find('\0') {
        return Err(Diagnostic::error(
            "source-invalid",
            "NUL is forbidden",
            offset..offset + 1,
        ));
    }
    let (metadata_span, units) = scan::scan(source, dialect, limits)?;
    metadata::preflight(
        &source[metadata_span.clone()],
        metadata_span.start,
        limits.metadata_depth,
    )?;
    let metadata: MetadataValue =
        source[metadata_span.clone()]
            .parse()
            .map_err(|error: toml10::de::Error| {
                let relative = error.span().unwrap_or(0..0);
                Diagnostic::error(
                    "source-invalid",
                    error.message(),
                    (metadata_span.start + relative.start)..(metadata_span.start + relative.end),
                )
            })?;
    metadata::check_values(&metadata, &metadata_span, 0, limits.metadata_depth)?;
    if metadata.get("schema").and_then(MetadataValue::as_str) != Some(dialect.schema()) {
        return Err(Diagnostic::error(
            "schema-unsupported",
            "Declared schema does not match the selected dialect",
            metadata_span,
        ));
    }
    Ok(Document {
        source,
        dialect,
        metadata,
        metadata_span,
        units,
    })
}
