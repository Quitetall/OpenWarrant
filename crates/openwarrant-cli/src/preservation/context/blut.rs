// SPDX-License-Identifier: Apache-2.0
//! BLUT input byte closure, not native authentication or execution qualification.
use openwarrant_core::{execution::StageDispatch, milestones::Stage};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn collect(
    files: &BTreeMap<String, Vec<u8>>,
    prefix: &str,
    directory: &str,
    alias: &str,
    warrant: &str,
    stage: &Stage,
) -> Result<BTreeSet<String>, String> {
    let root = format!("{prefix}{directory}/dispatches/");
    let marker = format!("{directory}/runtime-receipts/");
    let mut paths = BTreeSet::new();
    for (path, bytes) in files.iter().filter(|(p, _)| p.starts_with(&root)) {
        let value = crate::sdk::wire::decode_value(bytes).map_err(|e| e.to_string())?;
        let dispatch: StageDispatch = serde_json::from_value(value).map_err(|e| e.to_string())?;
        if dispatch.stage_id != stage.id {
            continue;
        }
        if path != &format!("{root}{}.json", dispatch.dispatch_id)
            || dispatch.warrant_ref != warrant
        {
            return Err(format!("{path}: dispatch identity differs"));
        }
        let mut matched = false;
        let mut reasons = BTreeSet::new();
        for (capture_path, capture_bytes) in files {
            let Some((capture_prefix, name)) = capture_path.split_once(&marker) else {
                continue;
            };
            if !capture_prefix.is_empty() && !capture_prefix.starts_with("__ow_archive__/history/")
            {
                continue;
            }
            let Some(hex) = name
                .strip_prefix("capture-")
                .and_then(|n| n.strip_suffix(".json"))
            else {
                continue;
            };
            let record = crate::runtime_capture::inspect_retained(capture_bytes, alias, hex)
                .map_err(|e| format!("{capture_path}: {}", e.message))?;
            if record["declared_capture"]["provider"]["kind"] != "blut"
                || record["declared_capture"]["dispatch_id"] != dispatch.dispatch_id
            {
                continue;
            }
            let connected = match super::super::runtime_basis::captures::reconnect_for_coverage(
                files,
                capture_prefix,
                directory,
                warrant,
                &record,
            ) {
                Ok(value) => value,
                Err(error) => {
                    reasons.insert(error);
                    continue;
                }
            };
            if connected != dispatch {
                reasons.insert("capture and selected Dispatch differ".into());
                continue;
            }
            let inputs = match super::super::native_inputs::reconnect(
                files,
                directory,
                &record,
                &format!("sha256:{hex}"),
            ) {
                Ok(paths) => paths,
                Err(error) => {
                    reasons.insert(error.to_string());
                    continue;
                }
            };
            paths.extend(inputs);
            paths.extend([
                path.clone(),
                capture_path.clone(),
                format!(
                    "{capture_prefix}{directory}/dispatches/{}.json",
                    dispatch.dispatch_id
                ),
                format!("{capture_prefix}{directory}/journal.jsonl"),
            ]);
            matched = true;
        }
        if !matched {
            return Err(format!(
                "{path}: native capture/input closure unavailable: {}",
                reasons.into_iter().collect::<Vec<_>>().join("; ")
            ));
        }
    }
    Ok(paths)
}
