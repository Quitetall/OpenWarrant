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
    // Host-root-owned ancestors are unmapped in this user namespace. A private
    // mount namespace and disposable chroot give normal-mode guard checks real
    // namespace-owned ancestors without weakening the production checks.
    let mountpoint = std::env::temp_dir().join(format!("ow-collector-root-{}", std::process::id()));
    fs::create_dir(&mountpoint).unwrap();
    let war = std::env::var_os("OW_COLLECTOR_FIXTURE_WAR")
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_war").into());
    let output = std::process::Command::new("/usr/bin/timeout")
        .args([
            "45",
            "/usr/bin/unshare",
            "--user",
            "--map-root-user",
            "--map-auto",
            "--mount",
            "--setuid",
            "0",
            "--setgid",
            "0",
            "/usr/bin/sh",
            "-eu",
            "-c",
            r#"
root=$1
/usr/bin/mount --make-rprivate /
/usr/bin/mount -t tmpfs -o mode=0755 tmpfs "$root"
/usr/bin/mkdir -p "$root/usr" "$root/proc" "$root/dev" "$root/scratch" "$root/tmp" "$root/etc"
/usr/bin/mount --bind /usr "$root/usr"
/usr/bin/mount -o remount,bind,ro "$root/usr"
/usr/bin/mount --rbind /proc "$root/proc"
/usr/bin/mount --rbind /dev "$root/dev"
/usr/bin/ln -s usr/bin "$root/bin"
printf 'root:x:0:0:fixture:/root:/bin/sh\nworker:x:1:1:fixture:/scratch:/bin/sh\n' > "$root/etc/passwd"
printf 'root:x:0:\nworker:x:1:\n' > "$root/etc/group"
printf 'passwd: files\ngroup: files\n' > "$root/etc/nsswitch.conf"
/usr/bin/ln -s usr/lib "$root/lib"
/usr/bin/ln -s usr/lib64 "$root/lib64"
/usr/bin/cp "$2" "$root/test-runner"
/usr/bin/cp "$3" "$root/war"
/usr/bin/chmod 0755 "$root/test-runner" "$root/war" "$root/scratch"
export TMPDIR=/scratch OW_COLLECTOR_FIXTURE_WAR=/war
export OW_COLLECTOR_NAMESPACE_FIXTURE=operator
cd "$root"
exec /usr/bin/chroot "$root" /test-runner --exact "$4" --ignored --nocapture
"#,
            "namespace-fixture",
        ])
        .arg(&mountpoint)
        .arg(std::env::current_exe().unwrap())
        .arg(war)
        .arg(namespace_entry_name())
        .output()
        .unwrap();
    fs::remove_dir(&mountpoint).unwrap();
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
    fn open_reader_repository(
        root: &std::path::Path,
        store: &std::path::Path,
        name: &str,
    ) -> openwarrant_cli::repo::Repository {
        let repository = root.join(name);
        fs::create_dir(&repository).unwrap();
        fs::write(repository.join("openwarrant.toml"), format!(
            "schema = \"oh.war/repository-config/v1\"\n[project]\nname = \"fixture\"\nnamespace = \"FX\"\n[paths]\n[authority]\nstore = {:?}\n", store
        )).unwrap();
        openwarrant_cli::repo::Repository::open(repository.try_into().unwrap()).unwrap()
    }
    if role == "capable-executor" {
        let root = std::path::PathBuf::from(std::env::var_os("OW_COLLECTOR_FIXTURE_ROOT").unwrap());
        assert_eq!(rustix::process::getuid().as_raw(), 1);
        assert_eq!(rustix::process::geteuid().as_raw(), 1);
        let caps = fs::read_to_string("/proc/self/status").unwrap();
        assert!(caps.lines().any(|s| s == "CapEff:\t0000000000000002"));
        // The actual effective account can mutate an operator-owned readonly
        // object even though access(2), using the real UID, denies WRITE_OK.
        let probe = root.join("capability-probe");
        assert!(rustix::fs::access(&probe, rustix::fs::Access::WRITE_OK).is_err());
        fs::write(&probe, b"effective account wrote readonly operator bytes").unwrap();
        let verifier =
            OpenSshSignatureCheck::new(std::env::temp_dir(), Duration::from_secs(5)).unwrap();
        let loaded =
            LoadedEnrollment::load_active(&root.join("store"), "fixture", "collector", &verifier);
        println!(
            "capable executor observation: accepted={}; fault={:?}",
            loaded.is_ok(),
            loaded.as_ref().err()
        );
        assert!(
            matches!(
                loaded,
                Err(Fault::Rejected(
                    "authority store is writable by the executor"
                ))
            ),
            "effective capability lets this executor write authority; loading must refuse"
        );
        println!(
            "capable execution UID 1: actual write succeeded; authority loading refused effective write capability"
        );
        // The general repository read path must not trust a caller-writable,
        // unsigned agent_uid setting as the condition for checking real access.
        let store = root.join("reader-store");
        let state_path = store.join("state.json");
        let mut state: serde_json::Value =
            serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
        let original_genesis = state["genesis"].clone();
        assert_eq!(state["agent_uid"], 1);
        state["agent_uid"] = serde_json::json!(2);
        fs::write(&state_path, serde_jcs::to_vec(&state).unwrap()).unwrap();
        assert_eq!(state["genesis"], original_genesis);
        let repo = open_reader_repository(&root, &store, "reader-repository");
        use openwarrant_core::config::Governance;
        let accepted = matches!(repo.config.governance, Governance::Store { .. });
        println!("general authority reader after unsigned UID reassignment: accepted={accepted}");
        assert!(
            matches!(repo.config.governance, Governance::FailedClosed { why, .. } if why.contains("authority-store-writable-by-reader")),
            "a writable non-owner reader must not accept authority by changing unsigned agent_uid"
        );

        return;
    }
    if role == "permission-editor" || role == "latent-permission-editor" {
        let root = std::path::PathBuf::from(std::env::var_os("OW_COLLECTOR_FIXTURE_ROOT").unwrap());
        assert_eq!(rustix::process::getuid().as_raw(), 1);
        assert_eq!(rustix::process::geteuid().as_raw(), 1);
        let caps = fs::read_to_string("/proc/self/status").unwrap();
        assert!(caps.lines().any(|s| s == "CapEff:\t0000000000000008"));
        let probe = root.join("permission-probe");
        use std::os::unix::fs::MetadataExt;
        assert_eq!(fs::metadata(&probe).unwrap().uid(), 0);
        assert!(
            rustix::fs::accessat(
                rustix::fs::CWD,
                &probe,
                rustix::fs::Access::WRITE_OK,
                rustix::fs::AtFlags::EACCESS,
            )
            .is_err()
        );
        fs::set_permissions(&probe, fs::Permissions::from_mode(0o666)).unwrap();
        fs::write(&probe, b"non-owner changed mode, then wrote operator bytes").unwrap();
        println!(
            "permission editor UID 1: EACCESS denied direct write; CAP_FOWNER changed operator file mode and actual write succeeded"
        );
        if role == "latent-permission-editor" {
            let mut sets = rustix::thread::capabilities(None).unwrap();
            sets.effective = rustix::thread::CapabilitySet::empty();
            rustix::thread::set_capabilities(None, sets).unwrap();
            let sets = rustix::thread::capabilities(None).unwrap();
            assert!(sets.effective.is_empty());
            assert_eq!(sets.permitted, rustix::thread::CapabilitySet::FOWNER);
            println!(
                "latent permission editor: effective capabilities empty; permitted CAP_FOWNER retained"
            );
        }
        let verifier =
            OpenSshSignatureCheck::new(std::env::temp_dir(), Duration::from_secs(5)).unwrap();
        let loaded =
            LoadedEnrollment::load_active(&root.join("store"), "fixture", "collector", &verifier);
        println!(
            "permission editor protected enrollment: accepted={}; fault={:?}",
            loaded.is_ok(),
            loaded.as_ref().err()
        );
        assert!(
            matches!(
                loaded,
                Err(Fault::Unavailable(
                    "execution privileges cannot establish protected authority"
                ))
            ),
            "a permission-changing executor cannot establish a protected authority boundary"
        );
        let repo = open_reader_repository(
            &std::env::temp_dir(),
            &root.join("reader-store"),
            &format!("{role}-reader-repository"),
        );
        assert!(matches!(repo.config.governance,
            openwarrant_core::config::Governance::FailedClosed { why, .. }
                if why.contains("authority-store-reader-privileges-unqualified")));
        println!(
            "permission editor: both collector and general authority reader refuse unsupported privilege profile"
        );
        if role == "latent-permission-editor" {
            let mut sets = rustix::thread::capabilities(None).unwrap();
            sets.effective = rustix::thread::CapabilitySet::FOWNER;
            rustix::thread::set_capabilities(None, sets).unwrap();
            fs::set_permissions(&probe, fs::Permissions::from_mode(0o444)).unwrap();
            fs::set_permissions(&probe, fs::Permissions::from_mode(0o666)).unwrap();
            fs::write(
                &probe,
                b"retained permitted capability re-enabled actual mutation",
            )
            .unwrap();
            println!(
                "latent permission editor: permitted capability re-enabled and actual mutation succeeded"
            );
        }
        return;
    }
    if role == "no-account" {
        assert_eq!(rustix::process::geteuid().as_raw(), 2);
        assert!(
            !fs::read_to_string("/etc/passwd")
                .unwrap()
                .lines()
                .any(|line| line.split(':').nth(2) == Some("2"))
        );
        let verifier =
            OpenSshSignatureCheck::new(std::env::temp_dir(), Duration::from_secs(5)).unwrap();
        let (key, signature) = fixture(NAMESPACE, b"exact fixture payload", 11);
        let result = verifier.verify(&key, NAMESPACE, b"exact fixture payload", &signature);
        assert!(
            matches!(result, Err(Fault::Unavailable(_))),
            "a verifier without its required account must be UNKNOWN, not invalid crypto: {result:?}"
        );
        println!("missing-account UID 2: verification UNKNOWN; no cryptographic verdict inferred");
        return;
    }
    if role == "executor" {
        let root = std::path::PathBuf::from(std::env::var_os("OW_COLLECTOR_FIXTURE_ROOT").unwrap());
        let scratch = root.join("worker-scratch");
        use openwarrant_cli::runtime_capture::protected_executable::ProtectedExecutable;
        use openwarrant_core::document::runtime::ProviderFailure;
        let program = root.join("protected-verifier");
        let approved_digest = fs::read_to_string(root.join("verifier.sha256")).unwrap();
        let protected = ProtectedExecutable::acquire(&program, &approved_digest).unwrap();
        assert!(matches!(
            ProtectedExecutable::acquire(&root.join("writable-verifier"), &approved_digest),
            Err(ProviderFailure::Rejected(_))
        ));
        assert!(matches!(
            ProtectedExecutable::acquire(&root.join("linked-verifier"), &approved_digest),
            Err(ProviderFailure::Rejected(_))
        ));
        assert!(matches!(
            ProtectedExecutable::acquire(&root.join("script-verifier"), &approved_digest),
            Err(ProviderFailure::Rejected(_))
        ));
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
        let before = fs::read(root.join("store/state.json")).unwrap();
        let refused = Command::new("/usr/bin/timeout")
            .args(["10", "/war", "authority", "activate-collector", "--store"])
            .arg(root.join("store"))
            .arg("--enrollment")
            .arg(root.join("enrollment.json"))
            .arg("--json")
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("OPENWARRANT_NO_PROJECTS", "1")
            .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
            .output()
            .unwrap();
        assert_eq!(
            refused.status.code(),
            Some(1),
            "execution account activated collector"
        );
        assert!(
            String::from_utf8_lossy(&refused.stdout)
                .contains("authority-store-state-unsafe-owner-or-mode")
        );
        assert_eq!(fs::read(root.join("store/state.json")).unwrap(), before);
        let verifier = OpenSshSignatureCheck::new(scratch.clone(), Duration::from_secs(5)).unwrap();
        assert!(matches!(
            LoadedEnrollment::load_active(
                &root.join("unactivated-store"),
                "fixture",
                "collector",
                &verifier,
            ),
            Err(Fault::Unavailable("collector activation unavailable"))
        ));
        let loaded =
            LoadedEnrollment::load_active(&root.join("store"), "fixture", "collector", &verifier)
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
        let repo =
            open_reader_repository(&scratch, &root.join("reader-store"), "readonly-repository");
        assert!(matches!(
            repo.config.governance,
            openwarrant_core::config::Governance::Store { .. }
        ));
        println!("general authority reader: non-owner without effective write capability accepted");

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
        use openwarrant_cli::runtime_capture::activated_verifier::ActivatedVerifier;
        use openwarrant_core::document::runtime::{ProviderInterface, ProviderKind};
        let interface = ProviderInterface {
            kind: ProviderKind::Katana,
            identity: "software-fixture".into(),
            version: "v1".into(),
        };
        let warrant = "01a0f502-4941-70a1-a446-e1eb77dff191";
        let load = || {
            LoadedEnrollment::load_active(&root.join("store"), "fixture", "collector", &verifier)
                .unwrap()
        };
        let acquire = |repository: &str,
                       collector: &str,
                       warrant: &str,
                       provider: &ProviderInterface,
                       path: &std::path::Path| {
            ActivatedVerifier::acquire(load(), repository, collector, warrant, provider, path)
        };
        let inactive = LoadedEnrollment::load(
            &root.join("store"),
            "fixture",
            &root.join("enrollment.json"),
            &verifier,
        )
        .unwrap();
        assert!(matches!(
            ActivatedVerifier::acquire(
                inactive,
                "fixture",
                "collector",
                warrant,
                &interface,
                &program
            ),
            Err(ProviderFailure::Unavailable(_))
        ));
        for (repository, collector, scope) in [
            ("other", "collector", warrant),
            ("fixture", "other", warrant),
            (
                "fixture",
                "collector",
                "01a0f502-4941-70a1-a446-e1eb77dff192",
            ),
        ] {
            assert!(matches!(
                acquire(repository, collector, scope, &interface, &program),
                Err(ProviderFailure::Rejected(_))
            ));
        }
        let wrong_provider = ProviderInterface {
            kind: ProviderKind::Blut,
            ..interface.clone()
        };
        assert!(matches!(
            acquire("fixture", "collector", warrant, &wrong_provider, &program),
            Err(ProviderFailure::Rejected(_))
        ));
        assert!(matches!(
            acquire(
                "fixture",
                "collector",
                warrant,
                &interface,
                &root.join("writable-verifier")
            ),
            Err(ProviderFailure::Rejected(_))
        ));
        let activated = acquire("fixture", "collector", warrant, &interface, &program).unwrap();
        assert!(activated.run(&[], Duration::from_secs(3), 1024).unwrap().0);
        println!(
            "activated verifier: sealed execution passed; inactive enrollment, repository, collector, scope, provider and writable executable refused"
        );
        fs::write(scratch.join("ready"), b"ready").unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !root.join("selection-changed").exists() {
            assert!(Instant::now() < deadline, "operator selection unavailable");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            !Command::new("/usr/bin/timeout")
                .arg("3")
                .arg(&program)
                .status()
                .unwrap()
                .success()
        );
        assert!(protected.run(&[], Duration::from_secs(3), 1024).unwrap().0);
        assert!(matches!(
            ProtectedExecutable::acquire(&program, &approved_digest),
            Err(ProviderFailure::Rejected(_))
        ));
        println!(
            "execution UID 1: sealed approved image survived operator path replacement; fresh mismatched bytes, writable image, symlink and script refused"
        );
        assert!(matches!(
            loaded.allows(usage),
            Err(Fault::Rejected("collector activation changed"))
        ));
        assert!(matches!(
            activated.run(&[], Duration::from_secs(3), 1024),
            Err(ProviderFailure::Rejected(_))
        ));
        let replacement_verifier =
            acquire("fixture", "collector", warrant, &interface, &program).unwrap();
        assert!(
            !replacement_verifier
                .run(&[], Duration::from_secs(3), 1024)
                .unwrap()
                .0
        );
        let reloaded =
            LoadedEnrollment::load_active(&root.join("store"), "fixture", "collector", &verifier)
                .unwrap();
        let updated = reloaded.enrollment();
        let usage = Use {
            repository: "fixture",
            warrant: updated.warrants.iter().next().unwrap(),
            provider: &updated.provider,
            verifier_digest: &updated.verifier_digest,
            collector: &updated.collector,
        };
        reloaded.allows(Use { ..usage }).unwrap();
        fs::write(scratch.join("selection-checked"), b"checked").unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !root.join("updated").exists() {
            assert!(Instant::now() < deadline, "operator update unavailable");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            matches!(reloaded.allows(usage), Err(Fault::Rejected(_))),
            "revoked authority remained usable"
        );
        assert!(matches!(
            replacement_verifier.run(&[], Duration::from_secs(3), 1024),
            Err(ProviderFailure::Rejected(_))
        ));
        println!(
            "activated verifier: changed selection and signed revocation refused before execution"
        );
        println!(
            "execution UID 1: authority write refused; activated enrollment accepted; inactive enrollment unavailable; changed selection, mismatched UID and signed revocation refused"
        );
        return;
    }
    assert_eq!(role, "operator");
    assert_eq!(rustix::process::geteuid().as_raw(), 0);
    let root = std::env::temp_dir().join(format!("ow-collector-ns-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
    let program = root.join("protected-verifier");
    fs::copy("/usr/bin/true", &program).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let digest = openwarrant_compiler::sha256_hex(&fs::read(&program).unwrap());
    fs::write(root.join("verifier.sha256"), digest).unwrap();
    fs::copy(&program, root.join("writable-verifier")).unwrap();
    fs::set_permissions(
        root.join("writable-verifier"),
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();
    std::os::unix::fs::symlink(&program, root.join("linked-verifier")).unwrap();
    fs::write(root.join("script-verifier"), b"#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(
        root.join("script-verifier"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let scratch = root.join("worker-scratch");
    fs::create_dir(&scratch).unwrap();
    rustix::fs::chown(
        &scratch,
        Some(rustix::process::Uid::from_raw(1)),
        Some(rustix::process::Gid::from_raw(1)),
    )
    .unwrap();
    fs::set_permissions(&scratch, fs::Permissions::from_mode(0o700)).unwrap();
    let no_account = root.join("no-account-scratch");
    fs::create_dir(&no_account).unwrap();
    rustix::fs::chown(
        &no_account,
        Some(rustix::process::Uid::from_raw(2)),
        Some(rustix::process::Gid::from_raw(2)),
    )
    .unwrap();
    fs::set_permissions(&no_account, fs::Permissions::from_mode(0o700)).unwrap();
    let out = Command::new("/usr/bin/timeout")
        .args([
            "10",
            "/usr/bin/setpriv",
            "--reuid",
            "2",
            "--regid",
            "2",
            "--clear-groups",
            "--inh-caps=-all",
            "--ambient-caps=-all",
            "--bounding-set=-all",
            "--no-new-privs",
        ])
        .arg(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &namespace_entry_name(),
            "--ignored",
            "--nocapture",
        ])
        .env_clear()
        .env("OW_COLLECTOR_NAMESPACE_FIXTURE", "no-account")
        .env("TMPDIR", &no_account)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout)
            .contains("missing-account UID 2: verification UNKNOWN")
    );
    println!("{}", String::from_utf8_lossy(&out.stdout));
    for name in [
        "store",
        "mismatched-store",
        "unactivated-store",
        "reader-store",
    ] {
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
    let authority_output = |args: &[&str]| {
        Command::new("/usr/bin/timeout")
            .arg("10")
            .arg(
                std::env::var_os("OW_COLLECTOR_FIXTURE_WAR")
                    .unwrap_or_else(|| env!("CARGO_BIN_EXE_war").into()),
            )
            .args(args)
            .arg("--json")
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("OPENWARRANT_NO_PROJECTS", "1")
            .env("OPENWARRANT_NO_UPDATE_CHECK", "1")
            .output()
            .unwrap()
    };
    let authority_command = |args: &[&str]| {
        let out = authority_output(args);
        assert!(
            out.status.success(),
            "authority command failed: {} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        out
    };
    for (name, uid) in [
        ("store", "1"),
        ("mismatched-store", "2"),
        ("unactivated-store", "1"),
        ("reader-store", "1"),
    ] {
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
        verifier_digest: format!(
            "sha256:{}",
            fs::read_to_string(root.join("verifier.sha256")).unwrap()
        ),
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
    authority_command(&[
        "authority",
        "activate-collector",
        "--store",
        root.join("store").to_str().unwrap(),
        "--enrollment",
        root.join("enrollment.json").to_str().unwrap(),
    ]);
    let repo = open_reader_repository(&root, &root.join("reader-store"), "operator-repository");
    assert!(matches!(
        repo.config.governance,
        openwarrant_core::config::Governance::Store { .. }
    ));
    println!("general authority reader: actual store owner accepted");
    let activated = fs::read(root.join("store/state.json")).unwrap();
    let mut invalid = Signed::decode(&fs::read(root.join("enrollment.json")).unwrap()).unwrap();
    invalid.enrollment.provider.identity = "substituted-provider".into();
    fs::write(
        root.join("invalid-enrollment.json"),
        invalid.encode().unwrap(),
    )
    .unwrap();
    let refused = authority_output(&[
        "authority",
        "activate-collector",
        "--store",
        root.join("store").to_str().unwrap(),
        "--enrollment",
        root.join("invalid-enrollment.json").to_str().unwrap(),
    ]);
    assert!(
        !refused.status.success(),
        "invalid enrollment signature activated"
    );
    assert_eq!(
        fs::read(root.join("store/state.json")).unwrap(),
        activated,
        "refused activation changed retained state"
    );

    let replay = authority_command(&[
        "authority",
        "activate-collector",
        "--store",
        root.join("store").to_str().unwrap(),
        "--enrollment",
        root.join("enrollment.json").to_str().unwrap(),
    ]);
    let replay: serde_json::Value = serde_json::from_slice(&replay.stdout).unwrap();
    assert_eq!(replay["result"]["replay"], true);
    assert_eq!(
        fs::read(root.join("store/state.json")).unwrap(),
        activated,
        "exact activation replay must not change retained state"
    );
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
    let probe = root.join("capability-probe");
    fs::write(&probe, b"readonly operator bytes").unwrap();
    fs::set_permissions(&probe, fs::Permissions::from_mode(0o444)).unwrap();
    let permission_probe = root.join("permission-probe");
    fs::write(&permission_probe, b"readonly operator bytes").unwrap();
    for editor_role in ["permission-editor", "latent-permission-editor"] {
        fs::set_permissions(&permission_probe, fs::Permissions::from_mode(0o444)).unwrap();
        let editor = Command::new("/usr/bin/timeout")
            .args([
                "10",
                "/usr/bin/setpriv",
                "--reuid",
                "1",
                "--regid",
                "1",
                "--clear-groups",
                "--inh-caps=-all,+fowner",
                "--ambient-caps=-all,+fowner",
                "--bounding-set=-all,+fowner",
                "--no-new-privs",
            ])
            .arg(std::env::current_exe().unwrap())
            .env_clear()
            .env("OW_COLLECTOR_NAMESPACE_FIXTURE", editor_role)
            .env("OW_COLLECTOR_FIXTURE_ROOT", &root)
            .env("TMPDIR", &scratch)
            .args([
                "--exact",
                namespace_entry_name().as_str(),
                "--ignored",
                "--nocapture",
            ])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            editor.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&editor.stdout),
            String::from_utf8_lossy(&editor.stderr)
        );
        println!("{}", String::from_utf8_lossy(&editor.stdout));
    }
    let capable = Command::new("/usr/bin/timeout")
        .args([
            "10",
            "/usr/bin/setpriv",
            "--reuid",
            "1",
            "--regid",
            "1",
            "--clear-groups",
            "--inh-caps=-all,+dac_override",
            "--ambient-caps=-all,+dac_override",
            "--bounding-set=-all,+dac_override",
            "--no-new-privs",
        ])
        .arg(std::env::current_exe().unwrap())
        .env_clear()
        .env("OW_COLLECTOR_NAMESPACE_FIXTURE", "capable-executor")
        .env("OW_COLLECTOR_FIXTURE_ROOT", &root)
        .env("TMPDIR", &scratch)
        .args([
            "--exact",
            namespace_entry_name().as_str(),
            "--ignored",
            "--nocapture",
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        capable.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&capable.stdout),
        String::from_utf8_lossy(&capable.stderr)
    );
    assert!(String::from_utf8_lossy(&capable.stdout).contains("capable execution UID 1:"));
    println!("{}", String::from_utf8_lossy(&capable.stdout));
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
    let mut replacement = Signed::decode(&fs::read(root.join("enrollment.json")).unwrap()).unwrap();
    // Replace the operator-owned path after the execution account sealed its
    // approved image. The old image must still execute; a fresh load must fail.
    fs::copy("/usr/bin/false", root.join("replacement-verifier")).unwrap();
    fs::set_permissions(
        root.join("replacement-verifier"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    fs::rename(root.join("replacement-verifier"), &program).unwrap();
    replacement.enrollment.verifier_digest = format!(
        "sha256:{}",
        openwarrant_compiler::sha256_hex(&fs::read(&program).unwrap())
    );
    let (_, signature) = fixture(NAMESPACE, &replacement.enrollment.encode().unwrap(), 11);
    replacement.signatures = BTreeMap::from([("owner".into(), signature)]);
    fs::write(root.join("replacement.json"), replacement.encode().unwrap()).unwrap();
    authority_command(&[
        "authority",
        "activate-collector",
        "--store",
        root.join("store").to_str().unwrap(),
        "--enrollment",
        root.join("replacement.json").to_str().unwrap(),
    ]);
    let retained: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("store/state.json")).unwrap()).unwrap();
    assert_eq!(
        retained["collector_enrollments"].as_object().unwrap().len(),
        2
    );
    assert_eq!(
        retained["collector_activation_events"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    fs::write(root.join("selection-changed"), b"changed").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !scratch.join("selection-checked").exists() {
        assert!(
            child.try_wait().unwrap().is_none(),
            "executor exited before checking selection"
        );
        assert!(
            Instant::now() < deadline,
            "executor selection observation unavailable"
        );
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
