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
        response["schema"] = toml::Value::String("oh.war/verification-response/v2".into());
        response.as_table_mut().unwrap().insert(
            "reviewed_subject".to_owned(),
            toml::Value::try_from(&request["result"]["reviewed_subject"]).unwrap(),
        );
        let packets = self.run(&[
            "verify",
            "IX-WAR-0003",
            "--performer",
            "fixture-performer",
            "--bundle",
            "--json",
        ]);
        assert_eq!(packets["exit_code"], 0, "{packets}");
        response.as_table_mut().unwrap().insert(
            "reviewed_packets".into(),
            toml::Value::try_from(&packets["result"]["packets"]).unwrap(),
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
    let response: toml::Value =
        toml::from_str(&fs::read_to_string(fixture.0.with_extension("response.toml")).unwrap())
            .unwrap();
    let packet_path = fixture
        .0
        .join(response["reviewed_packets"][0]["path"].as_str().unwrap());
    let retained = fs::read(&packet_path).unwrap();
    fs::remove_file(&packet_path).unwrap();
    let progress = fixture.run(&["status", "--json"]);
    let warrant = progress["result"]["warrants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["alias"] == "IX-WAR-0003")
        .unwrap();
    assert_eq!(
        warrant["review"]["verification_records"], 2,
        "history remains"
    );
    for obligation in warrant["obligations"].as_array().unwrap() {
        assert_eq!(
            obligation["disposition"], "unknown",
            "missing packet cannot qualify: {obligation}"
        );
    }
    assert_eq!(
        fixture.review_state(),
        before,
        "qualification does not rewrite history"
    );
    fs::write(&packet_path, retained).unwrap();
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

#[test]
fn wrong_packet_digest_refuses_all_verdicts_before_writing() {
    let fixture = Fixture::new();
    fixture.bound_response();
    let path = fixture.0.with_extension("response.toml");
    let original_response = fs::read_to_string(&path).unwrap();
    let mut response: toml::Value = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    response.as_table_mut().unwrap().insert("reviewed_packets".into(), toml::Value::Array(vec![toml::Value::try_from(serde_json::json!({"path":"docs/warrants/IX-WAR-0003/verifications/bundle-invalid.json","digest":"sha256:wrong"})).unwrap()]));
    fs::write(&path, toml::to_string(&response).unwrap()).unwrap();
    let result = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_ne!(result["exit_code"], 0, "{result}");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "verify.packet-binding"),
        "{result}"
    );
    assert!(
        !fixture
            .0
            .join("docs/warrants/IX-WAR-0003/verifications/OBL-001.toml")
            .exists()
    );
    assert!(
        !fixture
            .0
            .join("docs/warrants/IX-WAR-0003/verifications/OBL-002.toml")
            .exists()
    );
    // A well-formed identity must also refuse changed packet contents.
    fs::write(&path, &original_response).unwrap();
    let original: toml::Value = toml::from_str(&original_response).unwrap();
    let packet_path = fixture
        .0
        .join(original["reviewed_packets"][0]["path"].as_str().unwrap());
    let original_packet = fs::read(&packet_path).unwrap();
    let mut packet: serde_json::Value = serde_json::from_slice(&original_packet).unwrap();
    packet["scope"] = serde_json::json!("tampered scope");
    fs::write(&packet_path, serde_json::to_vec(&packet).unwrap()).unwrap();
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
            .any(|d| d["rule"] == "verify.packet-binding"),
        "{changed}"
    );
    assert!(
        !fixture
            .0
            .join("docs/warrants/IX-WAR-0003/verifications/OBL-001.toml")
            .exists()
    );
    fs::write(&packet_path, original_packet).unwrap();
    let accepted = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_eq!(accepted["exit_code"], 0, "{accepted}");
}

#[test]
fn unavailable_retained_packet_reports_unknown_without_rewriting_history() {
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
    let response: toml::Value =
        toml::from_str(&fs::read_to_string(fixture.0.with_extension("response.toml")).unwrap())
            .unwrap();
    let packet_path = fixture
        .0
        .join(response["reviewed_packets"][0]["path"].as_str().unwrap());
    let packet = fs::read(&packet_path).unwrap();
    fs::remove_file(&packet_path).unwrap();
    let replay = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_ne!(replay["exit_code"], 0, "{replay}");
    assert_eq!(replay["counts"]["error"], 0, "{replay}");
    assert_eq!(replay["counts"]["unknown"], 1, "{replay}");
    assert_eq!(
        replay["diagnostics"][0]["rule"], "verify.packet-unavailable",
        "{replay}"
    );
    assert_eq!(fixture.review_state(), before);
    fs::write(&packet_path, packet).unwrap();
    let restored = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_eq!(restored["exit_code"], 0, "{restored}");
    assert_eq!(fixture.review_state(), before);
}

