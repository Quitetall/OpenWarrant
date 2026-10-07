// SPDX-License-Identifier: Apache-2.0
//! One write path for the records that carry authority (OW-WAR-0121).
//!
//! SAS §86: file-native commands use temporary files, fsync, atomic rename
//! and prestate digest checks, and publish no partial generated parent.
//! §87.2: controlled writes avoid symlink races. Before this module each
//! authority-bearing writer called `fs::write`, which truncates the record in
//! place and then fills it: a crash between the two left a half record where a
//! whole one had been.
//!
//! # The protocol
//!
//! [`write_if`] does, in order:
//!
//! 1. refuses a target that is a symlink (`storage.symlink-target`);
//! 2. creates `.<name>.<pid>.<nonce>.war-tmp` in the target's own directory
//!    with `O_CREAT|O_EXCL`, so it never opens a file somebody else placed
//!    there (a symlink included);
//! 3. writes the bytes and fsyncs the temp file;
//! 4. re-reads the target and refuses, removing the temp file, when it is no
//!    longer the prestate the caller read (`storage.prestate-moved`) or has
//!    become a symlink;
//! 5. renames the temp file over the target (atomic within one directory);
//! 6. fsyncs the directory, so the rename itself survives a power loss.
//!
//! A crash before 5 leaves the old record whole and a temp file behind, which
//! `war check` reports as `storage.stray-temp` ([`stray`]). A crash after 5
//! leaves the new record whole. There is no point at which the target holds
//! part of either. `docs/STORAGE.md` walks each crash point.
//!
//! The window between step 4 and step 5 is not closed: a writer that bypasses
//! this module and lands in those microseconds is overwritten. What the check
//! does close is the window an act spends between READING a record and
//! writing its successor, which for `war sign` includes an ssh-agent dialog.
//!
//! # The fault hook (debug builds only)
//!
//! `OPENWARRANT_FAULT=after-temp` exits the process after step 3, and
//! `OPENWARRANT_FAULT=pause-before-rename:<ms>` sleeps there, so a plant can
//! observe a crash or a race deterministically. `OPENWARRANT_FAULT_FILE=<name>`
//! restricts either to the target whose file name is `<name>`; without it the
//! first atomic write of the process is the one stopped. The hook is compiled
//! out of release builds by `cfg(debug_assertions)`.

use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

use camino::{Utf8Path, Utf8PathBuf};

use crate::diagnostic::Diagnostic;
use crate::repo::RepoError;

/// The suffix every temp file of this module carries, and what [`stray`] looks
/// for. Nothing else in the tool writes a file with it.
pub(crate) const TEMP_SUFFIX: &str = ".war-tmp";

/// What a record was when it was read: absent, or these bytes (by SHA-256).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Prestate {
    Absent,
    Digest(String),
}

impl Prestate {
    /// The prestate a file holding exactly `bytes` has.
    #[must_use]
    pub(crate) fn of(bytes: &[u8]) -> Self {
        Self::Digest(openwarrant_compiler::digest::sha256_hex(bytes))
    }

    fn describe(&self) -> String {
        match self {
            Self::Absent => "absent".to_owned(),
            Self::Digest(d) => format!("sha256:{}", &d[..d.len().min(16)]),
        }
    }
}

/// A write this module refused or could not complete, by rule name.
#[derive(Debug)]
pub(crate) struct Refused {
    /// `storage.symlink-target`, `storage.prestate-moved` or `storage.io`.
    pub rule: &'static str,
    pub path: Utf8PathBuf,
    pub message: String,
}

impl Refused {
    fn io(path: &Utf8Path, what: &str, e: &std::io::Error) -> Self {
        Self {
            rule: "storage.io",
            path: path.to_path_buf(),
            message: format!("{what} {path}: {e}"),
        }
    }

    fn symlink(path: &Utf8Path) -> Self {
        let to = std::fs::read_link(path)
            .map(|t| t.display().to_string())
            .unwrap_or_else(|_| "?".to_owned());
        Self {
            rule: "storage.symlink-target",
            path: path.to_path_buf(),
            message: format!(
                "{path} is a symlink (to {to}); a record is never written through a link \
                 (§87.2). Nothing was written and the link's target is untouched"
            ),
        }
    }

    /// The refusal as an error diagnostic, for a command that reports rather
    /// than propagates.
    #[must_use]
    pub(crate) fn diagnostic(&self) -> Diagnostic {
        Diagnostic::error(self.rule, self.path.to_string(), self.message.clone())
    }
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.rule, self.message)
    }
}

impl std::error::Error for Refused {}

