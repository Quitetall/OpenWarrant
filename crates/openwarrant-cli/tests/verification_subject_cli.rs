// SPDX-License-Identifier: Apache-2.0
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        fn copy(from: &std::path::Path, to: &std::path::Path) {
            fs::create_dir_all(to).unwrap();
            for entry in fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                let target = to.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copy(&entry.path(), &target);
                } else {
                    fs::copy(entry.path(), target).unwrap();
                }
            }
        }
        let root = std::env::temp_dir().join(format!(
            "ow-review-subject-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        copy(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../conformance/fixtures/inbox/repository"),
            &root,
        );
        Self(root)
    }
    fn run(&self, args: &[&str]) -> serde_json::Value {
        // Transport responses are outside the source tree being reviewed.
        let routed: Vec<String> = args
            .iter()
            .map(|arg| {
                if *arg == "response.toml" {
                    self.0
                        .with_extension("response.toml")
                        .to_string_lossy()
                        .into_owned()
                } else {
                    (*arg).to_owned()
                }
            })
            .collect();
        let output = Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(&self.0)
            .args(routed)
            .env("OPENWARRANT_NO_PROJECTS", "1")
            .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
            .env("CLAUDE_BIN", self.0.join("fixture-claude"))
            .output()
            .unwrap();
        serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
            panic!(
                "{} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        })
    }
    fn bound_response(&self) {
        self.response();
        let request = self.run(&[
            "verify",
            "IX-WAR-0003",
            "--performer",
            "fixture-performer",
            "--json",
        ]);
        assert_eq!(request["exit_code"], 0, "{request}");
        let path = self.0.with_extension("response.toml");
        let mut response: toml::Value =
            toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        response.as_table_mut().unwrap().insert(
            "reviewed_subject".to_owned(),
            toml::Value::try_from(&request["result"]["reviewed_subject"]).unwrap(),
        );
        fs::write(path, toml::to_string(&response).unwrap()).unwrap();
    }
    fn review_state(&self) -> Vec<Vec<u8>> {
        [
            "verifications/OBL-001.toml",
            "verifications/OBL-002.toml",
            "journal.jsonl",
        ]
        .iter()
        .map(|p| fs::read(self.0.join("docs/warrants/IX-WAR-0003").join(p)).unwrap())
        .collect()
    }
    fn response(&self) {
        let mut text = String::from(
            "schema = \"oh.war/verification-response/v1\"\nwarrant = \"IX-WAR-0003\"\n",
        );
        for id in ["OBL-001", "OBL-002"] {
            text.push_str(&format!(
                r#"
[[verifications]]
obligation = "{id}"
disposition = "established"
evidence = "synthetic public-seam fixture, not production assurance"
performer = "fixture-performer"
[verifications.verifier]
actor = "fixture-independent-verifier"
kind = "service"
[verifications.verifier.independence]
performer_transcript_blind = true
performer_rationale_blind = true
separate_writable_workspace = true
cannot_modify_subject_artifacts = true
cannot_modify_gate_definition = true
cannot_modify_gate_fixtures = true
separate_context_compilation = true
distinct_model_required = false
distinct_human_required = false
"#
            ));
        }
        fs::write(self.0.with_extension("response.toml"), text).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
        let _ = fs::remove_file(self.0.with_extension("response.toml"));
    }
}

