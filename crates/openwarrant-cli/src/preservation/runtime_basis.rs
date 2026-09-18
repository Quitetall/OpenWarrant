// SPDX-License-Identifier: Apache-2.0
//! Source-derived provider query basis, not a trust or execution verdict.
use super::*;
use std::collections::BTreeSet;

pub(super) fn run(input: &Path, limits: Limits) -> Result<(String, serde_json::Value), Error> {
    let archive = Archive::decode(&read(input, limits.archive_bytes)?, limits)?;
    let mut files = BTreeMap::new();
    for record in &archive.records {
        if let Some(encoded) = &record.base64 {
            files.insert(
                record.path.clone(),
                openwarrant_core::attestation::base64_decode(encoded).map_err(Error)?,
            );
        }
    }
    let subject = verify_archive_basis(&archive, &files)?;
    if subject != archive.subject {
        return Err(Error(
            "archive subject differs from reconstructed Warrant".into(),
        ));
    }
    let basis: BasisSnapshot =
        serde_json::from_slice(&files[BASIS_PATH]).map_err(|e| Error(e.to_string()))?;
    let directory = basis
        .manifest_source
        .strip_suffix("/manifest.toml")
        .ok_or_else(|| Error("invalid basis manifest path".into()))?;
    let ir: openwarrant_compiler::WarIr =
        serde_json::from_slice(&files[IR_PATH]).map_err(|e| Error(e.to_string()))?;
    let current_digest = ir.contract_digest().map_err(|e| Error(e.to_string()))?;
    let source = format!("{directory}/authorization.toml");
    let mut retained = BTreeSet::new();
    for (path, bytes) in &files {
        if path != &source
            && !(path.starts_with("__ow_archive__/history/")
                && path.ends_with(&format!("/{source}")))
        {
            continue;
        }
        let record: crate::authorize::AuthorizationRecord =
            toml::from_str(std::str::from_utf8(bytes).map_err(|e| Error(e.to_string()))?)
                .map_err(|e| Error(e.to_string()))?;
        if record.schema != crate::authorize::AUTHORIZATION_SCHEMA {
            return Err(Error("unsupported retained contract record schema".into()));
        }
        let digest = record
            .revision
            .contract_digest
            .strip_prefix("sha256:")
            .unwrap_or(&record.revision.contract_digest);
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error("invalid retained contract digest".into()));
        }
        retained.insert((record.revision.revision, digest.to_owned(), path.clone()));
    }
    let retained: Vec<_> = retained.into_iter().map(|(revision, digest, path)| {
        serde_json::json!({"revision":revision,"digest":digest,"source":path})
    }).collect();
    Ok((
        "Reconstructed contract query basis. Provider authentication, runtime coverage and authority are not established.".into(),
        serde_json::json!({
            "schema":"oh.war/runtime-archive-basis/v1-draft.1",
            "archive_digest":archive.digest(limits)?,
            "subject":subject,
            "warrant_id":ir.identity.uuid,
            "current_contract":{"revision":ir.contract_revision,"digest":current_digest},
            "retained_contracts":retained,
            "contract_history_coverage":archive.coverage.get("contract revisions"),
            "authority_activated":false,
            "qualified":false
        }),
    ))
}
