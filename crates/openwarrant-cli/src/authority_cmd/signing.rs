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
pub(super) fn sign(
    current: &Revision,
    p: &Proposal,
    principal: &str,
    key: &Path,
) -> Result<String> {
    sdk::authorize_transition(current, p, &BTreeSet::from([principal.to_owned()])).map_err(err)?;
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
    verify(
        current,
        &Signed {
            proposal: p.clone(),
            signatures: BTreeMap::from([(principal.into(), signature.clone())]),
        },
    )?;
    Ok(signature)
}
