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
fn damaged_journal_never_becomes_open_work() {
    let root = std::env::temp_dir().join(format!("ow-frontier-integrity-{}", std::process::id()));
    copy_tree(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../conformance/fixtures/inbox/repository"),
        &root,
    );
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_war"))
            .args(["frontier", "IX-WAR-0001", "--json"])
            .current_dir(&root)
            .output()
            .unwrap()
    };
    let good = run();
    assert!(
        good.status.success(),
        "{}",
        String::from_utf8_lossy(&good.stdout)
    );
    let good: serde_json::Value = serde_json::from_slice(&good.stdout).unwrap();
    assert!(good["result"]["open"].as_u64().unwrap() > 0);
    let journal = root.join("docs/warrants/IX-WAR-0001/journal.jsonl");
    std::fs::write(&journal, "not json\n").unwrap();
    let bad = run();
    assert!(!bad.status.success());
    let bad: serde_json::Value = serde_json::from_slice(&bad.stdout).unwrap();
    assert!(bad["result"].is_null());
    assert!(bad.to_string().contains("journal.jsonl"));
    std::fs::remove_file(&journal).unwrap();
    std::fs::create_dir(&journal).unwrap();
    let bad = run();
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stdout).contains("journal.jsonl"));
    std::fs::remove_dir(&journal).unwrap();
    assert!(
        run().status.success(),
        "Genuinely absent journal remains valid"
    );
    let milestones = root.join("docs/warrants/IX-WAR-0001/atoms/45-milestones.yaml");
    std::fs::write(&milestones, "not: [valid YAML").unwrap();
    let bad = run();
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stdout).contains("milestones"));
    std::fs::remove_dir_all(root).unwrap();
}
