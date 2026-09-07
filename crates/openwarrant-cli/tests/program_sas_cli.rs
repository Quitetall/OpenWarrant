// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public-process refusal tests for selected program SAS phase authority.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new(namespace: &str, sas: &[u8], roadmap: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "openwarrant-program-sas-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("docs/sas/revisions")).unwrap();
        fs::create_dir_all(root.join("docs/warrants/LIM-WAR-0001/atoms")).unwrap();
        fs::write(
            root.join("openwarrant.toml"),
            format!(
                "schema = \"oh.war/repository-config/v1\"\n[project]\nname = \"fixture\"\nnamespace = \"{namespace}\"\n[paths]\nsas = \"docs/sas\"\nroadmap = \"docs/roadmap\"\nadrs = \"docs/adr\"\nwarrants = \"docs/warrants\"\n[generated]\ncommit = false\nverify_drift = false\n"
            ),
        )
        .unwrap();
        copy_dir(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/gates"),
            &root.join("docs/gates"),
        );
        let source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/warrants/OW-WAR-0065/atoms");
        for atom in [
            "10-intent.md",
            "20-basis.md",
            "40-work-order.md",
            "45-milestones.yaml",
            "60-assurance.md",
        ] {
            let text = fs::read_to_string(source.join(atom)).unwrap().replace(
                "01a0746d-d84b-7401-bc57-fb7d6304aeb1",
                "01a00000-0000-7000-8000-000000000001",
            );
            fs::write(
                root.join("docs/warrants/LIM-WAR-0001/atoms").join(atom),
                text,
            )
            .unwrap();
        }
        fs::write(
            root.join("docs/warrants/LIM-WAR-0001/manifest.toml"),
            format!(
                r#"schema = "oh.war/manifest/v1"
uuid = "01a00000-0000-7000-8000-000000000001"
local_alias = "LIM-WAR-0001"
enterprise_id = ""
title = "fixture"
profile = "delivery"
assurance_level = "basic"
[[roadmap]]
ref = "{roadmap}"
[[atoms]]
ordinal = 10
role = "intent"
path = "atoms/10-intent.md"
required = true
[[atoms]]
ordinal = 20
role = "basis"
path = "atoms/20-basis.md"
required = true
[[atoms]]
ordinal = 40
role = "work_order"
path = "atoms/40-work-order.md"
required = true
[[atoms]]
ordinal = 45
role = "milestones"
path = "atoms/45-milestones.yaml"
required = true
[[atoms]]
ordinal = 60
role = "assurance"
path = "atoms/60-assurance.md"
required = true
"#
            ),
        )
        .unwrap();
        Self::write_sas(&root, sas);
        Self(root)
    }

    fn write_sas(root: &Path, bytes: &[u8]) {
        fs::write(root.join("docs/sas/SAS.md"), bytes).unwrap();
        // Independent expected snapshot: do not use the parser under test to
        // manufacture its own expected requirements.
        let requirements =
            std::collections::BTreeMap::from([("LIM-SAS-RQ-001".to_owned(), "fixture".to_owned())]);
        let revision = openwarrant_core::SasRevision::proposed(
            "test",
            "docs/sas/SAS.md",
            openwarrant_compiler::sha256_hex(bytes),
            None,
            requirements,
            false,
        );
        fs::write(
            root.join("docs/sas/revisions/test.toml"),
            toml::to_string(&revision).unwrap(),
        )
        .unwrap();
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_war"))
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn copy_dir(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn simple_sas(body: &str) -> String {
    format!("# SAS\n\n## 98. Phases\n{body}\n## 106. Requirements\n| LIM-SAS-RQ-001 | fixture |\n")
}

#[test]
fn fourteen_signed_phases_reach_public_check_and_status() {
    let declarations = (-1..=12)
        .map(|phase| format!("### Phase {phase} — phase {phase}"))
        .collect::<Vec<_>>()
        .join("\n");
    let sas = simple_sas(&declarations);
    for phase in -1..=12 {
        let fixture = Fixture::new(
            "LIM",
            sas.as_bytes(),
            &format!("roadmap://LIM-PHASE-{phase}/work"),
        );
        let checked = fixture.run(&["check", "LIM-WAR-0001"]);
        assert!(
            checked.status.success(),
            "phase {phase}: {}",
            output_text(&checked)
        );
        let status = fixture.run(&["status", "--json"]);
        assert!(status.status.success(), "{}", output_text(&status));
        let json: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
        assert!(
            json["objectives"]
                .as_array()
                .unwrap()
                .iter()
                .any(|o| o["roadmap_ref"]["phase"].as_i64() == Some(i64::from(phase)))
        );
        assert!(
            json["objectives"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|o| !o["roadmap_ref"].is_null())
                .all(|o| o["achieved"]["state"] == "not_derivable")
        );
    }
    let sas = simple_sas("### Phase 13 — declared successor");
    let fixture = Fixture::new("LIM", sas.as_bytes(), "roadmap://LIM-PHASE-13/work");
    let checked = fixture.run(&["check", "LIM-WAR-0001"]);
    assert!(checked.status.success(), "{}", output_text(&checked));
}

#[test]
fn malformed_duplicate_and_code_examples_are_classified_at_the_cli() {
    for sas in [
        simple_sas("### Phase  1 — padded"),
        simple_sas("### Phase 1 — one\n### Phase 1 — duplicate"),
        "## 98. A\n### Phase 1 — one\n## 98. B\n### Phase 2 — two\n| LIM-SAS-RQ-001 | fixture |"
            .to_owned(),
        simple_sas("### Phase\t1 — tab\n### Phase 1 — real"),
    ] {
        let fixture = Fixture::new("LIM", sas.as_bytes(), "roadmap://LIM-PHASE-1/work");
        let result = fixture.run(&["check", "LIM-WAR-0001"]);
        assert_eq!(result.status.code(), Some(2));
        let output = output_text(&result);
        assert!(output.contains("ERROR sas.authority-invalid"), "{output}");
        assert!(!output.contains("traceability.refs"), "{output}");
    }
    for fake in [
        "```md\n### Phase 1 — fake\n```",
        "~~~~md\n### Phase 1 — fake\n~~~~",
        "> ```\n> ### Phase 1 — fake\n> ```",
    ] {
        let sas = simple_sas(&format!("{fake}\n### Phase 2 — real"));
        let fixture = Fixture::new("LIM", sas.as_bytes(), "roadmap://LIM-PHASE-1/work");
        let result = fixture.run(&["check", "LIM-WAR-0001"]);
        assert_eq!(result.status.code(), Some(2));
        let output = output_text(&result);
        assert!(output.contains("roadmap.undeclared-phase"), "{output}");
    }
    let sas = simple_sas("```bad`info\n### Phase 1 — visible");
    let fixture = Fixture::new("LIM", sas.as_bytes(), "roadmap://LIM-PHASE-1/work");
    let output = fixture.run(&["check", "LIM-WAR-0001"]);
    assert!(output.status.success(), "{}", output_text(&output));
}

#[test]
fn unavailable_authority_is_unknown_but_invalid_utf8_is_error() {
    for mutation in 0..3 {
        let sas = simple_sas("### Phase 1 — one");
        let fixture = Fixture::new("LIM", sas.as_bytes(), "roadmap://LIM-PHASE-1/work");
        match mutation {
            0 => fs::remove_file(fixture.0.join("docs/sas/SAS.md")).unwrap(),
            1 => fs::write(fixture.0.join("docs/sas/second.md"), &sas).unwrap(),
            2 => fs::write(fixture.0.join("docs/sas/SAS.md"), format!("{sas}\ndrift")).unwrap(),
            _ => unreachable!(),
        }
        let result = fixture.run(&["check", "LIM-WAR-0001"]);
        assert_eq!(result.status.code(), Some(2));
        let output = output_text(&result);
        assert!(
            output.contains("UNKNOWN sas.authority-unavailable"),
            "{output}"
        );
    }
    let sas = simple_sas("### Phase 1 — one");
    let fixture = Fixture::new("LIM", sas.as_bytes(), "roadmap://LIM-PHASE-1/work");
    fs::remove_file(fixture.0.join("docs/sas/revisions/test.toml")).unwrap();
    let checked = fixture.run(&["check", "LIM-WAR-0001"]);
    assert!(checked.status.success(), "{}", output_text(&checked));
    assert!(output_text(&checked).contains("WARN sas.draft-inspection"));
    let status = fixture.run(&["status", "--json"]);
    assert!(status.status.success(), "{}", output_text(&status));
    let json: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(json["release"]["authority"]["state"], "draft");
    assert!(
        json["objectives"]
            .as_array()
            .unwrap()
            .iter()
            .all(|objective| {
                objective["roadmap_ref"].is_null()
                    || objective["achieved"]["state"] == "not_derivable"
            })
    );
    let sas = simple_sas("### Phase 1 — one");
    let fixture = Fixture::new("LIM", sas.as_bytes(), "roadmap://OW-PHASE-1/work");
    fs::remove_file(fixture.0.join("docs/sas/SAS.md")).unwrap();
    let result = fixture.run(&["check", "LIM-WAR-0001"]);
    assert_eq!(result.status.code(), Some(2));
    let output = output_text(&result);
    assert!(output.contains("ERROR roadmap.wrong-program"), "{output}");
    let sas = simple_sas("### Phase 1 — one");
    let fixture = Fixture::new("LIM", sas.as_bytes(), "roadmap://LIM-PHASE-1/work");
    Fixture::write_sas(&fixture.0, &[0xff, 0xfe]);
    let result = fixture.run(&["check", "LIM-WAR-0001"]);
    assert_eq!(result.status.code(), Some(2));
    let output = output_text(&result);
    assert!(output.contains("ERROR sas.authority-invalid"), "{output}");
}
