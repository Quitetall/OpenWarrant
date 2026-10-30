// SPDX-License-Identifier: Apache-2.0
//! `war check` names what it reads (issue #365).
//!
//! A `git diff HEAD --` with nothing after the `--` is not "diff nothing": git
//! reads it as the whole tree, refreshes the index and stats, and on a racily
//! clean entry re-reads, every tracked file. A repository with paths no tool
//! may read (a locked held-out corpus) cannot run a checker that does that.
//! Every git command a check spawns is taken from git's own trace, so a new
//! call site with the same shape is caught without being named here.
use std::{path::Path, process::Command};

fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .env_remove("GIT_TRACE2_EVENT")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Every git argv `war <args>` spawned in `root`, from GIT_TRACE2_EVENT.
fn git_argvs(root: &Path, args: &[&str]) -> Vec<Vec<String>> {
    let trace = root.with_extension("trace2.jsonl");
    let _ = std::fs::remove_file(&trace);
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        .env("OPENWARRANT_NO_CACHE", "1")
        .env("GIT_TRACE2_EVENT", &trace)
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.code().is_some(),
        "war {args:?} did not exit: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = std::fs::read_to_string(&trace).unwrap_or_default();
    text.lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|v| v["event"] == "start")
        .filter_map(|v| {
            v["argv"].as_array().map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(str::to_owned))
                    .collect()
            })
        })
        .collect()
}

#[test]
fn a_check_with_no_work_journals_never_diffs_the_whole_tree() {
    let root = std::env::temp_dir().join(format!(
        "ow-pathspec-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    git(&root, &["init", "-q", "."]);
    git(&root, &["config", "user.email", "t@example.invalid"]);
    git(&root, &["config", "user.name", "t"]);
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["init", "--program", "Held Out", "--namespace", "HO"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "init: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::create_dir_all(root.join("heldout")).unwrap();
    std::fs::write(root.join("heldout/sample.txt"), "not war's to read\n").unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "fixture"]);

    for args in [&["check"][..], &["check", "--generated"][..]] {
        let argvs = git_argvs(&root, args);
        assert!(
            !argvs.is_empty(),
            "war {args:?}: no git trace was recorded, so this test proved nothing"
        );
        for argv in &argvs {
            assert_ne!(
                argv.last().map(String::as_str),
                Some("--"),
                "war {args:?} ran git with an empty pathspec, which reads the whole tree: {argv:?}"
            );
            if argv.iter().any(|a| a == "diff") {
                let named = argv
                    .iter()
                    .position(|a| a == "--")
                    .is_some_and(|i| i + 1 < argv.len());
                assert!(
                    named,
                    "war {args:?} ran a git diff over the whole tree: {argv:?}"
                );
            }
        }
    }
    std::fs::remove_dir_all(&root).unwrap();
    let _ = std::fs::remove_file(root.with_extension("trace2.jsonl"));
}
