// SPDX-License-Identifier: Apache-2.0
//! Experimental v2 storage. The v1 payload stays nested so old bare-record
//! readers refuse it rather than count a verdict without its review binding.
use openwarrant_core::verification::Verification;
use serde::{Deserialize, Serialize};

use super::{ReviewedPacket, ReviewedSubject, VerificationResponse};

pub(crate) const SCHEMA: &str = "oh.war/verification/v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredVerification {
    pub schema: String,
    pub verification: Verification,
    pub verification_protocol: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_subject: Option<ReviewedSubject>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reviewed_packets: Vec<ReviewedPacket>,
}

impl StoredVerification {
    pub(crate) fn new(verification: &Verification, response: &VerificationResponse) -> Self {
        Self {
            schema: SCHEMA.into(),
            verification: verification.clone(),
            verification_protocol: response.schema.clone(),
            reviewed_subject: response.reviewed_subject.clone(),
            reviewed_packets: response.reviewed_packets.clone(),
        }
    }

    pub(crate) fn binds(&self, subject: &ReviewedSubject, packets: &[ReviewedPacket]) -> bool {
        self.schema == SCHEMA
            && self.verification_protocol == super::RESPONSE_SCHEMA
            && self.reviewed_subject.as_ref() == Some(subject)
            && self.reviewed_packets == packets
            && !packets.is_empty()
    }
}

/// Explicitly select the format before decoding. Never decode an unsupported
/// envelope as v1 simply because serde would ignore its unknown root fields.
pub(crate) fn decode(text: &str) -> Result<StoredVerification, String> {
    let root: toml::Value = toml::from_str(text).map_err(|e| e.to_string())?;
    if root.get("schema").is_some_and(|value| !value.is_str()) {
        return Err("verification record schema must be a string".into());
    }
    match root.get("schema").and_then(toml::Value::as_str) {
        Some(SCHEMA) => toml::from_str(text).map_err(|e| e.to_string()),
        None | Some("oh.war/verification/v1") => {
            let verification = toml::from_str(text).map_err(|e| e.to_string())?;
            Ok(StoredVerification {
                schema: "oh.war/verification/v1".into(),
                verification,
                verification_protocol: "oh.war/verification-response/v1".into(),
                reviewed_subject: None,
                reviewed_packets: vec![],
            })
        }
        Some(schema) => Err(format!("unsupported verification record schema {schema:?}")),
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