#[test]
fn changed_requirement_cannot_reuse_an_unbound_old_verdict() {
    let fixture = Fixture::new();
    fixture.response();
    fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert!(
        fixture
            .0
            .join("docs/warrants/IX-WAR-0003/verifications/OBL-001.toml")
            .exists(),
        "historical record must be preserved"
    );
    let assurance = fixture
        .0
        .join("docs/warrants/IX-WAR-0003/atoms/60-assurance.md");
    let original = fs::read_to_string(&assurance).unwrap();
    assert!(original.contains("the SAS is accepted and pinned"));
    fs::write(
        &assurance,
        original.replace(
            "the SAS is accepted and pinned",
            "the new architecture requirement is reviewed",
        ),
    )
    .unwrap();
    let report = fixture.run(&["prepare", "IX-WAR-0003", "--dry-run", "--json"]);
    let steps = report["result"]["warrants"][0]["steps"].as_array().unwrap();
    let verify = steps.iter().find(|s| s["step"] == "verify").unwrap();
    assert_eq!(
        verify["outcome"], "planned",
        "old verification does not qualify a changed requirement: {report}"
    );
}

#[test]
fn progress_preserves_unbound_history_without_claiming_establishment() {
    let fixture = Fixture::new();
    fixture.response();
    fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    let report = fixture.run(&["status", "--json"]);
    let warrant = report["result"]["warrants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["alias"] == "IX-WAR-0003")
        .unwrap();
    assert_eq!(
        warrant["review"]["verification_records"], 2,
        "history is retained"
    );
    for obligation in warrant["obligations"].as_array().unwrap() {
        assert_eq!(
            obligation["disposition"], "unknown",
            "unbound history is not current assurance: {obligation}"
        );
        assert_eq!(obligation["verifier"], "fixture-independent-verifier");
        assert!(
            obligation["inadmissible_because"]
                .as_str()
                .unwrap()
                .contains("reviewed subject")
        );
    }
}

#[test]
fn fresh_bound_review_qualifies_and_exact_replay_preserves_records() {
    let fixture = Fixture::new();
    fixture.bound_response();
    let result = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_eq!(result["exit_code"], 0, "{result}");
    let before = fixture.review_state();
    let progress = fixture.run(&["status", "--json"]);
    let warrant = progress["result"]["warrants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["alias"] == "IX-WAR-0003")
        .unwrap();
    for obligation in warrant["obligations"].as_array().unwrap() {
        assert_eq!(obligation["disposition"], "established", "{obligation}");
    }
    let prepare = fixture.run(&["prepare", "IX-WAR-0003", "--dry-run", "--json"]);
    let verify = prepare["result"]["warrants"][0]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["step"] == "verify")
        .unwrap();
    assert_eq!(verify["outcome"], "current", "{prepare}");
    let replay = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_eq!(replay["exit_code"], 0, "{replay}");
    assert_eq!(
        fixture.review_state(),
        before,
        "exact replay must be byte preserving"
    );
}

#[test]
fn changed_artifact_refuses_old_bound_response_without_writes() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join("feature.txt"), "original feature").unwrap();
    fs::write(
        fixture
            .0
            .join("docs/warrants/IX-WAR-0003/deliverables.toml"),
        r#"
schema = "oh.war/deliverables/v1"
[[deliverable]]
id = "D-001"
title = "fixture feature"
kind = "file"
target_ref = "feature.txt"
required = true
content_addressed = false
provenance_required = false
obligation_refs = ["OBL-001"]
"#,
    )
    .unwrap();
    fixture.bound_response();
    let result = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_eq!(result["exit_code"], 0, "{result}");
    let before = fixture.review_state();
    fs::write(fixture.0.join("feature.txt"), "changed feature").unwrap();
    let stale = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_ne!(stale["exit_code"], 0, "{stale}");
    assert!(
        stale["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "verify.subject-stale"),
        "{stale}"
    );
    assert_eq!(
        fixture.review_state(),
        before,
        "stale import must not change history"
    );
    let progress = fixture.run(&["status", "--json"]);
    let warrant = progress["result"]["warrants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["alias"] == "IX-WAR-0003")
        .unwrap();
    assert_eq!(warrant["obligations"][0]["disposition"], "unknown");
}

#[test]
fn unknown_obligation_refuses_the_whole_response_before_any_write() {
    let fixture = Fixture::new();
    fixture.bound_response();
    let path = fixture.0.with_extension("response.toml");
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("OBL-001", "../outside-verifications");
    fs::write(path, text).unwrap();
    let before = fs::read(fixture.0.join("docs/warrants/IX-WAR-0003/journal.jsonl")).ok();
    let report = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_ne!(report["exit_code"], 0, "{report}");
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "verify.unknown-obligation"),
        "{report}"
    );
    assert!(
        !fixture
            .0
            .join("docs/warrants/IX-WAR-0003/outside-verifications.toml")
            .exists()
    );
    assert!(
        !fixture
            .0
            .join("docs/warrants/IX-WAR-0003/verifications/OBL-002.toml")
            .exists()
    );
    assert_eq!(
        fs::read(fixture.0.join("docs/warrants/IX-WAR-0003/journal.jsonl")).ok(),
        before
    );
}

