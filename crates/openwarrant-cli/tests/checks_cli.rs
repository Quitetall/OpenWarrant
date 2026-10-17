// SPDX-License-Identifier: Apache-2.0
//! Checks and triggers under FORMAT-1.0 §12, through the shipped binary: a
//! check record run (`war check run`), fired (`war check fire`, `war done`,
//! `war view watch --checks`) and applied (`war check evaluate`). Each claim
//! sits beside its refusal: a pass ticks `observed`, a fail and an UNKNOWN
//! run tick nothing; a run on an older revision ticks nothing; a trigger
//! fired twice runs once; a hook that is not ours is never overwritten.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ow-checks-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    git(&root, &["init", "-q", "."]);
    let out = war(
        &root,
        None,
        &[
            "init",
            "--no-preset",
            "--namespace",
            "CK",
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

fn git(root: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(root)
        .env_remove("SSH_AUTH_SOCK")
        .output()
        .unwrap();
    assert!(
        ok.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&ok.stderr)
    );
}

fn commit(root: &Path, msg: &str) {
    git(root, &["add", "-A"]);
    git(root, &["commit", "-qm", msg]);
}

fn war(root: &Path, actor: Option<&str>, args: &[&str]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_war"));
    c.current_dir(root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .env_remove("OPENWARRANT_ACTOR")
        .env_remove("SSH_AUTH_SOCK")
        .env_remove("SSH_AGENT_PID")
        .args(args);
    if let Some(a) = actor {
        c.env("OPENWARRANT_ACTOR", a);
    }
    c.output().unwrap()
}

fn json(root: &Path, actor: Option<&str>, args: &[&str]) -> serde_json::Value {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = war(root, actor, &all);
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{args:?}: not an envelope ({e}): {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn rules(v: &serde_json::Value) -> Vec<String> {
    v["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|d| d["severity"] != "pass")
        .filter_map(|d| d["rule"].as_str().map(str::to_owned))
        .collect()
}

const UUID: &str = "01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a9e10";

/// A Format 1.0 ticket `t-0c01` with two items and the record `checks`.
fn ticket(root: &Path, checks: &str) {
    let dir = root.join("docs/tickets");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("t-0c01.md"),
        format!(
            "---\nformat: openwarrant/1.0\nprofile: ticket\nalias: t-0c01\nuuid: {UUID}\n\
             summary: Checks.\n---\n# Checks\n\n## Checklist\n- [ ] first (i-0c01)\n\
             - [ ] second (i-0c02)\n\n{checks}"
        ),
    )
    .unwrap();
}

fn doc(root: &Path) -> String {
    std::fs::read_to_string(root.join("docs/tickets/t-0c01.md")).unwrap()
}

fn events(root: &Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(root.join("docs/tickets/t-0c01/journal.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn of_type<'a>(ev: &'a [serde_json::Value], t: &str) -> Vec<&'a serde_json::Value> {
    ev.iter().filter(|e| e["type"] == t).collect()
}

fn states(root: &Path) -> Vec<(String, String, String)> {
    let v = json(root, None, &["model"]);
    v["result"]["states"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["record"].as_str().unwrap().starts_with("t-0c01/CHK"))
        .map(|s| {
            (
                s["record"].as_str().unwrap().to_owned(),
                s["value"].as_str().unwrap().to_owned(),
                s["provenance"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

#[test]
fn a_pass_ticks_observed_and_a_fail_ticks_nothing() {
    let root = scratch("pass-fail");
    ticket(
        &root,
        "## CHK-a · check — the flag is there\nrun: test -f flag\non: manual\nevaluates i-0c01, i-0c02\n",
    );
    commit(&root, "ticket");
    // Refused half first: the command fails, nothing is ticked, the check
    // reads `failing` (recorded, from its run).
    let v = json(&root, Some("ci"), &["check", "run", "t-0c01/CHK-a"]);
    assert_eq!(v["exit_code"], 2, "{v}");
    assert!(rules(&v).contains(&"check.failed".to_owned()), "{v}");
    assert!(!doc(&root).contains("[x]"), "{}", doc(&root));
    assert_eq!(
        states(&root),
        vec![(
            "t-0c01/CHK-a".to_owned(),
            "failing".to_owned(),
            "recorded".to_owned()
        )]
    );
    let ev = events(&root);
    assert_eq!(of_type(&ev, "trigger.fired").len(), 1);
    let run = of_type(&ev, "check.run")[0];
    assert_eq!(run["v"], 2);
    assert_eq!(run["payload"]["verdict"], "FAIL");
    assert_eq!(run["payload"]["check"], "CHK-a");
    assert_eq!(run["payload"]["subjects"].as_array().unwrap().len(), 2);
    assert!(
        run["payload"]["output_sha256"]
            .as_str()
            .is_some_and(|s| s.len() == 64)
    );
    assert!(of_type(&ev, "work.item_done").is_empty());

    // Accepted half: with the flag there, a pass ticks both at observed.
    std::fs::write(root.join("flag"), "").unwrap();
    let v = json(&root, Some("ci"), &["check", "run", "CHK-a"]);
    assert_eq!(v["exit_code"], 0, "{v}");
    let d = doc(&root);
    assert!(d.contains("- [x] first (i-0c01) — done by ci, "), "{d}");
    assert!(d.contains("[observed]"), "{d}");
    assert_eq!(d.matches("[observed]").count(), 2, "{d}");
    assert!(states(&root).is_empty(), "a pass is not failing");
    let ev = events(&root);
    let done = of_type(&ev, "work.item_done");
    assert_eq!(done.len(), 2);
    assert_eq!(done[0]["payload"]["level"], "observed");
    assert_eq!(done[0]["payload"]["episode"], 0);
    let pass = of_type(&ev, "check.run")[1];
    assert_eq!(done[0]["payload"]["runs"][0], pass["payload"]["run"]);
    // The tick is backed: the checker says nothing about it.
    let c = json(&root, None, &["check"]);
    assert!(!rules(&c).contains(&"work.tick-unbacked".to_owned()), "{c}");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn an_unknown_run_ticks_nothing_and_is_neither_pass_nor_fail() {
    let root = scratch("unknown");
    ticket(
        &root,
        "## CHK-slow · check — too slow\nrun: sleep 5\non: manual\ntimeout: 1\nevaluates i-0c01\n\n\
         ## CHK-gone · check — no such tool\nrun: gate://nope.gate@1.0.0\non: manual\nevaluates i-0c02\n",
    );
    commit(&root, "ticket");
    let v = json(&root, Some("ci"), &["check", "run", "t-0c01"]);
    assert_eq!(v["exit_code"], 2, "UNKNOWN is not ready: {v}");
    let r = rules(&v);
    assert!(r.contains(&"check.unknown".to_owned()), "{v}");
    assert!(
        !r.contains(&"check.failed".to_owned()),
        "an UNKNOWN is never a fail: {v}"
    );
    assert!(!doc(&root).contains("[x]"));
    assert!(states(&root).is_empty(), "an UNKNOWN run is not failing");
    let ev = events(&root);
    let verdicts: Vec<&str> = of_type(&ev, "check.run")
        .iter()
        .map(|e| e["payload"]["verdict"].as_str().unwrap())
        .collect();
    assert_eq!(verdicts, vec!["UNKNOWN", "UNKNOWN"]);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_fail_reopens_only_when_the_check_says_so() {
    let root = scratch("reopen");
    ticket(
        &root,
        "## CHK-keep · check — keeps\nrun: false\non: manual\nevaluates i-0c01\n\n\
         ## CHK-back · check — reopens\nrun: false\non: manual\nreopen: true\nevaluates i-0c02\n",
    );
    commit(&root, "ticket");
    for item in ["t-0c01/i-0c01", "t-0c01/i-0c02"] {
        assert!(war(&root, Some("bob"), &["claim", item]).status.success());
    }
    // `war done` fires `done`: these checks declare `manual`, so the ticks
    // are claimed, nothing run.
    for item in ["t-0c01/i-0c01", "t-0c01/i-0c02"] {
        let out = war(&root, Some("bob"), &["done", item]);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    assert!(events(&root).iter().all(|e| e["type"] != "check.run"));
    let v = json(&root, Some("ci"), &["check", "run"]);
    assert_eq!(v["exit_code"], 2, "{v}");
    let d = doc(&root);
    assert!(
        d.contains("- [x] first (i-0c01) — done by bob"),
        "without reopen it stays: {d}"
    );
    assert!(
        d.contains("- [ ] second (i-0c02)\n"),
        "with reopen: true it opens: {d}"
    );
    let ev = events(&root);
    let reopened = of_type(&ev, "work.item_reopened");
    assert_eq!(reopened.len(), 1);
    assert_eq!(reopened[0]["payload"]["target"], "t-0c01/i-0c02");
    assert_eq!(reopened[0]["payload"]["episode"], 0);
    // Evaluating again reopens nothing more: the box is open.
    let v = json(&root, Some("ci"), &["check", "evaluate"]);
    assert_eq!(
        of_type(&events(&root), "work.item_reopened").len(),
        1,
        "{v}"
    );
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn done_fires_the_default_trigger() {
    let root = scratch("done");
    // No `on`: the default trigger is `done`.
    ticket(
        &root,
        "## CHK-a · check — the flag is there\nrun: test -f flag\nevaluates i-0c01\n",
    );
    commit(&root, "ticket");
    assert!(
        war(&root, Some("bob"), &["claim", "t-0c01/i-0c01"])
            .status
            .success()
    );
    let v = json(&root, Some("bob"), &["done", "t-0c01/i-0c01"]);
    assert!(rules(&v).contains(&"check.failed".to_owned()), "{v}");
    assert!(
        !doc(&root).contains("[x]"),
        "a failing done check ticks nothing"
    );
    std::fs::write(root.join("flag"), "").unwrap();
    let v = json(&root, Some("bob"), &["done", "t-0c01/i-0c01"]);
    assert_eq!(v["exit_code"], 0, "{v}");
    assert!(doc(&root).contains("- [x] first (i-0c01) — done by bob, "));
    assert!(doc(&root).contains("[observed]"));
    // An item no check evaluates is ticked as claimed.
    assert!(
        war(&root, Some("bob"), &["claim", "t-0c01/i-0c02"])
            .status
            .success()
    );
    assert!(
        war(&root, Some("bob"), &["done", "t-0c01/i-0c02"])
            .status
            .success()
    );
    assert_eq!(
        doc(&root).matches("[observed]").count(),
        1,
        "{}",
        doc(&root)
    );
    let ev = events(&root);
    let fired: Vec<&str> = of_type(&ev, "trigger.fired")
        .iter()
        .map(|e| e["payload"]["trigger"].as_str().unwrap())
        .collect();
    assert_eq!(fired, vec!["done", "done"]);
    std::fs::remove_dir_all(&root).ok();
}

/// One `check.run` line, keyed as §11.2 keys a `v: 2` event.
fn run_line(payload: serde_json::Value) -> String {
    use sha2::Digest as _;
    let canonical = serde_jcs::to_string(&payload).unwrap();
    let key = sha2::Sha256::digest(format!("{UUID}\ncheck.run\n{canonical}").as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let ev = serde_json::json!({
        "v": 2, "id": "01a0f00e-0000-7000-8000-00000000000e", "document_uuid": UUID,
        "type": "check.run", "class": "action_receipt", "actor_ref": "ci",
        "occurred_at": "2026-10-14T09:30:00Z", "payload": payload, "idempotency_key": key,
    });
    format!("{}\n", serde_jcs::to_string(&ev).unwrap())
}

fn sha(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    format!(
        "sha256:{}",
        sha2::Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}

#[test]
fn a_run_counts_only_for_the_revisions_it_names() {
    let root = scratch("stale");
    let check = "## CHK-a · check — passes\nrun: true\non: manual\nevaluates i-0c01, i-0c02\n";
    ticket(&root, check);
    commit(&root, "ticket");
    let rev = sha(check.as_bytes());
    let first = sha(b"- [ ] first (i-0c01)\n");
    let second = sha(b"- [ ] second (i-0c02)\n");
    std::fs::create_dir_all(root.join("docs/tickets/t-0c01")).unwrap();
    let write = |check_rev: &str, second_rev: &str| {
        std::fs::write(
            root.join("docs/tickets/t-0c01/journal.jsonl"),
            run_line(serde_json::json!({
                "check": "CHK-a", "run": "r-1", "verdict": "PASS", "revision": check_rev,
                "subjects": [["t-0c01/i-0c01", first], ["t-0c01/i-0c02", second_rev]],
            })),
        )
        .unwrap();
    };
    // A pass on an older revision of the check ticks nothing.
    write(
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        &second,
    );
    let before = doc(&root);
    let v = json(
        &root,
        Some("ci"),
        &["check", "evaluate", "--at", "2026-10-14T10:00:00Z"],
    );
    assert_eq!(v["exit_code"], 0, "{v}");
    assert_eq!(doc(&root), before);
    // A pass on the current check, naming an older revision of one item,
    // ticks the other only.
    write(
        &rev,
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    );
    let v = json(
        &root,
        Some("ci"),
        &["check", "evaluate", "--at", "2026-10-14T10:00:00Z"],
    );
    assert_eq!(v["exit_code"], 0, "{v}");
    let d = doc(&root);
    assert!(
        d.contains("- [x] first (i-0c01) — done by ci, 2026-10-14 [observed]\n"),
        "{d}"
    );
    assert!(d.contains("- [ ] second (i-0c02)\n"), "{d}");
    let effects: Vec<(String, String)> = v["result"]["evaluation"]["effects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["target"].as_str().unwrap().to_owned(),
                e["effect"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert!(
        effects.contains(&("t-0c01/i-0c02".to_owned(), "stale".to_owned())),
        "{v}"
    );
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_hand_tick_is_raised_by_a_later_pass() {
    let root = scratch("raise");
    let check = "## CHK-a · check — passes\nrun: true\non: manual\nevaluates i-0c01\n";
    ticket(&root, check);
    let d = doc(&root).replace(
        "- [ ] first (i-0c01)",
        "- [x] first (i-0c01) — done by bob, 2026-10-13",
    );
    std::fs::write(root.join("docs/tickets/t-0c01.md"), d).unwrap();
    commit(&root, "ticket");
    let v = json(&root, Some("ci"), &["check", "run"]);
    assert_eq!(v["exit_code"], 0, "{v}");
    assert!(
        doc(&root).contains("- [x] first (i-0c01) — done by bob, 2026-10-13 [observed]\n"),
        "who ticked it and when stay: {}",
        doc(&root)
    );
    let ev = events(&root);
    let raised = of_type(&ev, "work.tick_raised");
    assert_eq!(raised.len(), 1);
    assert_eq!(raised[0]["payload"]["from"], "claimed");
    assert_eq!(raised[0]["payload"]["to"], "observed");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_trigger_fires_once_per_revision_and_subject() {
    let root = scratch("fire");
    ticket(
        &root,
        "## CHK-src · check — src builds\nrun: true\non: paths:src/**\nevaluates i-0c01\n\n\
         ## CHK-ci · check — ci\nrun: true\non: ci:build, x.deploy\nevaluates i-0c02\n",
    );
    commit(&root, "ticket");
    // A commit that changes no matching path fires nothing.
    let v = json(&root, Some("ci"), &["check", "fire", "commit"]);
    assert!(
        v["result"]["firing"]["runs"].as_array().unwrap().is_empty(),
        "{v}"
    );
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/a.rs"), "").unwrap();
    commit(&root, "src");
    let v = json(&root, Some("ci"), &["check", "fire", "commit"]);
    let runs = v["result"]["firing"]["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1, "{v}");
    assert_eq!(runs[0]["check"], "t-0c01/CHK-src");
    assert_eq!(runs[0]["recorded"], true);
    // The same commit again, by another evaluator: recorded once.
    let v = json(&root, Some("ci-2"), &["check", "fire", "commit"]);
    assert_eq!(v["result"]["firing"]["runs"][0]["recorded"], false, "{v}");
    assert_eq!(of_type(&events(&root), "check.run").len(), 1);
    // A dotted extension is carried, never fired; a ci trigger fires.
    let v = json(&root, Some("ci"), &["check", "fire", "x.deploy"]);
    assert!(
        v["result"]["firing"]["runs"].as_array().unwrap().is_empty(),
        "{v}"
    );
    let v = json(&root, Some("ci"), &["check", "fire", "ci:build"]);
    assert_eq!(
        v["result"]["firing"]["runs"][0]["check"], "t-0c01/CHK-ci",
        "{v}"
    );
    assert_eq!(
        doc(&root).matches("[observed]").count(),
        2,
        "{}",
        doc(&root)
    );
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn the_watcher_fires_commits_and_the_hook_is_never_someone_elses() {
    let root = scratch("watch-hook");
    ticket(
        &root,
        "## CHK-c · check — on commit\nrun: true\non: commit\nevaluates i-0c01\n",
    );
    commit(&root, "ticket");
    let out = war(&root, Some("ci"), &["view", "watch", "--once", "--checks"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("t-0c01/i-0c01 ticked by t-0c01/CHK-c"),
        "{text}"
    );
    assert!(doc(&root).contains("[observed]"));

    let hook = root.join(".git/hooks/post-commit");
    std::fs::write(&hook, "#!/bin/sh\necho mine\n").unwrap();
    let v = json(&root, None, &["check", "install-hook"]);
    assert!(rules(&v).contains(&"check.hook-exists".to_owned()), "{v}");
    assert_eq!(
        std::fs::read_to_string(&hook).unwrap(),
        "#!/bin/sh\necho mine\n"
    );
    std::fs::remove_file(&hook).unwrap();
    let v = json(&root, None, &["check", "install-hook"]);
    assert_eq!(v["exit_code"], 0, "{v}");
    assert!(
        std::fs::read_to_string(&hook)
            .unwrap()
            .contains("check fire commit")
    );
    let v = json(&root, None, &["check", "install-hook"]);
    assert_eq!(v["result"]["written"], false, "{v}");
    std::fs::remove_dir_all(&root).ok();
}
