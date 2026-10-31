// SPDX-License-Identifier: Apache-2.0
//! The approved alpha.6 daily interface, through the real executable.
use std::process::Command;

fn war(root: &std::path::Path, args: &[&str]) -> serde_json::Value {
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["--root", root.to_str().unwrap(), "--json"])
        .args(args)
        .current_dir(root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .env_remove("OPENWARRANT_ACTOR")
        .env_remove("SSH_AUTH_SOCK")
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap_or_else(|_| {
        panic!(
            "{args:?}: {} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

#[test]
fn newly_created_work_is_one_expandable_markdown_source() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        war(root.path(), &["init", "--namespace", "NX"])["exit_code"],
        0
    );
    let created = war(
        root.path(),
        &[
            "create",
            "Reset passwords",
            "--body",
            "Keep existing accounts working.",
            "--item",
            "Reset endpoint",
        ],
    );
    assert_eq!(created["exit_code"], 0, "{created}");
    let id = created["result"]["id"].as_str().unwrap();
    let source = created["result"]["source"]
        .as_str()
        .expect("one source path is returned");
    let path = root.path().join(source);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("format: openwarrant/1.0"));
    assert!(text.contains("Keep existing accounts working."));
    assert!(text.contains("Reset endpoint"));
    assert!(
        !root
            .path()
            .join("docs/tickets")
            .join(id)
            .join("manifest.toml")
            .exists()
    );
    let added = war(root.path(), &["plan", "add", id, "Email delivery"]);
    assert_eq!(added["exit_code"], 0, "{added}");
    let edit = war(root.path(), &["plan", "edit", id, "--priority", "1"]);
    assert_eq!(edit["exit_code"], 0, "{edit}");
    let check = war(root.path(), &["plan", "add", id, "--test", "true"]);
    assert_eq!(check["exit_code"], 0, "{check}");
    let note = war(
        root.path(),
        &["plan", "note", id, "Keep this context for the next agent"],
    );
    assert_eq!(note["exit_code"], 0, "{note}");
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains(&format!("alias: {id}"))
    );
    assert!(
        !root.path().join("docs/authority/responses").exists(),
        "ordinary authoring needs no authority act"
    );
}

#[test]
fn start_delivers_complete_context_and_exact_dependency_text() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        war(root.path(), &["init", "--namespace", "CX"])["exit_code"],
        0
    );
    let dep = war(
        root.path(),
        &[
            "create",
            "Account contract",
            "--body",
            "Applicable rule: preserve all existing account identities.",
        ],
    );
    let dep_id = dep["result"]["id"].as_str().unwrap();
    assert_eq!(war(root.path(), &["done", dep_id])["exit_code"], 0);
    let task = war(
        root.path(),
        &[
            "create",
            "Reset API",
            "--body",
            "Keep the public reset contract compatible.",
            "--item",
            "Implement reset",
        ],
    );
    let id = task["result"]["id"].as_str().unwrap();
    let path = root.path().join(task["result"]["source"].as_str().unwrap());
    let text = std::fs::read_to_string(&path).unwrap().replacen(
        "\n---\n",
        &format!("\ndepends_on: {dep_id}\n---\n"),
        1,
    );
    std::fs::write(path, text).unwrap();
    let started = war(root.path(), &["start", id, "--as", "reset-builder"]);
    assert_eq!(started["exit_code"], 0, "{started}");
    let context = &started["result"]["context"];
    assert_eq!(context["schema"], "oh.war/task-context/v1", "{started}");
    assert!(
        context["projection"]["markdown"]
            .as_str()
            .unwrap()
            .contains("preserve all existing account identities")
    );
    assert!(
        context["projection"]["markdown"]
            .as_str()
            .unwrap()
            .contains("Keep the public reset contract compatible")
    );
    assert!(context["source_manifest"].as_array().unwrap().len() >= 2);
    assert!(
        context["selection_digest"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert_eq!(context["qualification_established"], false);
}

#[test]
fn single_source_milestone_cannot_skip_its_signed_minimum() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        war(root.path(), &["init", "--namespace", "MS"])["exit_code"],
        0
    );
    let made = war(
        root.path(),
        &[
            "create",
            "Reviewed delivery",
            "--item",
            "Review this outcome",
        ],
    );
    let id = made["result"]["id"].as_str().unwrap();
    let item = made["result"]["items"][0]["id"].as_str().unwrap();
    let target = format!("{id}/{item}");
    assert_eq!(
        war(root.path(), &["plan", "add", &target, "--min", "signed"])["exit_code"],
        0
    );
    assert_eq!(
        war(root.path(), &["plan", "add", id, "--test", "true"])["exit_code"],
        0
    );
    let result = war(root.path(), &["check", "run", id]);
    let source = root.path().join(made["result"]["source"].as_str().unwrap());
    assert!(
        std::fs::read_to_string(source)
            .unwrap()
            .contains("- [ ] Review this outcome"),
        "passing a command cannot clear a human-signature minimum: {result}"
    );
}

