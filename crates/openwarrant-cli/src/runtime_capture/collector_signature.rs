// SPDX-License-Identifier: Apache-2.0
//! Reference cryptographic adapter, not enrollment activation or human review.
//! The caller still supplies protected current authority and executable custody.
use openwarrant_core::{
    document::runtime::ProviderFailure,
    presence::{self, FLAG_USER_PRESENCE},
    runtime_collector::{Fault, NAMESPACE, SignatureCheck, SignatureEvidence},
};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub struct OpenSshSignatureCheck {
    scratch_root: PathBuf,
    timeout: Duration,
}
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
impl OpenSshSignatureCheck {
    pub fn new(scratch_root: PathBuf, timeout: Duration) -> Result<Self, Fault> {
        if !scratch_root.is_absolute() || timeout.is_zero() || timeout > Duration::from_secs(30) {
            return Err(Fault::Rejected("scratch path or deadline"));
        }
        Ok(Self {
            scratch_root,
            timeout,
        })
    }
    fn scratch(&self) -> Result<Scratch, Fault> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Fault::Unavailable("scratch clock"))?
            .as_nanos();
        let path = self
            .scratch_root
            .join(format!("collector-ssh-{}-{stamp}", std::process::id()));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(&path)
            .map_err(|_| Fault::Unavailable("private verification scratch"))?;
        Ok(Scratch(path))
    }
}
impl SignatureCheck for OpenSshSignatureCheck {
    fn verify(
        &self,
        public_key: &str,
        namespace: &str,
        payload: &[u8],
        signature: &str,
    ) -> Result<SignatureEvidence, Fault> {
        if namespace != NAMESPACE {
            return Err(Fault::Rejected("collector signature namespace"));
        }
        self.verify_exact(public_key, namespace, payload, signature)
    }
}
impl OpenSshSignatureCheck {
    /// Verify exact native acceptance bytes, using a key obtained independently
    /// from a protected human grant. This method does not itself grant a role.
    pub(crate) fn verify_response(
        &self,
        public_key: &str,
        payload: &[u8],
        signature: &str,
    ) -> Result<SignatureEvidence, Fault> {
        self.verify_exact(public_key, crate::sign::SSH_NAMESPACE, payload, signature)
    }
    fn verify_exact(
        &self,
        public_key: &str,
        namespace: &str,
        payload: &[u8],
        signature: &str,
    ) -> Result<SignatureEvidence, Fault> {
        if payload.len() > 65_536 || signature.len() > 16_384 || public_key.len() > 16_384 {
            return Err(Fault::Rejected("signature input bounds or namespace"));
        }
        let fields: Vec<_> = public_key.split(' ').collect();
        if fields.len() != 2
            || fields
                .iter()
                .any(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_graphic()))
        {
            return Err(Fault::Rejected("single exact public key required"));
        }
        let parsed = presence::parse_armored(signature)
            .map_err(|_| Fault::Rejected("signature structure"))?;
        let key = presence::public_key_blob(fields[0], fields[1])
            .map_err(|_| Fault::Rejected("public key structure"))?;
        if parsed.namespace != namespace || parsed.public_key != key {
            return Err(Fault::Rejected("signature key or namespace mismatch"));
        }
        let scratch = self.scratch()?;
        let allowed = scratch.0.join("allowed");
        let sig = scratch.0.join("signature");
        let input = scratch.0.join("payload");
        fs::write(
            &allowed,
            format!("collector namespaces=\"{namespace}\" {public_key}\n"),
        )
        .and_then(|_| fs::write(&sig, signature))
        .and_then(|_| fs::write(&input, payload))
        .map_err(|_| Fault::Unavailable("verification scratch write"))?;
        let file = fs::File::open(&input).map_err(|_| Fault::Unavailable("verification input"))?;
        let mut command = Command::new("/usr/bin/ssh-keygen");
        command
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LC_ALL", "C")
            .args(["-Y", "verify", "-f"])
            .arg(&allowed)
            .args(["-I", "collector", "-n", namespace, "-s"])
            .arg(&sig);
        let observed =
            super::process::observe_with_stdin(command, self.timeout, 16_384, Stdio::from(file))
                .map_err(|e| match e {
                    ProviderFailure::Rejected(_) => {
                        Fault::Rejected("signature verifier output budget")
                    }
                    _ => Fault::Unavailable("signature verifier unavailable or deadline exceeded"),
                })?;
        let valid = cryptographic_verdict(&observed)?;
        // Only cryptographically accepted flags become observations. Ordinary
        // key confirmation dialogs leave no authenticated presence bit.
        let user_present = if valid {
            parsed
                .authenticator
                .map(|a| a.flags & FLAG_USER_PRESENCE != 0)
        } else {
            None
        };
        Ok(SignatureEvidence {
            valid,
            user_present,
        })
    }
}

