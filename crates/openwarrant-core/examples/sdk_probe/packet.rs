// SPDX-License-Identifier: Apache-2.0
use super::read_bounded;
use openwarrant_core::document::packet::{PacketLimits, decode_packet};
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
    let directory = root.join("packet");
    let suite: Suite =
        serde_json::from_slice(&read_bounded(&directory.join("cases.json"), 1024 * 1024)?)
            .map_err(|e| e.to_string())?;
    if suite.schema != "oh.war/sdk-packet-cases/v1" || suite.cases.is_empty() {
        return Err("Unsupported or empty packet suite".into());
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
        let actual = decode_packet(&bytes, PacketLimits::default())
            .err()
            .map(|e| e.code.to_owned());
        let pass = actual == case.error;
        println!("{} {}", if pass { "PASS" } else { "FAIL" }, case.id);
        if !pass {
            failures.push(case.id.as_str());
        }
    }
    if !failures.is_empty() {
        return Err(format!("Packet cases failed: {}", failures.join(", ")));
    }
    println!(
        "{} packet wire cases passed; byte integrity and semantic coverage not evaluated",
        suite.cases.len()
    );
    Ok(())
}
