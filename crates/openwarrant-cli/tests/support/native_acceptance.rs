// SPDX-License-Identifier: Apache-2.0
//! Synthetic protocol fixture only. No project acceptance or human observation.
use openwarrant_cli::{
    authorize, bundle,
    diagnostic::{Report, Severity},
    repo::Repository,
    resolution_cmd::{self, native},
    runtime_capture::{Selection, native_host::NativeAssessment},
    sign::IngestMode,
};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

const ALIAS: &str = "IX-WAR-0003";
const ACTOR: &str = "software fixture owner";

fn command(root: &Path, argv: &[&str]) -> Output {
    Command::new("/usr/bin/timeout")
        .arg("15")
        .args(argv)
        .current_dir(root)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("TMPDIR", root)
        .env("OPENWARRANT_NO_PROJECTS", "1")
        .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
        .output()
        .unwrap()
}
fn success(root: &Path, argv: &[&str]) -> Output {
    let out = command(root, argv);
    assert!(
        out.status.success(),
        "{argv:?}: {} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    out
}
fn observed(report: &Report) {
    assert!(
        !report
            .diagnostics
            .iter()
            .any(|d| matches!(d.severity, Severity::Error | Severity::Unknown)),
        "{report:?}"
    );
}
fn open(root: &Path) -> Repository {
    Repository::open(root.to_path_buf().try_into().unwrap()).unwrap()
}
fn commit(root: &Path, message: &str) {
    success(root, &["/usr/bin/git", "add", "-A"]);
    success(
        root,
        &[
            "/usr/bin/git",
            "-c",
            "user.name=synthetic-native-fixture",
            "-c",
            "user.email=fixture@invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-qm",
            message,
        ],
    );
}

/// Assemble a new synthetic subject from copied atoms before authorization.
/// The readonly original inbox fixture and all real records remain untouched.
pub fn setup(root: &Path, store: &Path, uuid: &str) {
    let dir = root.join("docs/warrants").join(ALIAS);
    for name in ["authorization.toml", "journal.jsonl"] {
        let path = dir.join(name);
        if path.exists() {
            fs::remove_file(path).unwrap();
        }
    }
    for entry in fs::read_dir(dir.join("atoms")).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path)
            .unwrap()
            .replace("01a0ac9b-3953-7511-86f6-7face232c663", uuid);
        fs::write(path, text).unwrap();
    }
    let config = root.join("openwarrant.toml");
    let text = fs::read_to_string(&config)
        .unwrap()
        .replace("name = \"Inbox\"", "name = \"fixture\"");
    fs::write(config, format!("{text}\n[authority]\nstore = {store:?}\n")).unwrap();
    fs::create_dir_all(root.join("docs/gates")).unwrap();
    fs::write(root.join("ADOPTED.md"), "").unwrap();
    assert!(
        !command(root, &["/usr/bin/test", "-s", "ADOPTED.md"])
            .status
            .success()
    );
    fs::write(
        root.join("ADOPTED.md"),
        "synthetic native acceptance fixture\n",
    )
    .unwrap();
    success(root, &["/usr/bin/test", "-s", "ADOPTED.md"]);
    // This gate observes precisely its declared input, with an actual paired
    // empty-file refusal. It makes no claim about production code quality.
    fs::write(root.join("docs/gates/software.fixture.native-acceptance@1.0.0.yaml"), r#"gate_id: "software.fixture.native-acceptance"
version: "1.0.0"
lifecycle: "qualified"
implementation_ref: "artifact://coreutils/test"
output_schema_ref: "schema://exit-status/v1"
provenance: "local_candidate"
input_kinds: ["fixture-artifact"]
argv: ["/usr/bin/test", "-s", "ADOPTED.md"]
inputs: ["ADOPTED.md"]
fixtures: ["ADOPTED.md"]
mutating: "false"
timeout_secs: "5"
fault_model: ["fixture-artifact-empty"]
known_blind_spots: ["Synthetic protocol fixture; observes file length only. No product assurance."]
qualification_qualifier: "synthetic protocol fixture paired test, not project qualification"
qualification_digest: ""
qualification_positive_controls: ["Nonempty synthetic ADOPTED.md observed to pass."]
qualification_negative_controls: ["Empty synthetic ADOPTED.md observed to refuse."]
qualification_mutation_classes: ["empty artifact"]
qualification_environments: ["disposable Linux namespace, execution UID 1"]
qualification_limitations: ["No real human review, independent quality assessment or hardware qualification."]
detection_results:
  - fault_class: "fixture-artifact-empty"
    mutation: "empty ADOPTED.md"
    detected: "true"
"#).unwrap();
    fs::write(
        dir.join("atoms/60-assurance.md"),
        format!(
            r#"---
schema: oh.war/atom/v1
warrant_uuid: {uuid}
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Synthetic acceptance seam

### OBL-001 — a nonempty fixture artifact is supplied
- **scope:** `ADOPTED.md` only; no product quality claim.
- **gate:** `gate://software.fixture.native-acceptance@1.0.0`
- **evidence:** actual file-length check and simulated verification protocol response.

### OBL-002 — the fixture acceptance inputs are reviewable
- **scope:** this fixture's exact review packet and native acceptance response.
- **gate:** `gate://software.fixture.native-acceptance@1.0.0`
- **evidence:** a distinct fixture verifier's response; no real independent qualification.

## Gate Adequacy
**Adversarial question:** does an empty artifact pass the gate? It was observed to refuse.
- **outcome:** no_counterexample
"#
        ),
    )
    .unwrap();
    let digest = format!(
        "sha256:{}",
        openwarrant_compiler::sha256_hex(&fs::read(root.join("ADOPTED.md")).unwrap())
    );
    fs::write(
        dir.join("deliverables.toml"),
        format!(
            r#"schema = "oh.war/deliverables/v1"
[[deliverable]]
id = "D-001"
title = "Synthetic artifact"
kind = "file"
target_ref = "ADOPTED.md"
required = true
content_addressed = true
provenance_required = true
obligation_refs = ["OBL-001", "OBL-002"]
[deliverable.provenance]
producer = "synthetic-native-fixture"
producing_attempt = "fixture-attempt"
contract_digest = "unrecorded"
input_digests = []
tool_or_runtime_identity = "test fixture"
creation_method = "generated"
content_digest = "{digest}"
media_type = "text/markdown"
classification = "internal"
retention = "fixture-lifetime"
source_holder = "git"
"#
        ),
    )
    .unwrap();
    fs::write(
        dir.join("rationale.toml"),
        "schema = \"oh.war/rationale/v1\"\n",
    )
    .unwrap();
    success(root, &["/usr/bin/git", "init", "-q"]);
    let repo = open(root);
    let request = authorize::request(&repo, ALIAS).unwrap();
    let response: authorize::AuthorizationResponse = serde_json::from_value(serde_json::json!({
        "schema": "oh.war/authorization-response/v1", "warrant": ALIAS,
        "contract_digest": request.contract_digest, "authorizer": ACTOR, "acting_role": "authorizer",
        "meaning": "Synthetic namespace fixture only; not a human project act.",
        "effective_time": "2026-10-10T00:00:00Z", "independence": "separate_role",
        "deliverable_set_digest": request.deliverable_set_digest
    })).unwrap();
    let bytes = toml::to_string(&response).unwrap();
    let path = root.join(format!("docs/authority/responses/{ALIAS}.response.toml"));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, &bytes).unwrap();
    let (_, signature) = super::fixture("oh.war/response", bytes.as_bytes(), 11);
    fs::write(path.with_extension("toml.sig"), signature).unwrap();
    observed(&authorize::ingest(&repo, ALIAS, &path.try_into().unwrap()).unwrap());
    commit(root, "synthetic fixture authorization and delivery");
}

