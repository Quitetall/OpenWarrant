// SPDX-License-Identifier: Apache-2.0
//! Exact retained stage graphs for provider queries; never execution coverage.
use super::*;

pub(super) fn inventory(
    files: &BTreeMap<String, Vec<u8>>,
    directory: &str,
) -> Result<serde_json::Value, Error> {
    let manifest_path = format!("{directory}/manifest.toml");
    let mut declarations = BTreeMap::new();
    let mut gaps = BTreeSet::new();
    for (path, bytes) in files {
        if path != &manifest_path
            && !(path.starts_with("__ow_archive__/history/")
                && path.ends_with(&format!("/{manifest_path}")))
        {
            continue;
        }
        let prefix = path.strip_suffix(&manifest_path).unwrap();
        let manifest: openwarrant_core::Manifest =
            toml::from_str(std::str::from_utf8(bytes).map_err(|e| Error(e.to_string()))?)
                .map_err(|e| Error(e.to_string()))?;
        let contract_binding =
            match super::stage_contract::reconstruct(files, prefix, directory, &manifest, bytes) {
                Ok(binding) => Some(binding),
                Err(error) => {
                    gaps.insert(format!("{path}: contract binding unavailable: {error}"));
                    None
                }
            };
        let mut found = false;
        for atom in manifest
            .atoms
            .into_iter()
            .filter(|atom| atom.role == "milestones")
        {
            found = true;
            let Some(reference) = atom.path else {
                gaps.insert(format!("{path}: bound milestones require resolver"));
                continue;
            };
            let target = format!(
                "{prefix}{}",
                super::super::atom_record(directory, &reference)?
            );
            let Some(source) = files.get(&target) else {
                gaps.insert(format!("{path}: missing stage source {target}"));
                continue;
            };
            let graph = match std::str::from_utf8(source)
                .map_err(|e| e.to_string())
                .and_then(|text| {
                    openwarrant_core::milestones::parse(text).map_err(|e| e.to_string())
                }) {
                Ok(graph) => graph,
                Err(error) => {
                    gaps.insert(format!("{target}: unreadable stage declaration: {error}"));
                    continue;
                }
            };
            declarations.insert(
                target.clone(),
                serde_json::json!({
                    "source":target,
                    "source_digest":format!("sha256:{}", openwarrant_compiler::sha256_hex(source)),
                    "manifest_source":path,
                    "graph":graph,
                    "contract_binding":contract_binding
                }),
            );
        }
        if !found {
            gaps.insert(format!("{path}: no stage declaration"));
        }
    }
    Ok(serde_json::json!({
        "declarations":declarations.into_values().collect::<Vec<_>>(),
        "unresolved":gaps.into_iter().collect::<Vec<_>>(),
        "history_retained":files.contains_key("__ow_archive__/history.json"),
        "execution_coverage_established":false
    }))
}
