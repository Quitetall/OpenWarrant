// SPDX-License-Identifier: Apache-2.0
//! Actual local CLI captures; provider receipt fixtures and adapters are synthetic.
use openwarrant_cli::runtime_capture::{self as capture, Request, Verification};
use openwarrant_core::{
    document::{records::raw_digest, runtime::*},
    execution::StageDispatch,
    seam::KatanaReceipt,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

struct Fixture {
    root: PathBuf,
    dispatch: StageDispatch,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn war(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_war"))
        .args(args)
        .arg("--root")
        .arg(root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .output()
        .unwrap()
}
fn value(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{e}: {} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}
impl Fixture {
    fn new() -> Self {
        Self::with_runtime_count(usize::MAX)
    }
    fn with_runtime_count(count: usize) -> Self {
        fn copy(a: &Path, b: &Path) {
            fs::create_dir_all(b).unwrap();
            for e in fs::read_dir(a).unwrap() {
                let e = e.unwrap();
                if e.file_type().unwrap().is_dir() {
                    copy(&e.path(), &b.join(e.file_name()));
                } else {
                    fs::copy(e.path(), b.join(e.file_name())).unwrap();
                }
            }
        }
        let root = std::env::temp_dir().join(format!(
            "ow-capture-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        copy(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../conformance/fixtures/inbox/repository"),
            &root,
        );
        let graph = root.join("docs/warrants/IX-WAR-0003/atoms/45-milestones.yaml");
        let text = fs::read_to_string(&graph).unwrap().replacen(
            "executor_kind: \"agent\"",
            "executor_kind: \"katana\"\n    executor_ref: \"synthetic\"",
            count,
        );
        fs::write(graph, text).unwrap();
        let target = root.join("dispatch.pending.json");
        let out = war(
            &root,
            &[
                "dispatch",
                "IX-WAR-0003",
                "STAGE-001",
                "--prototype",
                "--emit",
                target.to_str().unwrap(),
            ],
        );
        assert!(
            out.status.success(),
            "{} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let dispatch: StageDispatch = serde_json::from_slice(&fs::read(&target).unwrap()).unwrap();
        fs::create_dir_all(root.join("docs/warrants/IX-WAR-0003/dispatches")).unwrap();
        fs::rename(
            target,
            root.join(format!(
                "docs/warrants/IX-WAR-0003/dispatches/{}.json",
                dispatch.dispatch_id
            )),
        )
        .unwrap();
        fs::write(
            root.join("receipt.bin"),
            b"synthetic native receipt bytes\0\xff",
        )
        .unwrap();
        let f = Self { root, dispatch };
        f.request();
        f
    }
    fn request(&self) -> Value {
        let req = json!({"schema":capture::REQUEST_SCHEMA,"dispatch_id":self.dispatch.dispatch_id,"receipt":"receipt.bin",
          "provider":{"kind":"katana","identity":"synthetic-build","version":"synthetic-interface/v1"},
          "metadata":{"observation_id":"synthetic-capture-1","observed_at":"2026-10-09T12:00:00Z","original_receipt_ref":"provider://synthetic/session/receipt","binary_identity":"synthetic-binary-declaration","source_identity":"synthetic-source-declaration","transport":"fixture-file","argv":[],"exit_code":0,"status":"synthetic-completed-status"}});
        fs::write(
            self.root.join("capture-request.json"),
            serde_json::to_vec(&req).unwrap(),
        )
        .unwrap();
        req
    }
    fn import(&self) -> Output {
        war(
            &self.root,
            &[
                "runtime",
                "import",
                "IX-WAR-0003",
                "--request",
                "capture-request.json",
                "--json",
            ],
        )
    }
    fn storage(&self) -> PathBuf {
        self.root.join("docs/warrants/IX-WAR-0003/runtime-receipts")
    }
}

#[test]
fn unverified_capture_is_replayable_and_survives_loss_of_original_inputs() {
    let f = Fixture::new();
    let out = f.import();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let response = value(&out);
    let digest = response["result"]["digest"].as_str().unwrap();
    assert_eq!(
        response["result"]["native_observation"]["standing"],
        "unknown"
    );
    assert_eq!(response["result"]["assurance_granted"], false);
    let path = f
        .root
        .join(response["result"]["reference"].as_str().unwrap());
    let bytes = fs::read(&path).unwrap();
    // Later unrelated journal events must not change exact replay identity.
    let journal = f.root.join("docs/warrants/IX-WAR-0003/journal.jsonl");
    let mut text = fs::read_to_string(&journal).unwrap();
    text.push_str(" \t\n");
    text.push_str(&serde_json::to_string(&json!({"v":1,"id":"synthetic-note-event","warrant_uuid":f.dispatch.warrant_ref.strip_prefix("war://").unwrap(),"type":"fixture.note","class":"draft_history","actor_ref":"agent://fixture","occurred_at":"2026-10-09T12:00:00Z","payload":"{}","idempotency_key":"fixture-note"})).unwrap());
    text.push('\n');
    fs::write(journal, text).unwrap();
    assert!(f.import().status.success());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fs::read_dir(f.storage()).unwrap().count(), 1);
    fs::remove_file(f.root.join("receipt.bin")).unwrap();
    fs::remove_dir_all(f.root.join("docs/warrants/IX-WAR-0003/dispatches")).unwrap();
    let out = war(
        &f.root,
        &["runtime", "show", "IX-WAR-0003", digest, "--json"],
    );
    assert!(out.status.success());
    let result = value(&out);
    assert_eq!(result["result"]["native_observation_is_trusted"], false);
    let raw = result["result"]["record"]["receipt"]["base64"]
        .as_str()
        .unwrap();
    assert_eq!(
        openwarrant_core::attestation::base64_decode(raw).unwrap(),
        b"synthetic native receipt bytes\0\xff"
    );
    assert_eq!(
        result["result"]["record"]["observation"]["provenance"]["authenticity"],
        "unverified"
    );
}

#[test]
fn archive_reconnects_capture_sources_offline_without_trusting_native_verdicts() {
    use openwarrant_compiler::preservation::{Archive, Limits};
    let f = Fixture::with_runtime_count(1);
    let imported = f.import();
    assert!(imported.status.success());
    let archive_path = f.root.join("source-archive.json");
    let exported = war(
        &f.root,
        &[
            "archive",
            "export",
            "IX-WAR-0003",
            archive_path.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(
        exported.status.success(),
        "{} {}",
        String::from_utf8_lossy(&exported.stdout),
        String::from_utf8_lossy(&exported.stderr)
    );
    let archive_bytes = fs::read(&archive_path).unwrap();
    fs::remove_dir_all(f.root.join("docs")).unwrap();
    fs::remove_file(f.root.join("receipt.bin")).unwrap();
    fs::remove_file(f.root.join("openwarrant.toml")).unwrap();
    let query = |path: &Path| {
        war(
            &f.root,
            &["archive", "runtime-basis", path.to_str().unwrap(), "--json"],
        )
    };
    let result = query(&archive_path);
    assert!(
        result.status.success(),
        "{} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let inventory = &value(&result)["result"]["provider_capture_inventory"];
    assert_eq!(inventory["records"].as_array().unwrap().len(), 1);
    let record = &inventory["records"][0];
    assert_eq!(record["dispatch_source_reconnected"], true);
    assert_eq!(record["contract_stage_reconstructed"], true);
    assert_eq!(record["native_verification"], "unknown");
    assert_eq!(record["saved_native_observation_is_trusted"], false);
    assert_eq!(inventory["execution_coverage_established"], false);
    assert_eq!(inventory["native_authentication_established"], false);
    assert_eq!(record["current_attempt_eligibility_established"], false);
    assert_eq!(record["assurance_granted"], false);
    let inert = f.root.join("inert");
    let imported = war(
        &f.root,
        &[
            "archive",
            "import",
            archive_path.to_str().unwrap(),
            inert.to_str().unwrap(),
            "--json",
        ],
    );
    // This fixture has intentionally incomplete whole-project coverage. Querying
    // retained observations must not relax the import's complete-coverage gate.
    assert!(!imported.status.success());
    assert!(
        format!(
            "{} {}",
            String::from_utf8_lossy(&imported.stdout),
            String::from_utf8_lossy(&imported.stderr)
        )
        .contains("required coverage unavailable")
    );
    assert!(!inert.exists());
    let offline = f.root.join("offline");
    fs::create_dir(&offline).unwrap();
    let again = offline.join("archive.json");
    fs::write(&again, &archive_bytes).unwrap();
    fs::remove_file(&archive_path).unwrap();
    assert_eq!(
        value(&query(&again))["result"]["provider_capture_inventory"],
        *inventory
    );
    let original = Archive::decode(&archive_bytes, Limits::default()).unwrap();
    let mut missing = original.clone();
    missing.records.retain(|r| {
        !r.path
            .ends_with(&format!("/dispatches/{}.json", f.dispatch.dispatch_id))
    });
    // Coverage references are transport declarations; remove the vanished path too.
    for coverage in missing.coverage.values_mut() {
        if let openwarrant_compiler::preservation::Coverage::Retained { paths } = coverage {
            paths.retain(|p| !p.ends_with(&format!("/dispatches/{}.json", f.dispatch.dispatch_id)));
        }
    }
    let missing_path = f.root.join("missing.json");
    fs::write(&missing_path, missing.encode(Limits::default()).unwrap()).unwrap();
    let out = query(&missing_path);
    assert!(out.status.success());
    let report = value(&out);
    let inventory = &report["result"]["provider_capture_inventory"];
    assert_eq!(
        inventory["records"][0]["dispatch_source_reconnected"],
        false
    );
    assert!(
        inventory["unresolved"]
            .to_string()
            .contains("dispatch source not retained")
    );
    let changed_capture = |change: fn(&mut Value)| {
        let mut forged = original.clone();
        let capture = forged
            .records
            .iter_mut()
            .find(|r| r.path.contains("/runtime-receipts/"))
            .unwrap();
        let bytes =
            openwarrant_core::attestation::base64_decode(capture.base64.as_ref().unwrap()).unwrap();
        let mut source: Value = serde_json::from_slice(&bytes).unwrap();
        change(&mut source);
        let altered = openwarrant_compiler::to_canonical_bytes(&source).unwrap();
        let hex = openwarrant_compiler::sha256_hex(&altered);
        let old_path = capture.path.clone();
        capture.path = format!(
            "{}/capture-{hex}.json",
            old_path.rsplit_once('/').unwrap().0
        );
        let new_path = capture.path.clone();
        capture.digest = format!("sha256:{hex}");
        capture.base64 = Some(openwarrant_core::attestation::base64_encode(&altered));
        for coverage in forged.coverage.values_mut() {
            if let openwarrant_compiler::preservation::Coverage::Retained { paths } = coverage {
                for p in paths {
                    if *p == old_path {
                        *p = new_path.clone();
                    }
                }
            }
        }
        forged
    };
    let forged = changed_capture(|source| {
        source["observation"]["binding"]["attempt"] = json!("different-attempt")
    });
    let forged_path = f.root.join("forged.json");
    fs::write(&forged_path, forged.encode(Limits::default()).unwrap()).unwrap();
    let out = query(&forged_path);
    assert!(!out.status.success());
    assert!(
        format!(
            "{} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
        .contains("does not bind the exact dispatch")
    );
    let future =
        changed_capture(|source| source["schema"] = json!("oh.war/runtime-capture/future"));
    let future_path = f.root.join("future.json");
    fs::write(&future_path, future.encode(Limits::default()).unwrap()).unwrap();
    let out = query(&future_path);
    assert!(out.status.success());
    let report = value(&out);
    let inventory = &report["result"]["provider_capture_inventory"];
    assert!(inventory["records"].as_array().unwrap().is_empty());
    assert!(
        inventory["unresolved"]
            .to_string()
            .contains("runtime.capture-unsupported-schema")
    );
    let saved_pass = changed_capture(|source| {
        source["native_observation"]["standing"] = json!("matches");
        source["native_observation_is_trusted_on_read"] = json!(true);
        source["assurance_granted"] = json!(true);
    });
    let saved_path = f.root.join("saved-pass.json");
    fs::write(&saved_path, saved_pass.encode(Limits::default()).unwrap()).unwrap();
    let out = query(&saved_path);
    assert!(out.status.success());
    let report = value(&out);
    let record = &report["result"]["provider_capture_inventory"]["records"][0];
    assert_eq!(record["native_verification"], "unknown");
    assert_eq!(record["saved_native_observation_is_trusted"], false);
    assert_eq!(record["assurance_granted"], false);
}

#[test]
fn wrong_recorded_binding_stale_current_basis_and_corrupt_capture_do_not_publish() {
    let f = Fixture::new();
    let out = f.import();
    assert!(out.status.success());
    let response = value(&out);
    let path = f
        .root
        .join(response["result"]["reference"].as_str().unwrap());
    let prior = fs::read(&path).unwrap();
    let dispatch_path = f.root.join(format!(
        "docs/warrants/IX-WAR-0003/dispatches/{}.json",
        f.dispatch.dispatch_id
    ));
    let original = fs::read(&dispatch_path).unwrap();
    let mut altered = f.dispatch.clone();
    altered.attempt_id = "wrong-attempt".into();
    fs::write(&dispatch_path, serde_json::to_vec(&altered).unwrap()).unwrap();
    let out = f.import();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("runtime.capture-recorded-binding"));
    assert_eq!(fs::read(&path).unwrap(), prior);
    fs::write(dispatch_path, original).unwrap();
    let intent = f.root.join("docs/warrants/IX-WAR-0003/atoms/10-intent.md");
    fs::write(intent, b"changed current source").unwrap();
    let out = f.import();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("runtime-basis.dispatch-mismatch"));
    assert_eq!(fs::read_dir(f.storage()).unwrap().count(), 1);
    fs::write(&path, b"altered retained capture").unwrap();
    let out = war(
        &f.root,
        &[
            "runtime",
            "show",
            "IX-WAR-0003",
            response["result"]["digest"].as_str().unwrap(),
            "--json",
        ],
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("runtime.capture-altered"));
}

#[test]
fn missing_compile_event_wrong_provider_and_duplicate_json_members_do_not_write() {
    let f = Fixture::new();
    let journal = f.root.join("docs/warrants/IX-WAR-0003/journal.jsonl");
    let original = fs::read(&journal).unwrap();
    fs::write(&journal, b"\n").unwrap();
    let out = f.import();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("runtime.capture-recorded-dispatch"));
    assert!(!f.storage().exists());
    fs::write(journal, original).unwrap();
    let mut req = f.request();
    req["provider"]["kind"] = "blut".into();
    fs::write(
        f.root.join("capture-request.json"),
        serde_json::to_vec(&req).unwrap(),
    )
    .unwrap();
    assert!(!f.import().status.success());
    assert!(!f.storage().exists());
    let original = serde_json::to_string(&f.request()).unwrap();
    fs::write(
        f.root.join("capture-request.json"),
        original.replacen("{", "{\"dispatch_id\":\"other\",", 1),
    )
    .unwrap();
    let out = f.import();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("runtime.capture-syntax"));
    assert!(!f.storage().exists());
}

#[test]
fn traversal_symlink_and_oversized_inputs_leave_retained_bytes_intact() {
    let f = Fixture::new();
    let out = f.import();
    assert!(out.status.success());
    let result = value(&out);
    let path = f.root.join(result["result"]["reference"].as_str().unwrap());
    let prior = fs::read(&path).unwrap();
    let mut req = f.request();
    req["receipt"] = "../receipt.bin".into();
    fs::write(
        f.root.join("capture-request.json"),
        serde_json::to_vec(&req).unwrap(),
    )
    .unwrap();
    assert!(!f.import().status.success());
    #[cfg(unix)]
    {
        fs::remove_file(f.root.join("receipt.bin")).unwrap();
        std::os::unix::fs::symlink(&path, f.root.join("receipt.bin")).unwrap();
        f.request();
        assert!(!f.import().status.success());
        fs::remove_file(f.root.join("receipt.bin")).unwrap();
    }
    fs::write(f.root.join("receipt.bin"), vec![b'x'; 4 * 1024 * 1024 + 1]).unwrap();
    f.request();
    assert!(!f.import().status.success());
    assert_eq!(fs::read(&path).unwrap(), prior);
    assert_eq!(fs::read_dir(f.storage()).unwrap().count(), 1);
}

#[test]
fn unsupported_capture_version_remains_unknown_without_publication() {
    let f = Fixture::new();
    fs::write(
        f.root.join("capture-request.json"),
        br#"{"schema":"oh.war/runtime-capture-request/future","future_field":true}"#,
    )
    .unwrap();
    let out = f.import();
    assert!(!out.status.success());
    let result = value(&out);
    assert_eq!(result["diagnostics"][0]["severity"], "unknown");
    assert_eq!(
        result["diagnostics"][0]["rule"],
        "runtime.capture-unsupported-schema"
    );
    assert!(!f.storage().exists());
}

struct SyntheticVerifier {
    interface: ProviderInterface,
    facts: VerifiedReceipt,
}
impl ReceiptVerifier for SyntheticVerifier {
    fn interface(&self) -> &ProviderInterface {
        &self.interface
    }
    fn verify(&self, _: &[u8]) -> Result<VerifiedReceipt, ProviderFailure> {
        Ok(self.facts.clone())
    }
}
#[test]
fn sdk_uses_native_verifier_and_refuses_wrong_receipt_before_any_write() {
    let f = Fixture::with_runtime_count(1);
    let repo = openwarrant_cli::repo::Repository::open(
        camino::Utf8PathBuf::from_path_buf(f.root.clone()).unwrap(),
    )
    .unwrap();
    let request: Request = serde_json::from_value(f.request()).unwrap();
    let raw = fs::read(f.root.join("receipt.bin")).unwrap();
    let interface = ProviderInterface {
        kind: ProviderKind::Katana,
        identity: "synthetic-build".into(),
        version: "synthetic-interface/v1".into(),
    };
    let facts = VerifiedReceipt {
        interface: interface.clone(),
        raw_digest: raw_digest(&raw),
        binding: RuntimeBinding::from_dispatch(&f.dispatch),
        receipt: NativeReceipt::Katana(KatanaReceipt {
            session_id: "synthetic-session".into(),
            dispatch_digest: f.dispatch.dispatch_digest.clone(),
            prompt_ir_digest: "synthetic-prompt".into(),
            provider_model_identity: "synthetic-model".into(),
            runtime_event_log_head: "synthetic-event-log".into(),
            confinement: "synthetic-sandbox".into(),
            usage: "unmetered".into(),
            terminal_runtime_status: "completed".into(),
            receipt_digest: "synthetic-provider-seal".into(),
            ..Default::default()
        }),
        outcome: RuntimeOutcome::Completed,
        execution: Observation::Established,
        confinement: Observation::Established,
        metered_cost: Observation::Unknown,
        spend_cap: Observation::Unknown,
        registry_digest: None,
    };
    let mut v = SyntheticVerifier { interface, facts };
    v.facts.binding.attempt_id = "wrong".into();
    fn verify(verifier: &SyntheticVerifier) -> Verification<'_> {
        Verification {
            verifier,
            registry_digest: None,
            authorized_capabilities: Some(&[]),
            confinement_required: true,
            hard_spend_cap_required: false,
        }
    }
    assert_eq!(
        capture::import(&repo, "IX-WAR-0003", &request, Some(verify(&v)))
            .unwrap_err()
            .code,
        "runtime.binding-mismatch"
    );
    assert!(!f.storage().exists());
    v.facts.binding.attempt_id = f.dispatch.attempt_id.clone();
    let response = capture::import(&repo, "IX-WAR-0003", &request, Some(verify(&v))).unwrap();
    assert_eq!(response["native_observation"]["standing"], "matches");
    assert_eq!(
        response["native_observation"]["cost_observation"],
        "unknown"
    );
    assert_eq!(
        response["native_observation"]["provider_receipt_digest"],
        "synthetic-provider-seal"
    );
    assert_eq!(response["assurance_granted"], false);
    let selected = [capture::Selection {
        stage_id: f.dispatch.stage_id.clone(),
        capture_digest: response["digest"].as_str().unwrap().into(),
    }];
    assert_eq!(
        capture::assess_selected(&repo, "IX-WAR-0003", &selected, |_, _| None)
            .unwrap()
            .standing,
        ReceiptStanding::Unknown
    );
    assert_eq!(
        capture::assess_selected(&repo, "IX-WAR-0003", &selected, |_, _| Some(verify(&v)))
            .unwrap()
            .standing,
        ReceiptStanding::Matches
    );
    // Stored MATCHES is never reused when current native verification disagrees.
    v.facts.binding.attempt_id = "wrong-new-native-observation".into();
    assert_eq!(
        capture::assess_selected(&repo, "IX-WAR-0003", &selected, |_, _| Some(verify(&v)))
            .unwrap()
            .standing,
        ReceiptStanding::Refused
    );
    v.facts.binding.attempt_id = f.dispatch.attempt_id.clone();
    let intent = f.root.join("docs/warrants/IX-WAR-0003/atoms/10-intent.md");
    let original = fs::read(&intent).unwrap();
    let mut changed = original.clone();
    changed.extend_from_slice(b"\nsource changed during native assessment\n");
    let changed_result = capture::assess_selected(&repo, "IX-WAR-0003", &selected, |_, _| {
        fs::write(&intent, &changed).unwrap();
        Some(verify(&v))
    })
    .unwrap_err();
    assert_eq!(changed_result.code, "runtime.selection-changed");
    assert!(changed_result.unknown);
    fs::write(&intent, original).unwrap();
    let partial = Fixture::new();
    v.facts.binding = RuntimeBinding::from_dispatch(&partial.dispatch);
    if let NativeReceipt::Katana(receipt) = &mut v.facts.receipt {
        receipt.dispatch_digest = partial.dispatch.dispatch_digest.clone();
    }
    let response = value(&partial.import());
    let selected = [capture::Selection {
        stage_id: partial.dispatch.stage_id.clone(),
        capture_digest: response["result"]["digest"].as_str().unwrap().into(),
    }];
    let repo = openwarrant_cli::repo::Repository::open(
        camino::Utf8PathBuf::from_path_buf(partial.root.clone()).unwrap(),
    )
    .unwrap();
    let result =
        capture::assess_selected(&repo, "IX-WAR-0003", &selected, |_, _| Some(verify(&v))).unwrap();
    assert_eq!(result.standing, ReceiptStanding::Unknown);
    assert_eq!(result.stages[0].receipt.standing, ReceiptStanding::Matches);
    assert_eq!(
        result.stages[1].receipt.code,
        "runtime-basis.receipt-missing"
    );
}

#[test]
fn current_selection_is_read_only_and_missing_native_support_stays_unknown() {
    let f = Fixture::new();
    let response = value(&f.import());
    let digest = response["result"]["digest"].as_str().unwrap();
    let path = f
        .root
        .join(response["result"]["reference"].as_str().unwrap());
    let prior = fs::read(&path).unwrap();
    let selected = json!({"schema":"oh.war/runtime-selection-request/v1-draft.1", "selections":[{"stage_id":"STAGE-001", "capture_digest":digest}]});
    fs::write(
        f.root.join("selected.json"),
        serde_json::to_vec(&selected).unwrap(),
    )
    .unwrap();
    fs::remove_file(f.root.join("receipt.bin")).unwrap();
    let out = war(
        &f.root,
        &[
            "runtime",
            "assess",
            "IX-WAR-0003",
            "--selection",
            "selected.json",
            "--json",
        ],
    );
    assert!(!out.status.success());
    let result = value(&out);
    assert_eq!(result["diagnostics"][0]["severity"], "unknown");
    assert_eq!(
        result["result"]["stages"][0]["receipt"]["code"],
        "runtime.verifier-unavailable"
    );
    assert_eq!(result["result"]["saved_native_observations_used"], false);
    assert_eq!(result["result"]["assurance_granted"], false);
    assert_eq!(fs::read(&path).unwrap(), prior);
    let repo = openwarrant_cli::repo::Repository::open(
        camino::Utf8PathBuf::from_path_buf(f.root.clone()).unwrap(),
    )
    .unwrap();
    let empty = capture::assess_selected(&repo, "IX-WAR-0003", &[], |_, _| None).unwrap();
    assert_eq!(empty.standing, ReceiptStanding::Unknown);
    assert_eq!(
        empty.stages[0].receipt.code,
        "runtime-basis.receipt-missing"
    );
    let mut selections: Vec<capture::Selection> =
        serde_json::from_value(selected["selections"].clone()).unwrap();
    selections.push(selections[0].clone());
    assert_eq!(
        capture::assess_selected(&repo, "IX-WAR-0003", &selections, |_, _| None)
            .unwrap_err()
            .code,
        "runtime.selection-ambiguous"
    );
    selections.truncate(1);
    selections[0].stage_id = "wrong".into();
    assert_eq!(
        capture::assess_selected(&repo, "IX-WAR-0003", &selections, |_, _| None)
            .unwrap_err()
            .code,
        "runtime.selection-stage"
    );
    assert_eq!(fs::read(&path).unwrap(), prior);
}

#[test]
fn later_recorded_attempt_prevents_fallback_to_historical_capture() {
    let f = Fixture::new();
    let response = value(&f.import());
    let selected = [capture::Selection {
        stage_id: f.dispatch.stage_id.clone(),
        capture_digest: response["result"]["digest"].as_str().unwrap().into(),
    }];
    let pending = f.root.join("new-attempt.json");
    let out = war(
        &f.root,
        &[
            "dispatch",
            "IX-WAR-0003",
            "STAGE-001",
            "--prototype",
            "--emit",
            pending.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let next: StageDispatch = serde_json::from_slice(&fs::read(&pending).unwrap()).unwrap();
    assert_ne!(next.attempt_id, f.dispatch.attempt_id);
    fs::rename(
        pending,
        f.root.join(format!(
            "docs/warrants/IX-WAR-0003/dispatches/{}.json",
            next.dispatch_id
        )),
    )
    .unwrap();
    let repo = openwarrant_cli::repo::Repository::open(
        camino::Utf8PathBuf::from_path_buf(f.root.clone()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        capture::assess_selected(&repo, "IX-WAR-0003", &selected, |_, _| None)
            .unwrap_err()
            .code,
        "runtime.selection-superseded-attempt"
    );
    assert!(capture::show(&repo, "IX-WAR-0003", &selected[0].capture_digest).is_ok());
    assert_eq!(fs::read_dir(f.storage()).unwrap().count(), 1);
}

#[test]
fn unknown_versions_and_stale_sources_do_not_invoke_provider_policy() {
    let f = Fixture::new();
    let response = value(&f.import());
    let repo = openwarrant_cli::repo::Repository::open(
        camino::Utf8PathBuf::from_path_buf(f.root.clone()).unwrap(),
    )
    .unwrap();
    let original_digest = response["result"]["digest"].as_str().unwrap();
    let mut record =
        capture::show(&repo, "IX-WAR-0003", original_digest).unwrap()["record"].clone();
    record["declared_capture"]["schema"] = "future-schema".into();
    record["declared_capture"]["future_field"] = true.into();
    let bytes = openwarrant_compiler::to_canonical_bytes(&record).unwrap();
    let hash = openwarrant_compiler::sha256_hex(&bytes);
    fs::write(f.storage().join(format!("capture-{hash}.json")), bytes).unwrap();
    let selected = [capture::Selection {
        stage_id: f.dispatch.stage_id.clone(),
        capture_digest: format!("sha256:{hash}"),
    }];
    let calls = std::cell::Cell::new(0);
    let resolve = |_: &StageDispatch, _: &ProviderInterface| {
        calls.set(calls.get() + 1);
        None
    };
    let future = capture::assess_selected(&repo, "IX-WAR-0003", &selected, resolve).unwrap_err();
    assert_eq!(future.code, "runtime.capture-unsupported-schema");
    assert!(future.unknown);
    assert_eq!(calls.get(), 0);
    let intent = f.root.join("docs/warrants/IX-WAR-0003/atoms/10-intent.md");
    let mut changed = fs::read(&intent).unwrap();
    changed.extend_from_slice(b"\nchanged contract\n");
    fs::write(intent, changed).unwrap();
    let selected = [capture::Selection {
        stage_id: f.dispatch.stage_id.clone(),
        capture_digest: original_digest.into(),
    }];
    let stale = capture::assess_selected(&repo, "IX-WAR-0003", &selected, resolve).unwrap();
    assert_eq!(stale.standing, ReceiptStanding::Refused);
    assert_eq!(
        stale.stages[0].receipt.code,
        "runtime-basis.dispatch-mismatch"
    );
    assert_eq!(calls.get(), 0);
}
