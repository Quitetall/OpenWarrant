// SPDX-License-Identifier: Apache-2.0
//! Experimental preservation transport (OW111). No authority or filesystem effects.
use std::collections::{BTreeMap, BTreeSet};

use openwarrant_core::attestation::{base64_decode, base64_encode};
use openwarrant_core::journal::EXPORT_CONTENTS;
use serde::{Deserialize, Serialize};

use crate::{sha256_hex, to_canonical_bytes};

pub const SCHEMA: &str = "oh.war/preservation-archive/v1-draft.1";

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub archive_bytes: usize,
    pub records: usize,
    pub content_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            archive_bytes: 32 * 1024 * 1024,
            records: 4096,
            content_bytes: 16 * 1024 * 1024,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("preservation: {0}")]
pub struct Error(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub path: String,
    pub digest: String,
    /// None means externally retained content, never an empty file.
    pub base64: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Coverage {
    Retained { paths: Vec<String> },
    Absent { reason: String },
    Unavailable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Archive {
    pub schema: String,
    pub subject: String,
    pub producer: String,
    pub records: Vec<Record>,
    pub coverage: BTreeMap<String, Coverage>,
    pub extensions: BTreeMap<String, serde_json::Value>,
}

fn fail<T>(message: impl Into<String>) -> Result<T, Error> {
    Err(Error(message.into()))
}

fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 1024
        && path.split('/').all(|part| {
            let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
            !part.is_empty()
                && part != "."
                && part != ".."
                && !part.ends_with('.')
                && !part.ends_with(' ')
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                && !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                && !(stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        })
}

fn digest_valid(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|v| {
        v.len() == 64
            && v.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

fn check_content(record: &Record, bytes: &[u8]) -> Result<(), Error> {
    if record.digest != format!("sha256:{}", sha256_hex(bytes)) {
        return fail(format!("content digest mismatch: {}", record.path));
    }
    Ok(())
}

impl Archive {
    pub fn validate(&self, limits: Limits) -> Result<(), Error> {
        if self.schema != SCHEMA {
            return fail("unsupported archive version");
        }
        if self.subject.trim().is_empty() || self.producer.trim().is_empty() {
            return fail("subject and producer are required");
        }
        if self.records.is_empty() || self.records.len() > limits.records {
            return fail("record count outside limit");
        }
        let mut paths = BTreeSet::new();
        let mut folded = BTreeSet::new();
        let mut previous: Option<&str> = None;
        let mut total = 0usize;
        for record in &self.records {
            if !safe_path(&record.path) || previous.is_some_and(|p| p >= record.path.as_str()) {
                return fail("unsafe, duplicate or unsorted record path");
            }
            previous = Some(&record.path);
            paths.insert(record.path.as_str());
            if !folded.insert(record.path.to_ascii_lowercase()) {
                return fail("case-insensitive path collision");
            }
            if !digest_valid(&record.digest) {
                return fail("invalid content digest");
            }
            if let Some(encoded) = &record.base64 {
                if encoded.len() > limits.archive_bytes {
                    return fail("encoded content exceeds limit");
                }
                let bytes = base64_decode(encoded).map_err(Error)?;
                if base64_encode(&bytes) != *encoded {
                    return fail("noncanonical base64");
                }
                total = total
                    .checked_add(bytes.len())
                    .ok_or_else(|| Error("content size overflow".into()))?;
                if total > limits.content_bytes {
                    return fail("embedded content exceeds limit");
                }
                check_content(record, &bytes)?;
            }
        }
        for path in &folded {
            let mut parent = path.as_str();
            while let Some((prefix, _)) = parent.rsplit_once('/') {
                if folded.contains(prefix) {
                    return fail("file/directory path collision");
                }
                parent = prefix;
            }
        }
        for category in EXPORT_CONTENTS
            .iter()
            .filter(|s| !s.starts_with("optional "))
        {
            if !self.coverage.contains_key(*category) {
                return fail(format!("missing coverage: {category}"));
            }
        }
        let mut covered = BTreeSet::new();
        for (category, coverage) in &self.coverage {
            if !EXPORT_CONTENTS.contains(&category.as_str()) {
                return fail("unknown coverage category");
            }
            match coverage {
                Coverage::Retained { paths: refs } => {
                    if refs.is_empty() {
                        return fail("retained coverage has no records");
                    }
                    let mut prior: Option<&str> = None;
                    for path in refs {
                        if !paths.contains(path.as_str())
                            || prior.is_some_and(|p| p >= path.as_str())
                        {
                            return fail("missing, duplicate or unsorted coverage reference");
                        }
                        prior = Some(path);
                        covered.insert(path.as_str());
                    }
                }
                Coverage::Absent { reason } | Coverage::Unavailable { reason } => {
                    if reason.trim().is_empty() {
                        return fail("coverage reason is required");
                    }
                }
            }
        }
        if covered != paths {
            return fail("record has no coverage classification");
        }
        if self.extensions.keys().any(|key| {
            key.split_once(':')
                .is_none_or(|(a, b)| a.is_empty() || b.is_empty())
        }) {
            return fail("extension keys must be namespaced");
        }
        Ok(())
    }

    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        self.validate(limits)?;
        let bytes = to_canonical_bytes(self).map_err(|e| Error(e.to_string()))?;
        if bytes.len() > limits.archive_bytes {
            return fail("archive exceeds byte limit");
        }
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        if bytes.len() > limits.archive_bytes {
            return fail("archive exceeds byte limit");
        }
        let archive: Self = serde_json::from_slice(bytes).map_err(|e| Error(e.to_string()))?;
        if archive.encode(limits)? != bytes {
            return fail("archive is not canonical");
        }
        Ok(archive)
    }

    pub fn digest(&self, limits: Limits) -> Result<String, Error> {
        self.encode(limits)?;
        let preimage = serde_json::json!({"digest_domain": SCHEMA, "payload": self});
        Ok(format!(
            "sha256:{}",
            sha256_hex(&to_canonical_bytes(&preimage).map_err(|e| Error(e.to_string()))?)
        ))
    }

    /// Resolve exact bytes; a boolean or a matching pair of metadata digests is insufficient.
    /// Caller must bound resolver I/O. This method bounds aggregate returned content.
    pub fn reconnect<F>(
        &self,
        limits: Limits,
        mut resolver: F,
    ) -> Result<BTreeMap<String, Vec<u8>>, Error>
    where
        F: FnMut(&str, usize) -> Result<Vec<u8>, Error>,
    {
        self.validate(limits)?;
        if self
            .coverage
            .values()
            .any(|v| matches!(v, Coverage::Unavailable { .. }))
        {
            return fail("required coverage unavailable");
        }
        let mut result = BTreeMap::new();
        let mut remaining = limits.content_bytes;
        for record in &self.records {
            let bytes = match &record.base64 {
                Some(value) => base64_decode(value).map_err(Error)?,
                None => resolver(&record.digest, remaining)?,
            };
            if bytes.len() > remaining {
                return fail("reconnected content exceeds limit");
            }
            remaining -= bytes.len();
            check_content(record, &bytes)?;
            result.insert(record.path.clone(), bytes);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Archive {
        let mut coverage: BTreeMap<_, _> = EXPORT_CONTENTS
            .iter()
            .filter(|s| !s.starts_with("optional "))
            .map(|s| {
                (
                    (*s).into(),
                    Coverage::Absent {
                        reason: "fixture has no such record".into(),
                    },
                )
            })
            .collect();
        coverage.insert(
            "source manifest".into(),
            Coverage::Retained {
                paths: vec!["manifest.toml".into()],
            },
        );
        Archive {
            schema: SCHEMA.into(),
            subject: "fixture://warrant".into(),
            producer: "fixture://producer".into(),
            records: vec![Record {
                path: "manifest.toml".into(),
                digest: format!("sha256:{}", sha256_hex(b"exact\r\nbytes\0")),
                base64: Some(base64_encode(b"exact\r\nbytes\0")),
            }],
            coverage,
            extensions: BTreeMap::from([(
                "test:opaque".into(),
                serde_json::json!({"preserve": [1, true]}),
            )]),
        }
    }

    #[test]
    fn canonical_codec_preserves_exact_bytes_and_extensions() {
        let first = fixture();
        let limits = Limits::default();
        let encoded = first.encode(limits).unwrap();
        let second = Archive::decode(&encoded, limits).unwrap();
        assert_eq!(second, first);
        assert_eq!(second.encode(limits).unwrap(), encoded);
        assert_eq!(
            second.digest(limits).unwrap(),
            first.digest(limits).unwrap()
        );
        let content = second
            .reconnect(limits, |_, _| {
                panic!("embedded record must not resolve externally")
            })
            .unwrap();
        assert_eq!(content["manifest.toml"], b"exact\r\nbytes\0");
    }

    #[test]
    fn missing_tampered_and_unavailable_content_cannot_reconnect() {
        let mut archive = fixture();
        let limits = Limits::default();
        archive.records[0].base64 = None;
        assert!(archive.reconnect(limits, |_, _| fail("not found")).is_err());
        assert!(
            archive
                .reconnect(limits, |_, _| Ok(b"wrong".to_vec()))
                .is_err()
        );
        assert!(
            archive
                .reconnect(limits, |_, _| Ok(b"exact\r\nbytes\0".to_vec()))
                .is_ok()
        );
        archive.coverage.insert(
            "artifacts".into(),
            Coverage::Unavailable {
                reason: "missing artifact".into(),
            },
        );
        assert!(
            archive
                .reconnect(limits, |_, _| panic!(
                    "unavailable coverage must stop resolution"
                ))
                .is_err()
        );
    }

    #[test]
    fn refuses_unsafe_paths_versions_omissions_and_limits() {
        for path in [
            "../x", "/x", "a//b", "a\\b", "C:x", "CON.txt", "a/../x", "a.",
        ] {
            let mut archive = fixture();
            archive.records[0].path = path.into();
            assert!(
                archive.validate(Limits::default()).is_err(),
                "accepted {path}"
            );
        }
        let mut archive = fixture();
        archive.schema = "unknown".into();
        assert!(archive.validate(Limits::default()).is_err());
        for category in EXPORT_CONTENTS
            .iter()
            .filter(|s| !s.starts_with("optional "))
        {
            let mut archive = fixture();
            archive.coverage.remove(*category);
            assert!(archive.validate(Limits::default()).is_err());
        }
        let bytes = fixture().encode(Limits::default()).unwrap();
        assert!(
            Archive::decode(
                &bytes,
                Limits {
                    archive_bytes: bytes.len() - 1,
                    ..Limits::default()
                }
            )
            .is_err()
        );
        assert!(
            fixture()
                .encode(Limits {
                    content_bytes: 1,
                    ..Limits::default()
                })
                .is_err()
        );
        assert!(
            fixture()
                .encode(Limits {
                    records: 0,
                    ..Limits::default()
                })
                .is_err()
        );
    }

    #[test]
    fn refuses_duplicate_json_keys_and_record_collisions() {
        let bytes = fixture().encode(Limits::default()).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let duplicate = text.replacen("{", "{\"schema\":\"ignored\",", 1);
        assert!(Archive::decode(duplicate.as_bytes(), Limits::default()).is_err());
        for path in ["manifest.toml", "MANIFEST.toml", "manifest.toml/child"] {
            let mut archive = fixture();
            let mut record = archive.records[0].clone();
            record.path = path.into();
            archive.records.push(record);
            archive.records.sort_by(|a, b| a.path.cmp(&b.path));
            assert!(archive.validate(Limits::default()).is_err());
        }
        let mut archive = fixture();
        archive.records[0].base64 = Some(base64_encode(b"changed"));
        assert!(archive.validate(Limits::default()).is_err());
    }
}
