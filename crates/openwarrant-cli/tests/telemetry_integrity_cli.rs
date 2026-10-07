// SPDX-License-Identifier: Apache-2.0
use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), dest).unwrap();
        }
    }
}

fn telemetry_report(output: &std::process::Output) -> serde_json::Value {
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "telemetry must emit one JSON report: {error}; stdout={}",
                String::from_utf8_lossy(&output.stdout)
            )
        });
    assert_eq!(report["schema"], "oh.war/report/v1");
    assert_eq!(
        report["command"],
        if output.status.success() {
            "telemetry"
        } else {
            "error"
        }
    );
    if output.status.success() {
        assert_eq!(report["exit_code"], 0);
    } else {
        assert!(report["exit_code"].as_u64().unwrap() > 0);
    }
    report
}

#[test]
fn unobserved_eligibility_is_not_measured_zero() {
    let root = std::env::temp_dir().join(format!("ow-telemetry-integrity-{}", std::process::id()));
    copy_tree(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../conformance/fixtures/inbox/repository"),
        &root,
    );
    // An actual local history is required by the collector. No global Git config.
    for args in [
        vec!["init", "-q"],
        vec!["add", "."],
        vec![
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "fixture",
        ],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(&root)
                .status()
                .unwrap()
                .success()
        );
    }
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&root)
        .output()
        .unwrap();
    let head = String::from_utf8(head.stdout).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_war"))
        .args([
            "telemetry",
            "--commit",
            head.trim(),
            "--out",
            "measurement.json",
            "--json",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
    let report = telemetry_report(&result);
    assert_eq!(report["result"]["operation"], "record");
    let measurement: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("measurement.json")).unwrap()).unwrap();
    assert_eq!(report["result"]["baseline"], measurement);
    let eligibility = &measurement["measures"]["auto-authorizable fraction"];
    assert!(
        eligibility.get("value").is_none(),
        "unobserved eligibility cannot be measured zero"
    );
    assert!(
        eligibility["not_measurable_yet"]
            .as_str()
            .unwrap()
            .contains("does not evaluate")
    );
    for name in [
        "reopenings",
        "post-resolution escapes",
        "interview questions",
        "replay, repair, restart",
    ] {
        assert!(
            measurement["measures"][name]["not_measurable_yet"]
                .as_str()
                .unwrap()
                .starts_with("this collector does not")
        );
    }
    // Real observed zero remains a measurement, so not every metric is unknown.
    assert_eq!(measurement["measures"]["amendments"]["value"], 0);
    assert!(
        !measurement["untracked_work_candidates"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let verified = Command::new(env!("CARGO_BIN_EXE_war"))
        .args([
            "telemetry",
            "--commit",
            head.trim(),
            "--out",
            "measurement.json",
            "--verify",
            "--json",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        verified.status.success(),
        "{}",
        String::from_utf8_lossy(&verified.stdout)
    );
    let report = telemetry_report(&verified);
    assert_eq!(report["result"]["operation"], "verify");
    assert_eq!(report["result"]["unchanged"], true);
    let attached = Command::new(env!("CARGO_BIN_EXE_war"))
        .args([
            "telemetry",
            "--commit",
            head.trim(),
            "--attach",
            "fixture commit",
            "--warrant",
            "IX-WAR-0002",
            "--reviewer",
            "fixture-reviewer",
            "--json",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(attached.status.success());
    let report = telemetry_report(&attached);
    assert_eq!(report["result"]["operation"], "attach");
    assert_eq!(report["result"]["scope"], "fixture commit");
    assert_eq!(report["result"]["warrant"], "IX-WAR-0002");
    assert_eq!(report["result"]["reviewer"], "fixture-reviewer");
    let original = std::fs::read(root.join("measurement.json")).unwrap();
    let mut drifted = original.clone();
    drifted.push(b'\n');
    std::fs::write(root.join("measurement.json"), &drifted).unwrap();
    let refused = Command::new(env!("CARGO_BIN_EXE_war"))
        .args([
            "telemetry",
            "--commit",
            head.trim(),
            "--out",
            "measurement.json",
            "--verify",
            "--json",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(!refused.status.success());
    let report = telemetry_report(&refused);
    assert!(
        report["diagnostics"]
            .to_string()
            .contains("baseline differs")
    );
    assert_eq!(
        std::fs::read(root.join("measurement.json")).unwrap(),
        drifted
    );
    std::fs::write(root.join("measurement.json"), &original).unwrap();
    let refused = Command::new(env!("CARGO_BIN_EXE_war"))
        .args([
            "telemetry",
            "--commit",
            head.trim(),
            "--attach",
            "fixture commit",
            "--warrant",
            "IX-WAR-0002",
            "--json",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(!refused.status.success());
    let report = telemetry_report(&refused);
    assert!(report["diagnostics"].to_string().contains("review"));
    let human = Command::new(env!("CARGO_BIN_EXE_war"))
        .args([
            "telemetry",
            "--commit",
            head.trim(),
            "--attach",
            "fixture commit",
            "--warrant",
            "IX-WAR-0002",
            "--reviewer",
            "fixture-reviewer",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(human.status.success());
    assert_eq!(
        String::from_utf8(human.stdout).unwrap(),
        "fixture commit related to IX-WAR-0002 (reviewed by fixture-reviewer)\n"
    );
    let human = Command::new(env!("CARGO_BIN_EXE_war"))
        .args([
            "telemetry",
            "--commit",
            head.trim(),
            "--out",
            "measurement.json",
            "--verify",
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(human.status.success());
    assert!(
        String::from_utf8(human.stdout)
            .unwrap()
            .starts_with(&format!(
                "telemetry baseline at {} is unchanged (untracked work read from ",
                head.trim()
            ))
    );
    assert_eq!(
        std::fs::read(root.join("measurement.json")).unwrap(),
        original
    );
    std::fs::remove_dir_all(root).unwrap();
}
