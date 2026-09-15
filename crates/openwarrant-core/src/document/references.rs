// SPDX-License-Identifier: Apache-2.0
use super::{Diagnostic, Document, MetadataValue, scan::unit_id};
use std::collections::BTreeSet;

fn path(value: &str) -> bool {
    !value.is_empty()
        && !value
            .chars()
            .any(|c| c.is_control() || matches!(c, '\\' | ':' | '#'))
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn target(value: &str) -> bool {
    match value.split_once('#') {
        Some((file, unit)) => (file.is_empty() || path(file)) && (unit == "*" || unit_id(unit)),
        None => path(value),
    }
}

fn pattern(value: &str) -> bool {
    path(value)
        && !value.contains(['?', '[', ']', '{', '}'])
        && value
            .split('/')
            .all(|part| part == "**" || !part.contains("**"))
}

fn strings(value: &MetadataValue, nonempty_array: bool, predicate: fn(&str) -> bool) -> bool {
    value.as_array().is_some_and(|values| {
        (!nonempty_array || !values.is_empty())
            && values.iter().all(|v| v.as_str().is_some_and(predicate))
    })
}

fn condition(value: &MetadataValue) -> bool {
    value.as_table().is_some_and(|fields| {
        !fields.is_empty()
            && fields.iter().all(|(key, value)| match key.as_str() {
                "stage" | "subsystem" => strings(value, true, |s| !s.is_empty()),
                "path" => strings(value, true, pattern),
                _ => false,
            })
    })
}

pub(super) fn validate(document: &Document<'_>, diagnostics: &mut Vec<Diagnostic>) {
    let fields = document.metadata.as_table().expect("TOML table");
    let mut issue = |code, message: String| {
        diagnostics.push(Diagnostic::error(
            code,
            message,
            document.metadata_span.clone(),
        ))
    };
    if let Some(scope) = fields.get("scope") {
        match scope.as_table() {
            Some(scope) => {
                for (key, value) in scope {
                    let valid = match key.as_str() {
                        "subsystems" => strings(value, false, |_| true),
                        "paths" => strings(value, false, path),
                        _ => false,
                    };
                    if !valid {
                        issue("source-invalid", format!("Invalid scope field {key}"));
                    }
                }
            }
            None => issue("source-invalid", "scope must be a table".into()),
        }
    }
    let local_units: BTreeSet<&str> = document.units.iter().map(|u| u.id.as_str()).collect();
    let mut pointers = BTreeSet::new();
    for field in ["context", "dependencies", "conflicts"] {
        let Some(value) = fields.get(field) else {
            continue;
        };
        let Some(entries) = value.as_array() else {
            issue("source-invalid", format!("{field} must be an array"));
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let Some(table) = entry.as_table() else {
                issue(
                    "source-invalid",
                    format!("{field}[{index}] must be a table"),
                );
                continue;
            };
            let allowed: &[&str] = if field == "context" {
                &["id", "target", "required", "when"]
            } else {
                &["unit", "target"]
            };
            for key in table.keys() {
                if !allowed.contains(&key.as_str()) {
                    issue("source-invalid", format!("Unknown {field} field {key}"));
                }
            }
            if !table
                .get("target")
                .and_then(MetadataValue::as_str)
                .is_some_and(target)
            {
                issue("target-invalid", format!("Invalid {field}[{index}] target"));
            }
            if field == "context" {
                if !table
                    .get("id")
                    .and_then(MetadataValue::as_str)
                    .is_some_and(|id| unit_id(id) && pointers.insert(id))
                {
                    issue(
                        "source-invalid",
                        format!("Invalid or duplicate pointer ID at {index}"),
                    );
                }
                if !table.get("required").is_some_and(MetadataValue::is_bool) {
                    issue(
                        "source-invalid",
                        format!("Pointer required must be a boolean at {index}"),
                    );
                }
                if table.get("when").is_some_and(|when| !condition(when)) {
                    issue(
                        "condition-invalid",
                        format!("Invalid condition at pointer {index}"),
                    );
                }
            } else if !table
                .get("unit")
                .and_then(MetadataValue::as_str)
                .is_some_and(|id| local_units.contains(id))
            {
                issue(
                    "source-invalid",
                    format!("Unknown local unit at {field}[{index}]"),
                );
            }
        }
    }
}
