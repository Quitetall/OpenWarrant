// SPDX-License-Identifier: Apache-2.0
//! Portable, experimental canonical producer shapes. No format adoption,
//! authentication, permissions, freshness or assurance follow from validation.
use super::*;
use crate::runtime_capture::{SelectionRequest, collector_loading::NativePolicyBinding};
use schemars::{JsonSchema, SchemaGenerator, generate::SchemaSettings};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const PACK_DIR: &str = "schemas/native-draft";
const POLICY: &str = "oh.war/runtime-native-policy-binding/v1-draft.1";
const SELECTION: &str = "oh.war/runtime-selection-request/v1-draft.1";
const RESULT: &str = "oh.war/native-signing-result/v1-draft.1";
const SIGNED: &str = "oh.war/native-signed-proposal/v1-draft.1";

fn canonical(value: &impl Serialize) -> Result<String, RepoError> {
    serde_jcs::to_string(value)
        .map(|s| s + "\n")
        .map_err(|e| RepoError::Message(format!("native schema canonicalization: {e}")))
}

fn root<T: JsonSchema>(id: &str) -> Value {
    let schema = SchemaGenerator::new(SchemaSettings::draft2020_12()).root_schema_for::<T>();
    let mut value = serde_json::to_value(schema).expect("schema serializes");
    value["$id"] = json!(id);
    value["additionalProperties"] = json!(false);
    value["properties"]["schema"] = json!({"const":id,"type":"string"});
    value
}

fn response_constraints(value: &mut Value) {
    let p = &mut value["properties"];
    p["schema"] = json!({"const":RESPONSE_SCHEMA,"type":"string"});
    p["acting_role"] = json!({"const":"resolver","type":"string"});
    p["meaning"]["pattern"] = json!("\\S");
    p["effective_time"]["format"] = json!("date-time");
    p["selections"]["maxItems"] = json!(256);
    for name in ["warrant_uuid", "resolution_id"] {
        p[name]["format"] = json!("uuid");
        p[name]["pattern"] =
            json!("^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$");
    }
    value["additionalProperties"] = json!(false);
}

fn locator_constraints(value: &mut Value) {
    value["additionalProperties"] = json!(false);
    value["properties"]["commit_sha"]["pattern"] = json!("^[0-9a-f]{40}$");
    value["properties"]["worktree_clean"] = json!({"type":"boolean","const":true});
    value["properties"]["paths_dirty"]["maxItems"] = json!(0);
}

fn definitions(value: &mut Value) {
    if let Some(defs) = value.get_mut("$defs").and_then(Value::as_object_mut) {
        if let Some(v) = defs.get_mut("Response") {
            response_constraints(v);
        }
        if let Some(v) = defs.get_mut("Locator") {
            locator_constraints(v);
        }
        if let Some(v) = defs.get_mut("NativePolicyBinding") {
            v["properties"]["schema"] = json!({"const":POLICY,"type":"string"});
        }
    }
}

fn schemas() -> BTreeMap<String, Value> {
    let mut response = root::<Response>(RESPONSE_SCHEMA);
    response_constraints(&mut response);
    let mut record = root::<ResolutionRecord>(RECORD_SCHEMA);
    record["properties"]["native_response"] = json!({"type":"string","minLength":1,
        "description":"Exact original TOML; runtime enforces the 65536-byte budget."});
    record["properties"]["native_signature"] = json!({"type":"string","minLength":1,
        "description":"Signature text; cryptographic validity is not evaluated by this schema."});
    record["properties"]["locator"] = json!({"$ref":"#/$defs/Locator"});
    let required = record["required"]
        .as_array_mut()
        .expect("required record fields");
    for name in ["locator", "native_response", "native_signature"] {
        if !required.contains(&json!(name)) {
            required.push(json!(name));
        }
    }
    let mut result = root::<signing::ResultPacket>(RESULT);
    result["required"] = json!(["schema", "response", "recorded", "signed_proposal_path"]);
    let signed = json!({"$schema":"https://json-schema.org/draft/2020-12/schema",
        "$id":SIGNED,"type":"object","additionalProperties":false,
        "required":["schema","response","signature"],"properties":{
            "schema":{"type":"string","const":SIGNED},
            "response":{"type":"string","minLength":1},
            "signature":{"type":"string","minLength":1}}});
    let policy = root::<NativePolicyBinding>(POLICY);
    let mut selection = root::<SelectionRequest>(SELECTION);
    selection["properties"]["selections"]["maxItems"] = json!(256);
    let mut files = BTreeMap::from([
        ("response".into(), response),
        ("resolution".into(), record),
        ("signing-result".into(), result),
        ("signed-proposal".into(), signed),
        ("policy".into(), policy),
        ("selection-request".into(), selection),
    ]);
    for value in files.values_mut() {
        definitions(value);
    }
    files
}

fn example() -> Response {
    let digest = format!("sha256:{}", "0".repeat(64));
    Response {
        schema: RESPONSE_SCHEMA.into(),
        warrant: "EXAMPLE-ONLY".into(),
        warrant_uuid: "01a12897-783e-71c5-a729-22bc7d21adfa".into(),
        resolution_id: "01a12897-783e-71c5-a729-22bc7d21adfb".into(),
        resolved_by: "example-only".into(),
        acting_role: "resolver".into(),
        common_outcome: CommonOutcome::Cancelled,
        profile_outcome: "cancelled".into(),
        meaning: "Unsigned format illustration only; no actual decision or approval.".into(),
        effective_time: "2026-10-11T00:00:00Z".into(),
        contract_revision: 1,
        contract_digest: "0".repeat(64),
        assurance_case_snapshot_digest: digest.clone(),
        artifact_manifest_digest: digest.clone(),
        gate_run_refs: Vec::new(),
        judgment_refs: Vec::new(),
        residual_risk_refs: Vec::new(),
        selections: vec![Selection {
            stage_id: "EXAMPLE-STAGE".into(),
            capture_digest: digest.clone(),
        }],
        policy: NativePolicyBinding {
            schema: POLICY.into(),
            repository: "example-only".into(),
            collector: "example-only".into(),
            authority_digest: digest.clone(),
            active_enrollment_digest: digest.clone(),
            host_configuration_digest: digest.clone(),
            native_configuration_digest: digest,
        },
        require_user_presence: true,
        locator: Locator {
            commit_sha: "a".repeat(40),
            worktree_clean: true,
            paths_dirty: Vec::new(),
        },
    }
}

