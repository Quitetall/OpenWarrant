// SPDX-License-Identifier: Apache-2.0
//! Explicit JSON comparison targets over the existing semantic walker.
//! The historical show.rs delivery remains byte-pinned; its default path is unchanged.
use std::io::Read;

use camino::Utf8Path;

use crate::{
    contract_history,
    diagnostic::{Diagnostic, Report},
    repo::{RepoError, Repository},
    sdk::wire,
    show,
};

fn read(path: &Utf8Path) -> Result<serde_json::Value, RepoError> {
    const LIMIT: u64 = 4 * 1024 * 1024;
    let io_error = |e| RepoError::Message(format!("cannot read {path}: {e}"));
    if !std::fs::metadata(path).map_err(io_error)?.is_file() {
        return Err(RepoError::Message(format!(
            "{path}: regular JSON file required"
        )));
    }
    let mut file = std::fs::File::open(path)
        .map_err(|e| RepoError::Message(format!("cannot read {path}: {e}")))?;
    if !file.metadata().map_err(io_error)?.is_file() {
        return Err(RepoError::Message(format!(
            "{path}: regular JSON file required"
        )));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() as u64 > LIMIT {
        return Err(RepoError::Message(format!(
            "{path}: comparison input exceeds 4 MiB"
        )));
    }
    let value =
        wire::decode_value(&bytes).map_err(|e| RepoError::Message(format!("{path}: {e}")))?;
    if !value.is_object() {
        return Err(RepoError::Message(format!("{path}: JSON object required")));
    }
    Ok(value)
}

// Bound cumulative derived path lengths before the historical walker allocates
// them. Counting every node in both inputs conservatively covers the union walk,
// including added/removed subtrees, without constructing any path strings.
fn charge_paths(
    value: &serde_json::Value,
    length: usize,
    budget: &mut usize,
) -> Result<(), RepoError> {
    *budget = budget
        .checked_sub(length + 1024)
        .ok_or_else(|| RepoError::Message("comparison path budget exceeds 8 MiB".into()))?;
    match value {
        serde_json::Value::Object(fields) => {
            for (key, child) in fields {
                charge_paths(child, length + key.len() + 1, budget)?;
            }
        }
        serde_json::Value::Array(items) => {
            for child in items {
                // usize indices cannot exceed 20 decimal digits on supported hosts.
                charge_paths(child, length + 22, budget)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn compare(
    repo: &Repository,
    alias: &str,
    from: Option<&Utf8Path>,
    to: &Utf8Path,
) -> Result<Report, RepoError> {
    let baseline = repo.warrant_dir(alias)?.join("generated/WAR.json");
    let from = from.unwrap_or(&baseline);
    let load = |path: &Utf8Path| -> Result<_, RepoError> {
        if let Some(revision) = contract_history::selector(path.as_str())? {
            contract_history::resolve(repo, alias, revision)
        } else {
            Ok((read(path)?, path.to_string()))
        }
    };
    let (before, from_source) = load(from)?;
    let (after, to_source) = load(to)?;
    let mut budget = 8 * 1024 * 1024;
    charge_paths(&before, 0, &mut budget)?;
    charge_paths(&after, 0, &mut budget)?;
    let changes = show::semantic_diff(&before, &after);
    let mut report = Report::default();
    report.note(format!("Baseline: {from_source}. Target: {to_source}."));
    if before["api_version"] == "oh.war/v1" && after["api_version"] == "oh.war/v1" {
        let old = contract_history::parse_ir(&before)?;
        let new = contract_history::parse_ir(&after)?;
        if old.identity.uuid != new.identity.uuid {
            return Err(RepoError::Message(
                "contract comparison requires the same Warrant UUID".into(),
            ));
        }
        let old_digest = old
            .contract_digest()
            .map_err(|e| RepoError::Message(e.to_string()))?;
        let new_digest = new
            .contract_digest()
            .map_err(|e| RepoError::Message(e.to_string()))?;
        report.note(format!("Recomputed contract digest: {old_digest} -> {new_digest}. Covers only declared contract inputs; this does not authenticate signatures."));
        if old_digest != new_digest {
            for change in &changes {
                if [
                    "contract_coverage",
                    "format_basis",
                    "identity",
                    "source_and_composition",
                    "relations",
                ]
                .iter()
                .any(|root| change.path == *root || change.path.starts_with(&format!("{root}.")))
                {
                    report.push(Diagnostic::warn(
                        "diff.contract-input",
                        repo.relative(to),
                        format!(
                            "Digest input {}: {} -> {}",
                            change.path, change.from, change.to
                        ),
                    ));
                }
            }
        }
    }
    if changes.is_empty() {
        report.push(Diagnostic::pass(
            "diff.identical",
            format!("{alias}: no semantic difference"),
        ));
    } else {
        for change in &changes {
            report.push(Diagnostic::warn(
                "diff.changed",
                repo.relative(to),
                format!("{}: {} -> {}", change.path, change.from, change.to),
            ));
        }
        report.note(format!("{} field change(s), comparing {from} to {to}. JSON whitespace and object-key order are ignored. Supplied digests are reported, not authenticated or independently attributed to a cause.", changes.len()));
    }
    Ok(report)
}
