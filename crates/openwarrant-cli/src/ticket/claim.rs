// SPDX-License-Identifier: Apache-2.0
//! Claims: who is working on an item (or a whole ticket) right now.
//!
//! A claim is a lock file under a gitignored runtime directory
//! (`.openwarrant/state/claims/` by default, SAS §59.1's disposable state),
//! one per claimed item or ticket. It is taken atomically: the claim is
//! written whole to a temporary file, then hard-linked to its name, which
//! fails if the name exists. Of two agents claiming the same item at the same
//! instant, exactly one link succeeds; the other reads the winner's file and
//! is refused by name. No reader ever sees a half-written claim, because the
//! name only ever points at a complete file.
//!
//! A claim is coordination, not authority. It names who is on an item so a
//! second agent picks something else; it grants nothing and nothing reads it
//! as a signature. The journal records every claim, release and steal, so
//! the history is in the repository even though the lock is not.
//!
//! A claim older than the TTL (`[tickets] claim_ttl_minutes`, 120 by default)
//! is stale: its holder probably stopped. `war claim --steal` takes it, and
//! the steal is journalled with whom it was taken from.
//!
//! # One lock set per clone (M11)
//!
//! Every worktree of one clone shares one claims directory, under git's
//! common directory (`git rev-parse --git-common-dir`, then
//! `openwarrant/claims/`). Before M11 each worktree kept its own
//! `.openwarrant/state/claims/`, so two agents in two worktrees could both
//! claim one item. Such a claim is still honoured where it lies: it is read,
//! it holds, and its holder can finish or release it; no new claim is taken
//! there. `[tickets] claims_dir` still names one directory outright.
//!
//! The common directory is found from the files git itself reads (`.git`,
//! a `.git` file's `gitdir:`, `commondir`), never by running `git`: a claim
//! costs a stat or two, and a hosted run reads only its basis.

use std::io::Write as _;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

pub const CLAIM_SCHEMA: &str = "oh.war/ticket-claim/v1";

/// One claim, as its lock file holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub schema: String,
    /// A fresh UUID per claim, carried by its journal events.
    pub claim: String,
    pub ticket: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    pub actor: String,
    /// RFC 3339, UTC.
    pub since: String,
    pub since_unix: u64,
}

impl Claim {
    /// How long it has been held, in seconds.
    #[must_use]
    pub fn age(&self, now: u64) -> u64 {
        now.saturating_sub(self.since_unix)
    }

    /// The claimed thing, `t-x` or `t-x/i-y`.
    #[must_use]
    pub fn target(&self) -> String {
        match &self.item {
            Some(item) => format!("{}/{item}", self.ticket),
            None => self.ticket.clone(),
        }
    }
}

/// The lock file's name for a target.
#[must_use]
pub fn lock_name(ticket: &str, item: Option<&str>) -> String {
    match item {
        Some(item) => format!("{ticket}--{item}.lock"),
        None => format!("{ticket}.lock"),
    }
}

/// What an attempt to take a lock found.
#[derive(Debug)]
pub enum Taken {
    /// The lock is now this claim.
    Won,
    /// Someone holds it: their claim, as read. `None` when the file exists
    /// but does not parse (a hand-made or damaged lock), which is held all
    /// the same.
    Held(Option<Claim>),
}

fn nonce() -> String {
    openwarrant_core::WarUuid::mint().to_string()
}

/// Read a lock, if it exists. A lock that exists and does not parse is
/// `Some(None)`: held, by nobody the file names.
pub fn read(path: &Utf8Path) -> std::io::Result<Option<Option<Claim>>> {
    match crate::vfs::read(path) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes).ok())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Take `path` for `claim`, atomically. See the module docs.
pub fn take(path: &Utf8Path, claim: &Claim) -> std::io::Result<Taken> {
    let dir = path.parent().unwrap_or(Utf8Path::new("."));
    std::fs::create_dir_all(dir)?;
    // Claims are runtime state, never a source: the directory ignores itself,
    // whatever the repository's own .gitignore says (a `war init` from before
    // tickets writes none for it).
    let ignore = dir.join(".gitignore");
    if !ignore.exists() {
        let _ = std::fs::write(
            &ignore,
            "# war claim locks: runtime state, never committed\n*\n",
        );
    }
    let bytes = serde_json::to_vec_pretty(claim).map_err(std::io::Error::other)?;
    let temp = dir.join(format!(
        ".{}.{}.{}.claim-tmp",
        path.file_name().unwrap_or("lock"),
        std::process::id(),
        nonce()
    ));
    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        f.write_all(&bytes)?;
        f.write_all(b"\n")?;
        f.sync_all()?;
    }
    let linked = std::fs::hard_link(&temp, path);
    let _ = std::fs::remove_file(&temp);
    match linked {
        Ok(()) => Ok(Taken::Won),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            Ok(Taken::Held(read(path)?.flatten()))
        }
        // A filesystem without hard links: O_EXCL create, then write. A
        // reader in the instant between the two sees an empty lock, which
        // reads as held by nobody named — never as free.
        Err(_) => match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(mut f) => {
                f.write_all(&bytes)?;
                f.write_all(b"\n")?;
                f.sync_all()?;
                Ok(Taken::Won)
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                Ok(Taken::Held(read(path)?.flatten()))
            }
            Err(e) => Err(e),
        },
    }
}