/// All thirteen and actual native receipts, then signed snapshot refusal and
/// exact ingestion. The test key is software-only and remains in memory.
pub fn roundtrip(root: &Path, store: &Path, selected: &[Selection]) {
    let repo = open(root);
    observed(&openwarrant_cli::evidence::record(&repo, ALIAS, None, None).unwrap());
    let (report, packets) = bundle::emit(&repo, ALIAS, &repo.performer()).unwrap();
    observed(&report);
    assert_eq!(packets["packets"].as_array().unwrap().len(), 1);
    let packet = root.join(packets["packets"][0]["path"].as_str().unwrap());
    let output = Command::new("/usr/bin/timeout")
        .arg("15")
        .arg("/fixture-verifier")
        .arg(packet)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("TMPDIR", root)
        .env(
            "OPENWARRANT_REVIEWED_PACKETS",
            serde_json::to_string(&packets["packets"]).unwrap(),
        )
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let verifier_path = root.join("fixture-verifier-response.toml");
    fs::write(&verifier_path, output.stdout).unwrap();
    observed(
        &openwarrant_cli::verify::ingest(&repo, ALIAS, &verifier_path.try_into().unwrap()).unwrap(),
    );
    commit(
        root,
        "synthetic native receipts, gate evidence and verification protocol",
    );
    let native = NativeAssessment::load(&repo, ALIAS, selected, store).unwrap();
    let intent = resolution_cmd::ResolutionResponse {
        schema: resolution_cmd::RESPONSE_SCHEMA.into(),
        warrant: ALIAS.into(),
        contract_digest: String::new(),
        resolved_by: ACTOR.into(),
        acting_role: "resolver".into(),
        common_outcome: openwarrant_core::resolution::CommonOutcome::Satisfied,
        profile_outcome: "delivered".into(),
        meaning: "Synthetic acceptance only; no real human or project qualification.".into(),
        effective_time: "2026-10-10T00:00:00Z".into(),
    };
    let response = native::prepare(&repo, ALIAS, &intent, selected, &native)
        .unwrap_or_else(|f| panic!("all thirteen native preparation: {f:?}"));
    assert!(
        !response.require_user_presence,
        "fixture policy is explicitly software-only"
    );
    let directory = repo.warrant_dir(ALIAS).unwrap();
    let record_path = directory.join("resolution.toml");
    let ingest = |response: &native::Response, mode| {
        let text = toml::to_string(response).unwrap();
        let (_, signature) = super::fixture("oh.war/response", text.as_bytes(), 11);
        native::ingest(&repo, ALIAS, text.as_bytes(), &signature, &native, mode)
    };
    for field in ["assurance", "artifact", "policy", "locator", "gate"] {
        let mut changed = response.clone();
        let wrong = format!("sha256:{}", "b".repeat(64));
        match field {
            "assurance" => changed.assurance_case_snapshot_digest = wrong,
            "artifact" => changed.artifact_manifest_digest = wrong,
            "policy" => changed.policy.authority_digest = wrong,
            "locator" => changed.locator.commit_sha = "b".repeat(40),
            "gate" => changed.gate_run_refs.push("invented-gate.toml".into()),
            _ => unreachable!(),
        }
        let f = ingest(&changed, IngestMode::Record).unwrap_err();
        assert_eq!(f.code, "native-resolution.stale-response", "{field}: {f:?}");
        assert!(!record_path.exists());
    }
    let mut missing = response.clone();
    missing.selections.clear();
    assert!(
        ingest(&missing, IngestMode::Record).is_err(),
        "cached native verdict reused without selections"
    );
    assert!(!record_path.exists());
    let artifact = root.join("ADOPTED.md");
    let original = fs::read(&artifact).unwrap();
    fs::write(&artifact, "changed after review\n").unwrap();
    assert!(
        ingest(&response, IngestMode::Record).is_err(),
        "changed artifact accepted"
    );
    assert!(!record_path.exists());
    fs::write(artifact, original).unwrap();
    let text = toml::to_string(&response).unwrap();
    let (_, signature) = super::fixture("oh.war/response", text.as_bytes(), 11);
    let response_path = camino::Utf8Path::new(".git/native-response.toml");
    fs::write(root.join(response_path), &text).unwrap();
    fs::write(root.join(".git/native-response.toml.sig"), signature).unwrap();
    assert_eq!(
        native::ingest_selected_from_path(
            &repo,
            ALIAS,
            response_path,
            &[],
            &native,
            IngestMode::Record
        )
        .unwrap_err()
        .code,
        "native-resolution.selection-mismatch"
    );
    assert!(!record_path.exists());
    observed(
        &native::ingest_selected_from_path(
            &repo,
            ALIAS,
            response_path,
            selected,
            &native,
            IngestMode::DryRun,
        )
        .unwrap(),
    );
    assert!(!record_path.exists(), "dry run wrote an acceptance");
    let selection_path = ".git/native-selection.json";
    fs::write(
        root.join(selection_path),
        serde_json::to_vec(&serde_json::json!({
            "schema": "oh.war/runtime-selection-request/v1-draft.1", "selections": selected
        }))
        .unwrap(),
    )
    .unwrap();
    let run_cli = |dry: bool| {
        let mut args = vec![
            "/war",
            "resolve",
            ALIAS,
            "--runtime-selection",
            selection_path,
            "--native-store",
            store.to_str().unwrap(),
            "--response",
            response_path.as_str(),
            "--json",
        ];
        if dry {
            args.push("--dry-run");
        }
        let output = success(root, &args);
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["command"], "resolve.native_response", "{value}");
    };
    run_cli(true);
    assert!(!record_path.exists(), "CLI dry run wrote an acceptance");
    run_cli(false);
    let record = repo.load_resolution(&directory).unwrap().unwrap();
    assert!(
        matches!(native::signature_for_record(&repo, &record), openwarrant_cli::authority_check::Verdict::Signed { principal, .. } if principal == "owner")
    );
    assert_eq!(
        ingest(&response, IngestMode::Record).unwrap_err().code,
        "native-resolution.exists"
    );
    println!(
        "native acceptance: all thirteen plus actual native receipt; signed snapshot substitutions, missing selection, changed artifact and replay refused; exact dry-run then record accepted; principal owner differs from actor label; synthetic only"
    );
}
