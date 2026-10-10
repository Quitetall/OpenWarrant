// SPDX-License-Identifier: Apache-2.0
//! Experimental exact source retention, never provider authentication or authority.
use super::*;
use openwarrant_compiler::{AtomSource, CompilationBasis, SasPin, ScopeSource, WarIr};

const SCHEMA: &str = "oh.war/runtime-contract-snapshot/v1-draft.1";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Blob {
    digest: String,
    base64: String,
}
impl Blob {
    fn new(bytes: &[u8]) -> Self {
        Self {
            digest: format!("sha256:{}", sha256_hex(bytes)),
            base64: base64_encode(bytes),
        }
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        let bytes = base64_decode(&self.base64)?;
        if base64_encode(&bytes) != self.base64
            || self.digest != format!("sha256:{}", sha256_hex(&bytes))
        {
            return Err("runtime contract snapshot source digest or encoding differs".into());
        }
        Ok(bytes)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    schema: String,
    ir: WarIr,
    manifest: Blob,
    atoms: Vec<Blob>,
    scope: Option<Blob>,
}

pub(crate) struct Checked {
    pub ir: WarIr,
    pub basis: CompilationBasis,
}

fn name(ir: &WarIr) -> Result<String, String> {
    let digest = ir.contract_digest().map_err(|e| e.to_string())?;
    Ok(format!(
        "contract-{}.json",
        digest.strip_prefix("sha256:").unwrap_or(&digest)
    ))
}

pub(super) fn retain(repo: &Repository, recorded: &recorded::Recorded) -> Result<(), Fault> {
    let encode = || -> Result<(String, Vec<u8>), String> {
        let ir = openwarrant_compiler::lower(&recorded.basis, &recorded.validated)
            .map_err(|e| e.to_string())?;
        let digest = ir.contract_digest().map_err(|e| e.to_string())?;
        if digest.strip_prefix("sha256:").unwrap_or(&digest)
            != recorded
                .dispatch
                .contract_digest
                .strip_prefix("sha256:")
                .unwrap_or(&recorded.dispatch.contract_digest)
        {
            return Err("runtime contract snapshot differs from recorded Dispatch".into());
        }
        let mut atoms = Vec::new();
        for source in &ir.source_and_composition.atoms {
            let atom = recorded
                .basis
                .atoms
                .iter()
                .find(|a| a.ordinal == source.ordinal)
                .ok_or("runtime contract snapshot atom missing")?;
            atoms.push(Blob::new(&atom.bytes));
        }
        let snapshot = Snapshot {
            schema: SCHEMA.into(),
            ir,
            manifest: Blob::new(&recorded.basis.manifest_bytes),
            atoms,
            scope: recorded.basis.scope.as_ref().map(|s| Blob::new(&s.bytes)),
        };
        let file = name(&snapshot.ir)?;
        let bytes =
            openwarrant_compiler::to_canonical_bytes(&snapshot).map_err(|e| e.to_string())?;
        if bytes.len() > RECORD_LIMIT {
            return Err("runtime contract snapshot exceeds byte limit".into());
        }
        Ok((file, bytes))
    };
    let (file, bytes) = encode().map_err(|e| fault("runtime.contract-snapshot", e, false))?;
    let directory = recorded.directory.join("runtime-contracts");
    let retained = crate::bundle::store::Directory::open(&repo.root, &directory)
        .map_err(|e| fault("runtime.contract-snapshot-storage", e, true))?;
    retained
        .retain(&file, &bytes)
        .map_err(|e| fault("runtime.contract-snapshot-storage", e, false))
}

/// Reproduce a snapshot through the original lowering interface, checking all bytes.
pub(crate) fn decode(bytes: &[u8], file: &str, directory: &str) -> Result<Checked, String> {
    if bytes.len() > RECORD_LIMIT {
        return Err("runtime contract snapshot exceeds byte limit".into());
    }
    let value = crate::sdk::wire::decode_value(bytes).map_err(|e| e.to_string())?;
    if value["schema"] != SCHEMA {
        return Err("unsupported runtime contract snapshot version".into());
    }
    let snapshot: Snapshot = serde_json::from_value(value).map_err(|e| e.to_string())?;
    // Re-encoding the typed value also rejects ignored or unknown nested IR fields.
    if openwarrant_compiler::to_canonical_bytes(&snapshot).map_err(|e| e.to_string())? != bytes {
        return Err("runtime contract snapshot is noncanonical or contains unknown fields".into());
    }
    if name(&snapshot.ir)? != file
        || snapshot.ir.source_and_composition.manifest_source
            != format!("{directory}/manifest.toml")
    {
        return Err("runtime contract snapshot identity or source differs".into());
    }
    let manifest_bytes = snapshot.manifest.bytes()?;
    let manifest: openwarrant_core::Manifest =
        toml::from_str(std::str::from_utf8(&manifest_bytes).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let validated = manifest.validate(None).map_err(|e| e.to_string())?;
    if snapshot.atoms.len() != snapshot.ir.source_and_composition.atoms.len() {
        return Err("runtime contract snapshot atom membership differs".into());
    }
    let atoms = snapshot
        .ir
        .source_and_composition
        .atoms
        .iter()
        .zip(&snapshot.atoms)
        .map(|(meta, blob)| {
            if !manifest.atoms.iter().any(|a| {
                a.ordinal == meta.ordinal
                    && a.role == meta.role
                    && a.path.as_deref() == Some(meta.source.as_str())
                    && a.required == meta.required
            }) {
                return Err("runtime contract snapshot atom membership differs".into());
            }
            Ok(AtomSource {
                ordinal: meta.ordinal,
                role: meta.role.clone(),
                jurisdiction: meta.jurisdiction.clone(),
                source: meta.source.clone(),
                required: meta.required,
                bytes: blob.bytes()?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let scope = match (&snapshot.ir.source_and_composition.scope, &snapshot.scope) {
        (Some(meta), Some(blob)) => Some(ScopeSource {
            source: meta.source.clone(),
            bytes: blob.bytes()?,
        }),
        (None, None) => None,
        _ => return Err("runtime contract snapshot scope membership differs".into()),
    };
    let sas = match (
        &snapshot.ir.format_basis.sas_revision,
        &snapshot.ir.format_basis.sas_digest,
    ) {
        (Some(version), Some(digest)) => {
            let hex = digest
                .strip_prefix("sha256:")
                .filter(|hex| {
                    hex.len() == 64
                        && hex
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                })
                .ok_or("runtime contract snapshot SAS digest differs")?;
            Some(SasPin {
                version: version.clone(),
                sha256: hex.into(),
            })
        }
        (None, None) => None,
        _ => return Err("runtime contract snapshot SAS pin incomplete".into()),
    };
    let basis = CompilationBasis {
        manifest,
        manifest_source: snapshot.ir.source_and_composition.manifest_source.clone(),
        manifest_bytes,
        atoms,
        scope,
        sas,
    };
    let reconstructed =
        openwarrant_compiler::lower(&basis, &validated).map_err(|e| e.to_string())?;
    if reconstructed != snapshot.ir {
        return Err("runtime contract snapshot IR differs from exact source reconstruction".into());
    }
    Ok(Checked {
        ir: snapshot.ir,
        basis,
    })
}
