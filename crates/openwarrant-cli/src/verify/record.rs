// SPDX-License-Identifier: Apache-2.0
//! Storage adapter for the shared public SDK record and retained bytes.
use super::VerificationResponse;
use openwarrant_core::compact_verification_record::{ReviewedView, StoredCompactVerification};
use openwarrant_core::verification::Verification;
use openwarrant_core::verification_record::{ReviewedPacket, ReviewedSubject, StoredVerification};

#[derive(serde::Serialize)]
#[serde(untagged)]
pub(crate) enum WrittenRecord {
    V2(StoredVerification),
    V3(StoredCompactVerification),
}

/// Read adapter only. It retains the full binding envelope and is deliberately
/// not serializable: history retention always uses the original wire bytes.
pub(crate) struct ReadRecord {
    pub verification: Verification,
    binding: Option<WrittenRecord>,
}

impl ReadRecord {
    pub(crate) fn binds_protocol(
        &self,
        subject: &ReviewedSubject,
        packets: &[ReviewedPacket],
        views: &[ReviewedView],
        protocol: &str,
    ) -> bool {
        match &self.binding {
            Some(WrittenRecord::V2(value)) => {
                protocol == super::RESPONSE_SCHEMA
                    && views.is_empty()
                    && value.binds(subject, packets)
            }
            Some(WrittenRecord::V3(value)) => {
                protocol == super::COMPACT_RESPONSE_SCHEMA && value.binds(subject, packets, views)
            }
            None => false,
        }
    }
}

pub(crate) fn new(
    verification: &Verification,
    response: &VerificationResponse,
) -> Result<WrittenRecord, crate::repo::RepoError> {
    if response.schema == super::COMPACT_RESPONSE_SCHEMA {
        return Ok(WrittenRecord::V3(StoredCompactVerification {
            schema: openwarrant_core::compact_verification_record::SCHEMA.into(),
            verification: verification.clone(),
            verification_protocol: response.schema.clone(),
            reviewed_subject: response.reviewed_subject.clone().ok_or_else(|| {
                crate::repo::RepoError::Message("compact record requires a reviewed subject".into())
            })?,
            reviewed_packets: response.reviewed_packets.clone(),
            reviewed_views: response.reviewed_views.clone(),
        }));
    }
    Ok(WrittenRecord::V2(StoredVerification {
        schema: openwarrant_core::verification_record::SCHEMA.into(),
        verification: verification.clone(),
        verification_protocol: response.schema.clone(),
        reviewed_subject: response.reviewed_subject.clone(),
        reviewed_packets: response.reviewed_packets.clone(),
    }))
}

/// Shared immutable parse of exactly these wire bytes; no source judgment is cached.
pub(crate) fn decode_shared(
    text: &str,
) -> Result<
    std::sync::Arc<ReadRecord>,
    openwarrant_core::verification_record::VerificationRecordError,
> {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex, OnceLock};
    const MAX_ENTRIES: usize = 32;
    const MAX_WIRE_BYTES: usize = 32 * 1024 * 1024;
    #[derive(Default)]
    struct Cache {
        entries: VecDeque<(Box<str>, Arc<ReadRecord>)>,
        wire_bytes: usize,
    }
    impl Cache {
        fn get(&mut self, text: &str) -> Option<Arc<ReadRecord>> {
            let index = self
                .entries
                .iter()
                .position(|(wire, _)| wire.as_ref() == text)?;
            let entry = self.entries.remove(index)?;
            let record = Arc::clone(&entry.1);
            self.entries.push_back(entry);
            Some(record)
        }
    }
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Ok(mut cached) = cache.lock()
        && let Some(record) = cached.get(text)
    {
        return Ok(record);
    }
    // Decode with the unchanged codec and outside the lock. Failed/unsupported
    // inputs are never remembered; callers still observe their current bytes.
    let record = Arc::new(decode(text)?);
    if text.len() <= MAX_WIRE_BYTES
        && let Ok(mut cached) = cache.lock()
    {
        if let Some(existing) = cached.get(text) {
            return Ok(existing);
        }
        while cached.entries.len() >= MAX_ENTRIES
            || cached.wire_bytes.saturating_add(text.len()) > MAX_WIRE_BYTES
        {
            let Some((wire, _)) = cached.entries.pop_front() else {
                break;
            };
            cached.wire_bytes = cached.wire_bytes.saturating_sub(wire.len());
        }
        cached.wire_bytes += text.len();
        cached.entries.push_back((text.into(), Arc::clone(&record)));
    }
    Ok(record)
}

