// SPDX-License-Identifier: Apache-2.0
use std::{fs, path::PathBuf, process::Command};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        fn copy(from: &std::path::Path, to: &std::path::Path) {
            fs::create_dir_all(to).unwrap();
            for entry in fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    copy(&entry.path(), &to.join(entry.file_name()));
                } else {
                    fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
                }
            }
        }
        let name = format!(
            "ow-decision-scaffold-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(name);
        copy(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../conformance/fixtures/inbox/repository"),
            &path,
        );
        Self(path)
    }
    fn run(&self, args: &[&str]) -> serde_json::Value {
        let out = Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(&self.0)
            .arg("--json")
            .args(args)
            .env("OPENWARRANT_NO_PROJECTS", "1")
            .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
            .output()
            .unwrap();
        serde_json::from_slice(&out.stdout).unwrap_or_else(|_| {
            panic!(
                "{} {}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
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
fn decision_preset_binds_a_distinct_proposed_adr_outside_its_warrant() {
    let fixture = Fixture::new();
    let mut previous = None;
    for _ in 0..2 {
        let created = fixture.run(&["plan", "new", "Choose cache mode", "--preset", "decision"]);
        assert_eq!(created["exit_code"], 0, "{created}");
        let alias = created["result"]["alias"].as_str().unwrap();
        let dir = fixture.0.join("docs/warrants").join(alias);
        let manifest: toml::Value =
            toml::from_str(&fs::read_to_string(dir.join("manifest.toml")).unwrap()).unwrap();
        let row = manifest["atoms"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["role"].as_str() == Some("adr"))
            .unwrap();
        let source = dir
            .join(row["path"].as_str().unwrap())
            .canonicalize()
            .unwrap();
        assert!(
            !source.starts_with(dir.canonicalize().unwrap()),
            "bound ADR must have its own source outside the Warrant"
        );
        let text = fs::read_to_string(&source).unwrap();
        assert!(text.contains("jurisdiction: bound"));
        let adr = openwarrant_core::adr::AdrRecord::parse(source.to_str().unwrap(), &text).unwrap();
        assert_eq!(adr.status, openwarrant_core::adr::AdrStatus::Proposed);
        assert!(
            adr.governs
                .contains(&format!("war://{}", manifest["uuid"].as_str().unwrap()))
        );
        if let Some((path, bytes, id)) = &previous {
            assert_eq!(fs::read(path).unwrap(), *bytes);
            assert_ne!(&adr.uuid, id);
        }
        previous = Some((source, text.into_bytes(), adr.uuid));
        let checked = fixture.run(&["check", alias]);
        assert_eq!(checked["counts"]["error"], 0, "{checked}");
    }
}

#[test]
fn decision_scaffold_preserves_quoted_titles_and_configured_adr_paths() {
    let fixture = Fixture::new();
    let config = fixture.0.join("openwarrant.toml");
    let mut settings: toml::Value = toml::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
    settings["paths"]["adrs"] = toml::Value::String("docs/decision \"drafts\"".into());
    fs::write(config, toml::to_string(&settings).unwrap()).unwrap();
    let title = "Choose \"short\" cache retention";
    let created = fixture.run(&["plan", "new", title, "--preset", "decision"]);
    assert_eq!(created["exit_code"], 0, "{created}");
    let alias = created["result"]["alias"].as_str().unwrap();
    let dir = fixture.0.join("docs/warrants").join(alias);
    let manifest: toml::Value =
        toml::from_str(&fs::read_to_string(dir.join("manifest.toml")).unwrap()).unwrap();
    assert_eq!(manifest["title"].as_str(), Some(title));
    let row = manifest["atoms"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["role"].as_str() == Some("adr"))
        .unwrap();
    let source = dir
        .join(row["path"].as_str().unwrap())
        .canonicalize()
        .unwrap();
    assert!(
        source.starts_with(
            fixture
                .0
                .join("docs/decision \"drafts\"/atoms")
                .canonicalize()
                .unwrap()
        )
    );
    let checked = fixture.run(&["check", alias]);
    assert_eq!(checked["counts"]["error"], 0, "{checked}");
}

#[test]
fn check_validates_rationale_without_resolving_a_blocking_unknown() {
    let fixture = Fixture::new();
    let created = fixture.run(&[
        "plan",
        "new",
        "Map unresolved retention",
        "--preset",
        "decision",
    ]);
    assert_eq!(created["exit_code"], 0, "{created}");
    let alias = created["result"]["alias"].as_str().unwrap();
    let path = fixture
        .0
        .join("docs/warrants")
        .join(alias)
        .join("rationale.toml");
    let source = r#"schema = "oh.war/rationale/v1"
[[assumption]]
id = "A-001"
statement = "The fixture has no selected retention mode."
epistemic_status = "blocking_unknown"
resolution_requirement = "The fixture owner supplies the missing mode."
judgment_ref = ""
"#;
    fs::write(&path, source).unwrap();
    let checked = fixture.run(&["check", alias]);
    assert_eq!(checked["counts"]["error"], 0, "{checked}");
    assert!(
        checked["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "rationale.parsed"
                && d["message"]
                    .as_str()
                    .unwrap()
                    .contains("A-001=blocking_unknown")),
        "{checked}"
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), source);
    for invalid in [
        source.replace("blocking_unknown", "invented_status"),
        source.replace("The fixture owner supplies the missing mode.", ""),
    ] {
        fs::write(&path, &invalid).unwrap();
        let refused = fixture.run(&["check", alias]);
        assert_eq!(refused["exit_code"], 2, "{refused}");
        assert!(
            refused["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["rule"] == "rationale.invalid"),
            "{refused}"
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), invalid);
    }
}
