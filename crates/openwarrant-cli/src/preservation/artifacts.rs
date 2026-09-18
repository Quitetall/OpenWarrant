// SPDX-License-Identifier: Apache-2.0
//! Retain exact locally available bytes named by current and captured historical declarations.
use openwarrant_compiler::preservation::{Error, Limits};
use openwarrant_core::deliverable::{Deliverable, DeliverableKind};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub(super) const INDEX: &str = "__ow_archive__/artifacts.json";
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Declarations {
    schema: String,
    #[serde(default)]
    deliverable: Vec<Deliverable>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Claim {
    declaration: String,
    id: String,
    target: String,
    digest: Option<String>,
    record: Option<String>,
    unavailable: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    schema: String,
    claims: Vec<Claim>,
}
fn local_path(value: &str) -> bool {
    !value.is_empty()
        && value.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        })
}
fn digest_hex(value: &str) -> Option<&str> {
    value.strip_prefix("sha256:").filter(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn add(
    files: &mut BTreeMap<String, Vec<u8>>,
    path: String,
    bytes: Vec<u8>,
    limits: Limits,
) -> Result<(), Error> {
    if let Some(existing) = files.get(&path) {
        if *existing == bytes {
            return Ok(());
        }
        return Err(Error("artifact content address collision".into()));
    }
    let used: usize = files.values().map(Vec::len).sum();
    if files.len() >= limits.records || bytes.len() > limits.content_bytes.saturating_sub(used) {
        return Err(Error("artifact capture exceeds archive limits".into()));
    }
    files.insert(path, bytes);
    Ok(())
}
pub(super) fn capture(
    root: &Path,
    relative: &str,
    files: &mut BTreeMap<String, Vec<u8>>,
    limits: Limits,
) -> Result<(), Error> {
    let current = format!("{relative}/deliverables.toml");
    let declarations: Vec<_> = files
        .iter()
        .filter(|(path, _)| {
            **path == current
                || (path.starts_with("__ow_archive__/history/")
                    && path.ends_with(&format!("/{current}")))
        })
        .map(|(path, bytes)| (path.clone(), bytes.clone()))
        .collect();
    let mut claims = Vec::new();
    for (declaration, bytes) in declarations {
        let source: Declarations =
            toml::from_str(std::str::from_utf8(&bytes).map_err(|e| Error(e.to_string()))?)
                .map_err(|e| Error(format!("artifact declarations: {e}")))?;
        if source.schema != "oh.war/deliverables/v1" {
            return Err(Error("unsupported artifact declaration schema".into()));
        }
        let mut ids = BTreeSet::new();
        for item in source.deliverable {
            if item.id.is_empty() || !ids.insert(item.id.clone()) {
                return Err(Error("duplicate or empty artifact id".into()));
            }
            let digest = item.provenance.as_ref().map(|p| p.content_digest.clone());
            let mut claim = Claim {
                declaration: declaration.clone(),
                id: item.id,
                target: item.target_ref,
                digest,
                record: None,
                unavailable: None,
            };
            let Some(hex) = claim.digest.as_deref().and_then(digest_hex) else {
                claim.unavailable = Some("No exact SHA-256 content identity declared".into());
                claims.push(claim);
                continue;
            };
            if item.kind != DeliverableKind::File || !local_path(&claim.target) {
                claim.unavailable =
                    Some("Declared artifact requires a non-local-file resolver".into());
                claims.push(claim);
                continue;
            }
            let record = format!("__ow_archive__/artifacts/{hex}");
            if !files.contains_key(&record) {
                match std::fs::symlink_metadata(root.join(&claim.target)) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        claim.unavailable =
                            Some("Declared artifact version is not available locally".into());
                        claims.push(claim);
                        continue;
                    }
                    Err(e) => return Err(Error(e.to_string())),
                    Ok(_) => (),
                }
                let bytes = crate::progress_viewer::source::read(
                    root,
                    Path::new(&claim.target),
                    limits.content_bytes,
                )
                .map_err(Error)?;
                if openwarrant_compiler::sha256_hex(&bytes) != hex {
                    claim.unavailable = Some(
                        "Current file differs from declared artifact version; original required"
                            .into(),
                    );
                    claims.push(claim);
                    continue;
                }
                add(files, record.clone(), bytes, limits)?;
            }
            claim.record = Some(record);
            claims.push(claim);
        }
    }
    let index = Index {
        schema: "oh.war/preservation-artifacts/v1-draft.1".into(),
        claims,
    };
    add(
        files,
        INDEX.into(),
        openwarrant_compiler::to_canonical_bytes(&index).map_err(|e| Error(e.to_string()))?,
        limits,
    )?;
    verify(files, relative)
}
pub(super) fn verify(files: &BTreeMap<String, Vec<u8>>, relative: &str) -> Result<(), Error> {
    let Some(bytes) = files.get(INDEX) else {
        return Ok(());
    };
    let index: Index = serde_json::from_slice(bytes).map_err(|e| Error(e.to_string()))?;
    if index.schema != "oh.war/preservation-artifacts/v1-draft.1" {
        return Err(Error("unsupported artifact inventory".into()));
    }
    let current = format!("{relative}/deliverables.toml");
    let mut expected = BTreeMap::new();
    for (path, bytes) in files.iter().filter(|(path, _)| {
        **path == current
            || (path.starts_with("__ow_archive__/history/")
                && path.ends_with(&format!("/{current}")))
    }) {
        let source: Declarations =
            toml::from_str(std::str::from_utf8(bytes).map_err(|e| Error(e.to_string()))?)
                .map_err(|e| Error(e.to_string()))?;
        if source.schema != "oh.war/deliverables/v1" {
            return Err(Error("unsupported artifact declaration schema".into()));
        }
        for item in source.deliverable {
            if item.id.is_empty()
                || expected
                    .insert(
                        (path.clone(), item.id),
                        (item.target_ref, item.provenance.map(|p| p.content_digest)),
                    )
                    .is_some()
            {
                return Err(Error("duplicate or empty artifact id".into()));
            }
        }
    }
    for claim in index.claims {
        if expected.remove(&(claim.declaration, claim.id))
            != Some((claim.target, claim.digest.clone()))
        {
            return Err(Error(
                "artifact inventory differs from source declaration".into(),
            ));
        }
        if let Some(record) = claim.record {
            if claim.unavailable.is_some() {
                return Err(Error("artifact cannot be retained and unavailable".into()));
            }
            let hex = claim
                .digest
                .as_deref()
                .and_then(digest_hex)
                .ok_or_else(|| Error("artifact has invalid content identity".into()))?;
            if record != format!("__ow_archive__/artifacts/{hex}") {
                return Err(Error(
                    "artifact address differs from declared digest".into(),
                ));
            }
            let bytes = files
                .get(&record)
                .ok_or_else(|| Error("missing retained artifact bytes".into()))?;
            if openwarrant_compiler::sha256_hex(bytes) != hex {
                return Err(Error("retained artifact digest mismatch".into()));
            }
        } else if claim.unavailable.as_deref().is_none_or(str::is_empty) {
            return Err(Error("unretained artifact lacks reason".into()));
        }
    }
    if !expected.is_empty() {
        return Err(Error("artifact inventory omits declared records".into()));
    }
    Ok(())
}
