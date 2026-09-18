// SPDX-License-Identifier: Apache-2.0
//! Bind a retained graph's snapshot to a contract reconstructed from exact sources.
use super::*;
use openwarrant_compiler::{AtomSource, CompilationBasis, SasPin, ScopeSource, WarIr};

pub(super) fn reconstruct(
    files: &BTreeMap<String, Vec<u8>>,
    prefix: &str,
    directory: &str,
    manifest: &openwarrant_core::Manifest,
    manifest_bytes: &[u8],
) -> Result<serde_json::Value, String> {
    let ir_path = if prefix.is_empty() {
        IR_PATH.to_owned()
    } else {
        format!("{prefix}{directory}/generated/WAR.json")
    };
    let bytes = files
        .get(&ir_path)
        .ok_or_else(|| format!("missing historical IR: {ir_path}"))?;
    let retained: WarIr = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let manifest_source = format!("{directory}/manifest.toml");
    if retained.source_and_composition.manifest_source != manifest_source {
        return Err("retained IR names another manifest".into());
    }
    let validated = manifest.validate(None).map_err(|e| e.to_string())?;
    let mut atoms = Vec::new();
    for atom in &manifest.atoms {
        let source = atom
            .path
            .as_ref()
            .ok_or("bound historical atom needs resolver")?;
        let record = super::super::atom_record(directory, source).map_err(|e| e.to_string())?;
        let bytes = files
            .get(&format!("{prefix}{record}"))
            .ok_or("historical atom bytes missing")?
            .clone();
        let metadata = retained
            .source_and_composition
            .atoms
            .iter()
            .find(|a| {
                a.ordinal == atom.ordinal
                    && a.role == atom.role
                    && &a.source == source
                    && a.required == atom.required
            })
            .ok_or("retained IR atom membership differs")?;
        atoms.push(AtomSource {
            ordinal: atom.ordinal,
            role: atom.role.clone(),
            jurisdiction: metadata.jurisdiction.clone(),
            source: source.clone(),
            required: atom.required,
            bytes,
        });
    }
    let scope = retained
        .source_and_composition
        .scope
        .as_ref()
        .map(|scope| {
            let bytes = files
                .get(&format!("{prefix}{}", scope.source))
                .ok_or("historical scope bytes missing")?
                .clone();
            Ok::<_, String>(ScopeSource {
                source: scope.source.clone(),
                bytes,
            })
        })
        .transpose()?;
    let sas = match (
        &retained.format_basis.sas_revision,
        &retained.format_basis.sas_digest,
    ) {
        (Some(version), Some(sha256)) => Some(SasPin {
            version: version.clone(),
            sha256: sha256.clone(),
        }),
        (None, None) => None,
        _ => return Err("retained IR has incomplete SAS pin".into()),
    };
    let basis = CompilationBasis {
        manifest: manifest.clone(),
        manifest_source,
        manifest_bytes: manifest_bytes.to_vec(),
        atoms,
        scope,
        sas,
    };
    let reconstructed =
        openwarrant_compiler::lower(&basis, &validated).map_err(|e| e.to_string())?;
    let digest = reconstructed.contract_digest().map_err(|e| e.to_string())?;
    if digest != retained.contract_digest().map_err(|e| e.to_string())?
        || reconstructed.contract_revision != retained.contract_revision
        || reconstructed.api_version != retained.api_version
    {
        return Err("retained IR contract differs from reconstructed snapshot".into());
    }
    Ok(
        serde_json::json!({"revision":reconstructed.contract_revision,"digest":digest,
        "ir_source":ir_path,"ir_source_digest":format!("sha256:{}", openwarrant_compiler::sha256_hex(bytes)),
        "reconstructed":true}),
    )
}
