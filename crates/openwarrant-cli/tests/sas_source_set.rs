// SPDX-License-Identifier: Apache-2.0
//! Public source-set proposal, acceptance, and status contracts (0074 AM-001).

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Repo(PathBuf);

impl Repo {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "ow-sas-source-set-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let repo = Self(root);
        repo.ok(&["init", "--namespace", "TEST"]);
        repo.write(
            "candidate/main.md",
            "# SAS\n\n## 106. Requirements\n\n| TEST-SAS-RQ-001 | Preserve evidence. |\n",
        );
        repo.write("candidate/rules.md", "Binding companion.\n");
        repo.write("candidate/schema.json", "{}\n");
        repo.write("candidate/example.txt", "Reference example.\n");
        repo.write("decision.md", "Adopt this exact source set.\n");
        repo.manifest();
        repo
    }

    fn write(&self, path: &str, body: &str) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    fn manifest(&self) {
        let files: Vec<_> = [
            ("main.md", "normative"),
            ("rules.md", "normative"),
            ("schema.json", "normative"),
            ("example.txt", "reference"),
        ]
        .iter()
        .map(|(path, role)| {
            let bytes = fs::read(self.0.join("candidate").join(path)).unwrap();
            json!({"path": path, "role": role, "bytes": bytes.len(),
                "sha256": format!("sha256:{}", Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect::<String>())})
        })
        .collect();
        self.write(
            "candidate/source-set.json",
            &json!({
                "schema": "oh.war/spec-source-set/1.0.0-rc.2",
                "edition": "1.0.0-rc.2", "status": "candidate-unaccepted",
                "files": files
            })
            .to_string(),
        );
    }

    fn run(&self, args: &[&str]) -> (i32, Value) {
        let out = Command::new(env!("CARGO_BIN_EXE_war"))
            .arg("--json")
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap();
        let code = out.status.code().unwrap();
        let value = serde_json::from_slice(&out.stdout).unwrap_or_else(|_| json!({
            "stdout": String::from_utf8_lossy(&out.stdout), "stderr": String::from_utf8_lossy(&out.stderr)
        }));
        (code, value)
    }

    fn ok(&self, args: &[&str]) -> Value {
        let (code, value) = self.run(args);
        assert_eq!(code, 0, "{args:?}: {value}");
        value
    }

    fn propose(&self) -> Value {
        self.ok(&[
            "sas",
            "propose",
            "1.0.0-rc.2",
            "--source",
            "candidate/main.md",
            "--source-set",
            "candidate/source-set.json",
            "--adr",
            "decision.md",
        ])
    }

    fn signer(&self) {
        self.write(
            "docs/authority/roles.toml",
            r#"
schema = "oh.war/authority-register/v1"
[[assignment]]
actor = "Fixture Human"
actor_kind = "human"
roles = ["authorizer"]
assigned_by = "Fixture Human"
effective_time = "2026-01-01T00:00:00Z"
ssh_principal = "fixture"
[[assignment]]
actor = "Fixture Agent"
actor_kind = "agent"
roles = ["authorizer"]
assigned_by = "Fixture Human"
effective_time = "2026-01-01T00:00:00Z"
"#,
        );
        let out = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(self.0.join("fixture-key"))
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
        let public = fs::read_to_string(self.0.join("fixture-key.pub")).unwrap();
        self.write(
            "docs/authority/allowed_signers",
            &format!("fixture {public}"),
        );
    }

    fn response(&self, request: &Value, actor: &str, schema: &str, signed: bool) {
        let body = format!(
            r#"schema = "{schema}"
version = "{version}"
sha256 = "{}"
accepted_by = "{actor}"
acting_role = "authorizer"
meaning = "Accept the exact fictional fixture subject."
effective_time = "2026-09-14T08:00:00Z"
adr_ref = "decision.md"
"#,
            request["result"]["sha256"].as_str().unwrap(),
            version = request["result"]["version"].as_str().unwrap()
        );
        self.write("response.toml", &body);
        if signed {
            let out = Command::new("ssh-keygen")
                .args(["-Y", "sign", "-n", "oh.war/response", "-f"])
                .arg(self.0.join("fixture-key"))
                .arg(self.0.join("response.toml"))
                .env_remove("SSH_AUTH_SOCK")
                .output()
                .unwrap();
            assert!(out.status.success(), "{out:?}");
        }
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn proposal_captures_the_complete_subject_and_refuses_overwrite() {
    let repo = Repo::new();
    repo.propose();
    let request = repo.ok(&["sas", "accept", "1.0.0-rc.2"]);
    let r = &request["result"];
    assert_eq!(r["schema"], "oh.war/sas-acceptance-request/v2");
    assert_eq!(r["source_set"]["main"]["path"], "candidate/main.md");
    assert_eq!(
        r["source_set"]["manifest"]["path"],
        "candidate/source-set.json"
    );
    assert_eq!(r["source_set"]["decision"]["path"], "decision.md");
    assert_ne!(r["sha256"], r["source_set"]["main"]["sha256"]);
    let (code, refused) = repo.run(&[
        "sas",
        "propose",
        "1.0.0-rc.2",
        "--source",
        "candidate/main.md",
        "--source-set",
        "candidate/source-set.json",
        "--adr",
        "decision.md",
    ]);
    assert_ne!(code, 0);
    assert!(refused.to_string().contains("already exists"), "{refused}");
    assert_eq!(request, repo.ok(&["sas", "accept", "1.0.0-rc.2"]));
}

#[test]
fn human_accepts_exact_capture_and_later_live_edits_preserve_history() {
    let repo = Repo::new();
    repo.signer();
    repo.propose();
    let request = repo.ok(&["sas", "accept", "1.0.0-rc.2"]);
    let preview = repo.ok(&["sign", "1.0.0-rc.2", "--show"]);
    assert!(preview.to_string().contains("sign.shown"));
    // Preview does not accept or change the request.
    assert_eq!(request, repo.ok(&["sas", "accept", "1.0.0-rc.2"]));
    repo.response(
        &request,
        "Fixture Human",
        "oh.war/sas-acceptance-response/v2",
        true,
    );
    let accepted = repo.ok(&["sas", "accept", "1.0.0-rc.2", "--response", "response.toml"]);
    assert!(accepted.to_string().contains("sas.accepted"));
    repo.write("candidate/rules.md", "Changed after acceptance.\n");
    let status = repo.ok(&["sas", "status"]);
    assert!(
        status.to_string().contains("sas.capture-intact"),
        "{status}"
    );
    assert!(
        status.to_string().contains("sas.current-differs"),
        "{status}"
    );
}

#[test]
fn warrant_checks_use_the_complete_captured_subject() {
    let repo = Repo::new();
    repo.propose();
    repo.ok(&["new", "Fixture outcome"]);
    let check = repo.ok(&["check", "TEST-WAR-0001"]);
    assert!(check.to_string().contains("sas.capture-intact"), "{check}");
}

#[test]
fn every_changed_input_refuses_acceptance_without_changing_the_proposal() {
    for path in [
        "candidate/main.md",
        "candidate/rules.md",
        "candidate/schema.json",
        "candidate/example.txt",
        "candidate/source-set.json",
        "decision.md",
    ] {
        let repo = Repo::new();
        repo.signer();
        repo.propose();
        let request = repo.ok(&["sas", "accept", "1.0.0-rc.2"]);
        repo.response(
            &request,
            "Fixture Human",
            "oh.war/sas-acceptance-response/v2",
            true,
        );
        let record_path = repo.0.join("docs/sas/revisions/1.0.0-rc.2.toml");
        let before = fs::read(&record_path).unwrap();
        repo.write(path, "Changed.\n");
        let (code, refused) =
            repo.run(&["sas", "accept", "1.0.0-rc.2", "--response", "response.toml"]);
        assert_ne!(code, 0, "{path}: {refused}");
        assert!(
            refused.to_string().contains("sas.stale-source-set"),
            "{path}: {refused}"
        );
        assert_eq!(before, fs::read(record_path).unwrap());
    }
}

#[test]
fn forged_unsigned_and_legacy_responses_cannot_accept_v2() {
    for (actor, schema, signed, reason) in [
        (
            "Fixture Agent",
            "oh.war/sas-acceptance-response/v2",
            false,
            "sas.not-permitted",
        ),
        (
            "Fixture Human",
            "oh.war/sas-acceptance-response/v2",
            false,
            "sas.response-signature",
        ),
        (
            "Fixture Human",
            "oh.war/sas-acceptance-response/v1",
            true,
            "sas.response-schema",
        ),
    ] {
        let repo = Repo::new();
        repo.signer();
        repo.propose();
        let request = repo.ok(&["sas", "accept", "1.0.0-rc.2"]);
        repo.response(&request, actor, schema, signed);
        let (code, refused) =
            repo.run(&["sas", "accept", "1.0.0-rc.2", "--response", "response.toml"]);
        assert_ne!(code, 0);
        assert!(refused.to_string().contains(reason), "{refused}");
        assert_eq!(request, repo.ok(&["sas", "accept", "1.0.0-rc.2"]));
    }
}

#[test]
fn invalid_manifests_publish_no_revision() {
    for (case, reason) in [
        ("duplicate", "duplicate member path"),
        ("traversal", "unsafe path"),
        ("absolute", "unsafe path"),
        ("role", "main source is not normative"),
        ("edition", "edition or status mismatch"),
        ("schema", "manifest schema"),
        ("oversized", "oversized member"),
        ("digest", "member length/digest mismatch"),
        ("missing", "unsafe or missing file"),
        ("main", "main source is not a manifest member"),
    ] {
        let repo = Repo::new();
        let manifest_path = repo.0.join("candidate/source-set.json");
        let mut manifest: Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        match case {
            "duplicate" => {
                let entry = manifest["files"][0].clone();
                manifest["files"].as_array_mut().unwrap().push(entry);
            }
            "traversal" => manifest["files"][0]["path"] = json!("../main.md"),
            "absolute" => manifest["files"][0]["path"] = json!("/tmp/main.md"),
            "role" => manifest["files"][0]["role"] = json!("reference"),
            "edition" => manifest["edition"] = json!("different"),
            "schema" => manifest["schema"] = json!("unrecognized"),
            "oversized" => manifest["files"][0]["bytes"] = json!(8 * 1024 * 1024 + 1),
            "digest" => {
                manifest["files"][0]["sha256"] = json!(format!("sha256:{}", "0".repeat(64)))
            }
            "missing" => {
                fs::remove_file(repo.0.join("candidate/rules.md")).unwrap();
            }
            "main" => {
                manifest["files"].as_array_mut().unwrap().remove(0);
            }
            _ => unreachable!(),
        }
        fs::write(manifest_path, manifest.to_string()).unwrap();
        let (code, refused) = repo.run(&[
            "sas",
            "propose",
            "1.0.0-rc.2",
            "--source",
            "candidate/main.md",
            "--source-set",
            "candidate/source-set.json",
            "--adr",
            "decision.md",
        ]);
        assert_ne!(code, 0, "{case}: {refused}");
        assert!(refused.to_string().contains(reason), "{case}: {refused}");
        assert!(!repo.0.join("docs/sas/revisions/1.0.0-rc.2.toml").exists());
    }
}

#[cfg(unix)]
#[test]
fn symlink_files_and_parent_directories_are_never_followed() {
    for parent in [false, true] {
        let repo = Repo::new();
        if parent {
            fs::rename(repo.0.join("candidate"), repo.0.join("real-candidate")).unwrap();
            std::os::unix::fs::symlink("real-candidate", repo.0.join("candidate")).unwrap();
        } else {
            fs::rename(repo.0.join("candidate/main.md"), repo.0.join("actual.md")).unwrap();
            std::os::unix::fs::symlink("../actual.md", repo.0.join("candidate/main.md")).unwrap();
        }
        let (code, refused) = repo.run(&[
            "sas",
            "propose",
            "1.0.0-rc.2",
            "--source",
            "candidate/main.md",
            "--source-set",
            "candidate/source-set.json",
            "--adr",
            "decision.md",
        ]);
        assert_ne!(code, 0);
        assert!(
            refused.to_string().contains("unsafe or missing"),
            "{refused}"
        );
        assert!(!repo.0.join("docs/sas/revisions/1.0.0-rc.2.toml").exists());
    }
}

#[test]
fn legacy_acceptance_and_authorized_warrant_pins_remain_unchanged() {
    let repo = Repo::new();
    repo.signer();
    let main = fs::read_to_string(repo.0.join("candidate/main.md")).unwrap();
    repo.write("docs/sas/main.md", &main);
    repo.ok(&["sas", "propose", "1.0.0"]);
    let old = repo.ok(&["sas", "accept", "1.0.0"]);
    assert_eq!(old["result"]["schema"], "oh.war/sas-acceptance-request/v1");
    assert!(old["result"].get("source_set").is_none());
    repo.response(
        &old,
        "Fixture Human",
        "oh.war/sas-acceptance-response/v1",
        false,
    );
    repo.ok(&["sas", "accept", "1.0.0", "--response", "response.toml"]);
    let legacy_bytes = fs::read(repo.0.join("docs/sas/revisions/1.0.0.toml")).unwrap();
    repo.ok(&["new", "Legacy fixture outcome"]);
    let old_warrant = repo.ok(&["authorize", "TEST-WAR-0001"]);
    repo.write(
        "authorization-fixture.toml",
        &format!(
            r#"schema = "oh.war/authorization-response/v1"
warrant = "TEST-WAR-0001"
contract_digest = "{}"
authorizer = "Fixture Human"
acting_role = "authorizer"
meaning = "Authorize this fictional fixture."
effective_time = "2026-09-14T08:00:00Z"
independence = "separate_role"
"#,
            old_warrant["result"]["contract_digest"].as_str().unwrap()
        ),
    );
    repo.ok(&[
        "authorize",
        "TEST-WAR-0001",
        "--response",
        "authorization-fixture.toml",
    ]);
    repo.write("candidate/main.md", &(main + "| TEST-SAS-RQ-002 | Bind the full source set. |\n\n## 107. Capture\n\nThe compiler SHALL retain the complete source-set capture.\n"));
    repo.manifest();
    repo.propose();
    let next = repo.ok(&["sas", "accept", "1.0.0-rc.2"]);
    assert_eq!(
        next["result"]["source_set"]["predecessor"]["sha256"],
        old["result"]["sha256"]
    );
    repo.response(
        &next,
        "Fixture Human",
        "oh.war/sas-acceptance-response/v2",
        true,
    );
    repo.ok(&["sas", "accept", "1.0.0-rc.2", "--response", "response.toml"]);
    assert_eq!(
        legacy_bytes,
        fs::read(repo.0.join("docs/sas/revisions/1.0.0.toml")).unwrap()
    );
    let still_old = repo.ok(&["authorize", "TEST-WAR-0001"]);
    assert_eq!(
        old_warrant["result"]["contract_digest"],
        still_old["result"]["contract_digest"]
    );
    repo.ok(&["new", "New fixture outcome"]);
    let manifest_path = repo.0.join("docs/warrants/TEST-WAR-0002/manifest.toml");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    fs::write(
        manifest_path,
        manifest
            + "\n[[implements]]\nref = \"sas://TEST-SAS-RQ-002\"\ncontribution = \"partial\"\n",
    )
    .unwrap();
    repo.ok(&["check", "TEST-WAR-0002"]);
    repo.ok(&["compile"]);
    let normative = fs::read_to_string(repo.0.join("docs/sas/generated/NORMATIVE.md")).unwrap();
    assert!(
        normative.contains("retain the complete source-set capture"),
        "{normative}"
    );
    let newer: Value = serde_json::from_slice(
        &fs::read(
            repo.0
                .join("docs/warrants/TEST-WAR-0002/generated/WAR.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(
        newer
            .to_string()
            .contains(next["result"]["sha256"].as_str().unwrap()),
        "{newer}"
    );
}

#[test]
fn changed_predecessor_and_main_only_signature_are_refused() {
    for predecessor_change in [false, true] {
        let repo = Repo::new();
        repo.signer();
        let main = fs::read_to_string(repo.0.join("candidate/main.md")).unwrap();
        repo.write("docs/sas/main.md", &main);
        repo.ok(&["sas", "propose", "1.0.0"]);
        repo.propose();
        let mut request = repo.ok(&["sas", "accept", "1.0.0-rc.2"]);
        if !predecessor_change {
            request["result"]["sha256"] = request["result"]["source_set"]["main"]["sha256"].clone();
        }
        repo.response(
            &request,
            "Fixture Human",
            "oh.war/sas-acceptance-response/v2",
            true,
        );
        if predecessor_change {
            let path = repo.0.join("docs/sas/revisions/1.0.0.toml");
            let old = fs::read_to_string(&path).unwrap();
            let original = request["result"]["source_set"]["predecessor"]["sha256"]
                .as_str()
                .unwrap();
            fs::write(path, old.replace(original, &"0".repeat(64))).unwrap();
        }
        let record = repo.0.join("docs/sas/revisions/1.0.0-rc.2.toml");
        let before = fs::read(&record).unwrap();
        let (code, refused) =
            repo.run(&["sas", "accept", "1.0.0-rc.2", "--response", "response.toml"]);
        assert_ne!(code, 0);
        let reason = if predecessor_change {
            "predecessor identity mismatch"
        } else {
            "sas.stale-digest"
        };
        assert!(refused.to_string().contains(reason), "{refused}");
        assert_eq!(before, fs::read(record).unwrap());
    }
}

#[test]
fn retained_snapshot_tampering_and_reacceptance_are_refused() {
    let repo = Repo::new();
    repo.signer();
    repo.propose();
    let request = repo.ok(&["sas", "accept", "1.0.0-rc.2"]);
    repo.response(
        &request,
        "Fixture Human",
        "oh.war/sas-acceptance-response/v2",
        true,
    );
    repo.ok(&["sas", "accept", "1.0.0-rc.2", "--response", "response.toml"]);
    let before = fs::read(repo.0.join("docs/sas/revisions/1.0.0-rc.2.toml")).unwrap();
    let (code, refused) = repo.run(&["sas", "accept", "1.0.0-rc.2", "--response", "response.toml"]);
    assert_ne!(code, 0);
    assert!(
        refused.to_string().contains("already accepted"),
        "{refused}"
    );
    assert_eq!(
        before,
        fs::read(repo.0.join("docs/sas/revisions/1.0.0-rc.2.toml")).unwrap()
    );
    repo.write(
        "docs/sas/revisions/source-sets/1.0.0-rc.2/members/rules.md",
        "Tampered capture.\n",
    );
    let (code, refused) = repo.run(&["sas", "status"]);
    assert_ne!(code, 0);
    assert!(
        refused.to_string().contains("sas.capture-invalid"),
        "{refused}"
    );
    assert!(
        !refused.to_string().contains("sas.capture-intact"),
        "{refused}"
    );
}

#[test]
fn existing_snapshot_and_retained_lock_are_never_overwritten() {
    for locked in [false, true] {
        let repo = Repo::new();
        let marker = if locked {
            "docs/sas/revisions/.1.0.0-rc.2.lock"
        } else {
            "docs/sas/revisions/source-sets/1.0.0-rc.2/marker"
        };
        repo.write(marker, "Preserve this.\n");
        let (code, refused) = repo.run(&[
            "sas",
            "propose",
            "1.0.0-rc.2",
            "--source",
            "candidate/main.md",
            "--source-set",
            "candidate/source-set.json",
            "--adr",
            "decision.md",
        ]);
        assert_ne!(code, 0);
        let reason = if locked {
            "retained lock"
        } else {
            "snapshot destination"
        };
        assert!(refused.to_string().contains(reason), "{refused}");
        assert_eq!(
            fs::read_to_string(repo.0.join(marker)).unwrap(),
            "Preserve this.\n"
        );
        assert!(!repo.0.join("docs/sas/revisions/1.0.0-rc.2.toml").exists());
    }
}

#[test]
fn accepted_record_cannot_substitute_another_adoption_decision() {
    let repo = Repo::new();
    repo.signer();
    repo.propose();
    let request = repo.ok(&["sas", "accept", "1.0.0-rc.2"]);
    repo.response(
        &request,
        "Fixture Human",
        "oh.war/sas-acceptance-response/v2",
        true,
    );
    repo.ok(&["sas", "accept", "1.0.0-rc.2", "--response", "response.toml"]);
    let path = repo.0.join("docs/sas/revisions/1.0.0-rc.2.toml");
    let record = fs::read_to_string(&path).unwrap();
    fs::write(
        path,
        record.replace("adr_ref = \"decision.md\"", "adr_ref = \"different.md\""),
    )
    .unwrap();
    let (code, refused) = repo.run(&["sas", "status"]);
    assert_ne!(code, 0, "{refused}");
    assert!(
        refused
            .to_string()
            .contains("inconsistent source-set subject"),
        "{refused}"
    );
}

#[test]
fn source_set_subject_matches_independent_canonical_vector() {
    let repo = Repo::new();
    repo.write(
        "candidate/source-set.json",
        include_str!("../../../conformance/fixtures/sas-source-set/manifest.json"),
    );
    repo.propose();
    let request = repo.ok(&["sas", "accept", "1.0.0-rc.2"]);
    let expected: Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/sas-source-set/subject.json"
    ))
    .unwrap();
    assert_eq!(request["result"]["source_set"], expected);
    assert_eq!(
        request["result"]["sha256"],
        include_str!("../../../conformance/fixtures/sas-source-set/subject.sha256").trim()
    );
}

#[test]
fn interrupted_publication_is_preserved_for_recovery() {
    let repo = Repo::new();
    let pending = "docs/sas/revisions/1.0.0-rc.2.toml.pending";
    repo.write(pending, "Interrupted publication bytes.\n");
    let (code, refused) = repo.run(&[
        "sas",
        "propose",
        "1.0.0-rc.2",
        "--source",
        "candidate/main.md",
        "--source-set",
        "candidate/source-set.json",
        "--adr",
        "decision.md",
    ]);
    assert_ne!(code, 0, "{refused}");
    assert!(refused.to_string().contains("already exists"), "{refused}");
    assert_eq!(
        fs::read_to_string(repo.0.join(pending)).unwrap(),
        "Interrupted publication bytes.\n"
    );
    assert!(!repo.0.join("docs/sas/revisions/1.0.0-rc.2.toml").exists());
}
