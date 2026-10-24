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
fn context_inputs_reach_the_agent_with_exact_sources() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let tmp = Temp(std::env::temp_dir().join(format!("ow-eval-context-{}-{}",
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
    task.as_table_mut().unwrap().insert(
        "context".into(),
        toml::Value::Array(vec![toml::Value::String("CONTEXT.md".into())]),
    );
    let wrapper = tmp.0.join("context-agent.py");
    std::fs::write(
        &wrapper,
        format!(
            r#"#!/usr/bin/env python3
import json,os,pathlib,subprocess,sys
packet=json.loads(pathlib.Path(os.environ['EVAL_CONTEXT_FILE']).read_text())
assert packet['schema']=='oh.war/eval-context/v1'
assert packet['documents'][0]['path']=='CONTEXT.md'
assert packet['documents'][0]['body'].startswith('# OpenWarrant context')
subprocess.run([{}, *sys.argv[1:]], input=sys.stdin.buffer.read(), check=True)
"#,
            serde_json::to_string(
                &repo
                    .join("evals/fixtures/fixture-agent.sh")
                    .to_string_lossy()
            )
            .unwrap()
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::write(task_dir.join("task.toml"), toml::to_string(&task).unwrap()).unwrap();
    let output = tmp.0.join("result.json");
    let run = Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(repo)
        .args(["eval", "run", "--task", "answers", "--tasks-dir"])
        .arg(tmp.0.join("tasks"))
        .arg("--drafter")
        .arg(&wrapper)
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
    let packet: serde_json::Value = serde_json::from_slice(
        &std::fs::read(scratch.0.join(".openwarrant/eval-context.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        packet["documents"][0]["body"],
        std::fs::read_to_string(repo.join("CONTEXT.md")).unwrap()
    );
    assert!(
        packet["documents"][0]["sha256"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert_eq!(task["score"], "would_satisfy", "{task}");
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

#[test]
fn eval_context_refuses_paths_outside_the_repository_or_missing_sources() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let tmp = Temp(std::env::temp_dir().join(format!(
            "ow-eval-context-refusal-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    let task_dir = tmp.0.join("tasks/context");
    std::fs::create_dir_all(&task_dir).unwrap();
    let original =
        std::fs::read_to_string(repo.join("evals/tasks/code-01-changelog/task.toml")).unwrap();
    for path in [
        "../CONTEXT.md".to_owned(),
        repo.join("CONTEXT.md").to_string_lossy().into_owned(),
        "missing-eval-context-source.md".to_owned(),
    ] {
        let mut task: toml::Value = toml::from_str(&original).unwrap();
        task.as_table_mut().unwrap().insert(
            "context".into(),
            toml::Value::Array(vec![toml::Value::String(path.clone())]),
        );
        std::fs::write(task_dir.join("task.toml"), toml::to_string(&task).unwrap()).unwrap();
        let run = Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(repo)
            .args(["eval", "run", "--task", "context", "--tasks-dir"])
            .arg(tmp.0.join("tasks"))
            .arg("--drafter")
            .arg(repo.join("evals/fixtures/fixture-agent.sh"))
            .arg("--verifier")
            .arg(repo.join("conformance/fixtures/verifier/establishes-all.sh"))
            .arg("--out")
            .arg(tmp.0.join("result.json"))
            .env("OPENWARRANT_NO_PROJECTS", "1")
            .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
            .output()
            .unwrap();
        let output = format!(
            "{}{}",
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        );
        assert!(
            !run.status.success() && output.contains("eval.context-invalid"),
            "{path}: {output}"
        );
    }
}

#[test]
fn skill_fixtures_preserve_answers_and_refuse_false_completion() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let tmp = Temp(std::env::temp_dir().join(format!(
            "ow-skill-cases-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    std::fs::create_dir_all(&tmp.0).unwrap();
    let output = tmp.0.join("result.json");
    let run = Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(repo)
        .args(["eval", "run", "--tasks-dir"])
        .arg(repo.join("evals/skills/tasks"))
        .arg("--drafter")
        .arg(repo.join("evals/skills/fixture-agent.py"))
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
    let tasks = result["tasks"].as_array().unwrap();
    let scratches: Vec<_> = tasks
        .iter()
        .map(|t| {
            Temp(std::path::PathBuf::from(
                t["notes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|n| n.as_str())
                    .find_map(|n| n.strip_prefix("scratch kept at "))
                    .unwrap(),
            ))
        })
        .collect();
    assert_eq!(tasks.len(), 5);
    for (task, scratch) in tasks.iter().zip(&scratches) {
        let id = task["id"].as_str().unwrap();
        let expected = match id {
            "grill" | "spec" => "would_satisfy",
            "tickets" => "partial",
            "grill-unanswered" => "needs_input",
            "grill-12k-budget" => "over_budget",
            _ => panic!("unexpected task {id}"),
        };
        assert_eq!(task["score"], expected, "{task}");
        assert_eq!(task["as_expected"], true, "{task}");
        if id == "grill" {
            assert!(
                std::fs::read_to_string(scratch.0.join("EXPORT.md"))
                    .unwrap()
                    .contains("Answer: Markdown")
            );
        }
        if id == "tickets" {
            assert!(scratch.0.join("API.md").is_file());
            assert!(!scratch.0.join("CLIENT.md").exists());
            assert!(
                task["requirements_unmet_by_the_loop"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v == "required deliverables exist")
            );
        }
        if id == "grill-unanswered" {
            assert!(task["warrant"].is_null());
            assert!(scratch.0.join("docs/intake").is_dir());
            assert!(
                task["notes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|n| n.as_str())
                    .any(|n| n.starts_with("interview awaiting input:"))
            );
        }
    }
}
