// SPDX-License-Identifier: Apache-2.0
//! Where the corpus readers read from (OW-WAR-0148 M8; SAS §82.2).
//!
//! The readers that build `oh.war/model/v1` — `Repository`, the corpus, the
//! status, the assessment, tickets, record atoms — read through these
//! functions instead of `std::fs`, so one implementation serves two sources:
//!
//! - **Disk** (the default, and the only mode outside `war host`): every
//!   function is the `std::fs` call it replaces, byte for byte.
//! - **Hosted** ([`hosted`]): the files are a Workspace Basis held in memory
//!   (an `oh.war/liminal-v1` request). A path under the virtual root is
//!   answered from the basis; any other path does not exist. Nothing touches
//!   the disk, and the two observations the readers would otherwise make
//!   outside the bytes — an `ssh-keygen -Y verify` verdict and a `git` read
//!   of the tree's history — are answered from the request's `observations`
//!   or, absent one, fail closed exactly as a missing tool does.
//! - **Recording** ([`recording`]): disk mode, remembering every file read,
//!   listed or found, and every observation made, so `war host --export` can
//!   write the request that reproduces the run.
//!
//! The mode is per thread and scoped to one closure; the readers on the
//! model's path are single-threaded.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use camino::{Utf8Path, Utf8PathBuf};

// The writers in modules that read through here: unchanged, and never
// reached by a hosted run (`war host` writes nothing).
pub use std::fs::{
    File, OpenOptions, copy, create_dir, create_dir_all, hard_link, remove_dir_all, remove_file,
    rename, write,
};

/// The outcome of one `ssh-keygen -Y verify`, as an observation.
pub type SshVerdict = Result<(), String>;
/// The outcome of one `git` read, as an observation.
pub type GitOutcome = Result<Vec<u8>, String>;

/// A Workspace Basis held in memory.
#[derive(Debug, Default)]
pub struct Tree {
    pub root: Utf8PathBuf,
    /// Root-relative `/`-separated path → exact bytes.
    pub files: BTreeMap<String, Vec<u8>>,
    /// Members known by digest only: listed and found, never readable.
    pub withheld: BTreeSet<String>,
    dirs: BTreeSet<String>,
    /// Signature verdicts by the digest of every byte they depend on.
    pub ssh: BTreeMap<String, SshVerdict>,
    /// `git` outcomes by their arguments (the repository root left out).
    pub git: BTreeMap<Vec<String>, GitOutcome>,
    consulted: RefCell<Consulted>,
}

/// Which observations a hosted run asked for.
#[derive(Debug, Default, Clone)]
pub struct Consulted {
    /// Withheld members a reader tried to read.
    pub withheld_read: BTreeSet<String>,
    pub ssh_supplied: BTreeSet<String>,
    pub ssh_missing: BTreeSet<String>,
    pub git_supplied: BTreeSet<Vec<String>>,
    pub git_missing: BTreeSet<Vec<String>>,
}

impl Tree {
    #[must_use]
    pub fn new(
        root: Utf8PathBuf,
        files: BTreeMap<String, Vec<u8>>,
        withheld: BTreeSet<String>,
    ) -> Self {
        let mut dirs = BTreeSet::from([String::new()]);
        for p in files.keys().chain(withheld.iter()) {
            let mut cur = p.as_str();
            while let Some((parent, _)) = cur.rsplit_once('/') {
                dirs.insert(parent.to_owned());
                cur = parent;
            }
        }
        Self {
            root,
            files,
            withheld,
            dirs,
            ..Self::default()
        }
    }

    /// What the run consulted.
    #[must_use]
    pub fn consulted(&self) -> Consulted {
        self.consulted.borrow().clone()
    }

    /// `p` relative to the root, lexically normalized; `None` outside it.
    fn rel(&self, p: &Path) -> Option<String> {
        let rest = p.strip_prefix(self.root.as_std_path()).ok()?;
        let mut parts: Vec<String> = Vec::new();
        for c in rest.components() {
            match c {
                Component::Normal(s) => parts.push(s.to_str()?.to_owned()),
                Component::CurDir => {}
                Component::ParentDir => {
                    parts.pop()?;
                }
                _ => return None,
            }
        }
        Some(parts.join("/"))
    }
}

