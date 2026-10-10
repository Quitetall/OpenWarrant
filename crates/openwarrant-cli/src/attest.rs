// SPDX-License-Identifier: Apache-2.0
//! `war sign attest` — attestations for ssh-signed acts (OW-ADR-0015).
//!
//! After `war sign --ssh-sign` ingests an authorization, resolution,
//! correction or SAS acceptance, an in-toto Statement about the records that
//! act wrote is signed as a DSSE envelope with the same key, under the
//! `oh.war/dsse` namespace, and written beside the records:
//! `docs/warrants/<alias>/attestations/<act>-<n>.dsse.json`, or
//! `docs/sas/revisions/attestations/sas-accept-<version>-<n>.dsse.json`.
//!
//! Only ssh-signed acts are attested: a TTY signature has no key. Acts made
//! before this existed are unattested; git is their witness, and `war sign attest
//! list` says so rather than pretending.
//!
//! Verification (`war sign attest verify`) rebuilds the PAE from the payload,
//! re-armors the SSHSIG blob, and hands both to `ssh-keygen -Y verify`
//! against the human-written `allowed_signers`, with the principal taken from
//! the predicate's actor through `roles.toml` — never from a flag. It also
//! recomputes every subject digest against the file on disk today, so a
//! record edited after its attestation is `attest.subject-drift`.
//! `war check` stays deterministic and structural; signature verification is
//! this command and one xtask step.
//!
//! # Custody (OW-WAR-0136, §41.5)
//!
//! A resolve attestation also names, by digest, every receipt the resolution
//! relied on (`gate_run_refs`) and that receipt's run, stdout and stderr. A
//! receipt's own seal covers only its own fields, so a receipt replaced by a
//! freshly minted one reseals and passes every check that reads the seal; the
//! attested digest is what catches it. `war sign attest --custody <alias>` reports
//! each §41.5 field per relied-on receipt as present or `UNKNOWN` with a
//! reason, fails on any subject that moved (`attest.custody-drift`), and with
//! `--record <auditor>` writes `custody-audit.toml` — refused to the
//! performer. Resolutions attested before this carry no receipt subjects:
//! their original digest is `UNKNOWN (not attested)`, and only the receipt's
//! own seal is compared.

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::digest::sha256_hex;
use openwarrant_core::attestation::{
    Envelope, PAYLOAD_TYPE, SSH_NAMESPACE, Signature, Statement, Subject, armor, base64_decode,
    base64_encode, dearmor, pae,
};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

/// What one attestation is about, gathered by the act that made it.
pub struct Attestable<'a> {
    /// `authorize` | `resolve` | `correct` | `sas-accept`.
    pub act: &'a str,
    /// The Warrant alias, or the SAS version for `sas-accept`.
    pub target: &'a str,
    /// Repository-relative files the Statement's subjects name, in order:
    /// the record the act wrote first, the signed response second, and for a
    /// `resolve` the evidence it relied on after them ([`relied_on_files`]).
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

