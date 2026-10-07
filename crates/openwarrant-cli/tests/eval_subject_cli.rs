// SPDX-License-Identifier: Apache-2.0
use std::process::Command;
struct Temp(std::path::PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
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
