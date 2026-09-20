// SPDX-License-Identifier: Apache-2.0
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
fn war(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_war"))
        .args(args)
        .output()
        .unwrap()
}
fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}
#[test]
fn unsigned_proposal_cannot_activate_and_signed_transition_survives_restart() {
    let root = std::env::temp_dir().join(format!(
        "ow-authority-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let key = root.join("key");
    assert!(
        Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f", text(&key)])
            .status()
            .unwrap()
            .success()
    );
    let key_text = fs::read_to_string(key.with_extension("pub"))
        .unwrap()
        .split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ");
    let current = serde_json::json!({"schema":"oh.war/authority-revision/1","repository":"test-repo","sequence":0,"principals":{"owner":{"public_key":key_text,"roles":["authority-admin","authority-recovery"]}}});
    let current_file = root.join("current.json");
    fs::write(&current_file, serde_jcs::to_vec(&current).unwrap()).unwrap();
    let next_file = root.join("next.json");
    let mut next = current.clone();
    next["sequence"] = 1.into();
    next["principals"]["owner"]["roles"] =
        serde_json::json!(["authority-admin", "authority-recovery", "warrant-review"]);
    fs::write(&next_file, serde_jcs::to_vec(&next).unwrap()).unwrap();
    let proposal = root.join("proposal.json");
    let made = war(&[
        "authority",
        "propose",
        "--current",
        text(&current_file),
        "--next",
        text(&next_file),
        "--emit",
        text(&proposal),
        "--json",
    ]);
    assert!(
        made.status.success(),
        "{}",
        String::from_utf8_lossy(&made.stderr)
    );
    let made: serde_json::Value = serde_json::from_slice(&made.stdout).unwrap();
    let digest = made["result"]["previous_digest"].as_str().unwrap();
    let store = root.join("store");
    fs::create_dir(&store).unwrap();
    let boot = war(&[
        "authority",
        "bootstrap",
        "--store",
        text(&store),
        "--revision",
        text(&current_file),
        "--expected-digest",
        digest,
        "--unprotected-test-store",
    ]);
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );
    let denied = war(&[
        "authority",
        "activate",
        "--store",
        text(&store),
        "--proposal",
        text(&proposal),
        "--unprotected-test-store",
    ]);
    assert!(!denied.status.success());
    assert!(String::from_utf8_lossy(&denied.stderr).contains("authority-signer"));
    let sig = root.join("approval.sig");
    let signed = war(&[
        "authority",
        "approve",
        "--current",
        text(&current_file),
        "--proposal",
        text(&proposal),
        "--principal",
        "owner",
        "--key",
        text(&key),
        "--emit",
        text(&sig),
    ]);
    assert!(
        signed.status.success(),
        "{}",
        String::from_utf8_lossy(&signed.stderr)
    );
    let signature = format!("owner={}", sig.display());
    let applied = war(&[
        "authority",
        "activate",
        "--store",
        text(&store),
        "--proposal",
        text(&proposal),
        "--signature",
        &signature,
        "--unprotected-test-store",
        "--json",
    ]);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let status = war(&[
        "authority",
        "status",
        "--store",
        text(&store),
        "--unprotected-test-store",
        "--json",
    ]);
    assert!(status.status.success());
    let status: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status["result"]["current"]["sequence"], 1);
    assert_eq!(status["result"]["isolation_enforced"], false);
    let receipt = &status["result"]["activation_receipts"]["1"];
    assert_eq!(receipt["previous_head"], digest);
    assert_eq!(receipt["new_head"], status["result"]["head"]);
    assert_eq!(
        receipt["authenticated_signers"],
        serde_json::json!(["owner"])
    );
    assert!(receipt["observed_at_unix_seconds"].as_u64().unwrap() > 0);
    assert_eq!(status["result"]["missing_activation_receipts"], 0);
    assert_eq!(status["result"]["activation_time_authenticated"], false);
    assert_eq!(status["result"]["human_review_established"], false);
    let saved = fs::read(store.join("state.json")).unwrap();
    let mut changed: serde_json::Value = serde_json::from_slice(&saved).unwrap();
    changed["activation_receipts"]["1"]["new_head"] = "sha256:wrong".into();
    fs::write(
        store.join("state.json"),
        serde_jcs::to_vec(&changed).unwrap(),
    )
    .unwrap();
    let refused = war(&[
        "authority",
        "status",
        "--store",
        text(&store),
        "--unprotected-test-store",
    ]);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("authority-receipt-subject"));
    #[cfg(unix)]
    assert_eq!(receipt["operator_uid"], rustix::process::geteuid().as_raw());
    for field in [
        "transition_digest",
        "previous_head",
        "authenticated_signers",
    ] {
        let mut planted: serde_json::Value = serde_json::from_slice(&saved).unwrap();
        planted["activation_receipts"]["1"][field] = if field == "authenticated_signers" {
            serde_json::json!(["intruder"])
        } else {
            serde_json::json!("sha256:wrong")
        };
        fs::write(
            store.join("state.json"),
            serde_jcs::to_vec(&planted).unwrap(),
        )
        .unwrap();
        let refused = war(&[
            "authority",
            "status",
            "--store",
            text(&store),
            "--unprotected-test-store",
        ]);
        assert!(!refused.status.success());
        assert!(String::from_utf8_lossy(&refused.stderr).contains("authority-receipt-subject"));
    }
    for sequence in ["0", "2"] {
        let mut planted: serde_json::Value = serde_json::from_slice(&saved).unwrap();
        let map = planted["activation_receipts"].as_object_mut().unwrap();
        let entry = map.remove("1").unwrap();
        map.insert(sequence.into(), entry);
        fs::write(
            store.join("state.json"),
            serde_jcs::to_vec(&planted).unwrap(),
        )
        .unwrap();
        let refused = war(&[
            "authority",
            "status",
            "--store",
            text(&store),
            "--unprotected-test-store",
        ]);
        assert!(!refused.status.success());
        assert!(String::from_utf8_lossy(&refused.stderr).contains("authority-receipt-sequence"));
    }
    // Old snapshots remain loadable, but missing receipts stay missing.
    changed
        .as_object_mut()
        .unwrap()
        .remove("activation_receipts");
    fs::write(
        store.join("state.json"),
        serde_jcs::to_vec(&changed).unwrap(),
    )
    .unwrap();
    let legacy = war(&[
        "authority",
        "status",
        "--store",
        text(&store),
        "--unprotected-test-store",
        "--json",
    ]);
    assert!(legacy.status.success());
    let legacy: serde_json::Value = serde_json::from_slice(&legacy.stdout).unwrap();
    assert_eq!(legacy["result"]["missing_activation_receipts"], 1);
    assert_eq!(
        legacy["result"]["activation_receipts"],
        serde_json::json!({})
    );
    fs::write(store.join("state.json"), saved).unwrap();
    let replay = war(&[
        "authority",
        "activate",
        "--store",
        text(&store),
        "--proposal",
        text(&proposal),
        "--signature",
        &signature,
        "--unprotected-test-store",
    ]);
    assert!(!replay.status.success());
    assert!(String::from_utf8_lossy(&replay.stderr).contains("authority-parent"));
    let unsafe_mode = war(&["authority", "status", "--store", text(&store)]);
    assert!(!unsafe_mode.status.success());
    assert!(String::from_utf8_lossy(&unsafe_mode.stderr).contains("authority-store-mode-mismatch"));
    let corrupt_sig = root.join("corrupt.sig");
    fs::write(&corrupt_sig, "not a signature").unwrap();
    let corrupt_arg = format!("owner={}", corrupt_sig.display());
    let invalid = war(&[
        "authority",
        "check",
        "--current",
        text(&current_file),
        "--proposal",
        text(&proposal),
        "--signature",
        &corrupt_arg,
    ]);
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("authority-signature-invalid"));
    let fake_dir = root.join("fake-bin");
    fs::create_dir(&fake_dir).unwrap();
    let fake = fake_dir.join("ssh-keygen");
    fs::write(&fake, "#!/bin/sh\ncat >/dev/null\nexit 0\n").unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let poisoned = Command::new(env!("CARGO_BIN_EXE_war"))
        .env("PATH", format!("{}:/usr/bin:/bin", fake_dir.display()))
        .args([
            "authority",
            "check",
            "--current",
            text(&current_file),
            "--proposal",
            text(&proposal),
            "--signature",
            &corrupt_arg,
        ])
        .output()
        .unwrap();
    assert!(!poisoned.status.success());
    assert!(String::from_utf8_lossy(&poisoned.stderr).contains("authority-signature-invalid"));
    let mut changed: serde_json::Value =
        serde_json::from_slice(&fs::read(&proposal).unwrap()).unwrap();
    changed["next"]["principals"]["owner"]["roles"] = serde_json::json!(["authority-admin"]);
    let changed_path = root.join("changed.json");
    fs::write(&changed_path, serde_jcs::to_vec(&changed).unwrap()).unwrap();
    let tampered = war(&[
        "authority",
        "check",
        "--current",
        text(&current_file),
        "--proposal",
        text(&changed_path),
        "--signature",
        &signature,
    ]);
    assert!(!tampered.status.success());
    assert!(String::from_utf8_lossy(&tampered.stderr).contains("authority-signature-invalid"));
    let again = war(&[
        "authority",
        "bootstrap",
        "--store",
        text(&store),
        "--revision",
        text(&current_file),
        "--expected-digest",
        digest,
        "--unprotected-test-store",
    ]);
    assert!(!again.status.success());
    assert!(String::from_utf8_lossy(&again.stderr).contains("authority-already-bootstrapped"));
    let exported = root.join("export.json");
    assert!(
        war(&[
            "authority",
            "status",
            "--store",
            text(&store),
            "--emit",
            text(&exported),
            "--unprotected-test-store"
        ])
        .status
        .success()
    );
    let head = status["result"]["head"].as_str().unwrap();
    assert!(
        war(&[
            "authority",
            "allows",
            "--store",
            text(&store),
            "--principal",
            "owner",
            "--role",
            "warrant-review",
            "--expected-head",
            head,
            "--unprotected-test-store"
        ])
        .status
        .success()
    );
    let refused = war(&[
        "authority",
        "allows",
        "--store",
        text(&store),
        "--principal",
        "owner",
        "--role",
        "other",
        "--expected-head",
        head,
        "--unprotected-test-store",
    ]);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("authority-role-denied"));
    let race_next = root.join("race-next.json");
    assert!(
        war(&[
            "authority",
            "draft",
            "--current",
            text(&exported),
            "--principal",
            "owner",
            "--role",
            "authority-admin",
            "--role",
            "authority-recovery",
            "--emit",
            text(&race_next)
        ])
        .status
        .success()
    );
    let race_proposal = root.join("race-proposal.json");
    assert!(
        war(&[
            "authority",
            "propose",
            "--current",
            text(&exported),
            "--next",
            text(&race_next),
            "--emit",
            text(&race_proposal)
        ])
        .status
        .success()
    );
    let race_sig = root.join("race.sig");
    assert!(
        war(&[
            "authority",
            "approve",
            "--current",
            text(&exported),
            "--proposal",
            text(&race_proposal),
            "--principal",
            "owner",
            "--key",
            text(&key),
            "--emit",
            text(&race_sig)
        ])
        .status
        .success()
    );
    let race_arg = format!("owner={}", race_sig.display());
    let args = [
        "authority",
        "activate",
        "--store",
        text(&store),
        "--proposal",
        text(&race_proposal),
        "--signature",
        &race_arg,
        "--unprotected-test-store",
    ];
    let mut a = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut b = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    assert_ne!(
        a.wait().unwrap().success(),
        b.wait().unwrap().success(),
        "only one transition may win"
    );
    let legacy = root.join("legacy");
    fs::create_dir(&legacy).unwrap();
    fs::write(legacy.join("roles.toml"), b"original roles\n").unwrap();
    fs::write(legacy.join("allowed_signers"), b"original keys\n").unwrap();
    let migrated = root.join("migrated");
    fs::create_dir(&migrated).unwrap();
    assert!(
        war(&[
            "authority",
            "bootstrap",
            "--store",
            text(&migrated),
            "--revision",
            text(&current_file),
            "--expected-digest",
            digest,
            "--legacy-dir",
            text(&legacy),
            "--unprotected-test-store"
        ])
        .status
        .success()
    );
    let combined = war(&[
        "authority",
        "approve",
        "--store",
        text(&migrated),
        "--proposal",
        text(&proposal),
        "--principal",
        "owner",
        "--key",
        text(&key),
        "--activate",
        "--unprotected-test-store",
        "--json",
    ]);
    assert!(
        combined.status.success(),
        "{}",
        String::from_utf8_lossy(&combined.stderr)
    );
    let combined: serde_json::Value = serde_json::from_slice(&combined.stdout).unwrap();
    assert_eq!(combined["result"]["current"]["sequence"], 1);
    let history = root.join("history.json");
    assert!(
        war(&[
            "authority",
            "history",
            "--store",
            text(&migrated),
            "--emit",
            text(&history),
            "--unprotected-test-store"
        ])
        .status
        .success()
    );
    let retained: serde_json::Value = serde_json::from_slice(&fs::read(&history).unwrap()).unwrap();
    assert_eq!(
        retained["legacy"]["roles.toml"],
        serde_json::json!(b"original roles\n".to_vec())
    );
    assert_eq!(
        fs::read(legacy.join("allowed_signers")).unwrap(),
        b"original keys\n"
    );
    let prod = root.join("prod");
    fs::create_dir(&prod).unwrap();
    let uid = rustix::process::geteuid().as_raw().to_string();
    let unprotected = war(&[
        "authority",
        "bootstrap",
        "--store",
        text(&prod),
        "--revision",
        text(&current_file),
        "--expected-digest",
        digest,
        "--agent-uid",
        &uid,
    ]);
    assert!(!unprotected.status.success());
    assert!(
        String::from_utf8_lossy(&unprotected.stderr)
            .contains("authority-store-separate-account-required")
    );
    assert!(!prod.join("state.json").exists());
    fs::remove_dir_all(root).unwrap();
}
