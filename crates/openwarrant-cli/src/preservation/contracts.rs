// SPDX-License-Identifier: Apache-2.0
//! Reconcile retained local contract revision records and predecessor references.
use openwarrant_compiler::preservation::{Coverage, Error};
use std::collections::{BTreeMap, BTreeSet};

fn digest(value: &str) -> Option<&str> {
    let value = value.strip_prefix("sha256:").unwrap_or(value);
    (value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
    .then_some(value)
}

pub(super) fn coverage(
    files: &BTreeMap<String, Vec<u8>>,
    directory: &str,
) -> Result<Coverage, Error> {
    if !files.contains_key("__ow_archive__/history.json") {
        return Ok(Coverage::Unavailable {
            reason: "Contract revision history requires --history".into(),
        });
    }
    let source = format!("{directory}/authorization.toml");
    let mut revisions = Vec::new();
    let mut known: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    let mut paths = BTreeSet::from(["__ow_archive__/history.json".to_owned()]);
    for (path, bytes) in files {
        if path == &source
            || (path.starts_with("__ow_archive__/history/")
                && path.ends_with(&format!("/{source}")))
        {
            let record: crate::authorize::AuthorizationRecord =
                toml::from_str(std::str::from_utf8(bytes).map_err(|e| Error(e.to_string()))?)
                    .map_err(|e| Error(format!("invalid retained contract record {path}: {e}")))?;
            if record.schema != crate::authorize::AUTHORIZATION_SCHEMA {
                return Err(Error("unsupported retained contract record schema".into()));
            }
            let hash = digest(&record.revision.contract_digest)
                .ok_or_else(|| Error("invalid retained contract digest".into()))?
                .to_owned();
            known
                .entry(hash)
                .or_default()
                .insert(record.revision.revision);
            revisions.push((path, record.revision));
        }
        // Include original sources and amendments alongside observed revision records.
        if path.starts_with(&format!("{directory}/")) || path.starts_with("__ow_archive__/history/")
        {
            paths.insert(path.clone());
        }
    }
    let mut gaps = BTreeSet::new();
    for (path, revision) in revisions {
        match (revision.revision, revision.predecessor_digest.as_deref()) {
            (1, None) => {}
            (number, Some(predecessor)) if number > 1 => {
                let predecessor = digest(predecessor)
                    .ok_or_else(|| Error("invalid predecessor contract digest".into()))?;
                if !known
                    .get(predecessor)
                    .is_some_and(|numbers| numbers.contains(&(number - 1)))
                {
                    gaps.insert(format!(
                        "{path}: revision {} predecessor {predecessor} not retained",
                        number
                    ));
                }
            }
            _ => {
                gaps.insert(format!(
                    "{path}: revision number and predecessor are inconsistent"
                ));
            }
        }
    }
    if !gaps.is_empty() {
        return Ok(Coverage::Unavailable {
            reason: gaps.into_iter().collect::<Vec<_>>().join("; "),
        });
    }
    Ok(Coverage::Retained {
        paths: paths.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(revision: u32, hash: &str, predecessor: Option<&str>) -> Vec<u8> {
        let predecessor = predecessor
            .map(|p| format!("predecessor_digest = \"{p}\"\n"))
            .unwrap_or_default();
        format!("schema = \"oh.war/authorization/v1\"\nwarrant = \"FIXTURE-WAR-0001\"\n[revision]\nrevision = {revision}\nstate = \"draft\"\ncontract_digest = \"{hash}\"\n{predecessor}[revision.coverage]\ncovered = []\n").into_bytes()
    }
    #[test]
    fn retained_revision_chain_requires_exact_predecessor_and_number() {
        let dir = "docs/warrants/FIXTURE-WAR-0001";
        let first = "a".repeat(64);
        let second = "b".repeat(64);
        let mut files = BTreeMap::from([(
            format!("{dir}/authorization.toml"),
            record(2, &second, Some(&first)),
        )]);
        assert!(matches!(
            coverage(&files, dir).unwrap(),
            Coverage::Unavailable { .. }
        ));
        files.insert(
            "__ow_archive__/history.json".into(),
            b"history checked by caller".to_vec(),
        );
        assert!(
            matches!(coverage(&files, dir).unwrap(), Coverage::Unavailable { reason } if reason.contains("predecessor"))
        );
        let prior = format!("__ow_archive__/history/fixture/{dir}/authorization.toml");
        files.insert(prior.clone(), record(1, &first, None));
        assert!(
            matches!(coverage(&files, dir).unwrap(), Coverage::Retained { paths } if paths.contains(&prior))
        );
        files.insert(prior, record(3, &first, Some(&second)));
        assert!(matches!(
            coverage(&files, dir).unwrap(),
            Coverage::Unavailable { .. }
        ));
    }
}
