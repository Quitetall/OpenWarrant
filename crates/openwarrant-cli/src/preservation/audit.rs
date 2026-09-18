// SPDX-License-Identifier: Apache-2.0
//! Reconnect local journal references to retained record and receipt bytes.
use openwarrant_compiler::preservation::{Coverage, Error};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn coverage(
    files: &BTreeMap<String, Vec<u8>>,
    directory: &str,
) -> Result<Coverage, Error> {
    if !files.contains_key("__ow_archive__/history.json") {
        return Ok(Coverage::Unavailable {
            reason: "Action and audit history requires --history".into(),
        });
    }
    let local: BTreeMap<_, _> = files
        .iter()
        .filter(|(path, _)| {
            path.starts_with(&format!("{directory}/"))
                || path.starts_with("__ow_archive__/history/")
        })
        .collect();
    let digests: BTreeSet<_> = local
        .values()
        .map(|bytes| format!("sha256:{}", openwarrant_compiler::sha256_hex(bytes)))
        .collect();
    let mut receipts = BTreeSet::new();
    let mut gaps = BTreeSet::new();
    let mut paths = BTreeSet::from(["__ow_archive__/history.json".to_owned()]);
    for (path, bytes) in &local {
        if !path.ends_with(".receipt.json") {
            continue;
        }
        paths.insert((*path).clone());
        let receipt: openwarrant_core::GateReceipt = match serde_json::from_slice(bytes) {
            Ok(receipt) => receipt,
            Err(error) => {
                gaps.insert(format!("{path}: receipt cannot be read: {error}"));
                continue;
            }
        };
        if receipt.validate().is_err() || !crate::evidence::receipt_digest_recomputes(&receipt) {
            gaps.insert(format!("{path}: receipt seal or required fields invalid"));
            continue;
        }
        receipts.insert(receipt.receipt_digest);
        let prefix = path
            .strip_prefix("__ow_archive__/history/")
            .and_then(|rest| rest.split_once('/'))
            .map(|(commit, _)| format!("__ow_archive__/history/{commit}/"))
            .unwrap_or_default();
        for reference in receipt
            .raw_evidence_refs
            .iter()
            .chain([&receipt.stdout_ref, &receipt.stderr_ref])
        {
            let target = format!("{prefix}{reference}");
            if !local.contains_key(&target) {
                gaps.insert(format!(
                    "{path}: receipt evidence not retained at {reference}"
                ));
            } else {
                paths.insert(target);
            }
        }
    }
    let journal = format!("{directory}/journal.jsonl");
    let mut journal_count = 0;
    for (path, bytes) in &local {
        if **path != journal && !path.ends_with(&format!("/{journal}")) {
            continue;
        }
        journal_count += 1;
        paths.insert((*path).clone());
        let text = std::str::from_utf8(bytes).map_err(|e| Error(e.to_string()))?;
        let journal = crate::journal_cmd::parse(text)
            .map_err(|e| Error(format!("invalid retained journal {path}: {e}")))?;
        for event in journal.events {
            if event.v != 1 || event.class != openwarrant_core::journal::EventClass::DraftHistory {
                gaps.insert(format!(
                    "{path}#{}: unsupported version or provider action needs resolver",
                    event.id
                ));
            }
            let payload: serde_json::Value = match serde_json::from_str(&event.payload) {
                Ok(value) => value,
                Err(_) => {
                    gaps.insert(format!("{path}#{}: payload needs resolver", event.id));
                    continue;
                }
            };
            for (field, available) in [("record_digest", &digests), ("receipt_digest", &receipts)] {
                if let Some(value) = payload.get(field)
                    && !value
                        .as_str()
                        .is_some_and(|digest| available.contains(digest))
                {
                    gaps.insert(format!("{path}#{}: {field} bytes not retained", event.id));
                }
            }
            match event.event_type.as_str() {
                "draft.created"
                | "authorization.recorded"
                | "resolution.recorded"
                | "verification.recorded"
                | "correction.recorded"
                | "sync.receipt_attached" => {}
                other => {
                    gaps.insert(format!(
                        "{path}#{}: {other} reference resolver unavailable",
                        event.id
                    ));
                }
            }
        }
    }
    if journal_count == 0 {
        gaps.insert("No local journal retained; action inventory cannot be established".into());
    }
    if !gaps.is_empty() {
        return Ok(Coverage::Unavailable {
            reason: gaps.into_iter().collect::<Vec<_>>().join("; "),
        });
    }
    // Raw record digests may resolve to any retained historical record. Keep
    // those source pointers in coverage, without changing their authority.
    paths.extend(local.keys().map(|path| (*path).clone()));
    Ok(Coverage::Retained {
        paths: paths.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn journal_digest_requires_retained_bytes_and_known_event_semantics() {
        let dir = "docs/warrants/FIXTURE-WAR-0001";
        let bytes = b"exact verification fixture";
        let hash = format!("sha256:{}", openwarrant_compiler::sha256_hex(bytes));
        let event = |kind: &str, payload: &str| {
            serde_json::to_vec(&crate::journal_cmd::event(
                "fixture",
                kind,
                "fixture",
                "2026-09-18T00:00:00Z",
                payload,
            ))
            .unwrap()
        };
        let key = format!("{dir}/journal.jsonl");
        let mut files = BTreeMap::from([
            (
                "__ow_archive__/history.json".into(),
                b"history independently checked by caller".to_vec(),
            ),
            (
                key.clone(),
                event(
                    "verification.recorded",
                    &format!("{{\"record_digest\":\"{hash}\"}}"),
                ),
            ),
        ]);
        assert!(
            matches!(coverage(&files, dir).unwrap(), Coverage::Unavailable { reason } if reason.contains("bytes not retained"))
        );
        let record = format!("{dir}/verifications/fixture.toml");
        files.insert(record.clone(), bytes.to_vec());
        assert!(matches!(
            coverage(&files, dir).unwrap(),
            Coverage::Retained { .. }
        ));
        files.insert(record, b"changed fixture".to_vec());
        assert!(matches!(
            coverage(&files, dir).unwrap(),
            Coverage::Unavailable { .. }
        ));
        files.insert(key.clone(), event("verification.recorded", "{}"));
        // Older envelopes did not record content addresses. Preserve their
        // actual meaning; no new receipt claim is inferred from that absence.
        assert!(matches!(
            coverage(&files, dir).unwrap(),
            Coverage::Retained { .. }
        ));
        files.insert(key, event("unknown.event", "{}"));
        assert!(
            matches!(coverage(&files, dir).unwrap(), Coverage::Unavailable { reason } if reason.contains("resolver unavailable"))
        );
    }
    #[test]
    fn receipt_seal_and_referenced_output_are_required() {
        let dir = "docs/warrants/OW-WAR-0030";
        let bytes = include_bytes!(
            "../../../../conformance/fixtures/preservation/legacy-gate-receipt.json"
        );
        let receipt: openwarrant_core::GateReceipt = serde_json::from_slice(bytes).unwrap();
        let path = format!("{dir}/gate-runs/fixture.receipt.json");
        let event = crate::journal_cmd::event(
            "fixture",
            "sync.receipt_attached",
            "fixture",
            "2026-09-18T00:00:00Z",
            &format!("{{\"receipt_digest\":\"{}\"}}", receipt.receipt_digest),
        );
        let mut files = BTreeMap::from([
            (
                "__ow_archive__/history.json".into(),
                b"history independently checked by caller".to_vec(),
            ),
            (
                format!("{dir}/journal.jsonl"),
                serde_json::to_vec(&event).unwrap(),
            ),
            (path.clone(), bytes.to_vec()),
            (receipt.stdout_ref.clone(), b"retained output".to_vec()),
            (receipt.stderr_ref.clone(), Vec::new()),
        ]);
        assert!(matches!(
            coverage(&files, dir).unwrap(),
            Coverage::Retained { .. }
        ));
        files.remove(&receipt.stdout_ref);
        assert!(
            matches!(coverage(&files, dir).unwrap(), Coverage::Unavailable { reason } if reason.contains("receipt evidence not retained"))
        );
        files.insert(receipt.stdout_ref.clone(), b"retained output".to_vec());
        let mut changed = receipt;
        changed.runner = "substituted runner".into();
        files.insert(path, serde_json::to_vec(&changed).unwrap());
        assert!(
            matches!(coverage(&files, dir).unwrap(), Coverage::Unavailable { reason } if reason.contains("receipt seal"))
        );
    }
}
