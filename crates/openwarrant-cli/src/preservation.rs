// SPDX-License-Identifier: Apache-2.0
//! Experimental archive import and source-detached re-export. No authority activation.
use std::{collections::BTreeMap, path::Path};
mod artifacts;
mod audit;
mod cases;
mod context;
mod contracts;
mod history;
mod identity;

use camino::Utf8PathBuf;
use clap::Subcommand;
use openwarrant_compiler::preservation::{Archive, Error, Limits};

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Inspect retained source reconstruction without claiming complete preservation.
    Inspect { input: Utf8PathBuf },
    /// Capture current Warrant sources and local records; unresolved categories stay explicit.
    Export {
        alias: String,
        output: Utf8PathBuf,
        /// Include bounded history reachable from pinned local HEAD; refuse unavailable history.
        #[arg(long)]
        history: bool,
        /// Include another local history root, pinned to a commit before capture. Repeatable.
        #[arg(long, requires = "history")]
        history_ref: Vec<String>,
    },
    /// Import experimental canonical archive into a NEW private inert directory.
    Import {
        input: Utf8PathBuf,
        destination: Utf8PathBuf,
        /// Optional content-addressed evidence directory (files named by SHA-256 hex).
        #[arg(long)]
        evidence: Option<Utf8PathBuf>,
    },
    /// Re-read and verify imported record bytes before emitting original canonical archive.
    Reexport {
        directory: Utf8PathBuf,
        output: Utf8PathBuf,
    },
}

fn read(path: &Path, limit: usize) -> Result<Vec<u8>, Error> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|e| Error(e.to_string()))?
            .join(path)
    };
    let relative = absolute
        .strip_prefix("/")
        .map_err(|e| Error(e.to_string()))?;
    crate::progress_viewer::source::read(Path::new("/"), relative, limit).map_err(Error)
}

pub fn run(command: Command) -> Result<(String, serde_json::Value), Error> {
    let limits = Limits::default();
    match command {
        Command::Inspect { input } => {
            let bytes = read(input.as_std_path(), limits.archive_bytes)?;
            let archive = Archive::decode(&bytes, limits)?;
            let mut files = BTreeMap::new();
            for record in &archive.records {
                if let Some(encoded) = &record.base64 {
                    files.insert(
                        record.path.clone(),
                        openwarrant_core::attestation::base64_decode(encoded).map_err(Error)?,
                    );
                }
            }
            if verify_archive_basis(&archive, &files)? != archive.subject {
                return Err(Error(
                    "archive subject differs from reconstructed Warrant".into(),
                ));
            }
            Ok((
                format!(
                    "Archived current sources reconstruct their IR. Coverage remains a source declaration. Unavailable categories: {}.",
                    unavailable_categories(&archive).join(", ")
                ),
                serde_json::json!({"schema":"oh.war/preservation-result/v1-draft.1", "operation":"inspect", "source_reconstructed":true, "coverage":archive.coverage, "unavailable_categories":unavailable_categories(&archive), "authority_activated":false}),
            ))
        }
        Command::Export {
            alias,
            output,
            history,
            history_ref,
        } => {
            let repo = crate::repo::Repository::discover(None).map_err(|e| Error(e.to_string()))?;
            let archive = assemble(&repo, &alias, limits, history, &history_ref)?;
            let bytes = archive.encode(limits)?;
            write_new(output.as_std_path(), &bytes)?;
            Ok((
                format!(
                    "Captured selected local sources and history. Category coverage complete: {}. Unavailable categories: {}. No authority or qualification granted.",
                    unavailable_categories(&archive).is_empty(),
                    unavailable_categories(&archive).join(", ")
                ),
                serde_json::json!({"schema":"oh.war/preservation-result/v1-draft.1", "operation":"export", "archive_digest":archive.digest(limits)?, "output":output.as_str(), "complete":unavailable_categories(&archive).is_empty(), "completeness_scope":"selected-local-current-and-history", "coverage":archive.coverage, "unavailable_categories":unavailable_categories(&archive), "authority_activated":false}),
            ))
        }
        Command::Import {
            input,
            destination,
            evidence,
        } => {
            let bytes = read(input.as_std_path(), limits.archive_bytes)?;
            let archive = Archive::decode(&bytes, limits)?;
            let content = archive.reconnect(limits, |digest, limit| {
                let root = evidence
                    .as_ref()
                    .ok_or_else(|| Error("external evidence directory required".into()))?;
                let hex = digest
                    .strip_prefix("sha256:")
                    .ok_or_else(|| Error("invalid evidence digest".into()))?;
                read(root.join(hex).as_std_path(), limit)
            })?;
            if content.contains_key(BASIS_PATH)
                && verify_archive_basis(&archive, &content)? != archive.subject
            {
                return Err(Error(
                    "archive subject differs from reconstructed Warrant".into(),
                ));
            }
            materialize(destination.as_std_path(), &bytes, &content)?;
            let digest = archive.digest(limits)?;
            Ok((
                format!(
                    "Imported experimental archive {digest}. Inert records only; no authority granted."
                ),
                serde_json::json!({"schema": "oh.war/preservation-result/v1-draft.1", "operation": "import", "archive_digest": digest, "destination": destination.as_str(), "authority_activated": false}),
            ))
        }
        Command::Reexport { directory, output } => {
            let bytes = reexport(directory.as_std_path(), limits)?;
            write_new(output.as_std_path(), &bytes)?;
            Ok(("Re-exported checked record bytes. No human assurance or KF interoperability claimed.".into(),
                serde_json::json!({"schema": "oh.war/preservation-result/v1-draft.1", "operation": "reexport", "output": output.as_str(), "authority_activated": false})))
        }
    }
}