/// What a recording run saw.
#[derive(Debug, Default)]
pub struct Recorder {
    pub root: Utf8PathBuf,
    /// Root-relative paths of every file read.
    pub files: BTreeSet<String>,
    /// Root-relative paths of every file listed or found and not read.
    pub seen: BTreeSet<String>,
    pub ssh: BTreeMap<String, SshVerdict>,
    pub git: BTreeMap<Vec<String>, GitOutcome>,
}

enum Mode {
    Hosted(Rc<Tree>),
    Recording(Rc<RefCell<Recorder>>),
}

thread_local! {
    static MODE: RefCell<Option<Mode>> = const { RefCell::new(None) };
}

struct Restore(Option<Mode>);
impl Drop for Restore {
    fn drop(&mut self) {
        let prev = self.0.take();
        MODE.with(|m| *m.borrow_mut() = prev);
    }
}

fn scoped<R>(mode: Mode, f: impl FnOnce() -> R) -> R {
    let prev = MODE.with(|m| m.borrow_mut().replace(mode));
    let _restore = Restore(prev);
    f()
}

/// Run `f` with every read answered from `tree`.
pub fn hosted<R>(tree: Rc<Tree>, f: impl FnOnce() -> R) -> R {
    scoped(Mode::Hosted(tree), f)
}

/// Run `f` against the disk, recording what it reads under `recorder.root`.
pub fn recording<R>(recorder: Rc<RefCell<Recorder>>, f: impl FnOnce() -> R) -> R {
    scoped(Mode::Recording(recorder), f)
}

fn tree() -> Option<Rc<Tree>> {
    MODE.with(|m| match &*m.borrow() {
        Some(Mode::Hosted(t)) => Some(Rc::clone(t)),
        _ => None,
    })
}

fn recorder() -> Option<Rc<RefCell<Recorder>>> {
    MODE.with(|m| match &*m.borrow() {
        Some(Mode::Recording(r)) => Some(Rc::clone(r)),
        _ => None,
    })
}

/// Whether this thread is in a hosted run.
#[must_use]
pub fn is_hosted() -> bool {
    tree().is_some()
}

fn note(p: &Path) {
    note_as(p, true);
}

fn note_seen(p: &Path) {
    note_as(p, false);
}

fn note_as(p: &Path, read: bool) {
    if let Some(r) = recorder() {
        let mut r = r.borrow_mut();
        if let Ok(rest) = p.strip_prefix(r.root.as_std_path())
            && let Some(s) = rest.to_str()
            && !s.is_empty()
        {
            // Lexically, as a hosted run resolves it (`Tree::rel`): an atom
            // may name `../../adr/atoms/…`.
            let mut parts: Vec<&str> = Vec::new();
            for c in rest.components() {
                match c {
                    Component::Normal(x) => match x.to_str() {
                        Some(x) => parts.push(x),
                        None => return,
                    },
                    Component::CurDir => {}
                    Component::ParentDir => {
                        if parts.pop().is_none() {
                            return;
                        }
                    }
                    _ => return,
                }
            }
            let rel = parts.join("/");
            if read {
                r.seen.remove(&rel);
                r.files.insert(rel);
            } else if !r.files.contains(&rel) {
                r.seen.insert(rel);
            }
        }
    }
}

fn not_found() -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        "No such file or directory (os error 2)",
    )
}

/// `std::fs::read`.
pub fn read<P: AsRef<Path>>(p: P) -> io::Result<Vec<u8>> {
    let p = p.as_ref();
    if let Some(t) = tree() {
        let r = t.rel(p).ok_or_else(not_found)?;
        if t.withheld.contains(&r) {
            t.consulted.borrow_mut().withheld_read.insert(r);
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "the basis holds this member by digest only",
            ));
        }
        return t.files.get(&r).cloned().ok_or_else(not_found);
    }
    let bytes = std::fs::read(p)?;
    note(p);
    Ok(bytes)
}

/// `std::fs::read_to_string`.
pub fn read_to_string<P: AsRef<Path>>(p: P) -> io::Result<String> {
    if is_hosted() {
        return String::from_utf8(read(p)?).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "stream did not contain valid UTF-8",
            )
        });
    }
    let p = p.as_ref();
    let text = std::fs::read_to_string(p)?;
    note(p);
    Ok(text)
}

/// A file's kind, as `std::fs::FileType` answers it.
#[derive(Debug, Clone, Copy)]
pub struct FileType {
    dir: bool,
    file: bool,
}
impl FileType {
    #[must_use]
    pub fn is_dir(&self) -> bool {
        self.dir
    }
    #[must_use]
    pub fn is_file(&self) -> bool {
        self.file
    }
}