#[test]
fn mark_requirement_does_not_treat_unbound_history_as_current_review() {
    let fixture = Fixture::new();
    fixture.response();
    fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    let report = fixture.run(&["mark", "IX-WAR-0003", "--json"]);
    let requirement = report["result"]["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["check"] == "obligations.independently_established")
        .unwrap();
    assert_eq!(requirement["result"], "unknown", "{report}");
    assert!(
        requirement["detail"]
            .as_str()
            .unwrap()
            .contains("reviewed subject")
    );
}

#[cfg(unix)]
#[test]
fn configured_verifier_echoes_the_subject_it_actually_reviewed() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let stub = fixture.0.join("fixture-claude");
    fs::write(
        &stub,
        r#"#!/usr/bin/env python3
import json, sys
if "--version" in sys.argv:
    print("synthetic-verifier-fixture")
else:
    bundle = json.load(sys.stdin)
    print(json.dumps({"verdicts": [{"obligation": o["id"], "disposition": "established",
        "evidence": "synthetic configured-verifier fixture, no real model qualification"}
        for o in bundle["request"]["obligations"]]}))
"#,
    )
    .unwrap();
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o700)).unwrap();
    let config_path = fixture.0.join("openwarrant.toml");
    let mut config: toml::Value =
        toml::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    let wrapper =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/verifier/claude-verifier.sh");
    config["verify"]["verifier_argv"] = toml::Value::Array(vec![
        toml::Value::String("bash".to_owned()),
        toml::Value::String(wrapper.to_str().unwrap().to_owned()),
    ]);
    fs::write(config_path, toml::to_string(&config).unwrap()).unwrap();
    let report = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--performer",
        "fixture-performer",
        "--run",
        "--json",
    ]);
    assert_eq!(report["exit_code"], 0, "{report}");
    let prepare = fixture.run(&["prepare", "IX-WAR-0003", "--dry-run", "--json"]);
    let verify = prepare["result"]["warrants"][0]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["step"] == "verify")
        .unwrap();
    assert_eq!(verify["outcome"], "current", "{prepare}");
}

#[test]
fn malformed_deliverables_cannot_be_bound_as_an_empty_artifact_set() {
    let fixture = Fixture::new();
    fs::write(
        fixture
            .0
            .join("docs/warrants/IX-WAR-0003/deliverables.toml"),
        "[[deliverable]]\nid = [malformed",
    )
    .unwrap();
    let report = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--performer",
        "fixture-performer",
        "--json",
    ]);
    assert_ne!(
        report["exit_code"], 0,
        "malformed declarations cannot produce a reviewed subject: {report}"
    );
}

