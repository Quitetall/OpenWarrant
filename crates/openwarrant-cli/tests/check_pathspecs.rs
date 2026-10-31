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

/// A committed `war init` scaffold with a held-out tree beside it.
fn scaffold(tag: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ow-pathspec-{tag}-{}-{}",
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
    root
}

#[test]
fn a_check_with_no_work_journals_never_diffs_the_whole_tree() {
    let root = scaffold("tree");
    for args in [
        &["check"][..],
        &["check", "--generated"][..],
        &["check", "--changed"][..],
    ] {
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

fn check_changed(root: &Path) -> (i32, String) {
    check_changed_with(root, &[])
}

fn check_changed_with(root: &Path, extra: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        .env("OPENWARRANT_NO_CACHE", "1")
        .env_remove("OPENWARRANT_VERBOSE")
        .args(["check", "--changed"])
        .args(extra)
        .current_dir(root)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

fn war_in(root: &Path, args: &[&str]) {
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "war {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `war check --changed` (the end-of-turn hook's check) checks what the
/// working tree changed and nothing else: nothing governed changed is
/// nothing checked; a Warrant's record is that Warrant; a file every
/// Warrant reads is the corpus; an unrelated file is nothing.
#[test]
fn check_changed_checks_only_what_changed() {
    let root = scaffold("changed");
    let (code, out) = check_changed(&root);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("nothing OpenWarrant governs differs from HEAD"),
        "{out}"
    );
    assert!(
        !out.contains("--verbose"),
        "a run that folded nothing offers nothing: {out}"
    );

    std::fs::write(
        root.join("heldout/sample.txt"),
        "moved, and not war's business\n",
    )
    .unwrap();
    let (code, out) = check_changed(&root);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("nothing OpenWarrant governs differs from HEAD"),
        "{out}"
    );

    let manifest = root.join("docs/warrants/HO-WAR-0001/manifest.toml");
    let before = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, format!("{before}\n# touched\n")).unwrap();
    let (_, out) = check_changed(&root);
    assert!(out.contains("checked what changed: HO-WAR-0001"), "{out}");
    std::fs::write(&manifest, before).unwrap();

    let config = root.join("openwarrant.toml");
    let before = std::fs::read_to_string(&config).unwrap();
    std::fs::write(&config, format!("{before}\n# touched\n")).unwrap();
    let (_, out) = check_changed(&root);
    assert!(
        out.contains("checked the corpus: openwarrant.toml changed"),
        "{out}"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// What `--changed` must not miss: work committed since the session began
/// (`--since`), a Warrant that cites a changed parent, a ticket and a
/// document; and a `--since` that names no commit is refused, never read as
/// "nothing changed".
#[test]
fn check_changed_reaches_commits_dependents_tickets_and_documents() {
    let root = scaffold("reach");
    let head = |root: &Path| {
        let out = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(root)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    };
    let base = head(&root);
    let manifest = root.join("docs/warrants/HO-WAR-0001/manifest.toml");
    let parent = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, format!("{parent}\n# committed in the turn\n")).unwrap();
    git(&root, &["commit", "-qam", "turn"]);
    let (_, out) = check_changed(&root);
    assert!(
        out.contains("nothing OpenWarrant governs differs from HEAD"),
        "{out}"
    );
    let (_, out) = check_changed_with(&root, &["--since", &base]);
    assert!(out.contains("checked what changed: HO-WAR-0001"), "{out}");
    let (code, out) = check_changed_with(&root, &["--since", "no-such-rev"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("names no commit"), "{out}");

    // A child citing HO-WAR-0001 is checked when its parent changes.
    war_in(&root, &["new", "Child work"]);
    let uuid = parent
        .lines()
        .find_map(|l| l.strip_prefix("uuid = \""))
        .and_then(|l| l.strip_suffix('"'))
        .expect("the scaffold's Warrant has a uuid")
        .to_owned();
    let child = root.join("docs/warrants/HO-WAR-0002/manifest.toml");
    let text = std::fs::read_to_string(&child).unwrap();
    std::fs::write(
        &child,
        format!("{text}\n[[parents]]\nref = \"war://{uuid}\"\ncontract_revision = 1\n"),
    )
    .unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "child"]);
    std::fs::write(&manifest, format!("{parent}\n# again\n")).unwrap();
    let (_, out) = check_changed(&root);
    assert!(
        out.contains("checked what changed: HO-WAR-0001; HO-WAR-0002 (they cite a changed parent)"),
        "{out}"
    );
    git(
        &root,
        &[
            "checkout",
            "-q",
            "--",
            "docs/warrants/HO-WAR-0001/manifest.toml",
        ],
    );

    // A ticket is the tickets, and not a document; a document is the documents.
    war_in(&root, &["create", "A ticket"]);
    let (_, out) = check_changed(&root);
    assert!(out.contains("checked what changed: the tickets"), "{out}");
    assert!(!out.contains("the documents"), "{out}");
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "ticket"]);
    std::fs::write(root.join("docs/notes.md"), "# Notes\n").unwrap();
    let (_, out) = check_changed(&root);
    assert!(out.contains("checked what changed: the documents"), "{out}");
    std::fs::remove_dir_all(&root).unwrap();
}
