// SPDX-License-Identifier: Apache-2.0
use std::process::Command;
struct Temp(std::path::PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn interview_answers_reach_the_recorded_draft_request_as_data() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let tmp = Temp(std::env::temp_dir().join(format!("ow-eval-answers-{}-{}",
        std::process::id(), std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())));
    let task_dir = tmp.0.join("tasks/answers");
    std::fs::create_dir_all(&task_dir).unwrap();
    let source = repo.join("evals/tasks/code-01-changelog");
    for name in [
        "task.toml",
        "proposal.json",
        "deliver/CHANGELOG.md",
        "deliver/docs/warrants/WARRANT_ALIAS/deliverables.toml.tmpl",
    ] {
        let target = task_dir.join(name);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(source.join(name), target).unwrap();
    }
    let answer = "Markdown with \"quotes\"\nand literal $(not_a_command)";
    let mut task: toml::Value =
        toml::from_str(&std::fs::read_to_string(task_dir.join("task.toml")).unwrap()).unwrap();
    task.as_table_mut().unwrap().insert(
        "answers".into(),
        toml::Value::Table(toml::Table::from_iter([(
            "Q-001".into(),
            toml::Value::String(answer.into()),
        )])),
    );
    std::fs::write(task_dir.join("task.toml"), toml::to_string(&task).unwrap()).unwrap();
    let output = tmp.0.join("result.json");
    let run = Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(repo)
        .args(["eval", "run", "--task", "answers", "--tasks-dir"])
        .arg(tmp.0.join("tasks"))
        .arg("--drafter")
        .arg(repo.join("evals/fixtures/fixture-agent.sh"))
        .arg("--verifier")
        .arg(repo.join("conformance/fixtures/verifier/establishes-all.sh"))
        .args(["--keep", "--out"])
        .arg(&output)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    let task = &result["tasks"][0];
    let scratch = Temp(std::path::PathBuf::from(
        task["notes"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|n| n.as_str())
            .find_map(|n| n.strip_prefix("scratch kept at "))
            .unwrap(),
    ));
    let alias = task["warrant"].as_str().expect("draft must be applied");
    let request: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            scratch
                .0
                .join("docs/warrants")
                .join(alias)
                .join("plan/request.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(request["answers"]["Q-001"], answer);
}

#[test]
fn document_evaluation_reviews_the_final_gate_evidence() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let tmp = Temp(std::env::temp_dir().join(format!("ow-eval-subject-{}-{}",
        std::process::id(), std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())));
    std::fs::create_dir(&tmp.0).unwrap();
    let output = tmp.0.join("result.json");
    let run = Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(repo)
        .args(["eval", "run", "--task", "doc-01-tokenizer", "--tasks-dir"])
        .arg(repo.join("evals/tasks"))
        .arg("--drafter")
        .arg(repo.join("evals/fixtures/fixture-agent.sh"))
        .arg("--verifier")
        .arg(repo.join("conformance/fixtures/verifier/establishes-all.sh"))
        .arg("--out")
        .arg(&output)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let result: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    let task = &result["tasks"][0];
    assert_eq!(task["score"], "would_satisfy", "{task}");
    assert_eq!(task["obligations_established"], 1);
    assert_eq!(
        task["requirements_unmet_by_the_loop"],
        serde_json::json!([])
    );
    let names: Vec<_> = task["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    let evidence = names
        .iter()
        .position(|n| *n == "evidence.record.after-review")
        .unwrap();
    let review = names
        .iter()
        .position(|n| *n == "verify.run.after-evidence")
        .unwrap();
    assert!(evidence < review);
    assert_eq!(task["tokens"]["stable"], false);
}
