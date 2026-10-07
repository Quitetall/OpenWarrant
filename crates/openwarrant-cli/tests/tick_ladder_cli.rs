// SPDX-License-Identifier: Apache-2.0
//! Optional parts and the tick ladder (OW-WAR-0148 M13), through the shipped
//! binary: a title-only Warrant gains a test and ticks at observed only when
//! it passes; a minimum refuses a lower tick; a claimed tick never reads as
//! checked; KPIs keep best, latest and target; a KPI that prints no number is
//! UNKNOWN; an independent verdict by the performer is refused; the Claude
//! Code bridge does nothing for a task that names nothing.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ow-ladder-{tag}-{}-{}",
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
    let out = war(&root, &["init", "--namespace", "TL"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    root
}

fn war_env(root: &Path, args: &[&str], env: &[(&str, &str)], stdin: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_war"));
    cmd.current_dir(root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .env_remove("OPENWARRANT_ACTOR")
        .env_remove("SSH_AUTH_SOCK")
        .env_remove("SSH_AGENT_PID")
        .args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    match stdin {
        None => cmd.output().unwrap(),
        Some(text) => {
            cmd.stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            let mut child = cmd.spawn().unwrap();
            std::io::Write::write_all(child.stdin.as_mut().unwrap(), text.as_bytes()).unwrap();
            child.wait_with_output().unwrap()
        }
    }
}

fn war(root: &Path, args: &[&str]) -> Output {
    war_env(root, args, &[], None)
}

fn json(root: &Path, args: &[&str]) -> serde_json::Value {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = war(root, &all);
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{args:?}: not an envelope ({e}): {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn rules(v: &serde_json::Value) -> Vec<String> {
    v["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|d| d["rule"].as_str().map(str::to_owned))
        .collect()
}

#[test]
fn a_title_only_warrant_ticks_at_observed_only_when_its_test_passes() {
    let root = scratch("title");
    let created = json(&root, &["create", "Make the parser fast"]);
    let t = created["result"]["id"].as_str().unwrap().to_owned();
    // The hint is one line, and says how to add a test.
    let hint = created["result"]["hint"].as_str().unwrap_or_default();
    assert!(hint.contains(&format!("war add {t} --test")), "{created}");
    assert!(
        !root
            .join(format!("docs/tickets/{t}/atoms/30-checks.md"))
            .exists()
    );

    std::fs::write(root.join("gate.sh"), "#!/bin/sh\nexit \"${GATE:-1}\"\n").unwrap();
    let added = json(
        &root,
        &["add", &t, "--test", "sh gate.sh", "--name", "unit"],
    );
    assert_eq!(added["exit_code"], 0, "{added}");
    assert!(
        root.join(format!("docs/tickets/{t}/atoms/30-checks.md"))
            .exists()
    );
    assert!(war(&root, &["claim", &t]).status.success());

    // Refused: the failing test is named, nothing is ticked.
    let refused = json(&root, &["done", &t, "--check"]);
    assert_eq!(refused["exit_code"], 2, "{refused}");
    assert!(rules(&refused).contains(&"ticket.check-failed".to_owned()));
    let message = refused["diagnostics"][0]["message"].as_str().unwrap();
    assert!(message.contains("unit (`sh gate.sh`)"), "{message}");
    let checklist =
        std::fs::read_to_string(root.join(format!("docs/tickets/{t}/atoms/15-checklist.md")))
            .unwrap();
    assert!(!checklist.contains("[x]"), "{checklist}");

    // Accepted: passing, it ticks at observed and journals the receipt.
    let ok = war_env(
        &root,
        &["--json", "done", &t, "--check"],
        &[("GATE", "0")],
        None,
    );
    let ok: serde_json::Value = serde_json::from_slice(&ok.stdout).unwrap();
    assert_eq!(ok["exit_code"], 0, "{ok}");
    assert_eq!(ok["result"]["level"], "observed");
    let checklist =
        std::fs::read_to_string(root.join(format!("docs/tickets/{t}/atoms/15-checklist.md")))
            .unwrap();
    assert!(checklist.contains("[observed]"), "{checklist}");
    let journal =
        std::fs::read_to_string(root.join(format!("docs/tickets/{t}/journal.jsonl"))).unwrap();
    assert!(journal.contains("ticket.check_run") && journal.contains("stdout_sha256"));

    let show = text(&war(&root, &["show", &t]));
    assert!(
        show.contains("- [x] (observed) Make the parser fast"),
        "{show}"
    );
}

#[test]
fn a_minimum_of_observed_refuses_a_claimed_tick_and_names_the_command() {
    let root = scratch("min");
    let t = json(&root, &["create", "Release"])["result"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let added = json(
        &root,
        &["add", &t, "--milestone", "Beta ships", "--min", "observed"],
    );
    assert_eq!(added["exit_code"], 0, "{added}");
    let item = added["result"]["item"].as_str().unwrap().to_owned();
    let target = format!("{t}/{item}");
    assert!(war(&root, &["claim", &target]).status.success());
    let refused = json(&root, &["done", &target]);
    assert_eq!(refused["exit_code"], 2, "{refused}");
    assert!(rules(&refused).contains(&"ticket.tick-below-minimum".to_owned()));
    let message = refused["diagnostics"][0]["message"].as_str().unwrap();
    assert!(
        message.contains(&format!("`war done {target} --check`")),
        "{message}"
    );
    // With a test that passes, --check reaches the minimum.
    json(&root, &["add", &target, "--test", "true"]);
    let ok = json(&root, &["done", &target, "--check"]);
    assert_eq!(ok["exit_code"], 0, "{ok}");
    let roadmap_free = text(&war(&root, &["status"]));
    assert!(roadmap_free.contains("Milestones:"), "{roadmap_free}");
    assert!(roadmap_free.contains("(observed)"), "{roadmap_free}");
}

#[test]
fn claimed_and_observed_ticks_read_differently_and_a_forged_marker_reads_claimed() {
    let root = scratch("words");
    let created = json(
        &root,
        &["create", "Two items", "-i", "plain", "-i", "checked"],
    );
    let t = created["result"]["id"].as_str().unwrap().to_owned();
    let items = created["result"]["items"].as_array().unwrap().clone();
    let (a, b) = (
        format!("{t}/{}", items[0]["id"].as_str().unwrap()),
        format!("{t}/{}", items[1]["id"].as_str().unwrap()),
    );
    json(&root, &["add", &b, "--test", "true"]);
    for x in [&a, &b] {
        assert!(war(&root, &["claim", x]).status.success());
    }
    let plain = text(&war(&root, &["done", &a]));
    assert!(plain.contains("ticked as claimed"), "{plain}");
    assert!(war(&root, &["done", &b, "--check"]).status.success());
    // The claimed line on disk is the bytes a tick always had.
    let checklist =
        std::fs::read_to_string(root.join(format!("docs/tickets/{t}/atoms/15-checklist.md")))
            .unwrap();
    let line = checklist.lines().find(|l| l.contains("plain")).unwrap();
    let date = line.rsplit(", ").next().unwrap();
    assert!(
        line.contains("— done by claude, ") && date.len() == 10 && !line.contains("claimed"),
        "{line}"
    );
    let show = text(&war(&root, &["show", &t]));
    assert!(show.contains("- [x] (claimed) plain"), "{show}");
    assert!(show.contains("- [x] (observed) checked"), "{show}");
    let list = text(&war(&root, &["tickets"]));
    assert!(list.contains("[ticks: 1 claimed, 1 observed]"), "{list}");

    // Refusal: a hand-written [signed] with nothing behind it reads claimed.
    let path = root.join(format!("docs/tickets/{t}/atoms/15-checklist.md"));
    let forged = std::fs::read_to_string(&path)
        .unwrap()
        .replace(line, &format!("{line} [signed]"));
    std::fs::write(&path, forged).unwrap();
    let show = json(&root, &["show", &t]);
    let ticked = show["result"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["text"] == "plain")
        .unwrap()
        .clone();
    assert_eq!(ticked["tick"]["level"], "claimed", "{ticked}");
    assert_eq!(ticked["tick"]["written"], "signed", "{ticked}");
    let human = text(&war(&root, &["show", &t]));
    assert!(human.contains("- [x] (claimed) plain"), "{human}");
    assert!(human.contains("marker is not believed"), "{human}");
}

#[test]
fn kpis_keep_best_latest_and_target_and_no_number_is_unknown() {
    let root = scratch("kpi");
    let t = json(&root, &["create", "Latency"])["result"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    std::fs::write(root.join("p95.sh"), "#!/bin/sh\necho \"$P95\"\n").unwrap();
    let added = json(
        &root,
        &[
            "add",
            &t,
            "--kpi",
            "p95",
            "--cmd",
            "sh p95.sh",
            "--direction",
            "min",
            "--target",
            "100",
        ],
    );
    assert_eq!(added["exit_code"], 0, "{added}");
    for v in ["120", "90", "99"] {
        let o = war_env(&root, &["--json", "kpi", "run", &t], &[("P95", v)], None);
        assert!(!o.stdout.is_empty(), "{}", text(&o));
    }
    let show = json(&root, &["show", &t]);
    let k = &show["result"]["checks"]["kpis"][0];
    assert_eq!(k["best"], 90.0, "{k}");
    assert_eq!(k["latest"], 99.0, "{k}");
    assert_eq!(k["target"], 100.0, "{k}");
    assert_eq!(k["runs"], 3, "{k}");
    let journal =
        std::fs::read_to_string(root.join(format!("docs/tickets/{t}/journal.jsonl"))).unwrap();
    assert_eq!(journal.matches("ticket.kpi_run").count(), 3);

    // A KPI that prints words is UNKNOWN, never a pass or a fail.
    json(
        &root,
        &[
            "add",
            &t,
            "--kpi",
            "words",
            "--cmd",
            "echo fast",
            "--direction",
            "max",
            "--target",
            "1",
        ],
    );
    let o = war_env(&root, &["--json", "kpi", "run", &t], &[("P95", "50")], None);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert!(rules(&v).contains(&"kpi.unknown".to_owned()), "{v}");
    let run = v["result"]["runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "words")
        .unwrap()
        .clone();
    assert_eq!(run["verdict"], "unknown", "{run}");
    assert!(run["value"].is_null(), "{run}");
    // And it holds a --check tick: UNKNOWN is not a pass.
    assert!(war(&root, &["claim", &t]).status.success());
    let held = war_env(
        &root,
        &["--json", "done", &t, "--check"],
        &[("P95", "50")],
        None,
    );
    let held: serde_json::Value = serde_json::from_slice(&held.stdout).unwrap();
    assert!(
        rules(&held).contains(&"ticket.check-unknown".to_owned()),
        "{held}"
    );
}

#[test]
fn an_independent_verdict_by_the_performer_is_refused_and_by_another_ticks() {
    let root = scratch("verify");
    let created = json(&root, &["create", "Reviewable", "-i", "the change"]);
    let t = created["result"]["id"].as_str().unwrap().to_owned();
    let item = created["result"]["items"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let target = format!("{t}/{item}");
    assert!(war(&root, &["claim", &target]).status.success());
    assert!(war(&root, &["done", &target]).status.success());
    let request = json(&root, &["verify", &target]);
    assert_eq!(request["result"]["performer"], "claude", "{request}");
    let response = |actor: &str| {
        format!(
            "schema = \"oh.war/tick-verification-response/v1\"\nticket = \"{t}\"\nitem = \"{item}\"\n\
             [verification]\nobligation = \"{target}\"\nperformer = \"claude\"\n\
             disposition = \"established\"\nevidence = \"read the diff, ran the tests\"\n\
             [verification.verifier]\nactor = \"{actor}\"\nkind = \"agent\"\n\
             [verification.verifier.independence]\nperformer_transcript_blind = true\n\
             performer_rationale_blind = true\nseparate_writable_workspace = true\n\
             cannot_modify_subject_artifacts = true\ncannot_modify_gate_definition = true\n\
             cannot_modify_gate_fixtures = true\nseparate_context_compilation = true\n\
             distinct_model_required = true\ndistinct_human_required = false\n"
        )
    };
    let file = root.join("response.toml");
    std::fs::write(&file, response("claude")).unwrap();
    let refused = json(
        &root,
        &["verify", &target, "--response", file.to_str().unwrap()],
    );
    assert!(
        rules(&refused).contains(&"tick.self-verification".to_owned()),
        "{refused}"
    );
    std::fs::write(&file, response("reviewer-b")).unwrap();
    let ok = json(
        &root,
        &["verify", &target, "--response", file.to_str().unwrap()],
    );
    assert_eq!(ok["exit_code"], 0, "{ok}");
    let show = text(&war(&root, &["show", &t]));
    assert!(
        show.contains("- [x] (independent: verified by reviewer-b) the change"),
        "{show}"
    );
}

#[test]
fn the_bridge_does_nothing_for_a_task_that_names_nothing() {
    let root = scratch("bridge");
    let created = json(&root, &["create", "Bridge", "-i", "one"]);
    let t = created["result"]["id"].as_str().unwrap().to_owned();
    let item = created["result"]["items"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let before =
        std::fs::read(root.join(format!("docs/tickets/{t}/atoms/15-checklist.md"))).unwrap();
    let unrelated = r#"{"hook_event_name":"TaskCompleted","task_id":"task-1","task_subject":"Implement user authentication"}"#;
    let o = war_env(
        &root,
        &[
            "--json",
            "bridge",
            "claude-tasks",
            "--event",
            "-",
            "--apply",
        ],
        &[],
        Some(unrelated),
    );
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["exit_code"], 0, "{v}");
    assert!(v["result"]["tasks"][0]["command"].is_null(), "{v}");
    let after =
        std::fs::read(root.join(format!("docs/tickets/{t}/atoms/15-checklist.md"))).unwrap();
    assert_eq!(before, after);
    // A task naming the item: proposed without --apply, ticked with it.
    let naming = format!(
        r#"{{"hook_event_name":"TaskCompleted","task_id":"task-2","task_subject":"Do {t}/{item}"}}"#
    );
    let o = war_env(
        &root,
        &["--json", "bridge", "claude-tasks", "--event", "-"],
        &[],
        Some(&naming),
    );
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(
        v["result"]["tasks"][0]["command"],
        format!("war done {t}/{item}"),
        "{v}"
    );
    assert_eq!(
        before,
        std::fs::read(root.join(format!("docs/tickets/{t}/atoms/15-checklist.md"))).unwrap()
    );
    let o = war_env(
        &root,
        &[
            "--json",
            "bridge",
            "claude-tasks",
            "--event",
            "-",
            "--apply",
        ],
        &[],
        Some(&naming),
    );
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["exit_code"], 0, "{v}");
    let checklist =
        std::fs::read_to_string(root.join(format!("docs/tickets/{t}/atoms/15-checklist.md")))
            .unwrap();
    assert!(checklist.contains("- [x] one"), "{checklist}");
}
