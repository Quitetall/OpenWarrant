// SPDX-License-Identifier: Apache-2.0
//! What `ssh-keygen -Y verify` already answered, so the daily read verbs do
//! not ask it again.
//!
//! `war next` on this repository asked `ssh-keygen` about 140 times per run,
//! once for each distinct signed response and batch (131), plus a few more
//! when two threads asked the same question at once. The per-process memo in
//! `authority_check` already kept the count to the distinct inputs; the cost
//! was that every run asked all of them again, each a process spawn, and a
//! spawn on a loaded machine is slow. Two things here:
//!
//! - [`Slots`]: one slot per question, held across the asking, so threads
//!   asking the same question at once wait for the first answer instead of
//!   each spawning `ssh-keygen` (the same fix `gate_cmd::source::tree_read_in`
//!   carries for its git reads).
//! - A persistent cache of **good** verdicts, for the one-shot read-only
//!   advice verbs only ([`permit`]).
//!
//! # What the persistent cache may and may not do
//!
//! It can only make a read faster; it never changes an answer.
//!
//! - **Key.** The SHA-256 of every input `ssh-keygen -Y verify` reads, each
//!   length-prefixed: the allowed_signers file's bytes, the principal (`-I`),
//!   the namespace (`-n`), the message (stdin), the signature file's bytes,
//!   and `ssh-keygen`'s own identity (its resolved path and the SHA-256 of
//!   its executable's bytes), under this cache's format tag. A changed byte
//!   in any of them is another key, never a stale answer.
//! - **Exactly those bytes.** When the cache is permitted, `ssh-keygen` is
//!   run over private copies of exactly the bytes that were hashed (message
//!   on stdin from memory), so the verdict stored is the verdict for the key
//!   and a file that changes while it is read cannot be recorded under the
//!   other contents' key.
//! - **Good only.** Only an accepted signature is stored. A rejection, a
//!   missing `ssh-keygen`, a file that could not be read: never stored, and
//!   asked again on every run through the ordinary path, which also supplies
//!   the ordinary message. A tampered signature is a new key and is refused
//!   by `ssh-keygen` exactly as before.
//! - **Time.** `ssh-keygen` consults the clock only for an allowed_signers
//!   line with `valid-after`, `valid-before` or `cert-authority` (certificate
//!   validity). A file mentioning any of them is never cached: a "good" there
//!   is good only now.
//! - **Not keyed.** The shared libraries `ssh-keygen` links (libcrypto) are
//!   not part of the key. A library update that changed whether a valid
//!   signature verifies would be seen by every judging command, which never
//!   reads this cache, but not by the advice verbs until the binary or an
//!   input changed. `OPENWARRANT_NO_CACHE=1` turns the cache off.
//! - **Scope.** Only the commands that call [`permit`] use it: `war next`,
//!   `war status`, `war view overview` (plain and `--snapshot`) and `war view
//!   board` — one-shot read-only advice. `war check` (and `--generated`),
//!   compile, `war sign` (including `--list` and every dry run outside `war
//!   next`), authorize, resolve, verify, the MCP server and every long-running
//!   server never permit it: each of those is a judgement or a gate, and asks
//!   `ssh-keygen` itself. It is off in a hosted or recording run
//!   ([`crate::vfs::is_disk`]), outside a git checkout, on a non-Unix host,
//!   and whenever [`DISABLE_ENV`] is set to anything but `0` or nothing.
//! - **Location.** `<git common dir>/openwarrant/cache/ssh-verdicts/`: never
//!   in the tree, never committed, gone with the clone. An entry is a file
//!   named by its key whose content is the format tag and the key; anything
//!   else there is ignored. A write is a temporary file renamed into place,
//!   and a failed write is ignored: the cache is never needed.
//! - **Trust.** Whoever can write `.git` can plant an entry, and could as
//!   well rewrite the tree. An entry only ever turns the advice verbs' answer
//!   to "verified"; every command that records, gates or signs asks
//!   `ssh-keygen` itself.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use camino::{Utf8Path, Utf8PathBuf};

/// Set to anything but `0` or nothing: the cache is neither read nor written.
pub const DISABLE_ENV: &str = "OPENWARRANT_NO_CACHE";

/// Where under git's common directory the entries lie.
pub const SUBDIR: &str = "openwarrant/cache/ssh-verdicts";

/// The cache's own format; part of every key and every entry.
const FORMAT: &str = "oh.war/ssh-verdict-cache/v1";

/// The allowed_signers options under which `ssh-keygen` reads the clock.
const TIME_BOUND: [&str; 3] = ["valid-after", "valid-before", "cert-authority"];

static PERMITTED: AtomicBool = AtomicBool::new(false);

/// Let this process read and write the cache: for the one-shot read-only
/// advice verbs only (see the module docs).
pub fn permit() {
    PERMITTED.store(true, Ordering::Relaxed);
}

/// Whether this thread may use the cache now.
#[must_use]
pub fn permitted() -> bool {
    cfg!(unix)
        && PERMITTED.load(Ordering::Relaxed)
        && crate::vfs::is_disk()
        && !std::env::var_os(DISABLE_ENV).is_some_and(|v| !v.is_empty() && v != "0")
}

/// One memo of answers, one slot per question. The slot is held while the
/// question is asked, so a second thread asking it waits and takes the
/// answer; an answer the asker declines to keep leaves the slot empty.
pub struct Slots<V> {
    map: Mutex<HashMap<String, Arc<Mutex<Option<V>>>>>,
}

impl<V: Clone> Default for Slots<V> {
    fn default() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
        }
    }
}

