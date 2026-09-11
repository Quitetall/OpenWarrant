// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war attest` — attestations for ssh-signed acts (OW-ADR-0015).
//!
//! After `war sign --ssh-sign` ingests an authorization, resolution,
//! correction or SAS acceptance, an in-toto Statement about the records that
//! act wrote is signed as a DSSE envelope with the same key, under the
//! `oh.war/dsse` namespace, and written beside the records:
//! `docs/warrants/<alias>/attestations/<act>-<n>.dsse.json`, or
//! `docs/sas/revisions/attestations/sas-accept-<version>-<n>.dsse.json`.
//!
//! Only ssh-signed acts are attested: a TTY signature has no key. Acts made
//! before this existed are unattested; git is their witness, and `war attest
//! list` says so rather than pretending.
//!
//! Verification (`war attest verify`) rebuilds the PAE from the payload,
//! re-armors the SSHSIG blob, and hands both to `ssh-keygen -Y verify`
//! against the human-written `allowed_signers`, with the principal taken from
//! the predicate's actor through `roles.toml` — never from a flag. It also
//! recomputes every subject digest against the file on disk today, so a
//! record edited after its attestation is `attest.subject-drift`.
//! `war check` stays deterministic and structural; signature verification is
//! this command and one xtask step.

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_core::attestation::{
    Envelope, PAYLOAD_TYPE, SSH_NAMESPACE, Signature, Statement, Subject, armor, base64_decode,
    base64_encode, dearmor, pae,
};
use sha2::{Digest, Sha256};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

/// What one attestation is about, gathered by the act that made it.
pub struct Attestable<'a> {
    /// `authorize` | `resolve` | `correct` | `sas-accept`.
    pub act: &'a str,
    /// The Warrant alias, or the SAS version for `sas-accept`.
    pub target: &'a str,
    /// Repository-relative files the Statement's subjects name, in order:
    /// the record the act wrote first, the signed response second.
    pub files: Vec<Utf8PathBuf>,
    /// Extra named digests (`contract:<alias>`) with a hex sha256.
    pub extra_subjects: Vec<(String, String)>,
    /// The record's own fields, as the predicate.
    pub predicate: serde_json::Value,
    /// The human who signed (an `actor` in roles.toml).
    pub actor: &'a str,
}

/// A private scratch directory for one ssh-keygen exchange: created 0700 with
/// a per-call name (pid + nanoseconds), removed on drop. Predictable names
/// under a shared /tmp are a symlink-following risk on the write and a race
/// on the `.sig` ssh-keygen writes beside its input; a directory nobody else
/// can enter closes both.
struct Scratch(Utf8PathBuf);

impl Scratch {
    fn new(tag: &str) -> Result<Self, String> {
        // A clock before the epoch would degrade the name to a predictable
        // one; refuse rather than sign in that environment.
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("system clock is before the Unix epoch: {e}"))?
            .as_nanos();
        let dir = Utf8PathBuf::from(format!(
            "{}/war-attest-{tag}-{}-{stamp}",
            std::env::temp_dir().display(),
            std::process::id()
        ));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(&dir)
            .map_err(|e| format!("could not create scratch directory {dir}: {e}"))?;
        Ok(Self(dir))
    }

    fn file(&self, name: &str) -> Utf8PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Where attestations for a target live.
pub fn dir_for(repo: &Repository, act: &str, target: &str) -> Result<Utf8PathBuf, RepoError> {
    if act == "sas-accept" {
        Ok(repo
            .root
            .join(&repo.config.paths.sas)
            .join("revisions")
            .join("attestations"))
    } else {
        Ok(repo.warrant_dir(target)?.join("attestations"))
    }
}