impl From<Refused> for RepoError {
    fn from(r: Refused) -> Self {
        Self::Message(r.to_string())
    }
}

/// Read what `path` is now. Refuses a symlink: a prestate read through a link
/// describes a file the write would not replace.
pub(crate) fn prestate(path: &Utf8Path) -> Result<Prestate, Refused> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Prestate::Absent),
        Err(e) => Err(Refused::io(path, "could not stat", &e)),
        Ok(m) if m.file_type().is_symlink() => Err(Refused::symlink(path)),
        Ok(_) => std::fs::read(path)
            .map(|b| Prestate::of(&b))
            .map_err(|e| Refused::io(path, "could not read", &e)),
    }
}

/// Write `bytes` to `path` the §86 way, refusing if the target changes while
/// the write is in flight. For a caller that read the record earlier and must
/// not overwrite a change made since, [`write_if`] with that earlier prestate.
pub(crate) fn write(path: &Utf8Path, bytes: impl AsRef<[u8]>) -> Result<(), Refused> {
    let before = prestate(path)?;
    write_if(path, bytes, &before)
}

/// Write `bytes` to `path` only if the target is still `expected`.
pub(crate) fn write_if(
    path: &Utf8Path,
    bytes: impl AsRef<[u8]>,
    expected: &Prestate,
) -> Result<(), Refused> {
    let bytes = bytes.as_ref();
    if is_symlink(path) {
        return Err(Refused::symlink(path));
    }
    let dir = match path.parent() {
        Some(p) if !p.as_str().is_empty() => p.to_path_buf(),
        _ => Utf8PathBuf::from("."),
    };
    let name = path.file_name().ok_or_else(|| Refused {
        rule: "storage.io",
        path: path.to_path_buf(),
        message: format!("{path} names no file"),
    })?;
    let temp = dir.join(temp_name(name));

    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| Refused::io(&temp, "could not create", &e))?;
    let written = f.write_all(bytes).and_then(|()| f.sync_all());
    drop(f);
    if let Err(e) = written {
        let _ = std::fs::remove_file(&temp);
        return Err(Refused::io(&temp, "could not write", &e));
    }

    fault(path, &temp);

    // The last look before the rename. A symlink placed meanwhile, or a record
    // changed by another process since the caller read it, is refused and the
    // temp file removed: the other writer's bytes stand.
    let refusal = if is_symlink(path) {
        Some(Refused::symlink(path))
    } else {
        match prestate(path) {
            Ok(now) if now == *expected => None,
            Ok(now) => Some(Refused {
                rule: "storage.prestate-moved",
                path: path.to_path_buf(),
                message: format!(
                    "{path} changed after it was read (read as {}, now {}); another process \
                     wrote it. Nothing was written over it — read it again and redo the act",
                    expected.describe(),
                    now.describe()
                ),
            }),
            Err(r) => Some(r),
        }
    };
    if let Some(r) = refusal {
        let _ = std::fs::remove_file(&temp);
        return Err(r);
    }

    if let Err(e) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(Refused::io(path, "could not rename the temp file over", &e));
    }
    sync_dir(&dir).map_err(|e| Refused::io(&dir, "could not fsync", &e))
}

fn is_symlink(path: &Utf8Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}

fn temp_name(name: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        ".{name}.{}.{nanos:08x}{n:x}{TEMP_SUFFIX}",
        std::process::id()
    )
}

/// The record a temp file of this module was meant to replace, or `None` when
/// the name is not one this module writes.
#[must_use]
pub(crate) fn record_of(temp: &Utf8Path) -> Option<Utf8PathBuf> {
    let file = temp.file_name()?;
    let inner = file.strip_prefix('.')?.strip_suffix(TEMP_SUFFIX)?;
    // `<name>.<pid>.<nonce>`: the name may itself contain dots, so the two
    // generated parts are taken from the right.
    let (rest, nonce) = inner.rsplit_once('.')?;
    let (name, pid) = rest.rsplit_once('.')?;
    if name.is_empty()
        || pid.is_empty()
        || !pid.bytes().all(|b| b.is_ascii_digit())
        || nonce.is_empty()
        || !nonce.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    Some(temp.with_file_name(name))
}

/// One temp file left behind: a write that stopped before its rename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Stray {
    pub temp: Utf8PathBuf,
    pub record: Utf8PathBuf,
}

