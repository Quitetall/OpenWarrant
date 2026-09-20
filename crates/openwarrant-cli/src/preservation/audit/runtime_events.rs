// SPDX-License-Identifier: Apache-2.0
//! Reconnect local journal references, without authorizing or qualifying execution.
use openwarrant_core::execution::{StageDispatch, StageSubmission};
use std::collections::BTreeMap;

pub(super) fn check(
    files: &BTreeMap<String, Vec<u8>>,
    journal: &str,
    directory: &str,
    event: &openwarrant_core::journal::JournalEvent,
    payload: &serde_json::Value,
) -> Result<(), String> {
    let id = payload
        .get("dispatch_id")
        .and_then(|v| v.as_str())
        .filter(|s| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        })
        .ok_or("invalid dispatch reference")?;
    let suffix = format!("{directory}/journal.jsonl");
    let prefix = journal
        .strip_suffix(&suffix)
        .ok_or("invalid journal location")?;
    let dispatch_path = format!("{prefix}{directory}/dispatches/{id}.json");
    let bytes = files
        .get(&dispatch_path)
        .ok_or_else(|| format!("dispatch bytes not retained: {dispatch_path}"))?;
    let dispatch: StageDispatch = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let mut blank = dispatch.clone();
    blank.dispatch_digest.clear();
    let digest = openwarrant_compiler::canonical::sha256_digest(
        openwarrant_compiler::digest::DigestDomain::Dispatch,
        &blank,
    )
    .map_err(|e| e.to_string())?;
    if dispatch.dispatch_id != id
        || dispatch.warrant_ref != format!("war://{}", event.warrant_uuid)
        || dispatch.api_version != openwarrant_core::execution::DISPATCH_API_VERSION
        || dispatch.dispatch_digest != digest
        || payload.get("stage").and_then(|v| v.as_str()) != Some(dispatch.stage_id.as_str())
    {
        return Err("dispatch identity, stage or digest differs from retained journal".into());
    }
    if event.event_type == "dispatch.compiled" {
        if payload.get("dispatch_digest").and_then(|v| v.as_str()) != Some(digest.as_str()) {
            return Err("compiled dispatch digest differs from journal".into());
        }
        return Ok(());
    }
    let source = format!("{directory}/submissions/{id}.json");
    if payload.get("path").and_then(|v| v.as_str()) != Some(source.as_str()) {
        return Err("submission path differs from dispatch reference".into());
    }
    let path = format!("{prefix}{source}");
    let bytes = files
        .get(&path)
        .ok_or_else(|| format!("submission bytes not retained: {path}"))?;
    let submission: StageSubmission = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    submission.validate().map_err(|e| e.to_string())?;
    if submission.dispatch_id != id
        || submission.attempt_id != dispatch.attempt_id
        || submission.contract_digest != dispatch.contract_digest
        || submission.stage_id != dispatch.stage_id
        || payload.get("requested_next_action")
            != Some(
                &serde_json::to_value(submission.requested_next_action)
                    .map_err(|e| e.to_string())?,
            )
        || payload.get("blockers").and_then(|v| v.as_u64())
            != Some(submission.blockers.len() as u64)
    {
        return Err("submission identity or outcome differs from retained dispatch/journal".into());
    }
    Ok(())
}
