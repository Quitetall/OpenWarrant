// SPDX-License-Identifier: Apache-2.0
//! The compiled graph, kept so `war context` answers without reading the
//! whole corpus again. Building it runs the full status of every Warrant,
//! thousands of processes on a real corpus, for a call an agent makes every
//! few turns.
//!
//! It can only make a read faster; it never changes an answer. An entry is
//! used only when its key matches, and the key covers everything the graph
//! is made from: this `war` binary, the worktree, its `HEAD`, and every path
//! `git status` reports with its stat. The graph reads tracked inputs only
//! ([`super::adapter`]), so an ignored file cannot change it.
//!
//! Only `war context` reads it: nothing that judges a record or authority
//! does. It is off in a hosted or recording run, whose every read must be
//! observed, outside a git checkout, and under `OPENWARRANT_NO_CACHE`. It
//! lives in `<git common dir>/openwarrant/cache/`, one file per worktree,
//! written to a temporary name and renamed. A failed write is ignored. An
//! edit in the last [`SETTLE`] is not stored, so a write in the same
//! timestamp tick as the read cannot hide behind an unchanged stat.

use std::os::unix::fs::MetadataExt as _;
use std::time::{SystemTime, UNIX_EPOCH};

use camino::{Utf8Path, Utf8PathBuf};

use super::Graph;
use crate::corpus::Corpus;
use crate::ticket::cache::{DISABLE_ENV, SETTLE, SUBDIR, identity};

/// A change to the file's layout or to [`Graph`] changes this.
const FORMAT: u32 = 1;

/// The key, and whether the working tree is settled enough to store under it.
fn key(root: &Utf8Path) -> Option<(String, bool)> {
    let head = crate::go::git::rev(root, "HEAD")?;
    let status = std::process::Command::new("git")
        .args(["-C", root.as_str(), "status", "--porcelain=v1", "-z"])
        .args(["--untracked-files=all", "--no-renames"])
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let mut hash = blake3::Hasher::new();
    hash.update(format!("{FORMAT}\0{}\0{root}\0{head}\0", identity()?).as_bytes());
    hash.update(&status.stdout);
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let mut settled = true;
    for entry in status.stdout.split(|b| *b == 0) {
        let Some(path) = entry.get(3..).and_then(|p| std::str::from_utf8(p).ok()) else {
            continue;
        };
        if let Ok(m) = std::fs::symlink_metadata(root.join(path)) {
            let (mtime, ctime) = (m.mtime(), m.ctime());
            let newest = u64::try_from(mtime.max(ctime)).unwrap_or(u64::MAX);
            settled &= now.saturating_sub(newest) >= SETTLE.as_secs();
            hash.update(
                format!(
                    "{} {} {} {} {}\0",
                    m.len(),
                    mtime,
                    m.mtime_nsec(),
                    ctime,
                    m.ctime_nsec()
                )
                .as_bytes(),
            );
        }
    }
    Some((hash.finalize().to_hex().to_string(), settled))
}

fn path(root: &Utf8Path) -> Option<Utf8PathBuf> {
    let name = blake3::hash(root.as_str().as_bytes()).to_hex();
    Some(
        crate::go::git::common_dir(root)?
            .join(SUBDIR)
            .join(format!("context-graph-{}.json", &name[..16])),
    )
}

fn load(file: &Utf8Path, key: &str) -> Option<Graph> {
    let bytes = std::fs::read(file).ok()?;
    let at = bytes.iter().position(|b| *b == b'\n')?;
    (bytes[..at] == *key.as_bytes())
        .then(|| serde_json::from_slice(&bytes[at + 1..]).ok())
        .flatten()
}

fn store(file: &Utf8Path, key: &str, graph: &Graph) -> Option<()> {
    let mut bytes = format!("{key}\n").into_bytes();
    serde_json::to_writer(&mut bytes, graph).ok()?;
    std::fs::create_dir_all(file.parent()?).ok()?;
    let temporary = file.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temporary, bytes).ok()?;
    std::fs::rename(&temporary, file).ok()
}

fn off() -> bool {
    !crate::vfs::is_disk() || std::env::var(DISABLE_ENV).is_ok_and(|v| !v.is_empty() && v != "0")
}

/// The graph of [`super::graph_of`], from the cache when nothing it is made
/// from has changed.
///
/// # Errors
/// As [`super::graph_of`].
pub fn graph(corpus: &Corpus) -> Result<Graph, crate::repo::RepoError> {
    let root = &corpus.repo().root;
    let found = (!off()).then(|| key(root).zip(path(root))).flatten();
    let Some(((key, settled), file)) = found else {
        return super::graph_of(corpus);
    };
    if let Some(graph) = load(&file, &key) {
        return Ok(graph);
    }
    let graph = super::graph_of(corpus)?;
    if settled {
        let _ = store(&file, &key, &graph);
    }
    Ok(graph)
}
