// SPDX-License-Identifier: Apache-2.0
//! Cooperative write scopes are coordination, never signing authority.
use serde_json::Value;
use std::{path::Path, process::Command};

fn war(root: &Path, args: &[&str]) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["--root", root.to_str().unwrap(), "--json"])
        .args(args)
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
