// SPDX-License-Identifier: Apache-2.0
//! Descriptor-relative source reads. No symlink race may escape the repository,
//! and replacing a source with a FIFO/device cannot block the serving thread.
use std::path::Path;

#[cfg(unix)]
pub(super) fn read(root: &Path, relative: &Path, limit: usize) -> Result<Vec<u8>, String> {
    use rustix::fs::{Mode, OFlags, open, openat};
    use std::{fs::File, io::Read, path::Component};
    let parts: Vec<_> = relative.components().collect();
    if parts.is_empty() || parts.iter().any(|p| !matches!(p, Component::Normal(_))) {
        return Err("Relative source without traversal required".into());
    }
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let mut current =
        open(root, flags | OFlags::DIRECTORY, Mode::empty()).map_err(|e| e.to_string())?;
    for (index, part) in parts.iter().enumerate() {
        let directory = if index + 1 < parts.len() {
            OFlags::DIRECTORY
        } else {
            OFlags::empty()
        };
        current = openat(&current, part.as_os_str(), flags | directory, Mode::empty())
            .map_err(|e| e.to_string())?;
    }
    let file = File::from(current);
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Source must be a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("Source exceeds byte limit".into());
    }
    Ok(bytes)
}
#[cfg(not(unix))]
pub(super) fn read(_root: &Path, _relative: &Path, _limit: usize) -> Result<Vec<u8>, String> {
    Err("Progress viewer safe source reads currently support Linux and macOS".into())
}
