// SPDX-License-Identifier: Apache-2.0
//! Named repositories (t-3c16c W6).
//!
//! `openwarrant.toml` names the repositories this one builds against:
//!
//! ```toml
//! [repos.hpr]
//! path = "vendor/hpr"                  # a submodule or a sibling checkout
//! url = "https://example.com/hpr.git"
//! pin = "<commit>"                     # only for a sibling
//! ```
//!
//! A pin says exactly what this side has, so a read through it is
//! deterministic: the answer comes from the other repository's git objects at
//! that commit, never from its working tree. The commit is, in order, a `pin`
//! field; the gitlink committed at `path`; or the `#<commit>` a committed
//! `Cargo.lock` holds for `url`. No read here touches the network: what the
//! other side has done reaches this one when it is fetched.
//!
//! A pinned commit opens as [`crate::vfs`] hosted tree, so every reader
//! (`war context`, `war view prime`, the item list) answers as it would in
//! that repository, and none of them can see a live file by accident.
//!
//! # What cannot be read
//!
//! A repository not declared, not on this machine, or without the commit
//! reads UNKNOWN (Law 15), never a guess and never a failure: nothing here
//! fails a command that was not asking about the other repository. A hosted
//! or recording run has no other repository: everything reads UNKNOWN, and no
//! process starts.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::process::{Command, Stdio};
use std::rc::Rc;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::repo::Repository;
use crate::vfs::Tree;

/// The rule a read that could not be made reports.
pub const UNKNOWN_RULE: &str = "repos.unknown";
/// `[repos]` does not parse.
pub const CONFIG_RULE: &str = "repos.config";

/// What one file of another repository may weigh and still be read.
const LARGEST_FILE: u64 = 4 * 1024 * 1024;
/// Where a pinned commit appears to the readers.
const VIRTUAL_ROOT: &str = "/openwarrant-repos";
/// Words before a `:` that already mean something to a reader.
const RESERVED: [&str; 7] = ["md", "war", "doc", "host", "http", "https", "file"];

/// One entry of `[repos]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Named {
    /// A submodule or sibling checkout, relative to this repository's root.
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    /// The commit, for a sibling. A submodule's gitlink needs none.
    #[serde(default)]
    pub pin: Option<String>,
}

#[derive(Deserialize)]
struct File {
    #[serde(default)]
    repos: BTreeMap<String, Named>,
}

/// Every repository `root`'s `openwarrant.toml` declares, by name. A file
/// that cannot be read declares none; a `[repos]` that does not parse is the
/// `Err`, which only `war check` treats as a finding.
pub fn declared(root: &Utf8Path) -> Result<BTreeMap<String, Named>, String> {
    let Ok(text) = crate::vfs::read_to_string(root.join(crate::init::CONFIG_FILE)) else {
        return Ok(BTreeMap::new());
    };
    toml::from_str::<File>(&text)
        .map(|f| f.repos)
        .map_err(|e| format!("the [repos] table: {}", e.message()))
}

/// What a target's first word turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target<'a> {
    /// No repository in it: a local target, for the local readers.
    Local,
    /// `name:rest`, where `name` is declared.
    Named(&'a str, &'a str),
    /// `name:rest`, where nothing declares `name`.
    Undeclared(&'a str),
}

/// The part of `target` before a `:`, when it reads as a repository's name.
fn prefix(target: &str) -> Option<(&str, &str)> {
    let (name, rest) = target.trim().split_once(':')?;
    let shaped = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    let rest_shaped =
        !rest.is_empty() && !rest.starts_with("//") && !rest.contains(char::is_whitespace);
    (shaped && rest_shaped && !RESERVED.contains(&name)).then_some((name, rest))
}

/// Which repository `target` means, in the repository at `root`.
#[must_use]
pub fn classify<'a>(root: &Utf8Path, target: &'a str) -> Target<'a> {
    let Some((name, rest)) = prefix(target) else {
        return Target::Local;
    };
    match declared(root) {
        Ok(all) if all.contains_key(name) => Target::Named(name, rest),
        _ => Target::Undeclared(name),
    }
}

