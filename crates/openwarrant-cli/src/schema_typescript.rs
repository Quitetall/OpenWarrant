// SPDX-License-Identifier: Apache-2.0
//! Structural TypeScript projection. JSON Schema remains runtime authority.
use std::collections::BTreeMap;

use serde_json::Value;

fn quote(value: &str) -> String {
    serde_json::to_string(value).expect("string serialization")
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .enumerate()
            .all(|(i, b)| b.is_ascii_alphabetic() || b == b'_' || (i > 0 && b.is_ascii_digit()))
}

struct Emitter<'a> {
    definitions: &'a serde_json::Map<String, Value>,
}

impl Emitter<'_> {
    fn expression(&self, schema: &Value, depth: usize) -> Result<String, String> {
        if depth > 128 {
            return Err("TypeScript schema nesting limit exceeded".into());
        }
        if let Some(boolean) = schema.as_bool() {
            return Ok(if boolean { "unknown" } else { "never" }.into());
        }
        let object = schema
            .as_object()
            .ok_or("Schema must be an object or boolean")?;
        if depth > 0
            && ["$defs", "$id", "$schema"]
                .iter()
                .any(|key| object.contains_key(*key))
        {
            return Err("Nested schema scopes unsupported".into());
        }
        for key in object.keys() {
            if !matches!(
                key.as_str(),
                "$schema"
                    | "$id"
                    | "$defs"
                    | "$ref"
                    | "title"
                    | "description"
                    | "default"
                    | "type"
                    | "const"
                    | "enum"
                    | "anyOf"
                    | "oneOf"
                    | "properties"
                    | "required"
                    | "additionalProperties"
                    | "items"
                    | "prefixItems"
                    | "format"
                    | "minimum"
                    | "maximum"
                    | "minItems"
                    | "maxItems"
                    | "uniqueItems"
            ) {
                return Err(format!("Unsupported TypeScript schema keyword: {key}"));
            }
        }
        if let Some(kinds) = object.get("type").and_then(Value::as_array) {
            if kinds.is_empty() {
                return Err("Empty type union unsupported".into());
            }
            let alternatives = kinds
                .iter()
                .map(|kind| {
                    if !kind.is_string() {
                        return Err("Invalid type union member".into());
                    }
                    let mut branch = schema.clone();
                    branch["type"] = kind.clone();
                    self.expression(&branch, depth + 1)
                        .map(|text| format!("({text})"))
                })
                .collect::<Result<Vec<_>, String>>()?;
            return Ok(alternatives.join(" | "));
        }
        let mut parts = Vec::new();
        if let Some(reference) = object.get("$ref") {
            let reference = reference.as_str().ok_or("Invalid schema reference")?;
            let name = reference
                .strip_prefix("#/$defs/")
                .ok_or("Only local $defs references supported")?;
            if !identifier(name) || !self.definitions.contains_key(name) {
                return Err(format!(
                    "Unresolved or unsupported schema reference: {reference}"
                ));
            }
            parts.push(format!("Def_{name}"));
        }
        if let Some(value) = object.get("const") {
            parts.push(literal(value)?);
        }
        if let Some(values) = object.get("enum") {
            let values = values.as_array().ok_or("Invalid enum")?;
            if values.is_empty() {
                return Err("Empty enum unsupported".into());
            }
            parts.push(
                values
                    .iter()
                    .map(literal)
                    .collect::<Result<Vec<_>, _>>()?
                    .join(" | "),
            );
        }
        for key in ["anyOf", "oneOf"] {
            if let Some(values) = object.get(key) {
                let values = values.as_array().ok_or("Invalid union")?;
                if values.is_empty() {
                    return Err("Empty union unsupported".into());
                }
                parts.push(
                    values
                        .iter()
                        .map(|v| self.expression(v, depth + 1).map(|s| format!("({s})")))
                        .collect::<Result<Vec<_>, _>>()?
                        .join(" | "),
                );
            }
        }
        if let Some(kind) = object.get("type") {
            let kind = kind
                .as_str()
                .ok_or("Non-string type unsupported; use explicit union")?;
            parts.push(match kind {
                "string" => "string".into(),
                "integer" | "number" => "number".into(),
                "boolean" => "boolean".into(),
                "null" => "null".into(),
                "object" => self.object(schema, depth + 1)?,
                "array" => self.array(schema, depth + 1)?,
                _ => return Err(format!("Unsupported schema type: {kind}")),
            });
        }
        for (key, expected) in [
            ("properties", "object"),
            ("required", "object"),
            ("additionalProperties", "object"),
            ("items", "array"),
            ("prefixItems", "array"),
        ] {
            if object.contains_key(key) && schema["type"] != expected {
                return Err(format!("{key} requires explicit {expected} type"));
            }
        }
        Ok(if parts.is_empty() {
            "unknown".into()
        } else {
            parts
                .into_iter()
                .map(|part| format!("({part})"))
                .collect::<Vec<_>>()
                .join(" & ")
        })
    }

    fn object(&self, schema: &Value, depth: usize) -> Result<String, String> {
        let empty = serde_json::Map::new();
        let properties = match schema.get("properties") {
            Some(v) => v.as_object().ok_or("Invalid properties")?,
            None => &empty,
        };
        let required = match schema.get("required") {
            Some(v) => v
                .as_array()
                .ok_or("Invalid required fields")?
                .iter()
                .map(|v| v.as_str().ok_or("Invalid required name"))
                .collect::<Result<Vec<_>, _>>()?,
            None => Vec::new(),
        };
        if required.iter().any(|name| !properties.contains_key(*name)) {
            return Err("Required field without declared property unsupported".into());
        }
        let mut fields = Vec::new();
        for (name, value) in properties {
            fields.push(format!(
                "{}{}: {};",
                quote(name),
                if required.contains(&name.as_str()) {
                    ""
                } else {
                    "?"
                },
                self.expression(value, depth + 1)?
            ));
        }
        match schema.get("additionalProperties") {
            None | Some(Value::Bool(true)) => fields.push("[key: string]: unknown;".into()),
            Some(Value::Bool(false)) => {
                if properties.is_empty() {
                    fields.push("[key: string]: never;".into());
                }
            }
            Some(value) => {
                if !properties.is_empty() {
                    return Err("Typed additional properties with named fields unsupported".into());
                }
                fields.push(format!(
                    "[key: string]: {};",
                    self.expression(value, depth + 1)?
                ));
            }
        }
        Ok(format!("{{ {} }}", fields.join(" ")))
    }

    fn array(&self, schema: &Value, depth: usize) -> Result<String, String> {
        if let Some(prefix) = schema.get("prefixItems") {
            let prefix = prefix.as_array().ok_or("Invalid tuple")?;
            let length = prefix.len() as u64;
            if schema["minItems"].as_u64() != Some(length)
                || schema["maxItems"].as_u64() != Some(length)
                || schema
                    .get("items")
                    .is_some_and(|v| v != &Value::Bool(false))
            {
                return Err("Only fixed-length prefixItems tuples supported".into());
            }
            return Ok(format!(
                "[{}]",
                prefix
                    .iter()
                    .map(|v| self.expression(v, depth + 1))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            ));
        }
        let item = self.expression(schema.get("items").unwrap_or(&Value::Bool(true)), depth + 1)?;
        Ok(format!("Array<{item}>"))
    }
}

