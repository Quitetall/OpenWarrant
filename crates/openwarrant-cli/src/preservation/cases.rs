// SPDX-License-Identifier: Apache-2.0
//! Preserve assurance and resolution records without reissuing their judgments.
use openwarrant_compiler::preservation::{Coverage, Error};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn coverage(
    files: &BTreeMap<String, Vec<u8>>,
    directory: &str,
) -> Result<BTreeMap<String, Coverage>, Error> {
    let names = ["assurance case", "resolution and standing"];
    if !files.contains_key("__ow_archive__/history.json") {
        return Ok(names
            .into_iter()
            .map(|name| {
                (
                    name.into(),
                    Coverage::Unavailable {
                        reason: "Retained case and resolution history requires --history".into(),
                    },
                )
            })
            .collect());
    }
    let mut paths = BTreeSet::from(["__ow_archive__/history.json".to_owned()]);
    let mut assurance_gaps = BTreeSet::new();
    let mut resolution_gaps = BTreeSet::new();
    let mut resolutions = BTreeSet::new();
    for (path, bytes) in files {
        let source = path
            .strip_prefix("__ow_archive__/history/")
            .and_then(|rest| rest.split_once('/'))
            .map_or(path.as_str(), |(_, source)| source);
        let Some(local) = source.strip_prefix(&format!("{directory}/")) else {
            continue;
        };
        paths.insert(path.clone());
        let text = || std::str::from_utf8(bytes).map_err(|e| e.to_string());
        if local == "manifest.toml" {
            let manifest: openwarrant_core::Manifest =
                toml::from_str(text().map_err(Error)?).map_err(|e| Error(e.to_string()))?;
            for atom in manifest
                .atoms
                .iter()
                .filter(|atom| atom.role == "assurance")
            {
                let Some(reference) = &atom.path else {
                    assurance_gaps
                        .insert(format!("{path}: bound assurance source requires resolver"));
                    continue;
                };
                let target = format!(
                    "{}{}",
                    path.strip_suffix(source).unwrap_or_default(),
                    super::atom_record(directory, reference)?
                );
                if files.contains_key(&target) {
                    paths.insert(target);
                } else {
                    assurance_gaps
                        .insert(format!("{path}: assurance source not retained at {target}"));
                }
            }
        }

        if local == "judgments.toml" {
            let record = text().and_then(|text| {
                toml::from_str::<crate::authorize::JudgmentRecord>(text).map_err(|e| e.to_string())
            });
            match record {
                Ok(record) if record.schema == crate::authorize::JUDGMENTS_SCHEMA => {}
                _ => {
                    assurance_gaps
                        .insert(format!("{path}: unsupported or unreadable judgment record"));
                }
            }
        } else if local.starts_with("verifications/") && local.ends_with(".toml") {
            if text()
                .and_then(|text| {
                    toml::from_str::<openwarrant_core::verification::Verification>(text)
                        .map_err(|e| e.to_string())
                })
                .is_err()
            {
                assurance_gaps.insert(format!("{path}: unreadable verification record"));
            }
        } else if local == "resolution.toml" {
            let record = text().and_then(|text| {
                toml::from_str::<crate::resolution_cmd::ResolutionRecord>(text)
                    .map_err(|e| e.to_string())
            });
            match record {
                Ok(record) if record.schema == "oh.war/resolution/v1" => {
                    resolutions.insert(path.clone());
                }
                _ => {
                    resolution_gaps.insert(format!(
                        "{path}: unsupported or unreadable resolution record"
                    ));
                }
            }
        }
    }
    let assurance = if assurance_gaps.is_empty() {
        Coverage::Retained {
            paths: paths.iter().cloned().collect(),
        }
    } else {
        Coverage::Unavailable {
            reason: assurance_gaps.into_iter().collect::<Vec<_>>().join("; "),
        }
    };
    let resolution = if !resolution_gaps.is_empty() {
        Coverage::Unavailable {
            reason: resolution_gaps.into_iter().collect::<Vec<_>>().join("; "),
        }
    } else if resolutions.is_empty() {
        Coverage::Absent { reason: "No resolution record exists in the selected local current and historical source inventory".into() }
    } else {
        Coverage::Retained {
            paths: paths.into_iter().collect(),
        }
    };
    Ok(BTreeMap::from([
        ("assurance case".into(), assurance),
        ("resolution and standing".into(), resolution),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absence_requires_history_and_unreadable_records_are_not_absent() {
        let dir = "docs/warrants/FIXTURE-WAR-0001";
        let mut files = BTreeMap::new();
        assert!(matches!(
            coverage(&files, dir).unwrap()["resolution and standing"],
            Coverage::Unavailable { .. }
        ));
        files.insert(
            "__ow_archive__/history.json".into(),
            b"history checked by caller".to_vec(),
        );
        assert!(matches!(
            coverage(&files, dir).unwrap()["resolution and standing"],
            Coverage::Absent { .. }
        ));
        files.insert(format!("{dir}/resolution.toml"), b"broken = [".to_vec());
        files.insert(
            format!("{dir}/verifications/OBL-001.toml"),
            b"disposition = 'invented'".to_vec(),
        );
        let result = coverage(&files, dir).unwrap();
        assert!(matches!(
            result["resolution and standing"],
            Coverage::Unavailable { .. }
        ));
        assert!(matches!(
            result["assurance case"],
            Coverage::Unavailable { .. }
        ));
    }
}
