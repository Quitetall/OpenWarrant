// SPDX-License-Identifier: Apache-2.0
#![cfg(unix)]
use openwarrant_compiler::{
    preservation::{Archive, Coverage, Limits, Record, SCHEMA},
    sha256_hex,
};
use openwarrant_core::{attestation::base64_encode, journal::EXPORT_CONTENTS};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ow-archive-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
    fn archive(&self, external: bool) -> Vec<u8> {
        let records: Vec<_> = ["annulled", "disputed", "superseded"]
            .into_iter()
            .map(|state| {
                let bytes = format!("retained fixture state: {state}\r\n").into_bytes();
                let hex = sha256_hex(&bytes);
                if external {
                    std::fs::create_dir_all(self.0.join("evidence")).unwrap();
                    std::fs::write(self.0.join("evidence").join(&hex), &bytes).unwrap();
                }
                Record {
                    path: format!("history/{state}.txt"),
                    digest: format!("sha256:{hex}"),
                    base64: (!external).then(|| base64_encode(&bytes)),
                }
            })
            .collect();
        let mut coverage: BTreeMap<_, _> = EXPORT_CONTENTS
            .iter()
            .filter(|s| !s.starts_with("optional "))
            .map(|s| {
                (
                    (*s).into(),
                    Coverage::Absent {
                        reason: "synthetic transport fixture".into(),
                    },
                )
            })
            .collect();
        coverage.insert(
            "resolution and standing".into(),
            Coverage::Retained {
                paths: records.iter().map(|r| r.path.clone()).collect(),
            },
        );
        let archive = Archive {
            schema: SCHEMA.into(),
            subject: "fixture://history".into(),
            producer: "fixture://cli".into(),
            records,
            coverage,
            extensions: BTreeMap::new(),
        };
        let bytes = archive.encode(Limits::default()).unwrap();
        std::fs::write(self.0.join("source.json"), &bytes).unwrap();
        bytes
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn success(out: Output) {
    assert!(
        out.status.success(),
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
fn refusal(out: Output, expected: &str) {
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains(expected),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn source_detached_history_roundtrip_preserves_exact_bytes_without_authority() {
    let f = Fixture::new();
    let bytes = f.archive(false);
    success(f.run(&["archive", "import", "source.json", "imported"]));
    std::fs::remove_file(f.0.join("source.json")).unwrap();
    success(f.run(&["archive", "reexport", "imported", "again.json"]));
    assert_eq!(std::fs::read(f.0.join("again.json")).unwrap(), bytes);
    for state in ["annulled", "disputed", "superseded"] {
        assert_eq!(
            std::fs::read(f.0.join(format!("imported/records/history/{state}.txt"))).unwrap(),
            format!("retained fixture state: {state}\r\n").as_bytes()
        );
    }
    assert!(!f.0.join("imported/.git").exists());
    assert!(!f.0.join("imported/docs/authority").exists());
    refusal(
        f.run(&["archive", "reexport", "imported", "again.json"]),
        "File exists",
    );
    std::fs::write(
        f.0.join("imported/records/history/annulled.txt"),
        "tampered",
    )
    .unwrap();
    refusal(
        f.run(&["archive", "reexport", "imported", "tampered.json"]),
        "imported record changed",
    );
    assert!(!f.0.join("tampered.json").exists());
}

#[test]
fn external_evidence_is_observed_and_then_source_can_disappear() {
    let f = Fixture::new();
    let bytes = f.archive(true);
    refusal(
        f.run(&["archive", "import", "source.json", "missing"]),
        "external evidence directory required",
    );
    assert!(!f.0.join("missing").exists());
    success(f.run(&[
        "archive",
        "import",
        "source.json",
        "imported",
        "--evidence",
        "evidence",
    ]));
    refusal(
        f.run(&[
            "archive",
            "import",
            "source.json",
            "imported",
            "--evidence",
            "evidence",
        ]),
        "new destination required",
    );
    std::fs::remove_dir_all(f.0.join("evidence")).unwrap();
    std::fs::remove_file(f.0.join("source.json")).unwrap();
    success(f.run(&["archive", "reexport", "imported", "again.json"]));
    assert_eq!(std::fs::read(f.0.join("again.json")).unwrap(), bytes);
}

#[test]
fn symlink_sources_destinations_and_imported_records_refuse() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    f.archive(false);
    symlink("source.json", f.0.join("link.json")).unwrap();
    assert!(
        !f.run(&["archive", "import", "link.json", "nope"])
            .status
            .success()
    );
    std::fs::create_dir(f.0.join("outside")).unwrap();
    symlink("outside", f.0.join("parent-link")).unwrap();
    assert!(
        !f.run(&["archive", "import", "source.json", "parent-link/nope"])
            .status
            .success()
    );
    assert!(!f.0.join("outside/nope").exists());
    success(f.run(&["archive", "import", "source.json", "imported"]));
    let path = f.0.join("imported/records/history/annulled.txt");
    std::fs::remove_file(&path).unwrap();
    symlink(f.0.join("source.json"), &path).unwrap();
    assert!(
        !f.run(&["archive", "reexport", "imported", "nope.json"])
            .status
            .success()
    );
    assert!(!f.0.join("nope.json").exists());
}

#[test]
fn machine_output_keeps_report_envelope_and_explicit_authority_bound() {
    let f = Fixture::new();
    f.archive(false);
    let out = f.run(&["--json", "archive", "import", "source.json", "imported"]);
    assert!(out.status.success());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["schema"], "oh.war/report/v1");
    assert_eq!(value["result"]["authority_activated"], false);
    assert_eq!(value["result"]["operation"], "import");
}

#[test]
fn actual_warrant_sources_reconstruct_after_source_repository_disappears() {
    let f = Fixture::new();
    success(f.run(&[
        "init",
        "--namespace",
        "ARCH",
        "--program",
        "Archive fixture",
    ]));
    success(f.run(&["new", "Retain actual source bytes"]));
    let output = f.run(&[
        "--json",
        "archive",
        "export",
        "ARCH-WAR-0001",
        "snapshot.json",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["result"]["complete"], false);
    let captured = Archive::decode(
        &std::fs::read(f.0.join("snapshot.json")).unwrap(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        report["result"]["coverage"],
        serde_json::to_value(&captured.coverage).unwrap()
    );
    let unavailable = report["result"]["unavailable_categories"]
        .as_array()
        .unwrap();
    assert!(unavailable.iter().any(|v| v == "artifacts"));
    assert!(!unavailable.iter().any(|v| v == "canonical IR"));

    std::fs::rename(f.0.join("docs"), f.0.join("original-docs-hidden")).unwrap();
    let inspected = f.run(&["archive", "inspect", "snapshot.json", "--json"]);
    assert!(inspected.status.success());
    let inspected: serde_json::Value = serde_json::from_slice(&inspected.stdout).unwrap();
    assert_eq!(
        inspected["result"]["coverage"],
        report["result"]["coverage"]
    );
    assert_eq!(
        inspected["result"]["unavailable_categories"],
        report["result"]["unavailable_categories"]
    );

    // Structural capture does not claim KF or historical completeness.
    refusal(
        f.run(&["archive", "import", "snapshot.json", "not-complete"]),
        "required coverage unavailable",
    );
    assert!(!f.0.join("not-complete").exists());
    let bytes = std::fs::read(f.0.join("snapshot.json")).unwrap();
    let mut archive = Archive::decode(&bytes, Limits::default()).unwrap();
    let mut false_contracts = archive.clone();
    false_contracts.coverage.insert(
        "contract revisions".into(),
        Coverage::Retained {
            paths: vec!["__ow_archive__/basis.json".into()],
        },
    );
    std::fs::write(
        f.0.join("false-contract-coverage.json"),
        false_contracts.encode(Limits::default()).unwrap(),
    )
    .unwrap();
    refusal(
        f.run(&["archive", "inspect", "false-contract-coverage.json"]),
        "contract revision coverage differs",
    );
    let mut false_absence = archive.clone();
    false_absence.coverage.insert(
        "artifacts".into(),
        Coverage::Absent {
            reason: "caller claims nothing exists".into(),
        },
    );
    std::fs::write(
        f.0.join("false-absence.json"),
        false_absence.encode(Limits::default()).unwrap(),
    )
    .unwrap();
    refusal(
        f.run(&["archive", "inspect", "false-absence.json"]),
        "artifact coverage differs",
    );
    let original_subject = archive.subject.clone();
    archive.subject = "war://wrong-warrant".into();
    std::fs::write(
        f.0.join("wrong-subject.json"),
        archive.encode(Limits::default()).unwrap(),
    )
    .unwrap();
    refusal(
        f.run(&["archive", "inspect", "wrong-subject.json"]),
        "archive subject differs",
    );
    archive.subject = original_subject;
    let ir = archive
        .records
        .iter_mut()
        .find(|r| r.path == "__ow_archive__/WAR.json")
        .unwrap();
    let changed = b"{}";
    ir.base64 = Some(base64_encode(changed));
    ir.digest = format!("sha256:{}", sha256_hex(changed));
    std::fs::write(
        f.0.join("changed.json"),
        archive.encode(Limits::default()).unwrap(),
    )
    .unwrap();
    refusal(
        f.run(&["archive", "inspect", "changed.json"]),
        "reconstructed IR differs",
    );
}

#[test]
fn legacy_reconnect_flag_cannot_claim_an_observed_import() {
    let f = Fixture::new();
    success(f.run(&[
        "init",
        "--namespace",
        "ARCH",
        "--program",
        "Archive fixture",
    ]));
    success(f.run(&["new", "Reject false round-trip claim"]));
    refusal(
        f.run(&["export", "ARCH-WAR-0001", "--round-trip", "--reconnect"]),
        "legacy round-trip cannot verify",
    );
}

#[test]
fn bounded_git_history_keeps_all_observed_state_bytes_and_commit_identity() {
    let f = Fixture::new();
    success(f.run(&[
        "init",
        "--namespace",
        "ARCH",
        "--program",
        "Archive history fixture",
    ]));
    success(f.run(&["new", "Retain changing historical records"]));
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(&f.0)
            .args([
                "-c",
                "user.name=Archive Fixture",
                "-c",
                "user.email=archive@example.invalid",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    };
    git(&["init", "-q"]);
    let record = "docs/warrants/ARCH-WAR-0001/state-history.json";
    let mut commits = Vec::new();
    for state in ["superseded", "disputed", "annulled"] {
        std::fs::write(
            f.0.join(record),
            format!("{{\"fixture_state\":\"{state}\"}}\n"),
        )
        .unwrap();
        git(&["add", "docs", "openwarrant.toml"]);
        git(&["commit", "-q", "-m", state]);
        commits.push((state, git(&["rev-parse", "HEAD"]).trim().to_owned()));
    }
    success(f.run(&[
        "archive",
        "export",
        "ARCH-WAR-0001",
        "history.json",
        "--history",
    ]));
    let archive = Archive::decode(
        &std::fs::read(f.0.join("history.json")).unwrap(),
        Limits::default(),
    )
    .unwrap();
    for (state, commit) in commits {
        let path = format!("__ow_archive__/history/{commit}/{record}");
        let record = archive
            .records
            .iter()
            .find(|r| r.path == path)
            .unwrap_or_else(|| {
                let index = archive
                    .records
                    .iter()
                    .find(|r| r.path == "__ow_archive__/artifacts.json")
                    .unwrap();
                panic!(
                    "missing {path}: {}",
                    String::from_utf8(
                        openwarrant_core::attestation::base64_decode(
                            index.base64.as_ref().unwrap()
                        )
                        .unwrap()
                    )
                    .unwrap()
                );
            });
        let bytes =
            openwarrant_core::attestation::base64_decode(record.base64.as_ref().unwrap()).unwrap();
        assert_eq!(
            bytes,
            format!("{{\"fixture_state\":\"{state}\"}}\n").as_bytes()
        );
        assert!(
            archive
                .records
                .iter()
                .any(|r| r.path == format!("__ow_archive__/history/{commit}/commit.txt"))
        );
    }
    let head = git(&["rev-parse", "HEAD"]);
    std::fs::write(f.0.join(".git/shallow"), head).unwrap();
    refusal(
        f.run(&[
            "archive",
            "export",
            "ARCH-WAR-0001",
            "shallow.json",
            "--history",
        ]),
        "shallow history",
    );
    assert!(!f.0.join("shallow.json").exists());
    std::fs::remove_file(f.0.join(".git/shallow")).unwrap();
    std::fs::rename(f.0.join(".git"), f.0.join("old-git-hidden")).unwrap();
    std::fs::rename(f.0.join("docs"), f.0.join("old-docs-hidden")).unwrap();
    success(f.run(&["archive", "inspect", "history.json"]));
}

#[test]
fn schema_pack_bytes_and_producer_identity_are_retained_and_cross_checked() {
    let f = Fixture::new();
    success(f.run(&[
        "init",
        "--namespace",
        "ARCH",
        "--program",
        "Schema preservation",
    ]));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pack_bytes = std::fs::read(root.join("schemas/pack.json")).unwrap();
    let pack: serde_json::Value = serde_json::from_slice(&pack_bytes).unwrap();
    std::fs::create_dir(f.0.join("schemas")).unwrap();
    std::fs::write(f.0.join("schemas/pack.json"), &pack_bytes).unwrap();
    for name in pack["files"].as_object().unwrap().keys() {
        let path = format!("schemas/oh.war/{name}/v1.json");
        std::fs::create_dir_all(f.0.join(&path).parent().unwrap()).unwrap();
        std::fs::copy(root.join(&path), f.0.join(&path)).unwrap();
    }
    success(f.run(&["archive", "export", "ARCH-WAR-0001", "snapshot.json"]));
    let bytes = std::fs::read(f.0.join("snapshot.json")).unwrap();
    let mut archive = Archive::decode(&bytes, Limits::default()).unwrap();
    assert!(
        matches!(&archive.coverage["schema and compiler identity"], Coverage::Retained { paths } if paths.len() == pack["files"].as_object().unwrap().len() + 2)
    );
    let producer = archive
        .records
        .iter()
        .find(|r| r.path == "__ow_archive__/producer.json")
        .unwrap();
    let producer: serde_json::Value = serde_json::from_slice(
        &openwarrant_core::attestation::base64_decode(producer.base64.as_ref().unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        producer["executable_sha256"],
        format!(
            "sha256:{}",
            sha256_hex(&std::fs::read(env!("CARGO_BIN_EXE_war")).unwrap())
        )
    );
    std::fs::rename(f.0.join("docs"), f.0.join("hidden-docs")).unwrap();
    std::fs::rename(f.0.join("schemas"), f.0.join("hidden-schemas")).unwrap();
    success(f.run(&["archive", "inspect", "snapshot.json"]));

    // Recomputing a record's digest cannot conceal inconsistency with retained pack.
    let member = archive
        .records
        .iter_mut()
        .find(|r| r.path == "schemas/oh.war/war/v1.json")
        .unwrap();
    member.base64 = Some(base64_encode(b"{}"));
    member.digest = format!("sha256:{}", sha256_hex(b"{}"));
    std::fs::write(
        f.0.join("changed.json"),
        archive.encode(Limits::default()).unwrap(),
    )
    .unwrap();
    refusal(
        f.run(&["archive", "inspect", "changed.json"]),
        "schema member digest mismatch",
    );
    std::fs::rename(f.0.join("hidden-docs"), f.0.join("docs")).unwrap();
    std::fs::rename(f.0.join("hidden-schemas"), f.0.join("schemas")).unwrap();
    std::fs::write(f.0.join("schemas/oh.war/war/v1.json"), b"{}").unwrap();
    refusal(
        f.run(&["archive", "export", "ARCH-WAR-0001", "bad.json"]),
        "schema member digest mismatch",
    );
    assert!(!f.0.join("bad.json").exists());
}

#[test]
fn declared_artifact_bytes_survive_source_loss_and_inventory_tampering_refuses() {
    let f = Fixture::new();
    success(f.run(&[
        "init",
        "--namespace",
        "ARCH",
        "--program",
        "Artifact preservation",
    ]));
    let content = [0, 255, 13, 10, 65];
    let digest = sha256_hex(&content);
    let declaration = artifact_declaration(&digest);
    let manifest = f.0.join("docs/warrants/ARCH-WAR-0001/deliverables.toml");
    std::fs::write(&manifest, &declaration).unwrap();
    std::fs::write(f.0.join("delivered.bin"), content).unwrap();
    success(f.run(&["archive", "export", "ARCH-WAR-0001", "artifact.json"]));
    let original = Archive::decode(
        &std::fs::read(f.0.join("artifact.json")).unwrap(),
        Limits::default(),
    )
    .unwrap();
    let address = format!("__ow_archive__/artifacts/{digest}");
    let record = original.records.iter().find(|r| r.path == address).unwrap();
    assert_eq!(
        openwarrant_core::attestation::base64_decode(record.base64.as_ref().unwrap()).unwrap(),
        content
    );
    std::fs::remove_file(f.0.join("delivered.bin")).unwrap();
    std::fs::rename(f.0.join("docs"), f.0.join("hidden-docs")).unwrap();
    success(f.run(&["archive", "inspect", "artifact.json"]));
    let mut forged = original.clone();
    forged.coverage.insert(
        "artifacts".into(),
        Coverage::Retained {
            paths: vec!["__ow_archive__/artifacts.json".into()],
        },
    );
    std::fs::write(
        f.0.join("false-coverage.json"),
        forged.encode(Limits::default()).unwrap(),
    )
    .unwrap();
    refusal(
        f.run(&["archive", "inspect", "false-coverage.json"]),
        "artifact coverage differs",
    );
    let mut changed = original.clone();
    let record = changed
        .records
        .iter_mut()
        .find(|r| r.path == address)
        .unwrap();
    record.base64 = Some(base64_encode(b"changed"));
    record.digest = format!("sha256:{}", sha256_hex(b"changed"));
    std::fs::write(
        f.0.join("changed-artifact.json"),
        changed.encode(Limits::default()).unwrap(),
    )
    .unwrap();
    refusal(
        f.run(&["archive", "inspect", "changed-artifact.json"]),
        "retained artifact digest mismatch",
    );
    let mut changed = original;
    let index = changed
        .records
        .iter_mut()
        .find(|r| r.path == "__ow_archive__/artifacts.json")
        .unwrap();
    let bytes = br#"{"schema":"oh.war/preservation-artifacts/v1-draft.1","claims":[]}"#;
    index.base64 = Some(base64_encode(bytes));
    index.digest = format!("sha256:{}", sha256_hex(bytes));
    std::fs::write(
        f.0.join("omitted-claim.json"),
        changed.encode(Limits::default()).unwrap(),
    )
    .unwrap();
    refusal(
        f.run(&["archive", "inspect", "omitted-claim.json"]),
        "artifact inventory omits declared records",
    );
    std::fs::rename(f.0.join("hidden-docs"), f.0.join("docs")).unwrap();
    std::fs::write(f.0.join("delivered.bin"), b"newer version").unwrap();
    success(f.run(&["archive", "export", "ARCH-WAR-0001", "unavailable.json"]));
    let archive = Archive::decode(
        &std::fs::read(f.0.join("unavailable.json")).unwrap(),
        Limits::default(),
    )
    .unwrap();
    assert!(!archive.records.iter().any(|r| r.path == address));
    let index = archive
        .records
        .iter()
        .find(|r| r.path == "__ow_archive__/artifacts.json")
        .unwrap();
    let index: serde_json::Value = serde_json::from_slice(
        &openwarrant_core::attestation::base64_decode(index.base64.as_ref().unwrap()).unwrap(),
    )
    .unwrap();
    assert!(
        index["claims"][0]["unavailable"]
            .as_str()
            .unwrap()
            .contains("differs")
    );
    std::fs::write(
        manifest,
        declaration.replace("kind = \"file\"", "kind = \"unknown-required-kind\""),
    )
    .unwrap();
    refusal(
        f.run(&["archive", "export", "ARCH-WAR-0001", "unknown.json"]),
        "unknown variant `unknown-required-kind`",
    );
}

fn artifact_declaration(digest: &str) -> String {
    format!(
        r#"schema = "oh.war/deliverables/v1"
[[deliverable]]
id = "D-001"
title = "Retained binary"
kind = "file"
target_ref = "delivered.bin"
[deliverable.provenance]
producer = "fixture"
producing_attempt = "fixture"
contract_digest = "unrecorded"
tool_or_runtime_identity = "fixture"
creation_method = "fixture"
content_digest = "sha256:{digest}"
media_type = "application/octet-stream"
classification = "internal"
retention = "fixture"
source_holder = "git"
"#
    )
}

#[test]
fn historical_artifact_versions_are_selected_by_digest_not_current_path() {
    let f = Fixture::new();
    success(f.run(&[
        "init",
        "--namespace",
        "ARCH",
        "--program",
        "Artifact history",
    ]));
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args([
                "-c",
                "user.name=Archive Fixture",
                "-c",
                "user.email=archive@example.invalid",
            ])
            .args(args)
            .current_dir(&f.0)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "-q"]);
    let declaration = f.0.join("docs/warrants/ARCH-WAR-0001/deliverables.toml");
    let first = b"original\0artifact";
    // NUL prevents Git text normalization from discarding the fixture CRLF bytes.
    let second = b"revised\0\r\nartifact";
    for (bytes, message) in [(first.as_slice(), "first"), (second.as_slice(), "second")] {
        std::fs::write(f.0.join("delivered.bin"), bytes).unwrap();
        std::fs::write(&declaration, artifact_declaration(&sha256_hex(bytes))).unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", message]);
    }
    let head = git(&["rev-parse", "HEAD"]);
    std::fs::write(f.0.join("delivered.bin"), b"uncommitted third version").unwrap();
    success(f.run(&[
        "archive",
        "export",
        "ARCH-WAR-0001",
        "history.json",
        "--history",
    ]));
    let redirected = Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(&f.0)
        .args([
            "archive",
            "export",
            "ARCH-WAR-0001",
            "ambient-git.json",
            "--history",
        ])
        .env("GIT_DIR", f.0.join("not-the-selected-repository"))
        .env("GIT_WORK_TREE", f.0.join("wrong-worktree"))
        .env("GIT_COMMON_DIR", f.0.join("wrong-common-dir"))
        .env("GIT_OBJECT_DIRECTORY", f.0.join("wrong-objects"))
        .env(
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            f.0.join("wrong-alternates"),
        )
        .env("GIT_INDEX_FILE", f.0.join("wrong-index"))
        .env("GIT_SHALLOW_FILE", f.0.join("wrong-shallow"))
        .output()
        .unwrap();
    success(redirected);
    assert_eq!(
        std::fs::read(f.0.join("history.json")).unwrap(),
        std::fs::read(f.0.join("ambient-git.json")).unwrap()
    );
    let archive = Archive::decode(
        &std::fs::read(f.0.join("history.json")).unwrap(),
        Limits::default(),
    )
    .unwrap();
    assert!(
        matches!(&archive.coverage["artifacts"], Coverage::Retained { paths } if paths.iter().any(|p| p.starts_with("__ow_archive__/artifacts/")))
    );

    for bytes in [first.as_slice(), second.as_slice()] {
        let path = format!("__ow_archive__/artifacts/{}", sha256_hex(bytes));
        let record = archive
            .records
            .iter()
            .find(|r| r.path == path)
            .unwrap_or_else(|| {
                let index = archive
                    .records
                    .iter()
                    .find(|r| r.path == "__ow_archive__/artifacts.json")
                    .unwrap();
                panic!(
                    "missing {path}: {}",
                    String::from_utf8(
                        openwarrant_core::attestation::base64_decode(
                            index.base64.as_ref().unwrap()
                        )
                        .unwrap()
                    )
                    .unwrap()
                );
            });
        assert_eq!(
            openwarrant_core::attestation::base64_decode(record.base64.as_ref().unwrap()).unwrap(),
            bytes
        );
    }
    let index = archive
        .records
        .iter()
        .find(|r| r.path == "__ow_archive__/artifacts.json")
        .unwrap();
    let index: serde_json::Value = serde_json::from_slice(
        &openwarrant_core::attestation::base64_decode(index.base64.as_ref().unwrap()).unwrap(),
    )
    .unwrap();
    assert!(
        index["claims"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["unavailable"].is_null())
    );
    let origins: Vec<_> = index["claims"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| !c["git_source"].is_null())
        .collect();
    assert_eq!(origins.len(), 2);
    assert!(origins.iter().all(|c| c["git_source"]["head"] == head));
    std::fs::remove_file(f.0.join("delivered.bin")).unwrap();
    success(f.run(&[
        "archive",
        "export",
        "ARCH-WAR-0001",
        "deleted-path.json",
        "--history",
    ]));
    assert_eq!(
        std::fs::read(f.0.join("history.json")).unwrap(),
        std::fs::read(f.0.join("deleted-path.json")).unwrap()
    );
    let mut changed = archive.clone();
    let record = changed
        .records
        .iter_mut()
        .find(|r| r.path == "__ow_archive__/artifacts.json")
        .unwrap();
    let mut index: serde_json::Value = serde_json::from_slice(
        &openwarrant_core::attestation::base64_decode(record.base64.as_ref().unwrap()).unwrap(),
    )
    .unwrap();
    let claim = index["claims"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| !c["git_source"].is_null())
        .unwrap();
    claim["git_source"]["head"] = "0000000000000000000000000000000000000000".into();
    let bytes = openwarrant_compiler::to_canonical_bytes(&index).unwrap();
    record.digest = format!("sha256:{}", sha256_hex(&bytes));
    record.base64 = Some(base64_encode(&bytes));
    std::fs::write(
        f.0.join("changed-origin.json"),
        changed.encode(Limits::default()).unwrap(),
    )
    .unwrap();
    refusal(
        f.run(&["archive", "inspect", "changed-origin.json"]),
        "artifact Git origin differs",
    );
    std::fs::remove_dir_all(f.0.join(".git")).unwrap();
    std::fs::remove_dir_all(f.0.join("docs")).unwrap();
    success(f.run(&["archive", "inspect", "history.json"]));
}

#[test]
fn explicit_external_atoms_survive_source_removal_without_repository_escape() {
    let f = Fixture::new();
    success(f.run(&[
        "init",
        "--namespace",
        "ARCH",
        "--program",
        "External atom fixture",
    ]));
    success(f.run(&["new", "Preserve explicit shared atoms"]));
    let manifest_path = f.0.join("docs/warrants/ARCH-WAR-0001/manifest.toml");
    let original = std::fs::read_to_string(&manifest_path).unwrap();
    let shared = f.0.join("docs/shared");
    std::fs::create_dir(&shared).unwrap();
    let bytes = std::fs::read(f.0.join("docs/warrants/ARCH-WAR-0001/atoms/10-intent.md")).unwrap();
    std::fs::write(shared.join("intent.md"), &bytes).unwrap();
    let amended = original.replace("atoms/10-intent.md", "../../shared/intent.md");
    std::fs::write(&manifest_path, &amended).unwrap();
    success(f.run(&["archive", "export", "ARCH-WAR-0001", "shared.json"]));
    let mut archive = Archive::decode(
        &std::fs::read(f.0.join("shared.json")).unwrap(),
        Limits::default(),
    )
    .unwrap();
    assert!(
        archive
            .records
            .iter()
            .any(|r| r.path == "docs/shared/intent.md")
    );
    std::fs::rename(f.0.join("docs"), f.0.join("hidden-docs")).unwrap();
    success(f.run(&["archive", "inspect", "shared.json"]));
    // A descriptor must preserve the manifest's source-to-record binding even
    // if a different archive record happens to contain identical bytes.
    let basis = archive
        .records
        .iter_mut()
        .find(|r| r.path == "__ow_archive__/basis.json")
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(
        &openwarrant_core::attestation::base64_decode(basis.base64.as_ref().unwrap()).unwrap(),
    )
    .unwrap();
    value["atoms"][0]["record"] = "docs/warrants/ARCH-WAR-0001/atoms/10-intent.md".into();
    let changed = openwarrant_compiler::to_canonical_bytes(&value).unwrap();
    basis.digest = format!("sha256:{}", sha256_hex(&changed));
    basis.base64 = Some(base64_encode(&changed));
    std::fs::write(
        f.0.join("substituted.json"),
        archive.encode(Limits::default()).unwrap(),
    )
    .unwrap();
    refusal(
        f.run(&["archive", "inspect", "substituted.json"]),
        "basis atom differs",
    );
    std::fs::rename(f.0.join("hidden-docs"), f.0.join("docs")).unwrap();
    for bad in ["../../../../outside.md", "../../shared/../shared/intent.md"] {
        std::fs::write(&manifest_path, original.replace("atoms/10-intent.md", bad)).unwrap();
        let output = f.run(&["archive", "export", "ARCH-WAR-0001", "refused.json"]);
        assert!(!output.status.success());
        assert!(!f.0.join("refused.json").exists());
    }
    #[cfg(unix)]
    {
        std::fs::write(&manifest_path, &amended).unwrap();
        std::fs::rename(&shared, f.0.join("real-shared")).unwrap();
        std::os::unix::fs::symlink(f.0.join("real-shared"), &shared).unwrap();
        assert!(
            !f.run(&["archive", "export", "ARCH-WAR-0001", "symlink.json"])
                .status
                .success()
        );
        assert!(!f.0.join("symlink.json").exists());
    }
}

#[test]
fn historical_shared_atoms_use_each_manifest_commit_not_current_bytes() {
    let f = Fixture::new();
    success(f.run(&[
        "init",
        "--namespace",
        "ARCH",
        "--program",
        "Historical shared atom",
    ]));
    success(f.run(&["new", "Retain historical shared source"]));
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .current_dir(&f.0)
            .args([
                "-c",
                "user.name=Archive Fixture",
                "-c",
                "user.email=archive@example.invalid",
            ])
            .args(args)
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
    let manifest_path = f.0.join("docs/warrants/ARCH-WAR-0001/manifest.toml");
    let manifest = std::fs::read_to_string(&manifest_path)
        .unwrap()
        .replace("atoms/10-intent.md", "../../shared/intent.md");
    let source =
        std::fs::read_to_string(f.0.join("docs/warrants/ARCH-WAR-0001/atoms/10-intent.md"))
            .unwrap();
    std::fs::create_dir(f.0.join("docs/shared")).unwrap();
    let shared = f.0.join("docs/shared/intent.md");
    let mut revisions = Vec::new();
    for name in ["first", "second"] {
        let bytes = format!("{source}\nHistorical body: {name}\n");
        std::fs::write(&shared, &bytes).unwrap();
        std::fs::write(&manifest_path, format!("{manifest}\n# revision {name}\n")).unwrap();
        git(&["add", "docs", "openwarrant.toml"]);
        git(&["commit", "-q", "-m", name]);
        revisions.push((git(&["rev-parse", "HEAD"]), bytes));
        let shared_only = format!("{source}\nShared-only revision after {name}\n");
        std::fs::write(&shared, &shared_only).unwrap();
        git(&["add", "docs/shared/intent.md"]);
        git(&["commit", "-q", "-m", "shared atom only"]);
        revisions.push((git(&["rev-parse", "HEAD"]), shared_only));
    }
    std::fs::write(&shared, format!("{source}\nUncommitted third body\n")).unwrap();
    success(f.run(&[
        "archive",
        "export",
        "ARCH-WAR-0001",
        "history.json",
        "--history",
    ]));
    let archive = Archive::decode(
        &std::fs::read(f.0.join("history.json")).unwrap(),
        Limits::default(),
    )
    .unwrap();
    let query = f.run(&["archive", "runtime-basis", "history.json", "--json"]);
    assert!(
        query.status.success(),
        "{}",
        String::from_utf8_lossy(&query.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&query.stdout).unwrap();
    let inventory = &report["result"]["stage_inventory"];
    assert_eq!(inventory["history_retained"], true);
    assert_eq!(inventory["execution_coverage_established"], false);
    let declarations = inventory["declarations"].as_array().unwrap();
    assert!(
        declarations
            .iter()
            .any(|d| d["source"].as_str().unwrap().starts_with("docs/"))
    );
    for (commit, _) in &revisions {
        let prefix = format!("__ow_archive__/history/{commit}/");
        assert!(
            declarations
                .iter()
                .any(|d| d["source"].as_str().unwrap().starts_with(&prefix))
        );
    }
    for declaration in declarations {
        let record = archive
            .records
            .iter()
            .find(|r| r.path == declaration["source"].as_str().unwrap())
            .unwrap();
        assert_eq!(declaration["source_digest"], record.digest);
    }
    // Rehashing modified index bytes cannot hide omitted or duplicate claims.
    for mutation in ["duplicate", "omit", "redirect"] {
        let mut changed = archive.clone();
        let index = changed
            .records
            .iter_mut()
            .find(|r| r.path == "__ow_archive__/history.json")
            .unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(
            &openwarrant_core::attestation::base64_decode(index.base64.as_ref().unwrap()).unwrap(),
        )
        .unwrap();
        match mutation {
            "duplicate" => {
                let copy = value["commits"][0].clone();
                value["commits"].as_array_mut().unwrap().push(copy);
            }
            "omit" => {
                value["commits"][0]["files"].as_array_mut().unwrap().pop();
            }
            _ => {
                value["commits"][0]["files"][0]["source"] = "different-source.md".into();
            }
        }
        let bytes = openwarrant_compiler::to_canonical_bytes(&value).unwrap();
        index.digest = format!("sha256:{}", sha256_hex(&bytes));
        index.base64 = Some(base64_encode(&bytes));
        std::fs::write(
            f.0.join("bad-index.json"),
            changed.encode(Limits::default()).unwrap(),
        )
        .unwrap();
        assert!(
            !f.run(&["archive", "inspect", "bad-index.json"])
                .status
                .success(),
            "accepted {mutation}"
        );
    }
    for (commit, expected) in revisions {
        let path = format!("__ow_archive__/history/{commit}/docs/shared/intent.md");
        let record = archive.records.iter().find(|r| r.path == path).unwrap();
        assert_eq!(
            openwarrant_core::attestation::base64_decode(record.base64.as_ref().unwrap()).unwrap(),
            expected.as_bytes()
        );
    }
    std::fs::rename(f.0.join("docs"), f.0.join("hidden-docs")).unwrap();
    std::fs::rename(f.0.join(".git"), f.0.join("hidden-git")).unwrap();
    success(f.run(&["archive", "inspect", "history.json"]));
    std::fs::rename(f.0.join("hidden-docs"), f.0.join("docs")).unwrap();
    std::fs::rename(f.0.join("hidden-git"), f.0.join(".git")).unwrap();
    // A historical reference to missing bytes cannot be silently replaced with
    // today's uncommitted file, even though current compilation succeeds.
    git(&["rm", "-f", "--", "docs/shared/intent.md"]);
    std::fs::write(
        &manifest_path,
        format!("{manifest}\n# missing shared source\n"),
    )
    .unwrap();
    git(&["add", "docs"]);
    git(&["commit", "-q", "-m", "missing historical source"]);
    std::fs::create_dir_all(shared.parent().unwrap()).unwrap();
    std::fs::write(&shared, &source).unwrap();
    refusal(
        f.run(&[
            "archive",
            "export",
            "ARCH-WAR-0001",
            "missing.json",
            "--history",
        ]),
        "historical atom source missing",
    );
    assert!(!f.0.join("missing.json").exists());
}

#[test]
fn explicit_additional_history_root_preserves_unmerged_records() {
    let f = Fixture::new();
    success(f.run(&[
        "init",
        "--namespace",
        "ARCH",
        "--program",
        "Additional history root",
    ]));
    success(f.run(&["new", "Retain unmerged evidence"]));
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .current_dir(&f.0)
            .args([
                "-c",
                "user.name=Archive Fixture",
                "-c",
                "user.email=archive@example.invalid",
            ])
            .args(args)
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
    git(&["add", "docs", "openwarrant.toml"]);
    git(&["commit", "-qm", "base"]);
    let base = git(&["rev-parse", "HEAD"]);
    git(&["checkout", "-qb", "retained-evidence"]);
    let source = "docs/warrants/ARCH-WAR-0001/retained.txt";
    std::fs::write(f.0.join(source), b"unmerged retained evidence").unwrap();
    git(&["add", "docs"]);
    git(&["commit", "-qm", "unmerged evidence"]);
    let extra = git(&["rev-parse", "HEAD"]);
    git(&["checkout", "-q", "--detach", &base]);
    success(f.run(&[
        "archive",
        "export",
        "ARCH-WAR-0001",
        "head-only.json",
        "--history",
    ]));
    let read = |name: &str| {
        Archive::decode(&std::fs::read(f.0.join(name)).unwrap(), Limits::default()).unwrap()
    };
    let target = format!("__ow_archive__/history/{extra}/{source}");
    assert!(
        !read("head-only.json")
            .records
            .iter()
            .any(|r| r.path == target)
    );
    success(f.run(&[
        "archive",
        "export",
        "ARCH-WAR-0001",
        "extra.json",
        "--history",
        "--history-ref",
        "retained-evidence",
    ]));
    let archive = read("extra.json");
    assert!(archive.records.iter().any(|r| r.path == target));
    let history = archive
        .records
        .iter()
        .find(|r| r.path == "__ow_archive__/history.json")
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(
        &openwarrant_core::attestation::base64_decode(history.base64.as_ref().unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(value["head"], base);
    assert_eq!(value["additional_heads"], serde_json::json!([extra]));
    assert_eq!(value["other_refs_included"], true);
    refusal(
        f.run(&[
            "archive",
            "export",
            "ARCH-WAR-0001",
            "bad-root.json",
            "--history",
            "--history-ref",
            "missing-root",
        ]),
        "required local Git history unavailable",
    );
    assert!(!f.0.join("bad-root.json").exists());
    std::fs::rename(f.0.join(".git"), f.0.join("hidden-git")).unwrap();
    std::fs::rename(f.0.join("docs"), f.0.join("hidden-docs")).unwrap();
    success(f.run(&["archive", "inspect", "extra.json"]));
}

#[test]
fn generated_local_archive_roundtrips_and_runtime_absence_cannot_be_forged() {
    let f = Fixture::new();
    success(f.run(&[
        "init",
        "--namespace",
        "ARCH",
        "--program",
        "Complete local transport fixture",
    ]));
    success(f.run(&["new", "Retain complete declared local sources"]));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pack = std::fs::read(root.join("schemas/pack.json")).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&pack).unwrap();
    std::fs::create_dir_all(f.0.join("schemas")).unwrap();
    std::fs::write(f.0.join("schemas/pack.json"), pack).unwrap();
    for name in value["files"].as_object().unwrap().keys() {
        let path = format!("schemas/oh.war/{name}/v1.json");
        std::fs::create_dir_all(f.0.join(&path).parent().unwrap()).unwrap();
        std::fs::copy(root.join(&path), f.0.join(&path)).unwrap();
    }
    for args in [
        vec!["init", "-q"],
        vec!["add", "docs", "schemas", "openwarrant.toml"],
        vec!["commit", "-qm", "fixture source"],
    ] {
        let out = Command::new("git")
            .current_dir(&f.0)
            .args([
                "-c",
                "user.name=Archive Fixture",
                "-c",
                "user.email=archive@example.invalid",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    success(f.run(&[
        "archive",
        "export",
        "ARCH-WAR-0002",
        "complete.json",
        "--history",
    ]));
    let original = std::fs::read(f.0.join("complete.json")).unwrap();
    let archive = Archive::decode(&original, Limits::default()).unwrap();
    assert!(
        archive
            .coverage
            .values()
            .all(|value| !matches!(value, Coverage::Unavailable { .. })),
        "{:?}",
        archive.coverage
    );
    let milestones =
        f.0.join("docs/warrants/ARCH-WAR-0002/atoms/45-milestones.yaml");
    let before = std::fs::read_to_string(&milestones).unwrap();
    let after = before.replace("executor_kind: \"human\"", "executor_kind: \"katana\"");
    assert_ne!(before, after);
    std::fs::write(&milestones, after).unwrap();
    success(f.run(&[
        "archive",
        "export",
        "ARCH-WAR-0002",
        "runtime.json",
        "--history",
    ]));
    let mut runtime = Archive::decode(
        &std::fs::read(f.0.join("runtime.json")).unwrap(),
        Limits::default(),
    )
    .unwrap();
    assert!(matches!(
        runtime.coverage["runtime receipt refs"],
        Coverage::Unavailable { .. }
    ));
    runtime.coverage.insert(
        "runtime receipt refs".into(),
        archive.coverage["runtime receipt refs"].clone(),
    );
    std::fs::write(
        f.0.join("false-runtime.json"),
        runtime.encode(Limits::default()).unwrap(),
    )
    .unwrap();
    refusal(
        f.run(&["archive", "inspect", "false-runtime.json"]),
        "runtime receipt refs coverage differs",
    );
    for path in ["docs", "schemas", ".git"] {
        std::fs::rename(f.0.join(path), f.0.join(format!("hidden-{path}"))).unwrap();
    }
    success(f.run(&["archive", "import", "complete.json", "imported"]));
    std::fs::remove_file(f.0.join("complete.json")).unwrap();
    success(f.run(&["archive", "reexport", "imported", "again.json"]));
    assert_eq!(std::fs::read(f.0.join("again.json")).unwrap(), original);
}

#[test]
fn warrant_subject_cannot_downgrade_to_unchecked_transport() {
    let f = Fixture::new();
    let bytes = f.archive(false);
    success(f.run(&["archive", "import", "source.json", "generic"]));
    let mut archive = Archive::decode(&bytes, Limits::default()).unwrap();
    archive.subject = "war://01a0b53a-3315-7ed3-a369-35af9154f89d".into();
    let changed = archive.encode(Limits::default()).unwrap();
    std::fs::write(f.0.join("warrant.json"), &changed).unwrap();
    refusal(
        f.run(&["archive", "import", "warrant.json", "unchecked"]),
        "missing basis descriptor",
    );
    assert!(!f.0.join("unchecked").exists());
    std::fs::write(f.0.join("generic/ARCHIVE.json"), changed).unwrap();
    refusal(
        f.run(&["archive", "reexport", "generic", "unchecked.json"]),
        "missing basis descriptor",
    );
    assert!(!f.0.join("unchecked.json").exists());
}

#[test]
fn runtime_query_basis_comes_from_reconstructed_sources_not_caller_identity() {
    let f = Fixture::new();
    success(f.run(&[
        "init",
        "--namespace",
        "BASIS",
        "--program",
        "Runtime basis fixture",
    ]));
    success(f.run(&["new", "Retain a runtime query basis"]));
    success(f.run(&["archive", "export", "BASIS-WAR-0001", "snapshot.json"]));
    let bytes = std::fs::read(f.0.join("snapshot.json")).unwrap();
    let mut archive = Archive::decode(&bytes, Limits::default()).unwrap();
    std::fs::rename(f.0.join("docs"), f.0.join("source-hidden")).unwrap();
    let out = f.run(&["archive", "runtime-basis", "snapshot.json", "--json"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let basis = &report["result"];
    assert_eq!(basis["schema"], "oh.war/runtime-archive-basis/v1-draft.1");
    assert_eq!(basis["subject"], archive.subject);
    assert_eq!(
        basis["archive_digest"],
        archive.digest(Limits::default()).unwrap()
    );
    let ir_record = archive
        .records
        .iter()
        .find(|r| r.path == "__ow_archive__/WAR.json")
        .unwrap();
    let ir: openwarrant_compiler::WarIr = serde_json::from_slice(
        &openwarrant_core::attestation::base64_decode(ir_record.base64.as_ref().unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        basis["current_contract"]["digest"],
        ir.contract_digest().unwrap()
    );
    assert_eq!(basis["warrant_id"], ir.identity.uuid);
    assert_eq!(basis["current_contract"]["revision"], 1);
    assert_eq!(
        basis["current_contract"]["digest"].as_str().unwrap().len(),
        64
    );
    assert_eq!(basis["contract_history_coverage"]["state"], "unavailable");
    let stages = &basis["stage_inventory"];
    assert_eq!(stages["history_retained"], false);
    assert_eq!(stages["execution_coverage_established"], false);
    let declarations = stages["declarations"].as_array().unwrap();
    assert!(!declarations.is_empty());
    for declaration in declarations {
        let record = archive
            .records
            .iter()
            .find(|r| r.path == declaration["source"].as_str().unwrap())
            .unwrap();
        let source =
            openwarrant_core::attestation::base64_decode(record.base64.as_ref().unwrap()).unwrap();
        assert_eq!(
            declaration["source_digest"],
            format!("sha256:{}", sha256_hex(&source))
        );
        let graph =
            openwarrant_core::milestones::parse(std::str::from_utf8(&source).unwrap()).unwrap();
        assert_eq!(declaration["graph"], serde_json::to_value(graph).unwrap());
    }
    assert_eq!(basis["authority_activated"], false);
    assert_eq!(basis["qualified"], false);
    assert_eq!(std::fs::read(f.0.join("snapshot.json")).unwrap(), bytes);
    archive.subject = "war://different-warrant".into();
    std::fs::write(
        f.0.join("forged.json"),
        archive.encode(Limits::default()).unwrap(),
    )
    .unwrap();
    refusal(
        f.run(&["archive", "runtime-basis", "forged.json"]),
        "archive subject differs",
    );
}
