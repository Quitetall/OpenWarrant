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
    for (path, digest) in &subject.context_sources {
        let Some(source) = sources.iter().find(|source| source["path"] == *path) else {
            return false;
        };
        let Some(Some(bytes)) = source_bytes(source) else {
            return false;
        };
        if source["kind"] != source_kind(path, &bytes) || !source_matches(source, digest) {
            return false;
        }
        checked.insert(path.as_str());
    }
    for (kind, expected) in [
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
    if source.as_object()?.keys().any(|key| {
        !matches!(
            key.as_str(),
            "path" | "kind" | "sha256" | "present" | "text" | "bytes"
        )
    }) {
        return None;
    }
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

/// Identify the native source type from the captured bytes, not a filename or
/// a packet author's label. This label neither accepts an ADR nor grants trust.
pub(crate) fn source_kind(path: &str, bytes: &[u8]) -> &'static str {
    if std::str::from_utf8(bytes)
        .ok()
        .and_then(|text| openwarrant_core::adr::AdrRecord::parse(path, text).ok())
        .is_some()
    {
        "governing-adr"
    } else {
        "governing-sas"
    }
}

fn unavailable(message: impl Into<String>) -> RepoError {
    RepoError::ObservationUnavailable {
        rule: "verify.context-unavailable",
        message: message.into(),
    }
}

/// No pin means no implicit SAS prerequisite. Explicit current ADRs governing
/// this exact Warrant still apply. Preserve full bytes, never paraphrase rules.
pub(crate) fn capture(
    repo: &Repository,
    one: &Loaded,
) -> Result<BTreeMap<String, Vec<u8>>, RepoError> {
    let mut sources = capture_sas(repo, one)?;
    let directory = repo.relative(&repo.adr_atoms_dir());
    let warrant = one
        .validated
        .as_ref()
        .ok_or_else(|| unavailable("invalid Warrant identity"))?;
    let target = format!("war://{}", warrant.uuid);
    // Retain native alias relations already present in the historical corpus;
    // do not rewrite their bytes into newly invented UUID relations.
    let legacy_target = format!("war://{}", one.alias());
    let mut identities = std::collections::BTreeSet::new();
    for path in adr_paths(repo, &directory)? {
        if repo.source_links.contains_key(&path) {
            return Err(unavailable(format!("ADR source {path} is a link")));
        }
        let bytes = crate::progress_viewer::source::read(
            repo.root.as_std_path(),
            std::path::Path::new(&path),
            usize::MAX - 1,
        )
        .map_err(|error| unavailable(format!("cannot read ADR {path}: {error}")))?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|error| unavailable(format!("cannot parse ADR {path}: {error}")))?;
        let metadata = match openwarrant_core::frontmatter::parse(text) {
            Ok(metadata) => metadata,
            // Plain Markdown decisions can be explicit contract atoms. They
            // are carried there, without inventing a native governs relation.
            Err(openwarrant_core::frontmatter::FrontmatterError::MissingOpenFence) => continue,
            Err(error) => {
                return Err(unavailable(format!(
                    "ADR applicability unavailable at {path}: {error}"
                )));
            }
        };
        let governs = match metadata.get("governs") {
            None => &[][..],
            Some(value) => value.as_list().ok_or_else(|| {
                unavailable(format!("ADR {path} has a non-list governs relation"))
            })?,
        };
        if !governs.contains(&target) && !governs.contains(&legacy_target) {
            continue;
        }
        let decision = openwarrant_core::adr::AdrRecord::parse(&path, text)
            .map_err(|error| unavailable(format!("cannot parse governing ADR {path}: {error}")))?;
        if !decision.status.is_current() {
            continue;
        }
        if !identities.insert(decision.uuid) {
            return Err(unavailable(
                "governing ADR identity has multiple current sources",
            ));
        }
        if sources.insert(path, bytes).is_some() {
            return Err(unavailable(
                "one source cannot supply both the pinned SAS and a governing ADR",
            ));
        }
    }
    Ok(sources)
}

/// Enumerate the same immediate native ADR namespace as the repository reader.
/// An ignored source or a .md directory must not become an absent decision.
/// Open directory components by descriptor so namespace replacement cannot
/// follow a link outside the repository. Actual files use the same safe reader.
#[cfg(unix)]
fn adr_paths(repo: &Repository, directory: &str) -> Result<Vec<String>, RepoError> {
    use rustix::fs::{Dir, Mode, OFlags, open, openat};
    let flags =
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let mut current = open(repo.root.as_std_path(), flags, Mode::empty())
        .map_err(|error| unavailable(format!("cannot open context root: {error}")))?;
    for part in std::path::Path::new(directory).components() {
        if !matches!(part, std::path::Component::Normal(_)) {
            return Err(unavailable(
                "ADR namespace must be repository-relative without traversal",
            ));
        }
        current = match openat(&current, part.as_os_str(), flags, Mode::empty()) {
            Ok(next) => next,
            Err(rustix::io::Errno::NOENT) => return Ok(vec![]),
            Err(error) => {
                return Err(unavailable(format!(
                    "cannot open ADR namespace {directory}: {error}"
                )));
            }
        };
    }
    let entries = Dir::new(current)
        .map_err(|error| unavailable(format!("cannot enumerate ADR namespace: {error}")))?;
    let mut paths = vec![];
    for entry in entries {
        let entry =
            entry.map_err(|error| unavailable(format!("cannot enumerate ADR source: {error}")))?;
        if !entry.file_name().to_bytes().ends_with(b".md") {
            continue;
        }
        let name = entry
            .file_name()
            .to_str()
            .map_err(|error| unavailable(format!("unsupported ADR source name: {error}")))?;
        paths.push(format!("{directory}/{name}"));
    }
    paths.sort();
    Ok(paths)
}

#[cfg(not(unix))]
fn adr_paths(_repo: &Repository, _directory: &str) -> Result<Vec<String>, RepoError> {
    Err(unavailable(
        "safe ADR namespace reads currently support Linux and macOS",
    ))
}

/// A pin requires exact retained SAS bytes, including the full rules.
fn capture_sas(repo: &Repository, one: &Loaded) -> Result<BTreeMap<String, Vec<u8>>, RepoError> {
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
