// SPDX-License-Identifier: Apache-2.0
//! Software fixture signatures do not establish human presence or acceptance.
use openwarrant_cli::runtime_capture::collector_signature::OpenSshSignatureCheck;
use openwarrant_core::{
    attestation::base64_encode,
    runtime_collector::{Fault, NAMESPACE, SignatureCheck},
};
use ring::{
    digest,
    signature::{Ed25519KeyPair, KeyPair},
};
use std::{fs, time::Duration};

fn string(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&u32::try_from(bytes.len()).unwrap().to_be_bytes());
    out.extend_from_slice(bytes);
}
fn fixture(namespace: &str, payload: &[u8], seed: u8) -> (String, String) {
    signed_fixture(namespace, payload, seed, None)
}
fn signed_fixture(
    namespace: &str,
    payload: &[u8],
    seed: u8,
    flags: Option<u8>,
) -> (String, String) {
    // Private test material lives only in memory; no key file or agent is used.
    let key = Ed25519KeyPair::from_seed_unchecked(&[seed; 32]).unwrap();
    let mut public = Vec::new();
    let kind = if flags.is_some() {
        "sk-ssh-ed25519@openssh.com"
    } else {
        "ssh-ed25519"
    };
    string(&mut public, kind.as_bytes());
    string(&mut public, key.public_key().as_ref());
    if flags.is_some() {
        string(&mut public, b"ssh:");
    }
    let mut message = b"SSHSIG".to_vec();
    string(&mut message, namespace.as_bytes());
    string(&mut message, b"");
    string(&mut message, b"sha512");
    string(
        &mut message,
        digest::digest(&digest::SHA512, payload).as_ref(),
    );
    let mut signature = Vec::new();
    string(&mut signature, kind.as_bytes());
    if let Some(flags) = flags {
        let mut authenticator_message = digest::digest(&digest::SHA256, b"ssh:").as_ref().to_vec();
        authenticator_message.push(flags);
        authenticator_message.extend_from_slice(&42u32.to_be_bytes());
        authenticator_message.extend_from_slice(digest::digest(&digest::SHA256, &message).as_ref());
        string(&mut signature, key.sign(&authenticator_message).as_ref());
        signature.push(flags);
        signature.extend_from_slice(&42u32.to_be_bytes());
    } else {
        string(&mut signature, key.sign(&message).as_ref());
    }
    let mut blob = b"SSHSIG".to_vec();
    blob.extend_from_slice(&1u32.to_be_bytes());
    string(&mut blob, &public);
    string(&mut blob, namespace.as_bytes());
    string(&mut blob, b"");
    string(&mut blob, b"sha512");
    string(&mut blob, &signature);
    (
        format!("{kind} {}", base64_encode(&public)),
        format!(
            "-----BEGIN SSH SIGNATURE-----\n{}\n-----END SSH SIGNATURE-----\n",
            base64_encode(&blob)
        ),
    )
}
#[test]
fn real_openssh_verifies_exact_bytes_without_inferring_presence() {
    let root = std::env::temp_dir().join(format!("ow-collector-signature-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let verifier = OpenSshSignatureCheck::new(root.clone(), Duration::from_secs(5)).unwrap();
    let payload = b"exact collector configuration";
    let (public, signature) = fixture(NAMESPACE, payload, 7);
    let evidence = verifier
        .verify(&public, NAMESPACE, payload, &signature)
        .unwrap();
    assert!(evidence.valid);
    assert_eq!(evidence.user_present, None);
    assert!(
        !verifier
            .verify(&public, NAMESPACE, b"changed", &signature)
            .unwrap()
            .valid
    );
    let (wrong_key, _) = fixture(NAMESPACE, payload, 9);
    assert!(matches!(
        verifier.verify(&wrong_key, NAMESPACE, payload, &signature),
        Err(Fault::Rejected(_))
    ));
    let (_, wrong_namespace) = fixture("other-purpose", payload, 7);
    assert!(matches!(
        verifier.verify(&public, NAMESPACE, payload, &wrong_namespace),
        Err(Fault::Rejected(_))
    ));
    assert!(matches!(
        verifier.verify(
            &format!("{public}\nattacker {public}"),
            NAMESPACE,
            payload,
            &signature
        ),
        Err(Fault::Rejected(_))
    ));
    assert_eq!(
        fs::read_dir(&root).unwrap().count(),
        0,
        "scratch survives neither success nor refusal"
    );
    fs::remove_dir(&root).unwrap();
}
#[test]
fn unavailable_scratch_and_invalid_deadlines_do_not_pass() {
    let root = std::env::temp_dir().join(format!("ow-collector-absent-{}", std::process::id()));
    let (public, signature) = fixture(NAMESPACE, b"payload", 8);
    let verifier = OpenSshSignatureCheck::new(root.clone(), Duration::from_secs(5)).unwrap();
    assert!(matches!(
        verifier.verify(&public, NAMESPACE, b"payload", &signature),
        Err(Fault::Unavailable(_))
    ));
    assert!(OpenSshSignatureCheck::new(root, Duration::ZERO).is_err());
}
#[test]
fn authenticated_software_sk_flags_are_observed_only_after_crypto() {
    // This tests the wire control, not a physical authenticator or human act.
    let root = std::env::temp_dir().join(format!("ow-collector-sk-fixture-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let verifier = OpenSshSignatureCheck::new(root.clone(), Duration::from_secs(5)).unwrap();
    for (flags, expected) in [(0, false), (1, true)] {
        let (public, signature) = signed_fixture(NAMESPACE, b"configuration", 10, Some(flags));
        let observed = verifier
            .verify(&public, NAMESPACE, b"configuration", &signature)
            .unwrap();
        assert!(observed.valid);
        assert_eq!(observed.user_present, Some(expected));
        let changed = verifier
            .verify(&public, NAMESPACE, b"tampered", &signature)
            .unwrap();
        assert!(!changed.valid);
        assert_eq!(
            changed.user_present, None,
            "unverified flags cannot become an observation"
        );
    }
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    fs::remove_dir(&root).unwrap();
}

#[test]
fn enrollment_checks_use_real_crypto_and_keep_presence_requirements() {
    use openwarrant_core::{
        authority_transition::{Policy, Principal, REVISION_SCHEMA_V2, Revision},
        contract::ActorKind,
        runtime_collector::{Enrollment, Provider, SCHEMA, Signed},
    };
    use std::collections::{BTreeMap, BTreeSet};
    let root = std::env::temp_dir().join(format!("ow-collector-enrollment-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let verifier = OpenSshSignatureCheck::new(root.clone(), Duration::from_secs(5)).unwrap();
    let (owner_key, _) = fixture(NAMESPACE, b"", 11);
    let (collector_key, _) = fixture(NAMESPACE, b"", 12);
    // These are synthetic authority records, not operator enrollment or consent.
    let mut authority = Revision {
        schema: REVISION_SCHEMA_V2.into(),
        repository: "fixture".into(),
        sequence: 0,
        policy: Some(Policy::default()),
        principals: BTreeMap::from([
            (
                "owner".into(),
                Principal {
                    public_key: owner_key,
                    roles: BTreeSet::from(["authority-admin".into()]),
                    actor: Some("fixture owner".into()),
                    kind: Some(ActorKind::Human),
                },
            ),
            (
                "collector".into(),
                Principal {
                    public_key: collector_key,
                    roles: BTreeSet::from(["runtime-collector".into()]),
                    actor: Some("fixture agent".into()),
                    kind: Some(ActorKind::Agent),
                },
            ),
        ]),
    };
    let mut enrollment = Enrollment {
        schema: SCHEMA.into(),
        repository: "fixture".into(),
        authority_digest: authority.digest().unwrap(),
        provider: Provider {
            kind: "katana".into(),
            identity: "fixture-provider".into(),
            version: "katana/openwarrant-verification/v1".into(),
        },
        verifier_digest: format!("sha256:{}", "a".repeat(64)),
        collector: "collector".into(),
        warrants: BTreeSet::from(["01a0f502-4941-70a1-a446-e1eb77dff191".into()]),
    };
    let (_, signature) = fixture(NAMESPACE, &enrollment.encode().unwrap(), 11);
    let mut signed = Signed {
        enrollment: enrollment.clone(),
        signatures: BTreeMap::from([("owner".into(), signature)]),
    };
    assert!(
        signed
            .authenticate(&authority, "fixture", &verifier)
            .is_ok()
    );
    signed.enrollment.provider.identity = "substituted".into();
    assert!(matches!(
        signed.authenticate(&authority, "fixture", &verifier),
        Err(Fault::Rejected("invalid signature"))
    ));
    authority.policy.as_mut().unwrap().require_user_presence = true;
    enrollment.authority_digest = authority.digest().unwrap();
    let (_, signature) = fixture(NAMESPACE, &enrollment.encode().unwrap(), 11);
    let signed = Signed {
        enrollment,
        signatures: BTreeMap::from([("owner".into(), signature)]),
    };
    assert!(matches!(
        signed.authenticate(&authority, "fixture", &verifier),
        Err(Fault::Unavailable(_))
    ));
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    fs::remove_dir(&root).unwrap();
}

#[cfg(target_os = "linux")]
fn namespace_entry_name() -> String {
    module_path!().split_once("::").map_or_else(
        || "namespace_fixture_entry".to_owned(),
        |(_, module)| format!("{module}::namespace_fixture_entry"),
    )
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires provisioned unshare subordinate UID/GID mapping; run explicitly"]
fn protected_namespace_roundtrip() {
    let output = std::process::Command::new("/usr/bin/timeout")
        .args([
            "45",
            "/usr/bin/unshare",
            "--user",
            "--map-root-user",
            "--map-auto",
            "--setuid",
            "0",
            "--setgid",
            "0",
        ])
        .arg(std::env::current_exe().unwrap())
        .args([
            "--exact",
            namespace_entry_name().as_str(),
            "--ignored",
            "--nocapture",
        ])
        .env("OW_COLLECTOR_NAMESPACE_FIXTURE", "operator")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("operator UID 0:") && text.contains("execution UID 1:"),
        "namespace subprocesses must actually run both account observations: {text}"
    );
    println!("{text}");
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "subprocess fixture entry; not an operator enrollment or human act"]
fn namespace_fixture_entry() {
    use openwarrant_cli::runtime_capture::collector_loading::LoadedEnrollment;
    use openwarrant_core::{
        authority_transition::{
            Operation, PROPOSAL_SCHEMA, Policy, Principal, Proposal, REVISION_SCHEMA_V2, Revision,
        },
        contract::ActorKind,
        runtime_collector::{Enrollment, Provider, SCHEMA, Signed, Use},
    };
    use std::{
        collections::{BTreeMap, BTreeSet},
        os::unix::fs::PermissionsExt,
        process::{Command, Stdio},
        time::Instant,
    };
    let role =
        std::env::var("OW_COLLECTOR_NAMESPACE_FIXTURE").expect("explicit fixture role required");
    if role == "executor" {
        let root = std::path::PathBuf::from(std::env::var_os("OW_COLLECTOR_FIXTURE_ROOT").unwrap());
        let scratch = root.join("worker-scratch");
        assert_eq!(rustix::process::getuid().as_raw(), 1);
        assert_eq!(rustix::process::geteuid().as_raw(), 1);
        assert!(
            fs::read_to_string("/proc/self/status")
                .unwrap()
                .lines()
                .any(|s| s == "CapEff:\t0000000000000000")
        );
        assert!(
            fs::write(root.join("store/state.json"), b"forged").is_err(),
            "execution account wrote authority"
        );
        let verifier = OpenSshSignatureCheck::new(scratch.clone(), Duration::from_secs(5)).unwrap();
        let loaded = LoadedEnrollment::load(
            &root.join("store"),
            "fixture",
            &root.join("enrollment.json"),
            &verifier,
        )
        .unwrap();
        let enrollment = loaded.enrollment();
        let usage = Use {
            repository: "fixture",
            warrant: enrollment.warrants.iter().next().unwrap(),
            provider: &enrollment.provider,
            verifier_digest: &enrollment.verifier_digest,
            collector: &enrollment.collector,
        };
        loaded.allows(Use { ..usage }).unwrap();
        assert!(matches!(
            LoadedEnrollment::load(
                &root.join("mismatched-store"),
                "fixture",
                &root.join("enrollment.json"),
                &verifier
            ),
            Err(Fault::Rejected(
                "authority store execution account mismatch"
            ))
        ));
        fs::write(scratch.join("ready"), b"ready").unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !root.join("updated").exists() {
            assert!(Instant::now() < deadline, "operator update unavailable");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            matches!(loaded.allows(usage), Err(Fault::Rejected(_))),
            "revoked authority remained usable"
        );
        println!(
            "execution UID 1: authority write refused; real enrollment accepted; mismatched execution UID and signed revocation refused"
        );
        return;
    }
    assert_eq!(role, "operator");
    assert_eq!(rustix::process::geteuid().as_raw(), 0);
    let root = std::env::temp_dir().join(format!("ow-collector-ns-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
    let scratch = root.join("worker-scratch");
    fs::create_dir(&scratch).unwrap();
    rustix::fs::chown(
        &scratch,
        Some(rustix::process::Uid::from_raw(1)),
        Some(rustix::process::Gid::from_raw(1)),
    )
    .unwrap();
    fs::set_permissions(&scratch, fs::Permissions::from_mode(0o700)).unwrap();
    for name in ["store", "mismatched-store"] {
        let path = root.join(name);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let (owner_key, _) = fixture(NAMESPACE, b"", 11);
    let (collector_key, _) = fixture(NAMESPACE, b"", 12);
    let genesis = Revision {
        schema: REVISION_SCHEMA_V2.into(),
        repository: "fixture".into(),
        sequence: 0,
        policy: Some(Policy::default()),
        principals: BTreeMap::from([
            (
                "owner".into(),
                Principal {
                    public_key: owner_key,
                    roles: BTreeSet::from(["authority-admin".into()]),
                    actor: Some("software fixture owner".into()),
                    kind: Some(ActorKind::Human),
                },
            ),
            (
                "collector".into(),
                Principal {
                    public_key: collector_key,
                    roles: BTreeSet::from(["runtime-collector".into()]),
                    actor: Some("software fixture collector".into()),
                    kind: Some(ActorKind::Agent),
                },
            ),
        ]),
    };
    let genesis_file = root.join("genesis.json");
    fs::write(&genesis_file, genesis.encode().unwrap()).unwrap();
    let authority_command = |args: &[&str]| {
        let out = Command::new("/usr/bin/timeout")
            .arg("10")
            .arg(
                std::env::var_os("OW_COLLECTOR_FIXTURE_WAR")
                    .unwrap_or_else(|| env!("CARGO_BIN_EXE_war").into()),
            )
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("OPENWARRANT_NO_PROJECTS", "1")
            .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "authority command failed: {} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        out
    };
    for (name, uid) in [("store", "1"), ("mismatched-store", "2")] {
        authority_command(&[
            "authority",
            "bootstrap",
            "--store",
            root.join(name).to_str().unwrap(),
            "--revision",
            genesis_file.to_str().unwrap(),
            "--expected-digest",
            &genesis.digest().unwrap(),
            "--agent-uid",
            uid,
            "--execution-readable",
        ]);
    }
    let enrollment = Enrollment {
        schema: SCHEMA.into(),
        repository: "fixture".into(),
        authority_digest: genesis.digest().unwrap(),
        provider: Provider {
            kind: "katana".into(),
            identity: "software-fixture".into(),
            version: "v1".into(),
        },
        verifier_digest: format!("sha256:{}", "a".repeat(64)),
        collector: "collector".into(),
        warrants: BTreeSet::from(["01a0f502-4941-70a1-a446-e1eb77dff191".into()]),
    };
    let (_, signature) = fixture(NAMESPACE, &enrollment.encode().unwrap(), 11);
    fs::write(
        root.join("enrollment.json"),
        Signed {
            enrollment,
            signatures: BTreeMap::from([("owner".into(), signature)]),
        }
        .encode()
        .unwrap(),
    )
    .unwrap();
    let mut next = genesis.clone();
    next.sequence = 1;
    next.principals.get_mut("collector").unwrap().roles.clear();
    let proposal = Proposal {
        schema: PROPOSAL_SCHEMA.into(),
        operation: Operation::Update,
        previous_digest: genesis.digest().unwrap(),
        next,
    };
    let (_, signature) = fixture(
        "openwarrant-authority-v1",
        &proposal.signing_bytes().unwrap(),
        11,
    );
    let proposal_file = root.join("transition.json");
    let signature_file = root.join("transition.sig");
    fs::write(&proposal_file, proposal.encode().unwrap()).unwrap();
    fs::write(&signature_file, signature).unwrap();
    let mut child = Command::new("/usr/bin/setpriv")
        .args([
            "--reuid",
            "1",
            "--regid",
            "1",
            "--keep-groups",
            "--inh-caps=-all",
            "--ambient-caps=-all",
            "--bounding-set=-all",
            "--no-new-privs",
        ])
        .arg(std::env::current_exe().unwrap())
        .env_clear()
        .env("OW_COLLECTOR_NAMESPACE_FIXTURE", "executor")
        .env("OW_COLLECTOR_FIXTURE_ROOT", &root)
        .env("TMPDIR", &scratch)
        .args([
            "--exact",
            namespace_entry_name().as_str(),
            "--ignored",
            "--nocapture",
        ])
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !scratch.join("ready").exists() {
        assert!(
            child.try_wait().unwrap().is_none(),
            "executor exited before readiness"
        );
        assert!(Instant::now() < deadline, "executor readiness unavailable");
        std::thread::sleep(Duration::from_millis(10));
    }
    authority_command(&[
        "authority",
        "activate",
        "--store",
        root.join("store").to_str().unwrap(),
        "--proposal",
        proposal_file.to_str().unwrap(),
        "--signature",
        &format!("owner={}", signature_file.display()),
    ]);
    fs::write(root.join("updated"), b"updated").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "executor completion unavailable");
        std::thread::sleep(Duration::from_millis(10));
    };
    fs::remove_dir_all(root).unwrap();
    assert!(status.success());
    println!(
        "operator UID 0: real normal-mode CLI bootstrap and activation, software signature; fixture removed; no human acceptance or host-account qualification claimed"
    );
}
