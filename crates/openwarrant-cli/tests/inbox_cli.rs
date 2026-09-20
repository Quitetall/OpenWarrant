// SPDX-License-Identifier: Apache-2.0
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
fn war(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn empty_inbox_is_success_and_readable() {
    let root = std::env::temp_dir().join(format!("ow70-empty-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    assert!(
        war(&root, &["init", "--namespace", "TEST"])
            .status
            .success()
    );
    let result = war(&root, &["inbox"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("Nothing waiting"));
    let json = war(&root, &["inbox", "--json"]);
    let v: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(v["result"]["api_version"], "oh.war/inbox/v1");
    assert_eq!(v["result"]["items"], serde_json::json!([]));
    fs::remove_dir_all(root).unwrap();
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).unwrap();
        }
    }
}
fn fixture(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("ow70-{name}-{}", std::process::id()));
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/fixtures/inbox/repository"),
        &root,
    );
    root
}
fn snapshot(root: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut out = std::collections::BTreeMap::new();
    for entry in fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            out.extend(snapshot(&entry.path()));
        } else {
            out.insert(entry.path(), fs::read(entry.path()).unwrap());
        }
    }
    out
}
#[test]
fn fixtures_list_exact_human_rows_and_never_write() {
    let root = fixture("rows");
    let before = snapshot(&root);
    let output = war(&root, &["inbox", "--json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let items = v["result"]["items"].as_array().unwrap();
    assert_eq!(
        items
            .iter()
            .map(|i| i["alias"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["IX-WAR-0001", "IX-WAR-0002"]
    );
    assert_eq!(items[0]["awaited_act"], "authorize");
    assert_eq!(items[1]["awaited_act"], "answer");
    assert_eq!(items[1]["state"], "authorized");
    assert_eq!(items[0]["waiting_since"], "2020-01-01T01:00:00+01:00");
    assert_eq!(
        items[0]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["alias", "title", "state", "awaited_act", "waiting_since"]
    );
    let table = war(&root, &["inbox"]);
    let table = String::from_utf8(table.stdout).unwrap();
    assert!(table.contains("IX-WAR-0001\t"));
    assert!(table.contains("IX-WAR-0002\t"));
    assert!(!table.contains("IX-WAR-0003"));
    assert!(!table.contains("IX-WAR-0004"));
    assert_eq!(before, snapshot(&root));
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn unknown_times_sort_last_and_other_events_do_not_reset_age() {
    let root = fixture("times");
    let journal = root.join("docs/warrants/IX-WAR-0001/journal.jsonl");
    let original = fs::read_to_string(&journal).unwrap();
    let mut event: serde_json::Value = serde_json::from_str(&original).unwrap();
    event["id"] = "01a0ac9b-3953-7511-86f6-7fb1f92a1526".into();
    event["idempotency_key"] = "other-event".into();
    event["type"] = "dispatch.compiled".into();
    event["occurred_at"] = "2025-01-01T00:00:00Z".into();
    fs::write(
        &journal,
        format!("{original}{}\n", serde_json::to_string(&event).unwrap()),
    )
    .unwrap();
    let output = war(&root, &["inbox", "--json"]);
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        v["result"]["items"][0]["waiting_since"],
        "2020-01-01T01:00:00+01:00"
    );
    for time in ["unknown", "9999-01-01T00:00:00Z", "猫猫猫猫"] {
        fs::write(
            &journal,
            original.replace("2020-01-01T01:00:00+01:00", time),
        )
        .unwrap();
        let output = war(&root, &["inbox", "--json"]);
        assert!(output.status.success());
        let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(v["result"]["items"][0]["alias"], "IX-WAR-0002");
        assert_eq!(
            v["result"]["items"][1]["waiting_since"],
            serde_json::Value::Null
        );
        assert!(String::from_utf8_lossy(&war(&root, &["inbox"]).stdout).contains("unknown"));
    }
    fs::remove_file(journal).unwrap();
    assert!(war(&root, &["inbox"]).status.success());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn damaged_records_fail_instead_of_claiming_empty_inbox() {
    for path in [
        "manifest.toml",
        "authorization.toml",
        "journal.jsonl",
        "questions/Q-001.toml",
        "atoms/10-intent.md",
        "verifications/broken.toml",
        "corrections/broken.toml",
        "deliverables.toml",
    ] {
        let root = fixture(&format!("bad-{}", path.replace('/', "-")));
        fs::create_dir_all(
            root.join("docs/warrants/IX-WAR-0002")
                .join(path)
                .parent()
                .unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("docs/warrants/IX-WAR-0002").join(path),
            "broken {",
        )
        .unwrap();
        let output = war(&root, &["inbox", "--json"]);
        assert!(!output.status.success(), "{path}");
        let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_ne!(v["exit_code"], 0);
        assert!(v.get("result").is_none_or(serde_json::Value::is_null));
        fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn question_from_another_warrant_is_rejected() {
    let root = fixture("wrong-question");
    let path = root.join("docs/warrants/IX-WAR-0002/questions/Q-001.toml");
    fs::write(
        &path,
        fs::read_to_string(&path)
            .unwrap()
            .replace("IX-WAR-0002", "IX-WAR-9999"),
    )
    .unwrap();
    let output = war(&root, &["inbox"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("another Warrant"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_question_directory_replaced_by_a_file_is_not_an_empty_inbox() {
    let root = fixture("question-directory");
    let path = root.join("docs/warrants/IX-WAR-0002/questions");
    fs::remove_dir_all(&path).unwrap();
    fs::write(&path, "not a directory").unwrap();
    assert!(!war(&root, &["inbox"]).status.success());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn damaged_repository_containers_refuse() {
    for name in ["warrants", "authority"] {
        let root = fixture(&format!("root-{name}"));
        if name == "warrants" {
            let path = root.join("docs/warrants");
            fs::remove_dir_all(&path).unwrap();
            fs::write(path, "not a directory").unwrap();
        } else {
            // The fixture now ships a real register (the signed responses need
            // one), so replace the file with a directory rather than assuming
            // the path is free.
            let path = root.join("docs/authority/roles.toml");
            fs::remove_file(&path).unwrap();
            fs::create_dir_all(&path).unwrap();
        }
        assert!(!war(&root, &["inbox"]).status.success(), "{name}");
        fs::remove_dir_all(root).unwrap();
    }
}