/// All pack artifacts are produced without repository, service or model reads.
pub fn render() -> Result<BTreeMap<String, String>, RepoError> {
    let schema_text: BTreeMap<_, _> = schemas()
        .into_iter()
        .map(|(name, value)| canonical(&value).map(|s| (name, s)))
        .collect::<Result<_, _>>()?;
    let mut files = BTreeMap::new();
    for (name, text) in &schema_text {
        files.insert(format!("{name}.json"), text.clone());
    }
    let ts = crate::schema_typescript::render(&schema_text).map_err(RepoError::Message)?;
    let source_digests: BTreeMap<_, _> = schema_text
        .iter()
        .map(|(n, t)| (format!("{n}.json"), sha256_hex(t.as_bytes())))
        .collect();
    let ts_digests: BTreeMap<_, _> = ts
        .iter()
        .map(|(n, t)| (n.clone(), sha256_hex(t.as_bytes())))
        .collect();
    for (name, text) in ts {
        files.insert(format!("typescript/{name}"), text);
    }
    files.insert(
        "typescript/manifest.json".into(),
        canonical(&json!({
        "schema":"oh.war/native-typescript-projection/v1-draft.1",
        "source_files":source_digests,"artifacts":ts_digests,"runtime_validation_required":true}))?,
    );
    let response = example();
    let wire = toml::to_string(&response).map_err(|e| RepoError::Message(e.to_string()))?;
    // Parsing proves only the unsigned wire shape. It deliberately grants nothing.
    parse(wire.as_bytes()).map_err(|e| RepoError::Message(e.message))?;
    let signature = "UNSIGNED EXAMPLE; NOT A SIGNATURE";
    let record = ResolutionRecord {
        schema: RECORD_SCHEMA.into(),
        warrant: response.warrant.clone(),
        resolution: response.resolution("2026-10-11T00:00:00Z"),
        locator: Some(response.locator.clone()),
        native_response: Some(wire.clone()),
        native_signature: Some(signature.into()),
    };
    let values = [
        ("response", serde_json::to_value(&response)),
        ("resolution", serde_json::to_value(&record)),
        (
            "signing-result",
            serde_json::to_value(signing::ResultPacket {
                schema: RESULT,
                response: response.clone(),
                recorded: false,
                signed_proposal_path: None,
            }),
        ),
        (
            "signed-proposal",
            Ok(json!({"schema":SIGNED,"response":wire,"signature":signature})),
        ),
        ("policy", serde_json::to_value(&response.policy)),
        (
            "selection-request",
            serde_json::to_value(SelectionRequest {
                schema: SELECTION.into(),
                selections: response.selections,
            }),
        ),
    ];
    for (name, value) in values {
        files.insert(
            format!("examples/{name}.json"),
            canonical(&value.map_err(|e| RepoError::Message(e.to_string()))?)?,
        );
    }
    files.insert("examples/response.toml".into(), wire);
    files.insert("README.md".into(), "# Experimental native acceptance pack\n\nGenerated by `war admin schemas` (feature `schema`); do not edit.\n\nThese schemas describe canonical producer shapes for six draft formats. They\ndo not adopt a format, change the active v1 pack or bind a contract. TypeScript\nprovides structural types only; retain JSON Schema runtime validation.\n\nThe portable examples are unsigned illustrations, not project records, human\ndecisions, signatures or qualification. JSON Schema success proves no authority.\nOriginal TOML bytes are retained exactly in the example record and proposal.\nSign the original TOML; do not reconstruct signature input from JSON. Enable\ndate-time format assertions in your JSON Schema validator.\n\nActual readers still enforce byte budgets, native version support, exact signed\nfield bindings, trusted keys, permissions, presence and fresh evidence. Canonical\nUUIDs use lowercase hyphenated v7 form; permissive parsing is not authorization.\n\n`pack.json` hashes every schema, type, example and this guide. The transitive\ndigest uses sorted relative-path:sha256 lines. TypeScript's manifest pins its\nsource schemas without a circular dependency on the containing pack.\n\nValidate offline with `python3 conformance/native_schema_pack.py schemas/native-draft`\n(jsonschema 4.26.0). `war admin schemas --check` refuses missing or changed artifacts.\n".into());
    let digests: BTreeMap<_, _> = files
        .iter()
        .map(|(n, t)| (n.clone(), sha256_hex(t.as_bytes())))
        .collect();
    let lines: String = digests.iter().map(|(n, d)| format!("{n}:{d}\n")).collect();
    files.insert("pack.json".into(),canonical(&json!({"schema":"oh.war/native-schema-pack/v1-draft.1",
        "id":"oh.war/native-acceptance-draft","version":"1-draft.1","experimental":true,
        "runtime_validation_required":true,"files":digests,"transitive_digest":sha256_hex(lines.as_bytes())}))?);
    Ok(files)
}