/// `git --git-dir=<dir> <args>`: its stdout, or `None` when it failed. Git's
/// own environment is dropped: a hook runs `war` with `GIT_DIR` set, and it
/// names the wrong repository here.
fn git(dir: &Utf8Path, args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new("git")
        .arg(format!("--git-dir={dir}"))
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status.success().then_some(out.stdout)
}

fn git_line(dir: &Utf8Path, args: &[&str]) -> Option<String> {
    let bytes = git(dir, args)?;
    let line = String::from_utf8_lossy(&bytes).trim().to_owned();
    (!line.is_empty()).then_some(line)
}

/// The git directory of the repository whose worktree holds `root`.
fn own_git_dir(root: &Utf8Path) -> Option<Utf8PathBuf> {
    crate::ticket::claim::git_layout(root).map(|l| l.common_dir)
}

/// The submodule name `.gitmodules` gives `path` (its path, when it says
/// nothing), which is what its git directory is called.
fn submodule_name(root: &Utf8Path, path: &str) -> String {
    let Ok(text) = crate::vfs::read_to_string(root.join(".gitmodules")) else {
        return path.to_owned();
    };
    let mut name = None;
    for line in text.lines().map(str::trim) {
        if let Some(n) = line
            .strip_prefix("[submodule \"")
            .and_then(|l| l.strip_suffix("\"]"))
        {
            name = Some(n);
        } else if let Some((key, value)) = line.split_once('=')
            && key.trim() == "path"
            && value.trim() == path
            && let Some(n) = name
        {
            return n.to_owned();
        }
    }
    path.to_owned()
}

/// Where the objects of `named` are: its own checkout, or the superproject's
/// `modules/` when it was never checked out here (a worktree has no copy).
/// A directory that only sits inside this repository is not one.
fn objects_of(root: &Utf8Path, named: &Named) -> Option<Utf8PathBuf> {
    let path = named.path.as_deref()?;
    let checkout = |at: &Utf8Path| {
        let dir = at.join(path);
        if dir.join(".git").exists() {
            crate::ticket::claim::git_layout(&dir).map(|l| l.common_dir)
        } else {
            None
        }
    };
    let common = own_git_dir(root);
    checkout(root)
        .or_else(|| {
            // A worktree: the checkout that owns the clone.
            let main = common.as_deref()?.parent()?;
            checkout(main)
        })
        .or_else(|| {
            let dir = common?.join("modules").join(submodule_name(root, path));
            dir.join("HEAD").exists().then_some(dir)
        })
}

/// How a pin was read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pin {
    pub commit: String,
    /// `pin`, `submodule` or `Cargo.lock`.
    pub from: String,
}

fn cargo_lock_pin(text: &str, url: &str) -> Option<String> {
    let wanted = url.trim_end_matches('/').trim_end_matches(".git");
    text.lines().find_map(|l| {
        let source = l
            .trim()
            .strip_prefix("source = \"git+")?
            .strip_suffix('"')?;
        let (at, commit) = source.rsplit_once('#')?;
        let at = at.split('?').next().unwrap_or(at);
        (at.trim_end_matches('/').trim_end_matches(".git") == wanted).then(|| commit.to_owned())
    })
}

/// The commit this side holds of `named`: never its working tree or an
/// uncommitted index, so one commit of this repository reads one way.
fn pin_of(root: &Utf8Path, name: &str, named: &Named) -> Result<Pin, String> {
    if let Some(commit) = named
        .pin
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        return Ok(Pin {
            commit: commit.to_owned(),
            from: "pin".to_owned(),
        });
    }
    let own = own_git_dir(root).ok_or_else(|| format!("{root} is no git repository"))?;
    // HEAD of this checkout, not of the clone: a worktree has its own gitlink.
    let checked = checked_out_head(root)
        .ok_or_else(|| format!("{root} has no commit, so {name} has no pin"))?;
    if let Some(path) = named.path.as_deref() {
        let rel = path.trim_matches('/');
        if let Some(listed) = git(&own, &["ls-tree", &checked, "--", rel]) {
            let listed = String::from_utf8_lossy(&listed);
            if let Some(commit) = listed.lines().find_map(|l| {
                let mut parts = l.split_whitespace();
                (parts.next()? == "160000" && parts.next()? == "commit")
                    .then(|| parts.next().map(str::to_owned))
                    .flatten()
            }) {
                return Ok(Pin {
                    commit,
                    from: "submodule".to_owned(),
                });
            }
        }
    }
    if let Some(url) = named.url.as_deref()
        && let Some(lock) = git(
            &own,
            &["cat-file", "blob", &format!("{checked}:Cargo.lock")],
        )
        && let Some(commit) = cargo_lock_pin(&String::from_utf8_lossy(&lock), url)
    {
        return Ok(Pin {
            commit,
            from: "Cargo.lock".to_owned(),
        });
    }
    Err(format!(
        "{name} has no pin: no `pin` field, no submodule committed at {}, nothing for its url in \
         Cargo.lock",
        named.path.as_deref().unwrap_or("(no path)")
    ))
}

