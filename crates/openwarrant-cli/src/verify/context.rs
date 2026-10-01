// SPDX-License-Identifier: Apache-2.0
//! Exact governing sources selected by the captured contract, never by a newer
//! document's filename or an agent's summary. This module grants no authority.
use std::collections::BTreeMap;

use crate::repo::{Loaded, RepoError, Repository};

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
