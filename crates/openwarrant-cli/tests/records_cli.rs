// SPDX-License-Identifier: Apache-2.0
//! Record atoms and `war impact` through the shipped binary (OW-WAR-0148
//! OBL-006, OBL-007): the password-reset records parse into the model with
//! their relations, `war check` refuses a duplicate id by rule, and `war
//! impact` walks incoming relations and refuses an unknown id by name.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn war(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .env_remove("OPENWARRANT_ACTOR")
        .env_remove("SSH_AUTH_SOCK")
        .args(args)
        .output()
        .unwrap()
}

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ow-records-{tag}-{}-{}",
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
    let out = war(
        &root,
        &[
            "init",
            "--program",
            "Records Test",
            "--namespace",
            "RT",
            "--root",
            root.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::create_dir_all(root.join("profiles")).unwrap();
    for p in ["delivery.toml", "decision.toml", "ticket.toml"] {
        std::fs::copy(
            repo().join("profiles").join(p),
            root.join("profiles").join(p),
        )
        .unwrap();
    }
    let area = root.join("docs/records/password-reset");
    std::fs::create_dir_all(&area).unwrap();
    std::fs::copy(
        repo().join("docs/records/password-reset/10-records.md"),
        area.join("10-records.md"),
    )
    .unwrap();
    root
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{e}: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

#[test]
fn records_join_the_model_and_a_duplicate_is_refused() {
    let root = scratch("model");
    let m = json(&war(&root, &["model", "--json"]))["result"].clone();
    let req = m["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "REQ-pr1")
        .expect("REQ-pr1 is a record");
    assert_eq!(req["type"], "requirement");
    assert!(req["revision"].as_str().unwrap().starts_with("sha256:"));
    let has = |from: &str, kind: &str, to: &str| {
        m["relations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["from"] == from && r["kind"] == kind && r["to"] == to)
    };
    assert!(has("REQ-pr1", "implements", "OUT-pr1"));
    assert!(has("DEC-pr1", "selected_over", "OPT-pr2"));
    assert_eq!(
        m["diagnostics"].as_array().unwrap().len(),
        0,
        "{}",
        m["diagnostics"]
    );

    let check = war(&root, &["check"]);
    let text = String::from_utf8_lossy(&check.stdout);
    assert!(check.status.success(), "{text}");
    assert!(text.contains("records.well-formed"), "{text}");

    std::fs::write(
        root.join("docs/records/password-reset/20-again.md"),
        "---\nschema: oh.war/records/v1\nprofile: delivery\n---\n## REQ-pr1 · requirement\nimplements OUT-pr1\n",
    )
    .unwrap();
    let check = war(&root, &["check"]);
    let text = String::from_utf8_lossy(&check.stdout);
    assert!(!check.status.success(), "a duplicate passed: {text}");
    assert!(text.contains("record.duplicate-id"), "{text}");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn impact_walks_incoming_relations_and_refuses_an_unknown_id() {
    let root = scratch("impact");
    let out = war(&root, &["impact", "REQ-pr1", "--json"]);
    assert!(out.status.success());
    let i = json(&out)["result"].clone();
    assert_eq!(i["schema"], "oh.war/impact/v1");
    let affected: Vec<&str> = i["affected"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap())
        .collect();
    assert_eq!(affected, ["CON-pr1", "DEC-pr1"]);
    // OUT-pr1 is what REQ-pr1 implements: an outgoing edge, so a change to
    // REQ-pr1 does not reach it.
    let out = war(&root, &["impact", "OUT-pr1", "--json"]);
    let i = json(&out)["result"].clone();
    let ids: Vec<&str> = i["affected"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["REQ-pr1", "CON-pr1", "DEC-pr1"]);

    let out = war(&root, &["impact", "REQ-nope"]);
    assert!(!out.status.success());
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("impact.unknown-record"), "{text}");
    assert!(text.contains("REQ-nope"), "{text}");
    std::fs::remove_dir_all(&root).ok();
}
