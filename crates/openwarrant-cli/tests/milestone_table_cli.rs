// SPDX-License-Identifier: Apache-2.0
use serde_json::Value;
use std::{path::Path, process::Command};

fn war(root: &Path, args: &[&str]) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["--root", root.to_str().unwrap(), "--json"])
        .args(args)
        .current_dir(root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap_or_else(|_| {
        panic!(
            "{} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

#[test]
fn milestone_import_preserves_originals_ids_and_local_edits() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    assert_eq!(war(root, &["init", "--namespace", "MT"])["exit_code"], 0);
    let original = "# Roadmap\n\n## Queue\n\n| Phase | Package | Outcome | Engineering state |\n| --- | --- | --- | --- |\n| P1 | WP-1 | Parse documents | Planned |\n| P2 | WP-2 | Export packets | Done |\n";
    std::fs::write(root.join("roadmap.md"), original).unwrap();
    let first = war(
        root,
        &[
            "admin",
            "import",
            "table",
            "roadmap.md",
            "--section",
            "Queue",
        ],
    );
    assert_eq!(first["exit_code"], 0, "{first}");
    let created = first["result"]["created"].as_array().unwrap();
    assert_eq!(created.len(), 2, "{first}");
    let id = created[0]["id"].as_str().unwrap();
    let source = root.join(format!("docs/tickets/{id}.md"));
    assert!(
        source.is_file(),
        "new imported work uses one Markdown source"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("roadmap.md")).unwrap(),
        original
    );
    let archive = first["result"]["source_snapshot"].as_str().unwrap();
    assert_eq!(
        std::fs::read_to_string(root.join(archive)).unwrap(),
        original
    );
    let again = war(
        root,
        &[
            "admin",
            "import",
            "table",
            "roadmap.md",
            "--section",
            "Queue",
        ],
    );
    assert_eq!(again["exit_code"], 0, "{again}");
    assert!(again["result"]["created"].as_array().unwrap().is_empty());
    let before = std::fs::read_to_string(&source).unwrap();
    std::fs::write(
        &source,
        format!("{before}\nHuman implementation notes remain here.\n"),
    )
    .unwrap();
    std::fs::write(
        root.join("roadmap.md"),
        original.replace("Parse documents", "Parse documents and imports"),
    )
    .unwrap();
    let changed = war(
        root,
        &[
            "admin",
            "import",
            "table",
            "roadmap.md",
            "--section",
            "Queue",
        ],
    );
    assert_eq!(changed["exit_code"], 0, "{changed}");
    let proposed = changed["result"]["proposals"].as_array().unwrap();
    assert_eq!(proposed.len(), 1, "{changed}");
    assert_eq!(proposed[0]["id"], id);
    assert!(
        proposed[0]["before"]
            .as_str()
            .unwrap()
            .contains("Parse documents")
    );
    assert!(
        proposed[0]["after"]
            .as_str()
            .unwrap()
            .contains("Parse documents and imports")
    );
    assert_eq!(
        std::fs::read_to_string(&source).unwrap(),
        format!("{before}\nHuman implementation notes remain here.\n")
    );
}

#[test]
fn milestone_dependencies_are_explicit_and_bad_mappings_write_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    assert_eq!(war(root, &["init", "--namespace", "MD"])["exit_code"], 0);
    let table = "# Queue\n\n| ID | Title | Status | Depends on |\n| --- | --- | --- | --- |\n| A | Parser | Planned | |\n| B | SDK | Planned | A |\n";
    std::fs::write(
        root.join("queue.md"),
        table.replace("| A |\n", "| MISSING |\n"),
    )
    .unwrap();
    let bad = war(root, &["admin", "import", "table", "queue.md"]);
    assert_ne!(
        bad["exit_code"], 0,
        "unknown dependencies must refuse before writes: {bad}"
    );
    assert!(!root.join("docs/imports/table").exists());
    assert!(
        !root.join("docs/tickets").exists()
            || !root.join("docs/tickets").read_dir().unwrap().any(|p| p
                .unwrap()
                .path()
                .extension()
                .is_some_and(|e| e == "md"))
    );
    std::fs::write(
        root.join("queue.md"),
        table.replace("| B | SDK", "| A | SDK"),
    )
    .unwrap();
    let duplicate = war(root, &["admin", "import", "table", "queue.md"]);
    assert_ne!(duplicate["exit_code"], 0, "{duplicate}");
    assert!(!root.join("docs/imports/table").exists());
    std::fs::write(root.join("queue.md"), table).unwrap();
    let imported = war(root, &["admin", "import", "table", "queue.md"]);
    assert_eq!(imported["exit_code"], 0, "{imported}");
    let ids = imported["result"]["created"].as_array().unwrap();
    let first = ids[0]["id"].as_str().unwrap();
    let second = ids[1]["id"].as_str().unwrap();
    let blocked = war(root, &["start", second]);
    assert_ne!(blocked["exit_code"], 0, "{blocked}");
    assert_eq!(war(root, &["start", first])["exit_code"], 0);
    assert_eq!(war(root, &["done", first])["exit_code"], 0);
    assert_eq!(war(root, &["start", second])["exit_code"], 0);
}
