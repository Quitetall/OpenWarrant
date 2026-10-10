// SPDX-License-Identifier: Apache-2.0
//! Local CLI capture controls plus an opt-in real native BLUT CPU roundtrip.
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

#[cfg(unix)]
#[test]
fn katana_process_preserves_unavailable_and_refuses_contradictory_responses() {
    use capture::katana_process::{KatanaProcessConfig, KatanaProcessVerifier};
    use std::{os::unix::fs::PermissionsExt, time::Duration};
    let f = Fixture::with_runtime_count(1);
    let repo = openwarrant_cli::repo::Repository::open(f.root.clone().try_into().unwrap()).unwrap();
    let executable = f.root.join("synthetic-provider.sh");
    for (body, exit, unavailable) in [
        (
            json!({"schema":"katana/openwarrant-verification/v1","status":"unavailable","assurance":"not-established","reason":"native log missing"}),
            1,
            true,
        ),
        (
            json!({"schema":"katana/openwarrant-verification/v1","status":"unavailable","assurance":"not-established","reason":"native log missing"}),
            0,
            false,
        ),
        (
            json!({"schema":"katana/openwarrant-verification/v1","status":"validated","assurance":"granted","native":{}}),
            0,
            false,
        ),
    ] {
        // Synthetic protocol control, not a native verifier or authenticated provider.
        fs::write(
            &executable,
            format!("#!/bin/sh\nprintf '%s\\n' '{}'\nexit {exit}\n", body),
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let config = KatanaProcessConfig {
            provider: ProviderInterface {
                kind: ProviderKind::Katana,
                identity: "synthetic-protocol".into(),
                version: "katana/openwarrant-verification/v1".into(),
            },
            executable: executable.clone(),
            event_log: f.root.join("native.jsonl"),
            trusted_log_head: format!("b3:{}", "1".repeat(64)),
            scratch_root: f.root.clone(),
            timeout: Duration::from_secs(1),
            max_response_bytes: 4096,
        };
        let verifier = KatanaProcessVerifier::for_recorded_dispatch(
            &repo,
            "IX-WAR-0003",
            &f.dispatch.dispatch_id,
            config,
        )
        .unwrap();
        let error = verifier.verify(b"synthetic receipt").unwrap_err();
        if unavailable {
            assert!(matches!(error, ProviderFailure::Unavailable(_)));
        } else {
            assert!(matches!(error, ProviderFailure::Rejected(_)));
        }
        assert!(!fs::read_dir(&f.root).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("ow-katana-verify-")
        }));
    }
}