#[test]
fn known_independence_failure_is_not_relabelled_as_unknown_binding() {
    let fixture = Fixture::new();
    fixture.response();
    fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    let path = fixture
        .0
        .join("docs/warrants/IX-WAR-0003/verifications/OBL-001.toml");
    let text = fs::read_to_string(&path).unwrap().replace(
        "cannot_modify_gate_fixtures = true",
        "cannot_modify_gate_fixtures = false",
    );
    fs::write(path, text).unwrap();
    let report = fixture.run(&["status", "--json"]);
    let warrant = report["result"]["warrants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["alias"] == "IX-WAR-0003")
        .unwrap();
    let obligation = warrant["obligations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == "OBL-001")
        .unwrap();
    assert_eq!(obligation["disposition"], "inadmissible", "{obligation}");
    assert!(
        obligation["inadmissible_because"]
            .as_str()
            .unwrap()
            .contains("cannot_modify_gate_fixtures")
    );
}

#[test]
fn changed_required_fixture_refuses_the_old_review_without_writes() {
    let fixture = Fixture::new();
    let gates = fixture.0.join("docs/gates");
    fs::create_dir_all(&gates).unwrap();
    let mut definition = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/gates/software.repo.war-check@1.0.0.yaml"),
    )
    .unwrap();
    definition.push_str("\nfixtures: [\"fixtures/required.txt\"]\n");
    fs::write(gates.join("software.repo.war-check@1.0.0.yaml"), definition).unwrap();
    fs::create_dir_all(fixture.0.join("fixtures")).unwrap();
    fs::write(
        fixture.0.join("fixtures/required.txt"),
        "required expectation",
    )
    .unwrap();
    fixture.bound_response();
    let first = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_eq!(first["exit_code"], 0, "{first}");
    let before = fixture.review_state();
    fs::write(
        fixture.0.join("fixtures/required.txt"),
        "changed expectation",
    )
    .unwrap();
    let stale = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_ne!(
        stale["exit_code"], 0,
        "a new fixture is a new question: {stale}"
    );
    assert!(
        stale["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "verify.subject-stale"),
        "{stale}"
    );
    assert_eq!(fixture.review_state(), before);
    // Restoring fixture bytes does not hide a changed gate question.
    fs::write(
        fixture.0.join("fixtures/required.txt"),
        "required expectation",
    )
    .unwrap();
    let path = gates.join("software.repo.war-check@1.0.0.yaml");
    let definition = fs::read_to_string(&path).unwrap();
    assert!(definition.contains("timeout_secs: \"120\""));
    fs::write(
        path,
        definition.replace("timeout_secs: \"120\"", "timeout_secs: \"121\""),
    )
    .unwrap();
    let changed_gate = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_ne!(changed_gate["exit_code"], 0, "{changed_gate}");
    assert!(
        changed_gate["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "verify.subject-stale"),
        "{changed_gate}"
    );
    assert_eq!(fixture.review_state(), before);
}

#[test]
fn changed_contract_refuses_a_bound_old_response_without_writes() {
    let fixture = Fixture::new();
    fixture.bound_response();
    let first = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_eq!(first["exit_code"], 0, "{first}");
    let before = fixture.review_state();
    let path = fixture
        .0
        .join("docs/warrants/IX-WAR-0003/atoms/60-assurance.md");
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("the SAS is accepted and pinned"));
    fs::write(
        path,
        text.replace(
            "the SAS is accepted and pinned",
            "the new requirement is reviewed",
        ),
    )
    .unwrap();
    let stale = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_ne!(stale["exit_code"], 0, "{stale}");
    assert!(
        stale["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "verify.subject-stale"),
        "{stale}"
    );
    assert_eq!(fixture.review_state(), before);
}

#[test]
fn offline_bundle_carries_exact_gate_and_fixture_sources() {
    let fixture = Fixture::new();
    let config = fixture.0.join("openwarrant.toml");
    let settings = fs::read_to_string(&config).unwrap();
    fs::write(
        config,
        settings.replace("[verify]", "[verify]\nmax_bundle_tokens = 1"),
    )
    .unwrap();
    let gates = fixture.0.join("docs/gates");
    fs::create_dir_all(&gates).unwrap();
    let mut gate = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/gates/software.repo.war-check@1.0.0.yaml"),
    )
    .unwrap();
    gate.push_str("\nfixtures: [\"fixtures/required.bin\"]\n");
    fs::write(gates.join("software.repo.war-check@1.0.0.yaml"), &gate).unwrap();
    fs::create_dir_all(fixture.0.join("fixtures")).unwrap();
    fs::write(fixture.0.join("fixtures/required.bin"), [0, 255, 13, 10]).unwrap();
    let emitted = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--performer",
        "fixture-performer",
        "--bundle",
        "--json",
    ]);
    assert_eq!(emitted["exit_code"], 0, "{emitted}");
    let files: Vec<_> = fs::read_dir(fixture.0.join("docs/warrants/IX-WAR-0003/verifications"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("bundle-"))
        .collect();
    assert_eq!(
        files.len(),
        2,
        "the forced budget must split two obligations"
    );
    for file in files {
        let bundle: serde_json::Value =
            serde_json::from_slice(&fs::read(file.path()).unwrap()).unwrap();
        let sources = bundle["required_sources"]
            .as_array()
            .expect("offline review needs source bytes, not only hashes");
        let definition = sources
            .iter()
            .find(|s| s["path"] == "docs/gates/software.repo.war-check@1.0.0.yaml")
            .unwrap();
        assert_eq!(definition["text"], gate);
        assert_eq!(definition["kind"], "gate-definition");
        let binary = sources
            .iter()
            .find(|s| s["path"] == "fixtures/required.bin")
            .unwrap();
        assert_eq!(binary["bytes"], serde_json::json!([0, 255, 13, 10]));
        assert_eq!(binary["kind"], "fixture");
        assert!(
            binary.get("text").is_none(),
            "binary content must not become lossy text"
        );
        assert_eq!(
            binary["sha256"],
            bundle["request"]["reviewed_subject"]["fixtures"]["fixtures/required.bin"]
        );
    }
}

#[test]
fn changed_declared_gate_input_refuses_old_review_without_writes() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.0.join("docs/gates")).unwrap();
    let mut definition = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/gates/software.repo.war-check@1.0.0.yaml"),
    )
    .unwrap();
    definition.push_str("\ninputs: [\"src/**\"]\n");
    fs::write(
        fixture
            .0
            .join("docs/gates/software.repo.war-check@1.0.0.yaml"),
        definition,
    )
    .unwrap();
    fs::create_dir_all(fixture.0.join("src")).unwrap();
    fs::write(fixture.0.join("src/helper.txt"), "reviewed input").unwrap();
    fixture.bound_response();
    let first = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_eq!(first["exit_code"], 0, "{first}");
    let before = fixture.review_state();
    fs::write(fixture.0.join("src/helper.txt"), "different input").unwrap();
    let replay = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_ne!(
        replay["exit_code"], 0,
        "an unlisted delivery is still a declared gate input: {replay}"
    );
    assert!(
        replay["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "verify.subject-stale"),
        "{replay}"
    );
    assert_eq!(
        fixture.review_state(),
        before,
        "a stale replay writes nothing"
    );
}

