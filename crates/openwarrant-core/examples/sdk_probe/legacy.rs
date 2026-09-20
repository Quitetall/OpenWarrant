// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::document::legacy::{Limits, check_successor, export, import};
use serde::Deserialize;
use std::path::Path;
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
    successor: Option<String>,
    paths: Option<Vec<String>>,
    error: Option<String>,
}
pub fn run(root: &Path) -> Result<(), String> {
    let root = root
        .join("legacy")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let suite: Suite =
        serde_json::from_slice(&super::read_bounded(&root.join("cases.json"), 1024 * 1024)?)
            .map_err(|e| e.to_string())?;
    if suite.schema != "oh.war/sdk-legacy-cases/v1"
        || suite.cases.is_empty()
        || suite.cases.len() > 4096
    {
        return Err("Invalid legacy suite".into());
    }
    let read = |name: &str| -> Result<Vec<u8>, String> {
        let path = root.join(name).canonicalize().map_err(|e| e.to_string())?;
        if !path.starts_with(&root) {
            return Err("Fixture escapes root".into());
        }
        super::read_bounded(&path, 16 * 1024 * 1024)
    };
    let mut ids = std::collections::BTreeSet::new();
    for case in &suite.cases {
        if !ids.insert(&case.id) {
            return Err("Duplicate case identity".into());
        }
        let bytes = read(&case.file)?;
        let successor = case.successor.as_ref().map(|f| read(f)).transpose()?;
        let result = import(&bytes, Limits::default()).and_then(|old| {
            if let Some(bytes) = successor {
                let new = import(&bytes, Limits::default())?;
                let paths: Vec<_> = case
                    .paths
                    .as_deref()
                    .unwrap_or(&[])
                    .iter()
                    .map(String::as_str)
                    .collect();
                check_successor(&old, &new, &paths, Limits::default())?;
            }
            let encoded = export(&old, Limits::default())?;
            let restored = import(&encoded, Limits::default())?;
            assert_eq!(restored.files(), old.files());
            assert_eq!(export(&restored, Limits::default())?, encoded);
            assert!(!restored.report().qualification_established);
            Ok(())
        });
        match (result, case.error.as_deref()) {
            (Ok(()), None) => println!(
                "{} PASS (preserved inventory; no authority inferred)",
                case.id
            ),
            (Err(e), Some(code)) if e.code == code => println!("{} PASS ({code})", case.id),
            (actual, expected) => {
                return Err(format!(
                    "{}: expected {expected:?}, got {actual:?}",
                    case.id
                ));
            }
        }
    }
    println!(
        "{} file-backed preservation/lineage cases passed; external historical closure and signature authenticity not established",
        suite.cases.len()
    );
    Ok(())
}
