// SPDX-License-Identifier: Apache-2.0
//! Storage adapter for the shared public SDK record and retained bytes.
use super::VerificationResponse;
use openwarrant_core::verification::Verification;
pub(crate) use openwarrant_core::verification_record::StoredVerification;

pub(crate) fn new(
    verification: &Verification,
    response: &VerificationResponse,
) -> StoredVerification {
    StoredVerification {
        schema: openwarrant_core::verification_record::SCHEMA.into(),
        verification: verification.clone(),
        verification_protocol: response.schema.clone(),
        reviewed_subject: response.reviewed_subject.clone(),
        reviewed_packets: response.reviewed_packets.clone(),
    }
}

pub(crate) fn decode(
    text: &str,
) -> Result<StoredVerification, openwarrant_core::verification_record::VerificationRecordError> {
    use openwarrant_core::verification_record::DecodedVerificationRecord;
    match openwarrant_core::verification_record::decode(text)? {
        DecodedVerificationRecord::V2(record) => Ok(*record),
        // The legacy CLI assessment reads one view. It never serializes this
        // unbound adapter value or promotes it to current qualification.
        DecodedVerificationRecord::Legacy(verification) => Ok(StoredVerification {
            schema: "oh.war/verification/v1".into(),
            verification,
            verification_protocol: "oh.war/verification-response/v1".into(),
            reviewed_subject: None,
            reviewed_packets: vec![],
        }),
    }
}

/// Preserve the earlier wire bytes before replacing an active record. History
/// is outside the active loader's non-recursive enumeration. Publication uses
/// the existing no-follow, no-overwrite retained-evidence primitive.
pub(crate) fn retain_previous(
    repo: &crate::repo::Repository,
    path: &camino::Utf8Path,
) -> Result<crate::compile::atomic::Prestate, crate::repo::RepoError> {
    use crate::repo::RepoError;
    let unavailable = |error: String| RepoError::ObservationUnavailable {
        rule: "verify.history-unavailable",
        message: format!(
            "could not retain earlier verification {path}: {error}; no active record replaced"
        ),
    };
    let relative = path
        .strip_prefix(&repo.root)
        .map_err(|error| unavailable(error.to_string()))?;
    let parent = relative
        .parent()
        .ok_or_else(|| unavailable("missing record directory".into()))?;
    // Check every directory component before the ordinary atomic writer is
    // allowed to use this path. This is not a sandbox against a same-uid actor.
    crate::bundle::store::Directory::open(&repo.root, parent)
        .map_err(|error| unavailable(error.to_string()))?;
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(crate::compile::atomic::Prestate::Absent)
        }
        Err(error) => Err(unavailable(error.to_string())),
        Ok(_) => {
            let bytes = crate::bundle::store::read(&repo.root, relative)
                .map_err(|error| unavailable(error.to_string()))?;
            let digest = openwarrant_compiler::sha256_hex(&bytes);
            let history =
                crate::bundle::store::Directory::open(&repo.root, &parent.join("history"))
                    .map_err(|error| unavailable(error.to_string()))?;
            history
                .retain(&format!("{digest}.toml"), &bytes)
                .map_err(|error| unavailable(error.to_string()))?;
            Ok(crate::compile::atomic::Prestate::of(&bytes))
        }
    }
}

/// Prior observations are optional background, never current qualification.
/// Check displayed claims against retained source records rather than today's
/// active list alone: ingesting a newer review must not stale its own packet.
/// This checks recorded bytes, not verifier authenticity or independent custody.
pub(crate) fn prior_matches(
    repo: &crate::repo::Repository,
    dir: &camino::Utf8Path,
    shown: &serde_json::Value,
) -> Result<bool, crate::repo::RepoError> {
    use crate::repo::RepoError;
    let unavailable = |rule, message: &str| RepoError::ObservationUnavailable {
        rule,
        message: message.into(),
    };
    let Some(rows) = shown.as_array() else {
        return Ok(false);
    };
    let mut claims = Vec::new();
    for row in rows {
        let Ok(claim) = serde_json::from_value::<Verification>(row.clone()) else {
            return Ok(false);
        };
        if serde_json::to_value(&claim).map_err(|e| RepoError::Message(e.to_string()))? != *row
            || claims.contains(&claim)
        {
            return Ok(false);
        }
        claims.push(claim);
    }
    if claims.is_empty() {
        return Ok(true);
    }
    let mut retained = Vec::new();
    let base = dir.join("verifications");
    for (directory, history) in [(&base, false), (&base.join("history"), true)] {
        let metadata = match std::fs::symlink_metadata(directory) {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(unavailable("verify.history-unavailable", &e.to_string())),
        };
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(unavailable(
                "verify.history-unavailable",
                "review history is not a regular directory",
            ));
        }
        let entries = std::fs::read_dir(directory)
            .map_err(|e| unavailable("verify.history-unavailable", &e.to_string()))?;
        for entry in entries {
            let entry =
                entry.map_err(|e| unavailable("verify.history-unavailable", &e.to_string()))?;
            let Ok(path) = camino::Utf8PathBuf::from_path_buf(entry.path()) else {
                continue;
            };
            if path.extension() != Some("toml") {
                continue;
            }
            let relative = path
                .strip_prefix(&repo.root)
                .map_err(|e| RepoError::Message(e.to_string()))?;
            let bytes = crate::bundle::store::read(&repo.root, relative)?;
            // A retained historical name is the ordinary hash of its original
            // bytes. Do not introduce another canonicalization or digest domain.
            if history
                && path.file_stem() != Some(openwarrant_compiler::sha256_hex(&bytes).as_str())
            {
                continue;
            }
            let Ok(text) = std::str::from_utf8(&bytes) else {
                continue;
            };
            match decode(text) {
                Ok(record) => retained.push(record.verification),
                Err(openwarrant_core::verification_record::VerificationRecordError::UnsupportedSchema { schema }) => {
                    return Err(unavailable("verify.record-schema-unsupported", &format!("{relative}: unsupported verification record {schema}")));
                }
                Err(_) => {}
            }
        }
    }
    Ok(claims.iter().all(|claim| retained.contains(claim)))
}
