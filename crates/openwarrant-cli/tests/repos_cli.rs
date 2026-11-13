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

// ---- dependencies and pin-aware state --------------------------------------

/// A ticket of WMD's waiting on `target`.
fn waiting(t: &Two, target: &str, more: &[&str]) -> String {
    // One title for every waiting ticket of these tests: created outright.
    let mut args = vec![
        "create",
        "The engine feature",
        "--force",
        "--depends-on",
        target,
    ];
    args.extend(more);
    let v = json(&t.wmd, "o", &args);
    assert_eq!(v["exit_code"], 0, "{v}");
    git(&t.wmd, &["add", "-A"]);
    git(&t.wmd, &["commit", "-qm", "wait"]);
    v["result"]["id"].as_str().unwrap().to_owned()
}

fn ready(root: &Path) -> String {
    text(&run(root, "o", &["view", "ready"]))
}

fn prime(root: &Path) -> String {
    text(&run(root, "o", &["view", "prime"]))
}

#[test]
fn done_upstream_is_not_done_here_until_the_pin_moves() {
    let t = two();
    let target = format!("hpr:{}", t.feature);
    let id = waiting(&t, &target, &[]);
    // Open on both sides: it waits, and says so.
    assert!(!ready(&t.wmd).contains(&id), "{}", ready(&t.wmd));
    assert!(prime(&t.wmd).contains("held: "), "{}", prime(&t.wmd));

    // HPR's agent finishes. Nothing here knows until it is fetched.
    assert_eq!(json(&t.hpr, "a", &["start", &t.feature])["exit_code"], 0);
    assert_eq!(json(&t.hpr, "a", &["done", &t.feature])["exit_code"], 0);
    git(&t.hpr, &["add", "-A"]);
    git(&t.hpr, &["commit", "-qm", "hpr work"]);
    git(&t.hpr, &["push", "-q", "origin", "main"]);
    assert!(
        !prime(&t.wmd).contains("done upstream"),
        "{}",
        prime(&t.wmd)
    );

    git(&t.sub(), &["fetch", "-q", "origin"]);
    let said = prime(&t.wmd);
    assert!(said.contains("done upstream, not pinned here"), "{said}");
    assert!(!ready(&t.wmd).contains(&id), "{}", ready(&t.wmd));

    // A bump releases it.
    t.bump();
    assert!(ready(&t.wmd).contains(&id), "{}", ready(&t.wmd));
    assert!(!prime(&t.wmd).contains("held: "), "{}", prime(&t.wmd));
}

#[test]
fn a_ticket_warns_at_done_and_a_warrant_is_refused() {
    let t = two();
    let target = format!("hpr:{}", t.feature);
    let plain = waiting(&t, &target, &[]);
    let v = json(&t.wmd, "a", &["start", &plain]);
    assert_eq!(v["exit_code"], 0, "explicit start is never forbidden: {v}");
    let out = run(&t.wmd, "a", &["done", &plain]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("done.dependency"), "{}", text(&out));

    let tested = waiting(&t, &target, &["--test", "true"]);
    assert_eq!(json(&t.wmd, "a", &["start", &tested])["exit_code"], 0);
    let out = run(&t.wmd, "a", &["done", &tested]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("done.dependency"), "{}", text(&out));
    assert!(text(&out).contains("nothing was written"), "{}", text(&out));
}