#[test]
fn legacy_protocol_cannot_qualify_even_when_new_binding_fields_are_supplied() {
    let fixture = Fixture::new();
    fixture.bound_response();
    let path = fixture.0.with_extension("response.toml");
    let mut response: toml::Value = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    response["schema"] = toml::Value::String("oh.war/verification-response/v1".into());
    fs::write(&path, toml::to_string(&response).unwrap()).unwrap();
    let retained = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_eq!(retained["counts"]["error"], 0, "{retained}");
    assert!(
        retained["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "verify.protocol-legacy"),
        "{retained}"
    );
    let progress = fixture.run(&["status", "--json"]);
    let warrant = progress["result"]["warrants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["alias"] == "IX-WAR-0003")
        .unwrap();
    assert_eq!(
        warrant["review"]["verification_records"], 2,
        "history remains available"
    );
    for obligation in warrant["obligations"].as_array().unwrap() {
        assert_eq!(
            obligation["disposition"], "unknown",
            "legacy protocol is not current assurance: {obligation}"
        );
    }
}

#[test]
fn unsupported_future_response_is_unknown_and_v2_missing_bindings_is_invalid() {
    let fixture = Fixture::new();
    fixture.response();
    let path = fixture.0.with_extension("response.toml");
    let mut response: toml::Value = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    response["schema"] = toml::Value::String("oh.war/verification-response/v3".into());
    fs::write(&path, toml::to_string(&response).unwrap()).unwrap();
    let future = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_eq!(future["counts"]["error"], 0, "{future}");
    assert_eq!(future["counts"]["unknown"], 1, "{future}");
    assert!(
        !fixture
            .0
            .join("docs/warrants/IX-WAR-0003/verifications")
            .exists()
    );
    response["schema"] = toml::Value::String("oh.war/verification-response/v2".into());
    fs::write(&path, toml::to_string(&response).unwrap()).unwrap();
    let incomplete = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert!(
        incomplete["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "verify.response-binding-required"),
        "{incomplete}"
    );
    assert!(
        !fixture
            .0
            .join("docs/warrants/IX-WAR-0003/verifications")
            .exists()
    );
}

#[test]
fn unsupported_stored_record_is_unknown_while_malformed_supported_record_is_an_error() {
    let fixture = Fixture::new();
    fixture.bound_response();
    let ingested = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_eq!(ingested["exit_code"], 0, "{ingested}");
    let path = fixture
        .0
        .join("docs/warrants/IX-WAR-0003/verifications/OBL-001.toml");
    let original = fs::read(&path).unwrap();
    let journal_path = fixture.0.join("docs/warrants/IX-WAR-0003/journal.jsonl");
    let journal = fs::read(&journal_path).unwrap();
    let baseline = fixture.run(&["resolve", "--dry-run", "IX-WAR-0003", "--json"]);

    fs::write(&path, "schema = \"oh.war/verification/v77\"\n").unwrap();
    for args in [
        vec!["resolve", "--dry-run", "IX-WAR-0003", "--json"],
        vec!["status", "--json"],
        vec![
            "verify",
            "IX-WAR-0003",
            "--performer",
            "fixture-performer",
            "--bundle",
            "--json",
        ],
    ] {
        let report = fixture.run(&args);
        assert_eq!(report["counts"]["error"], 0, "{report}");
        assert_eq!(report["counts"]["unknown"], 1, "{report}");
        assert!(
            report["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["rule"] == "verify.record-schema-unsupported"),
            "{report}"
        );
    }
    assert_eq!(fs::read(&journal_path).unwrap(), journal);
    assert_eq!(
        fs::read(&path).unwrap(),
        b"schema = \"oh.war/verification/v77\"\n"
    );

    fs::write(&path, "schema = \"oh.war/verification/v2\"\n").unwrap();
    let malformed = fixture.run(&["resolve", "--dry-run", "IX-WAR-0003", "--json"]);
    assert!(
        malformed["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "verification.malformed" && d["severity"] == "error"),
        "{malformed}"
    );
    fs::write(&path, original).unwrap();
    let restored = fixture.run(&["resolve", "--dry-run", "IX-WAR-0003", "--json"]);
    assert_eq!(
        restored, baseline,
        "restoring exact bytes must restore the observation"
    );
    assert_eq!(fs::read(&journal_path).unwrap(), journal);
}

#[test]
fn document_review_reports_an_unsupported_record_instead_of_an_absent_review() {
    let fixture = Fixture::new();
    let dir = fixture.0.join("docs/warrants/IX-WAR-0003");
    fs::create_dir_all(dir.join("verifications")).unwrap();
    fs::write(
        dir.join("verifications/OBL-001.toml"),
        "schema = \"oh.war/verification/v77\"\n",
    )
    .unwrap();
    fs::write(
        fixture.0.join("review.md"),
        "# Fixture document\n\nSynthetic data only.\n",
    )
    .unwrap();
    fs::write(
        dir.join("deliverables.toml"),
        r#"schema = "oh.war/deliverables/v1"
[[deliverable]]
id = "D-001"
title = "Fixture document"
kind = "document"
target_ref = "review.md"
required = true
content_addressed = false
provenance_required = false
obligation_refs = ["OBL-001"]
"#,
    )
    .unwrap();
    let report = fixture.run(&["document", "review", "IX-WAR-0003", "--json"]);
    let diagnostics = report["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|d| d["rule"] == "verify.record-schema-unsupported" && d["severity"] == "unknown"),
        "{report}"
    );
    assert!(
        !diagnostics
            .iter()
            .any(|d| d["rule"] == "document.review-absent"),
        "{report}"
    );
}

#[test]
fn emitting_again_never_replaces_a_retained_packet_with_different_bytes() {
    let fixture = Fixture::new();
    fixture.bound_response();
    let response: toml::Value =
        toml::from_str(&fs::read_to_string(fixture.0.with_extension("response.toml")).unwrap())
            .unwrap();
    let packet_path = fixture
        .0
        .join(response["reviewed_packets"][0]["path"].as_str().unwrap());
    let original = fs::read(&packet_path).unwrap();
    let args = [
        "verify",
        "IX-WAR-0003",
        "--performer",
        "fixture-performer",
        "--bundle",
        "--json",
    ];
    let unchanged = fixture.run(&args);
    assert_eq!(unchanged["exit_code"], 0, "{unchanged}");
    assert_eq!(fs::read(&packet_path).unwrap(), original);
    let occupied = b"different retained bytes: never silently repair or overwrite";
    fs::write(&packet_path, occupied).unwrap();
    let refused = fixture.run(&args);
    assert_ne!(refused["exit_code"], 0, "{refused}");
    assert!(
        refused["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["message"]
                .as_str()
                .unwrap_or_default()
                .contains("verify.packet-collision")),
        "{refused}"
    );
    assert_eq!(fs::read(&packet_path).unwrap(), occupied);
    fs::write(&packet_path, original).unwrap();
    let restored = fixture.run(&args);
    assert_eq!(restored["exit_code"], 0, "{restored}");
}

#[cfg(unix)]
#[test]
fn retained_packet_link_never_writes_its_external_target() {
    let fixture = Fixture::new();
    let external = Fixture::new();
    let outside = external.0.join("outside-sentinel.txt");
    let sentinel = b"outside bytes must remain unchanged";
    fs::write(&outside, sentinel).unwrap();
    fixture.bound_response();
    let response: toml::Value =
        toml::from_str(&fs::read_to_string(fixture.0.with_extension("response.toml")).unwrap())
            .unwrap();
    let packet_path = fixture
        .0
        .join(response["reviewed_packets"][0]["path"].as_str().unwrap());
    let original = fs::read(&packet_path).unwrap();
    fs::remove_file(&packet_path).unwrap();
    std::os::unix::fs::symlink(&outside, &packet_path).unwrap();
    let refused = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--performer",
        "fixture-performer",
        "--bundle",
        "--json",
    ]);
    assert_ne!(refused["exit_code"], 0, "{refused}");
    assert_eq!(refused["counts"]["error"], 0, "{refused}");
    assert_eq!(refused["counts"]["unknown"], 1, "{refused}");
    assert_eq!(
        refused["diagnostics"][0]["rule"], "verify.packet-unavailable",
        "{refused}"
    );
    assert_eq!(fs::read(&outside).unwrap(), sentinel);
    assert!(packet_path.is_symlink());
    fs::remove_file(&packet_path).unwrap();
    fs::write(&packet_path, original).unwrap();
    let restored = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--performer",
        "fixture-performer",
        "--bundle",
        "--json",
    ]);
    assert_eq!(restored["exit_code"], 0, "{restored}");
    assert_eq!(fs::read(&outside).unwrap(), sentinel);
}

#[test]
fn ambiguous_packet_object_members_refuse_all_verdicts_before_writes() {
    let fixture = Fixture::new();
    fixture.bound_response();
    let response: toml::Value =
        toml::from_str(&fs::read_to_string(fixture.0.with_extension("response.toml")).unwrap())
            .unwrap();
    let path = fixture
        .0
        .join(response["reviewed_packets"][0]["path"].as_str().unwrap());
    let original = fs::read_to_string(&path).unwrap();
    let member = "\"warrant\":\"IX-WAR-0003\"";
    assert!(original.contains(member));
    fs::write(
        &path,
        original.replacen(
            member,
            "\"warrant\":\"WRONG-SUBJECT\",\"warrant\":\"IX-WAR-0003\"",
            1,
        ),
    )
    .unwrap();
    let refused = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_ne!(refused["exit_code"], 0, "{refused}");
    assert!(
        refused["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "verify.packet-binding"),
        "{refused}"
    );
    for id in ["OBL-001", "OBL-002"] {
        assert!(
            !fixture
                .0
                .join(format!("docs/warrants/IX-WAR-0003/verifications/{id}.toml"))
                .exists()
        );
    }
    fs::write(&path, original).unwrap();
    let restored = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_eq!(restored["exit_code"], 0, "{restored}");
}

#[test]
fn strict_packet_reader_preserves_large_required_binary_fixtures() {
    let fixture = Fixture::new();
    let gates = fixture.0.join("docs/gates");
    fs::create_dir_all(&gates).unwrap();
    let mut gate = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/gates/software.repo.war-check@1.0.0.yaml"),
    )
    .unwrap();
    gate.push_str("\nfixtures: [\"fixtures/large.bin\"]\n");
    fs::write(gates.join("software.repo.war-check@1.0.0.yaml"), gate).unwrap();
    fs::create_dir_all(fixture.0.join("fixtures")).unwrap();
    fs::write(fixture.0.join("fixtures/large.bin"), vec![255u8; 70_000]).unwrap();
    fixture.bound_response();
    let response: toml::Value =
        toml::from_str(&fs::read_to_string(fixture.0.with_extension("response.toml")).unwrap())
            .unwrap();
    for reference in response["reviewed_packets"].as_array().unwrap() {
        let packet: serde_json::Value = serde_json::from_slice(
            &fs::read(fixture.0.join(reference["path"].as_str().unwrap())).unwrap(),
        )
        .unwrap();
        let binary = packet["required_sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["path"] == "fixtures/large.bin")
            .unwrap();
        let bytes = binary["bytes"].as_array().unwrap();
        assert_eq!(bytes.len(), 70_000);
        assert!(bytes.iter().all(|b| *b == serde_json::json!(255)));
    }
    let ingested = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_eq!(ingested["exit_code"], 0, "{ingested}");
    assert_eq!(ingested["counts"]["pass"], 2, "{ingested}");
}

#[test]
fn offline_review_carries_exact_pinned_architecture_rules() {
    let fixture = Fixture::new();
    let source = "# Inbox SAS\n\n## 106. Requirements\n\n| ID | Requirement |\n| --- | --- |\n| IX-SAS-RQ-001 | Email verification is required before account activation. |\n";
    fs::create_dir_all(fixture.0.join("docs/sas")).unwrap();
    fs::write(fixture.0.join("docs/sas/Inbox_SAS.md"), source).unwrap();
    let proposal = fixture.run(&["sas", "propose", "0.1.0", "--json"]);
    assert_eq!(proposal["exit_code"], 0, "{proposal}");
    let emitted = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--performer",
        "fixture-performer",
        "--bundle",
        "--json",
    ]);
    assert_eq!(emitted["exit_code"], 0, "{emitted}");
    for reference in emitted["result"]["packets"].as_array().unwrap() {
        let packet: serde_json::Value = serde_json::from_slice(
            &fs::read(fixture.0.join(reference["path"].as_str().unwrap())).unwrap(),
        )
        .unwrap();
        assert!(
            packet["request"]["inputs"]["required_context_refs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r == "docs/sas/Inbox_SAS.md")
        );
        let rule = packet["required_sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["path"] == "docs/sas/Inbox_SAS.md")
            .expect("offline packet must carry the pinned governing source");
        assert_eq!(rule["text"], source);
        assert_eq!(rule["kind"], "governing-sas");
    }
    fs::write(
        fixture.0.join("docs/sas/Inbox_SAS.md"),
        "# Different architecture\n",
    )
    .unwrap();
    let missing = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--performer",
        "fixture-performer",
        "--bundle",
        "--json",
    ]);
    assert!(
        missing["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "verify.context-unavailable" && d["severity"] == "unknown"),
        "{missing}"
    );
    fs::write(fixture.0.join("docs/sas/Inbox_SAS.md"), source).unwrap();
    let restored = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--performer",
        "fixture-performer",
        "--bundle",
        "--json",
    ]);
    assert_eq!(restored["exit_code"], 0, "{restored}");
}

