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
        if namespace != NAMESPACE
            || payload.len() > 65_536
            || signature.len() > 16_384
            || public_key.len() > 16_384
        {
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
            format!("collector namespaces=\"{NAMESPACE}\" {public_key}\n"),
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
            .args(["-I", "collector", "-n", NAMESPACE, "-s"])
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