#[test]
fn six_daily_commands_and_grouped_compatibility() {
    let help = Command::new(env!("CARGO_BIN_EXE_war"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap();
    let section = text
        .split("Commands:\n")
        .nth(1)
        .unwrap()
        .split("More:")
        .next()
        .unwrap();
    let names: Vec<_> = section
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| *name != "help")
        .collect();
    assert_eq!(
        names,
        ["init", "create", "start", "done", "status", "check"]
    );
    for (group, member) in [
        ("view", "next"),
        ("view", "show"),
        ("admin", "claim"),
        ("plan", "add"),
        ("plan", "note"),
        ("plan", "edit"),
    ] {
        let canonical = Command::new(env!("CARGO_BIN_EXE_war"))
            .args([group, member, "--help"])
            .output()
            .unwrap();
        let old = Command::new(env!("CARGO_BIN_EXE_war"))
            .args([member, "--help"])
            .output()
            .unwrap();
        assert!(
            canonical.status.success(),
            "{group} {member}: {}",
            String::from_utf8_lossy(&canonical.stderr)
        );
        assert!(old.status.success(), "legacy {member}");
    }
}

#[test]
fn title_only_work_cannot_bypass_a_required_check() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        war(root.path(), &["init", "--namespace", "TC"])["exit_code"],
        0
    );
    let created = war(root.path(), &["create", "Preserve required checks"]);
    let id = created["result"]["id"].as_str().unwrap();
    assert_eq!(
        war(root.path(), &["plan", "add", id, "--test", "false"])["exit_code"],
        0
    );
    assert_eq!(war(root.path(), &["start", id])["exit_code"], 0);
    let done = war(root.path(), &["done", id]);
    assert_ne!(done["exit_code"], 0, "{done}");
    assert!(
        done["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "check.failed"),
        "{done}"
    );
    let shown = war(root.path(), &["view", "show", id]);
    assert_ne!(shown["result"]["state"], "done", "{shown}");
    let path = root
        .path()
        .join(created["result"]["source"].as_str().unwrap());
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("run: false"));
    std::fs::write(path, text.replace("run: false", "run: true")).unwrap();
    let done = war(root.path(), &["done", id]);
    assert_eq!(done["exit_code"], 0, "{done}");
    assert_eq!(done["result"]["level"], "observed", "{done}");
}

#[test]
fn single_source_checks_execute_multiword_commands_as_authored() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        war(root.path(), &["init", "--namespace", "CM"])["exit_code"],
        0
    );
    let created = war(
        root.path(),
        &["create", "Run a real command", "--item", "Deliver result"],
    );
    let id = created["result"]["id"].as_str().unwrap();
    assert_eq!(
        war(
            root.path(),
            &["plan", "add", id, "--test", "printf 'command passed\\n'"]
        )["exit_code"],
        0
    );
    assert_eq!(war(root.path(), &["start", id])["exit_code"], 0);
    let done = war(root.path(), &["done", id]);
    assert_eq!(done["exit_code"], 0, "{done}");
}
