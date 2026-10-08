// SPDX-License-Identifier: Apache-2.0
//! Exact governing sources selected by the captured contract, never by a newer
//! document's filename or an agent's summary. This module grants no authority.
use std::collections::BTreeMap;

use crate::repo::{Loaded, RepoError, Repository};

/// A packet identity binds the bytes supplied, not their completeness. Check
/// every exact review source against the subject before accepting a verdict.
/// This reads only the retained packet, never substitutes newer workspace data.
pub(crate) fn packet_sources_match(
    packet: &serde_json::Value,
    subject: &crate::verify::ReviewedSubject,
) -> bool {
    let Some(sources) = packet["required_sources"].as_array() else {
        return false;
    };
    let mut paths = std::collections::BTreeSet::new();
    if sources.iter().any(|source| {
        source["path"]
            .as_str()
            .is_none_or(|path| !paths.insert(path))
    }) {
        return false;
    }
    let mut checked = std::collections::BTreeSet::new();
    for (kind, expected) in [
        ("governing-sas", &subject.context_sources),
        ("fixture", &subject.fixtures),
        ("gate-evidence", &subject.gate_evidence),
    ] {
        for (path, digest) in expected {
            let Some(source) = sources.iter().find(|source| source["path"] == *path) else {
                return false;
            };
            if source["kind"] != kind || !source_matches(source, digest) {
                return false;
            }
            checked.insert(path.as_str());
        }
    }
    let mut gates = std::collections::BTreeSet::new();
    for source in sources.iter().filter(|s| s["kind"] == "gate-definition") {
        let Some(Some(bytes)) = source_bytes(source) else {
            return false;
        };
        let Some(definition) = std::str::from_utf8(&bytes)
            .ok()
            .and_then(|text| openwarrant_core::structured::parse(text).ok())
        else {
            return false;
        };
        let key = format!(
            "{}@{}",
            definition.scalar("gate_id").unwrap_or_default(),
            definition.scalar("version").unwrap_or_default()
        );
        let Some(digest) = subject.gate_definitions.get(&key) else {
            return false;
        };
        if digest == "missing" || !source_matches(source, digest) || !gates.insert(key) {
            return false;
        }
        // Every path was checked above; no duplicate source can acquire a
        // second meaning by appearing later in the array.
        checked.insert(source["path"].as_str().unwrap());
    }
    checked.len() == sources.len()
        && subject
            .gate_definitions
            .iter()
            .all(|(key, digest)| digest == "missing" || gates.contains(key))
}

fn source_matches(source: &serde_json::Value, digest: &str) -> bool {
    if source["sha256"] != digest {
        return false;
    }
    match source_bytes(source) {
        Some(Some(bytes)) => {
            format!("sha256:{}", openwarrant_compiler::sha256_hex(&bytes)) == digest
        }
        Some(None) => digest == "missing",
        None => false,
    }
}

/// Distinguish an explicit missing source from malformed or ambiguous content.
fn source_bytes(source: &serde_json::Value) -> Option<Option<Vec<u8>>> {
    match (
        source["present"].as_bool()?,
        source.get("text"),
        source.get("bytes"),
    ) {
        (false, None, None) => Some(None),
        (true, Some(text), None) => Some(Some(text.as_str()?.as_bytes().to_vec())),
        (true, None, Some(bytes)) => Some(Some(serde_json::from_value(bytes.clone()).ok()?)),
        _ => None,
    }
}

fn unavailable(message: impl Into<String>) -> RepoError {
    RepoError::ObservationUnavailable {
        rule: "verify.context-unavailable",
        message: message.into(),
    }
}

/// No pin means no implicit SAS prerequisite for prototype work. A pin means
/// the exact recorded revision must be recoverable, including its full rules.
pub(crate) fn capture(
    repo: &Repository,
    one: &Loaded,
) -> Result<BTreeMap<String, Vec<u8>>, RepoError> {
    let Some(pin) = one.basis.as_ref().and_then(|basis| basis.sas.as_ref()) else {
        return Ok(BTreeMap::new());
    };
    let revisions = repo.load_sas_revisions()?;
    let matching: Vec<_> = revisions
        .iter()
        .filter(|revision| revision.version == pin.version && revision.sha256 == pin.sha256)
        .collect();
    let [revision] = matching.as_slice() else {
        return Err(unavailable(format!(
            "SAS {} at sha256:{} has no unique retained revision record",
            pin.version, pin.sha256
        )));
    };
    let source = camino::Utf8Path::new(&revision.source);
    if source.as_std_path().components().next().is_none()
        || source
            .as_std_path()
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(unavailable(
            "SAS source must be repository-relative without traversal",
        ));
    }
    let current = crate::progress_viewer::source::read(
        repo.root.as_std_path(),
        source.as_std_path(),
        usize::MAX - 1,
    );
    let bytes = match current {
        Ok(bytes) if openwarrant_compiler::sha256_hex(&bytes) == pin.sha256 => bytes,
        _ if repo.source_inventory.is_some() => candidate_revision_bytes(repo, revision)?,
        _ => crate::sas::historical_revision_bytes(repo, revision).map_err(unavailable)?,
    };
    // A history locator is evidence only after the exact bytes match the pin.
    if openwarrant_compiler::sha256_hex(&bytes) != pin.sha256 {
        return Err(unavailable(
            "retained SAS bytes do not match the captured pin",
        ));
    }
    Ok(BTreeMap::from([(revision.source.clone(), bytes)]))
}

/// Candidate history is its retained ancestry, not whichever branch happens
/// to be checked out now. Read only regular blob objects and verify their
/// content against the recorded SHA-256 before using them as context.
fn candidate_revision_bytes(
    repo: &Repository,
    revision: &openwarrant_core::SasRevision,
) -> Result<Vec<u8>, RepoError> {
    let Some((root, anchor)) = &repo.candidate_history else {
        return Err(unavailable("candidate has no retained history anchor"));
    };
    let log = crate::acceptance::git(
        root,
        &[
            "log",
            "--full-history",
            "--format=%H",
            anchor,
            "--",
            &revision.source,
        ],
    )
    .ok_or_else(|| unavailable("candidate ancestry cannot be read locally"))?;
    for commit in String::from_utf8_lossy(&log).lines() {
        let Some(entry) = crate::acceptance::git(
            root,
            &[
                "ls-tree",
                "-z",
                "--full-tree",
                commit,
                "--",
                &revision.source,
            ],
        ) else {
            continue;
        };
        for entry in entry
            .split(|byte| *byte == 0)
            .filter(|entry| !entry.is_empty())
        {
            let Some(separator) = entry.iter().position(|byte| *byte == b'\t') else {
                continue;
            };
            if &entry[separator + 1..] != revision.source.as_bytes() {
                continue;
            }
            let Ok(metadata) = std::str::from_utf8(&entry[..separator]) else {
                continue;
            };
            let fields: Vec<_> = metadata.split_whitespace().collect();
            let [mode, "blob", object] = fields.as_slice() else {
                continue;
            };
            if !matches!(*mode, "100644" | "100755") {
                continue;
            }
            let Some(bytes) = crate::acceptance::git(root, &["cat-file", "blob", object]) else {
                continue;
            };
            if openwarrant_compiler::sha256_hex(&bytes) == revision.sha256 {
                return Ok(bytes);
            }
        }
    }
    Err(unavailable(format!(
        "candidate {anchor} does not retain SAS {} at sha256:{} in its ancestors",
        revision.version, revision.sha256
    )))
}