/// What a steal did.
#[derive(Debug)]
pub enum Stolen {
    /// The stale claim `from` was set aside and the lock is now ours.
    Won { from: Option<Claim> },
    /// Someone else got there first: the lock is theirs now.
    Lost(Option<Claim>),
}

/// Take a lock whose current holder is `stale` (as the caller read it and
/// judged it stale). Atomic against another stealer: the stale file is renamed
/// aside, which only one rename can do; a stealer whose rename caught a FRESH
/// claim (someone stole first) puts it back and loses. One window is not
/// closed: a third agent's plain claim landing in the instant between that
/// rename and the put-back wins the lock over the first stealer's. That needs
/// three agents on one stale item within microseconds; the journal records
/// all three claims either way.
pub fn steal(path: &Utf8Path, stale: Option<&Claim>, claim: &Claim) -> std::io::Result<Stolen> {
    steal_into(path, stale, path, claim)
}

/// [`steal`], taking the lock at `to` once the stale one at `from` is set
/// aside: a claim from before claims were shared lies in a worktree's own
/// directory, and the claim that replaces it is taken in the shared one.
pub fn steal_into(
    from: &Utf8Path,
    stale: Option<&Claim>,
    to: &Utf8Path,
    claim: &Claim,
) -> std::io::Result<Stolen> {
    let aside = Utf8PathBuf::from(format!("{from}.stolen.{}", nonce()));
    match std::fs::rename(from, &aside) {
        Ok(()) => {
            let moved: Option<Claim> = std::fs::read(&aside)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok());
            if moved.as_ref() != stale {
                // Not the claim we judged: put it back, and lose.
                let _ = std::fs::hard_link(&aside, from);
                let _ = std::fs::remove_file(&aside);
                return Ok(Stolen::Lost(read(from)?.flatten()));
            }
            let _ = std::fs::remove_file(&aside);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    match take(to, claim)? {
        Taken::Won => Ok(Stolen::Won {
            from: stale.cloned(),
        }),
        Taken::Held(other) => Ok(Stolen::Lost(other)),
    }
}

/// Remove `path` if `actor` holds it. Returns whether it was removed.
pub fn release(path: &Utf8Path, actor: &str) -> std::io::Result<bool> {
    match read(path)? {
        Some(Some(claim)) if claim.actor == actor => {
            std::fs::remove_file(path)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Every claim in `dir`, keyed by lock file name. An unreadable lock is
/// included as held by nobody named.
pub fn all(dir: &Utf8Path) -> std::io::Result<std::collections::BTreeMap<String, Option<Claim>>> {
    let mut out = std::collections::BTreeMap::new();
    let entries = match crate::vfs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let entry = entry?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if !name.ends_with(".lock") || name.starts_with('.') {
            continue;
        }
        let path = dir.join(&name);
        if let Some(claim) = read(&path)? {
            out.insert(name, claim);
        }
    }
    Ok(out)
}

// ---- where claims live (M11) ----------------------------------------------

/// Under git's common directory, where every worktree of a clone keeps its
/// claims.
pub const SHARED_SUBDIR: &str = "openwarrant/claims";

/// How the checkout around a repository root is laid out, as git lays it out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitLayout {
    /// The worktree's top level: the nearest directory, from the root up,
    /// holding `.git`.
    pub toplevel: Utf8PathBuf,
    /// What `git rev-parse --git-common-dir` names.
    pub common_dir: Utf8PathBuf,
}

/// Disk reads that leave no trace in a `war host --export` recording, and
/// basis reads in a hosted run: where claims live is a fact about this
/// checkout, never a member of the repository's basis.
fn layout_is_dir(p: &Utf8Path) -> bool {
    if crate::vfs::is_hosted() {
        crate::vfs::is_dir(p)
    } else {
        p.is_dir()
    }
}

fn layout_read(p: &Utf8Path) -> Option<String> {
    if crate::vfs::is_hosted() {
        crate::vfs::read_to_string(p).ok()
    } else {
        std::fs::read_to_string(p).ok()
    }
}

/// `p` with `.` and `..` resolved by name.
fn lexical(p: &Utf8Path) -> Utf8PathBuf {
    let mut out = Utf8PathBuf::new();
    for c in p.components() {
        match c {
            camino::Utf8Component::CurDir => {}
            camino::Utf8Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_str()),
        }
    }
    out
}

/// A path a git file names, relative to `base` unless absolute.
fn named_path(base: &Utf8Path, text: &str) -> Utf8PathBuf {
    let p = Utf8PathBuf::from(text.trim());
    lexical(&if p.is_absolute() { p } else { base.join(p) })
}

/// The git layout around `root`, or `None` outside a git checkout. Read
/// from `.git` (a directory, or a file naming one with `gitdir:`) and the
/// git directory's `commondir`, as git reads them. No process is started.
#[must_use]
pub fn git_layout(root: &Utf8Path) -> Option<GitLayout> {
    let mut at = Some(root);
    while let Some(dir) = at {
        let dotgit = dir.join(".git");
        let git_dir = if layout_is_dir(&dotgit) {
            Some(dotgit)
        } else {
            layout_read(&dotgit).and_then(|text| {
                text.lines()
                    .find_map(|l| l.strip_prefix("gitdir:"))
                    .map(|p| named_path(dir, p))
            })
        };
        if let Some(git_dir) = git_dir {
            let common_dir = layout_read(&git_dir.join("commondir"))
                .map_or_else(|| git_dir.clone(), |c| named_path(&git_dir, &c));
            return Some(GitLayout {
                toplevel: dir.to_owned(),
                common_dir,
            });
        }
        at = dir.parent();
    }
    None
}

impl GitLayout {
    /// The claims directory every worktree of this clone shares.
    #[must_use]
    pub fn shared_claims_dir(&self) -> Utf8PathBuf {
        self.common_dir.join(SHARED_SUBDIR)
    }

    /// The top level of every worktree of this clone, this one first: the
    /// main worktree (the common directory's parent, when it is a `.git`),
    /// and each linked one git lists under `worktrees/*/gitdir`. A worktree
    /// whose directory is gone is listed all the same; reading it finds
    /// nothing.
    #[must_use]
    pub fn worktrees(&self) -> Vec<Utf8PathBuf> {
        let mut out = vec![self.toplevel.clone()];
        let mut add = |p: Utf8PathBuf| {
            if !out.contains(&p) {
                out.push(p);
            }
        };
        if self.common_dir.file_name() == Some(".git")
            && let Some(main) = self.common_dir.parent()
        {
            add(main.to_owned());
        }
        let listed = self.common_dir.join("worktrees");
        let entries = if crate::vfs::is_hosted() {
            crate::vfs::read_dir_utf8(&listed).unwrap_or_default()
        } else {
            std::fs::read_dir(&listed)
                .map(|rd| {
                    rd.filter_map(Result::ok)
                        .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
                        .collect()
                })
                .unwrap_or_default()
        };
        {
            let mut entries = entries;
            entries.sort();
            for entry in entries {
                if let Some(gitfile) = layout_read(&entry.join("gitdir")) {
                    let gitfile = named_path(&entry, &gitfile);
                    if let Some(top) = gitfile.parent() {
                        add(top.to_owned());
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(label: &str) -> Utf8PathBuf {
        let base = Utf8PathBuf::from_path_buf(std::env::temp_dir()).expect("utf-8 temp dir");
        let dir = base.join(format!("war-claims-{label}-{}", nonce()));
        std::fs::create_dir_all(&dir).expect("scratch");
        dir
    }

    fn claim(actor: &str, since_unix: u64) -> Claim {
        Claim {
            schema: CLAIM_SCHEMA.to_owned(),
            claim: nonce(),
            ticket: "t-3f2a".to_owned(),
            item: Some("i-0001".to_owned()),
            actor: actor.to_owned(),
            since: "2026-09-25T00:00:00Z".to_owned(),
            since_unix,
        }
    }

    /// Thirty-two threads claim one item at once: exactly one wins, and every
    /// loser reads the winner by name.
    #[test]
    fn of_concurrent_claims_exactly_one_wins() {
        for round in 0..20 {
            let dir = scratch(&format!("race{round}"));
            let path = dir.join(lock_name("t-3f2a", Some("i-0001")));
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(32));
            let handles: Vec<_> = (0..32)
                .map(|n| {
                    let path = path.clone();
                    let barrier = barrier.clone();
                    std::thread::spawn(move || {
                        let mine = claim(&format!("agent-{n}"), 1);
                        barrier.wait();
                        (mine.actor.clone(), take(&path, &mine).expect("take"))
                    })
                })
                .collect();
            let results: Vec<(String, Taken)> = handles
                .into_iter()
                .map(|h| h.join().expect("thread"))
                .collect();
            let winners: Vec<&String> = results
                .iter()
                .filter(|(_, t)| matches!(t, Taken::Won))
                .map(|(a, _)| a)
                .collect();
            assert_eq!(winners.len(), 1, "round {round}: {winners:?}");
            let holder = read(&path).expect("read").flatten().expect("a whole claim");
            assert_eq!(&holder.actor, winners[0]);
            for (actor, taken) in &results {
                if let Taken::Held(seen) = taken {
                    assert_eq!(
                        seen.as_ref().map(|c| c.actor.as_str()),
                        Some(holder.actor.as_str()),
                        "{actor} saw the winner by name"
                    );
                }
            }
            // No temporary file survives.
            let names: Vec<String> = std::fs::read_dir(&dir)
                .expect("dir")
                .map(|e| e.expect("entry").file_name().into_string().expect("utf-8"))
                .filter(|n| n != ".gitignore")
                .collect();
            assert_eq!(names, vec![lock_name("t-3f2a", Some("i-0001"))]);
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// The layout is read as git lays it out: a main worktree's `.git`
    /// directory, a linked worktree's `.git` file and `commondir`, and a root
    /// below the top level.
    #[test]
    fn the_common_dir_is_found_from_every_worktree() {
        let base = scratch("layout");
        let main = base.join("main");
        let linked = base.join("linked");
        let admin = main.join(".git/worktrees/linked");
        std::fs::create_dir_all(&admin).expect("admin dir");
        std::fs::create_dir_all(main.join("sub/deeper")).expect("sub");
        std::fs::create_dir_all(&linked).expect("linked");
        std::fs::write(linked.join(".git"), format!("gitdir: {admin}\n")).expect(".git file");
        std::fs::write(admin.join("commondir"), "../..\n").expect("commondir");
        std::fs::write(admin.join("gitdir"), format!("{}\n", linked.join(".git"))).expect("gitdir");

        let from_main = git_layout(&main).expect("main is a checkout");
        assert_eq!(from_main.toplevel, main);
        assert_eq!(from_main.common_dir, main.join(".git"));
        let from_linked = git_layout(&linked).expect("linked is a checkout");
        assert_eq!(from_linked.toplevel, linked);
        assert_eq!(from_linked.common_dir, main.join(".git"));
        assert_eq!(
            from_linked.shared_claims_dir(),
            from_main.shared_claims_dir(),
            "one lock set per clone"
        );
        assert_eq!(from_linked.worktrees(), vec![linked.clone(), main.clone()]);
        assert_eq!(from_main.worktrees(), vec![main.clone(), linked.clone()]);
        let below = git_layout(&main.join("sub/deeper")).expect("below the top level");
        assert_eq!(below.toplevel, main);
        // Refused: a directory that is no checkout has no layout.
        let bare = base.join("plain");
        std::fs::create_dir_all(&bare).expect("plain");
        assert!(git_layout(&bare).is_none_or(|l| l.toplevel != bare));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_stale_claim_is_stolen_once_and_a_fresh_one_is_not_clobbered() {
        let dir = scratch("steal");
        let path = dir.join(lock_name("t-3f2a", Some("i-0001")));
        let old = claim("gone", 1);
        assert!(matches!(take(&path, &old).expect("take"), Taken::Won));
        let a = claim("agent-a", 100);
        let b = claim("agent-b", 100);
        // A steals the stale claim.
        assert!(matches!(
            steal(&path, Some(&old), &a).expect("steal"),
            Stolen::Won { from: Some(ref f) } if f.actor == "gone"
        ));
        // B judged the SAME stale claim a moment ago; its steal finds A's
        // fresh claim instead, puts it back and loses.
        match steal(&path, Some(&old), &b).expect("steal") {
            Stolen::Lost(Some(holder)) => assert_eq!(holder.actor, "agent-a"),
            other => panic!("{other:?}"),
        }
        assert_eq!(
            read(&path).expect("read").flatten().expect("claim").actor,
            "agent-a"
        );
        // Release is the holder's only.
        assert!(!release(&path, "agent-b").expect("release"));
        assert!(release(&path, "agent-a").expect("release"));
        assert!(read(&path).expect("read").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
