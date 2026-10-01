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
        let output = Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(&self.0)
            .args(args)
            .env("OPENWARRANT_NO_PROJECTS", "1")
            .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
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
        fs::write(self.0.join("response.toml"), text).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
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
