// SPDX-License-Identifier: Apache-2.0
//! A local cache of parsed tickets: the daily read verbs at 10,000 Warrants
//! (CONTRIBUTING.md, "The speed budget").
//!
//! Every run of `war next` or `war view ready` read and parsed every ticket:
//! its `manifest.toml` (TOML), both atoms, and, for an open ticket, its
//! journal. At 9,800 tickets that was most of the run. This module keeps what
//! those reads produced — the manifest's fields, the atoms' text, the
//! manifest's revision, and an open ticket's set-aside nodes
//! ([`crate::go::BlockedNode`]) — beside the stat of every file it came from,
//! and answers a ticket from it only when every one of those files stats
//! exactly as it did when it was read.
//!
//! # What it may and may not do
//!
//! It can only make a read faster; it never changes an answer.
//!
//! - **Validity.** An entry is used only when each source file's (size,
//!   mtime, ctime, inode, device, mode) is unchanged, from one `stat` per
//!   file taken now. ctime is in the key because `touch -d` can put mtime
//!   back and nothing unprivileged can put ctime back. A file is stat'ed
//!   *before* it is read, so the stat an entry carries is never newer than
//!   the bytes it describes. An entry is written only for files whose mtime
//!   and ctime are at least [`SETTLE`] old, so a write in the same timestamp
//!   tick as the read ("racily clean", as git calls it) cannot hide behind
//!   an unchanged stat; such a ticket is read from its files until it
//!   settles.
//! - **The listing, and only the listing, by its directory.** The store's
//!   sorted directory listing is kept under the directory's own stat: an
//!   entry made, removed or renamed in a directory changes that directory's
//!   mtime and ctime, so an unchanged stat means the same names. That says
//!   nothing about the files inside, which an in-place edit changes without
//!   touching any directory: every ticket is still judged by its own files'
//!   stats, never by a directory's.
//! - **Journals.** `war next` lists the nodes a run set aside, read from
//!   each open ticket's journal ([`permit_with_journals`]); the nodes are
//!   kept beside the journal's stat and the ticket entry they were read
//!   with. A journal that was absent is keyed as absent.
//! - **Refusal.** Anything else — a missing, unreadable, truncated or
//!   corrupt shard, a checksum that does not match, another cache format, a
//!   different `war` binary ([`identity`]), another ticket profile, another
//!   store directory — and the ticket is read from its files, and the shard
//!   rewritten. Nothing in a shard is trusted until its BLAKE3 checksum and
//!   its key both match.
//! - **Scope.** Only the commands that call [`permit`] use it (`war next`,
//!   `war view ready`, `war view tickets`, `war status`, `war claim`): views of
//!   the ticket loop. `war check`, compile, and every signing, authorizing,
//!   resolving and verifying path never permit it, so nothing that judges a
//!   record or authority reads it. It is off in a hosted (`war admin host`)
//!   or recording run, whose every read must be observed ([`vfs::is_disk`]),
//!   outside a git checkout, on a non-Unix host, and whenever
//!   `OPENWARRANT_NO_CACHE` is set (to anything but `0` or nothing).
//! - **Location.** Under git's common directory, `<common>/openwarrant/cache/`,
//!   beside M11's shared claims: never in the tree, so never committed and
//!   never in `git status`, and gone with the clone. Each worktree's store has
//!   its own files there (the name carries a digest of the store's path).
//! - **Writes.** Only a shard that gained a new entry is rewritten, as a
//!   temporary file renamed over it: a reader sees the old shard or the new
//!   one, never part of either, and two processes racing each write a whole
//!   shard (the last rename wins). A failed write is ignored; the cache is
//!   never needed.
//!
//! [`vfs::is_disk`]: crate::vfs::is_disk

use std::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_core::ticket::{self, CHECKLIST_ROLE, TicketAtom, TicketManifest};

use super::{Store, Ticket};
use crate::diagnostic::Diagnostic;
use crate::go::BlockedNode;

/// Set to anything but `0` or nothing: the cache is neither read nor written.
pub const DISABLE_ENV: &str = "OPENWARRANT_NO_CACHE";

/// Set to anything but `0` or nothing: each use of the cache says on stderr
/// what it answered from the cache and what it read from the files.
pub const TRACE_ENV: &str = "OPENWARRANT_CACHE_TRACE";

/// Where under git's common directory the cache lies.
pub const SUBDIR: &str = "openwarrant/cache";

/// How old a source file's mtime and ctime must be before an entry built
/// from it is written. Far above any local filesystem's timestamp tick.
pub const SETTLE: Duration = Duration::from_secs(2);

/// The cache's own format. Any change to what an entry holds or how it is
/// laid out changes this; a shard of another format is refused.
const FORMAT: u32 = 2;
const MAGIC: &[u8; 8] = b"OWTCACHE";
/// Shards per store: a changed ticket rewrites one shard, not the cache.
const SHARDS: usize = 64;