impl<V: Clone> Slots<V> {
    /// The remembered answer for `key` (through `recall`), or `ask()`'s,
    /// remembered when `keep` returns one.
    pub fn ask<R>(
        &self,
        key: String,
        ask: impl FnOnce() -> R,
        keep: impl FnOnce(&R) -> Option<V>,
        recall: impl FnOnce(V) -> R,
    ) -> R {
        let slot = Arc::clone(
            self.map
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .entry(key)
                .or_default(),
        );
        let mut answer = slot.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(known) = answer.as_ref() {
            return recall(known.clone());
        }
        let fresh = ask();
        *answer = keep(&fresh);
        fresh
    }
}

/// Exactly what one `ssh-keygen -Y verify` reads.
pub struct Question<'a> {
    pub allowed_signers: &'a [u8],
    pub principal: &'a str,
    pub namespace: &'a str,
    pub message: &'a [u8],
    pub signature: &'a [u8],
}

/// `ssh-keygen` as found on `PATH`, and the digest of its bytes. `None` when
/// it cannot be found or read: then there is no cache, and the ordinary path
/// reports what it reports.
#[must_use]
pub fn ssh_keygen() -> Option<&'static (Utf8PathBuf, String)> {
    static ID: OnceLock<Option<(Utf8PathBuf, String)>> = OnceLock::new();
    ID.get_or_init(|| {
        let path = std::env::var_os("PATH")?;
        let found = std::env::split_paths(&path)
            .map(|d| d.join("ssh-keygen"))
            .find(|p| is_executable(p))?;
        let found = Utf8PathBuf::from_path_buf(std::fs::canonicalize(found).ok()?).ok()?;
        let bytes = std::fs::read(&found).ok()?;
        Some((found, openwarrant_compiler::sha256_hex(&bytes)))
    })
    .as_ref()
}

#[cfg(unix)]
fn is_executable(p: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(_: &std::path::Path) -> bool {
    false
}

impl Question<'_> {
    /// The cache key, or `None` when this question must never be cached:
    /// an allowed_signers file under which the verdict depends on the clock,
    /// or an `ssh-keygen` that cannot be identified.
    #[must_use]
    pub fn key(&self, tool: &(Utf8PathBuf, String)) -> Option<String> {
        let lower = String::from_utf8_lossy(self.allowed_signers).to_ascii_lowercase();
        if TIME_BOUND.iter().any(|o| lower.contains(o)) {
            return None;
        }
        let mut material = Vec::new();
        for part in [
            FORMAT.as_bytes(),
            tool.0.as_str().as_bytes(),
            tool.1.as_bytes(),
            self.allowed_signers,
            self.principal.as_bytes(),
            self.namespace.as_bytes(),
            self.message,
            self.signature,
        ] {
            material.extend_from_slice(&(part.len() as u64).to_le_bytes());
            material.extend_from_slice(part);
        }
        Some(openwarrant_compiler::sha256_hex(&material))
    }
}

/// The cache directory for the checkout at `root`, or `None` outside git.
#[must_use]
pub fn dir_for(root: &Utf8Path) -> Option<Utf8PathBuf> {
    crate::ticket::claim::git_layout(root).map(|l| l.common_dir.join(SUBDIR))
}

fn entry_path(dir: &Utf8Path, key: &str) -> Utf8PathBuf {
    dir.join(key)
}

fn entry_body(key: &str) -> String {
    format!("{FORMAT}\n{key}\n")
}

/// Whether `dir` holds a good verdict for `key`.
#[must_use]
pub fn known_good(dir: &Utf8Path, key: &str) -> bool {
    std::fs::read(entry_path(dir, key)).is_ok_and(|b| b == entry_body(key).as_bytes())
}

/// What [`verify_exact`] observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exact {
    /// `ssh-keygen` accepted exactly these bytes.
    Good,
    /// It did not, or could not be asked. The caller asks the ordinary way,
    /// which reports why.
    NotGood,
}

/// Ask `tool` about exactly the bytes of `q`, from private copies under
/// `dir`, and record a good verdict under `key`. Never records anything else.
pub fn verify_exact(dir: &Utf8Path, key: &str, tool: &Utf8Path, q: &Question<'_>) -> Exact {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    if std::fs::create_dir_all(dir).is_err() {
        return Exact::NotGood;
    }
    let work = dir.join(format!(
        ".ask-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    if std::fs::create_dir(&work).is_err() {
        return Exact::NotGood;
    }
    let verdict = ask_in(&work, tool, q);
    let _ = std::fs::remove_dir_all(&work);
    if verdict == Exact::Good {
        remember_good(dir, key);
    }
    verdict
}

fn ask_in(work: &Utf8Path, tool: &Utf8Path, q: &Question<'_>) -> Exact {
    use std::io::Write as _;
    let allowed = work.join("allowed_signers");
    let sig = work.join("message.sig");
    if std::fs::write(&allowed, q.allowed_signers).is_err()
        || std::fs::write(&sig, q.signature).is_err()
    {
        return Exact::NotGood;
    }
    let Ok(mut child) = std::process::Command::new(tool)
        .args(["-Y", "verify", "-f"])
        .arg(&allowed)
        .args(["-I", q.principal, "-n", q.namespace, "-s"])
        .arg(&sig)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    else {
        return Exact::NotGood;
    };
    // The message is small (a response or a batch document); a write error
    // (ssh-keygen exiting early) is a verdict of its own below.
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(q.message);
    }
    match child.wait() {
        Ok(status) if status.success() => Exact::Good,
        _ => Exact::NotGood,
    }
}

fn remember_good(dir: &Utf8Path, key: &str) {
    let tmp = dir.join(format!(".{key}.{}.tmp", std::process::id()));
    if std::fs::write(&tmp, entry_body(key)).is_ok()
        && std::fs::rename(&tmp, entry_path(dir, key)).is_err()
    {
        let _ = std::fs::remove_file(&tmp);
    }
}