/// The commit `root` has checked out: a worktree's HEAD is not the clone's.
fn checked_out_head(root: &Utf8Path) -> Option<String> {
    let out = Command::new("git")
        .args([
            "-C",
            root.as_str(),
            "rev-parse",
            "--verify",
            "-q",
            "HEAD^{commit}",
        ])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// A repository found on this machine, at its pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    pub name: String,
    pub git_dir: Utf8PathBuf,
    pub pin: Pin,
}

/// Find `name`: declared, on this machine, holding its pin. `Err` says why
/// not, in words that follow `UNKNOWN: `.
pub fn locate(root: &Utf8Path, name: &str) -> Result<Located, String> {
    if !crate::vfs::is_disk() {
        return Err(format!(
            "{name}: a hosted or recording run reads no other repository"
        ));
    }
    let all = declared(root)?;
    let named = all
        .get(name)
        .ok_or_else(|| format!("{name} is not declared in [repos]"))?;
    let pin = pin_of(root, name, named)?;
    let git_dir = objects_of(root, named).ok_or_else(|| {
        format!(
            "{name} is not on this machine (no checkout at {}); `git submodule update --init` \
             brings it",
            named.path.as_deref().unwrap_or("(no path)")
        )
    })?;
    let commit = git_line(
        &git_dir,
        &[
            "rev-parse",
            "--verify",
            "-q",
            &format!("{}^{{commit}}", pin.commit),
        ],
    )
    .ok_or_else(|| {
        format!(
            "{name} does not hold its pin {}; fetch it there",
            short(&pin.commit)
        )
    })?;
    Ok(Located {
        name: name.to_owned(),
        git_dir,
        pin: Pin {
            commit,
            from: pin.from,
        },
    })
}

/// What a person reads of a commit.
#[must_use]
pub fn short(commit: &str) -> &str {
    &commit[..commit.len().min(12)]
}

/// The newest commit of the other repository's default branch this machine
/// has: the further ahead of its local branch and what was fetched from
/// `origin`. Reads refs only; nothing here fetches.
#[must_use]
pub fn upstream_tip(git_dir: &Utf8Path) -> Option<String> {
    let line = |name: &str| git_line(git_dir, &["rev-parse", "--verify", "-q", name]);
    let branch = git_line(git_dir, &["symbolic-ref", "-q", "refs/remotes/origin/HEAD"])
        .and_then(|r| r.strip_prefix("refs/remotes/origin/").map(str::to_owned));
    branch
        .into_iter()
        .chain(["main".to_owned(), "master".to_owned()])
        .find_map(|b| {
            let local = line(&format!("refs/heads/{b}^{{commit}}"));
            let fetched = line(&format!("refs/remotes/origin/{b}^{{commit}}"));
            match (local, fetched) {
                (Some(l), Some(f)) if l != f => {
                    let behind = git(git_dir, &["merge-base", "--is-ancestor", &l, &f]).is_some();
                    Some(if behind { f } else { l })
                }
                (Some(l), _) => Some(l),
                (None, f) => f,
            }
        })
}

/// Whether `commit` is held by `tip`: `tip` itself or something after it.
#[must_use]
pub fn holds(git_dir: &Utf8Path, commit: &str, tip: &str) -> bool {
    commit == tip || git(git_dir, &["merge-base", "--is-ancestor", tip, commit]).is_some()
}

