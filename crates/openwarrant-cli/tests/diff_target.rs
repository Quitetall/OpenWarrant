// SPDX-License-Identifier: Apache-2.0
use std::{fs, path::PathBuf, process::Command};

#[test]
fn explicit_target_reports_field_movement_and_ignores_json_layout() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temp = std::env::temp_dir().join(format!("ow-diff-target-{}", std::process::id()));
    fs::create_dir_all(&temp).unwrap();
    let from = temp.join("from.json");
    let to = temp.join("to.json");
    fs::write(&from, r#"{"contract_digest":"old","scope":{"limit":1}}"#).unwrap();
    fs::write(&to, r#"{"contract_digest":"new","scope":{"limit":2}}"#).unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(&root)
            .args(["diff", "OW-WAR-0037", "--from"])
            .arg(&from)
            .arg("--to")
            .arg(&to)
            .arg("--json")
            .output()
            .unwrap()
    };
    let output = run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("scope.limit: 1 -> 2"), "{text}");
    assert!(text.contains("contract_digest"), "{text}");
    fs::write(
        &to,
        "{\n\"scope\": {\"limit\":1}, \"contract_digest\":\"old\"\n}",
    )
    .unwrap();
    let output = run();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("diff.identical")
    );
    fs::write(&to, r#"{"scope":1,"scope":2}"#).unwrap();
    assert!(!run().status.success(), "duplicate members must refuse");
    fs::write(&to, "not JSON").unwrap();
    assert!(!run().status.success(), "malformed target must refuse");
    let key = "k".repeat(8192);
    fs::write(
        &from,
        serde_json::to_vec(&serde_json::json!({key.clone(): vec![0; 2048]})).unwrap(),
    )
    .unwrap();
    fs::write(
        &to,
        serde_json::to_vec(&serde_json::json!({key: vec![1; 2048]})).unwrap(),
    )
    .unwrap();
    let output = run();
    assert!(
        !output.status.success(),
        "amplified paths must refuse before walking"
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("comparison path budget"));
    fs::remove_dir_all(temp).unwrap();
}
