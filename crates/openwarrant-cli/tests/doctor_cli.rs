// SPDX-License-Identifier: Apache-2.0
use std::{fs, path::Path, process::Command};
fn war(root: &Path, args: &[&str]) -> (std::process::ExitStatus, serde_json::Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(root)
        .args(args)
        .arg("--json")
        .output()
        .unwrap();
    let value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "args={args:?} stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output.status, value)
}
fn snapshot(root: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut files = std::collections::BTreeMap::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(snapshot(&path));
        } else {
            files.insert(path.clone(), fs::read(path).unwrap());
        }
    }
    files
}
#[test]
fn doctor_aggregates_refusals_without_running_backends_or_mutating_records() {
    let root = std::env::temp_dir().join(format!(
        "ow-doctor-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    // A malformed repository must still produce a doctor envelope.
    fs::write(root.join("openwarrant.toml"), "[broken").unwrap();
    let (status, value) = war(&root, &["doctor"]);
    assert!(!status.success());
    assert_eq!(value["command"], "doctor");
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "doctor.repository" && d["severity"] == "error")
    );
    fs::remove_file(root.join("openwarrant.toml")).unwrap();
    assert!(
        Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(&root)
            .args(["init", "--namespace", "DOC"])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(war(&root, &["new", "diagnose this"]).0.success());
    let config_path = root.join("openwarrant.toml");
    let mut config: toml::Value =
        toml::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    config.as_table_mut().unwrap().insert(
        "perform".into(),
        toml::Value::try_from(
            serde_json::json!({"performer_argv":["sh","-c","touch BACKEND_RAN"]}),
        )
        .unwrap(),
    );
    fs::write(&config_path, toml::to_string(&config).unwrap()).unwrap();
    fs::create_dir_all(root.join("docs/authority")).unwrap();
    fs::write(root.join("docs/authority/roles.toml"), "[malformed").unwrap();
    let before = snapshot(&root);
    let (status, value) = war(&root, &["doctor", "DOC-WAR-0001"]);
    assert!(!status.success());
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "doctor.authority" && d["severity"] == "error")
    );
    assert_eq!(
        value["result"]["configuration"]["performer_configured"],
        true
    );
    assert_eq!(value["result"]["configuration"]["backend_probed"], false);
    assert_eq!(value["result"]["admission"], "UNKNOWN");
    assert_eq!(value["result"]["execution_authorized"], false);
    assert_eq!(
        value["result"]["remedies"][0]["argv"],
        serde_json::json!(["war", "check", "DOC-WAR-0001"])
    );
    assert_eq!(before, snapshot(&root));
    fs::remove_file(root.join("docs/authority/roles.toml")).unwrap();
    let (_, value) = war(&root, &["doctor"]);
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "doctor.authority-absent" && d["severity"] == "warn")
    );
    fs::create_dir(root.join("docs/authority/roles.toml")).unwrap();
    let (status, value) = war(&root, &["doctor"]);
    assert!(!status.success());
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "doctor.authority-not-file" && d["severity"] == "error")
    );
    // Missing target cannot yield a successful diagnostic report.
    assert!(!war(&root, &["doctor", "DOC-WAR-9999"]).0.success());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn doctor_accepts_clean_scaffold_and_refuses_generated_drift() {
    let root = std::env::temp_dir().join(format!(
        "ow-doctor-clean-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(&root)
        .args([
            "init",
            "--namespace",
            "DOC",
            "--program",
            "Diagnostic fixture",
        ])
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    assert!(
        Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(&root)
            .arg("compile")
            .output()
            .unwrap()
            .status
            .success()
    );
    let before = snapshot(&root);
    let (status, value) = war(&root, &["doctor", "--generated"]);
    assert!(status.success(), "{value}");
    assert_eq!(before, snapshot(&root));
    assert_eq!(value["result"]["admission"], "UNKNOWN");
    // A clean record pass must not hide a planted projection drift.
    let generated = root.join("docs/warrants/DOC-WAR-0001/generated/WAR.md");
    fs::write(&generated, "planted drift").unwrap();
    let planted = snapshot(&root);
    let (status, value) = war(&root, &["doctor", "--generated"]);
    assert!(!status.success(), "{value}");
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["severity"] == "error"
                && d["rule"] == "generated.drift"
                && d["file"] == "docs/warrants/DOC-WAR-0001/generated/WAR.md")
    );
    assert_eq!(planted, snapshot(&root));
    fs::remove_dir_all(root).unwrap();
}
