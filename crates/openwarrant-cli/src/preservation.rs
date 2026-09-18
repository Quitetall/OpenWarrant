// SPDX-License-Identifier: Apache-2.0
//! Experimental archive import and source-detached re-export. No authority activation.
use std::{collections::BTreeMap, path::Path};

use camino::Utf8PathBuf;
use clap::Subcommand;
use openwarrant_compiler::preservation::{Archive, Error, Limits};

#[derive(Debug, Subcommand)]
pub enum Command {
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
    external.reconnect(limits, |digest, limit| {
        let relative = by_digest
            .get(digest)
            .ok_or_else(|| Error("missing imported record".into()))?;
        read(&directory.join("records").join(relative), limit)
    })?;
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