/// The paths of a commit a reader can reach: its top level, `docs/`, and
/// whatever its `openwarrant.toml` names. A game's assets stay in git.
fn prefixes(config: Option<&str>) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = ["docs/", "profiles/", "spec/", ".openwarrant/"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    let Some(value) = config.and_then(|c| toml::from_str::<toml::Value>(c).ok()) else {
        return out;
    };
    let mut add = |v: &toml::Value| {
        if let Some(s) = v.as_str() {
            out.insert(format!("{}/", s.trim_matches('/')));
        }
    };
    if let Some(paths) = value.get("paths").and_then(toml::Value::as_table) {
        paths.values().for_each(&mut add);
    }
    if let Some(dir) = value.get("tickets").and_then(|t| t.get("dir")) {
        add(dir);
    }
    out
}

/// Every wanted file of `commit`, read in one `git cat-file --batch`.
fn read_tree(
    git_dir: &Utf8Path,
    commit: &str,
    also: Option<&str>,
) -> Option<BTreeMap<String, Vec<u8>>> {
    let listed = git(git_dir, &["ls-tree", "-r", "-z", "--long", commit])?;
    let config = git(
        git_dir,
        &[
            "cat-file",
            "blob",
            &format!("{commit}:{}", crate::init::CONFIG_FILE),
        ],
    );
    let config = config.as_deref().map(String::from_utf8_lossy);
    let wanted = prefixes(config.as_deref());
    let mut names: Vec<String> = Vec::new();
    let mut shas = String::new();
    for entry in listed.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        let entry = String::from_utf8_lossy(entry);
        let Some((meta, path)) = entry.split_once('\t') else {
            continue;
        };
        let mut parts = meta.split_whitespace();
        let (_, kind, sha, size) = (parts.next(), parts.next(), parts.next(), parts.next());
        let (Some("blob"), Some(sha), Some(size)) = (kind, sha, size) else {
            continue;
        };
        let reachable = !path.contains('/')
            || wanted.iter().any(|p| path.starts_with(p.as_str()))
            || also == Some(path);
        if reachable && size.parse::<u64>().is_ok_and(|s| s <= LARGEST_FILE) {
            names.push(path.to_owned());
            shas.push_str(sha);
            shas.push('\n');
        }
    }
    let mut child = Command::new("git")
        .arg(format!("--git-dir={git_dir}"))
        .args(["cat-file", "--batch"])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdin = child.stdin.take()?;
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(shas.as_bytes());
    });
    let out = child.wait_with_output().ok()?;
    let _ = writer.join();
    let mut files = BTreeMap::new();
    let mut at = 0;
    for name in names {
        let line_end = at + out.stdout[at..].iter().position(|b| *b == b'\n')?;
        let header = String::from_utf8_lossy(&out.stdout[at..line_end]);
        let size: usize = header.split_whitespace().nth(2)?.parse().ok()?;
        let start = line_end + 1;
        files.insert(name, out.stdout.get(start..start + size)?.to_vec());
        at = start + size + 1;
    }
    Some(files)
}

thread_local! {
    /// The trees read by this command, by repository and commit. A tree is
    /// per thread, as a hosted run is.
    static TREES: RefCell<BTreeMap<(String, String), Rc<Tree>>> = const { RefCell::new(BTreeMap::new()) };
}

fn tree_at(git_dir: &Utf8Path, name: &str, commit: &str, also: Option<&str>) -> Option<Rc<Tree>> {
    let key = (git_dir.to_string(), commit.to_owned());
    if also.is_none()
        && let Some(t) = TREES.with(|c| c.borrow().get(&key).cloned())
    {
        return Some(t);
    }
    let files = read_tree(git_dir, commit, also)?;
    let root = Utf8PathBuf::from(format!("{VIRTUAL_ROOT}/{name}"));
    let tree = Rc::new(Tree::new(root, files, BTreeSet::new()));
    if also.is_none() {
        TREES.with(|c| c.borrow_mut().insert(key, Rc::clone(&tree)));
    }
    Some(tree)
}

