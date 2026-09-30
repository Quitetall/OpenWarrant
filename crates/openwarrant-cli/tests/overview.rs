// SPDX-License-Identifier: Apache-2.0
//! Read-only overview is derived from live records, never a stale projection.
use std::path::PathBuf;
use std::process::Command;

fn war(args: &[&str]) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(args)
        .current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["result"].clone()
}

#[test]
fn overview_lists_each_unresolved_record_and_progress_is_an_alias() {
    let status = war(&["status", "--json"]);
    let overview = war(&["overview", "--json"]);
    assert_eq!(overview, war(&["progress", "--json"]));
    let expected: Vec<_> = status["warrants"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| w["rung"] != "resolved")
        .map(|w| w["alias"].clone())
        .collect();
    let actual: Vec<_> = overview["warrants"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["alias"].clone())
        .collect();
    assert!(!expected.is_empty());
    assert_eq!(actual, expected);
    assert_eq!(
        overview["remaining"].as_u64().unwrap() as usize,
        actual.len()
    );
    assert_eq!(overview["status_model"], "legacy-records");
    assert!(!overview["qualification_assessed"].as_bool().unwrap());
    let all = war(&["overview", "--all", "--json"]);
    assert_eq!(
        all["warrants"].as_array().unwrap().len(),
        status["warrants"].as_array().unwrap().len()
    );
    assert!(
        all["warrants"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["assessment"] == "resolved")
    );
}

#[test]
fn overview_outside_repository_refuses_instead_of_claiming_no_work() {
    let output = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["overview", "--json"])
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_ne!(value["exit_code"], 0);
    assert!(value.get("result").is_none());
}
