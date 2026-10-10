// SPDX-License-Identifier: Apache-2.0
//! Exact, bounded job-tree snapshots for native verification. The expected
//! manifest comes from independently protected host policy, never a receipt.
//! No producer is executed and no provider or authority is authenticated here.
use super::protected_input::ProtectedInput;
use openwarrant_core::document::runtime::ProviderFailure;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

const MAX_FILES: usize = 4096;
#[cfg(target_os = "linux")]
const MAX_DIRECTORIES: usize = 8192;
#[cfg(target_os = "linux")]
const MAX_DEPTH: usize = 32;
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobFile {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
    /// Exact source mode including regular-file type bits. Captured descriptors
    /// are non-executable regardless of this retained native metadata.
    pub mode: u32,
}
pub struct CapturedJobFile {
    pub source: JobFile,
    pub input: ProtectedInput,
}
pub struct ProtectedJob {
    files: Vec<CapturedJobFile>,
}
fn rejected(reason: &str) -> ProviderFailure {
    ProviderFailure::Rejected(reason.into())
}
#[cfg(target_os = "linux")]
fn unavailable(error: impl std::fmt::Display) -> ProviderFailure {
    ProviderFailure::Unavailable(error.to_string())
}
fn canonical_relative(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 4096
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|c| !c.is_empty() && c != "." && c != "..")
        && !Path::new(path).is_absolute()
}
impl ProtectedJob {
    /// Require complete exact file coverage and positive finite budgets before
    /// sealing. Empty directories are not native file records. Each data image
    /// must remain held through native verification. Configuration authentication
    /// and task binding belong to the caller; this is not a writer launch fence.
    pub fn acquire(
        root: &Path,
        expected: &[JobFile],
        max_total_bytes: usize,
    ) -> Result<Self, ProviderFailure> {
        if !root.is_absolute()
            || root
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            || expected.is_empty()
            || expected.len() > MAX_FILES
            || max_total_bytes == 0
            || max_total_bytes > MAX_TOTAL_BYTES
        {
            return Err(rejected(
                "absolute job root, complete manifest and finite budget required",
            ));
        }
        let mut paths = BTreeSet::new();
        let mut total = 0u64;
        for file in expected {
            if !canonical_relative(&file.path)
                || !paths.insert(file.path.clone())
                || file.bytes > MAX_FILE_BYTES
                || file.sha256.len() != 64
                || !file
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || file.mode & 0o170000 != 0o100000
                || file.mode & 0o022 != 0
            {
                return Err(rejected(
                    "invalid, duplicate or over-budget native job file",
                ));
            }
            total = total
                .checked_add(file.bytes)
                .ok_or_else(|| rejected("job byte budget overflow"))?;
            if total > max_total_bytes as u64 {
                return Err(rejected("aggregate native job byte budget"));
            }
        }
        #[cfg(target_os = "linux")]
        {
            check_root(root)?;
            if tree_paths(root)? != paths {
                return Err(rejected("native job has missing or unexpected files"));
            }
            let mut files = Vec::with_capacity(expected.len());
            for source in expected {
                let input = ProtectedInput::acquire(
                    &root.join(&source.path),
                    &source.sha256,
                    source.bytes.max(1) as usize,
                )?;
                if input.bytes() as u64 != source.bytes || input.source_mode() != source.mode {
                    return Err(rejected(
                        "native job file size or mode differs from approved manifest",
                    ));
                }
                files.push(CapturedJobFile {
                    source: source.clone(),
                    input,
                });
            }
            // Operator changes remain possible. Never turn changed coverage into
            // a successful snapshot; captured bytes already match exact policy.
            if tree_paths(root)? != paths {
                return Err(rejected("native job coverage changed during capture"));
            }
            files.sort_by(|a, b| a.source.path.cmp(&b.source.path));
            Ok(Self { files })
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(ProviderFailure::Unavailable(
                "sealed native job snapshots unsupported on this platform".into(),
            ))
        }
    }
    pub fn files(&self) -> &[CapturedJobFile] {
        &self.files
    }
}
#[cfg(target_os = "linux")]
fn protected_metadata(path: &Path) -> Result<std::fs::Metadata, ProviderFailure> {
    use rustix::fs::{Access, AtFlags, CWD, accessat};
    use std::os::unix::fs::MetadataExt;
    let meta = std::fs::symlink_metadata(path).map_err(unavailable)?;
    let writable = match accessat(CWD, path, Access::WRITE_OK, AtFlags::EACCESS) {
        Ok(()) => true,
        Err(rustix::io::Errno::ACCESS | rustix::io::Errno::ROFS) => false,
        Err(e) => return Err(unavailable(e)),
    };
    if meta.file_type().is_symlink()
        || meta.uid() == rustix::process::geteuid().as_raw()
        || meta.mode() & 0o022 != 0
        || writable
    {
        return Err(rejected(
            "native job path is linked, writable or self-owned",
        ));
    }
    Ok(meta)
}
#[cfg(target_os = "linux")]
fn check_root(root: &Path) -> Result<(), ProviderFailure> {
    crate::authority_cmd::store::unprivileged_reader()
        .map_err(|_| unavailable("execution privileges cannot establish protected native jobs"))?;
    for path in root.ancestors() {
        if !protected_metadata(path)?.is_dir() {
            return Err(rejected("native job root ancestry is not a directory"));
        }
    }
    Ok(())
}
#[cfg(target_os = "linux")]
fn tree_paths(root: &Path) -> Result<BTreeSet<String>, ProviderFailure> {
    let mut pending = vec![(root.to_owned(), 0usize)];
    let mut dirs = 0usize;
    let mut paths = BTreeSet::new();
    while let Some((dir, depth)) = pending.pop() {
        dirs += 1;
        if dirs > MAX_DIRECTORIES || depth > MAX_DEPTH || !protected_metadata(&dir)?.is_dir() {
            return Err(rejected("native job directory type or traversal budget"));
        }
        // Bound queued entries as well as processed entries; an enormous flat
        // directory cannot allocate an unbounded pending list before refusal.
        for entry in std::fs::read_dir(&dir).map_err(unavailable)? {
            let path = entry.map_err(unavailable)?.path();
            let meta = protected_metadata(&path)?;
            if meta.is_dir() {
                if dirs + pending.len() >= MAX_DIRECTORIES {
                    return Err(rejected("native job directory budget"));
                }
                pending.push((path, depth + 1));
            } else if meta.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .map_err(unavailable)?
                    .to_str()
                    .ok_or_else(|| rejected("native job path is not UTF-8"))?
                    .to_owned();
                if !canonical_relative(&relative) || paths.len() >= MAX_FILES {
                    return Err(rejected("native job path or file-count budget"));
                }
                paths.insert(relative);
            } else {
                return Err(rejected(
                    "native job entry is not a regular file or directory",
                ));
            }
        }
    }
    Ok(paths)
}