#[test]
fn gate_without_input_list_binds_tree_without_publishing_performer_rationale() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.0.join("docs/gates")).unwrap();
    let definition = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/gates/software.repo.war-check@1.0.0.yaml"),
    )
    .unwrap();
    fs::write(
        fixture
            .0
            .join("docs/gates/software.repo.war-check@1.0.0.yaml"),
        definition,
    )
    .unwrap();
    fs::create_dir_all(fixture.0.join("src")).unwrap();
    fs::write(fixture.0.join("src/helper.txt"), "reviewed tree input").unwrap();
    fs::write(
        fixture.0.join("docs/warrants/IX-WAR-0003/rationale.toml"),
        "private_performer_story = 'must not enter the blind packet'",
    )
    .unwrap();
    fixture.bound_response();
    let first = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_eq!(first["exit_code"], 0, "{first}");
    let packet = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--performer",
        "fixture-performer",
        "--bundle",
        "--json",
    ]);
    assert_eq!(packet["exit_code"], 0, "{packet}");
    for file in fs::read_dir(fixture.0.join("docs/warrants/IX-WAR-0003/verifications")).unwrap() {
        let file = file.unwrap();
        if !file.file_name().to_string_lossy().starts_with("bundle-") {
            continue;
        }
        let text = fs::read_to_string(file.path()).unwrap();
        assert!(
            !text.contains("private_performer_story"),
            "source bindings do not publish performer rationale"
        );
    }
    let before = fixture.review_state();
    fs::write(fixture.0.join("src/helper.txt"), "unreviewed tree input").unwrap();
    let replay = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_ne!(
        replay["exit_code"], 0,
        "a gate with no input list still binds the source tree: {replay}"
    );
    assert!(
        replay["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "verify.subject-stale"),
        "{replay}"
    );
    assert_eq!(fixture.review_state(), before);
}

