// SPDX-License-Identifier: Apache-2.0
//! The ticket cache (`ticket::cache`) through the shipped binary: it answers
//! exactly what the files answer — cold, warm and disabled give the same
//! bytes — and it is refused wherever it could be wrong: an in-place edit
//! with its mtime put back, a corrupt or truncated file, a file another
//! build wrote, and every command that never permits it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const TRACE: &str = "OPENWARRANT_CACHE_TRACE";

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ow-ticket-cache-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q", "."])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );
    let out = run(
        &root,
        &[
            "init",
            "--namespace",
            "TC",
            "--root",
            root.to_str().unwrap(),
        ],
        &[],
    );
    assert!(out.status.success(), "{}", text(&out.stderr));
    root
}

fn run_with(bin: &Path, root: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(bin);
    cmd.current_dir(root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .env("OPENWARRANT_ACTOR", "cache-test")
        .env_remove("SSH_AUTH_SOCK")
        .env_remove("OPENWARRANT_NO_CACHE")
        .env_remove(TRACE)
        .arg("--root")
        .arg(root)
        .args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().unwrap()
}

fn run(root: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    run_with(Path::new(env!("CARGO_BIN_EXE_war")), root, args, env)
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// The trace line of one run, or "" when the cache was not used.
fn trace(out: &Output) -> String {
    text(&out.stderr)
        .lines()
        .find(|l| l.starts_with("ticket cache:"))
        .unwrap_or_default()
        .to_owned()
}

/// Stdout and stderr with the trace line taken out.
fn answer(out: &Output) -> (String, String, Option<i32>) {
    let err: String = text(&out.stderr)
        .lines()
        .filter(|l| !l.starts_with("ticket cache:"))
        .map(|l| format!("{l}\n"))
        .collect();
    (text(&out.stdout), err, out.status.code())
}

fn cache_files(root: &Path) -> Vec<PathBuf> {
    let dir = root.join(".git/openwarrant/cache");
    let mut out: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| rd.filter_map(Result::ok).map(|e| e.path()).collect())
        .unwrap_or_default();
    out.sort();
    out
}

/// A program with four tickets (items, a blocker, a claim), committed, and
/// old enough that the cache may keep what it reads (`SETTLE`, 2 s).
fn program(tag: &str) -> PathBuf {
    let root = scratch(tag);
    for args in [
        vec![
            "create",
            "Alpha ticket",
            "--item",
            "first",
            "--item",
            "second",
        ],
        vec!["create", "Bravo ticket"],
        vec!["create", "Charlie ticket", "--item", "only"],
        vec!["create", "Delta ticket", "--item", "one", "--item", "two"],
    ] {
        let out = run(&root, &args, &[]);
        assert!(out.status.success(), "{args:?}: {}", text(&out.stderr));
    }
    let ready = run(
        &root,
        &["--json", "view", "ready"],
        &[("OPENWARRANT_NO_CACHE", "1")],
    );
    let v: serde_json::Value = serde_json::from_slice(&ready.stdout).unwrap();
    let r = &v["result"]["ready"][0];
    let target = format!(
        "{}{}",
        r["ticket"].as_str().unwrap(),
        r["item"]
            .as_str()
            .map(|i| format!("/{i}"))
            .unwrap_or_default()
    );
    // Claimed as someone else: the views below run as `cache-test`, and an
    // actor's own claims are renewed (their lease moved) by every command it
    // runs, which would make two runs differ by the clock alone.
    let out = run(
        &root,
        &["claim", &target],
        &[
            ("OPENWARRANT_NO_CACHE", "1"),
            ("OPENWARRANT_ACTOR", "claimer"),
        ],
    );
    assert!(out.status.success(), "{}", text(&out.stderr));
    std::thread::sleep(std::time::Duration::from_millis(2300));
    root
}

const VIEWS: &[&[&str]] = &[
    &["view", "ready"],
    &["--json", "view", "ready"],
    &["next"],
    &["--json", "next"],
    &["view", "tickets"],
    &["--json", "view", "tickets"],
    &["status"],
    &["--json", "status"],
];

#[test]
fn cold_warm_and_disabled_give_the_same_bytes() {
    let root = program("same");
    for args in VIEWS {
        let off = run(&root, args, &[("OPENWARRANT_NO_CACHE", "1"), (TRACE, "1")]);
        assert_eq!(
            trace(&off),
            "",
            "{args:?}: the cache was used while disabled"
        );
        let cold = run(&root, args, &[(TRACE, "1")]);
        let warm = run(&root, args, &[(TRACE, "1")]);
        assert_eq!(answer(&off), answer(&cold), "{args:?}: cold differs");
        assert_eq!(answer(&off), answer(&warm), "{args:?}: warm differs");
        assert!(
            trace(&warm).starts_with("ticket cache: 4 of 4 ticket(s) from the cache, 0 read")
                && trace(&warm).contains("0 shard(s) written; the listing from the cache"),
            "{args:?}: a warm run did not answer from the cache: {}",
            trace(&warm)
        );
    }
    assert!(!cache_files(&root).is_empty());
    // Nothing of it is in the tree.
    let status = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        !text(&status.stdout).contains("cache"),
        "{}",
        text(&status.stdout)
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_in_place_edit_with_its_mtime_put_back_is_seen() {
    let root = program("inplace");
    let warm = run(&root, &["--json", "view", "ready"], &[]);
    assert!(text(&warm.stdout).contains("Alpha ticket"));
    let warm = run(&root, &["--json", "view", "ready"], &[(TRACE, "1")]);
    assert!(
        trace(&warm).starts_with("ticket cache: 4 of 4"),
        "{}",
        trace(&warm)
    );
    // The manifest holding "Alpha ticket": rewritten in place, the same
    // size, the same inode, its mtime put back to the nanosecond.
    let tickets = root.join("docs/tickets");
    let manifest = std::fs::read_dir(&tickets)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path().join("manifest.toml"))
        .find(|p| {
            std::fs::read_to_string(p)
                .unwrap_or_default()
                .contains("Alpha ticket")
        })
        .unwrap();
    let before = std::fs::metadata(&manifest).unwrap();
    let edited = std::fs::read_to_string(&manifest)
        .unwrap()
        .replace("Alpha ticket", "Omega ticket");
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .open(&manifest)
            .unwrap();
        f.write_all(edited.as_bytes()).unwrap();
        f.set_modified(before.modified().unwrap()).unwrap();
    }
    let after = std::fs::metadata(&manifest).unwrap();
    assert_eq!(before.len(), after.len());
    assert_eq!(before.modified().unwrap(), after.modified().unwrap());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(before.ino(), after.ino());
    }
    let seen = run(&root, &["--json", "view", "ready"], &[(TRACE, "1")]);
    let off = run(
        &root,
        &["--json", "view", "ready"],
        &[("OPENWARRANT_NO_CACHE", "1")],
    );
    assert!(text(&seen.stdout).contains("Omega ticket"));
    assert!(!text(&seen.stdout).contains("Alpha ticket"));
    assert_eq!(answer(&seen), answer(&off));
    assert!(
        trace(&seen).starts_with("ticket cache: 3 of 4 ticket(s) from the cache, 1 read"),
        "{}",
        trace(&seen)
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_corrupt_truncated_or_foreign_cache_is_never_trusted() {
    let root = program("corrupt");
    let reference = run(&root, &["--json", "next"], &[("OPENWARRANT_NO_CACHE", "1")]);
    let _ = run(&root, &["--json", "next"], &[]);
    let files = cache_files(&root);
    assert!(files.len() > 1, "{files:?}");
    type Damage = fn(&mut Vec<u8>);
    let damages: [(&str, Damage); 3] = [
        ("a byte flipped in the middle", |b| {
            let n = b.len() / 2;
            b[n] ^= 0x01;
        }),
        ("truncated", |b| b.truncate(b.len() - 1)),
        ("garbage", |b| *b = b"not a cache file".to_vec()),
    ];
    for (what, damage) in damages {
        for f in &files {
            let mut bytes = std::fs::read(f).unwrap();
            damage(&mut bytes);
            std::fs::write(f, bytes).unwrap();
        }
        let damaged = run(&root, &["--json", "next"], &[(TRACE, "1")]);
        assert_eq!(answer(&damaged), answer(&reference), "{what}");
        assert!(
            trace(&damaged).starts_with("ticket cache: 0 of 4 ticket(s) from the cache, 4 read")
                && trace(&damaged).contains("the listing from the directory"),
            "{what}: {}",
            trace(&damaged)
        );
        // Rebuilt: the next run answers from it again.
        let again = run(&root, &["--json", "next"], &[(TRACE, "1")]);
        assert_eq!(answer(&again), answer(&reference), "{what}");
        assert!(
            trace(&again).starts_with("ticket cache: 4 of 4"),
            "{what}: {}",
            trace(&again)
        );
    }
    // Another build: the same program at another path (another inode and
    // mtime) is another identity, and ignores what this one wrote.
    let other = root.join("other-war");
    std::fs::copy(env!("CARGO_BIN_EXE_war"), &other).unwrap();
    let foreign = run_with(&other, &root, &["--json", "next"], &[(TRACE, "1")]);
    assert_eq!(answer(&foreign), answer(&reference));
    assert!(
        trace(&foreign).starts_with("ticket cache: 0 of 4"),
        "{}",
        trace(&foreign)
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn commands_that_judge_never_use_it() {
    let root = program("judge");
    let _ = run(&root, &["view", "ready"], &[]);
    assert!(!cache_files(&root).is_empty());
    for args in [
        vec!["check"],
        vec!["--json", "check"],
        vec!["admin", "compile"],
        vec!["admin", "doctor"],
    ] {
        let out = run(&root, &args, &[(TRACE, "1")]);
        assert_eq!(trace(&out), "", "{args:?} used the ticket cache");
    }
    // Doctor says whether the views use it, and where.
    let doctor = run(&root, &["--json", "admin", "doctor"], &[]);
    let v: serde_json::Value = serde_json::from_slice(&doctor.stdout).unwrap();
    assert_eq!(v["result"]["ticket_cache"]["in_use"], true, "{v}");
    let off = run(
        &root,
        &["--json", "admin", "doctor"],
        &[("OPENWARRANT_NO_CACHE", "1")],
    );
    let v: serde_json::Value = serde_json::from_slice(&off.stdout).unwrap();
    assert_eq!(v["result"]["ticket_cache"]["in_use"], false, "{v}");
    let _ = std::fs::remove_dir_all(&root);
}