/// What `std::fs::metadata` answers that the readers use.
#[derive(Debug, Clone)]
pub struct Metadata {
    kind: FileType,
    len: u64,
}
impl Metadata {
    #[must_use]
    pub fn is_dir(&self) -> bool {
        self.kind.dir
    }
    #[must_use]
    pub fn is_file(&self) -> bool {
        self.kind.file
    }
    #[must_use]
    pub fn len(&self) -> u64 {
        self.len
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    #[must_use]
    pub fn file_type(&self) -> FileType {
        self.kind
    }
}

/// `std::fs::metadata` (follows symlinks).
pub fn metadata<P: AsRef<Path>>(p: P) -> io::Result<Metadata> {
    let p = p.as_ref();
    if let Some(t) = tree() {
        let r = t.rel(p).ok_or_else(not_found)?;
        if let Some(b) = t.files.get(&r) {
            return Ok(Metadata {
                kind: FileType {
                    dir: false,
                    file: true,
                },
                len: b.len() as u64,
            });
        }
        if t.withheld.contains(&r) {
            return Ok(Metadata {
                kind: FileType {
                    dir: false,
                    file: true,
                },
                len: 0,
            });
        }
        if t.dirs.contains(&r) {
            return Ok(Metadata {
                kind: FileType {
                    dir: true,
                    file: false,
                },
                len: 0,
            });
        }
        return Err(not_found());
    }
    let m = std::fs::metadata(p)?;
    if m.is_file() {
        note_seen(p);
    }
    Ok(Metadata {
        kind: FileType {
            dir: m.is_dir(),
            file: m.is_file(),
        },
        len: m.len(),
    })
}

/// `Path::is_file`.
pub fn is_file<P: AsRef<Path>>(p: P) -> bool {
    metadata(p).is_ok_and(|m| m.is_file())
}

/// `Path::is_dir`.
pub fn is_dir<P: AsRef<Path>>(p: P) -> bool {
    metadata(p).is_ok_and(|m| m.is_dir())
}

/// `Path::exists`.
pub fn exists<P: AsRef<Path>>(p: P) -> bool {
    metadata(p).is_ok()
}

/// One entry of [`read_dir`].
#[derive(Debug)]
pub struct DirEntry {
    path: PathBuf,
    kind: FileType,
}
impl DirEntry {
    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }
    #[must_use]
    pub fn file_name(&self) -> OsString {
        self.path
            .file_name()
            .map(OsString::from)
            .unwrap_or_default()
    }
    /// Infallible here; `Result` for `std::fs::DirEntry` parity.
    pub fn file_type(&self) -> io::Result<FileType> {
        Ok(self.kind)
    }
    pub fn metadata(&self) -> io::Result<Metadata> {
        metadata(&self.path)
    }
}

/// `std::fs::read_dir`: the entries, in the order the source gives them
/// (the disk's; a basis's sorted by name). Every caller sorts.
pub fn read_dir<P: AsRef<Path>>(p: P) -> io::Result<std::vec::IntoIter<io::Result<DirEntry>>> {
    let p = p.as_ref();
    if let Some(t) = tree() {
        let r = t.rel(p).ok_or_else(not_found)?;
        if !t.dirs.contains(&r) {
            return Err(if t.files.contains_key(&r) {
                io::Error::new(
                    io::ErrorKind::NotADirectory,
                    "Not a directory (os error 20)",
                )
            } else {
                not_found()
            });
        }
        let prefix = if r.is_empty() {
            String::new()
        } else {
            format!("{r}/")
        };
        let mut names: BTreeMap<String, bool> = BTreeMap::new();
        for f in t
            .files
            .keys()
            .chain(t.withheld.iter())
            .filter_map(|f| f.strip_prefix(&prefix))
        {
            match f.split_once('/') {
                Some((d, _)) => {
                    names.insert(d.to_owned(), true);
                }
                None => {
                    names.entry(f.to_owned()).or_insert(false);
                }
            }
        }
        return Ok(names
            .into_iter()
            .map(|(n, dir)| {
                Ok(DirEntry {
                    path: p.join(n),
                    kind: FileType { dir, file: !dir },
                })
            })
            .collect::<Vec<_>>()
            .into_iter());
    }
    let mut out = Vec::new();
    for e in std::fs::read_dir(p)? {
        out.push(e.and_then(|e| {
            let path = e.path();
            // `std::fs::FileType` does not follow symlinks; neither does this.
            let ft = e.file_type()?;
            let kind = FileType {
                dir: ft.is_dir(),
                file: ft.is_file(),
            };
            if kind.file {
                note_seen(&path);
            }
            Ok(DirEntry { path, kind })
        }));
    }
    Ok(out.into_iter())
}

