// SPDX-License-Identifier: Apache-2.0
//! Native file bytes and mode from one descriptor; no link or special-file reads.
use super::*;

pub(super) fn safe(path: &str) -> Result<(), Error> {
    if path.is_empty()
        || path.len() > 4096
        || path.contains(['\\', '\0'])
        || path
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(Error(
            "native input path must be repository-relative without traversal".into(),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn open_source(
    root: &Path,
    relative: &str,
    directory: bool,
) -> Result<std::os::fd::OwnedFd, Error> {
    use rustix::fs::{Mode, OFlags, open, openat};
    safe(relative)?;
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let mut fd =
        open(root, flags | OFlags::DIRECTORY, Mode::empty()).map_err(|e| Error(e.to_string()))?;
    let parts: Vec<_> = relative.split('/').collect();
    for (index, part) in parts.iter().enumerate() {
        let kind = if directory || index + 1 < parts.len() {
            OFlags::DIRECTORY
        } else {
            OFlags::empty()
        };
        fd = openat(&fd, *part, flags | kind, Mode::empty())
            .map_err(|e| Error(format!("native input {relative}: {e}")))?;
    }
    Ok(fd)
}

#[cfg(unix)]
pub(super) fn read(
    root: &Path,
    relative: &str,
    remaining: &mut usize,
) -> Result<(Vec<u8>, u32), Error> {
    use std::{fs::File, io::Read, os::unix::fs::PermissionsExt};
    let file = File::from(open_source(root, relative, false)?);
    let before = file.metadata().map_err(|e| Error(e.to_string()))?;
    if !before.is_file() {
        return Err(Error("native input must be a regular file".into()));
    }
    let bound = remaining
        .checked_add(1)
        .ok_or_else(|| Error("native input budget overflow".into()))?;
    let mut bytes = Vec::new();
    (&file)
        .take(bound as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| Error(e.to_string()))?;
    *remaining = remaining
        .checked_sub(bytes.len())
        .ok_or_else(|| Error("native input content exceeds limit".into()))?;
    let after = file.metadata().map_err(|e| Error(e.to_string()))?;
    if before.len() != bytes.len() as u64
        || before.len() != after.len()
        || before.modified().map_err(|e| Error(e.to_string()))?
            != after.modified().map_err(|e| Error(e.to_string()))?
        || before.permissions().mode() != after.permissions().mode()
    {
        return Err(Error("native input changed during read".into()));
    }
    Ok((bytes, after.permissions().mode()))
}

#[cfg(unix)]
pub(super) fn paths(root: &Path, relative: &str, limit: usize) -> Result<BTreeSet<String>, Error> {
    use rustix::fs::{Dir, Mode, OFlags, openat};
    use std::{fs::File, os::fd::OwnedFd};
    fn walk(
        fd: OwnedFd,
        prefix: &str,
        paths: &mut BTreeSet<String>,
        nodes: &mut usize,
        depth: usize,
    ) -> Result<(), Error> {
        if depth > 32 {
            return Err(Error("native job directory depth exceeds limit".into()));
        }
        let entries = Dir::read_from(&fd).map_err(|e| Error(e.to_string()))?;
        for entry in entries {
            let entry = entry.map_err(|e| Error(e.to_string()))?;
            let name = entry
                .file_name()
                .to_str()
                .map_err(|e| Error(e.to_string()))?;
            if matches!(name, "." | "..") {
                continue;
            }
            *nodes = nodes
                .checked_sub(1)
                .ok_or_else(|| Error("native job entry count exceeds limit".into()))?;
            let path = if prefix.is_empty() {
                name.to_owned()
            } else {
                format!("{prefix}/{name}")
            };
            safe(&path)?;
            let child = openat(
                &fd,
                name,
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|e| Error(e.to_string()))?;
            let file = File::from(child);
            let metadata = file.metadata().map_err(|e| Error(e.to_string()))?;
            if metadata.is_dir() {
                walk(file.into(), &path, paths, nodes, depth + 1)?;
            } else if metadata.is_file() {
                paths.insert(path);
            } else {
                return Err(Error("native job contains a special file".into()));
            }
        }
        Ok(())
    }
    let mut paths = BTreeSet::new();
    let mut nodes = limit.saturating_mul(33);
    walk(
        open_source(root, relative, true)?,
        "",
        &mut paths,
        &mut nodes,
        0,
    )?;
    if paths.len() > limit {
        return Err(Error("native job file count exceeds limit".into()));
    }
    Ok(paths)
}

#[cfg(not(unix))]
pub(super) fn read(
    _root: &Path,
    _relative: &str,
    _remaining: &mut usize,
) -> Result<(Vec<u8>, u32), Error> {
    Err(Error(
        "safe native input retention supports Linux and macOS".into(),
    ))
}
#[cfg(not(unix))]
pub(super) fn paths(
    _root: &Path,
    _relative: &str,
    _limit: usize,
) -> Result<BTreeSet<String>, Error> {
    Err(Error(
        "safe native input retention supports Linux and macOS".into(),
    ))
}
