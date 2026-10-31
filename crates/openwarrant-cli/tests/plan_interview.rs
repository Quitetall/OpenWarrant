// SPDX-License-Identifier: Apache-2.0
//! W35: bounded, answer-aware questions at the public planner validation seam.
//! No model, repository mutation, review or authority act is performed here.
use std::collections::BTreeSet;

use openwarrant_cli::plan;
use serde_json::{Value, json};

#[test]
fn interview_refusal_lists_only_unanswered_blockers_in_information_order() {
    let mut proposal: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../conformance/fixtures/proposals/v2-minimal.json"
    )))
    .unwrap();
    proposal["unresolved_questions"] = json!([
        {"id":"Q-ANSWERED","question":"ANSWERED-QUESTION-MARKER","removes_blocker":true,"expected_information_gain":100},
        {"id":"Q-LOW","question":"Lower priority blocker","removes_blocker":true,"expected_information_gain":50},
        {"id":"Q-OPTIONAL","question":"Optional background","removes_blocker":false,"expected_information_gain":1000},
        {"id":"Q-HIGH","question":"Higher priority blocker","removes_blocker":true,"expected_information_gain":90}
    ]);
    let proposal = serde_json::to_string(&proposal).unwrap();
    let answered = BTreeSet::from(["Q-ANSWERED".to_owned()]);
    let error = plan::validate_v2(&proposal, false, &answered, &BTreeSet::new())
        .unwrap_err()
        .to_string();
    assert!(error.starts_with("plan.interview-required"), "{error}");
    let questions: Value = serde_json::from_str(error.split_once('\n').unwrap().1).unwrap();
    let ids: Vec<_> = questions["questions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| q["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["Q-HIGH", "Q-LOW"]);
    assert!(!error.contains("ANSWERED-QUESTION-MARKER"), "{error}");

    // Answers remove the interview barrier without completing human review.
    let all_answered = ["Q-ANSWERED", "Q-LOW", "Q-HIGH"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let (_, pipeline) =
        plan::validate_v2(&proposal, false, &all_answered, &BTreeSet::new()).unwrap();
    assert!(pipeline.may_apply().is_err());
}

#[test]
fn interview_command_resumes_stored_input_and_exposes_bounded_question_counts() {
    use std::{collections::BTreeMap, fs, path::Path, process::Command};
    fn copy(source: &Path, target: &Path) {
        fs::create_dir_all(target).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let path = target.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &path);
            } else {
                fs::copy(entry.path(), path).unwrap();
            }
        }
    }
    fn bytes(root: &Path) -> BTreeMap<String, Vec<u8>> {
        fn read(root: &Path, at: &Path, output: &mut BTreeMap<String, Vec<u8>>) {
            for entry in fs::read_dir(at).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    read(root, &entry.path(), output);
                } else {
                    output.insert(
                        entry
                            .path()
                            .strip_prefix(root)
                            .unwrap()
                            .to_string_lossy()
                            .into_owned(),
                        fs::read(entry.path()).unwrap(),
                    );
                }
            }
        }
        let mut output = BTreeMap::new();
        read(root, root, &mut output);
        output
    }
    let root = std::env::temp_dir().join(format!(
        "ow35-interview-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/fixtures/inbox/repository"),
        &root,
    );
    let repo = openwarrant_cli::repo::Repository::open(
        camino::Utf8PathBuf::from_path_buf(root.clone()).unwrap(),
    )
    .unwrap();
    let key = plan::intake::sentence_key("Bounded interview fixture");
    let dir = plan::intake::store_dir(&repo, &key);
    fs::create_dir_all(&dir).unwrap();
    for (id, question) in [
        ("Q-ANSWERED", "Already supplied"),
        ("Q-LOW", "Lower priority"),
        ("Q-HIGH", "Higher priority"),
        ("Q-OPTIONAL", "Background only"),
    ] {
        let path = openwarrant_cli::questions::record_intake(
            &repo,
            &key,
            id,
            question,
            "agent://fixture",
            "war plan draft",
        )
        .unwrap();
        if id == "Q-OPTIONAL" {
            let mut q: openwarrant_cli::questions::Question =
                toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            q.blocking = false;
            fs::write(path, toml::to_string_pretty(&q).unwrap()).unwrap();
        }
    }
    let request = plan::request(
        &repo,
        "Bounded interview fixture",
        "delivery",
        "basic",
        &BTreeMap::from([("Q-ANSWERED".into(), "Supplied by the fixture".into())]),
    )
    .unwrap();
    fs::write(
        dir.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    let mut proposal: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../conformance/fixtures/proposals/v2-minimal.json"
    )))
    .unwrap();
    proposal["unresolved_questions"] = json!([
        {"id":"Q-ANSWERED","question":"Already supplied","removes_blocker":true,"expected_information_gain":100},
        {"id":"Q-LOW","question":"Lower priority","removes_blocker":true,"expected_information_gain":50},
        {"id":"Q-HIGH","question":"Higher priority","removes_blocker":true,"expected_information_gain":90},
        {"id":"Q-OPTIONAL","question":"Background only","removes_blocker":false,"expected_information_gain":1000}
    ]);
    fs::write(
        dir.join("proposal.json"),
        serde_json::to_vec(&proposal).unwrap(),
    )
    .unwrap();
    let before = bytes(&root);
    let output = Command::new(env!("CARGO_BIN_EXE_war"))
        .args([
            "--root",
            root.to_str().unwrap(),
            "--json",
            "interview",
            &key,
        ])
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["result"]["schema"], "oh.war/interview-view/v1");
    assert_eq!(value["result"]["recorded_question_count"], 4);
    let ids: Vec<_> = value["result"]["questions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| q["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["Q-HIGH", "Q-LOW"]);
    assert_eq!(
        bytes(&root),
        before,
        "interviewing is not application or authorization"
    );
    // Intake answers must not hide a later Warrant question with the same id.
    let warrant = root.join("docs/warrants/IX-WAR-0003");
    fs::create_dir_all(warrant.join("questions")).unwrap();
    fs::create_dir_all(warrant.join("plan")).unwrap();
    let mut question: openwarrant_cli::questions::Question =
        toml::from_str(&fs::read_to_string(dir.join("questions/Q-ANSWERED.toml")).unwrap()).unwrap();
    question.warrant = "IX-WAR-0003".to_owned();
    question.stage = "STAGE-001".to_owned();
    question.question = "A distinct implementation decision".to_owned();
    fs::write(warrant.join("questions/Q-ANSWERED.toml"), toml::to_string_pretty(&question).unwrap()).unwrap();
    fs::write(warrant.join("plan/request.json"), serde_json::to_vec(&request).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["--root", root.to_str().unwrap(), "--json", "interview", "IX-WAR-0003"])
        .env("OPENWARRANT_NO_PROJECTS", "1").env("OPENWARRANT_NO_UPDATE_CHECK", "1").output().unwrap();
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["result"]["questions"].as_array().unwrap().len(), 1, "{value}");
}
