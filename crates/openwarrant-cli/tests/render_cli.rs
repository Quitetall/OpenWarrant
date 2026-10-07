// SPDX-License-Identifier: Apache-2.0
//! Projection targets through the shipped binary (OW-WAR-0148 M6): the
//! password-reset area renders its four declared documents, `war render`
//! gives the compiled bytes and traces every line, `war impact` names
//! exactly the projections that select a record, and an unknown projection
//! or an over-budget packet is refused by name.

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

const AREA: &str = "docs/records/password-reset";

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ow-render-{tag}-{}-{}",
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
            "Render Test",
            "--namespace",
            "RN",
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
    for p in [
        "delivery",
        "decision",
        "ticket",
        "prd",
        "architecture",
        "test-plan",
        "agent-packet",
    ] {
        std::fs::copy(
            repo().join("profiles").join(format!("{p}.toml")),
            root.join("profiles").join(format!("{p}.toml")),
        )
        .unwrap();
    }
    std::fs::create_dir_all(root.join(AREA)).unwrap();
    for f in [
        "10-records.md",
        "20-product.md",
        "30-architecture.md",
        "documents.toml",
    ] {
        std::fs::copy(repo().join(AREA).join(f), root.join(AREA).join(f)).unwrap();
    }
    root
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "not JSON ({e}): {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

#[test]
fn four_documents_render_traced_and_impact_names_exactly_their_selectors() {
    let root = scratch("four");
    let out = war(&root, &["compile"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let generated = root.join(AREA).join("generated");
    for name in ["prd", "architecture", "test-plan", "agent-packet"] {
        let file = std::fs::read_to_string(generated.join(format!("{name}.md"))).unwrap();
        // `war render` gives the compiled bytes.
        let out = war(
            &root,
            &["render", name, "--of", &format!("password-reset/{name}")],
        );
        assert!(out.status.success(), "{name}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), file, "{name}");
        // Every line is traced, with no gap.
        let p = json(&war(
            &root,
            &[
                "render",
                name,
                "--of",
                &format!("password-reset/{name}"),
                "--json",
            ],
        ))["result"]
            .clone();
        assert_eq!(p["schema"], "oh.war/projection/v1");
        let mut next = 1;
        for t in p["trace"].as_array().unwrap() {
            assert_eq!(t["start_line"].as_u64().unwrap(), next, "{name}");
            next = t["end_line"].as_u64().unwrap() + 1;
        }
        assert_eq!(next as usize, file.lines().count() + 1, "{name}");
    }
    let check = war(&root, &["check", "--generated"]);
    let text = String::from_utf8_lossy(&check.stdout);
    assert!(text.contains("prd.md matches a fresh rendering"), "{text}");

    let i = json(&war(&root, &["impact", "REQ-pr1", "--json"]))["result"].clone();
    let declared: Vec<&str> = i["projections"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["scope"] == "declared")
        .map(|p| p["path"].as_str().unwrap().rsplit('/').next().unwrap())
        .collect();
    assert_eq!(declared, ["agent-packet.md", "prd.md", "test-plan.md"]);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn unknown_projections_and_over_budget_packets_are_refused_by_name() {
    let root = scratch("refuse");
    let out = war(&root, &["render", "poster", "--of", "REQ-pr1"]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("projection.unknown"), "{err}");

    let out = war(&root, &["render", "prd", "--of", "NOPE-1"]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("projection.of-unknown"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = war(
        &root,
        &[
            "render",
            "agent-packet",
            "--of",
            "REQ-pr1",
            "--max-bytes",
            "200",
        ],
    );
    assert!(!out.status.success());
    assert!(
        out.stdout.is_empty(),
        "nothing is printed, nothing truncated"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("projection.over-budget"), "{err}");
    assert!(err.contains("over its budget of 200 bytes"), "{err}");
    let out = war(&root, &["render", "agent-packet", "--of", "REQ-pr1"]);
    assert!(out.status.success());
    std::fs::remove_dir_all(&root).ok();
}
