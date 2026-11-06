// SPDX-License-Identifier: Apache-2.0
//! `war context` on what a record packet cannot answer (`oh.war/unit-context/v1`):
//! a long document, one unit, a topic, a code path. Each answer fits its
//! budget and declares its cuts; each is beside the input that still refuses.

use std::path::Path;
use std::process::{Command, Output};

fn war(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .env_remove("OPENWARRANT_ACTOR")
        .env_remove("SSH_AUTH_SOCK")
        .arg("--root")
        .arg(root)
        .args(args)
        .output()
        .unwrap()
}

fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(root)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "{args:?}");
}

fn stdout(out: &Output) -> String {
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn refused(out: &Output, rule: &str) {
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(out.stdout.is_empty(), "a refusal emits nothing: {out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains(rule),
        "{out:?}"
    );
}

/// A repository with one long design document: twenty 2 KB sections, one of
/// them about friendly fire, and a section too big for any default budget.
fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-q", "."]);
    assert!(war(root, &["init", "--namespace", "UC"]).status.success());
    let filler = "Armies march on stomachs and spreadsheets.\n".repeat(50);
    let mut doc = String::from("# Combat\n\nHow fights resolve.\n\n");
    for i in 0..20 {
        doc.push_str(&format!("## Rule {i}\n\n{filler}\n"));
    }
    doc.push_str(
        "## Friendly fire {#ff}\n\nShots at allies hit at half damage.\nSee src/world.rs.\n\n",
    );
    doc.push_str("## Huge\n\n");
    for i in 0..3 {
        doc.push_str(&format!("### Part {i}\n\n{}\n", filler.repeat(4)));
    }
    let docs = root.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("COMBAT.md"), doc).unwrap();
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", "fixture"]);
    dir
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn a_long_document_gives_what_fits_and_declares_the_rest() {
    let dir = repo();
    let md = stdout(&war(dir.path(), &["context", "docs/COMBAT.md"]));
    assert!(md.len() <= 16_000, "{}", md.len());
    assert!(md.contains("oh.war/unit-context/v1 · document"), "{md}");
    assert!(md.contains("[F] md:docs/COMBAT.md#lead"), "{md}");
    assert!(md.contains("[S] "), "{md}");
    assert!(md.contains("\n## Cut\n"), "{md}");
    // The section too big for the budget is offered by its parts.
    assert!(md.contains("#part-0 ") || md.contains("not listed"), "{md}");
}

#[test]
fn strict_refuses_a_cut_and_full_gives_everything() {
    let dir = repo();
    refused(
        &war(dir.path(), &["context", "docs/COMBAT.md", "--strict"]),
        "context.over-budget",
    );
    let md = stdout(&war(dir.path(), &["context", "docs/COMBAT.md", "--full"]));
    assert!(md.contains("budget none"), "{md}");
    assert!(!md.contains("[S] "), "{md}");
    assert!(!md.contains("\n## Cut\n"), "{md}");
    assert!(md.contains("Shots at allies"), "{md}");
}

#[test]
fn one_unit_is_named_by_its_explicit_id_with_or_without_the_prefix() {
    let dir = repo();
    for target in ["docs/COMBAT.md#ff", "md:docs/COMBAT.md#ff"] {
        let md = stdout(&war(dir.path(), &["context", target, "--strict"]));
        assert!(md.contains("Shots at allies"), "{md}");
        assert!(!md.contains("Rule 3"), "{md}");
    }
    let wrong = war(dir.path(), &["context", "docs/COMBAT.md#nope"]);
    refused(&wrong, "context.target-unknown");
    assert!(
        String::from_utf8_lossy(&wrong.stderr).contains("rule-0"),
        "the refusal lists what exists: {wrong:?}"
    );
}

#[test]
fn a_selector_that_misses_ranks_the_words_in_its_slug_and_says_so() {
    let dir = repo();
    let near = stdout(&war(dir.path(), &["context", "docs/COMBAT.md#friendly"]));
    assert!(near.contains("[F] md:docs/COMBAT.md#ff"), "{near}");
    assert!(near.contains("has no unit `friendly`"), "{near}");
    refused(
        &war(
            dir.path(),
            &["context", "docs/COMBAT.md#friendly", "--strict"],
        ),
        "context.over-budget",
    );
}

#[test]
fn a_unit_too_big_for_its_budget_falls_to_its_parts() {
    let dir = repo();
    let md = stdout(&war(dir.path(), &["context", "docs/COMBAT.md#huge"]));
    assert!(md.contains("[S] md:docs/COMBAT.md#huge"), "{md}");
    assert!(md.contains("[F] md:docs/COMBAT.md#part-0"), "{md}");
}

#[test]
fn a_topic_ranks_units_and_words_nothing_matches_stay_refused() {
    let dir = repo();
    let v = json(&war(dir.path(), &["--json", "context", "friendly fire"]));
    assert_eq!(v["result"]["schema"], "oh.war/unit-context/v1");
    assert_eq!(v["result"]["kind"], "topic");
    assert_eq!(v["result"]["units"][0]["id"], "md:docs/COMBAT.md#ff");
    assert_eq!(v["result"]["units"][0]["tier"], "full");
    assert!(v["result"]["source_bytes"].as_u64() > v["result"]["returned_bytes"].as_u64());
    refused(
        &war(dir.path(), &["context", "quantum zebra"]),
        "context.target-unknown",
    );
}

#[test]
fn a_code_path_finds_the_units_that_name_it() {
    let dir = repo();
    let v = json(&war(dir.path(), &["--json", "context", "src/world.rs"]));
    assert_eq!(v["result"]["kind"], "path");
    assert_eq!(v["result"]["units"][0]["id"], "md:docs/COMBAT.md#ff");
    refused(
        &war(dir.path(), &["context", "src/nothing_names_this.rs"]),
        "context.target-unknown",
    );
}

#[test]
fn a_file_nothing_names_falls_to_its_directory_and_says_so() {
    let dir = repo();
    let doc = dir.path().join("docs/COMBAT.md");
    let text = std::fs::read_to_string(&doc).unwrap();
    std::fs::write(&doc, text.replace("src/world.rs", "game/combat/world.rs")).unwrap();
    let md = stdout(&war(dir.path(), &["context", "game/combat/nearby.rs"]));
    assert!(md.contains("nothing names game/combat/nearby.rs"), "{md}");
    assert!(md.contains("[F] md:docs/COMBAT.md#ff"), "{md}");
}

#[test]
fn an_id_that_names_nothing_is_refused_however_its_words_match() {
    let dir = repo();
    refused(
        &war(dir.path(), &["context", "OW-WAR-9999"]),
        "context.target-unknown",
    );
    refused(
        &war(dir.path(), &["context", "t-rule-0"]),
        "context.target-unknown",
    );
}

#[test]
fn an_asked_for_budget_still_refuses_as_section_10_says() {
    let dir = repo();
    refused(
        &war(
            dir.path(),
            &["context", "docs/COMBAT.md", "--budget", "100"],
        ),
        "context.over-budget",
    );
}

#[test]
fn an_edit_is_seen_at_once_through_the_cache() {
    let dir = repo();
    let path = dir.path().join("docs/COMBAT.md");
    let first = stdout(&war(dir.path(), &["context", "docs/COMBAT.md#ff"]));
    assert!(first.contains("half damage"), "{first}");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace("half damage", "no damage")).unwrap();
    let second = stdout(&war(dir.path(), &["context", "docs/COMBAT.md#ff"]));
    assert!(second.contains("no damage"), "{second}");
}
