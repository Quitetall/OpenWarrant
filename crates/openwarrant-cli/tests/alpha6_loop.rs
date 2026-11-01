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

#[test]
fn native_completion_refreshes_the_same_index_as_full_compilation() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        war(root.path(), &["init", "--namespace", "NI"])["exit_code"],
        0
    );
    let created = war(
        root.path(),
        &[
            "create",
            "Keep the context index current",
            "--item",
            "Deliver result",
        ],
    );
    let id = created["result"]["id"].as_str().unwrap();
    assert_eq!(war(root.path(), &["admin", "compile"])["exit_code"], 0);
    assert_eq!(war(root.path(), &["start", id])["exit_code"], 0);
    assert_eq!(war(root.path(), &["done", id])["exit_code"], 0);
    let path = root.path().join("docs/generated/llms.txt");
    let incremental = std::fs::read_to_string(&path).unwrap();
    assert!(
        incremental.contains(&format!("{id} · ticket · done")),
        "{incremental}"
    );
    assert_eq!(war(root.path(), &["admin", "compile"])["exit_code"], 0);
    assert_eq!(std::fs::read_to_string(path).unwrap(), incremental);
}

#[test]
fn native_label_lists_can_be_created_replaced_and_cleared() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        war(root.path(), &["init", "--namespace", "NL"])["exit_code"],
        0
    );
    let created = war(
        root.path(),
        &[
            "create",
            "Labelled work",
            "--label",
            "api",
            "--label",
            "accounts",
        ],
    );
    assert_eq!(created["exit_code"], 0, "{created}");
    let id = created["result"]["id"].as_str().unwrap();
    let path = root
        .path()
        .join(created["result"]["source"].as_str().unwrap());
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("labels:\n  - ")
    );
    let edited = war(
        root.path(),
        &[
            "plan",
            "edit",
            id,
            "--unlabel",
            "api",
            "--unlabel",
            "accounts",
            "--label",
            "storage",
        ],
    );
    assert_eq!(edited["exit_code"], 0, "{edited}");
    let parsed = openwarrant_core::format::parse(&std::fs::read(&path).unwrap());
    assert_eq!(
        parsed.fields["labels"],
        openwarrant_core::format::FieldValue::Many(vec!["storage".into()])
    );
    let cleared = war(root.path(), &["plan", "edit", id, "--unlabel", "storage"]);
    assert_eq!(cleared["exit_code"], 0, "{cleared}");
    let parsed = openwarrant_core::format::parse(&std::fs::read(path).unwrap());
    assert!(!parsed.fields.contains_key("labels"));
    assert!(!parsed.refused());
}

