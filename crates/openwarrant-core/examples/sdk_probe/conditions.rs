// SPDX-License-Identifier: Apache-2.0
use super::read_bounded;
use openwarrant_core::document::{
    MetadataValue,
    condition::{ConditionLimits, validate_condition},
};
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
    file: String,
    error: Option<String>,
}

pub fn run(root: &Path) -> Result<(), String> {
    let directory = root.join("conditions");
    let suite: Suite =
        serde_json::from_slice(&read_bounded(&directory.join("cases.json"), 1024 * 1024)?)
            .map_err(|e| e.to_string())?;
    if suite.schema != "oh.war/sdk-condition-cases/v1" || suite.cases.is_empty() {
        return Err("Unsupported or empty condition suite".into());
    }
    let mut ids = BTreeSet::new();
    let mut failures = Vec::new();
    for case in &suite.cases {
        if !ids.insert(&case.id)
            || case.file.is_empty()
            || Path::new(&case.file)
                .components()
                .any(|p| !matches!(p, Component::Normal(_)))
        {
            return Err("Duplicate case or unsafe fixture path".into());
        }
        let bytes = read_bounded(&directory.join(&case.file), 1024 * 1024)?;
        let source = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
        let metadata: MetadataValue = source
            .parse()
            .map_err(|e: toml10::de::Error| e.to_string())?;
        let actual = validate_condition(&metadata, ConditionLimits::default())
            .err()
            .map(|e| e.code.to_owned());
        let pass = actual == case.error;
        println!("{} {}", if pass { "PASS" } else { "FAIL" }, case.id);
        if !pass {
            failures.push(case.id.as_str());
        }
    }
    if !failures.is_empty() {
        return Err(format!("Condition cases failed: {}", failures.join(", ")));
    }
    println!(
        "{} condition syntax cases passed; applicability not evaluated",
        suite.cases.len()
    );
    Ok(())
}