/// `f`, over the repository at `commit` of `located`. `Err` when it holds no
/// such repository.
pub fn read_at<R>(
    located: &Located,
    commit: &str,
    also: Option<&str>,
    f: impl FnOnce(&Repository) -> R,
) -> Result<R, String> {
    let name = &located.name;
    let tree = tree_at(&located.git_dir, name, commit, also)
        .ok_or_else(|| format!("{name}: commit {} could not be read", short(commit)))?;
    let root = tree.root.clone();
    crate::vfs::hosted(tree, || match Repository::open(root) {
        Ok(repo) => Ok(f(&repo)),
        Err(e) => Err(format!("{name} at {}: {e}", short(commit))),
    })
}

/// `f`, over `name` at its pin, with the commit it read.
pub fn through_pin<R>(
    root: &Utf8Path,
    name: &str,
    also: Option<&str>,
    f: impl FnOnce(&Repository) -> R,
) -> Result<(Located, R), String> {
    let located = locate(root, name)?;
    let commit = located.pin.commit.clone();
    let value = read_at(&located, &commit, also, f)?;
    Ok((located, value))
}

thread_local! {
    /// What the envelope of a read through a pin says it read.
    static READ: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// What a read through a pin came from, in one line.
#[must_use]
pub fn read_line(l: &Located) -> String {
    format!("{} @ {} ({})", l.name, short(&l.pin.commit), l.pin.from)
}

/// Have the next envelope say `line` in its `notes`.
pub fn note_read(line: String) {
    READ.with(|r| *r.borrow_mut() = Some(line));
}

/// What [`note_read`] left.
#[must_use]
pub fn take_read() -> Option<String> {
    READ.with(|r| r.borrow_mut().take())
}

/// The one line `war status` says of every declared repository; `None` when
/// there are none. What could not be read says UNKNOWN.
#[must_use]
pub fn status_line(root: &Utf8Path) -> Option<String> {
    let all = declared(root).ok()?;
    if all.is_empty() {
        return None;
    }
    let parts: Vec<String> = all
        .keys()
        .map(|name| match locate(root, name) {
            Ok(l) => read_line(&l),
            Err(_) => format!("{name} UNKNOWN"),
        })
        .collect();
    Some(format!("Repos: {}", parts.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_needs_a_rest_and_may_not_be_a_word_a_reader_owns() {
        assert_eq!(prefix("hpr:t-abc"), Some(("hpr", "t-abc")));
        assert_eq!(prefix("hpr:docs/a.md#b"), Some(("hpr", "docs/a.md#b")));
        for local in [
            "md:CLAUDE.md",
            "war://x",
            "doc:a",
            "host:AGENTS.md",
            "t-abc",
            "a b:c d",
            "https://x",
            "hpr:",
            ":x",
        ] {
            assert_eq!(prefix(local), None, "{local}");
        }
    }

    #[test]
    fn a_lock_entry_is_found_by_url_and_nothing_else_is() {
        let lock = "[[package]]\nname = \"a\"\nsource = \"git+https://x.example/hpr.git?rev=abc#0123\"\n\
                    [[package]]\nname = \"b\"\nsource = \"registry+https://crates.io\"\n";
        assert_eq!(
            cargo_lock_pin(lock, "https://x.example/hpr").as_deref(),
            Some("0123")
        );
        assert_eq!(
            cargo_lock_pin(lock, "https://x.example/hpr.git").as_deref(),
            Some("0123")
        );
        assert_eq!(cargo_lock_pin(lock, "https://x.example/other"), None);
    }

    #[test]
    fn a_hosted_run_reads_no_other_repository() {
        let tree = Rc::new(Tree::new(
            Utf8PathBuf::from("/v"),
            BTreeMap::new(),
            BTreeSet::new(),
        ));
        let got = crate::vfs::hosted(tree, || locate(Utf8Path::new("/v"), "hpr"));
        assert!(got.unwrap_err().contains("hosted"));
    }

    #[test]
    fn a_table_that_does_not_parse_is_said_and_no_table_declares_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        assert!(declared(root).unwrap().is_empty());
        std::fs::write(root.join("openwarrant.toml"), "[repos.hpr]\nnope = 1\n").unwrap();
        assert!(declared(root).is_err());
        std::fs::write(
            root.join("openwarrant.toml"),
            "[repos.hpr]\npath = \"v/h\"\n",
        )
        .unwrap();
        assert_eq!(declared(root).unwrap()["hpr"].path.as_deref(), Some("v/h"));
    }
}