static PERMITTED: AtomicBool = AtomicBool::new(false);
static JOURNALS: AtomicBool = AtomicBool::new(false);

/// Let this process read and write the cache: for the one-shot read verbs of
/// the ticket loop only (see the module docs).
pub fn permit() {
    PERMITTED.store(true, Ordering::Relaxed);
}

/// [`permit`], and have each open ticket's set-aside nodes
/// ([`crate::go::blocked_nodes`]) read with it: for `war next`, which lists
/// them. Without this the journals are neither read nor stat'ed.
pub fn permit_with_journals() {
    JOURNALS.store(true, Ordering::Relaxed);
    permit();
}

/// Whether this thread may use the cache at all: a Unix host, a command
/// that [`permit`]ted it, reads that go straight to the disk (never a hosted
/// or recording run, whose every read is observed), and no
/// [`DISABLE_ENV`].
fn may_use() -> bool {
    may_use_if(PERMITTED.load(Ordering::Relaxed))
}

/// [`may_use`] for a command that did (`true`) or did not permit it: what
/// a test asks without permitting it for its whole process, whose other
/// tests read this repository's own tickets.
fn may_use_if(permitted: bool) -> bool {
    cfg!(unix) && permitted && crate::vfs::is_disk() && !disabled_by_env()
}

fn env_on(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|v| !v.is_empty() && v != "0")
}

fn disabled_by_env() -> bool {
    env_on(DISABLE_ENV)
}

/// The binary's identity as the cache keys it: `war --version`'s line, the
/// build profile, and the stat of the executable itself, so two builds of
/// one dirty commit never share a cache. `None` when the executable cannot
/// be found, and then there is no cache.
#[must_use]
pub fn identity() -> Option<String> {
    static ID: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    ID.get_or_init(|| {
        let exe = std::env::current_exe().ok()?;
        let m = std::fs::metadata(exe).ok()?;
        let s = Stamp::of(&m)?;
        Some(format!(
            "{}\n{}\n{} {}.{} {} {}",
            crate::build_identity::version_line(),
            crate::build_identity::embedded_profile(),
            s.len,
            s.mtime,
            s.mtime_ns,
            s.ino,
            s.dev
        ))
    })
    .clone()
}

/// Whether the cache would be used for the repository at `root`, and why
/// not when it would not: for `war admin doctor`.
#[must_use]
pub fn describe(root: &Utf8Path) -> serde_json::Value {
    let off = |why: &str| serde_json::json!({"in_use": false, "why": why});
    if !cfg!(unix) {
        return off("not a Unix host");
    }
    if disabled_by_env() {
        return off(&format!("{DISABLE_ENV} is set"));
    }
    let Some(layout) = super::claim::git_layout(root) else {
        return off("not a git checkout");
    };
    if identity().is_none() {
        return off("the running executable cannot be found");
    }
    let dir = layout.common_dir.join(SUBDIR);
    let shards = std::fs::read_dir(&dir).map_or(0, |rd| {
        rd.filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".bin"))
            .count()
    });
    serde_json::json!({
        "in_use": true,
        "dir": dir.as_str(),
        "shards": shards,
        "commands": ["next", "view ready", "view tickets", "status", "claim"],
        "disable": format!("{DISABLE_ENV}=1"),
    })
}

/// One file's stat, as an entry keys it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp {
    len: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
    ino: u64,
    dev: u64,
    mode: u32,
}

impl Stamp {
    #[cfg(unix)]
    fn of(m: &std::fs::Metadata) -> Option<Self> {
        use std::os::unix::fs::MetadataExt;
        Some(Self {
            len: m.size(),
            mtime: m.mtime(),
            mtime_ns: m.mtime_nsec(),
            ctime: m.ctime(),
            ctime_ns: m.ctime_nsec(),
            ino: m.ino(),
            dev: m.dev(),
            mode: m.mode(),
        })
    }

    #[cfg(not(unix))]
    fn of(_: &std::fs::Metadata) -> Option<Self> {
        None
    }
}

/// What one `stat` (following symlinks, as `vfs::is_file` does) found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Seen {
    Absent,
    File(Stamp),
}

/// `None`: something other than a regular file or plain absence (a
/// directory, a permission error): nothing about it is cached.
fn seen(p: &Utf8Path) -> Option<Seen> {
    match std::fs::metadata(p) {
        Ok(m) if m.is_file() => Stamp::of(&m).map(Seen::File),
        Ok(_) => None,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(Seen::Absent),
        Err(_) => None,
    }
}

