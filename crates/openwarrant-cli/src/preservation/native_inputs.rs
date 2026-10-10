// SPDX-License-Identifier: Apache-2.0
//! Experimental native-input preservation. Byte reconnection is never native trust.
mod source;
use super::*;
use openwarrant_compiler::{sha256_hex, to_canonical_bytes};
use openwarrant_core::{attestation::base64_decode, execution::StageDispatch};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const SCHEMA: &str = "oh.war/preservation-runtime-inputs/v1-draft.1";
const REQUEST_SCHEMA: &str = "oh.war/preservation-runtime-inputs-request/v1-draft.1";
const MANIFEST_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    schema: String,
    public_key: String,
    plan: String,
    binding: String,
    producer: String,
    job: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    original_source: String,
    blob: String,
    digest: String,
    bytes: u64,
    mode: u32,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    capture_digest: String,
    provider: crate::runtime_capture::Provider,
    inputs: BTreeMap<String, Input>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    payload: Vec<u8>,
    signature: Vec<u8>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeFile {
    path: String,
    bytes: u64,
    digest: String,
    mode: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    version: u16,
    binding: Value,
    run_id: String,
    plan_digest: String,
    registry_digest: String,
    binary_digest: String,
    started_unix_ms: u64,
    ended_unix_ms: u64,
    outcome: String,
    warnings: usize,
    stages: usize,
    native_files: Vec<NativeFile>,
}
fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    let value = crate::sdk::wire::decode_value(bytes).map_err(|e| Error(e.to_string()))?;
    serde_json::from_value(value).map_err(|e| Error(e.to_string()))
}
fn blob(record: &Value, name: &str) -> Result<Vec<u8>, Error> {
    base64_decode(
        record[name]["base64"]
            .as_str()
            .ok_or_else(|| Error("native capture blob missing".into()))?,
    )
    .map_err(Error)
}
fn blake3(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}
fn capture_payload(record: &Value) -> Result<Payload, Error> {
    if record["declared_capture"]["provider"]["kind"] != "blut"
        || record["declared_capture"]["provider"]["version"]
            != crate::runtime_capture::blut_process::RESPONSE_SCHEMA
    {
        return Err(Error("unsupported native input provider interface".into()));
    }
    let envelope: Envelope = decode(&blob(record, "receipt")?)?;
    if envelope.signature.len() != 64 {
        return Err(Error("unsupported BLUT signature transport".into()));
    }
    let payload: Payload = decode(&envelope.payload)?;
    if payload.version != 1
        || payload.run_id.is_empty()
        || payload.ended_unix_ms < payload.started_unix_ms
        || !matches!(
            payload.outcome.as_str(),
            "completed" | "completed_with_warnings" | "failed" | "cancelled" | "deadline_exceeded"
        )
        || (payload.outcome == "completed" && (payload.stages == 0 || payload.warnings != 0))
        || (payload.outcome == "completed_with_warnings"
            && (payload.stages == 0 || payload.warnings == 0))
        || (matches!(
            payload.outcome.as_str(),
            "failed" | "cancelled" | "deadline_exceeded"
        ) && (payload.stages != 0 || payload.warnings != 0))
    {
        return Err(Error("unsupported or contradictory BLUT payload".into()));
    }
    let dispatch: StageDispatch = decode(&blob(record, "dispatch")?)?;
    let mut blank = dispatch.clone();
    blank.dispatch_digest.clear();
    let digest = openwarrant_compiler::canonical::sha256_digest(
        openwarrant_compiler::digest::DigestDomain::Dispatch,
        &blank,
    )
    .map_err(|e| Error(e.to_string()))?;
    if digest != dispatch.dispatch_digest
        || payload.binding
            != json!({"warrant_ref":dispatch.warrant_ref,"contract_digest":dispatch.contract_digest,"dispatch_digest":dispatch.dispatch_digest,"stage_id":dispatch.stage_id,"attempt_id":dispatch.attempt_id})
    {
        return Err(Error(
            "native input binding differs from captured Dispatch".into(),
        ));
    }
    let mut paths = BTreeSet::new();
    for file in &payload.native_files {
        source::safe(&file.path)?;
        if !paths.insert(file.path.clone()) {
            return Err(Error("duplicate native file path".into()));
        }
    }
    Ok(payload)
}
fn check_inputs(
    payload: &Payload,
    inputs: &BTreeMap<String, Input>,
    bytes: &BTreeMap<String, Vec<u8>>,
) -> Result<(), Error> {
    let expected: BTreeSet<_> = ["public_key", "plan", "binding", "producer"]
        .into_iter()
        .map(str::to_owned)
        .chain(
            payload
                .native_files
                .iter()
                .map(|f| format!("job/{}", f.path)),
        )
        .collect();
    if inputs.keys().cloned().collect::<BTreeSet<_>>() != expected
        || bytes.keys().cloned().collect::<BTreeSet<_>>() != expected
    {
        return Err(Error("native input roles or file table differ".into()));
    }
    for (role, input) in inputs {
        let actual = &bytes[role];
        let hex = sha256_hex(actual);
        source::safe(&input.original_source)?;
        if input.bytes != actual.len() as u64
            || input.digest != format!("sha256:{hex}")
            || input.blob != format!("sha256-{hex}.bin")
        {
            return Err(Error("native input bytes differ from manifest".into()));
        }
    }
    if bytes["public_key"].len() != 32 || blake3(&bytes["producer"]) != payload.binary_digest {
        return Err(Error(
            "native public key size or producer bytes differ".into(),
        ));
    }
    for file in &payload.native_files {
        let role = format!("job/{}", file.path);
        if bytes[&role].len() as u64 != file.bytes
            || blake3(&bytes[&role]) != file.digest
            || inputs[&role].mode != file.mode
        {
            return Err(Error(format!(
                "native job bytes or mode differ: {}",
                file.path
            )));
        }
    }
    let required = |name: &str| -> Result<&Vec<u8>, Error> {
        bytes
            .get(&format!("job/_openwarrant_inputs/{name}.json"))
            .ok_or_else(|| Error(format!("required native {name} input missing")))
    };
    let plan = required("plan")?;
    let registry = required("registry")?;
    let binding = required("binding")?;
    let supplied_plan: Value = decode(&bytes["plan"])?;
    let native_plan: Value = decode(plan)?;
    let supplied_binding: Value = decode(&bytes["binding"])?;
    let native_binding: Value = decode(binding)?;
    if blake3(plan) != payload.plan_digest
        || blake3(registry) != payload.registry_digest
        || supplied_plan != native_plan
        || supplied_binding != payload.binding
        || native_binding != json!({"dispatch":payload.binding,"run_id":payload.run_id})
    {
        return Err(Error(
            "native plan, registry or binding inputs differ".into(),
        ));
    }
    Ok(())
}