#[test]
fn a_required_pass_from_an_old_code_commit_does_not_complete_new_work() {
    let root = tempfile::tempdir().unwrap();
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
            .current_dir(root.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    git(&["init", "-q"]);
    assert_eq!(
        war(root.path(), &["init", "--namespace", "SB"])["exit_code"],
        0
    );
    let created = war(
        root.path(),
        &[
            "create",
            "Require current source observations",
            "--item",
            "Deliver",
        ],
    );
    let id = created["result"]["id"].as_str().unwrap();
    for name in ["first", "second"] {
        assert_eq!(
            war(
                root.path(),
                &[
                    "plan",
                    "add",
                    id,
                    "--test",
                    "test -s src/code.txt",
                    "--name",
                    name
                ]
            )["exit_code"],
            0
        );
    }
    std::fs::create_dir(root.path().join("src")).unwrap();
    std::fs::write(root.path().join("src/code.txt"), "version one").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-qm", "baseline"]);
    assert_eq!(
        war(root.path(), &["check", "run", &format!("{id}/CHK-first")])["exit_code"],
        0
    );
    let source = root
        .path()
        .join(created["result"]["source"].as_str().unwrap());
    std::fs::write(root.path().join("src/code.txt"), "version two").unwrap();
    git(&["add", "src/code.txt"]);
    git(&["commit", "-qm", "new source"]);
    assert_eq!(
        war(root.path(), &["check", "run", &format!("{id}/CHK-second")])["exit_code"],
        0
    );
    assert!(
        std::fs::read_to_string(&source)
            .unwrap()
            .contains("- [ ] Deliver")
    );
    assert_eq!(
        war(root.path(), &["check", "run", &format!("{id}/CHK-first")])["exit_code"],
        0
    );
    assert!(
        std::fs::read_to_string(&source)
            .unwrap()
            .contains("- [x] Deliver")
    );
}

#[test]
fn dirty_and_untracked_source_changes_require_fresh_required_results() {
    for mode in ["tracked", "untracked", "non-git"] {
        let root = tempfile::tempdir().unwrap();
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
                .current_dir(root.path())
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
        };
        if mode != "non-git" {
            git(&["init", "-q"]);
        }
        assert_eq!(
            war(root.path(), &["init", "--namespace", "DB"])["exit_code"],
            0
        );
        let created = war(
            root.path(),
            &[
                "create",
                "Require current dirty source",
                "--item",
                "Deliver",
            ],
        );
        let id = created["result"]["id"].as_str().unwrap();
        for name in ["first", "second"] {
            assert_eq!(
                war(
                    root.path(),
                    &[
                        "plan",
                        "add",
                        id,
                        "--test",
                        "test -s src/code.txt",
                        "--name",
                        name
                    ]
                )["exit_code"],
                0
            );
        }
        std::fs::create_dir(root.path().join("src")).unwrap();
        if mode != "untracked" {
            std::fs::write(root.path().join("src/code.txt"), "version one").unwrap();
        }
        if mode != "non-git" {
            git(&["add", "-A"]);
            git(&["commit", "-qm", "baseline"]);
        }
        if mode == "untracked" {
            std::fs::write(root.path().join("src/code.txt"), "version one").unwrap();
        }
        let first = war(root.path(), &["check", "run", &format!("{id}/CHK-first")]);
        assert_eq!(first["exit_code"], 0, "{mode}: {first}");
        std::fs::write(root.path().join("src/code.txt"), "uncommitted version two").unwrap();
        let second = war(root.path(), &["check", "run", &format!("{id}/CHK-second")]);
        assert_eq!(second["exit_code"], 0, "{mode}: {second}");
        let source = root
            .path()
            .join(created["result"]["source"].as_str().unwrap());
        assert!(
            std::fs::read_to_string(&source)
                .unwrap()
                .contains("- [ ] Deliver"),
            "{mode}: an old required pass must not complete edited source"
        );
        let fresh = war(root.path(), &["check", "run", &format!("{id}/CHK-first")]);
        assert_eq!(fresh["exit_code"], 0, "{mode}: {fresh}");
        assert!(
            std::fs::read_to_string(source)
                .unwrap()
                .contains("- [x] Deliver"),
            "{mode}: fresh required passes complete work"
        );
    }
}

#[test]
fn a_check_that_changes_its_source_is_unknown_and_duplicate_events_do_not_reuse_old_passes() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        war(root.path(), &["init", "--namespace", "MC"])["exit_code"],
        0
    );
    let created = war(
        root.path(),
        &[
            "create",
            "Keep source observations honest",
            "--item",
            "Deliver",
        ],
    );
    let id = created["result"]["id"].as_str().unwrap();
    std::fs::write(root.path().join("source.txt"), "old source").unwrap();
    assert_eq!(
        war(
            root.path(),
            &[
                "plan",
                "add",
                id,
                "--test",
                "printf new > source.txt",
                "--name",
                "mutation"
            ]
        )["exit_code"],
        0
    );
    let result = war(root.path(), &["check", "run", id]);
    assert_eq!(
        result["result"]["firing"]["runs"][0]["verdict"], "UNKNOWN",
        "{result}"
    );
    assert_eq!(
        result["result"]["firing"]["runs"][0]["reason"], "source-changed-during-check",
        "{result}"
    );
    let source = root
        .path()
        .join(created["result"]["source"].as_str().unwrap());
    assert!(
        std::fs::read_to_string(&source)
            .unwrap()
            .contains("- [ ] Deliver")
    );
    let text = std::fs::read_to_string(&source).unwrap();
    std::fs::write(
        &source,
        text.replace("run: printf new > source.txt", "run: true\non: ci:fixture")
            .replace("on: done\n", ""),
    )
    .unwrap();
    let first = war(
        root.path(),
        &["check", "fire", "ci:fixture", "--subject", "build-1"],
    );
    assert_eq!(
        first["result"]["firing"]["runs"][0]["verdict"], "PASS",
        "{first}"
    );
    std::fs::write(root.path().join("source.txt"), "changed after event").unwrap();
    let duplicate = war(
        root.path(),
        &["check", "fire", "ci:fixture", "--subject", "build-1"],
    );
    assert_eq!(
        duplicate["result"]["firing"]["runs"][0]["verdict"], "UNKNOWN",
        "{duplicate}"
    );
    assert_eq!(duplicate["result"]["firing"]["runs"][0]["recorded"], false);
    assert_eq!(
        duplicate["result"]["firing"]["runs"][0]["reason"],
        "stale-source-basis"
    );
}

