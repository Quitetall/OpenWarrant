// SPDX-License-Identifier: Apache-2.0
//! Retained review packets are immutable tool outputs, not mutable projections.
//! Publish complete bytes without replacing a retained name. Descriptor-relative
//! operations refuse link traversal; they do not sandbox another same-uid writer.
use camino::Utf8Path;

use crate::repo::RepoError;

fn unavailable(message: String) -> RepoError {
    RepoError::ObservationUnavailable {
        rule: "verify.packet-unavailable",
        message,
    }
}

/// Read the referenced inode, without a metadata/read path-following window.
/// Packet size policy remains a separate unfinished part of bundle budgeting.
pub(crate) fn read(root: &Utf8Path, relative: &Utf8Path) -> Result<Vec<u8>, RepoError> {
    crate::progress_viewer::source::read(root.as_std_path(), relative.as_std_path(), usize::MAX - 1)
        .map_err(|error| {
            unavailable(format!(
                "could not inspect retained packet {relative}: {error}"
            ))
        })
}

#[cfg(unix)]
pub(crate) use unix::Directory;

#[cfg(unix)]
mod unix {
    use super::*;
    use rustix::fs::{AtFlags, Mode, OFlags, fsync, linkat, mkdirat, open, openat, unlinkat};
    use std::{
        fs::File,
        io::{Read, Write},
        os::fd::OwnedFd,
        path::Component,
        sync::atomic::{AtomicU64, Ordering},
    };

    pub(crate) struct Directory(OwnedFd);

    impl Directory {
        pub(crate) fn open(root: &Utf8Path, relative: &Utf8Path) -> Result<Self, RepoError> {
            let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
            let mut fd = open(root.as_std_path(), flags, Mode::empty())
                .map_err(|e| unavailable(format!("could not anchor packet root {root}: {e}")))?;
            for component in relative.as_std_path().components() {
                let Component::Normal(name) = component else {
                    return Err(RepoError::Message(
                        "verify.packet-path: repository-relative directory required".into(),
                    ));
                };
                match mkdirat(&fd, name, Mode::from_raw_mode(0o700)) {
                    Ok(()) => fsync(&fd)
                        .map_err(|e| unavailable(format!("could not sync packet parent: {e}")))?,
                    Err(rustix::io::Errno::EXIST) => {}
                    Err(e) => {
                        return Err(unavailable(format!(
                            "could not create packet directory {relative}: {e}"
                        )));
                    }
                }
                fd = openat(&fd, name, flags, Mode::empty()).map_err(|e| {
                    unavailable(format!("could not anchor packet directory {relative}: {e}"))
                })?;
            }
            Ok(Self(fd))
        }

        fn existing_matches(&self, name: &str, bytes: &[u8]) -> Result<bool, RepoError> {
            let fd = match openat(
                &self.0,
                name,
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
            ) {
                Ok(fd) => fd,
                Err(rustix::io::Errno::NOENT) => return Ok(false),
                Err(e) => return Err(unavailable(format!("could not inspect packet {name}: {e}"))),
            };
            let file = File::from(fd);
            if !file
                .metadata()
                .map_err(|e| unavailable(e.to_string()))?
                .is_file()
            {
                return Err(unavailable(format!(
                    "retained packet {name} is not a regular file"
                )));
            }
            let mut retained = Vec::new();
            file.take(bytes.len() as u64 + 1)
                .read_to_end(&mut retained)
                .map_err(|e| unavailable(format!("could not read packet {name}: {e}")))?;
            if retained != bytes {
                return Err(RepoError::Message(format!(
                    "verify.packet-collision: {name} holds different bytes; nothing was overwritten"
                )));
            }
            Ok(true)
        }

        pub(crate) fn retain(&self, name: &str, bytes: &[u8]) -> Result<(), RepoError> {
            let parts: Vec<_> = std::path::Path::new(name).components().collect();
            if parts.len() != 1 || !matches!(parts[0], Component::Normal(_)) {
                return Err(RepoError::Message(
                    "verify.packet-path: a packet filename is required".into(),
                ));
            }
            if self.existing_matches(name, bytes)? {
                return Ok(());
            }
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| unavailable(e.to_string()))?
                .as_nanos();
            let temp = format!(
                ".{name}.{}.{stamp:x}{:x}.war-tmp",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            );
            let fd = openat(
                &self.0,
                temp.as_str(),
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::from_raw_mode(0o600),
            )
            .map_err(|e| unavailable(format!("could not stage packet {name}: {e}")))?;
            let result = (|| {
                let mut file = File::from(fd);
                file.write_all(bytes)
                    .and_then(|()| file.sync_all())
                    .map_err(|e| {
                        unavailable(format!("could not stage complete packet {name}: {e}"))
                    })?;
                // linkat never replaces a destination. Competing publishers must
                // prove identical bytes; the loser cannot truncate the winner.
                match linkat(&self.0, temp.as_str(), &self.0, name, AtFlags::empty()) {
                    Ok(()) => {}
                    Err(rustix::io::Errno::EXIST) if self.existing_matches(name, bytes)? => {}
                    Err(e) => {
                        return Err(unavailable(format!("could not publish packet {name}: {e}")));
                    }
                }
                fsync(&self.0)
                    .map_err(|e| unavailable(format!("could not sync packet directory: {e}")))
            })();
            let _ = unlinkat(&self.0, temp.as_str(), AtFlags::empty());
            result
        }
    }
}

#[cfg(not(unix))]
pub(crate) struct Directory;
#[cfg(not(unix))]
impl Directory {
    pub(crate) fn open(_: &Utf8Path, _: &Utf8Path) -> Result<Self, RepoError> {
        Err(unavailable(
            "safe retained packet output requires Linux or macOS".into(),
        ))
    }
    pub(crate) fn retain(&self, _: &str, _: &[u8]) -> Result<(), RepoError> {
        Err(unavailable(
            "safe retained packet output requires Linux or macOS".into(),
        ))
    }
}
