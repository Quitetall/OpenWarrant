// SPDX-License-Identifier: Apache-2.0
//! Retained capture inventory. Reconnection is not native authentication.
use super::*;
use openwarrant_core::{execution::StageDispatch, journal::JournalEvent};
use serde_json::{Value, json};

pub(super) fn inventory(
    files: &BTreeMap<String, Vec<u8>>,
    directory: &str,
    alias: &str,
    warrant: &str,
    stages: &Value,
) -> Result<Value, Error> {
    let marker = format!("{directory}/runtime-receipts/");
    let mut records = Vec::new();
    let mut gaps = BTreeSet::new();
    for (path, bytes) in files {
        let Some((prefix, name)) = path.split_once(&marker) else {
            continue;
        };
        if !prefix.is_empty() && !prefix.starts_with("__ow_archive__/history/") {
            continue;
        }
        let digest = name
            .strip_prefix("capture-")
            .and_then(|v| v.strip_suffix(".json"))
            .ok_or_else(|| Error(format!("invalid retained capture path: {path}")))?;
        let record = match crate::runtime_capture::inspect_retained(bytes, alias, digest) {
            Ok(value) => value,
            Err(error) if error.unknown => {
                gaps.insert(format!("{path}: {}: {}", error.code, error.message));
                continue;
            }
            Err(error) => return Err(Error(format!("{path}: {}: {}", error.code, error.message))),
        };
        let binding = reconnect(files, prefix, directory, warrant, &record);
        let (dispatch, connected) = match binding {
            Ok(dispatch) => (Some(dispatch), true),
            Err(ConnectionFault::Refused(reason)) => {
                return Err(Error(format!("{path}: {reason}")));
            }
            Err(ConnectionFault::Unknown(reason)) => {
                gaps.insert(format!("{path}: {reason}"));
                (None, false)
            }
        };
        let contract_connected = dispatch.as_ref().is_some_and(|d| {
            stages["declarations"]
                .as_array()
                .is_some_and(|declarations| {
                    declarations.iter().any(|s| {
                        s["manifest_source"] == format!("{prefix}{directory}/manifest.toml")
                            && s["contract_binding"]["reconstructed"] == true
                            && s["contract_binding"]["digest"]
                                .as_str()
                                .is_some_and(|digest| {
                                    digest.strip_prefix("sha256:").unwrap_or(digest)
                                        == d.contract_digest
                                            .strip_prefix("sha256:")
                                            .unwrap_or(&d.contract_digest)
                                })
                            && s["graph"]["stages"]
                                .as_array()
                                .is_some_and(|stages| stages.iter().any(|s| s["id"] == d.stage_id))
                    })
                })
        });
        if connected && !contract_connected {
            gaps.insert(format!(
                "{path}: exact captured contract/stage not reconstructed in this source snapshot"
            ));
        }
        records.push(json!({
            "source":path,"capture_digest":format!("sha256:{digest}"),
            "binding":record["observation"]["binding"],
            "provider_declaration":record["declared_capture"]["provider"],
            "original_receipt_ref":record["declared_capture"]["metadata"]["original_receipt_ref"],
            "receipt_bytes_digest":record["receipt"]["digest"],
            "dispatch_source_reconnected":connected,
            "contract_stage_reconstructed":contract_connected,
            "native_verification":"unknown","saved_native_observation_is_trusted":false,
            "current_attempt_eligibility_established":false,"assurance_granted":false
        }));
    }
    if records.is_empty() {
        gaps.insert(
            "No supported provider captures retained; runtime execution coverage is unknown".into(),
        );
    }
    Ok(
        json!({"records":records,"unresolved":gaps.into_iter().collect::<Vec<_>>(),
        "execution_coverage_established":false,"native_authentication_established":false}),
    )
}

fn blob(record: &Value, name: &str) -> Result<Vec<u8>, String> {
    openwarrant_core::attestation::base64_decode(
        record[name]["base64"]
            .as_str()
            .ok_or("missing embedded source bytes")?,
    )
}

