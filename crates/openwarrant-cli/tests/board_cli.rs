// SPDX-License-Identifier: Apache-2.0
use std::{collections::BTreeMap, fs, path::Path, process::Command};
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
fn files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().display().to_string(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
#[test]
fn board_is_complete_read_only_and_refuses_corrupt_questions() {
    let root = std::env::temp_dir().join(format!("ow-board-{}", std::process::id()));
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/fixtures/inbox/repository"),
        &root,
    );
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_war"))
            .args(args)
            .current_dir(&root)
            .output()
            .unwrap()
    };
    let before = files(&root);
    let board = run(&["board", "--json"]);
    assert!(
        board.status.success(),
        "{}",
        String::from_utf8_lossy(&board.stderr)
    );
    let board: serde_json::Value = serde_json::from_slice(&board.stdout).unwrap();
    let console: serde_json::Value =
        serde_json::from_slice(&run(&["console", "--json"]).stdout).unwrap();
    let frontier: serde_json::Value =
        serde_json::from_slice(&run(&["frontier", "--json"]).stdout).unwrap();
    assert_eq!(board["result"]["approvals"], console["result"]["acts"]);
    assert_eq!(board["result"]["frontier"], frontier["result"]);
    let status: serde_json::Value =
        serde_json::from_slice(&run(&["status", "--json"]).stdout).unwrap();
    assert_eq!(board["result"]["corpus"], status["result"]);
    assert!(
        !board["result"]["corpus"]["warrants"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!board["result"]["questions"].as_array().unwrap().is_empty());
    let signing = run(&["sign", "--list"]);
    assert!(signing.status.success());
    let signing = String::from_utf8(signing.stdout).unwrap();
    for act in board["result"]["approvals"].as_array().unwrap() {
        assert!(
            signing
                .lines()
                .any(|line| line.trim() == act["line"].as_str().unwrap())
        );
    }
    let html = run(&["board", "--html"]);
    assert!(html.status.success());
    let html = String::from_utf8(html.stdout).unwrap();
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.trim_end().ends_with("</html>"));
    for prohibited in ["<script", "<form", "<button", " src=", " href="] {
        assert!(!html.contains(prohibited));
    }
    assert!(html.contains("default-src 'none'"));
    assert_eq!(before, files(&root), "board must not mutate source records");
    fs::write(
        root.join("docs/warrants/IX-WAR-0002/questions/Q-002.toml"),
        "bad TOML",
    )
    .unwrap();
    assert!(!run(&["board", "--json"]).status.success());
    assert!(!run(&["board", "--html"]).status.success());
    fs::remove_file(root.join("docs/warrants/IX-WAR-0002/questions/Q-002.toml")).unwrap();
    fs::write(
        root.join("docs/warrants/IX-WAR-0002/journal.jsonl"),
        "broken journal",
    )
    .unwrap();
    assert!(
        !run(&["board", "--json"]).status.success(),
        "damaged stage history must not become an empty frontier"
    );
    fs::remove_dir_all(root).unwrap();
}
