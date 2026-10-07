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
            "ow-normative-{}-{}",
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
        fs::create_dir_all(root.join("docs/sas")).unwrap();
        Self(root)
    }
    fn source(&self, text: &str) {
        fs::write(
            self.0
                .join("docs/sas/WAR_Software_Architecture_Specification.md"),
            text,
        )
        .unwrap();
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
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn recompiling_cannot_hide_a_numbered_section_whose_rules_were_dropped() {
    let fixture = Fixture::new();
    fixture
        .source("# Fixture SAS\n\n## 8-bis. Added rules\n\nThe worker SHALL preserve evidence.\n");
    let compiled = fixture.run(&["compile", "--json"]);
    assert_eq!(compiled["exit_code"], 0, "{compiled}");
    let check = fixture.run(&["check", "--generated", "--json"]);
    assert!(
        check["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "sas-normative.section-dropped"
                && d["severity"] == "error"
                && d["message"]
                    .as_str()
                    .unwrap()
                    .contains("8-bis. Added rules")),
        "{check}"
    );

    fixture.source("# Fixture SAS\n\n## 8A. Added rules\n\n### 8A.1 Specific rule\n\nThe worker SHALL preserve evidence.\n");
    assert_eq!(fixture.run(&["compile", "--json"])["exit_code"], 0);
    let check = fixture.run(&["check", "--generated", "--json"]);
    assert!(
        !check["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "sas-normative.section-dropped"),
        "{check}"
    );
    let projection = fs::read_to_string(fixture.0.join("docs/sas/generated/NORMATIVE.md")).unwrap();
    assert!(
        projection.contains("§8A.1: The worker SHALL preserve evidence."),
        "{projection}"
    );
}