/// The cache for one store, for one run.
pub(crate) struct Cache {
    dir: Utf8PathBuf,
    /// File-name prefix: a digest of the store's directory.
    prefix: String,
    /// Every file's key: the format, the binary, the store and the ticket
    /// profile. A file with another key is ignored.
    key: [u8; 32],
    /// An entry is written only for files last changed before this.
    settled_before: (i64, i64),
    /// Whether open tickets' set-aside nodes are read too
    /// ([`permit_with_journals`]).
    journals: bool,
}

/// Where one entry lies in its shard: its name, its ticket part, and its
/// journal part when it has one.
type Place = (Range<usize>, Range<usize>, Option<Range<usize>>);

/// One shard as read: its bytes, and each entry's place in them, in name
/// order (the order it was written in, which a shard must keep).
#[derive(Default)]
struct Shard {
    bytes: Vec<u8>,
    index: Vec<Place>,
}

impl Shard {
    fn get(&self, name: &str) -> Option<&Place> {
        self.index
            .binary_search_by(|(n, _, _)| self.bytes[n.clone()].cmp(name.as_bytes()))
            .ok()
            .map(|i| &self.index[i])
    }
}

/// One part of an entry, after this run.
enum Part {
    /// Nothing to write: not cacheable now (unsettled, unusual, or not
    /// applicable).
    None,
    /// The shard's own bytes, still valid.
    Old(Range<usize>),
    /// Built this run from the files.
    New(Vec<u8>),
}

/// One ticket directory, after this run.
struct Got {
    ticket: Option<Result<Ticket, Diagnostic>>,
    shard: usize,
    ticket_part: Part,
    journal_part: Part,
}

impl Got {
    fn plain(ticket: Option<Result<Ticket, Diagnostic>>) -> Self {
        Self {
            ticket,
            shard: 0,
            ticket_part: Part::None,
            journal_part: Part::None,
        }
    }
}

fn shard_of(name: &str) -> usize {
    // FNV-1a: stable across builds and platforms, which a std hasher is not.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in name.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    usize::try_from(h % SHARDS as u64).unwrap_or(0)
}