#[test]
fn candidate_reads_exact_objects_without_git_replacements() {
    let fixture = Fixture::new();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(&fixture.0)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "-q", "--template=", "--initial-branch=fixture"]);
    git(&["add", "."]);
    let commit = |message: &str| {
        git(&[
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "--no-gpg-sign",
            "-qm",
            message,
        ]);
        git(&["rev-parse", "HEAD"])
    };
    let candidate = commit("Exact candidate");
    let config = fixture.0.join("openwarrant.toml");
    let original = fs::read(&config).unwrap();
    fs::write(&config, "not a valid repository TOML document\n").unwrap();
    git(&["add", "openwarrant.toml"]);
    let poison = commit("Replacement poison");
    fs::write(&config, original).unwrap();
    let before = fixture.run(&["pins", "--candidate", &candidate, "--json"]);
    assert_eq!(before["exit_code"], 0, "{before}");
    git(&["replace", &candidate, &poison]);
    let replaced = fixture.run(&["pins", "--candidate", &candidate, "--json"]);
    assert_eq!(
        replaced["exit_code"], 0,
        "replacement must not change exact candidate data: {replaced}"
    );
    assert_eq!(replaced["result"], before["result"]);
    let invalid = fixture.run(&["pins", "--candidate", &poison, "--json"]);
    assert_ne!(
        invalid["exit_code"], 0,
        "invalid candidate must still be inspected"
    );
    assert!(
        invalid["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "acceptance.unknown")
    );
    git(&["replace", "-d", &candidate]);
    let restored = fixture.run(&["pins", "--candidate", &candidate, "--json"]);
    assert_eq!(restored["exit_code"], 0, "{restored}");
}

