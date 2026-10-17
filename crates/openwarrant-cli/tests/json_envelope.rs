// SPDX-License-Identifier: Apache-2.0
//! `--json` (SAS §76.4): every envelope has the frozen keys, the exit code in
//! the envelope equals the process's, and stdout is exactly one JSON value.
//!
//! Reads the repository's own corpus; compilation writes only disposable fixtures.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn war(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        // This checkout's .git is not a test's to write caches into.
        .env("OPENWARRANT_NO_CACHE", "1")
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("war runs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

const FROZEN_KEYS: [&str; 8] = [
    "command",
    "counts",
    "diagnostics",
    "exit_code",
    "notes",
    "schema",
    "verdict",
    "verdict_line",
];

#[test]
fn schema_discovery_keeps_toml_root_and_body_rules() {
    let root = std::env::temp_dir().join(format!("war-schema-header-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let invoke = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_war"))
            .env("OPENWARRANT_NO_PROJECTS", "1")
            .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
            .env("OPENWARRANT_NO_CACHE", "1")
            .args(args)
            .current_dir(&root)
            .output()
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        (output.status, value)
    };
    let (status, value) = invoke(&[
        "init",
        "--namespace",
        "SC",
        "--program",
        "Schema test",
        "--json",
    ]);
    assert!(status.success(), "{value}");
    let path = root.join("docs/warrants/SC-WAR-0001/extra.toml");
    for (source, newer) in [
        (
            "schema = \"oh.war/verification/v3\"\nvalue = 'ordinary'\n",
            false,
        ),
        (
            "# comment\n\"schema\" = 'oh.war/verification/v3' # comment\nbody = '''\nschema = \"oh.war/verification/v99\"\n'''\n",
            false,
        ),
        (
            "schema = \"oh.war/verification/v3\"\nschema = \"oh.war/verification/v99\"\n",
            false,
        ),
        (
            "schema = \"oh.war/verification/v99\"\nvalue = 'future'\n",
            true,
        ),
        ("schema = \"oh.war/verification/v99\"\nbroken = [\n", false),
        (
            "note = 'ordinary'\nschema = \"oh.war/verification/v99\"\n",
            true,
        ),
        ("schema = '''\noh.war/verification/v99\n'''\n", true),
        (
            "schema = \"another.family/v99\"\nvalue = 'outside the frozen families'\n",
            false,
        ),
    ] {
        std::fs::write(&path, source).unwrap();
        let (_, report) = invoke(&["check", "--json"]);
        let findings: Vec<_> = report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["rule"] == "compat.newer-record")
            .collect();
        assert_eq!(findings.len(), usize::from(newer), "{source}\n{report}");
        if newer {
            assert_eq!(findings[0]["severity"], "unknown");
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn init_emits_one_json_value_without_changing_the_created_configuration() {
    let root = std::env::temp_dir().join(format!(
        "ow-init-json-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let machine = root.join("machine");
    let human = root.join("human");
    std::fs::create_dir_all(&machine).unwrap();
    std::fs::create_dir_all(&human).unwrap();
    let run = |dir: &std::path::Path, json: bool| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_war"));
        cmd.env("OPENWARRANT_NO_PROJECTS", "1")
            .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
            .args([
                "init",
                "--namespace",
                "JS",
                "--name",
                "Transport",
                "--no-preset",
            ])
            .current_dir(dir);
        if json {
            cmd.arg("--json");
        }
        cmd.output().unwrap()
    };
    let accepted = run(&machine, true);
    let before = std::fs::read(machine.join("openwarrant.toml")).unwrap();
    let refused = run(&machine, true);
    let after = std::fs::read(machine.join("openwarrant.toml")).unwrap();
    let ordinary = run(&human, false);
    let human_config = std::fs::read(human.join("openwarrant.toml")).unwrap();
    std::fs::remove_dir_all(root).unwrap();

    assert!(accepted.status.success());
    let positive = envelope(&String::from_utf8_lossy(&accepted.stdout));
    assert_eq!(positive["command"], "init");
    assert_eq!(positive["exit_code"], 0);
    assert!(positive["notes"].as_array().unwrap().iter().any(|n| {
        n.as_str()
            .is_some_and(|s| s.contains("initialized Transport"))
    }));
    assert!(!refused.status.success());
    let negative = envelope(&String::from_utf8_lossy(&refused.stdout));
    assert_eq!(negative["exit_code"], refused.status.code().unwrap());
    assert_eq!(
        before, after,
        "refused reinitialization preserves the config"
    );
    assert!(ordinary.status.success());
    assert!(String::from_utf8_lossy(&ordinary.stdout).contains("initialized Transport"));
    assert_eq!(before, human_config, "format changes no setup policy");
}

#[test]
fn init_json_covers_presets_and_the_program_scaffold() {
    let root = std::env::temp_dir().join(format!(
        "ow-init-json-options-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let cases: &[(&str, &[&str])] = &[
        ("plain", &[]),
        ("team", &["--team"]),
        ("regulated", &["--regulated"]),
        ("program", &["--program", "Transport", "--namespace", "JS"]),
        ("derived", &["--program", "Transport"]),
    ];
    let mut outputs = Vec::new();
    for (name, flags) in cases {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_war"))
            .env("OPENWARRANT_NO_PROJECTS", "1")
            .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
            .args(["init", "--json"])
            .args(*flags)
            .current_dir(&dir)
            .output()
            .unwrap();
        outputs.push((
            *name,
            out,
            std::fs::read_to_string(dir.join("openwarrant.toml")).unwrap(),
        ));
    }
    std::fs::remove_dir_all(root).unwrap();
    for (name, out, config) in outputs {
        assert!(
            out.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let v = envelope(&String::from_utf8_lossy(&out.stdout));
        assert_eq!(v["command"], "init");
        assert_eq!(v["exit_code"], 0);
        let notes = v["notes"].as_array().unwrap();
        if matches!(name, "program" | "derived") {
            assert!(notes.iter().any(|n| {
                n.as_str()
                    .is_some_and(|s| s.contains("scaffolded Transport"))
            }));
            assert!(!config.contains("[preset]"));
        } else {
            let expected = if name == "plain" { "vibe" } else { name };
            let config: toml::Value = toml::from_str(&config).unwrap();
            assert_eq!(config["preset"]["name"].as_str(), Some(expected));
        }
    }
}

#[test]
fn fixture_eval_keeps_setup_output_inside_the_child_process() {
    let root = std::env::temp_dir().join(format!(
        "ow-eval-json-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let run = |task: &str, name: &str| {
        Command::new(env!("CARGO_BIN_EXE_war"))
            .env("OPENWARRANT_NO_CACHE", "1")
            .env("OPENWARRANT_NO_PROJECTS", "1")
            .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
            .env("TMPDIR", &root)
            .args(["evidence", "eval", "run", "--json", "--task", task])
            .arg("--drafter")
            .arg(repo_root().join("evals/fixtures/fixture-agent.sh"))
            .arg("--verifier")
            .arg(repo_root().join("conformance/fixtures/verifier/establishes-all.sh"))
            .arg("--out")
            .arg(root.join(name))
            .current_dir(repo_root())
            .output()
            .unwrap()
    };
    let accepted = run("run-01-echo", "accepted.json");
    let result = std::fs::read(root.join("accepted.json"));
    let refused = run("no-such-eval-task", "refused.json");
    let refused_file_exists = root.join("refused.json").exists();
    // Clean up before assertions so the regression's red run leaves no scratch.
    std::fs::remove_dir_all(root).unwrap();

    assert!(
        accepted.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&accepted.stdout),
        String::from_utf8_lossy(&accepted.stderr)
    );
    let positive = envelope(&String::from_utf8_lossy(&accepted.stdout));
    assert_eq!(positive["exit_code"], 0);
    let result: serde_json::Value = serde_json::from_slice(&result.unwrap()).unwrap();
    assert_eq!(result["tasks"][0]["score"], "would_satisfy");
    assert_eq!(
        result["tasks"][0]["requirements_unmet_by_the_loop"],
        serde_json::json!([])
    );
    assert!(!refused.status.success());
    let negative = envelope(&String::from_utf8_lossy(&refused.stdout));
    assert_eq!(negative["exit_code"], refused.status.code().unwrap());
    assert!(!refused_file_exists);
}

fn envelope(stdout: &str) -> serde_json::Value {
    let v: serde_json::Value = serde_json::from_str(stdout)
        .unwrap_or_else(|e| panic!("stdout is not one JSON value: {e}\n{stdout}"));
    assert_eq!(v["schema"], "oh.war/report/v1");
    let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    keys.retain(|k| *k != "result");
    keys.sort_unstable();
    assert_eq!(keys, FROZEN_KEYS, "envelope keys are frozen at 1.0");
    v
}

#[test]
fn check_emits_an_envelope_whose_exit_code_is_the_process_exit_code() {
    let (code, out, _) = war(&["--json", "check", "OW-WAR-0001"]);
    let v = envelope(&out);
    assert_eq!(v["command"], "check");
    assert_eq!(v["exit_code"].as_i64().unwrap(), i64::from(code));
    assert!(v["diagnostics"].as_array().unwrap().len() > 3);
    assert!(v["counts"]["pass"].as_u64().unwrap() > 0);
}

#[test]
fn the_flag_works_after_the_subcommand_too() {
    let (_, a, _) = war(&["--json", "check", "OW-WAR-0001"]);
    let (_, b, _) = war(&["check", "OW-WAR-0001", "--json"]);
    assert_eq!(a, b, "global flag; position does not matter");
}

#[test]
fn a_request_document_rides_inside_result_with_its_own_schema() {
    let (code, out, _) = war(&["--json", "authorize", "OW-WAR-0001"]);
    let v = envelope(&out);
    assert_eq!(v["command"], "authorize.request");
    assert_eq!(v["result"]["schema"], "oh.war/authorization-request/v1");
    assert_eq!(v["result"]["warrant"], "OW-WAR-0001");
    assert!(v["result"]["contract_digest"].as_str().unwrap().len() == 64);
    assert_eq!(code, 0);
}

#[test]
fn the_error_path_is_still_json_on_stdout_and_exits_one() {
    let (code, out, err) = war(&["--json", "show", "OW-WAR-9999"]);
    let v = envelope(&out);
    assert_eq!(v["command"], "error");
    assert_eq!(v["diagnostics"][0]["rule"], "cli.error");
    assert_eq!(v["verdict"], "not_ready");
    assert_eq!(
        code, 1,
        "a diagnostic error is exit 1, not the envelope's 2"
    );
    assert!(
        err.is_empty() || !err.contains("error:"),
        "nothing human on stderr under --json: {err}"
    );
}

#[test]
fn the_corpus_projection_is_the_result_payload_verbatim() {
    let (_, out, _) = war(&["--json", "status"]);
    let mut v = envelope(&out);
    assert_eq!(v["command"], "status");
    assert_eq!(v["result"]["schema"], "oh.war/corpus-status/v1");
    // OW-WAR-0148 M17: the score rides beside the projection, computed on
    // read, and never enters the committed file (it moves with every commit).
    let score = v["result"]
        .as_object_mut()
        .and_then(|o| o.remove("compliance"))
        .expect("war status --json carries the score");
    assert_eq!(score["schema"], "oh.war/score/v1");
    let committed =
        std::fs::read_to_string(repo_root().join("docs/warrants/generated/CORPUS_STATUS.json"))
            .expect("committed projection");
    let committed: serde_json::Value = serde_json::from_str(&committed).unwrap();
    // Keep the exact equality check, but do not dump the entire corpus twice
    // on failure: its gate-input inventories can overwhelm the CI log.
    assert!(
        v["result"] == committed,
        "the envelope carries the committed projection; run war admin compile after staging source files"
    );
}

#[test]
fn without_the_flag_stdout_never_begins_with_a_brace() {
    let (_, out, _) = war(&["check", "OW-WAR-0001"]);
    assert!(
        !out.trim_start().starts_with('{'),
        "human mode leaks JSON: {out}"
    );
}

#[test]
fn compile_emits_one_envelope_and_reports_skipped_warrants() {
    fn copy(from: &std::path::Path, to: &std::path::Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }
    let root = std::env::temp_dir().join(format!(
        "ow-compile-json-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    copy(
        &repo_root().join("conformance/fixtures/inbox/repository"),
        &root,
    );
    // Typed document projections must obey the same one-envelope transport as
    // Warrant projections. The old inbox-only fixture never exercised them.
    copy(&repo_root().join("profiles"), &root.join("profiles"));
    let documents = root.join("docs/records/password-reset");
    copy(&repo_root().join("docs/records/password-reset"), &documents);
    std::fs::remove_dir_all(documents.join("generated")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_war"))
        // This checkout's .git is not a test's to write caches into.
        .env("OPENWARRANT_NO_CACHE", "1")
        .args(["compile", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();
    let v = envelope(&String::from_utf8_lossy(&out.stdout));
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(v["command"], "compile");
    assert_eq!(v["exit_code"], 0);
    assert!(out.stderr.is_empty(), "machine mode leaked terminal output");
    assert!(v["result"]["written"].as_u64().unwrap() > 0);
    assert!(v["result"]["skipped"].as_array().is_some());
    for name in ["prd", "architecture", "test-plan", "agent-packet"] {
        assert!(documents.join(format!("generated/{name}.md")).is_file());
    }
    let human = Command::new(env!("CARGO_BIN_EXE_war"))
        // This checkout's .git is not a test's to write caches into.
        .env("OPENWARRANT_NO_CACHE", "1")
        .arg("compile")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(human.status.success());
    assert!(
        String::from_utf8_lossy(&human.stdout)
            .contains("compiled docs/records/password-reset/generated/prd.md"),
        "terminal users still receive document progress"
    );
    // A valid config with an invalid authored Warrant must disclose the skip.
    let manifest = root.join("docs/warrants/IX-WAR-0001/manifest.toml");
    assert!(manifest.exists());
    let source = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        &manifest,
        source.replacen("ordinal = 20", "ordinal = 10", 1),
    )
    .unwrap();
    let rejected = Command::new(env!("CARGO_BIN_EXE_war"))
        // This checkout's .git is not a test's to write caches into.
        .env("OPENWARRANT_NO_CACHE", "1")
        .args(["compile", "IX-WAR-0001", "--json"])
        .current_dir(&root)
        .output()
        .unwrap();
    let refusal = envelope(&String::from_utf8_lossy(&rejected.stdout));
    assert_eq!(
        refusal["result"]["skipped"],
        serde_json::json!(["IX-WAR-0001"])
    );
    assert_eq!(refusal["result"]["written"], 0);
    assert!(rejected.stderr.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}