pub(crate) fn decode(
    text: &str,
) -> Result<ReadRecord, openwarrant_core::verification_record::VerificationRecordError> {
    use openwarrant_core::verification_record::DecodedVerificationRecord;
    let root: toml::Value = toml::from_str(text).map_err(|e| {
        openwarrant_core::verification_record::VerificationRecordError::Malformed(e.to_string())
    })?;
    if root.get("schema").and_then(toml::Value::as_str)
        == Some(openwarrant_core::compact_verification_record::SCHEMA)
    {
        // The compact codec selects the schema and then deserializes this
        // same structure. Reuse the parsed value instead of parsing large
        // subject maps twice more. Keep its original diagnostics on failure.
        let record: StoredCompactVerification = if contains_datetime(&root) {
            // toml::Value can coerce a datetime into a string where the text
            // codec refuses it. Preserve the original acceptance boundary.
            openwarrant_core::compact_verification_record::decode(text)?
        } else {
            match root.try_into() {
                Ok(record) => record,
                Err(_) => openwarrant_core::compact_verification_record::decode(text)?,
            }
        };
        return Ok(ReadRecord {
            verification: record.verification.clone(),
            binding: Some(WrittenRecord::V3(record)),
        });
    }
    match openwarrant_core::verification_record::decode(text)? {
        DecodedVerificationRecord::V2(record) => Ok(ReadRecord {
            verification: record.verification.clone(),
            binding: Some(WrittenRecord::V2(*record)),
        }),
        // The legacy CLI assessment reads one view. It never serializes this
        // unbound adapter value or promotes it to current qualification.
        DecodedVerificationRecord::Legacy(verification) => Ok(ReadRecord {
            verification,
            binding: None,
        }),
    }
}