#[test]
fn a_dependency_that_cannot_be_read_holds_a_warrant_as_unknown() {
    let t = two();
    let target = "nope:t-abc";
    let tested = waiting(&t, target, &["--test", "true"]);
    assert!(prime(&t.wmd).contains("UNKNOWN"), "{}", prime(&t.wmd));
    assert!(!ready(&t.wmd).contains(&tested), "{}", ready(&t.wmd));
    assert_eq!(json(&t.wmd, "a", &["start", &tested])["exit_code"], 0);
    let out = run(&t.wmd, "a", &["done", &tested]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert!(
        text(&out).contains("UNKNOWN (done.dependency-unknown)"),
        "{}",
        text(&out)
    );
    // The local item it holds is not hurt by reading it.
    assert!(run(&t.wmd, "o", &["status"]).status.success());
}

#[test]
fn a_tbd_clause_holds_work_until_it_is_agreed_and_pinned() {
    let t = two();
    let id = waiting(&t, "hpr:CL-handle", &[]);
    assert!(!ready(&t.wmd).contains(&id), "{}", ready(&t.wmd));
    assert!(
        prime(&t.wmd).contains("tbd (owner hpr, due 2026-11-20)"),
        "{}",
        prime(&t.wmd)
    );

    std::fs::write(
        t.hpr.join(DOC),
        INTERFACE.replace("status: tbd", "status: agreed"),
    )
    .unwrap();
    t.push_and_fetch();
    assert!(
        prime(&t.wmd).contains("agreed upstream, not pinned here"),
        "{}",
        prime(&t.wmd)
    );
    t.bump();
    assert!(ready(&t.wmd).contains(&id), "{}", ready(&t.wmd));
}

// ---- interfaces that act ---------------------------------------------------

#[test]
fn check_counts_what_is_open_and_flags_what_is_late() {
    let t = two();
    let late = INTERFACE.replace("due: 2026-11-20", "due: 2020-01-01");
    std::fs::write(t.hpr.join(DOC), late).unwrap();
    let out = run(&t.hpr, "o", &["--verbose", "check"]);
    let said = text(&out);
    assert!(said.contains("1 tbd, 0 tbr"), "{said}");
    assert!(said.contains("interface.overdue"), "{said}");
    // On time, it is not flagged.
    std::fs::write(
        t.hpr.join(DOC),
        INTERFACE.replace("due: 2026-11-20", "due: 2999-01-01"),
    )
    .unwrap();
    let said = text(&run(&t.hpr, "o", &["--verbose", "check"]));
    assert!(!said.contains("interface.overdue"), "{said}");
    // The other side counts through its pin.
    let said = text(&run(&t.wmd, "o", &["--verbose", "check"]));
    assert!(said.contains("1 tbd, 0 tbr"), "{said}");
}

#[test]
fn a_bad_repos_table_is_refused_by_check_and_breaks_no_read() {
    let t = two();
    t.config("[repos.hpr]\nnope = 1\n");
    let out = run(&t.wmd, "o", &["check"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("repos.config"), "{}", text(&out));
    assert!(run(&t.wmd, "o", &["view", "prime"]).status.success());
}

#[test]
fn a_clause_runs_and_one_without_a_check_is_unknown() {
    let t = two();
    let v = json(&t.wmd, "o", &["check", "--clauses"]);
    let runs = v["result"]["clauses"].as_array().unwrap();
    let of = |c: &str| {
        runs.iter()
            .find(|r| r["clause"] == c)
            .unwrap_or_else(|| panic!("{c}: {v}"))["result"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    assert_eq!(of("hpr:CL-step"), "PASS");
    assert_eq!(of("hpr:CL-handle"), "UNKNOWN");
    assert_eq!(v["exit_code"], 2, "{v}");

    // A check that fails is a failure, never UNKNOWN.
    std::fs::write(
        t.hpr.join(DOC),
        INTERFACE.replace("check: true", "check: false"),
    )
    .unwrap();
    let v = json(&t.hpr, "o", &["check", "--clauses"]);
    let runs = v["result"]["clauses"].as_array().unwrap();
    assert_eq!(runs[0]["result"], "FAIL", "{v}");
    // And a failure records no baseline.
    let v = json(&t.hpr, "o", &["check", "--clauses", "--record"]);
    assert_ne!(v["exit_code"], 0, "{v}");
    assert!(!t.hpr.join("docs/interfaces.lock").exists());
}

#[test]
fn drift_is_one_line_in_status_until_it_is_conformed_to() {
    let t = two();
    let line = |root: &Path| {
        text(&run(root, "o", &["status"]))
            .lines()
            .find(|l| l.starts_with("Interfaces: "))
            .map(str::to_owned)
    };
    assert!(
        line(&t.wmd).unwrap().contains("no baseline"),
        "{:?}",
        line(&t.wmd)
    );
    let _ = json(&t.wmd, "o", &["check", "--clauses", "--record"]);
    assert_eq!(line(&t.wmd).as_deref(), Some("Interfaces: 1 conform"));
    git(&t.wmd, &["add", "-A"]);
    git(&t.wmd, &["commit", "-qm", "conform"]);

    // A new revision upstream is no drift here until the pin moves.
    std::fs::write(
        t.hpr.join(DOC),
        INTERFACE.replace("version: 1.0.0", "version: 1.1.0"),
    )
    .unwrap();
    t.push_and_fetch();
    assert_eq!(line(&t.wmd).as_deref(), Some("Interfaces: 1 conform"));
    t.bump();
    let drift = line(&t.wmd).unwrap();
    assert!(drift.starts_with("Interfaces: drift: hpr:"), "{drift}");
    assert!(
        drift.contains("1.1.0") && drift.contains("1.0.0"),
        "{drift}"
    );
    let _ = json(&t.wmd, "o", &["check", "--clauses", "--record"]);
    assert_eq!(line(&t.wmd).as_deref(), Some("Interfaces: 1 conform"));
}

// ---- change requests -------------------------------------------------------

#[test]
fn a_breaking_change_is_an_order_with_one_acceptance_per_party() {
    let t = two();
    let v = json(
        &t.hpr,
        "o",
        &[
            "create",
            "Reuse handles",
            "--change",
            "breaking",
            "--clause",
            "CL-handle",
        ],
    );
    assert_eq!(v["exit_code"], 0, "{v}");
    let id = v["result"]["id"].as_str().unwrap().to_owned();
    assert_eq!(v["result"]["type"], "change-request");
    let shown = text(&run(&t.hpr, "o", &["view", "prime", &id]));
    assert!(
        shown.contains("accept: hpr") && shown.contains("accept: wmd"),
        "{shown}"
    );
    assert!(shown.contains("Order"), "{shown}");

    // Nobody but a person closes it: an agent's done hands it over, naming
    // what they sign.
    assert_eq!(json(&t.hpr, "a", &["start", &id])["exit_code"], 0);
    let done = json(&t.hpr, "a", &["done", &id]);
    assert_eq!(done["exit_code"], 0, "{done}");
    assert_eq!(
        done["result"]["ticket_state"], "awaiting_signature",
        "{done}"
    );
    let out = run(&t.hpr, "a", &["done", &id]);
    assert!(text(&out).contains("war sign"), "{}", text(&out));
}

#[test]
fn a_compatible_change_is_a_warrant_closed_by_the_clause_check() {
    let t = two();
    let v = json(
        &t.hpr,
        "o",
        &[
            "create",
            "Add a getter",
            "--change",
            "compatible",
            "--clause",
            "CL-step",
        ],
    );
    assert_eq!(v["exit_code"], 0, "{v}");
    let id = v["result"]["id"].as_str().unwrap().to_owned();
    assert!(text(&run(&t.hpr, "o", &["view", "prime", &id])).contains("Warrant"));

    // Refusals, each by name, and nothing written.
    for (args, rule) in [
        (
            vec!["create", "x", "--type", "change-request"],
            "change.class",
        ),
        (
            vec!["create", "x", "--change", "maybe", "--clause", "CL-step"],
            "change.class",
        ),
        (
            vec![
                "create",
                "x",
                "--change",
                "compatible",
                "--clause",
                "CL-gone",
            ],
            "change.clauses",
        ),
        // A clause with no check, and no --test: nothing could close it.
        (
            vec![
                "create",
                "x",
                "--change",
                "compatible",
                "--clause",
                "CL-handle",
            ],
            "change.no-trigger",
        ),
    ] {
        let out = run(&t.hpr, "o", &args);
        assert!(!out.status.success(), "{args:?}: {}", text(&out));
        assert!(text(&out).contains(rule), "{args:?}: {}", text(&out));
    }
}

#[test]
fn impact_lists_claims_on_both_sides_and_says_what_it_could_not_read() {
    let t = two();
    let agreed = INTERFACE.replace("status: tbd", "status: agreed");
    std::fs::write(t.hpr.join(DOC), &agreed).unwrap();
    t.push_and_fetch();
    t.bump();
    let step = waiting(&t, "hpr:CL-step", &[]);
    let other = waiting(&t, "hpr:CL-handle", &[]);
    assert_eq!(json(&t.wmd, "alice", &["start", &step])["exit_code"], 0);
    assert_eq!(json(&t.wmd, "bob", &["start", &other])["exit_code"], 0);

    // From the side that depends: its own claim, by the other side's id.
    let v = json(&t.wmd, "o", &["plan", "impact", "hpr:CL-step"]);
    let across = v["result"]["across"].as_array().unwrap();
    assert_eq!(across.len(), 1, "{v}");
    assert_eq!(across[0]["ticket"], step.as_str());
    assert_eq!(across[0]["side"], "here");

    // From the lower layer, naming its user: the same claim, on that side.
    let cfg = t.hpr.join("openwarrant.toml");
    let mut config = std::fs::read_to_string(&cfg).unwrap();
    config.push_str(
        "
[repos.wmd]
path = \"../wmd\"
downstream = true
",
    );
    std::fs::write(&cfg, config).unwrap();
    let v = json(&t.hpr, "o", &["plan", "impact", "CL-step"]);
    let across = v["result"]["across"].as_array().unwrap();
    assert_eq!(across.len(), 1, "{v}");
    assert_eq!(across[0]["side"], "wmd");
    assert_eq!(across[0]["actor"], "alice");

    // A side not on this machine is UNKNOWN, not nobody.
    let moved = t.wmd.with_extension("moved");
    std::fs::rename(&t.wmd, &moved).unwrap();
    let v = json(&t.hpr, "o", &["plan", "impact", "CL-step"]);
    assert!(v["result"]["across"].as_array().unwrap().is_empty(), "{v}");
    assert!(
        v["result"]["unread"][0]
            .as_str()
            .unwrap()
            .contains("UNKNOWN"),
        "{v}"
    );
}

// ---- notices across the boundary -------------------------------------------

fn heard(out: &Output) -> Vec<String> {
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter(|l| l.starts_with("notice: "))
        .map(str::to_owned)
        .collect()
}

#[test]
fn a_changed_clause_reaches_the_side_that_depends_on_it_once() {
    let t = two();
    // Agreed, so it can start; the other agent depends on another clause.
    let agreed = INTERFACE.replace("status: tbd", "status: agreed");
    std::fs::write(t.hpr.join(DOC), &agreed).unwrap();
    t.push_and_fetch();
    t.bump();
    let step = waiting(&t, "hpr:CL-step", &["--test", "true"]);
    let other = waiting(&t, "hpr:CL-handle", &[]);
    assert_eq!(json(&t.wmd, "alice", &["start", &step])["exit_code"], 0);
    assert_eq!(json(&t.wmd, "bob", &["start", &other])["exit_code"], 0);

    // HPR changes CL-step. Published, not yet fetched: nobody can know.
    std::fs::write(
        t.hpr.join(DOC),
        agreed.replace("advances simulated time", "steps time"),
    )
    .unwrap();
    git(&t.hpr, &["add", "-A"]);
    git(&t.hpr, &["commit", "-qm", "change a clause"]);
    git(&t.hpr, &["push", "-q", "origin", "main"]);
    assert!(heard(&run(&t.wmd, "alice", &["note", &step, "working"])).is_empty());

    git(&t.sub(), &["fetch", "-q", "origin"]);
    // Bob's write finds it. It is not his.
    assert!(heard(&run(&t.wmd, "bob", &["note", &other, "working"])).is_empty());
    let first = heard(&run(&t.wmd, "alice", &["note", &step, "again"]));
    assert_eq!(first.len(), 1, "{first:?}");
    assert!(
        first[0].contains("hpr:CL-step changed upstream"),
        "{first:?}"
    );
    assert!(heard(&run(&t.wmd, "alice", &["note", &step, "more"])).is_empty());

    // Unseen, a Warrant stays open; seen and unpinned, it is behind.
    let out = run(&t.wmd, "alice", &["done", &step]);
    assert!(text(&out).contains("done.unseen"), "{}", text(&out));
    assert_eq!(json(&t.wmd, "alice", &["start", &step])["exit_code"], 0);
    let out = run(&t.wmd, "alice", &["done", &step]);
    assert!(text(&out).contains("done.behind"), "{}", text(&out));

    // The pin moves: the same change is not heard twice, and it closes.
    t.bump();
    assert!(heard(&run(&t.wmd, "alice", &["note", &step, "bumped"])).is_empty());
    let out = run(&t.wmd, "alice", &["done", &step]);
    assert!(out.status.success(), "{}", text(&out));
    // Bob's clause never changed.
    assert!(heard(&run(&t.wmd, "bob", &["note", &other, "still fine"])).is_empty());
}

#[test]
fn a_notice_reaches_another_clone_through_the_remote() {
    let dir = tempfile::tempdir().unwrap();
    let origin = dir.path().join("o.git");
    std::fs::create_dir_all(&origin).unwrap();
    git(&origin, &["init", "-q", "--bare", "-b", "main"]);
    git(dir.path(), &["clone", "-q", origin.to_str().unwrap(), "a"]);
    let a = dir.path().join("a");
    git(&a, &["checkout", "-q", "-b", "main"]);
    assert_eq!(
        json(&a, "o", &["init", "--namespace", "CL"])["exit_code"],
        0
    );
    let cfg = a.join("openwarrant.toml");
    let mut config = std::fs::read_to_string(&cfg).unwrap();
    config.push_str("\n[claims]\nremote = \"origin\"\n");
    std::fs::write(&cfg, config).unwrap();
    std::fs::create_dir_all(a.join("docs")).unwrap();
    std::fs::write(a.join("docs/plan.md"), "# Plan\n\nfirst\n").unwrap();
    let body = "Follow docs/plan.md.";
    let one = json(&a, "o", &["create", "A first", "--body", body])["result"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let two = json(&a, "o", &["create", "B second", "--body", body])["result"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    git(&a, &["add", "-A"]);
    git(&a, &["commit", "-qm", "start"]);
    git(&a, &["push", "-q", "origin", "main"]);
    git(dir.path(), &["clone", "-q", origin.to_str().unwrap(), "b"]);
    let b = dir.path().join("b");

    // An agent in each clone watches the plan, on a branch of its own.
    git(&a, &["checkout", "-q", "-b", "work-a"]);
    git(&b, &["checkout", "-q", "-b", "work-b"]);
    assert_eq!(json(&a, "alice", &["start", &one])["exit_code"], 0);
    assert_eq!(json(&b, "bob", &["start", &two])["exit_code"], 0);

    // Main moves, seen only from A.
    git(&a, &["checkout", "-q", "main"]);
    std::fs::write(a.join("docs/plan.md"), "# Plan\n\nsecond\n").unwrap();
    git(&a, &["add", "-A"]);
    git(&a, &["commit", "-qm", "amend"]);
    git(&a, &["push", "-q", "origin", "main"]);
    let _ = run(&a, "other", &["note", &one, "looked"]);

    // B has fetched nothing, and hears.
    let refs = git(&b, &["ls-remote", "origin", "refs/openwarrant/notices/*"]);
    assert!(!refs.is_empty(), "nothing published");
    // Writes read the remote at most once a minute: a minute goes by.
    let stamp = b.join(".git/openwarrant/notices/.remote-read");
    std::fs::remove_file(&stamp).unwrap();
    let said = heard(&run(&b, "bob", &["note", &two, "working"]));
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].contains("docs/plan.md") && said[0].contains("another clone"),
        "{said:?}"
    );
    std::fs::remove_file(&stamp).unwrap();
    assert!(heard(&run(&b, "bob", &["note", &two, "again"])).is_empty());
}

// ---- the other ways in ------------------------------------------------------

#[test]
fn a_worktree_with_no_submodule_checkout_reads_through_its_own_pin() {
    let t = two();
    let wt = t.wmd.with_extension("wt");
    git(
        &t.wmd,
        &["worktree", "add", "-q", wt.to_str().unwrap(), "-b", "side"],
    );
    assert!(!wt.join("vendor/hpr/.git").exists());
    let said = text(&run(&wt, "o", &["context", "hpr:CL-step"]));
    assert!(said.contains("advances simulated time"), "{said}");
}

#[test]
fn the_mcp_server_reads_through_the_pin_and_says_unknown() {
    use std::io::{BufRead, BufReader, Write};
    let t = two();
    let mut child = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["mcp", "--all"])
        .current_dir(&t.wmd)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut out = BufReader::new(child.stdout.take().unwrap());
    let mut call = |id: u64, method: &str, params: Value| -> String {
        let v = serde_json::json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        stdin.write_all(format!("{v}\n").as_bytes()).unwrap();
        stdin.flush().unwrap();
        let mut line = String::new();
        out.read_line(&mut line).unwrap();
        line
    };
    call(
        1,
        "initialize",
        serde_json::json!({"protocolVersion":"2025-06-18","capabilities":{},
            "clientInfo":{"name":"test","version":"0"}}),
    );
    let tool = |name: &str, args: Value| serde_json::json!({"name": name, "arguments": args});
    let read = call(
        2,
        "tools/call",
        tool("war_context", serde_json::json!({"target": "hpr:CL-step"})),
    );
    assert!(read.contains("advances simulated time"), "{read}");
    assert!(read.contains("hpr @ "), "{read}");
    let missing = call(
        3,
        "tools/call",
        tool("war_context", serde_json::json!({"target": "nope:CL-step"})),
    );
    assert!(missing.contains("repos.unknown"), "{missing}");
    assert!(!missing.contains("advances simulated time"), "{missing}");
    let prime = call(
        4,
        "tools/call",
        tool(
            "war_prime",
            serde_json::json!({"ticket": format!("hpr:{}", t.feature)}),
        ),
    );
    assert!(prime.contains("The physics feature"), "{prime}");
    let _ = child.kill();
    let _ = child.wait();
}
