// SPDX-License-Identifier: Apache-2.0
//! `--json` (SAS §76.4): every envelope has the frozen keys, the exit code in
//! the envelope equals the process's, and stdout is exactly one JSON value.
//!
//! Runs the shipped binary against the repository's own corpus, read-only.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn war(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("war runs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

const FROZEN_KEYS: [&str; 8] = [
    "command",
    "counts",
    "diagnostics",
    "exit_code",
    "notes",
    "schema",
    "verdict",
    "verdict_line",
];

fn envelope(stdout: &str) -> serde_json::Value {
    let v: serde_json::Value = serde_json::from_str(stdout)
        .unwrap_or_else(|e| panic!("stdout is not one JSON value: {e}\n{stdout}"));
    assert_eq!(v["schema"], "oh.war/report/v1");
    let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    keys.retain(|k| *k != "result");
    keys.sort_unstable();
    assert_eq!(keys, FROZEN_KEYS, "envelope keys are frozen at 1.0");
    v
}

#[test]
fn check_emits_an_envelope_whose_exit_code_is_the_process_exit_code() {
    let (code, out, _) = war(&["--json", "check", "OW-WAR-0001"]);
    let v = envelope(&out);
    assert_eq!(v["command"], "check");
    assert_eq!(v["exit_code"].as_i64().unwrap(), i64::from(code));
    assert!(v["diagnostics"].as_array().unwrap().len() > 3);
    assert!(v["counts"]["pass"].as_u64().unwrap() > 0);
}

#[test]
fn the_flag_works_after_the_subcommand_too() {
    let (_, a, _) = war(&["--json", "check", "OW-WAR-0001"]);
    let (_, b, _) = war(&["check", "OW-WAR-0001", "--json"]);
    assert_eq!(a, b, "global flag; position does not matter");
}

#[test]
fn a_request_document_rides_inside_result_with_its_own_schema() {
    let (code, out, _) = war(&["--json", "authorize", "OW-WAR-0001"]);
    let v = envelope(&out);
    assert_eq!(v["command"], "authorize.request");
    assert_eq!(v["result"]["schema"], "oh.war/authorization-request/v1");
    assert_eq!(v["result"]["warrant"], "OW-WAR-0001");
    assert!(v["result"]["contract_digest"].as_str().unwrap().len() == 64);
    assert_eq!(code, 0);
}

#[test]
fn the_error_path_is_still_json_on_stdout_and_exits_one() {
    let (code, out, err) = war(&["--json", "show", "OW-WAR-9999"]);
    let v = envelope(&out);
    assert_eq!(v["command"], "error");
    assert_eq!(v["diagnostics"][0]["rule"], "cli.error");
    assert_eq!(v["verdict"], "not_ready");
    assert_eq!(
        code, 1,
        "a diagnostic error is exit 1, not the envelope's 2"
    );
    assert!(
        err.is_empty() || !err.contains("error:"),
        "nothing human on stderr under --json: {err}"
    );
}

#[test]
fn the_corpus_projection_is_the_result_payload_verbatim() {
    let (_, out, _) = war(&["--json", "status"]);
    let v = envelope(&out);
    assert_eq!(v["command"], "status");
    assert_eq!(v["result"]["schema"], "oh.war/corpus-status/v1");
    let committed =
        std::fs::read_to_string(repo_root().join("docs/warrants/generated/CORPUS_STATUS.json"))
            .expect("committed projection");
    let committed: serde_json::Value = serde_json::from_str(&committed).unwrap();
    assert_eq!(
        v["result"], committed,
        "the envelope carries the committed projection"
    );
}

#[test]
fn without_the_flag_stdout_never_begins_with_a_brace() {
    let (_, out, _) = war(&["check", "OW-WAR-0001"]);
    assert!(
        !out.trim_start().starts_with('{'),
        "human mode leaks JSON: {out}"
    );
}
