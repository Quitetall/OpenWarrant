// SPDX-License-Identifier: Apache-2.0
//! Explicit fixture I/O only. This is not a source-capture provider.
use super::read_bounded;
use openwarrant_core::document::source::{
    BoundReference, SourceLimits, check_sources, decode_source_descriptor,
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
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
    sources: Vec<Input>,
    references: Vec<Reference>,
    expected_lock: Option<String>,
    error: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    descriptor: String,
    blob: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    reference: BoundReference,
    expected: String,
}

fn file(root: &Path, name: &str, limit: usize) -> Result<Vec<u8>, String> {
    if name.is_empty()
        || Path::new(name)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Fixture path must be relative and contained".into());
    }
    read_bounded(&root.join(name), limit)
}

fn evaluate(root: &Path, case: &Case) -> Result<Vec<u8>, String> {
    let limits = SourceLimits::default();
    let mut sources = Vec::new();
    let mut blobs = BTreeMap::new();
    let mut total = 0usize;
    if case.sources.len() > limits.sources || case.references.len() > limits.units {
        return Err("resource-limit".into());
    }
    for input in &case.sources {
        let descriptor = decode_source_descriptor(
            &file(root, &input.descriptor, limits.metadata_bytes)?,
            limits,
        )
        .map_err(|e| e.code.to_owned())?;
        let bytes = file(root, &input.blob, limits.source_bytes)?;
        total = total
            .checked_add(bytes.len())
            .filter(|n| *n <= limits.total_bytes)
            .ok_or("resource-limit")?;
        if let Some(previous) = blobs.insert(descriptor.source_digest.clone(), bytes.clone())
            && previous != bytes
        {
            return Err("digest-mismatch".into());
        }
        sources.push(descriptor);
    }
    let checked = check_sources(&sources, &blobs, limits).map_err(|e| e.code.to_owned())?;
    for reference in &case.references {
        let actual = checked
            .check_reference(&reference.reference)
            .map_err(|e| e.code.to_owned())?;
        if actual != file(root, &reference.expected, limits.source_bytes)? {
            return Err("fixture-range-content-mismatch".into());
        }
    }
    checked.canonical_lock().map_err(|e| e.code.to_owned())
}

pub fn run(root: &Path) -> Result<(), String> {
    let root = root.join("source");
    let suite: Suite = serde_json::from_slice(&file(&root, "cases.json", 1024 * 1024)?)
        .map_err(|e| e.to_string())?;
    if suite.schema != "oh.war/sdk-source-cases/v1"
        || suite.cases.is_empty()
        || suite.cases.len() > 1024
    {
        return Err("Unsupported, empty or over-limit source suite".into());
    }
    let mut ids = BTreeSet::new();
    let mut failures = Vec::new();
    for case in &suite.cases {
        if !ids.insert(&case.id) || case.error.is_some() == case.expected_lock.is_some() {
            return Err("Duplicate case or ambiguous expectation".into());
        }
        let passed = match evaluate(&root, case) {
            Ok(lock) => {
                case.error.is_none()
                    && lock
                        == file(
                            &root,
                            case.expected_lock.as_ref().ok_or("Missing expected lock")?,
                            SourceLimits::default().output_bytes,
                        )?
            }
            Err(error) => case.error.as_deref() == Some(error.as_str()),
        };
        println!("{} {}", if passed { "PASS" } else { "FAIL" }, case.id);
        if !passed {
            failures.push(case.id.clone());
        }
    }
    if !failures.is_empty() {
        return Err(format!("Source cases failed: {}", failures.join(", ")));
    }
    println!(
        "{} source cases passed; supplied-byte integrity only; capture, access, context completeness, readiness and authority not evaluated",
        suite.cases.len()
    );
    Ok(())
}
