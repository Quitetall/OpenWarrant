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
    let measurement: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("measurement.json")).unwrap()).unwrap();
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
    std::fs::remove_dir_all(root).unwrap();
}
