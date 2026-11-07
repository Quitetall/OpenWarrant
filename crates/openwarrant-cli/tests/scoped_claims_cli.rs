// SPDX-License-Identifier: Apache-2.0
//! Cooperative write scopes are coordination, never signing authority.
use serde_json::Value;
use std::{path::Path, process::Command};

fn war(root: &Path, args: &[&str]) -> Value {
    // These tests put several writers in one checkout on purpose: a claim for
    // another actor says so with `--here`.
    let here = args.starts_with(&["admin", "claim"]) && args.contains(&"--as");
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["--root", root.to_str().unwrap(), "--json"])
        .args(args)
        .args(here.then_some("--here"))
        .current_dir(root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .env_remove("OPENWARRANT_ACTOR")
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
fn created(root: &Path, title: &str) -> String {
    let v = war(root, &["create", title]);
    assert_eq!(v["exit_code"], 0, "{v}");
    v["result"]["id"].as_str().unwrap().into()
}

#[test]
fn shared_checkout_refuses_overlapping_scopes_before_the_second_claim() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        war(root.path(), &["init", "--namespace", "SC"])["exit_code"],
        0
    );
    let a = created(root.path(), "Reset API");
    let b = created(root.path(), "Account storage");
    let first = war(
        root.path(),
        &[
            "admin",
            "claim",
            &a,
            "--paths",
            "src/*.rs",
            "--as",
            "reset-builder",
        ],
    );
    assert_eq!(first["exit_code"], 0, "{first}");
    assert_eq!(
        first["result"]["coordination"]["workspace"],
        root.path().canonicalize().unwrap().to_str().unwrap()
    );
    assert!(first["result"]["coordination"]["session"].is_string());
    let journal = std::fs::read_to_string(
        root.path()
            .join("docs/tickets")
            .join(&a)
            .join("journal.jsonl"),
    )
    .unwrap();
    let recorded = journal
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|event| event["type"] == "work.claimed")
        .unwrap();
    assert_eq!(
        recorded["payload"]["coordination"],
        first["result"]["coordination"]
    );
    let conflict = war(
        root.path(),
        &[
            "admin",
            "claim",
            &b,
            "--paths",
            "src/accounts.rs",
            "--as",
            "accounts-builder",
        ],
    );
    assert_ne!(conflict["exit_code"], 0, "{conflict}");
    assert!(
        conflict["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "work.write-conflict"),
        "{conflict}"
    );
    let independent = war(
        root.path(),
        &[
            "admin",
            "claim",
            &b,
            "--paths",
            "docs/**",
            "--as",
            "docs-builder",
        ],
    );
    assert_eq!(independent["exit_code"], 0, "{independent}");
}

/// Paths nobody declared are not a scope: a second agent in one checkout is
/// warned, not refused, and the first claim hides no other work from `next`.
#[test]
fn undeclared_paths_warn_and_never_block_other_work() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        war(root.path(), &["init", "--namespace", "UD"])["exit_code"],
        0
    );
    let a = created(root.path(), "First job");
    let b = created(root.path(), "Second job");
    let first = war(root.path(), &["admin", "claim", &a, "--as", "ann"]);
    assert_eq!(first["exit_code"], 0, "{first}");
    let ready = war(root.path(), &["view", "ready", "--as", "bob"]);
    assert!(ready.to_string().contains(&b), "{ready}");
    let second = war(root.path(), &["admin", "claim", &b, "--as", "bob"]);
    assert_eq!(second["exit_code"], 0, "{second}");
    assert!(
        second["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "work.integration-overlap"),
        "{second}"
    );
}

/// A claim with no declared paths keeps M11's rule: when its lease runs out a
/// plain claim takes it, so a dead agent never strands ordinary work.
#[test]
fn an_undeclared_claim_is_reclaimed_when_its_lease_runs_out() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        war(root.path(), &["init", "--namespace", "UE"])["exit_code"],
        0
    );
    let config = root.path().join("openwarrant.toml");
    let mut toml = std::fs::read_to_string(&config).unwrap();
    toml.push_str("\n[tickets]\nclaim_lease_minutes = 0\n");
    std::fs::write(&config, toml).unwrap();
    let id = created(root.path(), "Left behind");
    assert_eq!(
        war(root.path(), &["admin", "claim", &id, "--as", "gone"])["exit_code"],
        0
    );
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let taken = war(root.path(), &["admin", "claim", &id, "--as", "next"]);
    assert_eq!(taken["exit_code"], 0, "{taken}");
    assert_eq!(
        taken["result"]["reclaimed_from"]["actor"], "gone",
        "{taken}"
    );
}

