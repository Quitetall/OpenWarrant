// SPDX-License-Identifier: Apache-2.0
//! Exact-byte preservation and successor mapping. No filesystem access or authority transfer.
use super::{Dialect, ParseLimits, parse_document, source::raw_digest};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
mod codec;
pub const SCHEMA: &str = "oh.war/preservation/1.0.0-rc.3";
pub const ADAPTER_VERSION: &str = "ow-sdk-preservation/1";
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub files: usize,
    pub file_bytes: usize,
    pub total_bytes: usize,
    pub wire_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            files: 4096,
            file_bytes: 8 * 1024 * 1024,
            total_bytes: 64 * 1024 * 1024,
            wire_bytes: 256 * 1024 * 1024,
        }
    }
}
#[derive(Debug, thiserror::Error)]
#[error("{code}: {message}")]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}
fn err(code: &'static str, message: impl Into<String>) -> Error {
    Error {
        code,
        message: message.into(),
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub path: String,
    pub digest: String,
    pub bytes: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    schema: String,
    source_dialect: String,
    adapter_version: String,
    entry: String,
    inventory: Vec<Entry>,
    files: BTreeMap<String, Vec<u8>>,
}
#[derive(Debug)]
pub struct Report {
    pub original_schema: String,
    pub original_identity: String,
    pub original_state: Option<String>,
    pub historical_fields: BTreeMap<String, String>,
    pub historical_closure_established: bool,
    pub inventory_preserved: bool,
    pub qualification_established: bool,
    pub unsupported: Vec<String>,
}
#[derive(Debug)]
pub struct Preserved {
    payload: Payload,
    report: Report,
}
impl Preserved {
    pub fn files(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.payload.files
    }
    pub fn report(&self) -> &Report {
        &self.report
    }
    pub fn inventory(&self) -> &[Entry] {
        &self.payload.inventory
    }
}
fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 4096
        && !path.contains('\\')
        && !(path.as_bytes().get(1) == Some(&b':') && path.as_bytes()[0].is_ascii_alphabetic())
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}
fn bounds(files: &BTreeMap<String, Vec<u8>>, limits: Limits) -> Result<(), Error> {
    if limits.files == 0
        || limits.file_bytes == 0
        || limits.total_bytes == 0
        || limits.wire_bytes == 0
        || files.len() > limits.files
    {
        return Err(err("resource-limit", "Invalid limits or too many files"));
    }
    let mut total = 0usize;
    for (path, bytes) in files {
        if !safe_path(path) {
            return Err(err("legacy.path", path));
        }
        total = total
            .checked_add(path.len())
            .and_then(|n| n.checked_add(bytes.len()))
            .ok_or_else(|| err("resource-limit", "Size overflow"))?;
        if bytes.len() > limits.file_bytes || total > limits.total_bytes {
            return Err(err("resource-limit", "Preservation byte limit"));
        }
    }
    Ok(())
}
/// Capture a supplied inventory of regular-file bytes. Filesystem wrappers must reject links.
/// Completeness means this inventory only, not an inferred historical dependency closure.
pub fn capture(
    dialect: &str,
    adapter: &str,
    entry: &str,
    files: BTreeMap<String, Vec<u8>>,
    limits: Limits,
) -> Result<Preserved, Error> {
    if !matches!(dialect, "rc2" | "rc3" | "legacy-warrant-v1") {
        return Err(err("legacy.dialect", "Unsupported source dialect"));
    }
    if adapter != ADAPTER_VERSION {
        return Err(err("legacy.version", "Unsupported adapter version"));
    }
    if !safe_path(entry) {
        return Err(err("legacy.path", "Unsafe entry path"));
    }
    bounds(&files, limits)?;
    let inventory = files
        .iter()
        .map(|(path, bytes)| Entry {
            path: path.clone(),
            digest: raw_digest(bytes),
            bytes: bytes.len(),
        })
        .collect();
    check(
        Payload {
            schema: SCHEMA.into(),
            source_dialect: dialect.into(),
            adapter_version: adapter.into(),
            entry: entry.into(),
            inventory,
            files,
        },
        limits,
    )
}
fn check(payload: Payload, limits: Limits) -> Result<Preserved, Error> {
    bounds(&payload.files, limits)?;
    if payload.schema != SCHEMA || payload.adapter_version != ADAPTER_VERSION {
        return Err(err(
            "legacy.version",
            "Unsupported preservation schema or adapter version",
        ));
    }
    if !matches!(
        payload.source_dialect.as_str(),
        "rc2" | "rc3" | "legacy-warrant-v1"
    ) {
        return Err(err("legacy.dialect", &payload.source_dialect));
    }
    if !safe_path(&payload.entry) {
        return Err(err("legacy.path", &payload.entry));
    }
    if payload.inventory.len() != payload.files.len() || payload.inventory.len() > limits.files {
        return Err(err(
            "legacy.inventory",
            "Inventory and supplied files differ",
        ));
    }
    let mut paths = BTreeSet::new();
    for item in &payload.inventory {
        if !safe_path(&item.path) || !paths.insert(&item.path) {
            return Err(err(
                "legacy.inventory",
                "Unsafe or duplicate inventory path",
            ));
        }
        let bytes = payload
            .files
            .get(&item.path)
            .ok_or_else(|| err("legacy.missing", &item.path))?;
        if bytes.len() != item.bytes || raw_digest(bytes) != item.digest {
            return Err(err("legacy.integrity", &item.path));
        }
    }
    let bytes = payload
        .files
        .get(&payload.entry)
        .ok_or_else(|| err("legacy.missing", &payload.entry))?;
    let text =
        std::str::from_utf8(bytes).map_err(|_| err("legacy.syntax", "Entry is not UTF-8"))?;
    let mut report=Report {original_schema:String::new(),original_identity:String::new(),original_state:None,historical_fields:BTreeMap::new(),historical_closure_established:false,inventory_preserved:true,qualification_established:false,unsupported:vec!["Historical signature authenticity and acceptance meaning are not established by preservation".into()]};
    if payload.source_dialect == "legacy-warrant-v1" {
        let manifest: crate::manifest::Manifest =
            toml::from_str(text).map_err(|e| err("legacy.syntax", e.to_string()))?;
        manifest
            .validate(None)
            .map_err(|e| err("legacy.syntax", e.to_string()))?;
        report.original_schema = manifest.schema;
        report.original_identity = manifest.uuid.clone();
        let directory = payload.entry.rsplit_once('/').map_or("", |(base, _)| base);
        for atom in &manifest.atoms {
            if let Some(path) = &atom.path {
                if !safe_path(path) {
                    return Err(err("legacy.path", path));
                }
                let key = if directory.is_empty() {
                    path.clone()
                } else {
                    format!("{directory}/{path}")
                };
                let Some(bytes) = payload.files.get(&key) else {
                    report
                        .unsupported
                        .push(format!("Historical atom unavailable: {key}"));
                    continue;
                };
                // Markdown atoms retain the existing restricted frontmatter parser.
                if path.ends_with(".md") {
                    let text =
                        std::str::from_utf8(bytes).map_err(|_| err("legacy.syntax", &key))?;
                    let atom = crate::frontmatter::parse(text)
                        .map_err(|e| err("legacy.syntax", e.to_string()))?;
                    if atom.scalar("schema") != Some("oh.war/atom/v1")
                        || atom.scalar("warrant_uuid") != Some(manifest.uuid.as_str())
                    {
                        return Err(err("legacy.atom", "Wrong atom schema or Warrant identity"));
                    }
                }
            } else if let Some(reference) = &atom.r#ref {
                report
                    .unsupported
                    .push(format!("Bound atom not resolved: {reference}"));
            }
        }
        for name in ["authorization.toml", "resolution.toml"] {
            let key = if directory.is_empty() {
                name.into()
            } else {
                format!("{directory}/{name}")
            };
            if let Some(bytes) = payload.files.get(&key) {
                let value: toml::Value = toml::from_str(
                    std::str::from_utf8(bytes).map_err(|_| err("legacy.syntax", &key))?,
                )
                .map_err(|e| err("legacy.syntax", e.to_string()))?;
                let expected = if name == "authorization.toml" {
                    "oh.war/authorization/v1"
                } else {
                    "oh.war/resolution/v1"
                };
                if value.get("schema").and_then(toml::Value::as_str) != Some(expected)
                    || value.get("warrant").and_then(toml::Value::as_str)
                        != Some(manifest.local_alias.as_str())
                {
                    return Err(err("legacy.record", "Wrong historical schema or alias"));
                }
                for (section, fields) in [
                    (
                        "revision",
                        &["state", "contract_digest", "predecessor_digest"][..],
                    ),
                    (
                        "resolution",
                        &[
                            "common_outcome",
                            "profile_outcome",
                            "standing",
                            "contract_digest",
                            "artifact_manifest_digest",
                        ][..],
                    ),
                ] {
                    for field in fields {
                        if let Some(text) = value
                            .get(section)
                            .and_then(|v| v.get(*field))
                            .and_then(toml::Value::as_str)
                        {
                            report
                                .historical_fields
                                .insert(format!("{name}:{section}.{field}"), text.into());
                        }
                    }
                }
                if name == "authorization.toml" {
                    report.original_state = value
                        .get("revision")
                        .and_then(|v| v.get("state"))
                        .and_then(toml::Value::as_str)
                        .map(str::to_owned);
                } else {
                    report.original_state = Some("resolved-record-present".into());
                }
            }
        }
        report.unsupported.push(
            "Historical evidence and artifact closure is not inferred from the supplied inventory"
                .into(),
        );
    } else {
        let dialect = if payload.source_dialect == "rc2" {
            Dialect::Rc2
        } else {
            Dialect::Rc3
        };
        let doc = parse_document(bytes, dialect, ParseLimits::default())
            .map_err(|e| err("legacy.syntax", format!("{e:?}")))?;
        report.original_schema = dialect.schema().into();
        report.original_identity = doc
            .metadata
            .get("id")
            .and_then(super::MetadataValue::as_str)
            .unwrap_or("")
            .into();
        report.original_state = doc
            .metadata
            .get("state")
            .and_then(super::MetadataValue::as_str)
            .map(str::to_owned);
    }
    Ok(Preserved { payload, report })
}
/// Deterministic export retains original blobs verbatim; it never reserializes signed files.
pub fn export(value: &Preserved, limits: Limits) -> Result<Vec<u8>, Error> {
    bounds(&value.payload.files, limits)?;
    struct Bounded {
        bytes: Vec<u8>,
        remaining: usize,
    }
    impl std::io::Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.remaining {
                return Err(std::io::Error::other("Export wire limit"));
            }
            self.remaining -= bytes.len();
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Bounded {
        bytes: Vec::new(),
        remaining: limits.wire_bytes,
    };
    serde_json::to_writer(&mut writer, &value.payload)
        .map_err(|_| err("resource-limit", "Export wire limit"))?;
    Ok(writer.bytes)
}
pub fn import(bytes: &[u8], limits: Limits) -> Result<Preserved, Error> {
    if limits.wire_bytes == 0 || bytes.len() > limits.wire_bytes {
        return Err(err("resource-limit", "Import wire limit"));
    }
    if limits.files == 0 || limits.file_bytes == 0 || limits.total_bytes == 0 {
        return Err(err("resource-limit", "Zero preservation limit"));
    }
    codec::preflight(bytes, limits).map_err(|e| {
        let message = e.to_string();
        err(
            if message.starts_with("resource-limit:") {
                "resource-limit"
            } else {
                "legacy.syntax"
            },
            message,
        )
    })?;
    let payload = serde_json::from_slice(bytes).map_err(|e| err("legacy.syntax", e.to_string()))?;
    check(payload, limits)
}
#[derive(Debug, Serialize)]
pub struct VersionPair {
    pub path: String,
    pub predecessor_digest: String,
    pub successor_digest: String,
}
#[derive(Debug, Serialize)]
pub struct Lineage {
    pub predecessor_identity: String,
    pub successor_identity: String,
    pub predecessor_entry_digest: String,
    pub successor_entry_digest: String,
    pub versions: Vec<VersionPair>,
    pub authority_transferred: bool,
}
/// Check only named same-path replacements. Both original bundles remain recoverable.
/// This link is provenance, never authorization, acceptance or a correction request.
pub fn check_successor(
    old: &Preserved,
    new: &Preserved,
    paths: &[&str],
    limits: Limits,
) -> Result<Lineage, Error> {
    bounds(old.files(), limits)?;
    bounds(new.files(), limits)?;
    if paths.is_empty() || paths.len() > limits.files {
        return Err(err("resource-limit", "Invalid successor path count"));
    }
    if old.report.original_identity.is_empty()
        || new.report.original_identity.is_empty()
        || old.report.original_identity == new.report.original_identity
    {
        return Err(err(
            "legacy.lineage",
            "A successor requires distinct source identities",
        ));
    }
    let mut seen = BTreeSet::new();
    let mut versions = Vec::new();
    for path in paths {
        if !safe_path(path) || !seen.insert(*path) {
            return Err(err("legacy.lineage", "Unsafe or duplicate successor path"));
        }
        let before = old
            .files()
            .get(*path)
            .ok_or_else(|| err("legacy.missing", *path))?;
        let after = new
            .files()
            .get(*path)
            .ok_or_else(|| err("legacy.missing", *path))?;
        if before == after {
            return Err(err("legacy.lineage", "Declared replacement is unchanged"));
        }
        versions.push(VersionPair {
            path: (*path).into(),
            predecessor_digest: raw_digest(before),
            successor_digest: raw_digest(after),
        });
    }
    Ok(Lineage {
        predecessor_identity: old.report.original_identity.clone(),
        successor_identity: new.report.original_identity.clone(),
        predecessor_entry_digest: raw_digest(&old.files()[&old.payload.entry]),
        successor_entry_digest: raw_digest(&new.files()[&new.payload.entry]),
        versions,
        authority_transferred: false,
    })
}
