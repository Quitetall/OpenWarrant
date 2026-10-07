// SPDX-License-Identifier: Apache-2.0
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn empty() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "ow-program-phases-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        Self(directory)
    }

    fn new(document: &str) -> Self {
        let fixture = Self::empty();
        let directory = fixture.0.clone();
        fs::create_dir_all(directory.join("docs/sas")).unwrap();
        fs::create_dir_all(directory.join("docs/warrants")).unwrap();
        fs::write(directory.join("openwarrant.toml"),
            "schema = \"oh.war/repository-config/v1\"\n[project]\nname = \"Program\"\nnamespace = \"DEMO\"\n").unwrap();
        fs::write(directory.join("docs/sas/Program.md"), document).unwrap();
        fixture
    }

    fn status(&self) -> serde_json::Value {
        let output = Command::new(env!("CARGO_BIN_EXE_war"))
            .args(["status", "--json"])
            .current_dir(&self.0)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["result"].clone()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn status_uses_the_programs_own_phases_without_requiring_eleven() {
    let fixture = Fixture::new(
        "## 98. Implementation phases\n\
        | Phase | Objective and exit |\n|---|---|\n\
        | 0 | Contract basis: source inventory complete |\n\
        | 8 | Institutional service: scoped access qualified |\n\
        ## 99. Other\n| 9 | Ignore: outside phase section |\n",
    );
    let status = fixture.status();
    let objectives = status["objectives"].as_array().unwrap();
    assert_eq!(objectives.len(), 3); // Two authored objectives plus unassigned.
    assert_eq!(objectives[0]["title"], "Contract basis");
    assert_eq!(objectives[0]["exit_criterion"], "source inventory complete");
    assert_eq!(objectives[1]["title"], "Institutional service");
    assert_eq!(objectives[1]["roadmap_ref"]["prefix"], "DEMO");
    assert_eq!(objectives[1]["roadmap_ref"]["phase"], 8);
    assert_ne!(objectives[1]["title"], "Liminal production compiler");
}

#[test]
fn missing_phase_declarations_do_not_invent_framework_objectives() {
    let fixture = Fixture::new("# Program\n## 98. Phases\nNot declared yet.\n");
    let status = fixture.status();
    assert_eq!(status["objectives"].as_array().unwrap().len(), 1);
    assert!(status["objectives"][0]["roadmap_ref"].is_null());
    assert!(
        status["caveats"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str().unwrap().contains("No phase could be read"))
    );
}

#[test]
fn conflicting_phase_declarations_stay_unknown() {
    let fixture = Fixture::new(
        "## 98. Phases\n| Phase | Objective and exit |\n|---|---|\n\
        | 0 | First: first exit |\n| 0 | Second: other exit |\n",
    );
    let status = fixture.status();
    assert_eq!(status["objectives"].as_array().unwrap().len(), 1);
    assert!(
        status["caveats"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| { value.as_str().unwrap().contains("declares one twice") })
    );
}

#[test]
fn undeclared_referenced_phase_stays_visible_without_an_invented_exit() {
    let fixture = Fixture::new("# Program with no declared phases\n");
    let created = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["new", "Test phase reference"])
        .current_dir(&fixture.0)
        .output()
        .unwrap();
    assert!(created.status.success());
    let path = fixture.0.join("docs/warrants/DEMO-WAR-0001/manifest.toml");
    let mut manifest = fs::read_to_string(&path).unwrap();
    manifest.push_str("\n[[roadmap]]\nref = \"roadmap://DEMO-PHASE-8\"\n");
    fs::write(path, manifest).unwrap();
    let status = fixture.status();
    assert_eq!(status["objectives"][0]["title"], "Phase 8");
    assert!(status["objectives"][0]["exit_criterion"].is_null());
    assert_eq!(status["objectives"][0]["warrants"][0], "DEMO-WAR-0001");
    assert_eq!(
        status["objectives"][0]["achieved"]["state"],
        "not_derivable"
    );
    assert!(status["caveats"].as_array().unwrap().iter().any(|value| {
        value
            .as_str()
            .unwrap()
            .contains("name phase(s) 8 that the configured SAS does not declare")
    }));

    // The model lists no record for the undeclared phase: the Warrant's
    // relation to it stays visible and is reported as naming no record.
    let output = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["model", "--json"])
        .current_dir(&fixture.0)
        .output()
        .unwrap();
    let model = &serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["result"];
    let records = model["records"].as_array().unwrap();
    assert!(
        records.iter().all(|r| r["id"] != "DEMO-PHASE-8"),
        "{records:?}"
    );
    assert!(
        model["states"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["record"] != "DEMO-PHASE-8")
    );
    assert!(
        model["relations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["from"] == "DEMO-WAR-0001" && r["to"] == "DEMO-PHASE-8")
    );
    assert!(model["diagnostics"].as_array().unwrap().iter().any(|d| {
        d["rule"] == "model.relation-target-unknown" && d["record"] == "DEMO-WAR-0001"
    }));
}

/// The program `war init --program` scaffolds declares two phases in its SAS
/// and has no roadmap record. Its objectives are those two, under its own
/// namespace, and its adopt Warrant sits in its own Phase 1, not in the
/// framework's eleven.
#[test]
fn a_scaffolded_program_projects_its_own_two_phases() {
    let fixture = Fixture::empty();
    let root = fixture.0.to_str().unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_war"))
        .args([
            "init",
            "--program",
            "Demo",
            "--namespace",
            "DM",
            "--root",
            root,
        ])
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let status = fixture.status();
    let titles: Vec<&str> = status["objectives"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["title"].as_str().unwrap())
        .collect();
    assert_eq!(
        titles,
        [
            "Adopt OpenWarrant",
            "First real work",
            "unassigned — declares no [[roadmap]]"
        ]
    );
    assert_eq!(status["objectives"][0]["roadmap_ref"]["prefix"], "DM");
    assert_eq!(status["objectives"][0]["roadmap_ref"]["phase"], 1);
    assert_eq!(status["objectives"][0]["warrants"][0], "DM-WAR-0001");
    assert!(
        !titles.contains(&"File-native WAR compiler"),
        "the framework's phases stood in for the program's"
    );
}
