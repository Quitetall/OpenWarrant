// SPDX-License-Identifier: Apache-2.0
use super::*;
use std::{
    collections::BTreeSet,
    process::{Command, Stdio},
};
const NAMESPACE: &str = "openwarrant-authority-v1";
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Result<Self> {
        let p = std::env::temp_dir().join(format!(
            "war-authority-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(err)?
                .as_nanos()
        ));
        let mut b = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            b.mode(0o700);
        }
        b.create(&p).map_err(err)?;
        Ok(Self(p))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
pub(super) fn verify(current: &Revision, record: &Signed) -> Result<()> {
    record.proposal.validate_against(current).map_err(err)?;
    let identities: BTreeSet<String> = record.signatures.keys().cloned().collect();
    sdk::authorize_transition(current, &record.proposal, &identities).map_err(err)?;
    let scratch = Scratch::new()?;
    for (principal, signature) in &record.signatures {
        if signature.len() > 16384 {
            return Err(err("authority-signature-too-large"));
        }
        let entry = &current.principals[principal];
        let allowed = scratch.0.join("allowed");
        let sig = scratch.0.join("signature");
        fs::write(
            &allowed,
            format!(
                "{principal} namespaces=\"{NAMESPACE}\" {}\n",
                entry.public_key
            ),
        )
        .map_err(err)?;
        fs::write(&sig, signature).map_err(err)?;
        let mut child = Command::new("/usr/bin/ssh-keygen")
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LC_ALL", "C")
            .args(["-Y", "verify", "-f"])
            .arg(&allowed)
            .args(["-I", principal, "-n", NAMESPACE, "-s"])
            .arg(&sig)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(err)?;
        child
            .stdin
            .take()
            .ok_or_else(|| err("authority-verify-stdin"))?
            .write_all(&record.proposal.signing_bytes().map_err(err)?)
            .map_err(err)?;
        if !child.wait().map_err(err)?.success() {
            return Err(err("authority-signature-invalid"));
        }
    }
    Ok(())
}
fn admission(current: &Revision, proposal: &Proposal, identities: &BTreeSet<String>) -> Result<()> {
    sdk::authorize_activation(current, proposal, identities).map_err(|e| {
        if e.code == "authority-signer-kind-unknown" {
            RepoError::ObservationUnavailable {
                rule: e.code,
                message: e.message,
            }
        } else {
            err(e)
        }
    })
}
/// Prospective policy. `verify` alone remains the retained-history check.
pub(super) fn verify_activation(current: &Revision, record: &Signed) -> Result<()> {
    admission(
        current,
        &record.proposal,
        &record.signatures.keys().cloned().collect(),
    )?;
    verify(current, record)?;
    if current
        .policy
        .as_ref()
        .is_some_and(|p| p.require_user_presence)
    {
        use openwarrant_core::presence::{FLAG_USER_PRESENCE, parse_armored};
        // Read flags only after ALL supplied signatures have verified.
        let mut unknown = false;
        for signature in record.signatures.values() {
            let parsed = parse_armored(signature).map_err(err)?;
            match parsed.authenticator {
                Some(authenticator) if authenticator.flags & FLAG_USER_PRESENCE == 0 => {
                    return Err(err("authority-user-presence-absent"));
                }
                None => unknown = true,
                Some(_) => {}
            }
        }
        if unknown {
            return Err(RepoError::ObservationUnavailable {
                rule: "authority-user-presence-unknown",
                message: "Authenticated ordinary SSH signatures contain no user-presence evidence"
                    .into(),
            });
        }
    }
    Ok(())
}
pub(super) fn sign(
    current: &Revision,
    p: &Proposal,
    principal: &str,
    key: &Path,
) -> Result<String> {
    admission(current, p, &BTreeSet::from([principal.to_owned()]))?;
    let scratch = Scratch::new()?;
    let subject = scratch.0.join("subject");
    write_new(&subject, &p.signing_bytes().map_err(err)?)?;
    // Use supplied public-key path to ask an SSH agent; a private fixture key also
    // works for tests. Neither this command nor a signature proves human review.
    let result = Command::new("/usr/bin/ssh-keygen")
        .args(["-Y", "sign", "-f"])
        .arg(key)
        .args(["-n", NAMESPACE])
        .arg(&subject)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .map_err(err)?;
    if !result.success() {
        return Err(err("authority-signing-refused"));
    }
    let signature = String::from_utf8(read(&subject.with_extension("sig"))?).map_err(err)?;
    verify_activation(
        current,
        &Signed {
            proposal: p.clone(),
            signatures: BTreeMap::from([(principal.into(), signature.clone())]),
        },
    )?;
    Ok(signature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwarrant_core::{attestation::base64_encode, contract::ActorKind};
    use ring::{
        digest,
        signature::{Ed25519KeyPair, KeyPair},
    };
    fn string(out: &mut Vec<u8>, bytes: &[u8]) {
        out.extend_from_slice(&u32::try_from(bytes.len()).unwrap().to_be_bytes());
        out.extend_from_slice(bytes);
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
            let mut authenticator_message =
                digest::digest(&digest::SHA256, b"ssh:").as_ref().to_vec();
            authenticator_message.push(flags);
            authenticator_message.extend_from_slice(&42u32.to_be_bytes());
            authenticator_message
                .extend_from_slice(digest::digest(&digest::SHA256, &message).as_ref());
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

    fn record(kind: Option<ActorKind>, presence: bool, flags: Option<u8>) -> (Revision, Signed) {
        let seed = 23;
        let public = signed_fixture(NAMESPACE, b"public-only", seed, flags).0;
        let previous = Revision {
            schema: sdk::REVISION_SCHEMA_V2.into(),
            repository: "admission-fixture".into(),
            sequence: 0,
            principals: BTreeMap::from([(
                "owner".into(),
                sdk::Principal {
                    public_key: public,
                    roles: BTreeSet::from(["authority-admin".into(), "authority-recovery".into()]),
                    actor: kind.map(|_| "software fixture actor".into()),
                    kind,
                },
            )]),
            policy: Some(sdk::Policy {
                require_user_presence: presence,
                ..Default::default()
            }),
        };
        let mut next = previous.clone();
        next.sequence = 1;
        next.principals.get_mut("owner").unwrap().kind = Some(ActorKind::Human);
        next.principals.get_mut("owner").unwrap().actor = Some("software fixture actor".into());
        // Proposal cannot relax the policy used to authenticate its own adoption.
        next.policy.as_mut().unwrap().require_user_presence = false;
        let proposal = Proposal {
            schema: sdk::PROPOSAL_SCHEMA.into(),
            operation: Operation::Update,
            previous_digest: previous.digest().unwrap(),
            next,
        };
        let signature =
            signed_fixture(NAMESPACE, &proposal.signing_bytes().unwrap(), seed, flags).1;
        (
            previous,
            Signed {
                proposal,
                signatures: BTreeMap::from([("owner".into(), signature)]),
            },
        )
    }
    #[test]
    fn signed_nonhuman_and_unbound_administrators_cannot_activate_but_history_remains_readable() {
        for kind in [Some(ActorKind::Agent), Some(ActorKind::PolicyService), None] {
            let (current, record) = record(kind, false, None);
            verify(&current, &record).unwrap();
            let e = verify_activation(&current, &record).unwrap_err();
            if kind.is_none() {
                assert!(matches!(
                    e,
                    RepoError::ObservationUnavailable {
                        rule: "authority-signer-kind-unknown",
                        ..
                    }
                ));
            } else {
                assert!(e.to_string().contains("authority-signer-not-human"));
            }
            let scratch = Scratch::new().unwrap();
            store::bootstrap(
                &scratch.0,
                current.clone(),
                &current.digest().unwrap(),
                None,
                None,
                true,
                false,
            )
            .unwrap();
            let state = scratch.0.join("state.json");
            let before = fs::read(&state).unwrap();
            assert!(store::activate(&scratch.0, record, true).is_err());
            assert_eq!(
                fs::read(&state).unwrap(),
                before,
                "rejected activation must not change authority or retained history"
            );
        }
        let (current, record) = record(Some(ActorKind::Human), false, None);
        verify_activation(&current, &record).unwrap();
        let scratch = Scratch::new().unwrap();
        store::bootstrap(
            &scratch.0,
            current.clone(),
            &current.digest().unwrap(),
            None,
            None,
            true,
            false,
        )
        .unwrap();
        let view = store::activate(&scratch.0, record, true).unwrap();
        assert_eq!(view["current"]["sequence"], 1);
        assert_eq!(view["human_review_established"], false);
    }
    #[test]
    fn current_presence_policy_controls_activation_after_cryptography_without_inferring_a_human_review()
     {
        for flags in [None, Some(0), Some(1)] {
            let (current, record) = record(Some(ActorKind::Human), true, flags);
            verify(&current, &record).unwrap();
            match flags {
                None => assert!(matches!(
                    verify_activation(&current, &record).unwrap_err(),
                    RepoError::ObservationUnavailable {
                        rule: "authority-user-presence-unknown",
                        ..
                    }
                )),
                Some(0) => assert!(
                    verify_activation(&current, &record)
                        .unwrap_err()
                        .to_string()
                        .contains("authority-user-presence-absent")
                ),
                Some(_) => verify_activation(&current, &record).unwrap(),
            }
            let mut tampered = record.clone();
            tampered
                .proposal
                .next
                .policy
                .as_mut()
                .unwrap()
                .allow_automated_resolution = true;
            assert!(
                verify_activation(&current, &tampered)
                    .unwrap_err()
                    .to_string()
                    .contains("authority-signature-invalid")
            );
            if flags != Some(1) {
                let scratch = Scratch::new().unwrap();
                store::bootstrap(
                    &scratch.0,
                    current.clone(),
                    &current.digest().unwrap(),
                    None,
                    None,
                    true,
                    false,
                )
                .unwrap();
                let state = scratch.0.join("state.json");
                let before = fs::read(&state).unwrap();
                assert!(store::activate(&scratch.0, record, true).is_err());
                assert_eq!(fs::read(&state).unwrap(), before);
            }
        }
    }
}