#[cfg(unix)]
#[test]
fn katana_process_checks_native_mapping_without_inventing_cost_or_authority() {
    use capture::katana_process::{KatanaProcessConfig, KatanaProcessVerifier};
    use std::{os::unix::fs::PermissionsExt, time::Duration};
    let f = Fixture::with_runtime_count(1);
    let repo = openwarrant_cli::repo::Repository::open(f.root.clone().try_into().unwrap()).unwrap();
    let executable = f.root.join("synthetic-mapping.sh");
    let head = format!("b3:{}", "1".repeat(64));
    let config = || KatanaProcessConfig {
        provider: ProviderInterface {
            kind: ProviderKind::Katana,
            identity: "synthetic-mapping-only".into(),
            version: "katana/openwarrant-verification/v1".into(),
        },
        executable: executable.clone(),
        event_log: f.root.join("native.jsonl"),
        trusted_log_head: head.clone(),
        scratch_root: f.root.clone(),
        timeout: Duration::from_secs(1),
        max_response_bytes: 4096,
    };
    let mut invalid = config();
    invalid.trusted_log_head = "not-a-native-head".into();
    assert!(matches!(
        KatanaProcessVerifier::for_recorded_dispatch(
            &repo,
            "IX-WAR-0003",
            &f.dispatch.dispatch_id,
            invalid
        ),
        Err(ProviderFailure::Rejected(_))
    ));
    let mut invalid = config();
    invalid.provider.version = "future-interface".into();
    assert!(matches!(
        KatanaProcessVerifier::for_recorded_dispatch(
            &repo,
            "IX-WAR-0003",
            &f.dispatch.dispatch_id,
            invalid
        ),
        Err(ProviderFailure::Rejected(_))
    ));
    let verifier = KatanaProcessVerifier::for_recorded_dispatch(
        &repo,
        "IX-WAR-0003",
        &f.dispatch.dispatch_id,
        config(),
    )
    .unwrap();
    let original = json!({
        "schema":"katana/openwarrant-receipt/v1", "binding":{"warrant_ref":f.dispatch.warrant_ref,"contract_digest":f.dispatch.contract_digest,"dispatch_digest":f.dispatch.dispatch_digest,"stage_id":f.dispatch.stage_id,"attempt_id":f.dispatch.attempt_id},
        "session_id":"synthetic-session", "outcome":"completed", "prompt_ir_digest":head, "provider_model_identity":["synthetic-model"], "event_log_head":head, "receipt_digest":head,
        "realized_capabilities":[], "capability_observation":"dispatch-permission-upper-bound", "confinement":"none", "input_tokens":1,"output_tokens":1,"usage_estimated":true,"cost_usd":null,"artifact_event_refs":[],"taint_event_refs":[],"assurance_granted":false
    });
    let respond = |native: &Value| {
        // Protocol stand-in only. This cannot establish native log validity.
        let response = json!({"schema":"katana/openwarrant-verification/v1","status":"validated","assurance":"not-established","native":native});
        fs::write(
            &executable,
            format!("#!/bin/sh\nprintf '%s\\n' '{}'\n", response),
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    };
    for (status, outcome) in [
        ("completed", RuntimeOutcome::Completed),
        ("failed", RuntimeOutcome::Failed),
        ("halted", RuntimeOutcome::Halted),
        ("cancelled", RuntimeOutcome::Cancelled),
    ] {
        let mut native = original.clone();
        native["outcome"] = json!(status);
        respond(&native);
        let result = verifier
            .verify(&serde_json::to_vec(&native).unwrap())
            .unwrap();
        assert_eq!(result.outcome, outcome);
        assert_eq!(result.confinement, Observation::Unknown);
        assert_eq!(result.metered_cost, Observation::Unknown);
        assert_eq!(result.spend_cap, Observation::Unknown);
    }
    for (field, bad) in [
        ("cost_usd", json!(0)),
        ("assurance_granted", json!(true)),
        ("event_log_head", json!(format!("b3:{}", "2".repeat(64)))),
        ("prompt_ir_digest", Value::Null),
    ] {
        let mut native = original.clone();
        native[field] = bad;
        respond(&native);
        assert!(matches!(
            verifier.verify(&serde_json::to_vec(&native).unwrap()),
            Err(ProviderFailure::Rejected(_))
        ));
    }
    respond(&original);
    assert!(matches!(
        verifier.verify(b"different original receipt"),
        Err(ProviderFailure::Rejected(_))
    ));
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
        Self::with_executor(count, "katana")
    }
    fn with_executor(count: usize, executor: &str) -> Self {
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
            &format!("executor_kind: \"{executor}\"\n    executor_ref: \"synthetic\""),
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
fn capture_uses_an_emitted_dispatch_without_manual_record_surgery() {
    let mut f = Fixture::with_runtime_count(1);
    let emitted = f.root.join("fresh-emitted.json");
    let out = war(
        &f.root,
        &[
            "admin",
            "dispatch",
            "IX-WAR-0003",
            "STAGE-001",
            "--prototype",
            "--emit",
            emitted.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    f.dispatch = serde_json::from_slice(&fs::read(&emitted).unwrap()).unwrap();
    f.request();
    let captured = f.import();
    assert!(
        captured.status.success(),
        "{}",
        String::from_utf8_lossy(&captured.stdout)
    );
    assert_eq!(value(&captured)["result"]["assurance_granted"], false);
    assert_eq!(
        value(&captured)["result"]["native_observation"]["standing"],
        "unknown"
    );
    let retained = f.root.join(format!(
        "docs/warrants/IX-WAR-0003/dispatches/{}.json",
        f.dispatch.dispatch_id
    ));
    assert_eq!(fs::read(&retained).unwrap(), fs::read(&emitted).unwrap());
}

#[cfg(unix)]
#[test]
fn dispatch_retention_refuses_linked_store_without_a_new_compile_event() {
    use std::os::unix::fs::symlink;
    let f = Fixture::with_runtime_count(1);
    let directory = f.root.join("docs/warrants/IX-WAR-0003");
    let journal = fs::read(directory.join("journal.jsonl")).unwrap();
    let packet =
        fs::read(directory.join(format!("dispatches/{}.json", f.dispatch.dispatch_id))).unwrap();
    fs::rename(
        directory.join("dispatches"),
        directory.join("previous-dispatches"),
    )
    .unwrap();
    let outside = f.root.join("foreign-store");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("owner-file"), b"unchanged owner bytes").unwrap();
    symlink(&outside, directory.join("dispatches")).unwrap();
    let emitted = f.root.join("refused-emitted.json");
    let out = war(
        &f.root,
        &[
            "admin",
            "dispatch",
            "IX-WAR-0003",
            "STAGE-001",
            "--prototype",
            "--emit",
            emitted.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!out.status.success());
    assert!(
        value(&out)["diagnostics"]
            .to_string()
            .contains("dispatch.retention-unavailable")
    );
    assert_eq!(fs::read(directory.join("journal.jsonl")).unwrap(), journal);
    assert_eq!(
        fs::read(directory.join(format!(
            "previous-dispatches/{}.json",
            f.dispatch.dispatch_id
        )))
        .unwrap(),
        packet
    );
    assert!(!emitted.exists());
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);
    assert_eq!(
        fs::read(outside.join("owner-file")).unwrap(),
        b"unchanged owner bytes"
    );
}

#[cfg(unix)]
#[test]
fn blut_process_unavailable_evidence_stays_unknown_for_a_recorded_dispatch() {
    use capture::blut_process::{BlutProcessConfig, BlutProcessVerifier};
    use std::{os::unix::fs::PermissionsExt, time::Duration};
    let f = Fixture::with_executor(usize::MAX, "blut");
    let executable = f.root.join("provider.sh");
    fs::write(&executable, "#!/bin/sh\nprintf '%s\\n' '{\"schema\":\"blut/openwarrant-verification/v1\",\"status\":\"unavailable\",\"assurance\":\"not-established\",\"reason\":\"native job absent\"}'\nexit 1\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let repo = openwarrant_cli::repo::Repository::open(f.root.clone().try_into().unwrap()).unwrap();
    let config = BlutProcessConfig {
        provider: ProviderInterface {
            kind: ProviderKind::Blut,
            identity: "synthetic-protocol-process".into(),
            version: "blut/openwarrant-verification/v1".into(),
        },
        executable,
        public_key: f.root.join("public-key.bin"),
        plan: f.root.join("plan.json"),
        job: f.root.join("native-job"),
        producer_executable: f.root.join("producer"),
        scratch_root: f.root.clone(),
        timeout: Duration::from_secs(1),
        max_response_bytes: 4 * 1024 * 1024,
    };
    let verifier = BlutProcessVerifier::for_recorded_dispatch(
        &repo,
        "IX-WAR-0003",
        &f.dispatch.dispatch_id,
        config,
    )
    .unwrap();
    assert_eq!(
        verifier.verify(b"synthetic native bytes").unwrap_err(),
        ProviderFailure::Unavailable("native job absent".into())
    );
    assert!(!f.root.join("native-job").exists());
    assert!(!fs::read_dir(&f.root).unwrap().any(|e| {
        e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("ow-blut-verify-")
    }));
}

#[cfg(unix)]
#[test]
fn blut_process_maps_native_facts_without_awarding_assurance_or_relabeling_hashes() {
    // This neighbor is a synthetic process-protocol fixture, NOT a native seal verifier.
    use capture::blut_process::{BlutProcessConfig, BlutProcessVerifier};
    use std::{os::unix::fs::PermissionsExt, time::Duration};
    let f = Fixture::with_executor(1, "blut");
    let executable = f.root.join("provider.sh");
    fs::write(
        &executable,
        "#!/bin/sh\nexec /bin/cat \"${6%/*}/native-response.json\"\n",
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let hash = format!("blake3:{}", "a".repeat(64));
    let lineage = json!({"path":"status.jsonl","bytes":7,"digest":hash,"mode":33188});
    fs::write(f.root.join("native-response.json"), serde_json::to_vec(&json!({
        "schema":"blut/openwarrant-verification/v1","status":"validated","assurance":"not-established",
        "native":{"binding":{"warrant_ref":f.dispatch.warrant_ref,"contract_digest":f.dispatch.contract_digest,
            "dispatch_digest":f.dispatch.dispatch_digest,"stage_id":f.dispatch.stage_id,"attempt_id":f.dispatch.attempt_id},
            "outcome":"completed","run_id":"00000000-0000-4000-8000-000000000001","receipt_digest":hash,
            "registry_digest":hash,"native_files":[lineage.clone(),{"path":"stage/output.txt","bytes":26,"digest":hash,"mode":33188}],
            "lineage_reference":lineage}
    })).unwrap()).unwrap();
    let repo = openwarrant_cli::repo::Repository::open(f.root.clone().try_into().unwrap()).unwrap();
    let config = BlutProcessConfig {
        provider: ProviderInterface {
            kind: ProviderKind::Blut,
            identity: "synthetic-protocol-process".into(),
            version: "blut/openwarrant-verification/v1".into(),
        },
        executable,
        public_key: f.root.join("public-key.bin"),
        plan: f.root.join("plan.json"),
        job: f.root.join("native-job"),
        producer_executable: f.root.join("producer"),
        scratch_root: f.root.clone(),
        timeout: Duration::from_secs(5),
        max_response_bytes: 4 * 1024 * 1024,
    };
    let verifier = BlutProcessVerifier::for_recorded_dispatch(
        &repo,
        "IX-WAR-0003",
        &f.dispatch.dispatch_id,
        config,
    )
    .unwrap();
    let mapped = verifier.verify(b"synthetic original receipt").unwrap();
    assert_eq!(
        mapped.raw_digest,
        "sha256:3e92e2c9df3a15bd2f3a7dd1ec505b4b5a199fb4e3a81b6bedeb0bff1703ca4a"
    );
    assert_eq!(mapped.binding, RuntimeBinding::from_dispatch(&f.dispatch));
    assert_eq!(mapped.outcome, RuntimeOutcome::Completed);
    assert_eq!(mapped.execution, Observation::Established);
    assert_eq!(mapped.confinement, Observation::Unknown);
    assert_eq!(mapped.metered_cost, Observation::Unknown);
    assert_eq!(mapped.spend_cap, Observation::Unknown);
    assert_eq!(mapped.registry_digest.as_deref(), Some(hash.as_str()));
    let NativeReceipt::Blut(native) = mapped.receipt else {
        panic!("BLUT native receipt required")
    };
    assert_eq!(native.receipt_digest, hash);
    assert!(native.lineage_ref.contains("status.jsonl"));
    assert!(
        native
            .artifact_refs
            .iter()
            .any(|p| p.contains("stage/output.txt"))
    );
    let original: Value =
        serde_json::from_slice(&fs::read(f.root.join("native-response.json")).unwrap()).unwrap();
    for (pointer, replacement) in [
        ("/native/binding/attempt_id", json!("another-attempt")),
        ("/native/registry_digest", json!("sha256:wrong-domain")),
        ("/native/outcome", json!("invented-success")),
        ("/native/lineage_reference", Value::Null),
        ("/native/lineage_reference/bytes", json!(8)),
        ("/native/native_files/1/path", json!("../escape")),
        ("/native/native_files/1/path", json!("status.jsonl")),
        ("/assurance", json!("verified")),
        ("/status", json!("unavailable")),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        fs::write(
            f.root.join("native-response.json"),
            serde_json::to_vec(&changed).unwrap(),
        )
        .unwrap();
        assert!(
            matches!(
                verifier.verify(b"synthetic original receipt"),
                Err(ProviderFailure::Rejected(_))
            ),
            "{pointer}"
        );
    }
}

#[cfg(unix)]
#[test]
fn blut_process_refuses_bad_responses_and_bounds_nonresponsive_providers() {
    use capture::blut_process::{BlutProcessConfig, BlutProcessVerifier};
    use std::{os::unix::fs::PermissionsExt, time::Duration};
    let f = Fixture::with_executor(1, "blut");
    let executable = f.root.join("provider.sh");
    let repo = openwarrant_cli::repo::Repository::open(f.root.clone().try_into().unwrap()).unwrap();
    for (script, limit, expected) in [
        ("printf '%s\\n' '{broken}'", 1024, "syntax"),
        (
            "printf '%s\\n' '{\"schema\":\"blut/openwarrant-verification/v1\",\"schema\":\"blut/openwarrant-verification/v1\"}'",
            1024,
            "syntax",
        ),
        ("/usr/bin/head -c 4096 /dev/zero", 64, "budget"),
        ("/usr/bin/head -c 4096 /dev/zero >&2", 64, "budget"),
        ("/bin/sleep 5", 1024, "deadline"),
    ] {
        fs::write(&executable, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let config = BlutProcessConfig {
            provider: ProviderInterface {
                kind: ProviderKind::Blut,
                identity: "synthetic-protocol-process".into(),
                version: "blut/openwarrant-verification/v1".into(),
            },
            executable: executable.clone(),
            public_key: f.root.join("public-key.bin"),
            plan: f.root.join("plan.json"),
            job: f.root.join("native-job"),
            producer_executable: f.root.join("producer"),
            scratch_root: f.root.clone(),
            timeout: Duration::from_secs(1),
            max_response_bytes: limit,
        };
        let verifier = BlutProcessVerifier::for_recorded_dispatch(
            &repo,
            "IX-WAR-0003",
            &f.dispatch.dispatch_id,
            config,
        )
        .unwrap();
        let error = verifier.verify(b"synthetic original receipt").unwrap_err();
        match expected {
            "budget" => assert_eq!(
                error,
                ProviderFailure::Rejected("native verifier output byte budget".into())
            ),
            "deadline" => assert_eq!(
                error,
                ProviderFailure::Unavailable(
                    "native verifier deadline exceeded; no result established".into()
                )
            ),
            _ => assert!(matches!(error, ProviderFailure::Rejected(_)), "{error:?}"),
        }
        assert!(!f.root.join("native-job").exists());
        assert!(!fs::read_dir(&f.root).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("ow-blut-verify-")
        }));
    }
}

#[cfg(unix)]
#[test]
fn blut_verifier_does_not_inherit_the_callers_environment() {
    use capture::blut_process::{BlutProcessConfig, BlutProcessVerifier};
    use std::{os::unix::fs::PermissionsExt, time::Duration};
    // Cargo config supplies this benign marker. No private secret is planted.
    assert_eq!(std::env::var("OPENWARRANT_NO_PROJECTS").unwrap(), "1");
    let f = Fixture::with_executor(1, "blut");
    let executable = f.root.join("provider.sh");
    fs::write(&executable, "#!/bin/sh\nif [ \"${OPENWARRANT_NO_PROJECTS+x}\" = x ]; then\n printf '%s\\n' '{\"schema\":\"blut/openwarrant-verification/v1\",\"status\":\"rejected\",\"assurance\":\"not-established\",\"reason\":\"inherited caller context\"}'\nelse\n printf '%s\\n' '{\"schema\":\"blut/openwarrant-verification/v1\",\"status\":\"unavailable\",\"assurance\":\"not-established\",\"reason\":\"isolated context\"}'\nfi\nexit 1\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let repo = openwarrant_cli::repo::Repository::open(f.root.clone().try_into().unwrap()).unwrap();
    let config = BlutProcessConfig {
        provider: ProviderInterface {
            kind: ProviderKind::Blut,
            identity: "synthetic-protocol-process".into(),
            version: "blut/openwarrant-verification/v1".into(),
        },
        executable,
        public_key: f.root.join("public-key.bin"),
        plan: f.root.join("plan.json"),
        job: f.root.join("native-job"),
        producer_executable: f.root.join("producer"),
        scratch_root: f.root.clone(),
        timeout: Duration::from_secs(5),
        max_response_bytes: 4 * 1024 * 1024,
    };
    let verifier = BlutProcessVerifier::for_recorded_dispatch(
        &repo,
        "IX-WAR-0003",
        &f.dispatch.dispatch_id,
        config,
    )
    .unwrap();
    assert_eq!(
        verifier.verify(b"synthetic original receipt").unwrap_err(),
        ProviderFailure::Unavailable("isolated context".into())
    );
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
fn archive_recovers_exact_capture_contract_when_historical_ir_was_never_committed() {
    let f = Fixture::with_runtime_count(1);
    let directory = "docs/warrants/IX-WAR-0003";
    let ir = f.root.join(directory).join("generated/WAR.json");
    if ir.exists() {
        fs::remove_file(ir).unwrap();
    }
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .args([
                "--no-pager",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(&f.root)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Synthetic history fixture"]);
    git(&["config", "user.email", "fixture@example.invalid"]);
    git(&["add", "."]);
    git(&[
        "commit",
        "-qm",
        "Retain sources without generated contract IR",
    ]);
    let before_capture = git(&["rev-parse", "HEAD"]);
    let imported = f.import();
    assert!(
        imported.status.success(),
        "{}",
        String::from_utf8_lossy(&imported.stdout)
    );
    let captured = value(&imported);
    let capture_path = f
        .root
        .join(captured["result"]["reference"].as_str().unwrap());
    let original_capture = fs::read(&capture_path).unwrap();
    // Two interpretations of identical sources are not a historical SAS acceptance.
    let repo = openwarrant_cli::repo::Repository::open(f.root.clone().try_into().unwrap()).unwrap();
    let one = repo
        .load_warrant(&repo.warrant_dir("IX-WAR-0003").unwrap())
        .unwrap();
    let mut alternate_basis = one.basis.unwrap();
    alternate_basis.sas = Some(openwarrant_compiler::SasPin {
        version: "synthetic-context-variant".into(),
        sha256: "a".repeat(64),
    });
    let alternate_ir =
        openwarrant_compiler::lower(&alternate_basis, &one.validated.unwrap()).unwrap();
    git(&["add", "."]);
    git(&["commit", "-qm", "Retain an unverified source-bound capture"]);
    let archive = f.root.join("history.archive.json");
    let out = war(
        &f.root,
        &[
            "archive",
            "export",
            "IX-WAR-0003",
            archive.to_str().unwrap(),
            "--history",
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    fs::remove_dir_all(f.root.join("docs")).unwrap();
    fs::remove_file(f.root.join("openwarrant.toml")).unwrap();
    let out = war(
        &f.root,
        &[
            "archive",
            "runtime-basis",
            archive.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let result = value(&out);
    let historical = format!("__ow_archive__/history/{before_capture}/{directory}/manifest.toml");
    let graph = result["result"]["stage_inventory"]["declarations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["manifest_source"] == historical)
        .unwrap();
    assert_eq!(graph["contract_binding"]["reconstructed"], true);
    assert_eq!(
        graph["contract_binding"]["source_kind"],
        "retained-runtime-contract-snapshot"
    );
    assert_eq!(
        graph["contract_binding"]["digest"]
            .as_str()
            .unwrap()
            .trim_start_matches("sha256:"),
        f.dispatch
            .contract_digest
            .strip_prefix("sha256:")
            .unwrap_or(&f.dispatch.contract_digest)
    );
    assert_eq!(result["result"]["qualified"], false);
    assert_eq!(result["result"]["authority_activated"], false);
    let original: serde_json::Value = serde_json::from_slice(&original_capture).unwrap();
    assert_eq!(original["schema"], "oh.war/runtime-capture/v1-draft.1");
    assert!(
        !graph["contract_binding"]["ir_source"]
            .as_str()
            .unwrap()
            .starts_with(&format!("__ow_archive__/history/{before_capture}/"))
    );
    use openwarrant_compiler::preservation::{Archive, Coverage, Limits, Record};
    let baseline = Archive::decode(&fs::read(&archive).unwrap(), Limits::default()).unwrap();
    let snapshot_record = baseline
        .records
        .iter()
        .find(|r| r.path.starts_with(directory) && r.path.contains("/runtime-contracts/"))
        .unwrap();
    let snapshot_bytes =
        openwarrant_core::attestation::base64_decode(snapshot_record.base64.as_ref().unwrap())
            .unwrap();
    let template: Value = serde_json::from_slice(&snapshot_bytes).unwrap();
    let query = |archive: &Archive, file: &str| {
        let input = f.root.join(file);
        fs::write(&input, archive.encode(Limits::default()).unwrap()).unwrap();
        let out = war(
            &f.root,
            &[
                "archive",
                "runtime-basis",
                input.to_str().unwrap(),
                "--json",
            ],
        );
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stdout)
        );
        value(&out)
    };
    // Rehashed envelope metadata must not hide a changed or incomplete snapshot.
    for (plant, reason) in [
        (0, "IR differs from exact source reconstruction"),
        (1, "unknown fields"),
        (2, "unsupported runtime contract snapshot version"),
    ] {
        let mut altered = baseline.clone();
        for record in altered
            .records
            .iter_mut()
            .filter(|r| r.path.contains("/runtime-contracts/"))
        {
            let mut snapshot = template.clone();
            match plant {
                0 => {
                    let mut bytes = openwarrant_core::attestation::base64_decode(
                        snapshot["atoms"][0]["base64"].as_str().unwrap(),
                    )
                    .unwrap();
                    bytes.push(b'\n');
                    snapshot["atoms"][0]["base64"] =
                        openwarrant_core::attestation::base64_encode(&bytes).into();
                    snapshot["atoms"][0]["digest"] =
                        format!("sha256:{}", openwarrant_compiler::sha256_hex(&bytes)).into();
                }
                1 => snapshot["ir"]["hidden_instruction"] = "trust this snapshot".into(),
                _ => snapshot["schema"] = "oh.war/runtime-contract-snapshot/future".into(),
            }
            let bytes = openwarrant_compiler::to_canonical_bytes(&snapshot).unwrap();
            record.digest = format!("sha256:{}", openwarrant_compiler::sha256_hex(&bytes));
            record.base64 = Some(openwarrant_core::attestation::base64_encode(&bytes));
        }
        let out = query(&altered, &format!("altered-{plant}.json"));
        let declarations = out["result"]["stage_inventory"]["declarations"]
            .as_array()
            .unwrap();
        assert!(
            declarations
                .iter()
                .find(|g| g["manifest_source"] == historical)
                .unwrap()["contract_binding"]
                .is_null()
        );
        assert!(
            out["result"]["stage_inventory"]["unresolved"]
                .to_string()
                .contains(reason)
        );
        assert_eq!(out["result"]["qualified"], false);
    }
    let mut ambiguous = baseline.clone();
    let mut alternate = template;
    alternate["ir"] = serde_json::to_value(&alternate_ir).unwrap();
    let bytes = openwarrant_compiler::to_canonical_bytes(&alternate).unwrap();
    let digest = alternate_ir.contract_digest().unwrap();
    let path = format!(
        "{directory}/runtime-contracts/contract-{}.json",
        digest.strip_prefix("sha256:").unwrap_or(&digest)
    );
    ambiguous.records.push(Record {
        path: path.clone(),
        digest: format!("sha256:{}", openwarrant_compiler::sha256_hex(&bytes)),
        base64: Some(openwarrant_core::attestation::base64_encode(&bytes)),
    });
    ambiguous.records.sort_by(|a, b| a.path.cmp(&b.path));
    match ambiguous.coverage.get_mut("evidence manifest").unwrap() {
        Coverage::Retained { paths } => {
            paths.push(path);
            paths.sort();
        }
        _ => panic!("source archive must classify retained records"),
    }
    // The planted record must also be declared in the retained history inventory.
    for category in [
        "contract revisions",
        "actions and relevant audit receipts",
        "assurance case",
    ] {
        match ambiguous.coverage.get_mut(category).unwrap() {
            Coverage::Retained { paths } => {
                paths.push(format!(
                    "{directory}/runtime-contracts/contract-{}.json",
                    digest.strip_prefix("sha256:").unwrap_or(&digest)
                ));
                paths.sort();
            }
            _ => panic!("source archive must retain contract history"),
        }
    }
    let out = query(&ambiguous, "ambiguous.json");
    assert!(
        out["result"]["stage_inventory"]["unresolved"]
            .to_string()
            .contains("multiple retained contract snapshots match historical sources")
    );
    assert_eq!(out["result"]["qualified"], false);
}

#[test]
fn runtime_contract_snapshot_refuses_replacement_without_overwriting_capture() {
    let f = Fixture::with_runtime_count(1);
    let first = f.import();
    assert!(first.status.success());
    let first = value(&first);
    let capture = f.root.join(first["result"]["reference"].as_str().unwrap());
    let original_capture = fs::read(&capture).unwrap();
    let snapshots = f.root.join("docs/warrants/IX-WAR-0003/runtime-contracts");
    let snapshot = fs::read_dir(&snapshots)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let original = fs::read(&snapshot).unwrap();
    assert!(f.import().status.success());
    assert_eq!(fs::read(&snapshot).unwrap(), original);
    assert_eq!(fs::read(&capture).unwrap(), original_capture);
    fs::write(&snapshot, b"different owner bytes").unwrap();
    let refused = f.import();
    assert!(!refused.status.success());
    assert_eq!(
        value(&refused)["diagnostics"][0]["rule"],
        "runtime.contract-snapshot-storage"
    );
    assert_eq!(fs::read(&snapshot).unwrap(), b"different owner bytes");
    assert_eq!(fs::read(&capture).unwrap(), original_capture);
    assert_eq!(fs::read_dir(f.storage()).unwrap().count(), 1);
}

#[test]
fn archive_runtime_basis_resolves_exact_external_bytes_without_source_or_trust() {
    use openwarrant_compiler::preservation::{Archive, Limits};
    let f = Fixture::with_runtime_count(1);
    assert!(f.import().status.success());
    let embedded = f.root.join("embedded.json");
    let external = f.root.join("external.json");
    let out = war(
        &f.root,
        &[
            "archive",
            "export",
            "IX-WAR-0003",
            embedded.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let baseline = war(
        &f.root,
        &[
            "archive",
            "runtime-basis",
            embedded.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(baseline.status.success());
    let expected = value(&baseline);
    let mut archive = Archive::decode(
        &fs::read(f.root.join("embedded.json")).unwrap(),
        Limits::default(),
    )
    .unwrap();
    let evidence = f.root.join("evidence");
    fs::create_dir(&evidence).unwrap();
    for record in &mut archive.records {
        let bytes =
            openwarrant_core::attestation::base64_decode(record.base64.take().unwrap().as_str())
                .unwrap();
        fs::write(
            evidence.join(record.digest.strip_prefix("sha256:").unwrap()),
            bytes,
        )
        .unwrap();
    }
    fs::write(
        f.root.join("external.json"),
        archive.encode(Limits::default()).unwrap(),
    )
    .unwrap();
    fs::remove_dir_all(f.root.join("docs")).unwrap();
    fs::remove_file(f.root.join("openwarrant.toml")).unwrap();
    fs::remove_file(f.root.join("receipt.bin")).unwrap();
    fs::remove_file(f.root.join("embedded.json")).unwrap();
    let query = || {
        war(
            &f.root,
            &[
                "archive",
                "runtime-basis",
                external.to_str().unwrap(),
                "--evidence",
                evidence.to_str().unwrap(),
                "--json",
            ],
        )
    };
    let result = query();
    assert!(
        result.status.success(),
        "{} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let actual = value(&result);
    for field in [
        "current_contract",
        "stage_inventory",
        "provider_capture_inventory",
    ] {
        assert_eq!(actual["result"][field], expected["result"][field]);
    }
    assert_eq!(actual["result"]["authority_activated"], false);
    assert_eq!(actual["result"]["qualified"], false);
    assert_eq!(
        actual["result"]["provider_capture_inventory"]["records"][0]["native_verification"],
        "unknown"
    );
    let missing = war(
        &f.root,
        &[
            "archive",
            "runtime-basis",
            external.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!missing.status.success());
    assert!(
        String::from_utf8_lossy(&missing.stdout).contains("external evidence directory required")
    );
    // A file at the declared address is insufficient: its actual bytes must match.
    let record = archive
        .records
        .iter()
        .find(|r| r.path.contains("/runtime-receipts/"))
        .unwrap();
    let target = evidence.join(record.digest.strip_prefix("sha256:").unwrap());
    let original = fs::read(&target).unwrap();
    let mut altered = original.clone();
    altered[0] ^= 1;
    fs::write(&target, altered).unwrap();
    let bad = query();
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stdout).contains("content digest mismatch"));
    fs::remove_file(&target).unwrap();
    let absent = query();
    assert!(!absent.status.success());
    fs::write(&target, &original).unwrap();
    assert!(query().status.success());
    #[cfg(unix)]
    {
        // Matching bytes through a symlink are not an allowed evidence source.
        let linked = f.root.join("linked-evidence.bin");
        fs::write(&linked, &original).unwrap();
        fs::remove_file(&target).unwrap();
        std::os::unix::fs::symlink(&linked, &target).unwrap();
        assert!(!query().status.success());
        fs::remove_file(&target).unwrap();
        fs::write(&target, &original).unwrap();
    }
    let destination = f.root.join("inert-import");
    let imported = war(
        &f.root,
        &[
            "archive",
            "import",
            external.to_str().unwrap(),
            destination.to_str().unwrap(),
            "--evidence",
            evidence.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!imported.status.success());
    assert!(String::from_utf8_lossy(&imported.stdout).contains("required coverage unavailable"));
    assert!(!destination.exists());
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
    let one = repo
        .load_warrant(&repo.warrant_dir("IX-WAR-0003").unwrap())
        .unwrap();
    let native = |_: &StageDispatch, _: &ProviderInterface| Some(verify(&v));
    let resolution = openwarrant_cli::resolve::assess_with_runtime(
        &repo,
        &one,
        &[],
        openwarrant_cli::resolve::runtime::Input {
            selections: &selected,
            native: &native,
        },
    )
    .unwrap();
    assert!(resolution.checks.runtime_receipts_match_the_basis);
    assert_eq!(
        resolution.runtime_receipts.standing,
        ReceiptStanding::Matches
    );
    assert!(!resolution.checks.exact_authorized_contract_revision);
    assert!(!resolution.checks.independence_requirements_met);
    let unavailable = |_: &StageDispatch, _: &ProviderInterface| None;
    let resolution = openwarrant_cli::resolve::assess_with_runtime(
        &repo,
        &one,
        &[],
        openwarrant_cli::resolve::runtime::Input {
            selections: &selected,
            native: &unavailable,
        },
    )
    .unwrap();
    assert!(!resolution.checks.runtime_receipts_match_the_basis);
    assert_eq!(
        resolution.runtime_receipts.standing,
        ReceiptStanding::Unknown
    );
    // Stored MATCHES is never reused when current native verification disagrees.
    v.facts.binding.attempt_id = "wrong-new-native-observation".into();
    let native = |_: &StageDispatch, _: &ProviderInterface| Some(verify(&v));
    let resolution = openwarrant_cli::resolve::assess_with_runtime(
        &repo,
        &one,
        &[],
        openwarrant_cli::resolve::runtime::Input {
            selections: &selected,
            native: &native,
        },
    )
    .unwrap();
    assert!(!resolution.checks.runtime_receipts_match_the_basis);
    assert_eq!(
        resolution.runtime_receipts.standing,
        ReceiptStanding::Refused
    );
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
    let native = |_: &StageDispatch, _: &ProviderInterface| Some(verify(&v));
    let resolution = openwarrant_cli::resolve::assess_with_runtime(
        &repo,
        &one,
        &[],
        openwarrant_cli::resolve::runtime::Input {
            selections: &selected,
            native: &native,
        },
    )
    .unwrap();
    assert!(!resolution.checks.runtime_receipts_match_the_basis);
    assert_eq!(
        resolution.runtime_receipts.code,
        "runtime.resolution-basis-changed"
    );
    assert_eq!(
        resolution.runtime_receipts.standing,
        ReceiptStanding::Unknown
    );
    fs::write(&intent, &original).unwrap();
    // A native callback can outlive its original source snapshot. This fixture
    // changes source during verification; it does not prove native execution.
    struct SourceChangingVerifier<'a> {
        inner: &'a SyntheticVerifier,
        path: &'a Path,
        bytes: &'a [u8],
    }
    impl ReceiptVerifier for SourceChangingVerifier<'_> {
        fn interface(&self) -> &ProviderInterface {
            self.inner.interface()
        }
        fn verify(&self, raw: &[u8]) -> Result<VerifiedReceipt, ProviderFailure> {
            fs::write(self.path, self.bytes).unwrap();
            self.inner.verify(raw)
        }
    }
    let prior_path = f.root.join(response["reference"].as_str().unwrap());
    let prior_capture = fs::read(&prior_path).unwrap();
    let prior_count = fs::read_dir(f.storage()).unwrap().count();
    let mut changed_request = request.clone();
    changed_request.metadata.observation_id = "source-change-control".into();
    let mutating = SourceChangingVerifier {
        inner: &v,
        path: &intent,
        bytes: &changed,
    };
    let changed_result = capture::import(
        &repo,
        "IX-WAR-0003",
        &changed_request,
        Some(Verification {
            verifier: &mutating,
            registry_digest: None,
            authorized_capabilities: Some(&[]),
            confinement_required: true,
            hard_spend_cap_required: false,
        }),
    )
    .unwrap_err();
    assert_eq!(changed_result.code, "runtime.capture-changed");
    assert!(changed_result.unknown);
    assert_eq!(fs::read(&prior_path).unwrap(), prior_capture);
    assert_eq!(fs::read_dir(f.storage()).unwrap().count(), prior_count);
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
    let out = war(
        &f.root,
        &[
            "resolve",
            "IX-WAR-0003",
            "--dry-run",
            "--runtime-selection",
            "selected.json",
            "--json",
        ],
    );
    let resolution = value(&out);
    assert!(
        resolution["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "runtime.verifier-unavailable" && d["severity"] == "unknown")
    );
    assert!(!out.status.success());
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

#[test]
fn resolution_names_missing_current_runtime_receipts_as_unknown() {
    let f = Fixture::with_runtime_count(1);
    let out = war(&f.root, &["resolve", "IX-WAR-0003", "--dry-run", "--json"]);
    let report = value(&out);
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "runtime-basis.receipt-missing" && d["severity"] == "unknown"),
        "resolution must name its missing runtime source: {report}"
    );
}

#[cfg(unix)]
#[test]
#[ignore = "requires separately built BLUT fixture producer and native verifier"]
fn actual_blut_cpu_receipt_roundtrips_through_capture_and_current_resolution() {
    use capture::blut_process::{BlutProcessConfig, BlutProcessVerifier};
    use std::time::Duration;
    let producer =
        PathBuf::from(std::env::var_os("OW_BLUT_FIXTURE_PRODUCER").expect("fixture producer"));
    let native_verifier =
        PathBuf::from(std::env::var_os("OW_BLUT_NATIVE_VERIFIER").expect("native verifier"));
    assert!(producer.is_absolute() && native_verifier.is_absolute());
    let f = Fixture::with_executor(1, "blut");
    let binding = f.root.join("checked-binding.json");
    fs::write(&binding, serde_json::to_vec(&json!({
        "warrant_ref": f.dispatch.warrant_ref, "contract_digest":f.dispatch.contract_digest,
        "dispatch_digest":f.dispatch.dispatch_digest,"stage_id":f.dispatch.stage_id,"attempt_id":f.dispatch.attempt_id
    })).unwrap()).unwrap();
    let native = f.root.join("native-run");
    let produced = Command::new("/usr/bin/timeout")
        .arg("120")
        .arg(&producer)
        .args([&binding, &native])
        .env_clear()
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
    assert_eq!(metadata["test_key_only"], true);
    assert_eq!(metadata["assurance"], "not-established");
    let catalog = metadata["expected_catalog_digest"].as_str().unwrap();
    let interface = ProviderInterface {
        kind: ProviderKind::Blut,
        identity: "actual-local-blut-test-build".into(),
        version: "blut/openwarrant-verification/v1".into(),
    };
    let repo = openwarrant_cli::repo::Repository::open(f.root.clone().try_into().unwrap()).unwrap();
    let verifier = BlutProcessVerifier::for_recorded_dispatch(
        &repo,
        "IX-WAR-0003",
        &f.dispatch.dispatch_id,
        BlutProcessConfig {
            provider: interface.clone(),
            executable: native_verifier,
            public_key: native.join("public-key.bin"),
            plan: native.join("plan.json"),
            job: native.join("job"),
            producer_executable: native.join("producer.original"),
            scratch_root: f.root.clone(),
            timeout: Duration::from_secs(60),
            max_response_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    let raw = fs::read(native.join("receipt.json")).unwrap();
    let receipt = verifier.verify(&raw).unwrap();
    assert_eq!(receipt.binding, RuntimeBinding::from_dispatch(&f.dispatch));
    assert_eq!(receipt.outcome, RuntimeOutcome::Completed);
    assert_eq!(receipt.registry_digest.as_deref(), Some(catalog));
    assert_eq!(receipt.confinement, Observation::Unknown);
    assert_eq!(receipt.metered_cost, Observation::Unknown);
    assert_eq!(receipt.spend_cap, Observation::Unknown);
    let NativeReceipt::Blut(ref blut) = receipt.receipt else {
        panic!("native BLUT receipt required")
    };
    assert!(blut.lineage_ref.ends_with("/status.jsonl"));
    assert!(
        blut.artifact_refs
            .iter()
            .any(|p| p.ends_with("/object.ref.json"))
    );
    // Alter otherwise valid transport: the actual native signature must refuse.
    let mut altered: Value = serde_json::from_slice(&raw).unwrap();
    let octet = altered["signature"][0].as_u64().unwrap();
    altered["signature"][0] = json!(octet ^ 1);
    assert!(matches!(
        verifier.verify(&serde_json::to_vec(&altered).unwrap()),
        Err(ProviderFailure::Rejected(_))
    ));
    // Alter a same-length actual output. The original signed table must refuse.
    let fixture: Value = serde_json::from_slice(&raw).unwrap();
    let payload: Vec<u8> = serde_json::from_value(fixture["payload"].clone()).unwrap();
    let payload: Value = serde_json::from_slice(&payload).unwrap();
    let file = payload["native_files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| {
            f["path"]
                .as_str()
                .is_some_and(|p| p.ends_with("/object.ref.json"))
        })
        .unwrap();
    let output = native.join("job").join(file["path"].as_str().unwrap());
    let bytes = fs::read(&output).unwrap();
    let mut changed = bytes.clone();
    changed[0] ^= 1;
    fs::write(&output, changed).unwrap();
    assert!(matches!(
        verifier.verify(&raw),
        Err(ProviderFailure::Rejected(_))
    ));
    fs::write(&output, &bytes).unwrap();
    let mut request = f.request();
    request["provider"] =
        json!({"kind":"blut","identity":interface.identity,"version":interface.version});
    request["receipt"] = json!("native-run/receipt.json");
    request["metadata"]["original_receipt_ref"] = json!("fixture://actual-local-blut-run");
    request["metadata"]["transport"] = json!("local fixture producer and native process verifier");
    let request: Request = serde_json::from_value(request).unwrap();
    fn policy<'a>(verifier: &'a BlutProcessVerifier, registry: &'a str) -> Verification<'a> {
        Verification {
            verifier,
            registry_digest: Some(registry),
            authorized_capabilities: Some(&[]),
            confinement_required: false,
            hard_spend_cap_required: false,
        }
    }
    assert_eq!(
        capture::import(
            &repo,
            "IX-WAR-0003",
            &request,
            Some(policy(&verifier, "blake3:unrelated"))
        )
        .unwrap_err()
        .code,
        "runtime.registry-mismatch"
    );
    assert!(!f.storage().exists());
    let captured = capture::import(
        &repo,
        "IX-WAR-0003",
        &request,
        Some(policy(&verifier, catalog)),
    )
    .unwrap();
    assert_eq!(captured["native_observation"]["standing"], "matches");
    assert_eq!(captured["assurance_granted"], false);
    let selected = [capture::Selection {
        stage_id: f.dispatch.stage_id.clone(),
        capture_digest: captured["digest"].as_str().unwrap().into(),
    }];
    let one = repo
        .load_warrant(&repo.warrant_dir("IX-WAR-0003").unwrap())
        .unwrap();
    let resolve_native =
        |_: &StageDispatch, _: &ProviderInterface| Some(policy(&verifier, catalog));
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
    fs::rename(native.join("job"), native.join("unavailable-job")).unwrap();
    assert!(matches!(
        verifier.verify(&raw),
        Err(ProviderFailure::Unavailable(_))
    ));
    // Historical MATCHES cannot replace native evidence that is unavailable now.
    assert_eq!(
        capture::assess_selected(&repo, "IX-WAR-0003", &selected, |_, _| Some(policy(
            &verifier, catalog
        )))
        .unwrap()
        .standing,
        ReceiptStanding::Unknown
    );
    println!(
        "PASS actual CPU/native seal/capture/current resolution; tampered signature/output and wrong catalog refused; missing job UNKNOWN; no assurance"
    );
}

#[cfg(unix)]
#[test]
#[ignore = "requires separately built Katana local fixture producer and native verifier"]
fn actual_katana_turn_roundtrips_through_capture_and_current_resolution() {
    use capture::katana_process::{KatanaProcessConfig, KatanaProcessVerifier};
    use std::time::Duration;
    let producer =
        PathBuf::from(std::env::var_os("OW_KATANA_FIXTURE_PRODUCER").expect("fixture producer"));
    let executable =
        PathBuf::from(std::env::var_os("OW_KATANA_NATIVE_VERIFIER").expect("native verifier"));
    assert!(producer.is_absolute() && executable.is_absolute());
    let f = Fixture::with_runtime_count(1);
    let binding = f.root.join("checked-binding.json");
    fs::write(&binding, serde_json::to_vec(&json!({
        "warrant_ref":f.dispatch.warrant_ref,"contract_digest":f.dispatch.contract_digest,
        "dispatch_digest":f.dispatch.dispatch_digest,"stage_id":f.dispatch.stage_id,"attempt_id":f.dispatch.attempt_id
    })).unwrap()).unwrap();
    let native = f.root.join("native-local-turn");
    let produced = Command::new("/usr/bin/timeout")
        .arg("30")
        .arg(producer)
        .arg(&binding)
        .arg(&native)
        .output()
        .unwrap();
    assert!(
        produced.status.success(),
        "{}",
        String::from_utf8_lossy(&produced.stderr)
    );
    let metadata: Value =
        serde_json::from_slice(&fs::read(native.join("fixture.json")).unwrap()).unwrap();
    assert_eq!(metadata["scripted_provider"], true);
    assert_eq!(metadata["cost_metered"], false);
    // This head is fixture instrumentation, NOT protected production custody.
    let head = metadata["expected_log_head"].as_str().unwrap();
    let interface = ProviderInterface {
        kind: ProviderKind::Katana,
        identity: "actual-local-katana-test-build".into(),
        version: "katana/openwarrant-verification/v1".into(),
    };
    let repo = openwarrant_cli::repo::Repository::open(f.root.clone().try_into().unwrap()).unwrap();
    let config = |head: &str| KatanaProcessConfig {
        provider: interface.clone(),
        executable: executable.clone(),
        event_log: native.join("session.jsonl"),
        trusted_log_head: head.into(),
        scratch_root: f.root.clone(),
        timeout: Duration::from_secs(30),
        max_response_bytes: 1024 * 1024,
    };
    let verifier = KatanaProcessVerifier::for_recorded_dispatch(
        &repo,
        "IX-WAR-0003",
        &f.dispatch.dispatch_id,
        config(head),
    )
    .unwrap();
    let raw = fs::read(native.join("receipt.json")).unwrap();
    let checked = verifier.verify(&raw).unwrap();
    assert_eq!(checked.binding, RuntimeBinding::from_dispatch(&f.dispatch));
    assert_eq!(checked.outcome, RuntimeOutcome::Completed);
    assert_eq!(checked.confinement, Observation::Unknown);
    assert_eq!(checked.metered_cost, Observation::Unknown);
    assert_eq!(checked.spend_cap, Observation::Unknown);
    let NativeReceipt::Katana(receipt) = checked.receipt else {
        panic!("native Katana receipt required");
    };
    // The fixture's declared bounded action allows exactly this read. Do not
    // derive authorization from the returned receipt's requested capabilities.
    let allowed = vec![r#"{"glob":"input.txt","kind":"fs","mode":"read"}"#.to_owned()];
    assert_eq!(receipt.realized_capabilities, allowed);
    assert_eq!(receipt.receipt_digest, head);
    assert_eq!(receipt.runtime_event_log_head, head);
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
    for (sandbox, spend, code) in [
        (true, false, "runtime.confinement-unestablished"),
        (false, true, "runtime.accounting-unestablished"),
    ] {
        let required = assess_runtime_receipt(
            &RuntimeExpectation {
                dispatch: &f.dispatch,
                provider: &interface,
                registry_digest: None,
                authorized_capabilities: Some(&allowed),
                confinement_required: sandbox,
                hard_spend_cap_required: spend,
                max_receipt_bytes: 4 * 1024 * 1024,
            },
            &raw,
            Some(&verifier),
        );
        assert_eq!(required.standing, ReceiptStanding::Unknown);
        assert_eq!(required.code, code);
    }
    let mut changed: Value = serde_json::from_slice(&raw).unwrap();
    changed["assurance_granted"] = json!(true);
    assert!(matches!(
        verifier.verify(&serde_json::to_vec(&changed).unwrap()),
        Err(ProviderFailure::Rejected(_))
    ));
    let log = native.join("session.jsonl");
    let original = fs::read(&log).unwrap();
    let mut altered = original.clone();
    altered[0] ^= 1;
    fs::write(&log, &altered).unwrap();
    assert!(matches!(
        verifier.verify(&raw),
        Err(ProviderFailure::Rejected(_))
    ));
    fs::write(&log, &original).unwrap();
    let wrong = KatanaProcessVerifier::for_recorded_dispatch(
        &repo,
        "IX-WAR-0003",
        &f.dispatch.dispatch_id,
        config(&format!("b3:{}", "0".repeat(64))),
    )
    .unwrap();
    assert!(matches!(
        wrong.verify(&raw),
        Err(ProviderFailure::Rejected(_))
    ));
    fs::write(f.root.join("receipt.bin"), &raw).unwrap();
    let mut request = f.request();
    request["provider"] =
        json!({"kind":"katana","identity":interface.identity,"version":interface.version});
    request["metadata"]["transport"] =
        json!("actual scripted local Katana turn and native process verifier");
    request["metadata"]["original_receipt_ref"] =
        json!(native.join("receipt.json").to_str().unwrap());
    let request: Request = serde_json::from_value(request).unwrap();
    let response = capture::import(&repo, "IX-WAR-0003", &request, Some(policy())).unwrap();
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
    fs::rename(&log, native.join("unavailable-log")).unwrap();
    assert!(matches!(
        verifier.verify(&raw),
        Err(ProviderFailure::Unavailable(_))
    ));
    assert_eq!(
        capture::assess_selected(&repo, "IX-WAR-0003", &selected, |_, _| Some(policy()))
            .unwrap()
            .standing,
        ReceiptStanding::Unknown
    );
    println!(
        "PASS actual local turn/native log/capture/current resolution; capability excess, changed log, wrong head and fake assurance refused; missing log UNKNOWN; no assurance"
    );
}

#[cfg(unix)]
fn synthetic_blut_input_fixture() -> (Fixture, String) {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::with_executor(1, "blut");
    let binding = json!({"warrant_ref":f.dispatch.warrant_ref,"contract_digest":f.dispatch.contract_digest,"dispatch_digest":f.dispatch.dispatch_digest,"stage_id":f.dispatch.stage_id,"attempt_id":f.dispatch.attempt_id});
    let run_id = f.dispatch.attempt_id.clone();
    let job = f.root.join("native/job");
    fs::create_dir_all(job.join("_openwarrant_inputs")).unwrap();
    let plan = b"{\"steps\":[]}";
    let registry = b"{}";
    let native_binding = serde_json::to_vec(&json!({"dispatch":binding,"run_id":run_id})).unwrap();
    let mut files = Vec::new();
    for (path, bytes) in [
        (
            "_openwarrant_inputs/binding.json",
            native_binding.as_slice(),
        ),
        ("_openwarrant_inputs/plan.json", plan.as_slice()),
        ("_openwarrant_inputs/registry.json", registry.as_slice()),
        (
            "status.jsonl",
            b"synthetic failure; not native execution\n".as_slice(),
        ),
    ] {
        fs::write(job.join(path), bytes).unwrap();
        fs::set_permissions(job.join(path), fs::Permissions::from_mode(0o644)).unwrap();
        files.push(json!({"path":path,"bytes":bytes.len(),"digest":format!("blake3:{}",blake3::hash(bytes).to_hex()),"mode":0o100644}));
    }
    let producer = b"synthetic producer data; never executed";
    fs::write(f.root.join("native/producer.bin"), producer).unwrap();
    fs::write(f.root.join("native/public-key.bin"), [0u8; 32]).unwrap();
    fs::write(f.root.join("native/plan.json"), plan).unwrap();
    fs::write(
        f.root.join("native/binding.json"),
        serde_json::to_vec(&binding).unwrap(),
    )
    .unwrap();
    let payload = json!({"version":1,"binding":binding,"run_id":run_id,"plan_digest":format!("blake3:{}",blake3::hash(plan).to_hex()),"registry_digest":format!("blake3:{}",blake3::hash(registry).to_hex()),"binary_digest":format!("blake3:{}",blake3::hash(producer).to_hex()),"started_unix_ms":0,"ended_unix_ms":1,"outcome":"failed","warnings":0,"stages":0,"native_files":files});
    fs::write(
        f.root.join("receipt.bin"),
        serde_json::to_vec(
            &json!({"payload":serde_json::to_vec(&payload).unwrap(),"signature":vec![0u8;64]}),
        )
        .unwrap(),
    )
    .unwrap();
    let mut request = f.request();
    request["provider"] = json!({"kind":"blut","identity":"synthetic-input-transport","version":"blut/openwarrant-verification/v1"});
    fs::write(
        f.root.join("capture-request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    let imported = f.import();
    assert!(
        imported.status.success(),
        "{}",
        String::from_utf8_lossy(&imported.stdout)
    );
    let imported: Value = serde_json::from_slice(&imported.stdout).unwrap();
    let digest = imported["result"]["digest"].as_str().unwrap().to_owned();
    fs::write(f.root.join("native-inputs-request.json"),serde_json::to_vec(&json!({"schema":"oh.war/preservation-runtime-inputs-request/v1-draft.1","public_key":"native/public-key.bin","plan":"native/plan.json","binding":"native/binding.json","producer":"native/producer.bin","job":"native/job"})).unwrap()).unwrap();
    (f, digest)
}

#[cfg(unix)]
#[test]
fn native_inputs_retain_every_declared_file_without_running_or_trusting_imports() {
    let (f, digest) = synthetic_blut_input_fixture();
    let args = [
        "archive",
        "retain-runtime",
        "IX-WAR-0003",
        "--capture",
        &digest,
        "--request",
        "native-inputs-request.json",
        "--json",
    ];
    let result = war(&f.root, &args);
    assert!(
        result.status.success(),
        "{} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let result: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(result["result"]["retained"], true);
    assert_eq!(result["result"]["native_authentication_established"], false);
    assert_eq!(result["result"]["authority_activated"], false);
    let manifest_path = result["result"]["manifest"].as_str().unwrap();
    let manifest: Value =
        serde_json::from_slice(&fs::read(f.root.join(manifest_path)).unwrap()).unwrap();
    assert_eq!(manifest["capture_digest"], digest);
    assert_eq!(manifest["inputs"].as_object().unwrap().len(), 8);
    fs::remove_dir_all(f.root.join("native")).unwrap();
    let directory = f.root.join("docs/warrants/IX-WAR-0003/native-inputs");
    for input in manifest["inputs"].as_object().unwrap().values() {
        let bytes = fs::read(directory.join(input["blob"].as_str().unwrap())).unwrap();
        assert_eq!(
            input["digest"],
            format!("sha256:{}", openwarrant_compiler::sha256_hex(&bytes))
        );
    }
    assert!(
        fs::read(directory.join(manifest["inputs"]["producer"]["blob"].as_str().unwrap()))
            .unwrap()
            .starts_with(b"synthetic producer data")
    );
}

#[cfg(unix)]
#[test]
fn archive_queries_reconnect_native_inputs_after_original_job_removal() {
    let (f, digest) = synthetic_blut_input_fixture();
    let retained = war(
        &f.root,
        &[
            "archive",
            "retain-runtime",
            "IX-WAR-0003",
            "--capture",
            &digest,
            "--request",
            "native-inputs-request.json",
        ],
    );
    assert!(
        retained.status.success(),
        "{}",
        String::from_utf8_lossy(&retained.stderr)
    );
    fs::remove_dir_all(f.root.join("native")).unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["-c", "core.hooksPath=/dev/null", "add", "."],
        vec![
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "retain synthetic archive inputs",
        ],
    ] {
        let out = Command::new("git")
            .current_dir(&f.root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let transport = f.root.join("transport.json");
    let transport = transport.to_str().unwrap();
    let exported = war(
        &f.root,
        &[
            "archive",
            "export",
            "IX-WAR-0003",
            transport,
            "--history",
            "--json",
        ],
    );
    assert!(
        exported.status.success(),
        "{} {}",
        String::from_utf8_lossy(&exported.stdout),
        String::from_utf8_lossy(&exported.stderr)
    );
    let query = war(&f.root, &["archive", "runtime-basis", transport, "--json"]);
    assert!(
        query.status.success(),
        "{} {}",
        String::from_utf8_lossy(&query.stdout),
        String::from_utf8_lossy(&query.stderr)
    );
    let query: Value = serde_json::from_slice(&query.stdout).unwrap();
    let records = query["result"]["provider_capture_inventory"]["records"]
        .as_array()
        .unwrap();
    assert!(!records.is_empty());
    for record in records {
        assert_eq!(record["native_inputs_reconnected"], true);
        assert!(!record["native_input_paths"].as_array().unwrap().is_empty());
        assert_eq!(record["native_verification"], "unknown");
        assert_eq!(record["assurance_granted"], false);
    }
}

#[cfg(unix)]
#[test]
fn native_input_mismatches_and_limits_refuse_before_manifest_publication() {
    use std::os::unix::fs::PermissionsExt;
    for case in [
        "changed-file",
        "changed-mode",
        "extra-file",
        "linked-job",
        "record-limit",
        "content-limit",
    ] {
        let (f, digest) = synthetic_blut_input_fixture();
        let mut options = Vec::new();
        let expected = match case {
            "changed-file" => {
                fs::write(
                    f.root.join("native/job/status.jsonl"),
                    b"changed native observation",
                )
                .unwrap();
                "native job bytes or mode differ"
            }
            "changed-mode" => {
                fs::set_permissions(
                    f.root.join("native/job/status.jsonl"),
                    fs::Permissions::from_mode(0o600),
                )
                .unwrap();
                "native job bytes or mode differ"
            }
            "extra-file" => {
                fs::write(f.root.join("native/job/unlisted.txt"), b"not in receipt").unwrap();
                "native job file table differs"
            }
            "linked-job" => {
                fs::rename(f.root.join("native/job"), f.root.join("native/elsewhere")).unwrap();
                std::os::unix::fs::symlink("elsewhere", f.root.join("native/job")).unwrap();
                "native input native/job"
            }
            "record-limit" => {
                options.extend(["--max-records", "8"]);
                "native input record count exceeds limit"
            }
            "content-limit" => {
                options.extend(["--max-content-bytes", "1"]);
                "native input content exceeds limit"
            }
            _ => unreachable!(),
        };
        let mut args = vec![
            "archive",
            "retain-runtime",
            "IX-WAR-0003",
            "--capture",
            &digest,
            "--request",
            "native-inputs-request.json",
        ];
        args.extend(options);
        let result = war(&f.root, &args);
        assert!(!result.status.success(), "{case}");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(expected),
            "{case}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            !f.root
                .join("docs/warrants/IX-WAR-0003/native-inputs")
                .exists(),
            "{case}: no publication"
        );
        assert_eq!(
            fs::read_dir(f.storage()).unwrap().count(),
            1,
            "original capture preserved"
        );
    }
}