#[test]
fn a_whole_outcome_completes_only_after_all_required_current_checks() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        war(root.path(), &["init", "--namespace", "WA"])["exit_code"],
        0
    );
    let created = war(root.path(), &["create", "Finish a whole bounded outcome"]);
    let id = created["result"]["id"].as_str().unwrap();
    for name in ["first", "second"] {
        assert_eq!(
            war(
                root.path(),
                &["plan", "add", id, "--test", "true", "--name", name]
            )["exit_code"],
            0
        );
    }
    assert_eq!(
        war(root.path(), &["check", "run", &format!("{id}/CHK-first")])["exit_code"],
        0
    );
    let pending = war(root.path(), &["view", "show", id]);
    assert_eq!(pending["result"]["ticket"]["state"], "open", "{pending}");
    assert_eq!(
        war(root.path(), &["check", "run", &format!("{id}/CHK-second")])["exit_code"],
        0
    );
    let completed = war(root.path(), &["view", "show", id]);
    assert_eq!(
        completed["result"]["ticket"]["state"], "done",
        "{completed}"
    );
    let journal = root.path().join(format!("docs/tickets/{id}/journal.jsonl"));
    let before = std::fs::read(&journal).unwrap();
    assert_eq!(war(root.path(), &["check", "evaluate", id])["exit_code"], 0);
    assert_eq!(
        std::fs::read(journal).unwrap(),
        before,
        "repeat evaluation is not another completion"
    );
    assert!(
        !std::fs::read_to_string(
            root.path()
                .join(created["result"]["source"].as_str().unwrap())
        )
        .unwrap()
        .contains("- ["),
        "whole progress needs no synthetic checklist item"
    );
}

#[test]
fn watched_root_regressions_reopen_unaccepted_work_unless_reporting_only() {
    for report_only in [false, true] {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            war(root.path(), &["init", "--namespace", "WR"])["exit_code"],
            0
        );
        let created = war(root.path(), &["create", "Watch one complete outcome"]);
        let id = created["result"]["id"].as_str().unwrap();
        assert_eq!(
            war(
                root.path(),
                &["plan", "add", id, "--test", "test -f current.flag"]
            )["exit_code"],
            0
        );
        let source = root
            .path()
            .join(created["result"]["source"].as_str().unwrap());
        let text = std::fs::read_to_string(&source).unwrap().replace(
            "on: done",
            if report_only {
                "on: ci:build\nreopen: false"
            } else {
                "on: ci:build"
            },
        );
        std::fs::write(&source, &text).unwrap();
        std::fs::write(root.path().join("current.flag"), "available").unwrap();
        let pass = war(
            root.path(),
            &["check", "fire", "ci:build", "--subject", "build-1"],
        );
        assert_eq!(pass["exit_code"], 0, "{pass}");
        assert_eq!(
            war(root.path(), &["view", "show", id])["result"]["ticket"]["state"],
            "done"
        );
        std::fs::remove_file(root.path().join("current.flag")).unwrap();
        let fail = war(
            root.path(),
            &["check", "fire", "ci:build", "--subject", "build-2"],
        );
        assert_ne!(fail["exit_code"], 0, "{fail}");
        assert_eq!(
            war(root.path(), &["view", "show", id])["result"]["ticket"]["state"],
            if report_only { "done" } else { "open" }
        );
        assert_eq!(
            std::fs::read_to_string(&source).unwrap(),
            text,
            "progress remains in append-only history"
        );
        let events =
            std::fs::read_to_string(root.path().join(format!("docs/tickets/{id}/journal.jsonl")))
                .unwrap();
        assert_eq!(events.matches("\"type\":\"work.item_done\"").count(), 1);
        assert_eq!(
            events.matches("\"type\":\"work.item_reopened\"").count(),
            usize::from(!report_only)
        );
    }
}