#[test]
fn expired_scoped_lease_is_not_proof_that_its_writer_stopped() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        war(root.path(), &["init", "--namespace", "LE"])["exit_code"],
        0
    );
    let id = created(root.path(), "Retain interrupted work");
    assert_eq!(
        war(
            root.path(),
            &[
                "admin",
                "claim",
                &id,
                "--paths",
                "src/**",
                "--as",
                "original",
                "--at",
                "2000-01-01T00:00:00Z"
            ]
        )["exit_code"],
        0
    );
    let takeover = war(
        root.path(),
        &[
            "admin",
            "claim",
            &id,
            "--steal",
            "--as",
            "replacement",
            "--at",
            "2000-01-01T00:00:01Z",
        ],
    );
    assert_ne!(takeover["exit_code"], 0, "{takeover}");
    assert!(
        takeover["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "work.writer-unfenced"),
        "{takeover}"
    );
    let ready = war(
        root.path(),
        &["view", "next", "--at", "2100-01-01T00:00:00Z"],
    );
    assert!(
        ready["result"]["ready"]
            .as_array()
            .is_none_or(|rows| rows.iter().all(|r| r["ticket"] != id)),
        "expired scoped writers require fencing before scheduling: {ready}"
    );
    let next = war(
        root.path(),
        &[
            "admin",
            "claim",
            &id,
            "--paths",
            "src/**",
            "--as",
            "replacement",
            "--at",
            "2100-01-01T00:00:00Z",
        ],
    );
    assert_ne!(next["exit_code"], 0, "{next}");
    assert!(
        next["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "work.writer-unfenced"),
        "{next}"
    );
}

#[test]
fn isolated_worktrees_allow_overlap_but_surface_integration_work() {
    let container = tempfile::tempdir().unwrap();
    let root = container.path().join("repo");
    std::fs::create_dir(&root).unwrap();
    let git = |args: &[&str]| {
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
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    git(&["init", "-q"]);
    assert_eq!(war(&root, &["init", "--namespace", "IW"])["exit_code"], 0);
    let a = created(&root, "Reset API");
    let b = created(&root, "Account storage");
    git(&["add", "-A"]);
    git(&["commit", "-qm", "baseline"]);
    let other = container.path().join("worker");
    git(&[
        "worktree",
        "add",
        "--detach",
        other.to_str().unwrap(),
        "HEAD",
    ]);
    assert_eq!(
        war(
            &root,
            &[
                "admin",
                "claim",
                &a,
                "--paths",
                "src/shared.rs",
                "--as",
                "reset-builder"
            ]
        )["exit_code"],
        0
    );
    let independent = war(
        &other,
        &[
            "admin",
            "claim",
            &b,
            "--paths",
            "src/shared.rs",
            "--as",
            "accounts-builder",
        ],
    );
    assert_eq!(independent["exit_code"], 0, "{independent}");
    assert!(
        independent["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "work.integration-overlap"),
        "{independent}"
    );
    assert!(independent["result"]["coordination"]["base_revision"].is_string());
}

#[test]
fn simultaneous_scope_claims_cannot_both_win() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        war(root.path(), &["init", "--namespace", "RC"])["exit_code"],
        0
    );
    let a = created(root.path(), "First writer");
    let b = created(root.path(), "Second writer");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let run = |id: String, actor: &'static str| {
        let root = root.path().to_owned();
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            barrier.wait();
            war(
                &root,
                &[
                    "admin",
                    "claim",
                    &id,
                    "--paths",
                    "src/**/shared.rs",
                    "--as",
                    actor,
                ],
            )
        })
    };
    let first = run(a, "first-builder");
    let second = run(b, "second-builder");
    let results = [first.join().unwrap(), second.join().unwrap()];
    assert_eq!(
        results.iter().filter(|r| r["exit_code"] == 0).count(),
        1,
        "{results:?}"
    );
}
