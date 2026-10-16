// SPDX-License-Identifier: Apache-2.0
//! Read-only overview is derived from live records, never a stale projection.
use std::path::PathBuf;
use std::process::Command;

fn war(args: &[&str]) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_war"))
        // This checkout's .git is not a test's to write caches into.
        .env("OPENWARRANT_NO_CACHE", "1")
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
fn overview_lists_each_current_unresolved_record_and_progress_is_an_alias() {
    let status = war(&["status", "--json"]);
    let overview = war(&["overview", "--json"]);
    assert_eq!(overview, war(&["progress", "--json"]));
    let expected: Vec<_> = status["warrants"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| {
            w["rung"] != "resolved"
                && w["state"]["currency"] == "current"
                && w["state"]["standing"] != "annulled"
        })
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
        // This checkout's .git is not a test's to write caches into.
        .env("OPENWARRANT_NO_CACHE", "1")
        .args(["overview", "--json"])
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_ne!(value["exit_code"], 0);
    assert!(value.get("result").is_none());
}

#[test]
fn status_derives_supersession_without_changing_the_predecessors_recorded_phase() {
    // The authorized successor names this predecessor. Its original journal
    // must remain history, rather than being rewritten to set currency.
    let status = war(&["status", "--json"]);
    let predecessor = status["warrants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["alias"] == "OW-WAR-0073")
        .unwrap();
    assert_eq!(predecessor["state"]["phase"], "authorized");
    assert_eq!(predecessor["state"]["currency"], "superseded");
    assert_eq!(predecessor["state"]["outcome"], "none");
    let successor = status["warrants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["alias"] == "OW-WAR-0112")
        .unwrap();
    assert_eq!(successor["state"]["currency"], "current");
}

#[test]
fn replaced_work_is_retained_but_not_offered_as_remaining_work() {
    let overview = war(&["overview", "--json"]);
    assert!(
        !overview["warrants"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["alias"] == "OW-WAR-0073"),
        "a replaced Warrant is history, not remaining work"
    );
    assert!(
        overview["warrants"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["alias"] == "OW-WAR-0112")
    );
    assert_eq!(
        overview["remaining"],
        overview["warrants"].as_array().unwrap().len()
    );
    let all = war(&["overview", "--all", "--json"]);
    let old = all["warrants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["alias"] == "OW-WAR-0073")
        .expect("history remains available");
    assert_eq!(old["phase"], "authorized");
    assert_eq!(old["currency"], "superseded");
    assert_eq!(all["remaining"], overview["remaining"]);
    let next = war(&["next", "--json"]);
    assert!(
        !next["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["warrant"] == "OW-WAR-0073")
    );
}
