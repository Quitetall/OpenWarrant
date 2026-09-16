// SPDX-License-Identifier: Apache-2.0
//! File-backed conformance driver. Expectations are data; parser results determine pass/fail.
use openwarrant_core::document::{
    Dialect, ParseLimits, SemanticSupport, ValidationOptions, Validity, parse_document,
    validate_document,
};
#[path = "sdk_probe/author.rs"]
mod author;
#[path = "sdk_probe/conditions.rs"]
mod conditions;
#[path = "sdk_probe/legacy.rs"]
mod legacy;
#[path = "sdk_probe/packet.rs"]
mod packet;
#[path = "sdk_probe/records.rs"]
mod records;
#[path = "sdk_probe/source.rs"]
mod source;
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    path::{Component, Path},
    process::ExitCode,
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
    file: String,
    dialect: String,
    #[serde(default)]
    parse_error: Option<String>,
    #[serde(default)]
    validation_error: Option<String>,
    #[serde(default)]
    required_extension: bool,
    #[serde(default)]
    unit_ids: Vec<String>,
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err(format!("{}: input limit exceeded", path.display()));
    }
    Ok(bytes)
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut scope = None;
    let mut fixtures = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--scope" => scope = args.next(),
            "--fixtures" => fixtures = args.next(),
            _ => {
                return Err(format!(
                    "Unknown argument {arg}; use --scope 75 --fixtures conformance/sdk"
                ));
            }
        }
    }
    if !matches!(
        scope.as_deref(),
        Some("75" | "76" | "77" | "78" | "81" | "83" | "84" | "85")
    ) {
        return Err("This driver implements --scope 75, 76, 77, 78, 81, 83, 84 or 85".into());
    }
    let root = std::path::PathBuf::from(fixtures.ok_or("--fixtures is required")?);
    if scope.as_deref() == Some("84") {
        return legacy::run(&root);
    }
    if scope.as_deref() == Some("83") {
        return records::run(&root);
    }
    if scope.as_deref() == Some("81") {
        return packet::run(&root);
    }
    if scope.as_deref() == Some("78") {
        return conditions::run(&root);
    }
    if scope.as_deref() == Some("77") {
        return source::run(&root);
    }
    documents(&root, matches!(scope.as_deref(), Some("76" | "85")))?;
    if scope.as_deref() == Some("85") {
        source::run(&root)?;
        conditions::run(&root)?;
        packet::run(&root)?;
        records::run(&root)?;
        legacy::run(&root)?;
        println!(
            "SDK fixture suites passed; CLI, skills, platforms and phase exit are checked separately"
        );
    }
    Ok(())
}
fn documents(root: &Path, with_author: bool) -> Result<(), String> {
    let directory = root.join("document");
    let suite: Suite =
        serde_json::from_slice(&read_bounded(&directory.join("cases.json"), 1024 * 1024)?)
            .map_err(|e| e.to_string())?;
    if suite.schema != "oh.war/sdk-document-cases/v1" || suite.cases.is_empty() {
        return Err("Unsupported or empty suite".into());
    }
    let mut ids = BTreeSet::new();
    let mut failures = Vec::new();
    for case in &suite.cases {
        if !ids.insert(&case.id) {
            return Err(format!("Duplicate case {}", case.id));
        }
        let path = Path::new(&case.file);
        if path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
            || case.file.is_empty()
        {
            return Err("Fixture path must stay within document directory".into());
        }
        let dialect = match case.dialect.as_str() {
            "rc2" => Dialect::Rc2,
            "rc3" => Dialect::Rc3,
            _ => return Err("Unknown fixture dialect".into()),
        };
        let bytes = read_bounded(&directory.join(path), ParseLimits::default().source_bytes)?;
        let success = match parse_document(&bytes, dialect, ParseLimits::default()) {
            Err(error) => case.parse_error.as_deref() == Some(error.code),
            Ok(document) => {
                let report = validate_document(&document, &ValidationOptions::default());
                let validation_matches = match &case.validation_error {
                    Some(code) => {
                        report.validity == Validity::Invalid
                            && report.diagnostics.iter().any(|d| d.code == code)
                    }
                    None => report.validity == Validity::Valid,
                };
                case.parse_error.is_none()
                    && document.original() == bytes
                    && validation_matches
                    && (report.semantic_support == SemanticSupport::Unsupported)
                        == case.required_extension
                    && (case.unit_ids.is_empty()
                        || document.units().iter().map(|u| &u.id).eq(&case.unit_ids))
            }
        };
        println!("{} {}", if success { "PASS" } else { "FAIL" }, case.id);
        if !success {
            failures.push(case.id.clone());
        }
    }
    if !failures.is_empty() {
        return Err(format!(
            "{} failed: {}",
            failures.len(),
            failures.join(", ")
        ));
    }
    println!(
        "{} document cases passed; context/readiness not evaluated",
        suite.cases.len()
    );
    if with_author {
        author::run(root)?;
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