fn literal(value: &Value) -> Result<String, String> {
    match value {
        Value::String(_) | Value::Number(_) | Value::Bool(_) | Value::Null => Ok(value.to_string()),
        _ => Err("Structured const/enum values unsupported".into()),
    }
}

pub fn render(files: &BTreeMap<String, String>) -> Result<BTreeMap<String, String>, String> {
    let mut output = BTreeMap::new();
    for (record, text) in files {
        if record.is_empty()
            || !record
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err("Invalid TypeScript artifact name".into());
        }
        let schema: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if schema["$schema"] != "https://json-schema.org/draft/2020-12/schema" {
            return Err("TypeScript requires JSON Schema draft 2020-12".into());
        }
        let empty = serde_json::Map::new();
        let definitions = match schema.get("$defs") {
            Some(v) => v.as_object().ok_or("Invalid definitions")?,
            None => &empty,
        };
        let emitter = Emitter { definitions };
        let mut body = String::from(
            "// SPDX-License-Identifier: Apache-2.0\n// Generated by war schemas. Do not edit.\n// Structural types only; retain JSON Schema runtime validation.\n",
        );
        for (name, value) in definitions {
            if !identifier(name) {
                return Err(format!("Unsupported definition name: {name}"));
            }
            body.push_str(&format!(
                "type Def_{name} = {};\n",
                emitter.expression(value, 1)?
            ));
        }
        body.push_str(&format!(
            "export type Document = {};\n",
            emitter.expression(&schema, 0)?
        ));
        output.insert(format!("{record}.ts"), body);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn emit(mut value: Value) -> Result<String, String> {
        value["$schema"] = json!("https://json-schema.org/draft/2020-12/schema");
        Ok(render(&BTreeMap::from([("fixture".into(), value.to_string())]))?["fixture.ts"].clone())
    }

    #[test]
    fn refuses_unknown_keywords_and_missing_references() {
        assert!(
            emit(json!({"type":"string","pattern":"x"}))
                .unwrap_err()
                .contains("Unsupported")
        );
        assert!(
            emit(json!({"$ref":"#/$defs/Missing"}))
                .unwrap_err()
                .contains("Unresolved")
        );
        assert!(emit(json!({"$ref":"https://example.invalid/schema"})).is_err());
    }

    #[test]
    fn preserves_required_nullable_and_tuple_shapes() {
        let value = json!({"type":"object","additionalProperties":false,"required":["pair"],"properties":{
            "pair":{"type":"array","prefixItems":[{"type":"string"},{"type":"integer"}],"minItems":2,"maxItems":2},
            "maybe":{"anyOf":[{"type":"string"},{"type":"null"}]}}});
        let body = emit(value.clone()).unwrap();
        assert!(body.contains("\"pair\": ([(string), (number)])"));
        assert!(body.contains("\"maybe\"?:"));
        assert_eq!(body, emit(value).unwrap());
    }
}