/// Report declared coverage without upgrading it to independently verified completeness.
fn unavailable_categories(archive: &Archive) -> Vec<&str> {
    archive
        .coverage
        .iter()
        .filter_map(|(name, coverage)| {
            matches!(
                coverage,
                openwarrant_compiler::preservation::Coverage::Unavailable { .. }
            )
            .then_some(name.as_str())
        })
        .collect()
}

pub fn reexport(directory: &Path, limits: Limits) -> Result<Vec<u8>, Error> {
    let bytes = read(&directory.join("ARCHIVE.json"), limits.archive_bytes)?;
    let archive = Archive::decode(&bytes, limits)?;
    // Re-read every materialized file, even if the envelope embeds its bytes.
    let mut external = archive.clone();
    for record in &mut external.records {
        record.base64 = None;
    }
    let by_digest: BTreeMap<_, _> = external
        .records
        .iter()
        .map(|r| (r.digest.as_str(), r.path.as_str()))
        .collect();
    // Equal digests can occur at multiple paths: verify each separately first.
    let mut remaining = limits.content_bytes;
    for record in &archive.records {
        let actual = read(&directory.join("records").join(&record.path), remaining)?;
        remaining = remaining
            .checked_sub(actual.len())
            .ok_or_else(|| Error("content exceeds limit".into()))?;
        if format!("sha256:{}", openwarrant_compiler::sha256_hex(&actual)) != record.digest {
            return Err(Error(format!("imported record changed: {}", record.path)));
        }
    }
    let restored = external.reconnect(limits, |digest, limit| {
        let relative = by_digest
            .get(digest)
            .ok_or_else(|| Error("missing imported record".into()))?;
        read(&directory.join("records").join(relative), limit)
    })?;
    if restored.contains_key(BASIS_PATH)
        && verify_archive_basis(&archive, &restored)? != archive.subject
    {
        return Err(Error(
            "archive subject differs from reconstructed Warrant".into(),
        ));
    }
    archive.encode(limits)
}

#[cfg(unix)]
mod unix {
    use super::*;
    use rustix::fs::{Mode, OFlags, mkdirat, open, openat};
    use std::{fs::File, io::Write, os::fd::OwnedFd, path::Component};

    fn directory(path: &Path) -> Result<OwnedFd, Error> {
        let absolute = if path.is_absolute() {
            path.to_owned()
        } else {
            std::env::current_dir()
                .map_err(|e| Error(e.to_string()))?
                .join(path)
        };
        let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let mut fd = open("/", flags, Mode::empty()).map_err(|e| Error(e.to_string()))?;
        for part in absolute.components() {
            match part {
                Component::RootDir => {}
                Component::Normal(name) => {
                    fd =
                        openat(&fd, name, flags, Mode::empty()).map_err(|e| Error(e.to_string()))?
                }
                _ => return Err(Error("destination traversal refused".into())),
            }
        }
        Ok(fd)
    }

