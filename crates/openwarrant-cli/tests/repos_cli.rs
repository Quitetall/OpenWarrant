// SPDX-License-Identifier: Apache-2.0
//! t-3c16c W6: two real repositories, one a submodule of the other. Each
//! claim beside the input it refuses.
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(root: &Path, actor: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["--root", root.to_str().unwrap()])
        .args(args)
        .current_dir(root)
        .env("OPENWARRANT_ACTOR", actor)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .env_remove("SSH_AUTH_SOCK")
        .env_remove("OPENWARRANT_NO_NOTICES")
        .env_remove("OPENWARRANT_CLAIM_ID")
        .output()
        .unwrap()
}

fn json(root: &Path, actor: &str, args: &[&str]) -> Value {
    let mut all = vec!["--json"];
    all.extend(args);
    let out = run(root, actor, &all);
    serde_json::from_slice(&out.stdout).unwrap_or_else(|_| {
        panic!(
            "{args:?}: {} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "user.email=t@invalid", "-c", "user.name=t"])
        .args(["-c", "protocol.file.allow=always"])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

const INTERFACE: &str = "---
format: openwarrant/1.0
profile: interface
version: 1.0.0
owner: hpr
parties: hpr, wmd
summary: \"What the engine relies on\"
---
# Physics interface

## CL-step · clause — A step advances the world by exactly dt
status: agreed
verify: test
check: true

`World::step(dt)` advances simulated time by `dt`.

## CL-handle · clause — Handles are stable across a step
status: tbd
verify: test
owner: hpr
due: 2026-11-20

Whether a removed handle may be reused is not decided.
";

const DOC: &str = "docs/records/interfaces/physics.md";

struct Two {
    _dir: tempfile::TempDir,
    /// HPR, the lower layer: a clone of `origin`.
    hpr: PathBuf,
    /// WMD, with HPR as a submodule at `vendor/hpr`.
    wmd: PathBuf,
    /// An open ticket of HPR's, pinned open.
    feature: String,
}

impl Two {
    fn sub(&self) -> PathBuf {
        self.wmd.join("vendor/hpr")
    }
    /// Publish HPR's work; WMD's submodule fetches it and the pin stays.
    fn push_and_fetch(&self) {
        git(&self.hpr, &["add", "-A"]);
        git(&self.hpr, &["commit", "-qm", "hpr work"]);
        git(&self.hpr, &["push", "-q", "origin", "main"]);
        git(&self.sub(), &["fetch", "-q", "origin"]);
    }
    /// Move WMD's pin to what was fetched.
    fn bump(&self) {
        git(&self.sub(), &["checkout", "-q", "origin/main"]);
        git(&self.wmd, &["add", "vendor/hpr"]);
        git(&self.wmd, &["commit", "-qm", "move the pin"]);
    }
    fn config(&self, table: &str) {
        let path = self.wmd.join("openwarrant.toml");
        let mut text = std::fs::read_to_string(&path).unwrap();
        let at = text.find("[repos.hpr]").unwrap();
        text.truncate(at);
        text.push_str(table);
        std::fs::write(path, text).unwrap();
        git(&self.wmd, &["add", "-A"]);
        git(&self.wmd, &["commit", "-qm", "config"]);
    }
}

fn two() -> Two {
    let dir = tempfile::tempdir().unwrap();
    let origin = dir.path().join("hpr.git");
    let hpr = dir.path().join("hpr");
    let wmd = dir.path().join("wmd");
    for d in [&origin, &wmd] {
        std::fs::create_dir_all(d).unwrap();
    }
    git(&origin, &["init", "-q", "--bare", "-b", "main"]);
    git(
        dir.path(),
        &["clone", "-q", origin.to_str().unwrap(), "hpr"],
    );
    git(&hpr, &["checkout", "-q", "-b", "main"]);
    assert_eq!(
        json(&hpr, "o", &["init", "--namespace", "HP"])["exit_code"],
        0
    );
    assert_eq!(
        json(&hpr, "o", &["plan", "types", "add", "interfaces"])["exit_code"],
        0
    );
    let doc = hpr.join(DOC);
    std::fs::create_dir_all(doc.parent().unwrap()).unwrap();
    std::fs::write(&doc, INTERFACE).unwrap();
    let made = json(
        &hpr,
        "o",
        &["create", "The physics feature", "--body", "Steps."],
    );
    let feature = made["result"]["id"].as_str().unwrap().to_owned();
    git(&hpr, &["add", "-A"]);
    git(&hpr, &["commit", "-qm", "start"]);
    git(&hpr, &["push", "-q", "origin", "main"]);

    git(&wmd, &["init", "-q", "-b", "main"]);
    assert_eq!(
        json(&wmd, "o", &["init", "--namespace", "WM"])["exit_code"],
        0
    );
    git(
        &wmd,
        &[
            "submodule",
            "add",
            "-q",
            origin.to_str().unwrap(),
            "vendor/hpr",
        ],
    );
    let cfg = wmd.join("openwarrant.toml");
    let mut text = std::fs::read_to_string(&cfg).unwrap();
    text.push_str(
        "\n[repos.hpr]\npath = \"vendor/hpr\"\nurl = \"https://example.invalid/hpr.git\"\n",
    );
    std::fs::write(&cfg, text).unwrap();
    git(&wmd, &["add", "-A"]);
    git(&wmd, &["commit", "-qm", "start"]);
    Two {
        _dir: dir,
        hpr,
        wmd,
        feature,
    }
}

// ---- named repositories ----------------------------------------------------

#[test]
fn a_pin_is_read_from_a_submodule_and_a_field_and_a_lock() {
    let t = two();
    let pin = git(&t.sub(), &["rev-parse", "HEAD"]);
    let short = &pin[..12];
    let shown = text(&run(&t.wmd, "o", &["status"]));
    assert!(
        shown.contains(&format!("Repos: hpr @ {short} (submodule)")),
        "{shown}"
    );

    // A sibling's commit, from a field.
    t.config(&format!(
        "[repos.hpr]\npath = \"vendor/hpr\"\npin = \"{pin}\"\n"
    ));
    let shown = text(&run(&t.wmd, "o", &["status"]));
    assert!(shown.contains(&format!("hpr @ {short} (pin)")), "{shown}");

    // A Cargo dependency's commit, from the lock committed here.
    git(&t.wmd, &["rm", "-qf", "vendor/hpr"]);
    std::fs::write(
        t.wmd.join("Cargo.lock"),
        format!(
            "[[package]]\nname = \"hpr\"\nsource = \"git+https://example.invalid/hpr.git#{pin}\"\n"
        ),
    )
    .unwrap();
    t.config("[repos.hpr]\npath = \"vendor/hpr\"\nurl = \"https://example.invalid/hpr.git\"\n");
    let shown = text(&run(&t.wmd, "o", &["status"]));
    // The pin is read; the objects are gone with the submodule's checkout, or
    // kept in `.git/modules`. It reads as one or the other, never a guess.
    assert!(
        shown.contains(&format!("hpr @ {short} (Cargo.lock)")) || shown.contains("hpr UNKNOWN"),
        "{shown}"
    );
}

#[test]
fn a_repository_with_no_pin_reads_unknown_and_status_still_answers() {
    let t = two();
    git(&t.wmd, &["rm", "-qf", "vendor/hpr"]);
    t.config("[repos.hpr]\npath = \"vendor/hpr\"\n");
    let out = run(&t.wmd, "o", &["status"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("hpr UNKNOWN"), "{}", text(&out));
}

// ---- cross-repository targets ----------------------------------------------

#[test]
fn a_clause_and_an_item_read_through_the_pin() {
    let t = two();
    let out = run(&t.wmd, "o", &["context", "hpr:CL-step"]);
    assert!(out.status.success(), "{}", text(&out));
    let said = text(&out);
    assert!(said.contains("advances simulated time"), "{said}");
    assert!(
        !said.contains("not decided"),
        "the other clause came too: {said}"
    );
    assert!(said.starts_with("hpr @ "), "{said}");

    let v = json(&t.wmd, "o", &["context", &format!("hpr:{}", t.feature)]);
    assert_eq!(v["exit_code"], 0, "{v}");
    assert!(v["notes"][0].as_str().unwrap().starts_with("hpr @ "), "{v}");

    let out = run(
        &t.wmd,
        "o",
        &["view", "prime", &format!("hpr:{}", t.feature)],
    );
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("The physics feature"), "{}", text(&out));
}

#[test]
fn a_read_comes_from_the_commit_and_never_the_checkout() {
    let t = two();
    std::fs::write(
        t.sub().join(DOC),
        INTERFACE.replace("advances simulated time", "UNCOMMITTED WORDS"),
    )
    .unwrap();
    let said = text(&run(&t.wmd, "o", &["context", "hpr:CL-step"]));
    assert!(said.contains("advances simulated time"), "{said}");
    assert!(!said.contains("UNCOMMITTED"), "{said}");
}

#[test]
fn what_the_pin_does_not_hold_is_never_guessed_at() {
    let t = two();
    // An id nothing holds: refused, not searched for.
    let out = run(&t.wmd, "o", &["context", "hpr:CL-step-gone"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(
        !text(&out).contains("advances simulated time"),
        "{}",
        text(&out)
    );

    let out = run(&t.wmd, "o", &["view", "prime", "hpr:t-00000"]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert!(text(&out).contains("UNKNOWN"), "{}", text(&out));

    // A repository nothing declares.
    let out = run(&t.wmd, "o", &["context", "nope:CL-step"]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert!(text(&out).contains("UNKNOWN"), "{}", text(&out));

    // A pin the checkout does not hold.
    t.config(&format!(
        "[repos.hpr]\npath = \"vendor/hpr\"\npin = \"{}\"\n",
        "0123456789012345678901234567890123456789"
    ));
    let out = run(&t.wmd, "o", &["context", "hpr:CL-step"]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert!(text(&out).contains("UNKNOWN"), "{}", text(&out));
    assert!(
        !text(&out).contains("advances simulated time"),
        "{}",
        text(&out)
    );
    // Local reads do not care.
    let out = run(&t.wmd, "o", &["view", "prime"]);
    assert!(out.status.success(), "{}", text(&out));
}
