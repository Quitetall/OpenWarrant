// SPDX-License-Identifier: Apache-2.0
//! Opt-in real model experiment; never substitutes for scripted conformance.
use super::*;
use capture::katana_process::{KatanaProcessConfig, KatanaProcessVerifier};
use std::time::Duration;

#[test]
#[ignore = "requires explicitly configured isolated local LAMU model and native Katana verifier"]
fn actual_model_edit_receipt_matches_recorded_dispatch_and_refuses_stale_basis() {
    let producer = PathBuf::from(std::env::var_os("OW_KATANA_REAL_PRODUCER").unwrap());
    let executable = PathBuf::from(std::env::var_os("OW_KATANA_NATIVE_VERIFIER").unwrap());
    assert!(producer.is_absolute() && executable.is_absolute());
    let f = Fixture::with_runtime_count(1);
    let exported = war(
        &f.root,
        &[
            "runtime",
            "binding",
            "IX-WAR-0003",
            &f.dispatch.dispatch_id,
            "--json",
        ],
    );
    assert!(exported.status.success());
    let binding = f.root.join("checked-binding.json");
    fs::write(
        &binding,
        serde_json::to_vec(&value(&exported)["result"]["binding"]).unwrap(),
    )
    .unwrap();
    let native = f.root.join("real-local-turn");
    // Separate, opt-in experiment envelope. The scripted control remains 30s.
    let produced = Command::new("/usr/bin/timeout")
        .arg("180")
        .arg(producer)
        .arg(&binding)
        .arg(&native)
        .output()
        .unwrap();
    assert!(
        produced.status.success(),
        "{} {}",
        String::from_utf8_lossy(&produced.stdout),
        String::from_utf8_lossy(&produced.stderr)
    );
    let metadata: Value =
        serde_json::from_slice(&fs::read(native.join("fixture.json")).unwrap()).unwrap();
    assert_eq!(metadata["scripted_provider"], false);
    assert_eq!(metadata["task_verified"], true);
    assert_eq!(metadata["cost_metered"], false);
    let interface = ProviderInterface {
        kind: ProviderKind::Katana,
        identity: "actual-local-katana-real-model-experiment".into(),
        version: "katana/openwarrant-verification/v1".into(),
    };
    let repo = openwarrant_cli::repo::Repository::open(f.root.clone().try_into().unwrap()).unwrap();
    let verifier = KatanaProcessVerifier::for_recorded_dispatch(
        &repo,
        "IX-WAR-0003",
        &f.dispatch.dispatch_id,
        KatanaProcessConfig {
            provider: interface.clone(),
            executable,
            event_log: native.join("session.jsonl"),
            // Test instrumentation, not independently protected production custody.
            trusted_log_head: metadata["expected_log_head"].as_str().unwrap().into(),
            scratch_root: f.root.clone(),
            timeout: Duration::from_secs(15),
            max_response_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    let raw = fs::read(native.join("receipt.json")).unwrap();
    let checked = verifier.verify(&raw).unwrap();
    assert_eq!(checked.binding, RuntimeBinding::from_dispatch(&f.dispatch));
    assert_eq!(checked.outcome, RuntimeOutcome::Completed);
    assert_eq!(checked.confinement, Observation::Unknown);
    assert_eq!(checked.metered_cost, Observation::Unknown);
    // Declared before execution, not copied from the returned receipt. The
    // provider reports tool permission upper bounds, not observed file effects.
    let allowed: Vec<String> = [
        r#"{"glob":"calc.py","kind":"fs","mode":"read"}"#,
        r#"{"glob":"calc.py","kind":"fs","mode":"write"}"#,
        r#"{"kind":"exec","profile":"sandboxed"}"#,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let policy = || Verification {
        verifier: &verifier,
        registry_digest: None,
        authorized_capabilities: Some(&allowed),
        confinement_required: false,
        hard_spend_cap_required: false,
    };
    let denied = assess_runtime_receipt(
        &RuntimeExpectation {
            dispatch: &f.dispatch,
            provider: &interface,
            registry_digest: None,
            authorized_capabilities: Some(&[]),
            confinement_required: false,
            hard_spend_cap_required: false,
            max_receipt_bytes: 4 * 1024 * 1024,
        },
        &raw,
        Some(&verifier),
    );
    assert_eq!(denied.standing, ReceiptStanding::Refused);
    fs::write(f.root.join("receipt.bin"), &raw).unwrap();
    let mut request = f.request();
    request["metadata"]["observation_id"] = json!(format!("real-local-{}", f.dispatch.attempt_id));
    request["metadata"]["observed_at"] = json!(chrono::Utc::now().to_rfc3339());
    request["metadata"]["binary_identity"] = json!(format!(
        "sha256:{}",
        metadata["cli_sha256"].as_str().unwrap()
    ));
    request["metadata"]["source_identity"] =
        json!("installed CLI source unestablished; exact binary hash retained");
    request["metadata"]["status"] = json!("completed");
    request["metadata"]["argv"] =
        serde_json::from_slice(&fs::read(native.join("argv.json")).unwrap()).unwrap();
    request["provider"] =
        json!({"kind":"katana","identity":interface.identity,"version":interface.version});
    request["metadata"]["transport"] =
        json!("actual local model, bounded edit and native receipt experiment");
    request["metadata"]["original_receipt_ref"] =
        json!(native.join("receipt.json").to_str().unwrap());
    let response = capture::import(
        &repo,
        "IX-WAR-0003",
        &serde_json::from_value(request).unwrap(),
        Some(policy()),
    )
    .unwrap();
    assert_eq!(response["native_observation"]["standing"], "matches");
    assert_eq!(response["assurance_granted"], false);
    let selected = [capture::Selection {
        stage_id: f.dispatch.stage_id.clone(),
        capture_digest: response["digest"].as_str().unwrap().into(),
    }];
    assert_eq!(
        capture::assess_selected(&repo, "IX-WAR-0003", &selected, |_, _| Some(policy()))
            .unwrap()
            .standing,
        ReceiptStanding::Matches
    );
    let one = repo
        .load_warrant(&repo.warrant_dir("IX-WAR-0003").unwrap())
        .unwrap();
    let resolve_native = |_: &StageDispatch, _: &ProviderInterface| Some(policy());
    let resolution = openwarrant_cli::resolve::assess_with_runtime(
        &repo,
        &one,
        &[],
        openwarrant_cli::resolve::runtime::Input {
            selections: &selected,
            native: &resolve_native,
        },
    )
    .unwrap();
    assert!(resolution.checks.runtime_receipts_match_the_basis);
    assert!(!resolution.checks.exact_authorized_contract_revision);
    assert!(!resolution.checks.independence_requirements_met);
    for (confinement_required, hard_spend_cap_required, code) in [
        (true, false, "runtime.confinement-unestablished"),
        (false, true, "runtime.accounting-unestablished"),
    ] {
        let required = assess_runtime_receipt(
            &RuntimeExpectation {
                dispatch: &f.dispatch,
                provider: &interface,
                registry_digest: None,
                authorized_capabilities: Some(&allowed),
                confinement_required,
                hard_spend_cap_required,
                max_receipt_bytes: 4 * 1024 * 1024,
            },
            &raw,
            Some(&verifier),
        );
        assert_eq!(required.standing, ReceiptStanding::Unknown);
        assert_eq!(required.code, code);
    }
    if let Some(destination) = std::env::var_os("OW_KATANA_REAL_RETAIN") {
        fn copy_records(source: &Path, destination: &Path) {
            fs::create_dir_all(destination).unwrap();
            for entry in fs::read_dir(source).unwrap() {
                let entry = entry.unwrap();
                let kind = entry.file_type().unwrap();
                let target = destination.join(entry.file_name());
                if kind.is_dir() {
                    copy_records(&entry.path(), &target);
                } else {
                    assert!(kind.is_file(), "record retention refuses special files");
                    fs::copy(entry.path(), target).unwrap();
                }
            }
        }
        let retained =
            PathBuf::from(destination).join(format!("imported-{}", f.dispatch.attempt_id));
        assert!(retained.is_absolute() && !retained.starts_with(&f.root));
        fs::create_dir(&retained).unwrap();
        copy_records(&f.root.join("docs"), &retained.join("docs"));
        fs::copy(
            f.root.join("openwarrant.toml"),
            retained.join("openwarrant.toml"),
        )
        .unwrap();
        fs::write(
            retained.join("capture-import.json"),
            serde_json::to_vec_pretty(&response).unwrap(),
        )
        .unwrap();
    }
    let intent = f.root.join("docs/warrants/IX-WAR-0003/atoms/10-intent.md");
    let mut changed = fs::read(&intent).unwrap();
    changed.extend_from_slice(b"\nChanged task after native completion.\n");
    fs::write(intent, changed).unwrap();
    let stale =
        capture::assess_selected(&repo, "IX-WAR-0003", &selected, |_, _| Some(policy())).unwrap();
    assert_eq!(stale.standing, ReceiptStanding::Refused);
    assert_eq!(
        stale.stages[0].receipt.code,
        "runtime-basis.dispatch-mismatch"
    );
    println!(
        "PASS actual local model edit/tests/native capture/current binding; excess capability and stale basis refused; no assurance"
    );
}