/// `read_dir` over UTF-8 paths, unreadable entries skipped.
pub fn read_dir_utf8(p: &Utf8Path) -> io::Result<Vec<Utf8PathBuf>> {
    Ok(read_dir(p)?
        .filter_map(Result::ok)
        .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
        .collect())
}

/// A signature verdict for `key` in a hosted run: `None` outside one;
/// inside, the supplied verdict, or `Some(None)` when the request supplied
/// none (the caller then fails closed, as with no `ssh-keygen`).
#[must_use]
pub fn hosted_ssh(key: &str) -> Option<Option<SshVerdict>> {
    let t = tree()?;
    let v = t.ssh.get(key).cloned();
    let mut c = t.consulted.borrow_mut();
    if v.is_some() {
        c.ssh_supplied.insert(key.to_owned());
    } else {
        c.ssh_missing.insert(key.to_owned());
    }
    Some(v)
}

/// Remember a verdict a recording run observed.
pub fn record_ssh(key: &str, verdict: &SshVerdict) {
    if let Some(r) = recorder() {
        r.borrow_mut().ssh.insert(key.to_owned(), verdict.clone());
    }
}

/// A `git` outcome for `args` in a hosted run, as [`hosted_ssh`].
#[must_use]
pub fn hosted_git(args: &[&str]) -> Option<Option<GitOutcome>> {
    let t = tree()?;
    let key: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
    let v = t.git.get(&key).cloned();
    let mut c = t.consulted.borrow_mut();
    if v.is_some() {
        c.git_supplied.insert(key);
    } else {
        c.git_missing.insert(key);
    }
    Some(v)
}

/// Remember a `git` outcome a recording run observed.
pub fn record_git(args: &[&str], outcome: &GitOutcome) {
    if let Some(r) = recorder() {
        r.borrow_mut().git.insert(
            args.iter().map(|s| (*s).to_owned()).collect(),
            outcome.clone(),
        );
    }
}

/// Install a hosted tree's observations (builder for `war host`).
impl Tree {
    pub fn with_observations(
        mut self,
        ssh: BTreeMap<String, SshVerdict>,
        git: BTreeMap<Vec<String>, GitOutcome>,
    ) -> Self {
        self.ssh = ssh;
        self.git = git;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> Rc<Tree> {
        Rc::new(Tree::new(
            Utf8PathBuf::from("/basis"),
            BTreeMap::from([
                ("a/b.txt".to_owned(), b"hi".to_vec()),
                ("a/c/d.md".to_owned(), b"x".to_vec()),
                ("top".to_owned(), vec![0xff]),
            ]),
            BTreeSet::from(["a/w.txt".to_owned()]),
        ))
    }

    #[test]
    fn hosted_reads_answer_from_the_basis_only() {
        hosted(tree(), || {
            assert_eq!(read("/basis/a/b.txt").unwrap(), b"hi");
            assert_eq!(read("/basis/a/./c/../b.txt").unwrap(), b"hi");
            assert!(read("/basis/a/../../etc/passwd").is_err());
            assert!(read("/etc/hostname").is_err());
            assert!(read_to_string("/basis/top").is_err());
            assert!(is_dir("/basis/a/c") && !is_file("/basis/a/c"));
            assert!(is_file("/basis/a/c/d.md") && !exists("/basis/nope"));
            let names: Vec<_> = read_dir("/basis/a")
                .unwrap()
                .map(|e| {
                    let e = e.unwrap();
                    (e.file_name(), e.file_type().unwrap().is_dir())
                })
                .collect();
            assert_eq!(
                names,
                vec![
                    (OsString::from("b.txt"), false),
                    (OsString::from("c"), true),
                    (OsString::from("w.txt"), false)
                ]
            );
            assert!(is_file("/basis/a/w.txt"));
            assert_eq!(
                read("/basis/a/w.txt").unwrap_err().kind(),
                io::ErrorKind::PermissionDenied
            );
            assert!(read_dir("/basis/a/b.txt").is_err());
            assert_eq!(hosted_ssh("k"), Some(None));
        });
        assert!(!is_hosted());
        assert_eq!(hosted_ssh("k"), None);
    }
}