pub(super) fn retain(
    repo: &crate::repo::Repository,
    alias: &str,
    capture: &str,
    request: &Utf8PathBuf,
    limits: Limits,
) -> Result<Value, Error> {
    let request_bytes =
        crate::runtime_capture::read(repo, request, MANIFEST_LIMIT.min(limits.archive_bytes))
            .map_err(|e| Error(e.message))?;
    let request: Request = decode(&request_bytes)?;
    if request.schema != REQUEST_SCHEMA {
        return Err(Error("unsupported native input request version".into()));
    }
    let captured =
        crate::runtime_capture::show(repo, alias, capture).map_err(|e| Error(e.message))?;
    let record = &captured["record"];
    let payload = capture_payload(record)?;
    if payload.native_files.len().saturating_add(5) > limits.records {
        return Err(Error("native input record count exceeds limit".into()));
    }
    let actual_paths = source::paths(repo.root.as_std_path(), &request.job, limits.records)?;
    let declared_paths: BTreeSet<_> = payload
        .native_files
        .iter()
        .map(|f| f.path.clone())
        .collect();
    if actual_paths != declared_paths {
        return Err(Error(
            "native job file table differs from captured receipt".into(),
        ));
    }
    let mut sources = BTreeMap::from([
        ("public_key".to_owned(), request.public_key),
        ("plan".into(), request.plan),
        ("binding".into(), request.binding),
        ("producer".into(), request.producer),
    ]);
    sources.extend(payload.native_files.iter().map(|f| {
        (
            format!("job/{}", f.path),
            format!("{}/{}", request.job, f.path),
        )
    }));
    let mut remaining = limits.content_bytes;
    let mut bytes = BTreeMap::new();
    let mut inputs = BTreeMap::new();
    for (role, path) in sources {
        let (data, mode) = source::read(repo.root.as_std_path(), &path, &mut remaining)?;
        let hex = sha256_hex(&data);
        inputs.insert(
            role.clone(),
            Input {
                original_source: path,
                blob: format!("sha256-{hex}.bin"),
                digest: format!("sha256:{hex}"),
                bytes: data.len() as u64,
                mode,
            },
        );
        bytes.insert(role, data);
    }
    check_inputs(&payload, &inputs, &bytes)?;
    if source::paths(repo.root.as_std_path(), &request.job, limits.records)? != declared_paths {
        return Err(Error(
            "native job membership changed during retention".into(),
        ));
    }
    let provider = serde_json::from_value(record["declared_capture"]["provider"].clone())
        .map_err(|e| Error(e.to_string()))?;
    let manifest = Manifest {
        schema: SCHEMA.into(),
        capture_digest: captured["digest"]
            .as_str()
            .ok_or_else(|| Error("capture identity missing".into()))?
            .into(),
        provider,
        inputs,
    };
    let encoded = to_canonical_bytes(&manifest).map_err(|e| Error(e.to_string()))?;
    if encoded.len() > MANIFEST_LIMIT.min(limits.archive_bytes) {
        return Err(Error("native input manifest exceeds limit".into()));
    }
    let dir = repo.warrant_dir(alias).map_err(|e| Error(e.to_string()))?;
    let relative = dir
        .strip_prefix(&repo.root)
        .map_err(|e| Error(e.to_string()))?
        .join("native-inputs");
    let store = crate::bundle::store::Directory::open(&repo.root, &relative)
        .map_err(|e| Error(e.to_string()))?;
    for (role, input) in &manifest.inputs {
        store
            .retain(&input.blob, &bytes[role])
            .map_err(|e| Error(e.to_string()))?;
    }
    let name = format!("inputs-{}.json", sha256_hex(&encoded));
    store
        .retain(&name, &encoded)
        .map_err(|e| Error(e.to_string()))?;
    Ok(
        json!({"retained":true,"manifest":relative.join(name).as_str(),"capture_digest":manifest.capture_digest,"native_authentication_established":false,"authority_activated":false,"assurance_granted":false}),
    )
}

