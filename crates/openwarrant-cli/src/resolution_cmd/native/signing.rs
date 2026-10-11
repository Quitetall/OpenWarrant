// SPDX-License-Identifier: Apache-2.0
//! Human SSH transport over an immutable in-memory proposal. Not TTY approval.
use super::*;
use std::{fs, path::Path};

#[derive(Serialize)]
pub struct ResultPacket {
    pub schema: &'static str,
    pub response: Response,
    pub recorded: bool,
    pub signed_proposal_path: Option<String>,
}

struct Snapshot(std::path::PathBuf);
impl Drop for Snapshot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn snapshot(bytes: &[u8]) -> Result<(Snapshot, camino::Utf8PathBuf), Fault> {
    let path = std::env::temp_dir().join(format!("war-native-sign-{}", WarUuid::mint()));
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(&path)
        .map_err(|e| fault("native-sign.snapshot-unavailable", e, true))?;
    let guard = Snapshot(path);
    let file = camino::Utf8PathBuf::from_path_buf(guard.0.join("response.toml")).map_err(|_| {
        fault(
            "native-sign.snapshot-unavailable",
            "UTF8 scratch path required",
            true,
        )
    })?;
    fs::write(&file, bytes).map_err(|e| fault("native-sign.snapshot-unavailable", e, true))?;
    Ok((guard, file))
}

fn retain(repo: &Repository, bytes: &[u8], signature: &str) -> Result<String, Fault> {
    let directory = Utf8Path::new(".openwarrant/state/native-signatures");
    let retained = crate::bundle::store::Directory::open(&repo.root, directory).map_err(source)?;
    // One immutable packet publishes the pair together. It is an observation,
    // never a replacement authority root or a cached native verdict.
    let packet = serde_json::to_vec(&serde_json::json!({
        "schema": "oh.war/native-signed-proposal/v1-draft.1",
        "response": std::str::from_utf8(bytes).expect("parsed UTF8"),
        "signature": signature,
    }))
    .map_err(|e| fault("native-sign.retention", e, true))?;
    let name = format!("{}.json", sha256_hex(&packet));
    retained.retain(&name, &packet).map_err(source)?;
    Ok(directory.join(name).to_string())
}

fn complete(
    repo: &Repository,
    alias: &str,
    path: &Utf8Path,
    bytes: &[u8],
    signature: &str,
    native: &NativeAssessment,
) -> Result<(Report, ResultPacket), Fault> {
    // Retain even if current authority, source, or evidence moved while the key
    // was asked. A retained proposal confers no acceptance on its own.
    let saved = retain(repo, bytes, signature)?;
    let failure = |mut f: Fault| {
        f.message = format!(
            "{}; signed proposal retained at {saved}; recording did not complete; inspect current records before retry",
            f.message
        );
        f
    };
    let current = crate::runtime_capture::read(repo, path, 65_536).map_err(failure)?;
    if current != bytes {
        return Err(failure(fault(
            "native-sign.proposal-changed",
            "proposal changed during signing",
            false,
        )));
    }
    let report = ingest(
        repo,
        alias,
        bytes,
        signature,
        native,
        crate::sign::IngestMode::Record,
    )
    .map_err(failure)?;
    let response = parse(bytes)?;
    Ok((
        report,
        ResultPacket {
            schema: "oh.war/native-signing-result/v1-draft.1",
            response,
            recorded: true,
            signed_proposal_path: Some(saved),
        },
    ))
}

/// No-signature mode checks the exact reviewed decision without writing or
/// asking a key. Explicit SSH mode signs a private copy of those exact bytes.
pub fn run(
    repo: &Repository,
    alias: &str,
    path: &Utf8Path,
    native_store: &Path,
    ssh_sign: bool,
) -> Result<(Report, ResultPacket), Fault> {
    run_reviewed(repo, alias, path, native_store, ssh_sign, |_, _| {})
}

/// The caller renders the immutable decision before the key is requested.
/// Rendering receives the same bytes later signed, without reopening the source.
pub fn run_reviewed(
    repo: &Repository,
    alias: &str,
    path: &Utf8Path,
    native_store: &Path,
    ssh_sign: bool,
    review: impl FnOnce(&Response, &[u8]),
) -> Result<(Report, ResultPacket), Fault> {
    let bytes = crate::runtime_capture::read(repo, path, 65_536)?;
    let response = parse(&bytes)?;
    if response.warrant != alias {
        return Err(fault(
            "native-resolution.subject",
            "proposal names another Warrant",
            false,
        ));
    }
    let directory = repo.warrant_dir(alias).map_err(source)?;
    if repo.load_resolution(&directory).map_err(source)?.is_some() {
        return Err(fault(
            "native-resolution.exists",
            "resolution already exists; no key requested",
            false,
        ));
    }
    let native = NativeAssessment::load(repo, alias, &response.selections, native_store)?;
    recheck(repo, &response, &native)?;
    if !ssh_sign {
        let mut report = Report::default();
        report.push(Diagnostic::pass(
            "native-sign.ready",
            "current native proposal is reviewable; no signature or write",
        ));
        return Ok((
            report,
            ResultPacket {
                schema: "oh.war/native-signing-result/v1-draft.1",
                response,
                recorded: false,
                signed_proposal_path: None,
            },
        ));
    }
    review(&response, &bytes);
    if crate::runtime_capture::read(repo, path, 65_536)? != bytes {
        return Err(fault(
            "native-sign.proposal-changed",
            "proposal changed during review; no key requested",
            false,
        ));
    }
    recheck(repo, &response, &native)?;
    // Hold the normal protected signer's exact public key through the dialog.
    // Unlike legacy ssh_sign_act, retain the signed proposal before refusing a
    // moved authority head. Fresh ingestion is the only acceptance authority.
    let signer = crate::authority_check::signer_for(
        repo,
        &response.resolved_by,
        &[crate::authority_check::Act::Resolve],
    )
    .map_err(|(rule, why)| fault(rule, why, rule == "authority.verify-unavailable"))?;
    let (signature, presence) = {
        let (guard, file) = snapshot(&bytes)?;
        let signed = crate::sign::ssh_sign_file(&signer.allowed, &signer.principal, &file)
            .map_err(|f| fault(f.rule, f.why, f.rule == "sign.ssh-refused"))?;
        let bytes =
            crate::progress_viewer::source::read(&guard.0, Path::new("response.toml.sig"), 16_384)
                .map_err(|e| fault("native-sign.signature-unavailable", e, true))?;
        // `signed` was verified by the existing SSH transport; the later ingest
        // separately verifies against the exact original in-memory proposal.
        let signature = String::from_utf8(bytes)
            .map_err(|e| fault("native-sign.signature-syntax", e, false))?;
        (signature, signed.presence())
    };
    if repo.config.policy.require_user_presence
        && presence != openwarrant_core::presence::Presence::Verified
    {
        let saved = retain(repo, &bytes, &signature)?;
        return Err(fault(
            "sign.presence-required",
            format!(
                "configured policy requires authenticated presence; signed proposal retained at {saved}; no acceptance recorded"
            ),
            false,
        ));
    }
    complete(repo, alias, path, &bytes, &signature, &native)
}