// LC_ALL=C is fixed above. Only an explicit cryptographic refusal establishes
// invalidity; missing accounts, unsupported tools, loader failures or unfamiliar
// failures are unavailable observations, never invented invalid signatures.
fn cryptographic_verdict(observed: &super::process::Completed) -> Result<bool, Fault> {
    if observed.success {
        return Ok(true);
    }
    let error = String::from_utf8_lossy(&observed.stderr);
    if error
        .lines()
        .any(|line| line == "Signature verification failed: incorrect signature")
    {
        return Ok(false);
    }
    Err(Fault::Unavailable(
        "signature verifier produced no recognized cryptographic verdict",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn response_transport_verifies_exact_bytes_and_keeps_collector_namespace_separate() {
        use openwarrant_core::attestation::base64_encode;
        use ring::{
            digest,
            signature::{Ed25519KeyPair, KeyPair},
        };
        fn string(out: &mut Vec<u8>, value: &[u8]) {
            out.extend_from_slice(&u32::try_from(value.len()).unwrap().to_be_bytes());
            out.extend_from_slice(value);
        }
        // Software fixture key in memory; no human acceptance or presence.
        let key = Ed25519KeyPair::from_seed_unchecked(&[29; 32]).unwrap();
        let mut public = Vec::new();
        string(&mut public, b"ssh-ed25519");
        string(&mut public, key.public_key().as_ref());
        let payload = b"exact synthetic native acceptance";
        let mut preimage = b"SSHSIG".to_vec();
        string(&mut preimage, crate::sign::SSH_NAMESPACE.as_bytes());
        string(&mut preimage, b"");
        string(&mut preimage, b"sha512");
        string(
            &mut preimage,
            digest::digest(&digest::SHA512, payload).as_ref(),
        );
        let mut signature = Vec::new();
        string(&mut signature, b"ssh-ed25519");
        string(&mut signature, key.sign(&preimage).as_ref());
        let mut blob = b"SSHSIG".to_vec();
        blob.extend_from_slice(&1u32.to_be_bytes());
        string(&mut blob, &public);
        string(&mut blob, crate::sign::SSH_NAMESPACE.as_bytes());
        string(&mut blob, b"");
        string(&mut blob, b"sha512");
        string(&mut blob, &signature);
        let signature = format!(
            "-----BEGIN SSH SIGNATURE-----\n{}\n-----END SSH SIGNATURE-----\n",
            base64_encode(&blob)
        );
        let public = format!("ssh-ed25519 {}", base64_encode(&public));
        let root = std::env::temp_dir().join(format!(
            "ow-native-response-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        let check = OpenSshSignatureCheck::new(root, Duration::from_secs(10)).unwrap();
        let observed = check.verify_response(&public, payload, &signature).unwrap();
        assert!(observed.valid);
        assert_eq!(observed.user_present, None);
        assert!(
            !check
                .verify_response(&public, b"changed", &signature)
                .unwrap()
                .valid
        );
        assert!(matches!(
            check.verify(&public, NAMESPACE, payload, &signature),
            Err(Fault::Rejected(_))
        ));
        assert!(matches!(
            check.verify(&public, crate::sign::SSH_NAMESPACE, payload, &signature),
            Err(Fault::Rejected(_))
        ));
    }
    #[test]
    fn unsuccessful_processes_need_a_cryptographic_verdict() {
        for error in [
            "No user exists for uid 2",
            "error while loading shared libraries",
            "usage: ssh-keygen",
            "Signature verification failed: error in libcrypto",
            "",
        ] {
            assert!(matches!(
                cryptographic_verdict(&super::super::process::Completed {
                    success: false,
                    stdout: Vec::new(),
                    stderr: error.as_bytes().to_vec(),
                }),
                Err(Fault::Unavailable(_))
            ));
        }
        assert!(
            !cryptographic_verdict(&super::super::process::Completed {
                success: false,
                stdout: b"Could not verify signature.\n".to_vec(),
                stderr: b"Signature verification failed: incorrect signature\n".to_vec(),
            })
            .unwrap()
        );
    }
}
