// SPDX-License-Identifier: Apache-2.0
use super::read_bounded;
use openwarrant_core::document::*;
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    path::{Component, Path},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Suite {
    schema: String,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    input: String,
    expected: Option<String>,
    error: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    metadata: Vec<(String, MetadataValue)>,
    units: Vec<UnitInput>,
    #[serde(default)]
    crlf: bool,
    #[serde(default)]
    wrapped: bool,
    #[serde(default)]
    edits: Vec<Edit>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnitInput {
    id: String,
    kind: String,
    text: String,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
enum Edit {
    Metadata {
        key: String,
        value: Option<MetadataValue>,
    },
    Unit {
        id: String,
        text: String,
    },
}
fn file(root: &Path, name: &str) -> Result<Vec<u8>, String> {
    if name.is_empty()
        || Path::new(name)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Author fixture path must be relative and contained".into());
    }
    read_bounded(&root.join(name), ParseLimits::default().source_bytes)
}
pub fn run(root: &Path) -> Result<(), String> {
    let root = root.join("author");
    let suite: Suite =
        serde_json::from_slice(&file(&root, "cases.json")?).map_err(|e| e.to_string())?;
    if suite.schema != "oh.war/sdk-author-cases/v1" || suite.cases.is_empty() {
        return Err("Unsupported or empty author suite".into());
    }
    let mut ids = BTreeSet::new();
    let mut failures = Vec::new();
    for case in &suite.cases {
        if !ids.insert(&case.id) || case.expected.is_some() == case.error.is_some() {
            return Err("Duplicate case or ambiguous expectation".into());
        }
        let request: Request =
            serde_json::from_slice(&file(&root, &case.input)?).map_err(|e| e.to_string())?;
        let units = request
            .units
            .into_iter()
            .map(|u| {
                Ok(AuthoredUnit {
                    id: u.id,
                    text: u.text,
                    kind: match u.kind.as_str() {
                        "binding" => UnitKind::Binding,
                        "background" => UnitKind::Background,
                        _ => return Err("Unknown unit kind".to_string()),
                    },
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let result = author_document(
            &DocumentFields {
                metadata: request.metadata,
                units,
            },
            AuthorOptions {
                line_ending: if request.crlf {
                    LineEnding::CrLf
                } else {
                    LineEnding::Lf
                },
                wrap_metadata: request.wrapped,
            },
            ParseLimits::default(),
        )
        .and_then(|bytes| {
            let doc = parse_document(&bytes, Dialect::Rc3, ParseLimits::default())?;
            let edits: Vec<_> = request
                .edits
                .into_iter()
                .map(|e| match e {
                    Edit::Metadata { key, value } => DocumentEdit::Metadata { key, value },
                    Edit::Unit { id, text } => DocumentEdit::Unit { id, text },
                })
                .collect();
            edit_document(&doc, &edits, ParseLimits::default())
        });
        let success = match result {
            Err(e) => case.error.as_deref() == Some(e.code),
            Ok(bytes) => {
                if let Some(expected) = &case.expected {
                    let doc = parse_document(&bytes, Dialect::Rc3, ParseLimits::default())
                        .map_err(|e| e.message)?;
                    bytes == file(&root, expected)?
                        && validate_document(&doc, &ValidationOptions::default()).validity
                            == Validity::Valid
                } else {
                    false
                }
            }
        };
        println!("{} {}", if success { "PASS" } else { "FAIL" }, case.id);
        if !success {
            failures.push(case.id.clone());
        }
    }
    if !failures.is_empty() {
        return Err(format!("Author cases failed: {}", failures.join(", ")));
    }
    println!(
        "{} author/edit cases passed; no save, execution or assurance performed",
        suite.cases.len()
    );
    Ok(())
}
