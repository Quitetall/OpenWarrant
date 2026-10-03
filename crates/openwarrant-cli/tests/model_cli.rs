// SPDX-License-Identifier: Apache-2.0
//! `war model` through the shipped binary (OW-WAR-0148 OBL-002): the record
//! and relation kinds a program with a ticket and a promoted Warrant has, the
//! same bytes from the same tree, and an unknown relation target reported as
//! a diagnostic with its edge kept.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ow-model-{tag}-{}-{}",
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
            "Model Test",
            "--namespace",
            "MT",
            "--root",
            root.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    root
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

fn model(root: &Path) -> (Vec<u8>, serde_json::Value) {
    let out = war(root, &["model", "--json"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    (out.stdout, v["result"].clone())
}

fn kinds(m: &serde_json::Value, list: &str, field: &str) -> Vec<String> {
    let mut k: Vec<String> = m[list]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r[field].as_str().unwrap().to_owned())
        .collect();
    k.sort();
    k.dedup();
    k
}

#[test]
fn the_model_names_records_relations_and_is_deterministic() {
    let root = scratch("kinds");
    let created = war(
        &root,
        &["create", "A ticket", "-i", "first", "-i", "second"],
    );
    assert!(created.status.success());
    let ticket = String::from_utf8_lossy(&created.stdout)
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned();
    let (_, before) = model(&root);
    let first = before["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["type"] == "item")
        .map(|r| {
            r["id"]
                .as_str()
                .unwrap()
                .rsplit('/')
                .next()
                .unwrap()
                .to_owned()
        })
        .unwrap();
    assert!(
        war(&root, &["add", &ticket, "third", "--after", &first])
            .status
            .success()
    );
    assert!(war(&root, &["promote", &ticket]).status.success());

    let (bytes, m) = model(&root);
    assert_eq!(m["schema"], "oh.war/model/v1");
    let types = kinds(&m, "records", "type");
    for t in [
        "item",
        "obligation",
        "phase",
        "requirement",
        "stage",
        "ticket",
        "warrant",
    ] {
        assert!(types.contains(&t.to_owned()), "{t} missing from {types:?}");
    }
    let rel = kinds(&m, "relations", "kind");
    for k in [
        "depends_on",
        "implements",
        "part_of",
        "promoted_to",
        "roadmap",
    ] {
        assert!(rel.contains(&k.to_owned()), "{k} missing from {rel:?}");
    }
    assert_eq!(
        m["diagnostics"].as_array().unwrap().len(),
        0,
        "{}",
        m["diagnostics"]
    );

    let (again, m2) = model(&root);
    assert_eq!(bytes, again, "the same tree gave different bytes");
    assert_eq!(m["basis_digest"], m2["basis_digest"]);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn an_unknown_target_is_a_diagnostic_and_the_edge_is_kept() {
    let root = scratch("unknown");
    let manifest = root.join("docs/warrants/MT-WAR-0001/manifest.toml");
    let mut text = std::fs::read_to_string(&manifest).unwrap();
    let nobody = "war://01a0ffff-0000-7000-8000-000000000000";
    text.push_str(&format!(
        "\n[[parents]]\ncontract_revision = 1\nref = \"{nobody}\"\n"
    ));
    std::fs::write(&manifest, text).unwrap();

    let (_, m) = model(&root);
    let edge = m["relations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["kind"] == "parent" && r["to"] == nobody);
    assert!(edge, "the edge was dropped: {}", m["relations"]);
    let named = m["diagnostics"].as_array().unwrap().iter().any(|d| {
        d["rule"] == "model.relation-target-unknown"
            && d["message"].as_str().unwrap().contains(nobody)
    });
    assert!(named, "no diagnostic: {}", m["diagnostics"]);
    std::fs::remove_dir_all(&root).ok();
}
