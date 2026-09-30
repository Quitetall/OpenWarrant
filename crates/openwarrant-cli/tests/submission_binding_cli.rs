// SPDX-License-Identifier: Apache-2.0
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        fn copy(a: &std::path::Path, b: &std::path::Path) {
            fs::create_dir_all(b).unwrap();
            for entry in fs::read_dir(a).unwrap() {
                let entry = entry.unwrap();
                let dest = b.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copy(&entry.path(), &dest);
                } else {
                    fs::copy(entry.path(), dest).unwrap();
                }
            }
        }
        let root = std::env::temp_dir().join(format!(
            "ow-submit-binding-{}-{}",
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
        let graph = root.join("docs/warrants/IX-WAR-0003/atoms/45-milestones.yaml");
        fs::write(
            &graph,
            fs::read_to_string(&graph).unwrap().replace(
                "executor_kind: \"agent\"",
                "executor_kind: \"agent\"\n    executor_ref: \"agent://fixture\"",
            ),
        )
        .unwrap();
        Self(root)
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(&self.0)
            .args(args)
            .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
            .env("OPENWARRANT_NO_PROJECTS", "1")
            .output()
            .unwrap()
    }
    fn submission(&self) -> serde_json::Value {
        let out = self.run(&[
            "dispatch",
            "IX-WAR-0003",
            "STAGE-001",
            "--prototype",
            "--emit",
            "packet.json",
        ]);
        assert!(
            out.status.success(),
            "{} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let packet: serde_json::Value =
            serde_json::from_slice(&fs::read(self.0.join("packet.json")).unwrap()).unwrap();
        serde_json::json!({"dispatch_id":packet["dispatch_id"],"attempt_id":packet["attempt_id"],
            "contract_digest":packet["contract_digest"],"stage_id":packet["stage_id"],"requested_next_action":"verify"})
    }
    fn submit(&self, submission: &serde_json::Value) -> Output {
        fs::write(
            self.0.join("submission.json"),
            serde_json::to_vec(submission).unwrap(),
        )
        .unwrap();
        self.run(&["submit", "IX-WAR-0003", "submission.json", "--json"])
    }
    fn journal(&self) -> Vec<u8> {
        fs::read(self.0.join("docs/warrants/IX-WAR-0003/journal.jsonl")).unwrap()
    }
    fn recorded(&self, submission: &serde_json::Value) -> PathBuf {
        self.0
            .join("docs/warrants/IX-WAR-0003/submissions")
            .join(format!(
                "{}.json",
                submission["dispatch_id"].as_str().unwrap()
            ))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_known_dispatch_does_not_admit_a_different_contract() {
    let fixture = Fixture::new();
    let good = fixture.submission();
    let journal = fixture.journal();
    let mut bad = good.clone();
    bad["contract_digest"] =
        "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".into();
    let out = fixture.submit(&bad);
    assert!(
        !out.status.success(),
        "false contract was admitted: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "submission.dispatch-mismatch")
    );
    assert!(!fixture.recorded(&bad).exists());
    assert_eq!(
        fixture.journal(),
        journal,
        "refusal must not append a submission event"
    );
    let out = fixture.submit(&good);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stored: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.recorded(&good)).unwrap()).unwrap();
    assert_eq!(stored["contract_digest"], good["contract_digest"]);
}