fn contains_datetime(value: &toml::Value) -> bool {
    match value {
        toml::Value::Datetime(_) => true,
        toml::Value::Array(values) => values.iter().any(contains_datetime),
        toml::Value::Table(values) => values.values().any(contains_datetime),
        _ => false,
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
            match decode_shared(text) {
                Ok(record) => retained.push(record.verification.clone()),
                Err(openwarrant_core::verification_record::VerificationRecordError::UnsupportedSchema { schema }) => {
                    return Err(unavailable("verify.record-schema-unsupported", &format!("{relative}: unsupported verification record {schema}")));
                }
                Err(_) => {}
            }
        }
    }
    Ok(claims.iter().all(|claim| retained.contains(claim)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwarrant_core::verification_record::VerificationRecordError;

    #[test]
    fn shared_decode_reuses_only_identical_wire_bytes() {
        let text = r#"
obligation = "OBL-001"
disposition = "not_established"
evidence = "synthetic original evidence; no execution claim"
performer = "fixture-worker"
[verifier]
actor = "fixture-reviewer"
kind = "agent"
[verifier.independence]
performer_transcript_blind = false
performer_rationale_blind = false
separate_writable_workspace = false
cannot_modify_subject_artifacts = false
cannot_modify_gate_definition = false
cannot_modify_gate_fixtures = false
separate_context_compilation = false
distinct_model_required = false
distinct_human_required = false
"#;
        let first = decode_shared(text).unwrap();
        let repeated = decode_shared(text).unwrap();
        assert!(
            std::sync::Arc::ptr_eq(&first, &repeated),
            "identical successful parses share immutable data"
        );
        let changed = text.replace("original evidence", "changed evidence");
        let next = decode_shared(&changed).unwrap();
        assert!(!std::sync::Arc::ptr_eq(&first, &next));
        assert_eq!(
            next.verification.evidence,
            "synthetic changed evidence; no execution claim"
        );
        let malformed = format!("obligation = 7\n{text}");
        assert!(
            decode_shared(&malformed).is_err(),
            "a prior valid parse cannot hide changed invalid bytes"
        );
    }

    #[test]
    fn compact_read_adapter_preserves_codec_acceptance_and_bindings() {
        let verification = openwarrant_core::verification_record::decode(
            r#"
obligation = "OBL-001"
disposition = "not_established"
evidence = "synthetic codec fixture; no execution or assurance claim"
performer = "fixture-worker"
[verifier]
actor = "fixture-reviewer"
kind = "agent"
[verifier.independence]
performer_transcript_blind = false
performer_rationale_blind = false
separate_writable_workspace = false
cannot_modify_subject_artifacts = false
cannot_modify_gate_definition = false
cannot_modify_gate_fixtures = false
separate_context_compilation = false
distinct_model_required = false
distinct_human_required = false
"#,
        )
        .unwrap()
        .observation()
        .clone();
        let candidate = StoredCompactVerification {
            schema: openwarrant_core::compact_verification_record::SCHEMA.into(),
            verification_protocol: super::super::COMPACT_RESPONSE_SCHEMA.into(),
            verification,
            reviewed_subject: serde_json::from_value(
                serde_json::json!({"contract_digest":"fixture","artifacts":{}}),
            )
            .unwrap(),
            reviewed_packets: vec![ReviewedPacket {
                path: "bundle.json".into(),
                digest: "a".repeat(64),
            }],
            reviewed_views: vec![ReviewedView {
                path: "view.json".into(),
                sha256: "b".repeat(64),
            }],
        };
        let base = toml::Value::try_from(candidate).unwrap();
        for fault in [
            "valid",
            "unknown-field",
            "missing-view",
            "wrong-map",
            "datetime",
            "integer-evidence",
            "boolean-evidence",
            "float-evidence",
            "future",
            "schema-type",
            "duplicate",
            "empty-views",
        ] {
            let mut value = base.clone();
            match fault {
                "unknown-field" => {
                    value
                        .as_table_mut()
                        .unwrap()
                        .insert("unexpected".into(), true.into());
                }
                "missing-view" => {
                    value.as_table_mut().unwrap().remove("reviewed_views");
                }
                "wrong-map" => value["reviewed_subject"]["artifacts"] = 7.into(),
                "datetime" => {
                    value["verification"]["evidence"] =
                        toml::Value::Datetime("2026-01-01T00:00:00Z".parse().unwrap())
                }
                "integer-evidence" => value["verification"]["evidence"] = 7.into(),
                "boolean-evidence" => value["verification"]["evidence"] = true.into(),
                "float-evidence" => value["verification"]["evidence"] = toml::Value::Float(1.5),
                "future" => value["schema"] = "oh.war/verification/v4".into(),
                "schema-type" => value["schema"] = 7.into(),
                "empty-views" => value["reviewed_views"] = toml::Value::Array(Vec::new()),
                _ => {}
            }
            let mut text = toml::to_string(&value).unwrap();
            if fault == "duplicate" {
                text = format!("schema = \"duplicate\"\n{text}");
            }
            let expected = openwarrant_core::compact_verification_record::decode(&text);
            let actual = decode(&text);
            let shared = decode_shared(&text);
            match (&actual, &shared) {
                (Ok(actual), Ok(shared)) => {
                    assert_eq!(actual.verification, shared.verification, "{fault}");
                    assert_eq!(
                        serde_json::to_value(&actual.binding).unwrap(),
                        serde_json::to_value(&shared.binding).unwrap(),
                        "{fault}"
                    );
                }
                (Err(actual), Err(shared)) => {
                    assert_eq!(actual.to_string(), shared.to_string(), "{fault}")
                }
                _ => panic!("shared decoding changed codec acceptance for {fault}"),
            }
            match (expected, actual) {
                (Ok(expected), Ok(actual)) => {
                    assert_eq!(actual.verification, expected.verification, "{fault}");
                    assert_eq!(
                        serde_json::to_value(actual.binding.unwrap()).unwrap(),
                        serde_json::to_value(expected).unwrap(),
                        "{fault}"
                    );
                }
                (
                    Err(VerificationRecordError::Malformed(_)),
                    Err(VerificationRecordError::Malformed(_)),
                )
                | (
                    Err(VerificationRecordError::UnsupportedSchema { .. }),
                    Err(VerificationRecordError::UnsupportedSchema { .. }),
                ) => {}
                _ => panic!("compact decoder acceptance changed for {fault}"),
            }
        }
    }
}
