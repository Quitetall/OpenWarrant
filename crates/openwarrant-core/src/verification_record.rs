// SPDX-License-Identifier: Apache-2.0
//! Stored review observations. Decoding or matching bindings grants no authority.
//!
//! Frozen bare v1 records remain unbound history. New v2 records nest that
//! payload so older bare-record readers cannot silently count the observation.
//! Packet authenticity, independent custody, current context and human
//! acceptance require the caller's separate checks.
use crate::verification::Verification;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA: &str = "oh.war/verification/v2";
pub const RESPONSE_SCHEMA: &str = "oh.war/verification-response/v2";

/// A review binds the compiled contract, delivered bytes, cited gate definitions
/// and their declared fixture bytes. It does not grant approval or authority.
/// Digests use existing contract canonicalization and ordinary file SHA-256;
/// no new semantic digest domain or replacement canonicalizer is introduced.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedSubject {
    pub contract_digest: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub context_sources: BTreeMap<String, String>,
    pub artifacts: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub gate_definitions: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fixtures: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub gate_evidence: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub gate_inputs: BTreeMap<String, String>,
    /// Exact link target text also distinguishes a link from a regular file.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub gate_links: BTreeMap<String, String>,
}

/// References to exact retained packets, using the existing canonical domain.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedPacket {
    pub path: String,
    pub digest: String,
}

/// A v2 storage envelope. Legacy records are a separate decoded variant.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredVerification {
    pub schema: String,
    pub verification: Verification,
    pub verification_protocol: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_subject: Option<ReviewedSubject>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reviewed_packets: Vec<ReviewedPacket>,
}

impl StoredVerification {
    /// Compare declared bindings only. This does not verify packet bytes,
    /// journal provenance, actor independence or the current governing context.
    #[must_use]
    pub fn binds(&self, subject: &ReviewedSubject, packets: &[ReviewedPacket]) -> bool {
        self.schema == SCHEMA
            && self.verification_protocol == RESPONSE_SCHEMA
            && self.reviewed_subject.as_ref() == Some(subject)
            && self.reviewed_packets == packets
            && !packets.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VerificationRecordError {
    #[error("unsupported verification record schema {schema:?}")]
    UnsupportedSchema { schema: String },
    #[error("malformed verification record: {0}")]
    Malformed(String),
}

/// Reading old history and reading a bound v2 record are distinct results.
/// This enum deliberately has no serializer: retention uses the original bytes.
#[derive(Debug, Clone)]
pub enum DecodedVerificationRecord {
    Legacy(Verification),
    V2(Box<StoredVerification>),
}

impl DecodedVerificationRecord {
    /// The recorded observation, without an authority or currency judgment.
    #[must_use]
    pub fn observation(&self) -> &Verification {
        match self {
            Self::Legacy(value) => value,
            Self::V2(value) => &value.verification,
        }
    }

    /// Only an explicit v2 variant can match reviewed bindings.
    #[must_use]
    pub fn binds(&self, subject: &ReviewedSubject, packets: &[ReviewedPacket]) -> bool {
        match self {
            Self::Legacy(_) => false,
            Self::V2(value) => value.binds(subject, packets),
        }
    }
}

/// Select the declared format before decoding. No bytes are rewritten and
/// newer optional fields cannot promote a v1 observation into a v2 review.
pub fn decode(text: &str) -> Result<DecodedVerificationRecord, VerificationRecordError> {
    use VerificationRecordError::{Malformed, UnsupportedSchema};
    let root: toml::Value = toml::from_str(text).map_err(|e| Malformed(e.to_string()))?;
    if root.get("schema").is_some_and(|value| !value.is_str()) {
        return Err(Malformed(
            "verification record schema must be a string".into(),
        ));
    }
    match root.get("schema").and_then(toml::Value::as_str) {
        Some(SCHEMA) => toml::from_str(text)
            .map(|record| DecodedVerificationRecord::V2(Box::new(record)))
            .map_err(|e| Malformed(e.to_string())),
        None | Some("oh.war/verification/v1") => {
            let verification = toml::from_str(text).map_err(|e| Malformed(e.to_string()))?;
            Ok(DecodedVerificationRecord::Legacy(verification))
        }
        Some(schema) => Err(UnsupportedSchema {
            schema: schema.into(),
        }),
    }
}

/// Candidate v2 wire schema. Publication alone does not adopt a format pack.
#[cfg(feature = "schema")]
#[must_use]
pub fn schema() -> schemars::Schema {
    let mut schema = schemars::schema_for!(StoredVerification);
    let object = schema.as_object_mut().expect("record schema is an object");
    object.insert("$id".into(), serde_json::json!(SCHEMA));
    object
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .and_then(|properties| properties.get_mut("schema"))
        .and_then(serde_json::Value::as_object_mut)
        .expect("record schema declares its format")
        .insert("const".into(), serde_json::json!(SCHEMA));
    schema
}
