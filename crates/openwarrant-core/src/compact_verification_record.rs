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

/// What the compact verifier reports. Parsing and preparing records do not
/// establish current source bindings, independence, admissibility or authority.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompactVerificationResponse {
    pub schema: String,
    pub warrant: String,
    pub reviewed_subject: ReviewedSubject,
    pub reviewed_packets: Vec<ReviewedPacket>,
    pub reviewed_views: Vec<ReviewedView>,
    pub verifications: Vec<Verification>,
}

fn raw_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn valid_references(archives: &[ReviewedPacket], views: &[ReviewedView]) -> bool {
    if archives.is_empty() || views.is_empty() {
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

impl CompactVerificationResponse {
    /// Preflight all observation identifiers and reference sets before exposing
    /// any record. Output is
    /// lazy to avoid copying a large subject map for every observation at once.
    /// The caller must still validate actual files, current scope and verifier
    /// independence before writing anything or using a record for assurance.
    pub fn prepare_records(
        &self,
    ) -> Result<
        impl ExactSizeIterator<Item = StoredCompactVerification> + '_,
        VerificationRecordError,
    > {
        use VerificationRecordError::{Malformed, UnsupportedSchema};
        if self.schema != RESPONSE_SCHEMA {
            return Err(UnsupportedSchema {
                schema: self.schema.clone(),
            });
        }
        if self.warrant.trim().is_empty() || self.verifications.is_empty() {
            return Err(Malformed(
                "compact response requires a Warrant and observations".into(),
            ));
        }
        if !valid_references(&self.reviewed_packets, &self.reviewed_views) {
            return Err(Malformed(
                "compact response requires distinct valid archive and view references".into(),
            ));
        }
        let mut obligations = BTreeSet::new();
        for observation in &self.verifications {
            if observation.obligation.trim().is_empty()
                || !obligations.insert(observation.obligation.as_str())
            {
                return Err(Malformed(
                    "compact response has an empty or repeated obligation; no records prepared"
                        .into(),
                ));
            }
        }
        Ok(self
            .verifications
            .iter()
            .map(|observation| StoredCompactVerification {
                schema: SCHEMA.into(),
                verification: observation.clone(),
                verification_protocol: self.schema.clone(),
                reviewed_subject: self.reviewed_subject.clone(),
                reviewed_packets: self.reviewed_packets.clone(),
                reviewed_views: self.reviewed_views.clone(),
            }))
    }
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
        {
            return false;
        }
        valid_references(archives, views)
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

/// Explicit protocol selection before response decoding; no legacy upgrade or
/// interpretation of a future response as current metadata is permitted.
pub fn decode_response(text: &str) -> Result<CompactVerificationResponse, VerificationRecordError> {
    use VerificationRecordError::{Malformed, UnsupportedSchema};
    let root: toml::Value = toml::from_str(text).map_err(|e| Malformed(e.to_string()))?;
    let schema = root
        .get("schema")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| Malformed("compact response requires a string schema".into()))?;
    if schema != RESPONSE_SCHEMA {
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
    typed_schema::<StoredCompactVerification>(SCHEMA)
}

/// Candidate response schema; publication does not enable CLI ingestion.
#[cfg(feature = "schema")]
#[must_use]
pub fn response_schema() -> schemars::Schema {
    typed_schema::<CompactVerificationResponse>(RESPONSE_SCHEMA)
}

#[cfg(feature = "schema")]
fn typed_schema<T: schemars::JsonSchema>(id: &str) -> schemars::Schema {
    let mut schema = schemars::schema_for!(T);
    let object = schema.as_object_mut().expect("record schema is an object");
    object.insert("$id".into(), serde_json::json!(id));
    object
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .and_then(|p| p.get_mut("schema"))
        .and_then(serde_json::Value::as_object_mut)
        .expect("record schema declares its format")
        .insert("const".into(), serde_json::json!(id));
    schema
}