impl Cache {
    /// The cache for `store`, when this run may use one (module docs).
    pub(crate) fn open(store: &Store) -> Option<Self> {
        if !may_use() {
            return None;
        }
        let layout = super::claim::git_layout(&store.root)?;
        let identity = identity()?;
        let mut h = blake3::Hasher::new();
        h.update(&FORMAT.to_le_bytes());
        for part in [
            identity.as_str(),
            store.dir.as_str(),
            &format!("{:?}", store.definition),
        ] {
            h.update(&(part.len() as u64).to_le_bytes());
            h.update(part.as_bytes());
        }
        let key = *h.finalize().as_bytes();
        let prefix = format!(
            "tickets-{}",
            &blake3::hash(store.dir.as_str().as_bytes()).to_hex()[..16]
        );
        let settled = SystemTime::now()
            .checked_sub(SETTLE)
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())?;
        Some(Self {
            dir: layout.common_dir.join(SUBDIR),
            prefix,
            key,
            settled_before: (
                i64::try_from(settled.as_secs()).ok()?,
                i64::from(settled.subsec_nanos()),
            ),
            journals: JOURNALS.load(Ordering::Relaxed),
        })
    }

    fn shard_path(&self, i: usize) -> Utf8PathBuf {
        self.dir.join(format!("{}-{i:02}.bin", self.prefix))
    }

    fn list_path(&self) -> Utf8PathBuf {
        self.dir.join(format!("{}-list.bin", self.prefix))
    }

    fn settled(&self, s: &Stamp) -> bool {
        (s.mtime, s.mtime_ns) < self.settled_before && (s.ctime, s.ctime_ns) < self.settled_before
    }

    /// `payload` framed as a cache file: the magic, the key, its length and
    /// its BLAKE3 checksum, then itself.
    fn frame(&self, payload: &[u8]) -> Vec<u8> {
        let mut file = Vec::with_capacity(payload.len() + 80);
        file.extend_from_slice(MAGIC);
        file.extend_from_slice(&self.key);
        file.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        file.extend_from_slice(blake3::hash(payload).as_bytes());
        file.extend_from_slice(payload);
        file
    }

    /// Where the payload of a framed file starts, when every byte of the
    /// frame is exactly right: the magic, this key, the length to the last
    /// byte, and the checksum. Anything else is `None`, and nothing in the
    /// file is read.
    fn unframe(&self, bytes: &[u8]) -> Option<usize> {
        let mut r = Reader::new(bytes);
        if r.take(8)? != MAGIC || r.take(32)? != self.key {
            return None;
        }
        let len = usize::try_from(r.u64()?).ok()?;
        let sum = r.take(32)?;
        let start = r.pos;
        let payload = bytes.get(start..start.checked_add(len)?)?;
        (start + len == bytes.len() && blake3::hash(payload).as_bytes() == sum).then_some(start)
    }

    /// `bytes` at `path`, whole or not at all: a temporary file renamed over
    /// it. A failure leaves the old file (or none) and is otherwise ignored.
    fn write_atomic(&self, path: &Utf8Path, bytes: &[u8]) {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let tmp = self.dir.join(format!(
            ".{}.{}.{nanos}.{}.tmp",
            path.file_name().unwrap_or("cache"),
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        if std::fs::create_dir_all(&self.dir).is_err()
            || std::fs::write(&tmp, bytes).is_err()
            || std::fs::rename(&tmp, path).is_err()
        {
            let _ = std::fs::remove_file(&tmp);
        }
    }

    /// One shard, or an empty one for anything not exactly a shard of this
    /// key: missing, unreadable, short, corrupt, out of order, or another
    /// key.
    fn read_shard(&self, i: usize) -> Shard {
        let Ok(bytes) = std::fs::read(self.shard_path(i)) else {
            return Shard::default();
        };
        self.parse_shard(bytes).unwrap_or_default()
    }

    fn parse_shard(&self, bytes: Vec<u8>) -> Option<Shard> {
        let mut r = Reader::new(&bytes);
        r.pos = self.unframe(&bytes)?;
        let n = r.u32()?;
        let mut index: Vec<Place> = Vec::with_capacity(n.min(1 << 16) as usize);
        for _ in 0..n {
            let name = r.range()?;
            if std::str::from_utf8(&bytes[name.clone()]).is_err()
                || index
                    .last()
                    .is_some_and(|(prev, _, _)| bytes[prev.clone()] >= bytes[name.clone()])
            {
                return None;
            }
            let ticket = r.range()?;
            let journal = r.range()?;
            index.push((name, ticket, (!journal.is_empty()).then_some(journal)));
        }
        (r.pos == bytes.len()).then_some(Shard { bytes, index })
    }

    /// The store's directory listing, as [`Store::entries`] gives it: from
    /// the cache when the directory stats as it did when it was listed (an
    /// entry made, removed or renamed in a directory changes its mtime and
    /// ctime), else from the directory, stat'ed first, and the listing
    /// cached once the directory has settled.
    fn entries(&self, store: &Store) -> Result<(Vec<Utf8PathBuf>, bool), crate::repo::RepoError> {
        let stamp = std::fs::metadata(&store.dir)
            .ok()
            .filter(std::fs::Metadata::is_dir)
            .and_then(|m| Stamp::of(&m));
        if let Some(stamp) = stamp
            && let Some(names) = self.read_list(stamp)
        {
            return Ok((names.into_iter().map(|n| store.dir.join(n)).collect(), true));
        }
        let entries = store.entries()?;
        if let Some(stamp) = stamp.filter(|s| self.settled(s)) {
            let names: Vec<&str> = entries.iter().filter_map(|p| p.file_name()).collect();
            if names.len() == entries.len() {
                let mut w = Writer::default();
                w.stamp(&stamp);
                w.u32(u32::try_from(names.len()).unwrap_or(u32::MAX));
                for n in names {
                    w.str(n);
                }
                self.write_atomic(&self.list_path(), &self.frame(&w.buf));
            }
        }
        Ok((entries, false))
    }

    fn read_list(&self, stamp: Stamp) -> Option<Vec<String>> {
        let bytes = std::fs::read(self.list_path()).ok()?;
        let mut r = Reader::new(&bytes);
        r.pos = self.unframe(&bytes)?;
        if r.stamp()? != stamp {
            return None;
        }
        let n = r.u32()?;
        let mut out: Vec<String> = Vec::with_capacity(n.min(1 << 20) as usize);
        for _ in 0..n {
            let name = r.str()?;
            // A name with a separator, or one out of order, was never
            // written here.
            if name.is_empty()
                || name.contains('/')
                || out.last().is_some_and(|prev| prev.as_str() >= name)
            {
                return None;
            }
            out.push(name.to_owned());
        }
        (r.pos == bytes.len()).then_some(out)
    }

    /// [`Store::load_all`]'s per-directory answers, in listing order: exactly
    /// what reading the files gives, with the cache rewritten where it
    /// gained an entry. The shards are read while the store is listed.
    #[allow(clippy::type_complexity)]
    pub(crate) fn load(
        &self,
        store: &Store,
    ) -> Result<(Vec<Utf8PathBuf>, Vec<Option<Result<Ticket, Diagnostic>>>), crate::repo::RepoError>
    {
        let ids: Vec<usize> = (0..SHARDS).collect();
        let read = || crate::vfs::map_ordered(&ids, 1, |&i| self.read_shard(i));
        let (entries, shards) = std::thread::scope(|scope| {
            let reading = std::thread::Builder::new().spawn_scoped(scope, read).ok();
            let entries = self.entries(store);
            let shards = match reading.map(|h| h.join()) {
                Some(Ok(shards)) => shards,
                Some(Err(panic)) => std::panic::resume_unwind(panic),
                None => read(),
            };
            (entries, shards)
        });
        let (entries, listed) = entries?;
        let got = crate::vfs::map_ordered(&entries, 256, |dir| self.one(store, dir, &shards));
        let mut dirty = [false; SHARDS];
        for g in &got {
            if matches!(g.ticket_part, Part::New(_)) || matches!(g.journal_part, Part::New(_)) {
                dirty[g.shard] = true;
            }
        }
        let dirty: Vec<usize> = (0..SHARDS).filter(|&i| dirty[i]).collect();
        crate::vfs::map_ordered(&dirty, 1, |&i| {
            self.write_shard(i, &shards[i], &entries, &got);
        });
        if env_on(TRACE_ENV) {
            let tickets = got
                .iter()
                .filter(|g| matches!(g.ticket, Some(Ok(_))))
                .count();
            let cached = got
                .iter()
                .filter(|g| matches!(g.ticket_part, Part::Old(_)))
                .count();
            eprintln!(
                "ticket cache: {cached} of {tickets} ticket(s) from the cache, {} read from their \
                 files; {} shard(s) written; the listing from the {}",
                tickets - cached,
                dirty.len(),
                if listed { "cache" } else { "directory" }
            );
        }
        Ok((entries, got.into_iter().map(|g| g.ticket).collect()))
    }

    /// Shard `i` again: every entry of it this run can vouch for, in
    /// listing (name) order.
    fn write_shard(&self, i: usize, old: &Shard, entries: &[Utf8PathBuf], got: &[Got]) {
        let mut w = Writer::default();
        let mut n = 0u32;
        w.u32(0);
        for (dir, g) in entries.iter().zip(got) {
            if g.shard != i {
                continue;
            }
            let part = |p: &Part| -> Option<Vec<u8>> {
                match p {
                    Part::None => None,
                    Part::Old(r) => old.bytes.get(r.clone()).map(<[u8]>::to_vec),
                    Part::New(b) => Some(b.clone()),
                }
            };
            let (Some(name), Some(t)) = (dir.file_name(), part(&g.ticket_part)) else {
                continue;
            };
            let j = part(&g.journal_part).unwrap_or_default();
            w.str(name);
            w.bytes(&t);
            w.bytes(&j);
            n += 1;
        }
        let mut payload = w.buf;
        payload[..4].copy_from_slice(&n.to_le_bytes());
        self.write_atomic(&self.shard_path(i), &self.frame(&payload));
    }

    fn one(&self, store: &Store, dir: &Utf8PathBuf, shards: &[Shard]) -> Got {
        let manifest = dir.join("manifest.toml");
        let (Some(name), Some(Seen::File(stamp))) = (dir.file_name(), seen(&manifest)) else {
            // Absent, unusual or unnamed: exactly the uncached read.
            return Got::plain(crate::vfs::is_file(&manifest).then(|| store.load(dir)));
        };
        let shard = shard_of(name);
        let old = &shards[shard];
        if let Some((_, tr, jr)) = old.get(name)
            && let Some(mut t) = old
                .bytes
                .get(tr.clone())
                .and_then(|b| hit(&store.root, dir, stamp, b))
        {
            let old_journal = jr
                .as_ref()
                .and_then(|r| old.bytes.get(r.clone()).map(|b| (r.clone(), b)));
            let journal_part = if self.journals {
                self.journal(&mut t, old_journal)
            } else {
                // Not asked for: kept as it was, judged when it is.
                old_journal.map_or(Part::None, |(r, _)| Part::Old(r))
            };
            return Got {
                ticket: Some(Ok(t)),
                shard,
                ticket_part: Part::Old(tr.clone()),
                journal_part,
            };
        }
        // A miss: read the files, stat'ing each just before it is read.
        let mut stamps: Vec<Option<Seen>> = Vec::with_capacity(3);
        let loaded = store.load_observed(dir, &mut |p| stamps.push(seen(p)));
        let Ok(mut t) = loaded else {
            return Got::plain(Some(loaded));
        };
        let files: Vec<Stamp> = stamps
            .iter()
            .filter_map(|s| match s {
                Some(Seen::File(s)) => Some(*s),
                _ => None,
            })
            .collect();
        let ticket_part = if files.len() == 3 && files.iter().all(|s| self.settled(s)) {
            Part::New(encode_ticket(&files, &t))
        } else {
            Part::None
        };
        let journal_part = if self.journals {
            self.journal(&mut t, None)
        } else {
            Part::None
        };
        Got {
            ticket: Some(Ok(t)),
            shard,
            // A journal part is kept only beside the ticket part it was
            // judged with.
            journal_part: if matches!(ticket_part, Part::None) {
                Part::None
            } else {
                journal_part
            },
            ticket_part,
        }
    }

    /// An open ticket's set-aside nodes, onto `t`: the cached ones when the
    /// journal stats as it did, else [`crate::go::blocked_of`] over the
    /// files, stat'ed first. A done ticket has none to cache.
    fn journal(&self, t: &mut Ticket, old: Option<(Range<usize>, &[u8])>) -> Part {
        if t.is_done() {
            return Part::None;
        }
        let now = seen(&t.dir.join(crate::journal_cmd::FILE));
        if let Some((range, bytes)) = old
            && let Some((was, blocked)) = decode_journal(bytes)
            && Some(was) == now
        {
            t.blocked = Some(blocked);
            return Part::Old(range);
        }
        let blocked = crate::go::blocked_of(t);
        let part = match now {
            Some(Seen::File(s)) if !self.settled(&s) => Part::None,
            Some(s) => Part::New(encode_journal(s, &blocked)),
            None => Part::None,
        };
        t.blocked = Some(blocked);
        part
    }
}

/// The ticket an entry holds, when its three files stat as they did: the
/// manifest (`stamp`, already taken) and both atoms, at the paths the
/// manifest names. Exactly what [`Store::load`] builds from the same files.
fn hit(root: &Utf8Path, dir: &Utf8Path, stamp: Stamp, bytes: &[u8]) -> Option<Ticket> {
    let mut r = Reader::new(bytes);
    if r.stamp()? != stamp {
        return None;
    }
    let intent_stamp = r.stamp()?;
    let checklist_stamp = r.stamp()?;
    let manifest = r.manifest()?;
    let revision = r.str()?.to_owned();
    let intent = r.str()?.to_owned();
    let checklist_text = r.str()?.to_owned();
    if r.pos != bytes.len() {
        return None;
    }
    let intent_path = dir.join(manifest.atom_path("intent").unwrap_or_default());
    let checklist_path = dir.join(manifest.atom_path(CHECKLIST_ROLE).unwrap_or_default());
    if seen(&intent_path) != Some(Seen::File(intent_stamp))
        || seen(&checklist_path) != Some(Seen::File(checklist_stamp))
    {
        return None;
    }
    let checklist = ticket::parse(&checklist_text);
    Some(Ticket {
        // Derived from this reader's store; never trusted from cache bytes.
        repository_root: root.to_owned(),
        dir: dir.to_owned(),
        manifest,
        intent_path,
        checklist_path,
        intent,
        checklist_text,
        checklist,
        revision,
        blocked: None,
        format: false,
        done_whole: false,
        depends_on: Vec::new(),
    })
}

fn encode_ticket(stamps: &[Stamp], t: &Ticket) -> Vec<u8> {
    let mut w = Writer::default();
    for s in stamps {
        w.stamp(s);
    }
    w.manifest(&t.manifest);
    w.str(&t.revision);
    w.str(&t.intent);
    w.str(&t.checklist_text);
    w.buf
}

fn encode_journal(seen: Seen, blocked: &[BlockedNode]) -> Vec<u8> {
    let mut w = Writer::default();
    match seen {
        Seen::Absent => w.u8(0),
        Seen::File(s) => {
            w.u8(1);
            w.stamp(&s);
        }
    }
    w.u32(u32::try_from(blocked.len()).unwrap_or(u32::MAX));
    for b in blocked {
        // Every field, by name: a field added to the struct fails to
        // compile here until the cache carries it.
        let BlockedNode {
            node,
            reason,
            attempts,
            at,
            command,
        } = b;
        w.str(node);
        w.str(reason);
        w.u32(*attempts);
        w.str(at);
        w.str(command);
    }
    w.buf
}

fn decode_journal(bytes: &[u8]) -> Option<(Seen, Vec<BlockedNode>)> {
    let mut r = Reader::new(bytes);
    let seen = match r.u8()? {
        0 => Seen::Absent,
        1 => Seen::File(r.stamp()?),
        _ => return None,
    };
    let n = r.u32()?;
    let mut out = Vec::new();
    for _ in 0..n {
        out.push(BlockedNode {
            node: r.str()?.to_owned(),
            reason: r.str()?.to_owned(),
            attempts: r.u32()?,
            at: r.str()?.to_owned(),
            command: r.str()?.to_owned(),
        });
    }
    (r.pos == bytes.len()).then_some((seen, out))
}

#[derive(Default)]
struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn i64(&mut self, v: i64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn bytes(&mut self, b: &[u8]) {
        self.u32(u32::try_from(b.len()).unwrap_or(u32::MAX));
        self.buf.extend_from_slice(b);
    }
    fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }
    fn opt(&mut self, s: Option<&str>) {
        match s {
            None => self.u8(0),
            Some(s) => {
                self.u8(1);
                self.str(s);
            }
        }
    }
    fn stamp(&mut self, s: &Stamp) {
        self.u64(s.len);
        self.i64(s.mtime);
        self.i64(s.mtime_ns);
        self.i64(s.ctime);
        self.i64(s.ctime_ns);
        self.u64(s.ino);
        self.u64(s.dev);
        self.u32(s.mode);
    }
    fn manifest(&mut self, m: &TicketManifest) {
        // Every field, by name: a field added to the manifest fails to
        // compile here until the cache carries it.
        let TicketManifest {
            schema,
            id,
            uuid,
            title,
            profile,
            priority,
            created_at,
            created_by,
            promoted_to,
            kind,
            labels,
            part_of,
            issue,
            issue_url,
            imported_from,
            due,
            summary,
            atoms,
        } = m;
        for s in [schema, id, uuid, title, profile] {
            self.str(s);
        }
        self.u8(*priority);
        self.str(created_at);
        self.str(created_by);
        self.opt(promoted_to.as_deref());
        self.opt(kind.as_deref());
        self.u32(u32::try_from(labels.len()).unwrap_or(u32::MAX));
        for l in labels {
            self.str(l);
        }
        self.opt(part_of.as_deref());
        match issue {
            None => self.u8(0),
            Some(n) => {
                self.u8(1);
                self.u64(*n);
            }
        }
        self.opt(issue_url.as_deref());
        self.opt(imported_from.as_deref());
        self.opt(due.as_deref());
        self.opt(summary.as_deref());
        self.u32(u32::try_from(atoms.len()).unwrap_or(u32::MAX));
        for a in atoms {
            let TicketAtom {
                ordinal,
                role,
                path,
            } = a;
            self.u32(*ordinal);
            self.str(role);
            self.str(path);
        }
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let out = self.bytes.get(self.pos..end)?;
        self.pos = end;
        Some(out)
    }
    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn i64(&mut self) -> Option<i64> {
        Some(i64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    /// A length-prefixed run of bytes, as where it lies.
    fn range(&mut self) -> Option<Range<usize>> {
        let n = self.u32()? as usize;
        let start = self.pos;
        self.take(n)?;
        Some(start..self.pos)
    }
    fn str(&mut self) -> Option<&'a str> {
        let n = self.u32()? as usize;
        std::str::from_utf8(self.take(n)?).ok()
    }
    fn opt(&mut self) -> Option<Option<String>> {
        match self.u8()? {
            0 => Some(None),
            1 => Some(Some(self.str()?.to_owned())),
            _ => None,
        }
    }
    fn stamp(&mut self) -> Option<Stamp> {
        Some(Stamp {
            len: self.u64()?,
            mtime: self.i64()?,
            mtime_ns: self.i64()?,
            ctime: self.i64()?,
            ctime_ns: self.i64()?,
            ino: self.u64()?,
            dev: self.u64()?,
            mode: self.u32()?,
        })
    }
    fn manifest(&mut self) -> Option<TicketManifest> {
        let schema = self.str()?.to_owned();
        let id = self.str()?.to_owned();
        let uuid = self.str()?.to_owned();
        let title = self.str()?.to_owned();
        let profile = self.str()?.to_owned();
        let priority = self.u8()?;
        let created_at = self.str()?.to_owned();
        let created_by = self.str()?.to_owned();
        let promoted_to = self.opt()?;
        let kind = self.opt()?;
        let n = self.u32()?;
        let mut labels = Vec::new();
        for _ in 0..n {
            labels.push(self.str()?.to_owned());
        }
        let part_of = self.opt()?;
        let issue = match self.u8()? {
            0 => None,
            1 => Some(self.u64()?),
            _ => return None,
        };
        let issue_url = self.opt()?;
        let imported_from = self.opt()?;
        let due = self.opt()?;
        let summary = self.opt()?;
        let n = self.u32()?;
        let mut atoms = Vec::new();
        for _ in 0..n {
            atoms.push(TicketAtom {
                ordinal: self.u32()?,
                role: self.str()?.to_owned(),
                path: self.str()?.to_owned(),
            });
        }
        Some(TicketManifest {
            schema,
            id,
            uuid,
            title,
            profile,
            priority,
            created_at,
            created_by,
            promoted_to,
            kind,
            labels,
            part_of,
            issue,
            issue_url,
            imported_from,
            due,
            summary,
            atoms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> TicketManifest {
        TicketManifest {
            schema: "s".into(),
            id: "t-1".into(),
            uuid: "u".into(),
            title: "a title — with ünïcode".into(),
            profile: "ticket".into(),
            priority: 3,
            created_at: "2026-10-11T00:00:00Z".into(),
            created_by: "me".into(),
            promoted_to: Some("OW-WAR-0001".into()),
            kind: Some("bug".into()),
            labels: vec!["a".into(), "b".into()],
            part_of: Some("t-0".into()),
            issue: Some(42),
            issue_url: Some("https://example.invalid/42".into()),
            imported_from: Some("beads:bd-1".into()),
            due: Some("2026-12-01".into()),
            summary: Some("one line".into()),
            atoms: vec![TicketAtom {
                ordinal: 10,
                role: "intent".into(),
                path: "atoms/10-intent.md".into(),
            }],
        }
    }

    #[test]
    fn a_manifest_round_trips_every_field() {
        for m in [
            manifest(),
            TicketManifest {
                promoted_to: None,
                kind: None,
                labels: Vec::new(),
                part_of: None,
                issue: None,
                issue_url: None,
                imported_from: None,
                due: None,
                atoms: Vec::new(),
                ..manifest()
            },
        ] {
            let mut w = Writer::default();
            w.manifest(&m);
            let mut r = Reader::new(&w.buf);
            assert_eq!(r.manifest(), Some(m));
            assert_eq!(r.pos, w.buf.len());
        }
    }

    #[test]
    fn a_journal_part_round_trips_and_refuses_a_bad_tag() {
        let b = vec![BlockedNode {
            node: "t-1/i-2".into(),
            reason: "r".into(),
            attempts: 3,
            at: "now".into(),
            command: "war evidence go --retry t-1/i-2".into(),
        }];
        let bytes = encode_journal(Seen::Absent, &b);
        let (s, got) = decode_journal(&bytes).unwrap();
        assert_eq!(s, Seen::Absent);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].command, b[0].command);
        let mut bad = bytes.clone();
        bad[0] = 7;
        assert!(decode_journal(&bad).is_none());
        // Truncated anywhere: refused, never half-read.
        for n in 0..bytes.len() {
            assert!(decode_journal(&bytes[..n]).is_none());
        }
    }

    #[test]
    fn hosted_and_recording_runs_never_use_it() {
        // Never `permit()` here: the flag is the process's, and the other
        // tests of this binary read this repository's own tickets, whose
        // cache would then be written under its real git directory.
        // OPENWARRANT_NO_CACHE in the test's own environment would turn it
        // off everywhere, which proves nothing here.
        if disabled_by_env() || !cfg!(unix) {
            return;
        }
        assert!(may_use_if(true));
        assert!(!may_use_if(false));
        let tree = std::rc::Rc::new(crate::vfs::Tree::new(
            Utf8PathBuf::from("/basis"),
            std::collections::BTreeMap::new(),
            std::collections::BTreeSet::new(),
        ));
        assert!(!crate::vfs::hosted(tree, || may_use_if(true)));
        let recorder = std::rc::Rc::new(std::cell::RefCell::new(crate::vfs::Recorder::default()));
        assert!(!crate::vfs::recording(recorder, || may_use_if(true)));
        assert!(may_use_if(true));
    }

    #[test]
    fn a_frame_refuses_every_wrong_byte() {
        let cache = Cache {
            dir: Utf8PathBuf::from("/nonexistent"),
            prefix: "t".into(),
            key: [7; 32],
            settled_before: (0, 0),
            journals: false,
        };
        let framed = cache.frame(b"payload");
        assert_eq!(cache.unframe(&framed), Some(framed.len() - 7));
        // Any one byte changed, anywhere: the magic, the key, the length,
        // the checksum or the payload.
        for i in 0..framed.len() {
            let mut bad = framed.clone();
            bad[i] ^= 0x40;
            assert_eq!(cache.unframe(&bad), None, "byte {i}");
        }
        // Short, long, or framed under another key.
        assert_eq!(cache.unframe(&framed[..framed.len() - 1]), None);
        let mut long = framed.clone();
        long.push(0);
        assert_eq!(cache.unframe(&long), None);
        let other = Cache {
            key: [8; 32],
            ..cache
        };
        assert_eq!(other.unframe(&framed), None);
    }

    #[test]
    fn a_shard_out_of_order_is_refused() {
        let cache = Cache {
            dir: Utf8PathBuf::from("/nonexistent"),
            prefix: "t".into(),
            key: [7; 32],
            settled_before: (0, 0),
            journals: false,
        };
        let shard = |names: &[&str]| {
            let mut w = Writer::default();
            w.u32(u32::try_from(names.len()).unwrap());
            for n in names {
                w.str(n);
                w.bytes(b"t");
                w.bytes(b"");
            }
            cache.frame(&w.buf)
        };
        let ok = cache.parse_shard(shard(&["t-1", "t-2"])).unwrap();
        assert!(ok.get("t-2").is_some() && ok.get("t-3").is_none());
        assert!(cache.parse_shard(shard(&["t-2", "t-1"])).is_none());
        assert!(cache.parse_shard(shard(&["t-1", "t-1"])).is_none());
    }

    #[test]
    fn shards_are_stable_and_in_range() {
        assert_eq!(shard_of("t-0000"), shard_of("t-0000"));
        assert!((0..1000).all(|i| shard_of(&format!("t-{i:04x}")) < SHARDS));
    }
}