    fn write_at(root: &OwnedFd, path: &str, bytes: &[u8]) -> Result<(), Error> {
        let mut parent = rustix::io::dup(root).map_err(|e| Error(e.to_string()))?;
        let parts: Vec<_> = path.split('/').collect();
        for part in &parts[..parts.len() - 1] {
            match mkdirat(&parent, *part, Mode::from_raw_mode(0o700)) {
                Ok(()) | Err(rustix::io::Errno::EXIST) => {}
                Err(e) => return Err(Error(e.to_string())),
            }
            parent = openat(
                &parent,
                *part,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|e| Error(e.to_string()))?;
        }
        let fd = openat(
            &parent,
            *parts
                .last()
                .ok_or_else(|| Error("missing filename".into()))?,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o600),
        )
        .map_err(|e| Error(e.to_string()))?;
        let mut file = File::from(fd);
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| Error(e.to_string()))
    }

    pub fn write_new(path: &Path, bytes: &[u8]) -> Result<(), Error> {
        let parent = directory(
            path.parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )?;
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| Error("output filename required".into()))?;
        write_at(&parent, name, bytes)
    }

    pub fn materialize(
        destination: &Path,
        bytes: &[u8],
        content: &BTreeMap<String, Vec<u8>>,
    ) -> Result<(), Error> {
        let parent = directory(
            destination
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )?;
        let name = destination
            .file_name()
            .ok_or_else(|| Error("destination name required".into()))?;
        mkdirat(&parent, name, Mode::from_raw_mode(0o700))
            .map_err(|e| Error(format!("new destination required: {e}")))?;
        let root = openat(
            &parent,
            name,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|e| Error(e.to_string()))?;
        for (path, value) in content {
            write_at(&root, &format!("records/{path}"), value)?;
        }
        // Completion marker last. Partial failures preserve diagnostic bytes, never claim success.
        write_at(&root, "ARCHIVE.json", bytes)
    }
}
#[cfg(unix)]
use unix::{materialize, write_new};
#[cfg(not(unix))]
fn materialize(_: &Path, _: &[u8], _: &BTreeMap<String, Vec<u8>>) -> Result<(), Error> {
    Err(Error(
        "safe archive import currently requires Linux or macOS".into(),
    ))
}
#[cfg(not(unix))]
fn write_new(_: &Path, _: &[u8]) -> Result<(), Error> {
    Err(Error(
        "safe archive output currently requires Linux or macOS".into(),
    ))
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BasisSnapshot {
    schema: String,
    namespace: String,
    manifest_source: String,
    atoms: Vec<AtomSnapshot>,
    scope_source: Option<String>,
    sas: Option<(String, String)>,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct AtomSnapshot {
    ordinal: u32,
    role: String,
    jurisdiction: String,
    source: String,
    record: String,
    required: bool,
}
const BASIS_PATH: &str = "__ow_archive__/basis.json";
const IR_PATH: &str = "__ow_archive__/WAR.json";

/// Resolve leading parent references without letting the legacy loader traverse
/// an eliminated symlink component. Interior dot segments are refused.
fn atom_record(directory: &str, source: &str) -> Result<String, Error> {
    let mut parts: Vec<&str> = directory.split('/').collect();
    let mut descended = false;
    for part in source.split('/') {
        match part {
            ".." if !descended => {
                if parts.pop().is_none() {
                    return Err(Error("atom reference escapes repository".into()));
                }
            }
            "" | "." | ".." => return Err(Error("noncanonical atom reference".into())),
            _ => {
                descended = true;
                parts.push(part);
            }
        }
    }
    if !descended {
        return Err(Error("atom reference must name a file".into()));
    }
    Ok(parts.join("/"))
}

fn assemble(
    repo: &crate::repo::Repository,
    alias: &str,
    limits: Limits,
    include_history: bool,
    history_refs: &[String],
) -> Result<Archive, Error> {
    use openwarrant_compiler::preservation::{Coverage, Record, SCHEMA};
    use openwarrant_core::{attestation::base64_encode, journal::EXPORT_CONTENTS};
    let dir = repo.warrant_dir(alias).map_err(|e| Error(e.to_string()))?;
    let relative = dir
        .strip_prefix(&repo.root)
        .map_err(|e| Error(e.to_string()))?;
    let mut files = BTreeMap::new();
    let mut remaining = limits.content_bytes;
    let mut nodes = limits.records.saturating_mul(2);
    collect(
        repo.root.as_std_path(),
        relative.as_std_path(),
        &mut files,
        &mut remaining,
        limits.records,
        &mut nodes,
        0,
    )?;
    if include_history {
        let retained = history::capture(
            repo,
            relative.as_str(),
            history_refs,
            remaining,
            limits.records.saturating_sub(files.len()),
        )?;
        files.extend(retained);
    }
    artifacts::capture(repo, relative.as_str(), &mut files, limits)?;
    let schema_paths = identity::capture(repo.root.as_std_path(), &mut files, limits)?;
    // Check every declared source through no-follow reads before legacy loader touches it.
    let manifest_source = format!("{relative}/manifest.toml");
    let manifest_bytes = files
        .get(&manifest_source)
        .ok_or_else(|| Error("missing manifest".into()))?;
    let manifest: openwarrant_core::Manifest =
        toml::from_str(std::str::from_utf8(manifest_bytes).map_err(|e| Error(e.to_string()))?)
            .map_err(|e| Error(e.to_string()))?;
    for atom in &manifest.atoms {
        let source = atom.path.as_ref().ok_or_else(|| {
            Error("unresolved bound atom cannot be archived as complete source".into())
        })?;
        let record = atom_record(relative.as_str(), source)?;
        let bytes = crate::progress_viewer::source::read(
            repo.root.as_std_path(),
            Path::new(&record),
            limits.content_bytes,
        )
        .map_err(Error)?;
        if let Some(captured) = files.get(&record) {
            if captured != &bytes {
                return Err(Error("atom changed during capture".into()));
            }
        } else {
            let used: usize = files.values().map(Vec::len).sum();
            if files.len() >= limits.records
                || bytes.len() > limits.content_bytes.saturating_sub(used)
            {
                return Err(Error("atom capture exceeds archive limits".into()));
            }
            files.insert(record, bytes);
        }
    }

    let loaded = repo.load_warrant(&dir).map_err(|e| Error(e.to_string()))?;
    if !loaded.report.is_ready() {
        return Err(Error("source Warrant is not structurally ready".into()));
    }
    let basis = loaded
        .basis
        .ok_or_else(|| Error("missing compilation basis".into()))?;
    let validated = loaded
        .validated
        .ok_or_else(|| Error("missing validated manifest".into()))?;
    if files.get(&basis.manifest_source) != Some(&basis.manifest_bytes) {
        return Err(Error("manifest changed during capture".into()));
    }
    for atom in &basis.atoms {
        if files.get(&atom_record(relative.as_str(), &atom.source)?) != Some(&atom.bytes) {
            return Err(Error("atom changed during capture".into()));
        }
    }
    if let Some(scope) = &basis.scope
        && files.get(&scope.source) != Some(&scope.bytes)
    {
        return Err(Error("scope changed during capture".into()));
    }
    let snapshot = BasisSnapshot {
        schema: "oh.war/preservation-basis/v1-draft.1".into(),
        namespace: repo.config.project.namespace.as_str().to_owned(),
        manifest_source: basis.manifest_source.clone(),
        atoms: basis
            .atoms
            .iter()
            .map(|a| {
                Ok(AtomSnapshot {
                    ordinal: a.ordinal,
                    role: a.role.clone(),
                    jurisdiction: a.jurisdiction.clone(),
                    source: a.source.clone(),
                    record: atom_record(relative.as_str(), &a.source)?,
                    required: a.required,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?,
        scope_source: basis.scope.as_ref().map(|v| v.source.clone()),
        sas: basis
            .sas
            .as_ref()
            .map(|v| (v.version.clone(), v.sha256.clone())),
    };
    let ir = openwarrant_compiler::lower(&basis, &validated).map_err(|e| Error(e.to_string()))?;
    files.insert(
        BASIS_PATH.into(),
        openwarrant_compiler::to_canonical_bytes(&snapshot).map_err(|e| Error(e.to_string()))?,
    );
    files.insert(
        IR_PATH.into(),
        openwarrant_compiler::to_canonical_bytes(&ir).map_err(|e| Error(e.to_string()))?,
    );
    verify_basis(&files)?;
    let mut coverage: BTreeMap<_, _> = EXPORT_CONTENTS.iter().filter(|s| !s.starts_with("optional ")).map(|s| ((*s).into(), Coverage::Unavailable { reason: "Local snapshot alone does not establish complete historical/provider coverage".into() })).collect();
    for (category, paths) in [
        ("complete identity", vec![IR_PATH.to_owned()]),
        ("source manifest", vec![manifest_source]),
        (
            "exact atom revisions and digests",
            snapshot.atoms.iter().map(|a| a.record.clone()).collect(),
        ),
        ("Compilation Basis", vec![BASIS_PATH.to_owned()]),
        ("canonical IR", vec![IR_PATH.to_owned()]),
        ("evidence manifest", files.keys().cloned().collect()),
    ] {
        let mut paths: Vec<String> = paths;
        paths.sort();
        coverage.insert(category.into(), Coverage::Retained { paths });
    }
    if let Some(paths) = schema_paths {
        coverage.insert(
            "schema and compiler identity".into(),
            Coverage::Retained { paths },
        );
    } else {
        coverage.insert("schema and compiler identity".into(), Coverage::Unavailable {
            reason: "Repository schema pack missing; observed executable identity is retained separately".into()
        });
    }
    coverage.insert(
        "artifacts".into(),
        artifacts::coverage(&files, relative.as_str())?,
    );
    coverage.insert(
        "contract revisions".into(),
        contracts::coverage(&files, relative.as_str())?,
    );
    coverage.insert(
        "actions and relevant audit receipts".into(),
        audit::coverage(&files, relative.as_str())?,
    );
    coverage.extend(cases::coverage(&files, relative.as_str())?);
    coverage.extend(context::coverage(&files, relative.as_str())?);
    let archive = Archive {
        schema: SCHEMA.into(),
        subject: format!("war://{}", basis.manifest.uuid),
        producer: format!(
            "openwarrant-cli/{} experimental-current-snapshot",
            env!("CARGO_PKG_VERSION")
        ),
        records: files
            .into_iter()
            .map(|(path, bytes)| Record {
                path,
                digest: format!("sha256:{}", openwarrant_compiler::sha256_hex(&bytes)),
                base64: Some(base64_encode(&bytes)),
            })
            .collect(),
        coverage,
        extensions: BTreeMap::new(),
    };
    archive.validate(limits)?;
    Ok(archive)
}

fn collect(
    root: &Path,
    relative: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
    remaining: &mut usize,
    count_limit: usize,
    nodes: &mut usize,
    depth: usize,
) -> Result<(), Error> {
    *nodes = nodes
        .checked_sub(1)
        .ok_or_else(|| Error("source entry count exceeds limit".into()))?;
    if depth > 32 {
        return Err(Error("source directory depth exceeds limit".into()));
    }
    let path = root.join(relative);
    let metadata = std::fs::symlink_metadata(&path).map_err(|e| Error(e.to_string()))?;
    if metadata.file_type().is_symlink() {
        return Err(Error("source symlink refused".into()));
    }
    if metadata.is_dir() {
        for entry in std::fs::read_dir(&path).map_err(|e| Error(e.to_string()))? {
            let entry = entry.map_err(|e| Error(e.to_string()))?;
            collect(
                root,
                &relative.join(entry.file_name()),
                files,
                remaining,
                count_limit,
                nodes,
                depth + 1,
            )?;
        }
    } else if metadata.is_file() {
        if files.len() >= count_limit {
            return Err(Error("source record count exceeds limit".into()));
        }
        let bytes =
            crate::progress_viewer::source::read(root, relative, *remaining).map_err(Error)?;
        *remaining = remaining
            .checked_sub(bytes.len())
            .ok_or_else(|| Error("source content exceeds limit".into()))?;
        let key = relative
            .to_str()
            .ok_or_else(|| Error("non-UTF8 source path".into()))?
            .to_owned();
        if files.insert(key, bytes).is_some() {
            return Err(Error("duplicate source record".into()));
        }
    } else {
        return Err(Error("special source file refused".into()));
    }
    Ok(())
}

fn verify_archive_basis(
    archive: &Archive,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<String, Error> {
    let subject = verify_basis(files)?;
    let snapshot: BasisSnapshot =
        serde_json::from_slice(&files[BASIS_PATH]).map_err(|e| Error(e.to_string()))?;
    let directory = snapshot
        .manifest_source
        .strip_suffix("/manifest.toml")
        .ok_or_else(|| Error("invalid manifest source path".into()))?;
    let observations = [
        (
            "artifacts",
            artifacts::coverage(files, directory)?,
            "artifact coverage differs from checked declaration inventory",
        ),
        (
            "contract revisions",
            contracts::coverage(files, directory)?,
            "contract revision coverage differs from retained history",
        ),
        (
            "actions and relevant audit receipts",
            audit::coverage(files, directory)?,
            "action coverage differs from retained journal evidence",
        ),
    ];
    for (name, observed, message) in observations {
        if !matches!(
            archive.coverage.get(name),
            Some(openwarrant_compiler::preservation::Coverage::Unavailable { .. })
        ) && archive.coverage.get(name) != Some(&observed)
        {
            return Err(Error(message.into()));
        }
    }
    let mut source_coverage = cases::coverage(files, directory)?;
    source_coverage.extend(context::coverage(files, directory)?);
    for (name, observed) in source_coverage {
        if !matches!(
            archive.coverage.get(&name),
            Some(openwarrant_compiler::preservation::Coverage::Unavailable { .. })
        ) && archive.coverage.get(&name) != Some(&observed)
        {
            return Err(Error(format!(
                "{name} coverage differs from retained source records"
            )));
        }
    }
    Ok(subject)
}

fn verify_basis(files: &BTreeMap<String, Vec<u8>>) -> Result<String, Error> {
    identity::verify(files)?;
    use openwarrant_compiler::{AtomSource, CompilationBasis, SasPin, ScopeSource};
    let bytes = files
        .get(BASIS_PATH)
        .ok_or_else(|| Error("missing basis descriptor".into()))?;
    let snapshot: BasisSnapshot =
        serde_json::from_slice(bytes).map_err(|e| Error(e.to_string()))?;
    let directory = snapshot
        .manifest_source
        .strip_suffix("/manifest.toml")
        .ok_or_else(|| Error("invalid manifest source path".into()))?;
    artifacts::verify(files, directory)?;
    history::verify(files, directory)?;
    if snapshot.schema != "oh.war/preservation-basis/v1-draft.1" {
        return Err(Error("unsupported basis snapshot".into()));
    }
    let manifest_bytes = files
        .get(&snapshot.manifest_source)
        .ok_or_else(|| Error("missing manifest source".into()))?
        .clone();
    let manifest: openwarrant_core::Manifest =
        toml::from_str(std::str::from_utf8(&manifest_bytes).map_err(|e| Error(e.to_string()))?)
            .map_err(|e| Error(e.to_string()))?;
    let validated = manifest
        .validate(Some(&snapshot.namespace))
        .map_err(|e| Error(e.to_string()))?;
    if manifest.atoms.len() != snapshot.atoms.len() {
        return Err(Error("basis atom membership differs".into()));
    }
    let mut atoms = Vec::new();
    for (entry, atom) in manifest.atoms.iter().zip(snapshot.atoms) {
        if entry.ordinal != atom.ordinal
            || entry.role != atom.role
            || entry.required != atom.required
            || entry.path.as_deref() != Some(&atom.source)
            || atom.record != atom_record(directory, &atom.source)?
        {
            return Err(Error("basis atom differs from manifest".into()));
        }
        let bytes = files
            .get(&atom.record)
            .ok_or_else(|| Error("missing atom bytes".into()))?
            .clone();
        atoms.push(AtomSource {
            ordinal: atom.ordinal,
            role: atom.role,
            jurisdiction: atom.jurisdiction,
            source: atom.source,
            required: atom.required,
            bytes,
        });
    }
    let scope = snapshot
        .scope_source
        .map(|source| {
            files
                .get(&source)
                .cloned()
                .map(|bytes| ScopeSource { source, bytes })
                .ok_or_else(|| Error("missing scope bytes".into()))
        })
        .transpose()?;
    let basis = CompilationBasis {
        manifest,
        manifest_source: snapshot.manifest_source,
        manifest_bytes,
        atoms,
        scope,
        sas: snapshot
            .sas
            .map(|(version, sha256)| SasPin { version, sha256 }),
    };
    let ir = openwarrant_compiler::lower(&basis, &validated).map_err(|e| Error(e.to_string()))?;
    let actual = openwarrant_compiler::to_canonical_bytes(&ir).map_err(|e| Error(e.to_string()))?;
    if files.get(IR_PATH) != Some(&actual) {
        return Err(Error("reconstructed IR differs from archived IR".into()));
    }
    Ok(format!("war://{}", basis.manifest.uuid))
}