fn next_name(dir: &Utf8Path, stem: &str) -> Result<Utf8PathBuf, RepoError> {
    // Bounded: ten thousand attestations for one act is a defect to report,
    // not a loop to spin in.
    for n in 1u32..=10_000 {
        let candidate = dir.join(format!("{stem}-{n}.dsse.json"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(RepoError::Message(format!(
        "{dir}: more than 10000 {stem}-N.dsse.json files; refusing to allocate another"
    )))
}

/// `SHA256:<base64>` of the principal's public key, as `ssh-keygen -lf`
/// prints it — the DSSE `keyid`.
fn keyid_of(pubkey_line: &str) -> Result<String, String> {
    let scratch = Scratch::new("keyid")?;
    let tmp = scratch.file("key.pub");
    std::fs::write(&tmp, format!("{pubkey_line}\n"))
        .map_err(|e| format!("could not write {tmp}: {e}"))?;
    let out = std::process::Command::new("ssh-keygen")
        .args(["-lf"])
        .arg(&tmp)
        .args(["-E", "sha256"])
        .output()
        .map_err(|e| format!("could not run ssh-keygen -lf: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "ssh-keygen -lf refused: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    text.split_whitespace()
        .find(|w| w.starts_with("SHA256:"))
        .map(str::to_owned)
        .ok_or_else(|| format!("no SHA256 fingerprint in {text:?}"))
}

/// Sign `pae_bytes` under `oh.war/dsse` with `key` (a public key line, which
/// routes through the agent — or a private key file, which signs directly;
/// tests use the latter). Returns the base64 SSHSIG blob.
fn sign_pae(key_file: &Utf8Path, pae_bytes: &[u8]) -> Result<String, String> {
    let scratch = Scratch::new("sign")?;
    let tmp = scratch.file("statement.pae");
    std::fs::write(&tmp, pae_bytes).map_err(|e| format!("could not write {tmp}: {e}"))?;
    let out = std::process::Command::new("ssh-keygen")
        .args(["-Y", "sign", "-f"])
        .arg(key_file)
        .args(["-n", SSH_NAMESPACE])
        .arg(&tmp)
        .output()
        .map_err(|e| format!("could not run ssh-keygen -Y sign: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "ssh-keygen -Y sign refused ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let armored = std::fs::read_to_string(format!("{tmp}.sig"))
        .map_err(|e| format!("ssh-keygen wrote no signature: {e}"))?;
    dearmor(&armored).map_err(|e| e.to_string())
}

/// `ssh-keygen -Y verify` of a DSSE signature: PAE on stdin, re-armored blob
/// as the signature file, principal from the register.
fn verify_pae(
    allowed_signers: &Utf8Path,
    principal: &str,
    pae_bytes: &[u8],
    sig_blob_base64: &str,
) -> Result<(), String> {
    let scratch = Scratch::new("verify")?;
    let pae_file = scratch.file("statement.pae");
    let sig_file = scratch.file("statement.sig");
    std::fs::write(&pae_file, pae_bytes).map_err(|e| format!("could not write {pae_file}: {e}"))?;
    std::fs::write(&sig_file, armor(sig_blob_base64))
        .map_err(|e| format!("could not write {sig_file}: {e}"))?;
    let input = std::fs::File::open(&pae_file).map_err(|e| e.to_string())?;
    let out = std::process::Command::new("ssh-keygen")
        .args(["-Y", "verify", "-f"])
        .arg(allowed_signers)
        .args(["-I", principal, "-n", SSH_NAMESPACE, "-s"])
        .arg(&sig_file)
        .stdin(input)
        .output()
        .map_err(|e| format!("could not run ssh-keygen -Y verify: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "does not verify as {principal} under {SSH_NAMESPACE} ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// Build the Statement for an act: subjects are the files' sha256 today.
fn statement_for(
    repo: &Repository,
    a: &Attestable<'_>,
) -> Result<Statement<serde_json::Value>, RepoError> {
    let mut subjects = Vec::new();
    for rel in &a.files {
        let bytes = std::fs::read(repo.root.join(rel)).map_err(|source| RepoError::Io {
            context: format!("could not read {rel} to attest it"),
            source,
        })?;
        subjects.push(Subject::sha256(rel.as_str(), sha256_hex(&bytes)));
    }
    for (name, hex) in &a.extra_subjects {
        subjects.push(Subject::sha256(name.clone(), hex.clone()));
    }
    Ok(Statement::new(a.act, subjects, a.predicate.clone()))
}

/// Emit one attestation. `key_file` is the public key line's file for the
/// agent path (what `war sign` passes) or a private key (tests).
pub fn emit_with_key(
    repo: &Repository,
    a: &Attestable<'_>,
    key_file: &Utf8Path,
    keyid: &str,
) -> Result<Utf8PathBuf, RepoError> {
    let statement = statement_for(repo, a)?;
    let payload = serde_jcs::to_string(&statement)
        .map_err(|e| RepoError::Message(format!("could not canonicalise the statement: {e}")))?;
    let pae_bytes = pae(PAYLOAD_TYPE, payload.as_bytes());
    let sig = sign_pae(key_file, &pae_bytes)
        .map_err(|why| RepoError::Message(format!("attest.not-signed: {why}")))?;
    let envelope = Envelope {
        payload_type: PAYLOAD_TYPE.to_owned(),
        payload: base64_encode(payload.as_bytes()),
        signatures: vec![Signature {
            keyid: keyid.to_owned(),
            sig,
        }],
    };
    let dir = dir_for(repo, a.act, a.target)?;
    std::fs::create_dir_all(&dir).map_err(|source| RepoError::Io {
        context: format!("could not create {dir}"),
        source,
    })?;
    let stem = if a.act == "sas-accept" {
        format!("sas-accept-{}", a.target)
    } else {
        a.act.to_owned()
    };
    let path = next_name(&dir, &stem)?;
    let text = serde_json::to_string_pretty(&envelope).unwrap_or_default() + "\n";
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, text.as_bytes()))
        .map_err(|source| RepoError::Io {
            context: format!("could not write {path}"),
            source,
        })?;
    Ok(path)
}

/// Emit after an ssh-signed act: the key is the principal's public key from
/// `allowed_signers` (so `ssh-keygen` asks the agent, and the agent asks the
/// human once more). Returns the attestation path.
pub fn emit(repo: &Repository, a: &Attestable<'_>) -> Result<Utf8PathBuf, RepoError> {
    let allowed = crate::sign::allowed_signers_path(repo);
    let text = std::fs::read_to_string(&allowed)
        .map_err(|e| RepoError::Message(format!("attest.not-signed: {allowed}: {e}")))?;
    let principal = crate::sign::principal_of(repo, a.actor)
        .map_err(|why| RepoError::Message(format!("attest.not-signed: {why}")))?;
    let pubkey = crate::sign::pubkey_for_principal(&text, &principal)
        .map_err(|why| RepoError::Message(format!("attest.not-signed: {allowed}: {why}")))?;
    let keyid =
        keyid_of(&pubkey).map_err(|why| RepoError::Message(format!("attest.not-signed: {why}")))?;
    let scratch = Scratch::new("emit").map_err(RepoError::Message)?;
    let pub_path = scratch.file("signer.pub");
    std::fs::write(&pub_path, format!("{pubkey} {principal}\n")).map_err(|source| {
        RepoError::Io {
            context: format!("could not write {pub_path}"),
            source,
        }
    })?;
    emit_with_key(repo, a, &pub_path, &keyid)
}

/// Every attestation file for a target, sorted.
pub fn list(repo: &Repository, act_dir: &Utf8Path) -> Vec<Utf8PathBuf> {
    let _ = repo;
    let mut out: Vec<Utf8PathBuf> = std::fs::read_dir(act_dir)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
                .filter(|p| p.as_str().ends_with(".dsse.json"))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// Verify one attestation file: structure, subject digests today, signature.
pub fn verify_file(repo: &Repository, path: &Utf8Path, report: &mut Report) {
    let rel = repo.relative(path);
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            report.push(Diagnostic::error("attest.unreadable", rel, e.to_string()));
            return;
        }
    };
    let envelope: Envelope = match serde_json::from_str(&text) {
        Ok(e) => e,
        Err(e) => {
            report.push(Diagnostic::error(
                "attest.malformed",
                rel,
                format!("not a DSSE envelope: {e}"),
            ));
            return;
        }
    };
    let payload = match envelope.open() {
        Ok(v) => v,
        Err(e) => {
            report.push(Diagnostic::error("attest.malformed", rel, e.to_string()));
            return;
        }
    };
    let statement: Statement<serde_json::Value> = match serde_json::from_slice(&payload) {
        Ok(s) => s,
        Err(e) => {
            report.push(Diagnostic::error(
                "attest.malformed",
                rel,
                format!("payload is not an in-toto Statement: {e}"),
            ));
            return;
        }
    };
    if let Err(e) = statement.check_type() {
        report.push(Diagnostic::error("attest.malformed", rel, e.to_string()));
        return;
    }
    let Some(act) = statement.act() else {
        report.push(Diagnostic::error(
            "attest.malformed",
            rel,
            format!(
                "predicateType {:?} is not one of ours",
                statement.predicate_type
            ),
        ));
        return;
    };
    // Subjects: every file named must hash to what the statement says.
    let mut drifted = false;
    for s in &statement.subject {
        if s.name.contains(':') && !s.name.contains('/') {
            continue; // `contract:<alias>` — not a file
        }
        let Some(want) = s.digest.get("sha256") else {
            report.push(Diagnostic::error(
                "attest.malformed",
                rel.clone(),
                format!("subject {} carries no sha256", s.name),
            ));
            drifted = true;
            continue;
        };
        match std::fs::read(repo.root.join(&s.name)) {
            Ok(bytes) if sha256_hex(&bytes) == *want => {}
            Ok(_) => {
                drifted = true;
                report.push(Diagnostic::error(
                    "attest.subject-drift",
                    rel.clone(),
                    format!(
                        "{} no longer matches the digest attested for it (sha256:{want}); the \
                         record was edited after the {act} was attested",
                        s.name
                    ),
                ));
            }
            Err(e) => {
                drifted = true;
                report.push(Diagnostic::error(
                    "attest.subject-drift",
                    rel.clone(),
                    format!("{}: {e}", s.name),
                ));
            }
        }
    }
    // Signature: principal from the predicate's actor, through the register.
    let actor = statement
        .predicate
        .get("actor")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let principal = match crate::sign::principal_of(repo, actor) {
        Ok(p) => p,
        Err(why) => {
            report.push(Diagnostic::error(
                "attest.unknown-principal",
                rel,
                format!("predicate actor {actor:?}: {why}"),
            ));
            return;
        }
    };
    let sig = &envelope.signatures[0];
    if base64_decode(&sig.sig).is_err() {
        report.push(Diagnostic::error(
            "attest.malformed",
            rel,
            "signature is not base64".to_owned(),
        ));
        return;
    }
    if which_ssh_keygen().is_none() {
        report.push(Diagnostic::unknown(
            "attest.unchecked",
            rel,
            "ssh-keygen is not on PATH; the signature was not verified".to_owned(),
        ));
        return;
    }
    let pae_bytes = pae(PAYLOAD_TYPE, &payload);
    match verify_pae(
        &crate::sign::allowed_signers_path(repo),
        &principal,
        &pae_bytes,
        &sig.sig,
    ) {
        Ok(()) if !drifted => report.push(Diagnostic::pass(
            "attest.verified",
            format!(
                "{rel}: {act} attested by {principal} ({}), {} subject(s) intact",
                sig.keyid,
                statement.subject.len()
            ),
        )),
        Ok(()) => report.push(Diagnostic::pass(
            "attest.signature-verified",
            format!("{rel}: signature verifies as {principal}, but a subject drifted (above)"),
        )),
        Err(why) => report.push(Diagnostic::error("attest.failed", rel, why)),
    }
}

fn which_ssh_keygen() -> Option<()> {
    std::process::Command::new("ssh-keygen")
        .arg("-?")
        .output()
        .ok()
        .map(|_| ())
}

/// `war attest <alias> --verify` / `--list`.
pub fn run(repo: &Repository, target: &str, verify: bool) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let (dir, what) = if repo.warrant_dir(target).is_ok() {
        (dir_for(repo, "authorize", target)?, "Warrant")
    } else {
        (dir_for(repo, "sas-accept", target)?, "SAS")
    };
    let files: Vec<Utf8PathBuf> = list(repo, &dir)
        .into_iter()
        .filter(|p| what == "Warrant" || p.as_str().contains(&format!("sas-accept-{target}-")))
        .collect();
    if files.is_empty() {
        report.push(Diagnostic::warn(
            "attest.none",
            target.to_owned(),
            format!(
                "{what} {target} carries no attestation: its acts were tty-signed or made \
                 before attestations existed; git is their witness"
            ),
        ));
        return Ok(report);
    }
    for path in &files {
        if verify {
            verify_file(repo, path, &mut report);
        } else {
            report.push(Diagnostic::pass(
                "attest.listed",
                repo.relative(path).to_string(),
            ));
        }
    }
    Ok(report)
}

/// Verify every attestation in the repository (the xtask step).
pub fn verify_all(repo: &Repository) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let mut count = 0usize;
    for dir in repo.warrant_dirs()? {
        for path in list(repo, &dir.join("attestations")) {
            verify_file(repo, &path, &mut report);
            count += 1;
        }
    }
    for path in list(repo, &dir_for(repo, "sas-accept", "")?) {
        verify_file(repo, &path, &mut report);
        count += 1;
    }
    report.push(Diagnostic::pass(
        "attest.checked",
        format!("{count} attestation(s) checked"),
    ));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> Utf8PathBuf {
        let dir = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!("war-attest-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn keygen(dir: &Utf8Path) -> (Utf8PathBuf, String) {
        let key = dir.join("id_test");
        let s = std::process::Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-C", "test", "-f"])
            .arg(&key)
            .status()
            .unwrap();
        assert!(s.success());
        let pubkey = std::fs::read_to_string(format!("{key}.pub")).unwrap();
        let pubkey = pubkey
            .split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(" ");
        (key, pubkey)
    }

    #[test]
    fn a_dsse_signature_verifies_only_under_its_namespace_and_not_after_a_byte_moves() {
        let dir = scratch("dsse");
        let (key, pubkey) = keygen(&dir);
        let allowed = dir.join("allowed_signers");
        std::fs::write(
            &allowed,
            format!("tester namespaces=\"oh.war/response,oh.war/dsse\" {pubkey}\n"),
        )
        .unwrap();
        let statement = Statement::new(
            "authorize",
            vec![Subject::sha256("a.toml", "ab".repeat(32))],
            serde_json::json!({"actor": "tester"}),
        );
        let payload = serde_jcs::to_string(&statement).unwrap();
        let pae_bytes = pae(PAYLOAD_TYPE, payload.as_bytes());
        let sig = sign_pae(&key, &pae_bytes).expect("signs");
        verify_pae(&allowed, "tester", &pae_bytes, &sig).expect("verifies");
        // One byte of payload: refused.
        let mut moved = pae_bytes.clone();
        let last = moved.len() - 1;
        moved[last] ^= 1;
        assert!(verify_pae(&allowed, "tester", &moved, &sig).is_err());
        // Unknown principal: refused.
        assert!(verify_pae(&allowed, "someone", &pae_bytes, &sig).is_err());
        // A signature made under the RESPONSE namespace does not verify as DSSE.
        let file = dir.join("resp.toml");
        std::fs::write(&file, &pae_bytes).unwrap();
        let s = std::process::Command::new("ssh-keygen")
            .args(["-Y", "sign", "-f"])
            .arg(&key)
            .args(["-n", crate::sign::SSH_NAMESPACE])
            .arg(&file)
            .status()
            .unwrap();
        assert!(s.success());
        let replay = dearmor(&std::fs::read_to_string(format!("{file}.sig")).unwrap()).unwrap();
        assert!(
            verify_pae(&allowed, "tester", &pae_bytes, &replay).is_err(),
            "a response-namespace signature replayed as DSSE must not verify"
        );
        // keyid is the OpenSSH SHA256 fingerprint.
        assert!(keyid_of(&pubkey).unwrap().starts_with("SHA256:"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