/// Every temp file of this module's naming under `<root>/docs`, sorted.
/// Symlinked directories are not followed.
#[must_use]
pub(crate) fn stray(root: &Utf8Path) -> Vec<Stray> {
    let mut found = Vec::new();
    let mut pending = vec![root.join("docs")];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = dir.read_dir_utf8() else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path().to_path_buf();
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file()
                && let Some(record) = record_of(&path)
            {
                found.push(Stray { temp: path, record });
            }
        }
    }
    found.sort_by(|a, b| a.temp.cmp(&b.temp));
    found
}

#[cfg(unix)]
fn sync_dir(dir: &Utf8Path) -> std::io::Result<()> {
    std::fs::File::open(dir)?.sync_all()
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Utf8Path) -> std::io::Result<()> {
    Ok(())
}

/// The exit status of `OPENWARRANT_FAULT=after-temp`, distinct from every
/// status the tool reports on its own.
#[cfg(debug_assertions)]
const FAULT_EXIT: i32 = 86;

/// The debug-only fault hook. See the module documentation.
#[cfg(debug_assertions)]
fn fault(target: &Utf8Path, temp: &Utf8Path) {
    let Ok(spec) = std::env::var("OPENWARRANT_FAULT") else {
        return;
    };
    if let Ok(only) = std::env::var("OPENWARRANT_FAULT_FILE")
        && target.file_name() != Some(only.as_str())
    {
        return;
    }
    if spec == "after-temp" {
        eprintln!(
            "OPENWARRANT_FAULT=after-temp: stopped after writing {temp}, before renaming it \
             over {target}"
        );
        std::process::exit(FAULT_EXIT);
    }
    if let Some(ms) = spec
        .strip_prefix("pause-before-rename:")
        .and_then(|ms| ms.parse::<u64>().ok())
    {
        eprintln!("OPENWARRANT_FAULT=pause-before-rename: {temp} written; pausing {ms} ms");
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
}

#[cfg(not(debug_assertions))]
fn fault(_target: &Utf8Path, _temp: &Utf8Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh directory, removed when the guard drops.
    struct Scratch(Utf8PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn scratch() -> (Scratch, Utf8PathBuf) {
        static N: AtomicU64 = AtomicU64::new(0);
        let root = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8 temp dir")
            .join(format!(
                "war-atomic-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("scratch dir");
        (Scratch(root.clone()), root)
    }

    #[test]
    fn writes_and_replaces_whole() {
        let (_t, root) = scratch();
        let p = root.join("a.toml");
        write(&p, b"one").expect("first write");
        assert_eq!(std::fs::read(&p).unwrap(), b"one");
        write(&p, b"two").expect("replace");
        assert_eq!(std::fs::read(&p).unwrap(), b"two");
        assert!(stray(&root).is_empty());
        let left: Vec<_> = std::fs::read_dir(&root).unwrap().flatten().collect();
        assert_eq!(left.len(), 1, "no temp file left behind");
    }

    #[test]
    fn a_moved_prestate_is_refused_and_the_other_bytes_stand() {
        let (_t, root) = scratch();
        let p = root.join("a.toml");
        std::fs::write(&p, b"theirs").unwrap();
        let err = write_if(&p, b"mine", &Prestate::Absent).expect_err("absent was read");
        assert_eq!(err.rule, "storage.prestate-moved");
        assert_eq!(std::fs::read(&p).unwrap(), b"theirs");
        let err = write_if(&p, b"mine", &Prestate::of(b"older")).expect_err("digest moved");
        assert_eq!(err.rule, "storage.prestate-moved");
        write_if(&p, b"mine", &Prestate::of(b"theirs")).expect("prestate holds");
        assert_eq!(std::fs::read(&p).unwrap(), b"mine");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_target_is_refused() {
        let (_t, root) = scratch();
        let outside = root.join("outside");
        std::fs::write(&outside, b"keep").unwrap();
        let p = root.join("a.toml");
        std::os::unix::fs::symlink(&outside, &p).unwrap();
        let err = write(&p, b"x").expect_err("symlink");
        assert_eq!(err.rule, "storage.symlink-target");
        let err = prestate(&p).expect_err("symlink");
        assert_eq!(err.rule, "storage.symlink-target");
        assert_eq!(std::fs::read(&outside).unwrap(), b"keep");
    }

    #[test]
    fn stray_names_the_record() {
        let (_t, root) = scratch();
        let dir = root.join("docs/warrants/X");
        std::fs::create_dir_all(&dir).unwrap();
        let temp = dir.join(temp_name("authorization.toml"));
        std::fs::write(&temp, b"half").unwrap();
        std::fs::write(dir.join(".not-ours.war-tmp"), b"").unwrap();
        let found = stray(&root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].temp, temp);
        assert_eq!(found[0].record, dir.join("authorization.toml"));
    }
}
