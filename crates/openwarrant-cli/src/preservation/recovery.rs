// SPDX-License-Identifier: Apache-2.0
//! Recover exact historical bytes without ingesting a new verifier judgment.
use super::{history, read};
use crate::repo::Repository;
use openwarrant_compiler::preservation::{Error, Limits};

pub(super) fn verification(
    repo: &Repository,
    alias: &str,
    oid: &str,
    expected: &str,
    limits: Limits,
) -> Result<(String, serde_json::Value), Error> {
    if !matches!(oid.len(), 40 | 64)
        || !oid
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(Error("canonical Git blob object id required".into()));
    }
    let bytes = history::git(repo, &["cat-file", "blob", oid], limits.content_bytes)?;
    let hex = openwarrant_compiler::sha256_hex(&bytes);
    let digest = format!("sha256:{hex}");
    if expected != digest {
        return Err(Error("verification blob digest mismatch".into()));
    }
    let text = std::str::from_utf8(&bytes).map_err(|e| Error(e.to_string()))?;
    let record = crate::verify::record::decode(text).map_err(|e| Error(e.to_string()))?;
    let verdict = &record.verification;
    let dir = repo.warrant_dir(alias).map_err(|e| Error(e.to_string()))?;
    let mut remaining = limits.content_bytes - bytes.len();
    let manifest_bytes = read(dir.join("manifest.toml").as_std_path(), remaining)?;
    remaining -= manifest_bytes.len();
    let manifest: openwarrant_core::Manifest =
        toml::from_str(std::str::from_utf8(&manifest_bytes).map_err(|e| Error(e.to_string()))?)
            .map_err(|e| Error(e.to_string()))?;
    let journal_bytes = read(dir.join("journal.jsonl").as_std_path(), remaining)?;
    let journal = crate::journal_cmd::parse(
        std::str::from_utf8(&journal_bytes).map_err(|e| Error(e.to_string()))?,
    )
    .map_err(Error)?;
    if journal.events.len() > limits.records {
        return Err(Error("recovery journal exceeds record limit".into()));
    }
    let actor = format!("{}://{}", verdict.verifier.kind, verdict.verifier.actor);
    let disposition = verdict.disposition.to_string();
    let event = journal
        .events
        .iter()
        .find(|event| {
            if event.v != 1
                || event.class != openwarrant_core::journal::EventClass::DraftHistory
                || event.warrant_uuid != manifest.uuid
                || event.event_type != crate::journal_cmd::VERIFICATION_RECORDED
                || event.actor_ref != actor
            {
                return false;
            }
            let Ok(payload) = serde_json::from_str::<serde_json::Value>(&event.payload) else {
                return false;
            };
            payload["record_digest"].as_str() == Some(digest.as_str())
                && payload["obligation"].as_str() == Some(verdict.obligation.as_str())
                && payload["disposition"].as_str() == Some(disposition.as_str())
        })
        .ok_or_else(|| {
            Error("existing journal reference required for original verdict bytes".into())
        })?;
    let relative = dir
        .strip_prefix(&repo.root)
        .map_err(|e| Error(e.to_string()))?
        .join("verifications/history");
    let store = crate::bundle::store::Directory::open(&repo.root, &relative)
        .map_err(|e| Error(e.to_string()))?;
    store
        .retain(&format!("{hex}.toml"), &bytes)
        .map_err(|e| Error(e.to_string()))?;
    Ok((
        "Retained original verification bytes as history; current verdict and authority unchanged."
            .into(),
        serde_json::json!({"schema":"oh.war/preservation-result/v1-draft.1",
            "operation":"recover-verification", "git_blob":oid, "digest":digest,
            "reference":relative.join(format!("{hex}.toml")).as_str(), "bytes":bytes.len(),
            "journal_event":event.id, "authority_activated":false, "assurance_granted":false}),
    ))
}