/// Where attestations for a target live.
pub fn dir_for(repo: &Repository, act: &str, target: &str) -> Result<Utf8PathBuf, RepoError> {
    if act == "sas-accept" {
        Ok(repo
            .root
            .join(&repo.config.paths.sas)
            .join("revisions")
            .join("attestations"))
    } else if act == "roadmap-accept" {
        Ok(crate::roadmap_cmd::dir(repo)
            .join("revisions")
            .join("attestations"))
    } else if act == "batch" {
        Ok(repo
            .root
            .join(crate::batch_cmd::BATCHES)
            .join("attestations"))
    } else if act == "invalidate" {
        // OW-WAR-0136: a gate's invalidation has no Warrant; it is attested
        // beside its record.
        Ok(repo
            .root
            .join(&repo.config.paths.gates)
            .join(crate::invalidation::RECORDS_DIR)
            .join("attestations"))
    } else if act == "standing-accept" || act == "standing-revoke" {
        // OW-ADR-0029: a class belongs to no Warrant; its acts are attested
        // beside the class files.
        Ok(repo
            .root
            .join(crate::standing_cmd::DIR)
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
    // Where it will live is settled before the key is asked: a target with no
    // home fails here, not after the human has answered a dialog for nothing.
    let dir = dir_for(repo, a.act, a.target)?;
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
    std::fs::create_dir_all(&dir).map_err(|source| RepoError::Io {
        context: format!("could not create {dir}"),
        source,
    })?;
    let stem = if matches!(
        a.act,
        "sas-accept"
            | "roadmap-accept"
            | "batch"
            | "invalidate"
            | "standing-accept"
            | "standing-revoke"
    ) {
        format!("{}-{}", a.act, a.target)
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
    let signer = crate::authority_check::signer_for(repo, a.actor, &[])
        .map_err(|(rule, why)| RepoError::Message(format!("attest.not-signed: {rule}: {why}")))?;
    let allowed = &signer.allowed;
    let text = std::fs::read_to_string(allowed)
        .map_err(|e| RepoError::Message(format!("attest.not-signed: {allowed}: {e}")))?;
    let principal = signer.principal.clone();
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

/// The attested bytes, if a sibling of `name` still carries them.
///
/// `retire_prior` moves a response aside as `<stem>.<tag>.response.toml` when a
/// later act of the same kind takes its path, so an attestation naming the
/// original path is describing bytes that still exist under a different name.
/// Only siblings sharing the original stem are considered: a digest match
/// anywhere in the repository would be a different claim.
fn archived_copy(repo: &Repository, name: &str, want: &str) -> Option<String> {
    let path = camino::Utf8PathBuf::from(name);
    let dir = repo.root.join(path.parent()?);
    let file = path.file_name()?;
    let stem = file.split('.').next()?;
    let mut found: Vec<String> = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|e| camino::Utf8PathBuf::from_path_buf(e.path()).ok())
        .filter(|p| {
            p.file_name()
                .is_some_and(|f| f.starts_with(stem) && f != file)
        })
        .filter(|p| std::fs::read(p).is_ok_and(|bytes| sha256_hex(&bytes) == want))
        .map(|p| repo.relative(&p))
        .collect();
    found.sort();
    found.into_iter().next()
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
                // The bytes at that path are not the attested bytes. Before
                // calling it drift, look for them: a response is RETIRED to
                // `<name>.<digest>.response.toml` when a later act takes its
                // path (§34.4 — supersede, never erase), and the attestation
                // names the path it had when it was signed. Finding the same
                // bytes beside it is the file moving, not the record changing.
                match archived_copy(repo, &s.name, want) {
                    Some(found) => report.push(Diagnostic::pass(
                        "attest.subject-archived",
                        format!(
                            "{}: the attested bytes now live at {found}, retired when a later \
                             act took the path; the digest still holds",
                            s.name
                        ),
                    )),
                    None => {
                        drifted = true;
                        report.push(Diagnostic::error(
                            "attest.subject-drift",
                            rel.clone(),
                            format!(
                                "{} no longer matches the digest attested for it \
                                 (sha256:{want}), and no file beside it carries those bytes; \
                                 the record was edited after the {act} was attested",
                                s.name
                            ),
                        ));
                    }
                }
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
    let signer = match crate::authority_check::signer_for(repo, actor, &[]) {
        Ok(signer) => signer,
        Err((rule, why)) => {
            report.push(Diagnostic::error(
                "attest.unknown-principal",
                rel,
                format!("predicate actor {actor:?}: {rule}: {why}"),
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
    match verify_pae(&signer.allowed, &signer.principal, &pae_bytes, &sig.sig) {
        Ok(()) if !drifted => report.push(Diagnostic::pass(
            "attest.verified",
            format!(
                "{rel}: {act} attested by {} ({}), {} subject(s) intact",
                signer.principal,
                sig.keyid,
                statement.subject.len()
            ),
        )),
        Ok(()) => report.push(Diagnostic::pass(
            "attest.signature-verified",
            format!(
                "{rel}: signature verifies as {}, but a subject drifted (above)",
                signer.principal
            ),
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

/// `war sign attest <alias> --verify` / `--list`.
pub fn run(repo: &Repository, target: &str, verify: bool) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let (dir, what) = if repo.warrant_dir(target).is_ok() {
        (dir_for(repo, "authorize", target)?, "Warrant")
    } else {
        (dir_for(repo, "sas-accept", target)?, "SAS")
    };
    let mut files: Vec<Utf8PathBuf> = list(repo, &dir)
        .into_iter()
        .filter(|p| what == "Warrant" || p.as_str().contains(&format!("sas-accept-{target}-")))
        .collect();
    // A Warrant's acts may be attested by a batch (OW-WAR-0072), whose
    // envelope lives under docs/authority/batches/attestations/ and not in
    // the Warrant's own folder. Verifying only the folder would pass a
    // Warrant whose current record — attested by the batch — was edited,
    // on the strength of an older envelope whose bytes are kept beside it.
    if what == "Warrant" {
        files.extend(
            list(repo, &dir_for(repo, "batch", "")?)
                .into_iter()
                .filter(|p| batch_attests(p, target)),
        );
    }
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

/// Whether a batch envelope's statement names a file of Warrant `alias`: its
/// directory, or its response under docs/authority/responses/. An envelope
/// that cannot be opened is included, so `verify_file` reports it.
fn batch_attests(path: &Utf8Path, alias: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return true;
    };
    let Ok(envelope) = serde_json::from_str::<openwarrant_core::attestation::Envelope>(&text)
    else {
        return true;
    };
    let Ok(payload) = envelope.open() else {
        return true;
    };
    let Ok(statement) = serde_json::from_slice::<Statement<serde_json::Value>>(&payload) else {
        return true;
    };
    let dir = format!("docs/warrants/{alias}/");
    let response = format!("docs/authority/responses/{alias}.");
    statement
        .subject
        .iter()
        .any(|s| s.name.starts_with(&dir) || s.name.starts_with(&response))
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
    // `standing-revoke` shares `standing-accept`'s folder, so it is listed once.
    for act in [
        "sas-accept",
        "roadmap-accept",
        "batch",
        "invalidate",
        "standing-accept",
    ] {
        for path in list(repo, &dir_for(repo, act, "")?) {
            verify_file(repo, &path, &mut report);
            count += 1;
        }
    }
    report.push(Diagnostic::pass(
        "attest.checked",
        format!("{count} attestation(s) checked"),
    ));
    Ok(report)
}

/// The record a custody audit writes: `docs/warrants/<alias>/custody-audit.toml`.
pub const CUSTODY_AUDIT_FILE: &str = "custody-audit.toml";
pub const CUSTODY_AUDIT_SCHEMA: &str = "oh.war/custody-audit/v1";

/// The files a resolution relied on, as attestation subjects (OW-WAR-0136):
/// each `gate_run_refs` receipt, then its run, stdout and stderr, repository-
/// relative, each once. A receipt that is absent is an error — a resolution
/// relying on a file that is not there has no custody to attest. A run,
/// stdout or stderr that is absent is left out rather than invented.
pub fn relied_on_files(repo: &Repository, alias: &str) -> Result<Vec<Utf8PathBuf>, String> {
    let dir = repo.warrant_dir(alias).map_err(|e| e.to_string())?;
    let record = repo
        .load_resolution(&dir)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!("{alias}: no resolution.toml to take the relied-on evidence from")
        })?;
    let mut out: Vec<Utf8PathBuf> = Vec::new();
    let push = |p: Utf8PathBuf, out: &mut Vec<Utf8PathBuf>| {
        if !out.contains(&p) {
            out.push(p);
        }
    };
    for receipt_ref in &record.resolution.gate_run_refs {
        for (_, rel, required) in evidence_files(repo, receipt_ref) {
            if repo.root.join(&rel).is_file() {
                push(rel, &mut out);
            } else if required {
                return Err(format!(
                    "{alias}: the resolution relies on {rel}, and it is not on disk"
                ));
            }
        }
    }
    Ok(out)
}

/// One relied-on receipt's files: (what, repository-relative path, required).
/// The run sits beside the receipt; stdout and stderr are where the receipt
/// says, when it can be read.
fn evidence_files(repo: &Repository, receipt_ref: &str) -> Vec<(&'static str, Utf8PathBuf, bool)> {
    let receipt = Utf8PathBuf::from(receipt_ref);
    let mut out = vec![("receipt", receipt.clone(), true)];
    if let Some(stem) = receipt_ref.strip_suffix(".receipt.json") {
        out.push(("run", Utf8PathBuf::from(format!("{stem}.run.toml")), false));
    }
    if let Some(r) = read_receipt(repo, receipt_ref) {
        for (what, p) in [("stdout", r.stdout_ref), ("stderr", r.stderr_ref)] {
            if !p.is_empty() {
                out.push((what, Utf8PathBuf::from(p), false));
            }
        }
    }
    out
}

fn read_receipt(repo: &Repository, receipt_ref: &str) -> Option<openwarrant_core::GateReceipt> {
    let text = std::fs::read_to_string(repo.root.join(receipt_ref)).ok()?;
    serde_json::from_str(&text).ok()
}

/// One §41.5 field of one receipt's custody: its value, or why it is UNKNOWN.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CustodyField {
    pub field: &'static str,
    /// `present` | `unknown`.
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl CustodyField {
    fn present(field: &'static str, value: impl Into<String>) -> Self {
        Self {
            field,
            status: "present",
            value: Some(value.into()),
            reason: None,
        }
    }
    fn unknown(field: &'static str, reason: impl Into<String>) -> Self {
        Self {
            field,
            status: "unknown",
            value: None,
            reason: Some(reason.into()),
        }
    }
    fn render(&self) -> String {
        match (&self.value, &self.reason) {
            (Some(v), _) => format!("{}: {v}", self.field),
            (None, Some(r)) => format!("{}: UNKNOWN ({r})", self.field),
            (None, None) => format!("{}: UNKNOWN", self.field),
        }
    }
}

/// One relied-on receipt, audited.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReceiptCustody {
    pub receipt: String,
    pub fields: Vec<CustodyField>,
    /// Files of this receipt that moved since they were attested (or, for an
    /// unattested receipt, a seal that no longer recomputes).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub drift: Vec<String>,
}

/// The whole audit of one resolution.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CustodyAudit {
    pub schema: &'static str,
    pub warrant: String,
    pub resolution_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attestations: Vec<String>,
    /// `intact` | `drifted`.
    pub verdict: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auditor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audited_at: Option<String>,
    pub receipts: Vec<ReceiptCustody>,
}

/// Subject name → attested sha256, over every resolve attestation of a
/// Warrant (a later one wins), with the attestation that named it.
fn attested_subjects(
    repo: &Repository,
    files: &[Utf8PathBuf],
) -> std::collections::BTreeMap<String, (String, String)> {
    let mut out = std::collections::BTreeMap::new();
    for path in files {
        let Some(statement) = std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str::<Envelope>(&t).ok())
            .and_then(|e| e.open().ok())
            .and_then(|p| serde_json::from_slice::<Statement<serde_json::Value>>(&p).ok())
        else {
            continue;
        };
        for s in statement.subject {
            if let Some(d) = s.digest.get("sha256") {
                out.insert(s.name.clone(), (d.clone(), repo.relative(path)));
            }
        }
    }
    out
}

/// The commit that added `rel`, and how many later commits rewrote it.
/// `Err` says why git could not answer.
fn storage_event(repo: &Repository, rel: &Utf8Path) -> Result<String, String> {
    let out = std::process::Command::new("git")
        .args(["log", "--format=%H %cI", "--"])
        .arg(rel.as_str())
        .current_dir(&repo.root)
        .output()
        .map_err(|e| format!("git could not be run: {e}"))?;
    if !out.status.success() {
        return Err("git could not read the history (not a repository?)".to_owned());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let commits: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let Some(first) = commits.last() else {
        return Err("not committed: git records no event storing it".to_owned());
    };
    let later = commits.len() - 1;
    Ok(if later == 0 {
        format!("committed in {first}")
    } else {
        format!("committed in {first}; rewritten by {later} later commit(s)")
    })
}

/// The Gate Definition a run names, by `<gate_id>@<version>`.
fn qualification_of(repo: &Repository, gate_key: &str) -> CustodyField {
    const F: &str = "calibration or qualification";
    match crate::invalidation::find_definition(repo, gate_key) {
        Some((path, def)) => match def.qualification {
            Some(q) if !q.qualifier.trim().is_empty() => CustodyField::present(
                F,
                format!(
                    "{gate_key} qualified by {}{} ({})",
                    q.qualifier,
                    if q.qualification_digest.trim().is_empty() {
                        String::new()
                    } else {
                        format!(", digest {}", q.qualification_digest)
                    },
                    repo.relative(&path)
                ),
            ),
            _ => CustodyField::unknown(
                F,
                format!(
                    "{} declares no qualification for {gate_key}",
                    repo.relative(&path)
                ),
            ),
        },
        None => CustodyField::unknown(
            F,
            format!("no Gate Definition for {gate_key} under the gates path"),
        ),
    }
}

/// `war sign attest <alias> --custody [--record <auditor>]`.
///
/// Returns the report and, when there is a resolution, the audit itself.
pub fn custody(
    repo: &Repository,
    alias: &str,
    record_as: Option<&str>,
) -> Result<(Report, Option<CustodyAudit>), RepoError> {
    let mut report = Report::default();
    let dir = repo.warrant_dir(alias)?;
    let audit_path = dir.join(CUSTODY_AUDIT_FILE);
    // The refusals first, so a refused act has read nothing it would write.
    if let Some(auditor) = record_as {
        let performer = repo.performer();
        if auditor.trim().is_empty() {
            report.push(Diagnostic::error(
                "attest.custody-no-auditor",
                alias.to_owned(),
                "--record names no auditor; an audit nobody performed is not a record".to_owned(),
            ));
            return Ok((report, None));
        }
        if auditor == performer {
            let why = openwarrant_core::authority::AuthorityError::SelfAct {
                actor: auditor.to_owned(),
                act: "audit the custody of",
            };
            report.push(Diagnostic::error(
                "attest.custody-self-act",
                repo.relative(&audit_path),
                format!("SelfAct: {why}. Nothing is written"),
            ));
            return Ok((report, None));
        }
        if audit_path.exists() {
            report.push(Diagnostic::error(
                "attest.custody-audit-exists",
                repo.relative(&audit_path),
                format!(
                    "{alias} already carries a recorded custody audit; a record is not \
                     overwritten. Nothing is written"
                ),
            ));
            return Ok((report, None));
        }
    }
    let Some(record) = repo.load_resolution(&dir)? else {
        report.push(Diagnostic::error(
            "attest.custody-unresolved",
            alias.to_owned(),
            format!(
                "{alias} carries no resolution; custody is audited over the evidence a \
                 resolution relied on"
            ),
        ));
        return Ok((report, None));
    };
    let resolution_rel = repo.relative(&dir.join("resolution.toml"));
    let envelopes: Vec<Utf8PathBuf> = list(repo, &dir.join("attestations"))
        .into_iter()
        .filter(|p| p.file_name().is_some_and(|f| f.starts_with("resolve-")))
        .collect();
    // The envelopes' own signatures and subjects: a subject digest taken from
    // an envelope whose signature does not verify would be no custody at all.
    for path in &envelopes {
        verify_file(repo, path, &mut report);
    }
    let subjects = attested_subjects(repo, &envelopes);
    let mut receipts = Vec::new();
    for receipt_ref in &record.resolution.gate_run_refs {
        let mut drift = Vec::new();
        let files = evidence_files(repo, receipt_ref);
        // Every attested file of this receipt, against its bytes today.
        for (what, rel, _) in &files {
            let Some((want, envelope)) = subjects.get(rel.as_str()) else {
                continue;
            };
            let now = std::fs::read(repo.root.join(rel)).map(|b| sha256_hex(&b));
            match now {
                Ok(have) if have == *want => {}
                Ok(have) => {
                    drift.push(rel.to_string());
                    report.push(Diagnostic::error(
                        "attest.custody-drift",
                        rel.to_string(),
                        format!(
                            "{alias}: the {what} {rel} is sha256:{have}, and {envelope} attested \
                             sha256:{want} when the resolution was signed — the evidence the \
                             resolution relied on is not the evidence on disk"
                        ),
                    ));
                }
                Err(e) => {
                    drift.push(rel.to_string());
                    report.push(Diagnostic::error(
                        "attest.custody-drift",
                        rel.to_string(),
                        format!("{alias}: the attested {what} {rel} cannot be read: {e}"),
                    ));
                }
            }
        }
        let receipt = read_receipt(repo, receipt_ref);
        let attested = subjects.get(receipt_ref.as_str());
        // A-004: an unattested receipt is compared against its own seal only.
        if attested.is_none() {
            match &receipt {
                Some(r) if crate::evidence::receipt_digest_recomputes(r) => {}
                Some(_) => {
                    drift.push(receipt_ref.clone());
                    report.push(Diagnostic::error(
                        "attest.custody-drift",
                        receipt_ref.clone(),
                        format!(
                            "{alias}: {receipt_ref} is not attested, and its own seal no longer \
                             recomputes — it was edited after it was minted"
                        ),
                    ));
                }
                None => {
                    drift.push(receipt_ref.clone());
                    report.push(Diagnostic::error(
                        "attest.custody-drift",
                        receipt_ref.clone(),
                        format!(
                            "{alias}: the resolution relies on {receipt_ref}, and it is absent \
                             or not a receipt"
                        ),
                    ));
                }
            }
        }
        let gate_key = files
            .iter()
            .find(|(w, _, _)| *w == "run")
            .and_then(|(_, p, _)| std::fs::read_to_string(repo.root.join(p)).ok())
            .and_then(|t| toml::from_str::<openwarrant_core::GateRun>(&t).ok())
            .map(|r| r.gate);
        let text_or = |v: Option<&str>, f: &'static str, why: &str| match v {
            Some(s) if !s.trim().is_empty() => CustodyField::present(f, s.to_owned()),
            _ => CustodyField::unknown(f, why.to_owned()),
        };
        let fields = vec![
            text_or(
                receipt.as_ref().map(|r| r.runner.as_str()),
                "collector",
                "the receipt names no runner",
            ),
            match attested {
                Some((d, envelope)) => {
                    CustodyField::present("original digest", format!("sha256:{d} ({envelope})"))
                }
                None => CustodyField::unknown("original digest", "not attested"),
            },
            CustodyField::unknown(
                "transfer method",
                "the receipt records no transfer, and nothing else does",
            ),
            match storage_event(repo, Utf8Path::new(receipt_ref)) {
                Ok(v) => CustodyField::present("storage event", v),
                Err(why) => CustodyField::unknown("storage event", why),
            },
            text_or(
                receipt.as_ref().map(|r| r.runtime_environment.as_str()),
                "instrument or runner identity",
                "the receipt names no runtime environment",
            ),
            match &gate_key {
                Some(k) => qualification_of(repo, k),
                None => CustodyField::unknown(
                    "calibration or qualification",
                    "no run beside the receipt names its gate",
                ),
            },
            CustodyField::unknown(
                "transformations",
                "none is recorded, and an absent record is not evidence that none occurred",
            ),
            CustodyField::unknown("access history", "git records no reads"),
            CustodyField::present(
                "derivative lineage",
                format!(
                    "relied on by resolution {} ({resolution_rel})",
                    record.resolution.id
                ),
            ),
        ];
        let present = fields.iter().filter(|f| f.status == "present").count();
        for f in fields.iter().filter(|f| f.status == "unknown") {
            report.note(format!("{receipt_ref} · {}", f.render()));
        }
        if drift.is_empty() {
            report.push(Diagnostic::pass(
                "attest.custody-audited",
                format!(
                    "{receipt_ref}: {present} of {} §41.5 fields present, {} UNKNOWN (listed \
                     under NOT CHECKED); {}",
                    fields.len(),
                    fields.len() - present,
                    if attested.is_some() {
                        "every attested subject intact"
                    } else {
                        "not attested; its own seal recomputes"
                    }
                ),
            ));
        }
        for f in fields.iter().filter(|f| f.status == "present") {
            report.push(Diagnostic::pass(
                "attest.custody-field",
                format!("{receipt_ref} · {}", f.render()),
            ));
        }
        receipts.push(ReceiptCustody {
            receipt: receipt_ref.clone(),
            fields,
            drift,
        });
    }
    if record.resolution.gate_run_refs.is_empty() {
        report.note(format!(
            "{alias}: the resolution relied on no gate run; there is no receipt custody to audit"
        ));
    }
    let mut audit = CustodyAudit {
        schema: CUSTODY_AUDIT_SCHEMA,
        warrant: alias.to_owned(),
        resolution_id: record.resolution.id.clone(),
        attestations: envelopes.iter().map(|p| repo.relative(p)).collect(),
        verdict: if report.is_ready() {
            "intact"
        } else {
            "drifted"
        },
        auditor: None,
        audited_at: None,
        receipts,
    };
    if let Some(auditor) = record_as {
        audit.auditor = Some(auditor.to_owned());
        audit.audited_at = Some(crate::gate_cmd::receipt::now_rfc3339_public());
        let body = toml::to_string_pretty(&audit)
            .map_err(|e| RepoError::Message(format!("could not render the custody audit: {e}")))?;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&audit_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, body.as_bytes()))
            .map_err(|source| RepoError::Io {
                context: format!("could not write {audit_path}"),
                source,
            })?;
        report.push(Diagnostic::pass(
            "attest.custody-recorded",
            format!(
                "{alias}: custody audit ({}) recorded by {auditor} → {}",
                audit.verdict,
                repo.relative(&audit_path)
            ),
        ));
    }
    Ok((report, Some(audit)))
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

    /// A roadmap acceptance and a batch are not Warrants: their envelopes
    /// live beside their own records, and the directory is known without a
    /// Warrant lookup, so it is settled before the dialog. ROADMAP-1 and
    /// ROADMAP-2 went unattested because this fell through to `warrant_dir`.
    #[test]
    fn roadmap_and_batch_attestations_have_a_home_that_is_not_a_warrant() {
        let dir = scratch("home");
        let repo_root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize_utf8()
            .unwrap();
        std::fs::copy(
            repo_root.join("openwarrant.toml"),
            dir.join("openwarrant.toml"),
        )
        .unwrap();
        let repo = Repository::open(dir.clone()).expect("scratch repository opens");
        assert_eq!(
            dir_for(&repo, "roadmap-accept", "ROADMAP-3").unwrap(),
            crate::roadmap_cmd::dir(&repo).join("revisions/attestations")
        );
        assert_eq!(
            dir_for(&repo, "batch", "B-1").unwrap(),
            dir.join("docs/authority/batches/attestations")
        );
        // The refusal: an act that IS a Warrant's still needs that Warrant.
        assert!(dir_for(&repo, "authorize", "OW-WAR-9999").is_err());
        let _ = std::fs::remove_dir_all(&dir);
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