#[test]
fn archive_retains_wrapped_verification_without_claiming_current_assurance() {
    let fixture = Fixture::new();
    fixture.bound_response();
    let ingest = fixture.run(&[
        "verify",
        "IX-WAR-0003",
        "--response",
        "response.toml",
        "--json",
    ]);
    assert_eq!(ingest["exit_code"], 0, "{ingest}");
    for args in [
        vec!["init", "-q"],
        vec!["add", "."],
        vec![
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "Synthetic review history",
        ],
    ] {
        let output = Command::new("git")
            .current_dir(&fixture.0)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let archive = fixture.0.with_extension("archive.json");
    let exported = fixture.run(&[
        "archive",
        "export",
        "IX-WAR-0003",
        archive.to_str().unwrap(),
        "--history",
        "--json",
    ]);
    assert_eq!(exported["exit_code"], 0, "{exported}");
    assert_eq!(
        exported["result"]["coverage"]["assurance case"]["state"], "retained",
        "{exported}"
    );
    assert_eq!(exported["result"]["authority_activated"], false);
    let inspected = fixture.run(&["archive", "inspect", archive.to_str().unwrap(), "--json"]);
    assert_eq!(inspected["exit_code"], 0, "{inspected}");
    assert_eq!(
        inspected["result"]["coverage"]["assurance case"]["state"], "retained",
        "{inspected}"
    );
    let malformed = fixture
        .0
        .join("docs/warrants/IX-WAR-0003/verifications/OBL-001.toml");
    fs::write(
        malformed,
        "schema = 'oh.war/verification/v2'\nverification = 'not a record'\n",
    )
    .unwrap();
    let refused = fixture.run(&[
        "archive",
        "export",
        "IX-WAR-0003",
        fixture
            .0
            .with_extension("malformed-archive.json")
            .to_str()
            .unwrap(),
        "--history",
        "--json",
    ]);
    assert_eq!(refused["exit_code"], 0, "{refused}");
    assert_eq!(
        refused["result"]["coverage"]["assurance case"]["state"], "unavailable",
        "{refused}"
    );
    assert!(
        refused["result"]["coverage"]["assurance case"]["reason"]
            .as_str()
            .unwrap()
            .contains("unreadable verification record"),
        "{refused}"
    );
    let _ = fs::remove_file(archive);
    let _ = fs::remove_file(fixture.0.with_extension("malformed-archive.json"));
}
