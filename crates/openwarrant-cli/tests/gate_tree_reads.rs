// SPDX-License-Identifier: Apache-2.0
use openwarrant_cli::{gate_cmd::source, repo::Repository};
use std::{
    fs,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const TEST: &str =
    "gate_tree_reads::historical_comparisons_share_a_clean_tree_scan_without_hiding_dirty_changes";

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .env_remove("GIT_TRACE2_EVENT")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn shared_indexes(root: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    fs::read_dir(root.join(".git"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("sharedindex.")
        })
        .map(|path| {
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect()
}

#[test]
fn historical_comparisons_share_a_clean_tree_scan_without_hiding_dirty_changes() {
    let Ok(mode) = std::env::var("OW_TREE_READ_MODE") else {
        let scratch = std::env::temp_dir().join(format!(
            "ow-history-scans-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&scratch).unwrap();
        for mode in [
            "hosted", "recorded", "renamed", "clean", "dirty", "off", "split", "corrupt",
        ] {
            let root = scratch.join(mode);
            fs::create_dir(&root).unwrap();
            let output = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", TEST, "--nocapture"])
                .env("TMPDIR", &scratch)
                .env("OW_TREE_READ_MODE", mode)
                .env("OW_TREE_READ_ROOT", &root)
                .env("GIT_TRACE2_EVENT", scratch.join(format!("{mode}.jsonl")))
                .output()
                .unwrap();
            if !output.status.success() {
                fs::remove_dir_all(&scratch).unwrap();
                panic!(
                    "{mode}: {} {}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            assert!(
                !fs::read_dir(&scratch).unwrap().any(|entry| entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("ow-git-read-")),
                "private Git index was not removed"
            );
        }
        fs::remove_dir_all(scratch).unwrap();
        return;
    };
    let root = std::path::PathBuf::from(std::env::var_os("OW_TREE_READ_ROOT").unwrap());
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../conformance/fixtures/inbox/repository/openwarrant.toml"),
        root.join("openwarrant.toml"),
    )
    .unwrap();
    git(
        &root,
        &["init", "-q", "--template=", "--initial-branch=fixture"],
    );
    let commit = || {
        git(&root, &["add", "."]);
        git(
            &root,
            &[
                "-c",
                "user.name=fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "core.hooksPath=/dev/null",
                "commit",
                "--no-gpg-sign",
                "-qm",
                "synthetic source observation",
            ],
        );
        git(&root, &["rev-parse", "HEAD^{tree}"]).trim().to_owned()
    };
    fs::write(root.join("a"), "one").unwrap();
    let first = commit();
    fs::write(root.join("a"), "two").unwrap();
    let second = commit();
    fs::write(root.join("b"), "one").unwrap();
    let third = commit();
    if mode == "renamed" {
        git(&root, &["config", "diff.renames", "true"]);
        fs::create_dir_all(root.join("docs/warrants/IX-WAR-0003")).unwrap();
        git(
            &root,
            &["mv", "a", "docs/warrants/IX-WAR-0003/journal.jsonl"],
        );
        commit();
    }
    if mode == "split" {
        git(&root, &["update-index", "--split-index"]);
    }
    if matches!(mode.as_str(), "dirty" | "split") {
        fs::write(root.join("a"), "one").unwrap();
    }
    if mode == "corrupt" {
        fs::write(root.join(".git/index"), "invalid index").unwrap();
    }
    fs::write(root.join("untracked"), "also bound").unwrap();
    let index = fs::read(root.join(".git/index")).unwrap();
    let shared = shared_indexes(&root);
    let repo = Repository::open(camino::Utf8PathBuf::from_path_buf(root.clone()).unwrap()).unwrap();
    let ex = source::Exclusions::of(&repo);
    if mode != "off" {
        source::remember_tree_reads();
    }
    if mode == "hosted" {
        for name in ["first-frame", "second-frame"] {
            let mut frame = openwarrant_cli::vfs::Tree::new(
                repo.root.clone(),
                Default::default(),
                Default::default(),
            );
            frame.git.insert(
                vec![
                    "diff".to_owned(),
                    "--no-renames".to_owned(),
                    "--name-only".to_owned(),
                    "-z".to_owned(),
                    first.clone(),
                    "--".to_owned(),
                ],
                Ok(format!("{name}\0").into_bytes()),
            );
            frame.git.insert(
                ["ls-files", "--others", "--exclude-standard", "-z"]
                    .map(str::to_owned)
                    .to_vec(),
                Ok(Vec::new()),
            );
            let moved = openwarrant_cli::vfs::hosted(std::rc::Rc::new(frame), || {
                source::moved_since(&repo.root, &first, &ex).unwrap()
            });
            assert_eq!(
                moved,
                vec![name],
                "one hosted basis must not reuse another basis's observations"
            );
        }
        return;
    }
    if mode == "recorded" {
        for _ in 0..2 {
            let recorder =
                std::rc::Rc::new(std::cell::RefCell::new(openwarrant_cli::vfs::Recorder {
                    root: repo.root.clone(),
                    ..Default::default()
                }));
            openwarrant_cli::vfs::recording(std::rc::Rc::clone(&recorder), || {
                source::moved_since(&repo.root, &first, &ex).unwrap()
            });
            assert!(
                !recorder.borrow().git.is_empty(),
                "every recording must carry its own Git observations"
            );
        }
        return;
    }
    if mode == "corrupt" {
        assert!(
            source::moved_since(&repo.root, &first, &ex).is_err(),
            "an unreadable index cannot establish unchanged source"
        );
        assert!(fs::read(root.join(".git/index")).unwrap() == index);
        return;
    }
    if mode == "renamed" {
        assert_eq!(
            source::moved_since(&repo.root, &third, &ex).unwrap(),
            vec!["a", "untracked"],
            "moving source into excluded bookkeeping must still name the deleted source"
        );
        assert!(
            fs::read(root.join(".git/index")).unwrap() == index,
            "source observation changed index bytes"
        );
        return;
    }
    for (tree, expected) in [
        (
            &first,
            if matches!(mode.as_str(), "dirty" | "split") {
                vec!["b", "untracked"]
            } else {
                vec!["a", "b", "untracked"]
            },
        ),
        (
            &second,
            if matches!(mode.as_str(), "dirty" | "split") {
                vec!["a", "b", "untracked"]
            } else {
                vec!["b", "untracked"]
            },
        ),
    ] {
        assert_eq!(
            source::moved_since(&repo.root, tree, &ex).unwrap(),
            expected
        );
    }
    assert!(
        fs::read(root.join(".git/index")).unwrap() == index,
        "source observation changed index bytes"
    );
    assert_eq!(
        shared_indexes(&root),
        shared,
        "source observation changed shared-index bytes"
    );
    if mode == "dirty" {
        // A dirty tree is scanned whole once; each historical tree is compared
        // with the tip, and only paths changed on both sides are compared with
        // the working tree, by name.
        let trace = fs::read_to_string(std::env::var_os("GIT_TRACE2_EVENT").unwrap()).unwrap();
        let whole = trace
            .lines()
            .filter(|line| {
                let event: serde_json::Value = serde_json::from_str(line).unwrap();
                if event["event"] != "start" {
                    return false;
                }
                let args: Vec<&str> = event["argv"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .collect();
                let (Some(z), Some(dashes)) = (
                    args.iter().position(|a| *a == "-z"),
                    args.iter().position(|a| *a == "--"),
                ) else {
                    return false;
                };
                args.contains(&"diff") && dashes == z + 2 && dashes + 1 == args.len()
            })
            .count();
        assert_eq!(
            whole, 1,
            "two historical subjects must not each scan the same dirty working tree"
        );
    }
    if mode == "clean" {
        let trace = fs::read_to_string(std::env::var_os("GIT_TRACE2_EVENT").unwrap()).unwrap();
        let walks = trace
            .lines()
            .filter(|line| {
                let event: serde_json::Value = serde_json::from_str(line).unwrap();
                if event["event"] != "start" {
                    return false;
                }
                let args = event["argv"].as_array().unwrap();
                let Some(start) = args.iter().position(|arg| arg == "-z") else {
                    return false;
                };
                args.iter().any(|arg| arg == "diff")
                    && args[start + 1..]
                        .iter()
                        .take_while(|arg| **arg != "--")
                        .count()
                        == 1
            })
            .count();
        assert_eq!(
            walks, 1,
            "two historical subjects must not each scan the same clean working tree"
        );
    }
    if mode == "off" {
        fs::write(root.join("a"), "one").unwrap();
        assert_eq!(
            source::moved_since(&repo.root, &first, &ex).unwrap(),
            vec!["b", "untracked"]
        );
        assert!(
            fs::read(root.join(".git/index")).unwrap() == index,
            "uncached source observation changed index bytes"
        );
    }
}
