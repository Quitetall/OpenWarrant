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