enum ConnectionFault {
    Unknown(String),
    Refused(String),
}
impl From<String> for ConnectionFault {
    fn from(reason: String) -> Self {
        Self::Refused(reason)
    }
}
impl From<&str> for ConnectionFault {
    fn from(reason: &str) -> Self {
        Self::Refused(reason.into())
    }
}

fn reconnect(
    files: &BTreeMap<String, Vec<u8>>,
    prefix: &str,
    directory: &str,
    warrant: &str,
    record: &Value,
) -> Result<StageDispatch, ConnectionFault> {
    let dispatch_bytes = blob(record, "dispatch")?;
    let dispatch_value =
        crate::sdk::wire::decode_value(&dispatch_bytes).map_err(|e| e.to_string())?;
    if dispatch_value["api_version"] != openwarrant_core::execution::DISPATCH_API_VERSION {
        return Err(ConnectionFault::Unknown(
            "unsupported retained dispatch version".into(),
        ));
    }
    let dispatch: StageDispatch =
        serde_json::from_value(dispatch_value).map_err(|e| e.to_string())?;
    let dispatch_ref = format!("{directory}/dispatches/{}.json", dispatch.dispatch_id);
    if record["dispatch"]["reference"] != dispatch_ref {
        return Err("captured dispatch source reference differs".into());
    }
    let actual = files
        .get(&format!("{prefix}{dispatch_ref}"))
        .ok_or_else(|| ConnectionFault::Unknown("exact dispatch source not retained".into()))?;
    if actual != &dispatch_bytes {
        return Err("retained dispatch differs from captured source bytes".into());
    }
    let mut blank = dispatch.clone();
    blank.dispatch_digest.clear();
    let digest = openwarrant_compiler::canonical::sha256_digest(
        openwarrant_compiler::digest::DigestDomain::Dispatch,
        &blank,
    )
    .map_err(|e| e.to_string())?;
    if dispatch.warrant_ref != warrant
        || digest != dispatch.dispatch_digest
        || record["declared_capture"]["dispatch_id"] != dispatch.dispatch_id
    {
        return Err("captured dispatch identity or canonical digest differs".into());
    }
    let event_bytes = blob(record, "compile_event")?;
    let event_value = crate::sdk::wire::decode_value(&event_bytes).map_err(|e| e.to_string())?;
    if event_value["v"] != 1 {
        return Err(ConnectionFault::Unknown(
            "unsupported retained compile-event version".into(),
        ));
    }
    let event: JournalEvent = serde_json::from_value(event_value).map_err(|e| e.to_string())?;
    let journal_ref = format!("{directory}/journal.jsonl");
    let journal = files
        .get(&format!("{prefix}{journal_ref}"))
        .ok_or_else(|| {
            ConnectionFault::Unknown("captured compile-event journal not retained".into())
        })?;
    let reference = format!("{journal_ref}#event:{}", event.id);
    if record["compile_event"]["reference"] != reference {
        return Err("captured compile-event reference differs".into());
    }
    match journal
        .split(|b| *b == b'\n')
        .filter(|line| *line == event_bytes.as_slice())
        .count()
    {
        0 => {
            return Err(ConnectionFault::Unknown(
                "exact compile event not retained in the captured journal".into(),
            ));
        }
        1 => {}
        _ => return Err("captured compile event occurs more than once".into()),
    }
    let payload =
        crate::sdk::wire::decode_value(event.payload.as_bytes()).map_err(|e| e.to_string())?;
    let binding = &record["observation"]["binding"];
    if event.event_type != "dispatch.compiled"
        || format!("war://{}", event.warrant_uuid) != warrant
        || payload["dispatch_id"] != dispatch.dispatch_id
        || payload["dispatch_digest"] != digest
        || payload["contract_digest"] != dispatch.contract_digest
        || payload["stage"] != dispatch.stage_id
        || payload["attempt_id"] != dispatch.attempt_id
        || binding["warrant"] != warrant
        || binding["dispatch_digest"] != digest
        || binding["contract_digest"] != dispatch.contract_digest
        || binding["stage"] != dispatch.stage_id
        || binding["attempt"] != dispatch.attempt_id
    {
        return Err("captured observation/compile event does not bind the exact dispatch".into());
    }
    Ok(dispatch)
}
