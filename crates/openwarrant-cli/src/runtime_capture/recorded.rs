// SPDX-License-Identifier: Apache-2.0
//! Obtain identity from a retained Dispatch and its actual compile event.
use super::*;
use openwarrant_compiler::CompilationBasis;
use openwarrant_core::{ValidatedManifest, execution::StageDispatch, journal::JournalEvent};

pub(super) struct Recorded {
    pub dispatch: StageDispatch,
    pub dispatch_bytes: Vec<u8>,
    pub dispatch_ref: String,
    pub event_bytes: Vec<u8>,
    pub event_ref: String,
    pub directory: Utf8PathBuf,
    pub basis: CompilationBasis,
    pub validated: ValidatedManifest,
}

pub(super) fn load(repo: &Repository, alias: &str, id: &str) -> Result<Recorded, Fault> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(fault(
            "runtime.capture-dispatch-id",
            "a retained dispatch identifier is required",
            false,
        ));
    }
    let dir = repo
        .warrant_dir(alias)
        .map_err(|e| fault("runtime.capture-warrant", e, false))?;
    let directory = dir
        .strip_prefix(&repo.root)
        .map_err(|e| fault("runtime.capture-path", e, false))?
        .to_owned();
    let loaded = repo
        .load_warrant(&dir)
        .map_err(|e| fault("runtime.capture-basis", e, true))?;
    let basis = loaded.basis.ok_or_else(|| {
        fault(
            "runtime.capture-basis",
            "current source basis unavailable",
            true,
        )
    })?;
    let validated = loaded.validated.ok_or_else(|| {
        fault(
            "runtime.capture-basis",
            "validated manifest unavailable",
            true,
        )
    })?;
    let dispatch_ref = directory.join(format!("dispatches/{id}.json")).to_string();
    let dispatch_bytes = read(repo, Utf8Path::new(&dispatch_ref), 4 * 1024 * 1024)?;
    let dispatch: StageDispatch = serde_json::from_value(decode(&dispatch_bytes)?)
        .map_err(|e| fault("runtime.capture-dispatch", e, false))?;
    if dispatch.dispatch_id != id {
        return Err(fault(
            "runtime.capture-dispatch-id",
            "retained filename and dispatch identity differ",
            false,
        ));
    }
    let journal_ref = directory.join("journal.jsonl");
    let journal_bytes = read(repo, &journal_ref, 16 * 1024 * 1024)?;
    if crate::journal_cmd::torn_tail(&journal_bytes).is_some() {
        return Err(fault(
            "runtime.capture-journal",
            "incomplete journal tail; no capture published",
            true,
        ));
    }
    let mut matching = vec![];
    let mut journal = openwarrant_core::journal::Journal::default();
    for line in journal_bytes
        .split(|b| *b == b'\n')
        .filter(|line| !line.iter().all(u8::is_ascii_whitespace))
    {
        let event: JournalEvent = serde_json::from_value(decode(line)?)
            .map_err(|e| fault("runtime.capture-journal", e, false))?;
        journal
            .append(event.clone())
            .map_err(|e| fault("runtime.capture-journal", e, false))?;
        if event.event_type != "dispatch.compiled" {
            continue;
        }
        let payload = decode(event.payload.as_bytes())?;
        if payload["dispatch_id"] != id {
            continue;
        }
        if event.warrant_uuid != basis.manifest.uuid
            || payload["dispatch_digest"] != dispatch.dispatch_digest
            || payload["contract_digest"] != dispatch.contract_digest
            || payload["stage"] != dispatch.stage_id
            || payload["attempt_id"] != dispatch.attempt_id
        {
            return Err(fault(
                "runtime.capture-recorded-binding",
                "compile event does not bind these exact dispatch bytes",
                false,
            ));
        }
        matching.push((event.id, line.to_vec()));
    }
    if matching.len() != 1 {
        return Err(fault(
            "runtime.capture-recorded-dispatch",
            "exactly one matching compile event required; local records are not execution proof",
            matching.is_empty(),
        ));
    }
    let (event_id, event_bytes) = matching.pop().expect("one matching event");
    Ok(Recorded {
        dispatch,
        dispatch_bytes,
        dispatch_ref,
        event_bytes,
        event_ref: format!("{journal_ref}#event:{event_id}"),
        directory,
        basis,
        validated,
    })
}
