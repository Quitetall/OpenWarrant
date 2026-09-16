// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::document::records::{
    RecordLimits, Subject, check_records, decode_agent_act, workflow::decode_workflow_record,
};
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
    operation: String,
    subject: Option<Subject>,
    error: Option<String>,
}
pub fn run(root: &Path) -> Result<(), String> {
    let root = root
        .join("records")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let suite: Suite =
        serde_json::from_slice(&super::read_bounded(&root.join("cases.json"), 1024 * 1024)?)
            .map_err(|e| e.to_string())?;
    if suite.schema != "oh.war/sdk-record-cases/v1"
        || suite.cases.is_empty()
        || suite.cases.len() > 4096
    {
        return Err("Invalid record suite".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for case in &suite.cases {
        if !ids.insert(&case.id) {
            return Err("Duplicate case ID".into());
        }
        let path = root
            .join(&case.file)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !path.starts_with(&root) {
            return Err("Fixture escapes root".into());
        }
        let bytes = super::read_bounded(&path, 4 * 1024 * 1024)?;
        let limits = RecordLimits::default();
        let result = match case.operation.as_str() {
            "record" => check_records(
                &[&bytes],
                case.subject.as_ref().ok_or("Record case needs subject")?,
                &[],
                limits,
            )
            .map(|_| ()),
            "agent-act" => decode_agent_act(&bytes, limits).map(|_| ()),
            "workflow" => decode_workflow_record(&bytes, limits).map(|_| ()),
            _ => return Err("Unknown fixture operation".into()),
        };
        let actual = result.err().map(|e| e.code.to_owned());
        if actual != case.error {
            return Err(format!(
                "{}: expected {:?}, observed {:?}",
                case.id, case.error, actual
            ));
        }
        println!(
            "{} PASS ({})",
            case.id,
            actual
                .as_deref()
                .unwrap_or("structure admitted; trust not established")
        );
    }
    println!(
        "{} file-backed codec cases passed; semantic readiness/assurance tests are document_records and document_workflow_records; no qualification issued",
        suite.cases.len()
    );
    Ok(())
}
