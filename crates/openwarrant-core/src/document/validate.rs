// SPDX-License-Identifier: Apache-2.0
use super::{Diagnostic, Dialect, Document, MetadataValue, Severity, UnitKind};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Validity {
    Valid,
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Evaluation {
    NotEvaluated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSupport {
    Supported,
    Unsupported,
}

#[derive(Clone, Debug)]
pub struct ValidationOptions {
    pub supported_dialects: Vec<Dialect>,
    pub supported_extensions: BTreeSet<String>,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            supported_dialects: vec![Dialect::Rc2, Dialect::Rc3],
            supported_extensions: BTreeSet::new(),
        }
    }
}

/// Structural validity is independent of resolution, readiness and extension support.
#[derive(Clone, Debug)]
pub struct ValidationReport {
    pub validity: Validity,
    pub semantic_support: SemanticSupport,
    pub context_resolution: Evaluation,
    pub readiness: Evaluation,
    pub diagnostics: Vec<Diagnostic>,
}

pub(super) fn identity(value: &str) -> bool {
    let Some((namespace, name)) = value.split_once(':') else {
        return false;
    };
    !namespace.is_empty()
        && namespace.as_bytes()[0].is_ascii_lowercase()
        && namespace
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        && !name.is_empty()
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._:/-".contains(&c))
}

fn nonempty(value: &MetadataValue) -> bool {
    value.as_str().is_some_and(|s| !s.is_empty())
}

/// Validate declared document facts only; external references remain unevaluated.
pub fn validate_document(document: &Document<'_>, options: &ValidationOptions) -> ValidationReport {
    let mut diagnostics = Vec::new();
    let mut issue = |message: String| {
        diagnostics.push(Diagnostic::error(
            "source-invalid",
            message,
            document.metadata_span.clone(),
        ))
    };
    let fields = document.metadata.as_table().expect("TOML root table");
    const ALLOWED: &[&str] = &[
        "schema",
        "kind",
        "id",
        "revision",
        "title",
        "state",
        "alias",
        "scope",
        "context",
        "dependencies",
        "conflicts",
        "extensions",
        "requires_extensions",
    ];
    for key in fields.keys() {
        if !ALLOWED.contains(&key.as_str()) {
            issue(format!("Unknown core field {key}"));
        }
    }
    for key in ["schema", "kind", "id", "revision", "title", "state"] {
        if !fields.contains_key(key) {
            issue(format!("Missing required field {key}"));
        }
    }
    if !fields
        .get("id")
        .and_then(MetadataValue::as_str)
        .is_some_and(identity)
    {
        issue("Invalid id".into());
    }
    if !fields
        .get("revision")
        .and_then(MetadataValue::as_integer)
        .is_some_and(|v| v > 0)
    {
        issue("Invalid revision".into());
    }
    if !fields
        .get("title")
        .and_then(MetadataValue::as_str)
        .is_some_and(|v| {
            !v.is_empty() && v.chars().count() <= 256 && !v.chars().any(char::is_control)
        })
    {
        issue("Invalid title".into());
    }
    if !matches!(
        fields.get("state").and_then(MetadataValue::as_str),
        Some("draft" | "proposed")
    ) {
        issue("Invalid state".into());
    }
    if fields.get("alias").is_some_and(|v| !nonempty(v)) {
        issue("Invalid alias".into());
    }
    let required: &[&str] = match fields.get("kind").and_then(MetadataValue::as_str) {
        Some("warrant" | "sas") => &["outcome", "scope", "context"],
        Some("adr") => &["context", "decision", "consequences"],
        Some("context") => &[],
        _ => {
            issue("Invalid kind".into());
            &[]
        }
    };
    if document.units.is_empty() {
        issue("Document requires at least one unit".into());
    }
    if fields.get("kind").and_then(MetadataValue::as_str) == Some("context")
        && !document.units.iter().any(|unit| {
            unit.kind == UnitKind::Binding
                && document.source[unit.span.clone()]
                    .split_once('\n')
                    .is_some_and(|(_, body)| !body.trim().is_empty())
        })
    {
        issue("Context requires a nonempty binding unit".into());
    }
    for id in required {
        let valid = document
            .units
            .iter()
            .find(|u| u.id == *id)
            .is_some_and(|unit| {
                unit.kind == UnitKind::Binding
                    && document.source[unit.span.clone()]
                        .split_once('\n')
                        .is_some_and(|(_, body)| !body.trim().is_empty())
            });
        if !valid {
            issue(format!("Required binding unit {id} is missing or empty"));
        }
    }
    if document.dialect == Dialect::Rc3 {
        let first_heading = document.units.first().map(|unit| {
            document.source[unit.span.clone()]
                .lines()
                .next()
                .unwrap_or("")
        });
        let matches = first_heading
            .and_then(|line| line.strip_prefix("# "))
            .is_some_and(|heading| {
                Some(heading) == fields.get("title").and_then(MetadataValue::as_str)
            });
        if !matches {
            issue("First heading must be level one and equal title exactly".into());
        }
    }
    let mut required_extensions = BTreeSet::new();
    if let Some(required) = fields.get("requires_extensions") {
        match required.as_array() {
            Some(values) => {
                for value in values {
                    match value.as_str() {
                        Some(id) if identity(id) && required_extensions.insert(id) => {}
                        _ => issue(
                            "requires_extensions must contain unique namespace-qualified IDs"
                                .into(),
                        ),
                    }
                }
            }
            None => issue("requires_extensions must be an array".into()),
        }
    }
    if let Some(extensions) = fields.get("extensions") {
        match extensions.as_table() {
            Some(values) => {
                for id in values.keys() {
                    if !identity(id) {
                        issue(format!("Invalid extension ID {id}"));
                    }
                }
            }
            None => issue("extensions must be a table".into()),
        }
    }
    super::references::validate(document, &mut diagnostics);
    let validity = if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        Validity::Invalid
    } else {
        Validity::Valid
    };
    let mut semantic_support = SemanticSupport::Supported;
    for id in required_extensions {
        if !options.supported_extensions.contains(id) {
            semantic_support = SemanticSupport::Unsupported;
            diagnostics.push(Diagnostic::error(
                "extension-required",
                format!("Required extension {id} is unsupported"),
                document.metadata_span.clone(),
            ));
        }
    }
    if let Some(extensions) = fields.get("extensions").and_then(MetadataValue::as_table) {
        for id in extensions.keys() {
            if !options.supported_extensions.contains(id) {
                diagnostics.push(Diagnostic {
                    code: "extension-unknown",
                    severity: Severity::Warning,
                    message: format!("Optional extension {id} retained without interpretation"),
                    span: document.metadata_span.clone(),
                });
            }
        }
    }
    if !options.supported_dialects.contains(&document.dialect) {
        diagnostics.push(Diagnostic::error(
            "schema-unsupported",
            "Schema unsupported by this caller",
            document.metadata_span.clone(),
        ));
    }
    let validity = if !options.supported_dialects.contains(&document.dialect) {
        Validity::Invalid
    } else {
        validity
    };
    ValidationReport {
        validity,
        semantic_support,
        context_resolution: Evaluation::NotEvaluated,
        readiness: Evaluation::NotEvaluated,
        diagnostics,
    }
}
