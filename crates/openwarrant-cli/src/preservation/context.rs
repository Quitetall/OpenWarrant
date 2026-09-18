// SPDX-License-Identifier: Apache-2.0
//! Reconcile explicit ADR sources and declared runtime requirements.
use openwarrant_compiler::preservation::{Coverage, Error};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn coverage(
    files: &BTreeMap<String, Vec<u8>>,
    directory: &str,
) -> Result<BTreeMap<String, Coverage>, Error> {
    let adr_name = "ADR refs and accepted bodies where policy permits";
    let runtime_name = "runtime receipt refs";
    if !files.contains_key("__ow_archive__/history.json") {
        return Ok([adr_name, runtime_name]
            .into_iter()
            .map(|name| {
                (
                    name.into(),
                    Coverage::Unavailable {
                        reason: "Declared context history requires --history".into(),
                    },
                )
            })
            .collect());
    }
    let manifest_path = format!("{directory}/manifest.toml");
    let mut adr_paths = BTreeSet::new();
    let mut adr_gaps = BTreeSet::new();
    let mut runtime_gaps = BTreeSet::new();
    let mut milestones = 0;
    for (path, bytes) in files {
        if path != &manifest_path
            && !(path.starts_with("__ow_archive__/history/")
                && path.ends_with(&format!("/{manifest_path}")))
        {
            continue;
        }
        let prefix = path
            .strip_suffix(&manifest_path)
            .ok_or_else(|| Error("invalid context manifest path".into()))?;
        let manifest: openwarrant_core::Manifest =
            toml::from_str(std::str::from_utf8(bytes).map_err(|e| Error(e.to_string()))?)
                .map_err(|e| Error(e.to_string()))?;
        for atom in manifest.atoms {
            if !matches!(atom.role.as_str(), "adr" | "milestones") {
                continue;
            }
            let gaps = if atom.role == "adr" {
                &mut adr_gaps
            } else {
                &mut runtime_gaps
            };
            let Some(reference) = atom.path else {
                gaps.insert(format!("{path}: bound {} requires resolver", atom.role));
                continue;
            };
            let target = format!("{prefix}{}", super::atom_record(directory, &reference)?);
            let Some(bytes) = files.get(&target) else {
                gaps.insert(format!("{path}: missing {} source {target}", atom.role));
                continue;
            };
            let text = std::str::from_utf8(bytes).map_err(|e| Error(e.to_string()))?;
            if atom.role == "adr" {
                match openwarrant_core::adr::AdrRecord::parse(&target, text) {
                    Ok(_) => {
                        adr_paths.insert(path.clone());
                        adr_paths.insert(target);
                    }
                    Err(error) => {
                        adr_gaps.insert(format!("{target}: unreadable ADR: {error}"));
                    }
                }
            } else {
                milestones += 1;
                match openwarrant_core::milestones::parse(text) {
                    Ok(graph) => {
                        for stage in graph.stages {
                            if stage.executor_kind
                                != openwarrant_core::milestones::ExecutorKind::Human
                            {
                                runtime_gaps.insert(format!(
                                    "{target}#{}: {} runtime receipt resolver required",
                                    stage.id, stage.executor_kind
                                ));
                            }
                        }
                    }
                    Err(error) => {
                        runtime_gaps
                            .insert(format!("{target}: unreadable stage declaration: {error}"));
                    }
                }
            }
        }
    }
    let adr = if !adr_gaps.is_empty() {
        Coverage::Unavailable {
            reason: adr_gaps.into_iter().collect::<Vec<_>>().join("; "),
        }
    } else if adr_paths.is_empty() {
        Coverage::Absent {
            reason: "No explicit ADR atom references in selected current and historical manifests"
                .into(),
        }
    } else {
        adr_paths.insert("__ow_archive__/history.json".into());
        Coverage::Retained {
            paths: adr_paths.into_iter().collect(),
        }
    };
    if milestones == 0 {
        runtime_gaps.insert("No retained stage declarations establish runtime scope".into());
    }
    let runtime = if runtime_gaps.is_empty() {
        Coverage::Absent { reason: "Selected current and historical stage declarations contain no non-human runtime stages".into() }
    } else {
        Coverage::Unavailable {
            reason: runtime_gaps.into_iter().collect::<Vec<_>>().join("; "),
        }
    };
    Ok(BTreeMap::from([
        (adr_name.into(), adr),
        (runtime_name.into(), runtime),
    ]))
}
