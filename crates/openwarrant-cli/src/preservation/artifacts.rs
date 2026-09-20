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
    #[serde(default)]
    git_source: Option<GitSource>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GitSource {
    head: String,
    commit: String,
    blob: String,
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
    repo: &crate::repo::Repository,
    relative: &str,
    files: &mut BTreeMap<String, Vec<u8>>,
    limits: Limits,
) -> Result<(), Error> {
    let root = repo.root.as_std_path();
    let history_heads = files
        .get("__ow_archive__/history.json")
        .map(|bytes| {
            let value: serde_json::Value =
                serde_json::from_slice(bytes).map_err(|e| Error(e.to_string()))?;
            let head = value["head"]
                .as_str()
                .ok_or_else(|| Error("missing captured history head".into()))?;
            let mut roots = vec![head.to_owned()];
            if let Some(extra) = value.get("additional_heads") {
                let extra: Vec<String> =
                    serde_json::from_value(extra.clone()).map_err(|e| Error(e.to_string()))?;
                roots.extend(extra);
            }
            Ok::<_, Error>(roots)
        })
        .transpose()?
        .unwrap_or_default();
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
            if claims.len() >= limits.records {
                return Err(Error("artifact claim count exceeds archive limits".into()));
            }
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
                git_source: None,
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
                let available = limits
                    .content_bytes
                    .saturating_sub(files.values().map(Vec::len).sum());
                let local = match std::fs::symlink_metadata(root.join(&claim.target)) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                    Err(e) => return Err(Error(e.to_string())),
                    Ok(metadata) if metadata.is_file() && metadata.len() > available as u64 => None,
                    Ok(_) => Some(
                        crate::progress_viewer::source::read(
                            root,
                            Path::new(&claim.target),
                            available,
                        )
                        .map_err(Error)?,
                    ),
                };
                let bytes = match local
                    .filter(|bytes| openwarrant_compiler::sha256_hex(bytes) == hex)
                {
                    Some(bytes) => Some(bytes),
                    None => {
                        let mut bytes = None;
                        for head in &history_heads {
                            if let Some(recovered) =
                                super::history::artifact(repo, head, &claim.target, hex, available)?
                            {
                                claim.git_source = Some(GitSource {
                                    head: head.clone(),
                                    commit: recovered.commit,
                                    blob: recovered.blob,
                                });
                                bytes = Some(recovered.bytes);
                                break;
                            }
                        }
                        bytes
                    }
                };
                let Some(bytes) = bytes else {
                    claim.unavailable = Some("Local file differs or is missing; declared version not found within captured history and byte limits".into());
                    claims.push(claim);
                    continue;
                };
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
        if let Some(origin) = &claim.git_source {
            let oid = |value: &str| {
                matches!(value.len(), 40 | 64)
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            };
            let history: serde_json::Value = serde_json::from_slice(
                files
                    .get("__ow_archive__/history.json")
                    .ok_or_else(|| Error("artifact Git origin lacks captured history".into()))?,
            )
            .map_err(|e| Error(e.to_string()))?;
            if claim.record.is_none()
                || !oid(&origin.head)
                || !oid(&origin.commit)
                || !oid(&origin.blob)
                || (history["head"] != origin.head
                    && !history["additional_heads"]
                        .as_array()
                        .is_some_and(|roots| roots.iter().any(|root| root == &origin.head)))
            {
                return Err(Error(
                    "artifact Git origin differs from captured history".into(),
                ));
            }
        }
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

/// Coverage of declared artifacts in the selected local source/history boundary.
/// Provider-owned artifacts need their provider's export; no external inventory
/// is inferred from a local declaration set.
pub(super) fn coverage(
    files: &BTreeMap<String, Vec<u8>>,
    relative: &str,
) -> Result<openwarrant_compiler::preservation::Coverage, Error> {
    use openwarrant_compiler::preservation::Coverage;
    verify(files, relative)?;
    let Some(bytes) = files.get(INDEX) else {
        return Ok(Coverage::Unavailable {
            reason: "Artifact inventory missing".into(),
        });
    };
    if !files.contains_key("__ow_archive__/history.json") {
        return Ok(Coverage::Unavailable {
            reason: "Current declared artifacts captured where available; retained declaration history requires --history".into(),
        });
    }
    let index: Index = serde_json::from_slice(bytes).map_err(|e| Error(e.to_string()))?;
    let mut paths = BTreeSet::from([INDEX.to_owned(), "__ow_archive__/history.json".to_owned()]);
    let mut unavailable = Vec::new();
    for claim in index.claims {
        paths.insert(claim.declaration.clone());
        match claim.record {
            Some(record) => {
                paths.insert(record);
            }
            None => unavailable.push(format!(
                "{}#{}: {}",
                claim.declaration,
                claim.id,
                claim.unavailable.unwrap_or_else(|| "unavailable".into())
            )),
        }
    }
    if !unavailable.is_empty() {
        return Ok(Coverage::Unavailable {
            reason: format!(
                "Declared artifact versions unavailable: {}",
                unavailable.join("; ")
            ),
        });
    }
    Ok(Coverage::Retained {
        paths: paths.into_iter().collect(),
    })
}
