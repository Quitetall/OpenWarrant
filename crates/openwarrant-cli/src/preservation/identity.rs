// SPDX-License-Identifier: Apache-2.0
//! Preserve declared schema bytes and the observed producer executable identity.
use openwarrant_compiler::preservation::{Error, Limits};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read, path::Path};

pub(super) const IDENTITY: &str = "__ow_archive__/producer.json";
const PACK: &str = "schemas/pack.json";
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Pack {
    schema: String,
    id: String,
    version: String,
    files: BTreeMap<String, String>,
    transitive_digest: String,
}
fn parse(bytes: &[u8]) -> Result<Pack, Error> {
    let pack: Pack = serde_json::from_slice(bytes).map_err(|e| Error(e.to_string()))?;
    let canonical =
        openwarrant_compiler::to_canonical_bytes(&pack).map_err(|e| Error(e.to_string()))?;
    if bytes.strip_suffix(b"\n").unwrap_or(bytes) != canonical {
        return Err(Error(
            "schema pack is noncanonical or has duplicate keys".into(),
        ));
    }
    if pack.schema != "oh.war/schema-pack/v1"
        || pack.id != openwarrant_compiler::SCHEMA_PACK_ID
        || pack.version != openwarrant_compiler::SCHEMA_PACK_VERSION
        || pack.files.is_empty()
    {
        return Err(Error("unsupported or empty schema pack".into()));
    }
    for (name, digest) in &pack.files {
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error("invalid schema member or digest".into()));
        }
    }
    let preimage: String = pack
        .files
        .iter()
        .map(|(name, digest)| format!("{name}:{digest}\n"))
        .collect();
    if openwarrant_compiler::sha256_hex(preimage.as_bytes()) != pack.transitive_digest {
        return Err(Error("schema pack transitive digest mismatch".into()));
    }
    Ok(pack)
}
fn path(name: &str) -> String {
    format!("schemas/oh.war/{name}/v1.json")
}
fn insert(
    files: &mut BTreeMap<String, Vec<u8>>,
    name: String,
    bytes: Vec<u8>,
    limits: Limits,
) -> Result<(), Error> {
    let used: usize = files.values().map(Vec::len).sum();
    if files.len() >= limits.records || bytes.len() > limits.content_bytes.saturating_sub(used) {
        return Err(Error("schema capture exceeds archive limits".into()));
    }
    if files.insert(name, bytes).is_some() {
        return Err(Error("duplicate schema source".into()));
    }
    Ok(())
}
fn executable_digest() -> Result<String, Error> {
    let exe = std::env::current_exe().map_err(|e| Error(e.to_string()))?;
    let file = std::fs::File::open(exe).map_err(|e| Error(e.to_string()))?;
    if !file.metadata().map_err(|e| Error(e.to_string()))?.is_file() {
        return Err(Error("producer executable is not a regular file".into()));
    }
    let mut reader = file.take(512 * 1024 * 1024 + 1);
    let mut total = 0;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = reader.read(&mut buffer).map_err(|e| Error(e.to_string()))?;
        if count == 0 {
            break;
        }
        total += count;
        if total > 512 * 1024 * 1024 {
            return Err(Error("producer executable exceeds hash limit".into()));
        }
        hash.update(&buffer[..count]);
    }
    let hex: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!("sha256:{hex}"))
}
pub(super) fn capture(
    root: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
    limits: Limits,
) -> Result<Option<Vec<String>>, Error> {
    let metadata = serde_json::json!({
        "schema":"oh.war/preservation-producer/v1-draft.1",
        "package":"openwarrant-cli", "version":env!("CARGO_PKG_VERSION"),
        "executable_sha256":executable_digest()?,
        "identity_is_observation_not_authentication":true,
        "schema_pack_id":openwarrant_compiler::SCHEMA_PACK_ID,
        "schema_pack_version":openwarrant_compiler::SCHEMA_PACK_VERSION
    });
    insert(
        files,
        IDENTITY.into(),
        openwarrant_compiler::to_canonical_bytes(&metadata).map_err(|e| Error(e.to_string()))?,
        limits,
    )?;
    match std::fs::symlink_metadata(root.join(PACK)) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Error(e.to_string())),
        Ok(_) => (),
    }
    let bytes = crate::progress_viewer::source::read(root, Path::new(PACK), limits.content_bytes)
        .map_err(Error)?;
    let pack = parse(&bytes)?;
    insert(files, PACK.into(), bytes, limits)?;
    let mut paths = vec![IDENTITY.into(), PACK.into()];
    for (name, digest) in pack.files {
        let name = path(&name);
        let bytes =
            crate::progress_viewer::source::read(root, Path::new(&name), limits.content_bytes)
                .map_err(Error)?;
        if openwarrant_compiler::sha256_hex(&bytes) != digest {
            return Err(Error(format!("schema member digest mismatch: {name}")));
        }
        insert(files, name.clone(), bytes, limits)?;
        paths.push(name);
    }
    paths.sort();
    Ok(Some(paths))
}
pub(super) fn verify(files: &BTreeMap<String, Vec<u8>>) -> Result<(), Error> {
    if let Some(bytes) = files.get(IDENTITY) {
        let producer: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|e| Error(e.to_string()))?;
        let digest = producer["executable_sha256"].as_str().unwrap_or_default();
        let hex = digest.strip_prefix("sha256:").unwrap_or_default();
        if producer["schema"] != "oh.war/preservation-producer/v1-draft.1"
            || producer["package"] != "openwarrant-cli"
            || producer["version"].as_str().is_none_or(str::is_empty)
            || producer["schema_pack_id"] != openwarrant_compiler::SCHEMA_PACK_ID
            || producer["schema_pack_version"] != openwarrant_compiler::SCHEMA_PACK_VERSION
            || producer["identity_is_observation_not_authentication"] != true
            || hex.len() != 64
            || !hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error("invalid producer identity record".into()));
        }
    }
    if let Some(bytes) = files.get(PACK) {
        if !files.contains_key(IDENTITY) {
            return Err(Error("schema pack has no producer identity".into()));
        }
        for (name, digest) in parse(bytes)?.files {
            let bytes = files
                .get(&path(&name))
                .ok_or_else(|| Error(format!("missing schema member: {name}")))?;
            if openwarrant_compiler::sha256_hex(bytes) != digest {
                return Err(Error(format!("schema member digest mismatch: {name}")));
            }
        }
    }
    Ok(())
}
