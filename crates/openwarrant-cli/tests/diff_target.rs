// SPDX-License-Identifier: Apache-2.0
use std::{fs, path::PathBuf, process::Command};

#[test]
fn explicit_target_reports_field_movement_and_ignores_json_layout() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temp = std::env::temp_dir().join(format!("ow-diff-target-{}", std::process::id()));
    fs::create_dir_all(&temp).unwrap();
    let from = temp.join("from.json");
    let to = temp.join("to.json");
    fs::write(&from, r#"{"contract_digest":"old","scope":{"limit":1}}"#).unwrap();
    fs::write(&to, r#"{"contract_digest":"new","scope":{"limit":2}}"#).unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(&root)
            .args(["diff", "OW-WAR-0037", "--from"])
            .arg(&from)
            .arg("--to")
            .arg(&to)
            .arg("--json")
            .output()
            .unwrap()
    };
    let output = run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("scope.limit: 1 -> 2"), "{text}");
    assert!(text.contains("contract_digest"), "{text}");
    fs::write(
        &to,
        "{\n\"scope\": {\"limit\":1}, \"contract_digest\":\"old\"\n}",
    )
    .unwrap();
    let output = run();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("diff.identical")
    );
    fs::write(&to, r#"{"scope":1,"scope":2}"#).unwrap();
    assert!(!run().status.success(), "duplicate members must refuse");
    fs::write(&to, "not JSON").unwrap();
    assert!(!run().status.success(), "malformed target must refuse");
    let key = "k".repeat(8192);
    fs::write(
        &from,
        serde_json::to_vec(&serde_json::json!({key.clone(): vec![0; 2048]})).unwrap(),
    )
    .unwrap();
    fs::write(
        &to,
        serde_json::to_vec(&serde_json::json!({key: vec![1; 2048]})).unwrap(),
    )
    .unwrap();
    let output = run();
    assert!(
        !output.status.success(),
        "amplified paths must refuse before walking"
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("comparison path budget"));
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn retained_contracts_name_digest_inputs_and_refuse_missing_history() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temp = std::env::temp_dir().join(format!("ow-contract-history-{}", std::process::id()));
    fs::create_dir_all(temp.join("docs/warrants/OW-WAR-0037/generated")).unwrap();
    fs::copy(root.join("openwarrant.toml"), temp.join("openwarrant.toml")).unwrap();
    fs::copy(
        root.join("docs/warrants/OW-WAR-0037/manifest.toml"),
        temp.join("docs/warrants/OW-WAR-0037/manifest.toml"),
    )
    .unwrap();
    let run_git = |args: &[&str]| {
        let out = Command::new("git")
            .current_dir(&temp)
            .args([
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    run_git(&["init", "-q"]);
    let mut ir: openwarrant_compiler::ir::WarIr = serde_json::from_slice(
        &fs::read(root.join("docs/warrants/OW-WAR-0037/generated/WAR.json")).unwrap(),
    )
    .unwrap();
    let mut auth: toml::Value = toml::from_str(
        &fs::read_to_string(root.join("docs/warrants/OW-WAR-0037/authorization.toml")).unwrap(),
    )
    .unwrap();
    let write_revision = |ir: &openwarrant_compiler::ir::WarIr, auth: &mut toml::Value| {
        // Synthetic retained records in a disposable repository, never production authority.
        auth["revision"]["revision"] = toml::Value::Integer(ir.contract_revision.into());
        auth["revision"]["contract_digest"] = toml::Value::String(ir.contract_digest().unwrap());
        fs::write(
            temp.join("docs/warrants/OW-WAR-0037/generated/WAR.json"),
            serde_json::to_vec(ir).unwrap(),
        )
        .unwrap();
        fs::write(
            temp.join("docs/warrants/OW-WAR-0037/authorization.toml"),
            toml::to_string(auth).unwrap(),
        )
        .unwrap();
        run_git(&["add", "."]);
        run_git(&["commit", "-qm", "synthetic snapshot"]);
    };
    write_revision(&ir, &mut auth);
    ir.contract_revision = 2;
    ir.identity.title = "Changed fixture outcome".into();
    write_revision(&ir, &mut auth);
    let compare = |from: &str, to: &str| {
        Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(&temp)
            .args(["diff", "OW-WAR-0037", "--from", from, "--to", to, "--json"])
            .output()
            .unwrap()
    };
    let out = compare("contract:1", "contract:2");
    assert!(
        out.status.success(),
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("diff.contract-input"), "{text}");
    assert!(text.contains("identity.title"), "{text}");
    let missing = compare("contract:1", "contract:99");
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stdout).contains("unavailable"));
    let omitted = Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(&temp)
        .args(["diff", "OW-WAR-0037", "--from", "contract:1", "--json"])
        .output()
        .unwrap();
    assert!(!omitted.status.success());
    assert!(String::from_utf8_lossy(&omitted.stdout).contains("requires explicit --to"));
    assert!(!compare("contract:01", "contract:2").status.success());
    ir.contract_revision = 1;
    write_revision(&ir, &mut auth);
    let ambiguous = compare("contract:1", "contract:2");
    assert!(!ambiguous.status.success());
    assert!(String::from_utf8_lossy(&ambiguous.stdout).contains("ambiguous"));
    auth["revision"]["contract_digest"] = toml::Value::String("tampered".into());
    fs::write(
        temp.join("docs/warrants/OW-WAR-0037/authorization.toml"),
        toml::to_string(&auth).unwrap(),
    )
    .unwrap();
    run_git(&["add", "."]);
    run_git(&["commit", "-qm", "synthetic tampered record"]);
    let tampered = compare("contract:1", "contract:2");
    assert!(!tampered.status.success());
    assert!(String::from_utf8_lossy(&tampered.stdout).contains("digest mismatch"));
    ir.contract_revision = 3;
    write_revision(&ir, &mut auth);
    let mut unknown = serde_json::to_value(&ir).unwrap();
    unknown["identity"]["future_boundary"] = serde_json::json!("must not disappear");
    fs::write(
        temp.join("docs/warrants/OW-WAR-0037/generated/WAR.json"),
        serde_json::to_vec(&unknown).unwrap(),
    )
    .unwrap();
    run_git(&["add", "."]);
    run_git(&["commit", "-qm", "synthetic unsupported input"]);
    let unsupported = compare("contract:3", "contract:3");
    assert!(
        !unsupported.status.success(),
        "unknown IR fields must not disappear before digest attribution"
    );
    assert!(String::from_utf8_lossy(&unsupported.stdout).contains("unsupported IR field"));
    let shallow = temp.with_extension("shallow");
    let cloned = Command::new("git")
        .args(["clone", "--depth", "1", "--quiet"])
        .arg(format!("file://{}", temp.display()))
        .arg(&shallow)
        .output()
        .unwrap();
    assert!(cloned.status.success());
    let refused = Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(&shallow)
        .args([
            "diff",
            "OW-WAR-0037",
            "--from",
            "contract:1",
            "--to",
            "contract:3",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stdout).contains("shallow repository"));
    fs::remove_dir_all(shallow).unwrap();
    fs::remove_dir_all(temp).unwrap();
}