#[cfg(unix)]
#[test]
fn unavailable_symlink_source_is_unknown_and_writes_no_verdict() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.0.join("docs/gates")).unwrap();
    fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/gates/software.repo.war-check@1.0.0.yaml"),
        fixture
            .0
            .join("docs/gates/software.repo.war-check@1.0.0.yaml"),
    )
    .unwrap();
    std::os::unix::fs::symlink("/outside-unavailable-source", fixture.0.join("unsafe-link"))
        .unwrap();
    let verdicts = fixture.0.join("docs/warrants/IX-WAR-0003/verifications");
    assert!(!verdicts.exists());
    let result = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--performer",
        "fixture-performer",
        "--json",
    ]);
    assert_ne!(result["exit_code"], 0, "{result}");
    assert_eq!(result["counts"]["error"], 0, "{result}");
    assert_eq!(result["counts"]["unknown"], 1, "{result}");
    assert_eq!(
        result["diagnostics"][0]["rule"], "verify.subject-unavailable",
        "{result}"
    );
    assert!(
        !verdicts.exists(),
        "an unavailable observation writes no verdict"
    );
}

#[cfg(unix)]
#[test]
fn internal_link_binds_target_identity_and_selected_dependencies() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.0.join("docs/gates")).unwrap();
    let mut gate = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/gates/software.repo.war-check@1.0.0.yaml"),
    )
    .unwrap();
    gate.push_str("\ninputs:\n  - links/**\n");
    fs::write(
        fixture
            .0
            .join("docs/gates/software.repo.war-check@1.0.0.yaml"),
        gate,
    )
    .unwrap();
    fs::create_dir_all(fixture.0.join("src")).unwrap();
    fs::create_dir_all(fixture.0.join("other")).unwrap();
    fs::create_dir_all(fixture.0.join("links")).unwrap();
    fs::write(fixture.0.join("src/api.txt"), "reviewed contract").unwrap();
    fs::write(fixture.0.join("other/api.txt"), "reviewed contract").unwrap();
    std::os::unix::fs::symlink("../src", fixture.0.join("links/context")).unwrap();
    fixture.bound_response();
    let accepted = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_eq!(accepted["exit_code"], 0, "{accepted}");
    let before = fixture.review_state();
    fs::write(fixture.0.join("src/api.txt"), "unreviewed contract").unwrap();
    let changed = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert!(
        changed["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "verify.subject-stale"),
        "{changed}"
    );
    assert_eq!(fixture.review_state(), before);
    fs::write(fixture.0.join("src/api.txt"), "reviewed contract").unwrap();
    fs::remove_file(fixture.0.join("links/context")).unwrap();
    std::os::unix::fs::symlink("../other", fixture.0.join("links/context")).unwrap();
    let retargeted = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert!(
        retargeted["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "verify.subject-stale"),
        "{retargeted}"
    );
    assert_eq!(fixture.review_state(), before);
}
