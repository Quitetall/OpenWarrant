// SPDX-License-Identifier: Apache-2.0
//! Synthetic public delivery controls; no production provenance or authority.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        fn copy(from: &Path, to: &Path) {
            fs::create_dir_all(to).unwrap();
            for entry in fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                let target = to.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copy(&entry.path(), &target)
                } else {
                    fs::copy(entry.path(), target).unwrap();
                }
            }
        }
        let root = std::env::temp_dir().join(format!(
            "ow-delivery-provenance-{}-{}",
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
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn changed_delivery_uses_explicit_attempt_and_inputs_without_granting_authority() {
    delivery_case(false, false);
}

#[test]
fn explicit_inputs_replace_multiline_values_and_preserve_surrounding_comments() {
    delivery_case(true, false);
}

#[test]
fn observed_zero_inputs_can_clear_the_old_list_without_inventing_a_digest() {
    delivery_case(false, true);
}

fn delivery_case(multiline: bool, clear: bool) {
    let f = Fixture::new();
    let manifest = f.0.join("docs/warrants/IX-WAR-0003/deliverables.toml");
    let authority = f.0.join("docs/warrants/IX-WAR-0003/authorization.toml");
    let authority_before = fs::read(&authority).ok();
    fs::write(f.0.join("payload.txt"), "after\n").unwrap();
    fs::write(f.0.join("input-a.txt"), "input-a\n").unwrap();
    fs::write(f.0.join("input-b.txt"), "input-b\n").unwrap();
    fs::write(
        &manifest,
        r#"schema = "oh.war/deliverables/v1"
[[deliverable]]
id = "D-001"
title = "synthetic modified file"
kind = "file"
target_ref = "payload.txt"
required = true
content_addressed = true
provenance_required = true
obligation_refs = ["OBL-001"]
[deliverable.provenance]
producer = "fixture-old-producer"
producing_attempt = "fixture-old-attempt"
contract_digest = "fixture-old-contract"
input_digests = ["sha256:9160d4be34c8695bd172a76c7c7966587ea5a4d991ad22c87b2b91af54aa9ebb"]
tool_or_runtime_identity = "synthetic prior tool"
creation_method = "synthetic prior creation"
content_digest = "sha256:9160d4be34c8695bd172a76c7c7966587ea5a4d991ad22c87b2b91af54aa9ebb"
media_type = "text/plain"
classification = "internal"
retention = "repository-lifetime"
source_holder = "synthetic fixture"
"#,
    )
    .unwrap();
    let original = fs::read_to_string(&manifest).unwrap();
    let original = if multiline {
        original.replace("input_digests = [\"sha256:9160d4be34c8695bd172a76c7c7966587ea5a4d991ad22c87b2b91af54aa9ebb\"]", "input_digests = [\n # old value comment\n 'sha256:9160d4be34c8695bd172a76c7c7966587ea5a4d991ad22c87b2b91af54aa9ebb',\n] # input list comment")
    } else {
        original
    };
    fs::write(&manifest, format!("# 保留 surrounding comment\n{original}")).unwrap();
    let before = fs::read(&manifest).unwrap();
    let mut args = vec![
        "deliver",
        "IX-WAR-0003",
        "D-001",
        "--producer",
        "fixture-current-producer",
        "--producing-attempt",
        "fixture-current-attempt",
        "--method",
        "synthetic modification with explicitly observed inputs",
    ];
    if clear {
        args.push("--clear-input-digests");
    } else {
        args.extend([
            "--input-digest",
            "sha256:f5af18cdbc3f8f05a5ef469305dfaaada2189ef1a53789de1bba53b76409d087",
            "--input-digest",
            "sha256:14bffbef7013cb524932984251803d55670758928cdae5108cfdaca399e3313f",
        ]);
    }
    args.push("--json");
    args.push("D-999");
    let refused = f.run(&args);
    assert_ne!(refused["exit_code"], 0, "{refused}");
    assert!(
        refused["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "deliver.unknown-id"),
        "{refused}"
    );
    assert_eq!(fs::read(&manifest).unwrap(), before);
    args.pop();
    args.push("--dry-run");
    let dry = f.run(&args);
    assert_eq!(dry["exit_code"], 0, "{dry}");
    assert_eq!(fs::read(&manifest).unwrap(), before);
    args.pop();
    let recorded = f.run(&args);
    assert_eq!(recorded["exit_code"], 0, "{recorded}");
    let after = fs::read(&manifest).unwrap();
    let parsed: toml::Value = toml::from_str(std::str::from_utf8(&after).unwrap()).unwrap();
    let d = &parsed["deliverable"][0];
    let p = &d["provenance"];
    assert_eq!(d["id"].as_str(), Some("D-001"));
    assert_eq!(d["target_ref"].as_str(), Some("payload.txt"));
    assert_eq!(p["producer"].as_str(), Some("fixture-current-producer"));
    assert_eq!(
        p["producing_attempt"].as_str(),
        Some("fixture-current-attempt")
    );
    let expected_inputs = if clear {
        vec![]
    } else {
        vec![
            toml::Value::from(
                "sha256:f5af18cdbc3f8f05a5ef469305dfaaada2189ef1a53789de1bba53b76409d087",
            ),
            toml::Value::from(
                "sha256:14bffbef7013cb524932984251803d55670758928cdae5108cfdaca399e3313f",
            ),
        ]
    };
    assert_eq!(p["input_digests"].as_array().unwrap(), &expected_inputs);
    assert!(
        std::str::from_utf8(&after)
            .unwrap()
            .contains("# 保留 surrounding comment")
    );
    if multiline {
        assert!(
            std::str::from_utf8(&after)
                .unwrap()
                .contains("# input list comment")
        );
    }
    assert_eq!(
        p["content_digest"].as_str(),
        Some("sha256:7b9a72466d3960eb2aacccfc848939453490db0678bd4725def3f789b891c919")
    );
    let replay = f.run(&args);
    assert_eq!(replay["exit_code"], 0, "{replay}");
    assert!(
        replay["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "deliver.current"),
        "{replay}"
    );
    assert_eq!(fs::read(&manifest).unwrap(), after);
    assert_eq!(fs::read(authority).ok(), authority_before);
    assert!(
        !f.0.join("docs/warrants/IX-WAR-0003/resolution.toml")
            .exists()
    );
}
