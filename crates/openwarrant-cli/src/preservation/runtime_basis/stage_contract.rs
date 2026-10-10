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
    let (retained, ir_path, source_digest, source_kind) = match files.get(&ir_path) {
        Some(bytes) => (
            serde_json::from_slice::<WarIr>(bytes).map_err(|e| e.to_string())?,
            ir_path,
            openwarrant_compiler::sha256_hex(bytes),
            "retained-ir",
        ),
        None => {
            let (ir, path, digest) = retained_snapshot(files, prefix, directory, manifest_bytes)?;
            (ir, path, digest, "retained-runtime-contract-snapshot")
        }
    };
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
            sha256: sas_digest(sha256)?.to_owned(),
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
        "ir_source":ir_path,"ir_source_digest":format!("sha256:{source_digest}"),
        "source_kind":source_kind,
        "reconstructed":true}),
    )
}

/// A later retained source snapshot can reproduce earlier identical sources.
/// Its actual path is reported; it is never asserted to exist at the earlier commit.
fn retained_snapshot(
    files: &BTreeMap<String, Vec<u8>>,
    prefix: &str,
    directory: &str,
    manifest_bytes: &[u8],
) -> Result<(WarIr, String, String), String> {
    let marker = format!("{directory}/runtime-contracts/");
    let mut found: Option<(WarIr, String, String)> = None;
    for (path, bytes) in files {
        let Some((origin, file)) = path.split_once(&marker) else {
            continue;
        };
        if !origin.is_empty() && !origin.starts_with("__ow_archive__/history/") {
            continue;
        }
        let checked = crate::runtime_capture::contract_snapshot::decode(bytes, file, directory)
            .map_err(|e| format!("{path}: {e}"))?;
        if checked.basis.manifest_bytes != manifest_bytes {
            continue;
        }
        let mut matches = true;
        for atom in &checked.basis.atoms {
            let source =
                super::super::atom_record(directory, &atom.source).map_err(|e| e.to_string())?;
            if files.get(&format!("{prefix}{source}")) != Some(&atom.bytes) {
                matches = false;
                break;
            }
        }
        if let Some(scope) = &checked.basis.scope
            && files.get(&format!("{prefix}{}", scope.source)) != Some(&scope.bytes)
        {
            matches = false;
        }
        if !matches {
            continue;
        }
        if let Some((ir, _, _)) = &found {
            if ir != &checked.ir {
                return Err("multiple retained contract snapshots match historical sources".into());
            }
        } else {
            found = Some((
                checked.ir,
                path.clone(),
                openwarrant_compiler::sha256_hex(bytes),
            ));
        }
    }
    found.ok_or_else(|| {
        "missing historical IR and no exact retained runtime contract snapshot".into()
    })
}

fn sas_digest(value: &str) -> Result<&str, String> {
    let hex = value
        .strip_prefix("sha256:")
        .ok_or("retained SAS digest must name SHA-256")?;
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("retained SAS digest is not a canonical SHA-256 identity".into());
    }
    Ok(hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_sas_pin_roundtrips_through_the_same_lowering_interface() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../conformance/fixtures/inbox/repository");
        let repo = crate::repo::Repository::discover(Some(
            camino::Utf8PathBuf::from_path_buf(root).unwrap(),
        ))
        .unwrap();
        let dir = repo.warrant_dir("IX-WAR-0003").unwrap();
        let one = repo.load_warrant(&dir).unwrap();
        let mut basis = one.basis.unwrap();
        basis.sas = Some(SasPin {
            version: "synthetic-sas-revision".into(),
            sha256: "a".repeat(64),
        });
        let ir = openwarrant_compiler::lower(&basis, &one.validated.unwrap()).unwrap();
        let directory = "docs/warrants/IX-WAR-0003";
        let mut files = BTreeMap::from([(
            IR_PATH.into(),
            openwarrant_compiler::to_canonical_bytes(&ir).unwrap(),
        )]);
        for atom in &basis.atoms {
            files.insert(
                super::super::super::atom_record(directory, &atom.source).unwrap(),
                atom.bytes.clone(),
            );
        }
        let binding = reconstruct(
            &files,
            "",
            directory,
            &basis.manifest,
            &basis.manifest_bytes,
        )
        .unwrap();
        assert_eq!(binding["reconstructed"], true);
        assert_eq!(binding["digest"], ir.contract_digest().unwrap());
        let mut changed = ir;
        changed.format_basis.sas_digest = Some(format!("sha256:{}", "b".repeat(64)));
        // This changes what the retained IR claims; its digest remains computed
        // over those changed sources. No external acceptance is inferred here.
        files.insert(
            IR_PATH.into(),
            openwarrant_compiler::to_canonical_bytes(&changed).unwrap(),
        );
        let changed_binding = reconstruct(
            &files,
            "",
            directory,
            &basis.manifest,
            &basis.manifest_bytes,
        )
        .unwrap();
        assert_ne!(changed_binding["digest"], binding["digest"]);
        for bad in [
            "a".repeat(64),
            format!("sha256:sha256:{}", "a".repeat(64)),
            format!("sha256:{}", "A".repeat(64)),
        ] {
            changed.format_basis.sas_digest = Some(bad);
            files.insert(
                IR_PATH.into(),
                openwarrant_compiler::to_canonical_bytes(&changed).unwrap(),
            );
            assert!(
                reconstruct(
                    &files,
                    "",
                    directory,
                    &basis.manifest,
                    &basis.manifest_bytes
                )
                .is_err()
            );
        }
    }
}
