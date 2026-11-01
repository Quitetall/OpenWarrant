// SPDX-License-Identifier: Apache-2.0
use serde_json::Value;
use std::{path::Path, process::Command};

fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
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
fn worktree_inventory_preserves_foreign_dirty_and_unique_work() {
    let base = tempfile::tempdir().unwrap();
    let root = base.path().join("repo");
    std::fs::create_dir(&root).unwrap();
    git(&root, &["init", "-q"]);
    assert_eq!(war(&root, &["init", "--namespace", "WT"])["exit_code"], 0);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "baseline"]);
    let foreign = base.path().join("foreign");
    git(
        &root,
        &[
            "worktree",
            "add",
            "-b",
            "foreign",
            foreign.to_str().unwrap(),
            "HEAD",
        ],
    );
    let v = war(&root, &["admin", "worktrees", "--prune"]);
    assert_eq!(v["exit_code"], 0, "{v}");
    let rows = v["result"]["worktrees"].as_array().unwrap();
    let row = rows
        .iter()
        .find(|r| r["path"] == foreign.to_str().unwrap())
        .unwrap();
    assert_eq!(row["managed"], false);
    assert_eq!(row["prunable"], false);
    assert!(foreign.exists());
    let managed = base.path().join("managed");
    git(
        &root,
        &[
            "worktree",
            "add",
            "-b",
            "managed",
            managed.to_str().unwrap(),
            "HEAD",
        ],
    );
    openwarrant_cli::worktrees::register(
        camino::Utf8Path::from_path(&root).unwrap(),
        camino::Utf8Path::from_path(&managed).unwrap(),
        "managed",
        "HEAD",
    )
    .unwrap();
    let v = war(&root, &["admin", "worktrees", "--prune"]);
    let row = v["result"]["worktrees"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == managed.to_str().unwrap())
        .unwrap();
    // The host may forbid inspecting another same-user process (for example
    // a non-dumpable signing agent). That is an honest UNKNOWN, not inactivity.
    let activity_unknown = row["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r == "process activity unknown");
    assert_eq!(row["prunable"], !activity_unknown, "{row}");
    assert!(
        !row["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r == "dirty or unreadable worktree"),
        "ownership metadata is outside the worktree: {row}"
    );
    assert!(managed.exists(), "dry run removes nothing");
    std::fs::write(managed.join("authored.md"), "preserve this work").unwrap();
    let v = war(&root, &["admin", "worktrees", "--prune", "--apply"]);
    assert_eq!(v["exit_code"], 0, "{v}");
    assert!(managed.join("authored.md").exists());
    assert!(foreign.exists());
    std::fs::remove_file(managed.join("authored.md")).unwrap();
    let v = war(&root, &["admin", "worktrees", "--prune", "--apply"]);
    assert_eq!(v["exit_code"], 0, "{v}");
    let row = v["result"]["worktrees"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == managed.to_str().unwrap())
        .unwrap();
    if row["prunable"] == true {
        assert!(
            !managed.exists(),
            "eligible managed worktree should be removed: {v}"
        );
    } else {
        assert!(
            managed.exists(),
            "unknown activity must preserve the workspace: {v}"
        );
        assert!(
            row["reasons"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r == "process activity unknown"),
            "{row}"
        );
    }
    assert!(foreign.exists());
}

#[test]
fn execution_never_discards_an_existing_workspace_on_retry() {
    let base = tempfile::tempdir().unwrap();
    let root = base.path().join("repo");
    std::fs::create_dir(&root).unwrap();
    git(&root, &["init", "-q"]);
    std::fs::write(root.join("README.md"), "baseline").unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "baseline"]);
    let work = base.path().join("work");
    git(
        &root,
        &[
            "worktree",
            "add",
            "-b",
            "existing",
            work.to_str().unwrap(),
            "HEAD",
        ],
    );
    std::fs::write(work.join("progress.txt"), "irreplaceable progress").unwrap();
    let root = camino::Utf8Path::from_path(&root).unwrap();
    let work = camino::Utf8Path::from_path(&work).unwrap();
    assert!(openwarrant_cli::go::git::add_worktree(root, work, "existing", "HEAD").is_err());
    assert_eq!(
        std::fs::read_to_string(work.join("progress.txt")).unwrap(),
        "irreplaceable progress"
    );
    openwarrant_cli::go::git::remove_worktree(root, work, Some("existing"));
    assert!(
        work.join("progress.txt").exists(),
        "foreign work is retained"
    );
}

#[test]
fn native_worktree_start_delivers_context_and_preserves_its_session() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    git(root, &["init", "-q"]);
    assert_eq!(war(root, &["init", "--namespace", "NS"])["exit_code"], 0);
    let created = war(
        root,
        &["create", "Native isolated work", "--item", "Deliver"],
    );
    let id = created["result"]["id"].as_str().unwrap();
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "baseline"]);
    let args = [
        "start",
        id,
        "--worktree",
        "--print",
        "--as",
        "isolated-builder",
    ];
    let started = war(root, &args);
    assert_eq!(started["exit_code"], 0, "{started}");
    assert_eq!(
        started["result"]["context"]["schema"],
        "oh.war/task-context/v1"
    );
    let worktree = started["result"]["worktree"].as_str().unwrap();
    let nonce = started["result"]["claim_id"]
        .as_str()
        .expect("managed claim nonce");
    let inventory = war(root, &["admin", "worktrees"]);
    assert!(
        inventory["result"]["worktrees"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["path"] == worktree && r["managed"] == true),
        "{inventory}"
    );
    let settings: Value = serde_json::from_slice(
        &std::fs::read(Path::new(worktree).join(".claude/settings.local.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(settings["env"]["OPENWARRANT_CLAIM_ID"], nonce);
    let resumed = war(root, &args);
    assert_eq!(resumed["exit_code"], 0, "{resumed}");
    assert_eq!(resumed["result"]["resumed"], true);
    assert_eq!(resumed["result"]["claim_id"], nonce);
}
