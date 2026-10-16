// SPDX-License-Identifier: Apache-2.0
//! Opt-in candidate compact-review storage (proposed ADR0030).
//!
//! This codec does not activate ingestion, alter the v1/v2 reader, read files,
//! establish independence or award assurance. A workflow must validate both
//! referenced byte sets, their compiled relationship and current subject.
use crate::verification::Verification;
use crate::verification_record::{ReviewedPacket, ReviewedSubject, VerificationRecordError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const SCHEMA: &str = "oh.war/verification/v3";
pub const RESPONSE_SCHEMA: &str = "oh.war/verification-response/v3";
pub const VIEW_SCHEMA: &str = "oh.war/verification-view/v1";

/// Ordinary SHA-256 of the actual view file bytes, not an archive digest.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedView {
    pub path: String,
    pub sha256: String,
}

/// A distinct envelope keeps old readers from ignoring required view bindings.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredCompactVerification {
    pub schema: String,
    pub verification: Verification,
    pub verification_protocol: String,
    pub reviewed_subject: ReviewedSubject,
    pub reviewed_packets: Vec<ReviewedPacket>,
    pub reviewed_views: Vec<ReviewedView>,
}

fn raw_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

impl StoredCompactVerification {
    /// Compare metadata only. No file read, source-coverage judgment, actor
    /// authentication or independent verification follows from a true result.
    #[must_use]
    pub fn binds(
        &self,
        subject: &ReviewedSubject,
        archives: &[ReviewedPacket],
        views: &[ReviewedView],
    ) -> bool {
        if self.schema != SCHEMA
            || self.verification_protocol != RESPONSE_SCHEMA
            || &self.reviewed_subject != subject
            || self.reviewed_packets != archives
            || self.reviewed_views != views
            || archives.is_empty()
            || views.is_empty()
        {
            return false;
        }
        let mut paths = BTreeSet::new();
        archives.iter().all(|r| {
            crate::document::references::path(&r.path)
                && raw_sha256(&r.digest)
                && paths.insert(r.path.as_str())
        }) && views.iter().all(|r| {
            crate::document::references::path(&r.path)
                && raw_sha256(&r.sha256)
                && paths.insert(r.path.as_str())
        })
    }
}

/// Explicit opt-in: the existing record decoder continues to refuse v3.
/// Valid syntax is separate from matching bindings and from qualification.
pub fn decode(text: &str) -> Result<StoredCompactVerification, VerificationRecordError> {
    use VerificationRecordError::{Malformed, UnsupportedSchema};
    let root: toml::Value = toml::from_str(text).map_err(|e| Malformed(e.to_string()))?;
    let schema = root
        .get("schema")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| Malformed("compact verification record requires a string schema".into()))?;
    if schema != SCHEMA {
        return Err(UnsupportedSchema {
            schema: schema.into(),
        });
    }
    toml::from_str(text).map_err(|e| Malformed(e.to_string()))
}

/// Candidate schema only: exporting it does not activate or adopt a schema pack.
#[cfg(feature = "schema")]
#[must_use]
pub fn schema() -> schemars::Schema {
    let mut schema = schemars::schema_for!(StoredCompactVerification);
    let object = schema.as_object_mut().expect("record schema is an object");
    object.insert("$id".into(), serde_json::json!(SCHEMA));
    object
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .and_then(|p| p.get_mut("schema"))
        .and_then(serde_json::Value::as_object_mut)
        .expect("record schema declares its format")
        .insert("const".into(), serde_json::json!(SCHEMA));
    schema
}