/// Reconnect every matching supported input observation without consulting its
/// original paths. Keys and modes remain data; no trust choice or execution occurs.
pub(in crate::preservation) fn reconnect(
    files: &BTreeMap<String, Vec<u8>>,
    directory: &str,
    record: &Value,
    capture_digest: &str,
) -> Result<BTreeSet<String>, Error> {
    let payload = capture_payload(record)?;
    let marker = format!("{directory}/native-inputs/");
    let mut paths = BTreeSet::new();
    let mut matched = false;
    for (path, encoded) in files {
        let Some((prefix, name)) = path.split_once(&marker) else {
            continue;
        };
        if (!prefix.is_empty() && !prefix.starts_with("__ow_archive__/history/"))
            || !name.starts_with("inputs-")
            || !name.ends_with(".json")
        {
            continue;
        }
        if encoded.len() > MANIFEST_LIMIT {
            return Err(Error("native input manifest exceeds limit".into()));
        }
        let value: Value = decode(encoded)?;
        if value["schema"] != SCHEMA {
            return Err(Error("unsupported native input manifest version".into()));
        }
        let manifest: Manifest = serde_json::from_value(value).map_err(|e| Error(e.to_string()))?;
        if to_canonical_bytes(&manifest).map_err(|e| Error(e.to_string()))? != *encoded
            || name != format!("inputs-{}.json", sha256_hex(encoded))
        {
            return Err(Error(
                "native input manifest identity or canonical bytes differ".into(),
            ));
        }
        if manifest.capture_digest != capture_digest {
            continue;
        }
        if serde_json::to_value(&manifest.provider).map_err(|e| Error(e.to_string()))?
            != record["declared_capture"]["provider"]
        {
            return Err(Error("native input provider differs from capture".into()));
        }
        let mut bytes = BTreeMap::new();
        let base = path
            .rsplit_once('/')
            .ok_or_else(|| Error("native manifest path invalid".into()))?
            .0;
        for (role, input) in &manifest.inputs {
            // The filename is derived independently from the declared digest;
            // untrusted paths never select a file outside this retained directory.
            let hex = input
                .digest
                .strip_prefix("sha256:")
                .ok_or_else(|| Error("native input digest invalid".into()))?;
            if hex.len() != 64
                || !hex
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || input.blob != format!("sha256-{hex}.bin")
            {
                return Err(Error("native input blob identity invalid".into()));
            }
            let blob_path = format!("{base}/{}", input.blob);
            let data = files
                .get(&blob_path)
                .ok_or_else(|| Error(format!("native input blob missing: {blob_path}")))?;
            bytes.insert(role.clone(), data.clone());
            paths.insert(blob_path);
        }
        check_inputs(&payload, &manifest.inputs, &bytes)?;
        paths.insert(path.clone());
        matched = true;
    }
    if !matched {
        return Err(Error(
            "native input manifest not retained for capture".into(),
        ));
    }
    Ok(paths)
}