#[test]
fn prior_signoff_history_is_preserved_with_verified_or_missing_evidence() {
    for verified in [false, true] {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            war(root.path(), &["init", "--namespace", "RP"])["exit_code"],
            0
        );
        let created = war(
            root.path(),
            &[
                "create",
                "Retain an earlier signed result",
                "--item",
                "Deliver",
            ],
        );
        let id = created["result"]["id"].as_str().unwrap();
        let item = created["result"]["items"][0]["id"].as_str().unwrap();
        assert_eq!(
            war(
                root.path(),
                &["plan", "add", id, "--test", "test -f current.flag"]
            )["exit_code"],
            0
        );
        let source = root
            .path()
            .join(created["result"]["source"].as_str().unwrap());
        let text = std::fs::read_to_string(&source)
            .unwrap()
            .replace("on: done", "on: ci:build");
        std::fs::write(&source, text).unwrap();
        std::fs::write(root.path().join("current.flag"), "available").unwrap();
        assert_eq!(
            war(
                root.path(),
                &["check", "fire", "ci:build", "--subject", "first"]
            )["exit_code"],
            0
        );
        // Synthetic retained metadata with deliberately missing evidence. This is
        // not a signature, human consent, enrollment, or a project assurance act.
        let repo = openwarrant_cli::repo::Repository::open(
            camino::Utf8PathBuf::from_path_buf(root.path().to_owned()).unwrap(),
        )
        .unwrap();
        let store = openwarrant_cli::ticket::Store::open(&repo, None).unwrap();
        let old = store.load(&store.dir.join(id)).unwrap();
        let digest = openwarrant_cli::ticket::ladder::statement_digest(&old, item, "Deliver");
        let subject = openwarrant_cli::ticket::ladder::signoff_subject(&old, item);
        let response = openwarrant_cli::authority_check::response_path(
            &repo,
            openwarrant_cli::authority_check::Act::SignOff,
            &subject,
        );
        let response_relative = response.strip_prefix(&repo.root).unwrap().to_string();
        if verified {
            // Real cryptography over synthetic fixture data only. This does not
            // claim operator presence or grant authority in the actual project.
            let keys = tempfile::tempdir().unwrap();
            let key = keys.path().join("fixture-key");
            let generated = Command::new("ssh-keygen")
                .args(["-q", "-t", "ed25519", "-N", "", "-f"])
                .arg(&key)
                .output()
                .unwrap();
            assert!(
                generated.status.success(),
                "{}",
                String::from_utf8_lossy(&generated.stderr)
            );
            std::fs::create_dir_all(root.path().join("docs/authority")).unwrap();
            std::fs::write(root.path().join("docs/authority/roles.toml"), "[[assignment]]\nactor = \"missing-evidence-fixture\"\nactor_kind = \"human\"\nroles = [\"resolver\"]\nassigned_by = \"synthetic-test-bootstrap\"\neffective_time = \"2026-01-01T00:00:00Z\"\nssh_principal = \"missing-evidence-fixture\"\n").unwrap();
            let public = std::fs::read_to_string(key.with_extension("pub")).unwrap();
            std::fs::write(
                root.path().join("docs/authority/allowed_signers"),
                format!("missing-evidence-fixture {public}"),
            )
            .unwrap();
            std::fs::create_dir_all(response.parent().unwrap()).unwrap();
            let signed_body = format!(
                "schema = \"oh.war/tick-signoff-response/v1\"\nticket = \"{id}\"\nticket_uuid = \"{}\"\nitem = \"{item}\"\ntext = \"Deliver\"\nsha256 = \"{digest}\"\nperformer = \"claude\"\nsigned_by = \"missing-evidence-fixture\"\nsigned_at = \"2026-10-31T00:00:00Z\"\nmeaning = \"Synthetic test signature; no operator consent asserted\"\n",
                old.manifest.uuid
            );
            std::fs::write(&response, signed_body).unwrap();
            let signed = Command::new("ssh-keygen")
                .args(["-Y", "sign", "-n", "oh.war/response", "-f"])
                .arg(key)
                .arg(&response)
                .output()
                .unwrap();
            assert!(
                signed.status.success(),
                "{}",
                String::from_utf8_lossy(&signed.stderr)
            );
        }
        openwarrant_cli::journal_cmd::record(&old.dir, &old.manifest.uuid, "work.tick_signed", "fixture-observer", &serde_json::json!({"item":item,"signer":"missing-evidence-fixture","response":response_relative,"digest":digest}).to_string()).unwrap();
        let text = std::fs::read_to_string(&source)
            .unwrap()
            .replace("[observed]", "[signed]");
        std::fs::write(&source, &text).unwrap();
        let shown = war(root.path(), &["view", "show", id]);
        assert_eq!(
            shown["result"]["items"][0]["tick"]["level"],
            if verified { "signed" } else { "observed" },
            "{shown}"
        );
        std::fs::remove_file(root.path().join("current.flag")).unwrap();
        let failed = war(
            root.path(),
            &["check", "fire", "ci:build", "--subject", "regression"],
        );
        assert_ne!(failed["exit_code"], 0, "{failed}");
        assert_eq!(
            std::fs::read_to_string(&source).unwrap(),
            text,
            "uncertain prior history is preserved"
        );
        let journal =
            std::fs::read_to_string(root.path().join(format!("docs/tickets/{id}/journal.jsonl")))
                .unwrap();
        let proposal = journal
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .find(|event| event["type"] == "work.repair_proposed")
            .expect("linked repair proposal");
        assert_eq!(
            proposal["payload"]["acceptance_status"],
            if verified {
                "verified-legacy-signoff"
            } else {
                "unestablished"
            }
        );
        assert_eq!(proposal["payload"]["qualification_established"], false);
        assert!(proposal["payload"]["accepted_code_revision"].is_null());
        let repair = proposal["payload"]["repair"].as_str().unwrap();
        let draft = root.path().join(format!("docs/tickets/{repair}.md"));
        let repair_text = std::fs::read_to_string(&draft).unwrap();
        assert!(
            repair_text.contains(&format!("x.repairs {id}/{item}")),
            "{repair_text}"
        );
        assert!(
            repair_text.contains("test -f current.flag"),
            "the original required condition remains present"
        );
        let guidance = "Keep this reviewer's repair guidance.";
        assert_eq!(
            war(root.path(), &["plan", "note", repair, guidance])["exit_code"],
            0
        );
        let before_repeat = std::fs::read(&draft).unwrap();
        let _ = war(
            root.path(),
            &["check", "fire", "ci:build", "--subject", "regression-again"],
        );
        assert_eq!(
            std::fs::read(&draft).unwrap(),
            before_repeat,
            "a repeated failure does not overwrite the repair draft"
        );
        let history =
            std::fs::read_to_string(root.path().join(format!("docs/tickets/{id}/journal.jsonl")))
                .unwrap();
        assert_eq!(
            history.matches("\"type\":\"work.repair_proposed\"").count(),
            1,
            "one prior result has one linked proposal"
        );
        let started = war(root.path(), &["start", repair]);
        assert_eq!(started["exit_code"], 0, "{started}");
        let context = started["result"]["context"]["projection"]["markdown"]
            .as_str()
            .unwrap();
        assert!(
            context.contains("Retain an earlier signed result") && context.contains(guidance),
            "original={}, notes={}, sources={}",
            context.contains("Retain an earlier signed result"),
            context.contains(guidance),
            started["result"]["context"]["source_manifest"]
        );
    }
}
