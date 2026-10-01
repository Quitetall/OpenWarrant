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
        _ if repo.source_inventory.is_some() => {
            return Err(unavailable(format!(
                "candidate does not retain SAS {} at sha256:{}",
                pin.version, pin.sha256
            )));
        }
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
