// SPDX-License-Identifier: Apache-2.0
//! Live updates to running agents, on one machine (t-3c16c W3, steps 1 to 5).
//!
//! # What each agent was given
//!
//! `war start` keeps the `oh.war/task-context/v1` source manifest beside the
//! claim, as `given/<lock stem>.json` under the claims directory (a directory
//! of its own, so a recording that lists the locks never reaches it), with the
//! digest of every file it
//! came from and of every file that reaches the claim some other way
//! (`impact::reaching`): by a relation, or by overlapping declared paths.
//!
//! # What counts as a change
//!
//! A change counts once it is on the default branch: the working tree of a
//! checkout on that branch, committed or not, or that branch's tip seen from
//! any other worktree, whichever of the local branch and `origin`'s is further
//! ahead. A side branch is a proposal, and two agents touching the
//! same file on two branches stay a merge matter. It is looked for at every
//! `war` write and in `check --changed`, by comparing digests: no process
//! runs unless a digest differs. A difference is news only when the default
//! branch moved it since the claim's line left it (the file differs at the
//! merge base), and only when the claim's own workspace does not hold those
//! bytes already. An actor's own edit in its own workspace is never news.
//!
//! # Delivery
//!
//! A notice is a file under `<git common dir>/openwarrant/notices/`, named by
//! the claim, the path and the digest, so a second detector writes the same
//! file. The next `war` call by the affected actor, in the affected
//! workspace, renames its notices into `delivered/` and prints one line;
//! the rename is what makes it once. `war start` prints the changes and
//! records them as seen, and `war view prime` shows the line until then.
//!
//! # At done
//!
//! `war start` also keeps what the item declared its work may touch and run,
//! once: a resumed start never moves it, or a widening could be started away.
//! [`standing`] tells `war done` where a claim stands, and
//! [`crate::ticket::enforce`] decides what that means at the item's level.
//!
//! Nothing here signs or touches a signed record, and nothing here is
//! authority: a claim, and so a notice, is coordination. It is off in a hosted
//! or recording run and under `OPENWARRANT_NO_NOTICES`. A notice problem never
//! fails a read, and a write says so in one warning; only `war done` on an
//! item with a trigger or a signature holds it as UNKNOWN.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use super::{Store, claim, now_secs, rfc3339, scopes};
use crate::context::task::{SourceEntry, TaskPacket};
use crate::go::packet::Allowed;
use crate::impact::Reach;
use crate::repo::Repository;

pub const MANIFEST_SCHEMA: &str = "oh.war/claim-manifest/v1";
pub const NOTICE_SCHEMA: &str = "oh.war/notice/v1";
pub const SUBDIR: &str = "openwarrant/notices";
/// Set to anything non-empty, notices are neither posted nor delivered.
pub const DISABLE_ENV: &str = "OPENWARRANT_NO_NOTICES";

/// Under the claims directory: what each claim was given.
const GIVEN: &str = "given";
const DELIVERED: &str = "delivered";
/// The default branch's commit last compared, so a tip that did not move
/// starts no process.
const TIP_FILE: &str = ".tip";
/// A file that is not there, where a digest would be.
const ABSENT: &str = "absent";
/// How much of a diff `war start` prints for one file.
const DIFF_LINES: usize = 30;
/// Most units a notice names.
const NAMED_UNITS: usize = 3;

/// One file a claim is watching.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Watched {
    pub source: String,
    /// `manifest`, `relation` or `paths`.
    pub reach: String,
    /// The record it reached the claim through.
    pub because: String,
    /// The digest last given or seen.
    pub file: String,
    /// Every digest that is no news to this claim.
    pub known: Vec<String>,
}

/// What `war start` gave a claim, kept beside it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kept {
    pub schema: String,
    pub claim: String,
    pub ticket: String,
    pub actor: String,
    pub selection_digest: String,
    /// The packet's `source_manifest`, exactly.
    pub source_manifest: Vec<SourceEntry>,
    pub watched: Vec<Watched>,
    /// A unit's text, for each unit that shares its file with another, so a
    /// change can be narrowed to the units whose text is gone.
    #[serde(default)]
    pub units: BTreeMap<String, String>,
    /// The notices this claim has seen, by id.
    #[serde(default)]
    pub seen: Vec<String>,
    /// The newest change seen in each file: what its workspace must hold
    /// before a trigger can be said to run on the new state.
    #[serde(default)]
    pub taken: Vec<Taken>,
    /// What the item declared when work started, and never after. `None`:
    /// that could not be read, which is not the same as nothing declared.
    #[serde(default)]
    pub declared: Option<Allowed>,
}

/// A change a claim has seen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Taken {
    pub source: String,
    pub digest: String,
    pub place: String,
    /// The commit holding it; `None` when it was uncommitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

/// One change, for one claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notice {
    pub schema: String,
    pub id: String,
    pub claim: String,
    pub ticket: String,
    pub actor: String,
    pub workspace: String,
    pub source: String,
    pub units: Vec<String>,
    pub reach: String,
    /// Where it changed: `on main`, or `uncommitted on main`.
    pub place: String,
    pub digest: String,
    /// The commit the change is measured from, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// The commit holding it; `None` when it is uncommitted in `checkout`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    pub checkout: String,
    pub at: String,
    /// What changed and what to do, in one line.
    pub line: String,
}

/// Who is asking: set once, before a command runs, and touching no file, so
/// a command that reads a basis opens exactly what it did. The first thing to
/// answer (an envelope, or the end of the command) collects the line.
struct Asking {
    root: Option<Utf8PathBuf>,
    actor: Option<String>,
}

static ASKING: OnceLock<Asking> = OnceLock::new();

fn enabled() -> bool {
    crate::vfs::is_disk() && std::env::var_os(DISABLE_ENV).is_none_or(|v| v.is_empty())
}

fn digest(bytes: &[u8]) -> String {
    openwarrant_core::format::sha256(bytes)
}

fn dir_of(layout: &claim::GitLayout) -> Utf8PathBuf {
    layout.common_dir.join(SUBDIR)
}

fn write_whole(path: &Utf8Path, body: &[u8]) -> std::io::Result<()> {
    let tmp = Utf8PathBuf::from(format!("{path}.{}.tmp", std::process::id()));
    std::fs::write(&tmp, body)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

fn notice_files(dir: &Utf8Path) -> Vec<Utf8PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<Utf8PathBuf> = entries
        .flatten()
        .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.ends_with(".json") && !n.starts_with('.'))
        })
        .collect();
    out.sort();
    out
}

fn read_json<T: for<'a> Deserialize<'a>>(path: &Utf8Path) -> Option<T> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

fn kept_path(store: &Store, lock_name: &str) -> Utf8PathBuf {
    let stem = lock_name.strip_suffix(".lock").unwrap_or(lock_name);
    store.claims_dir.join(GIVEN).join(format!("{stem}.json"))
}

fn lock_of_kept(path: &Utf8Path) -> Option<String> {
    Some(format!("{}.lock", path.file_stem()?))
}

fn all_kept(claims_dir: &Utf8Path) -> Vec<(Utf8PathBuf, Option<Kept>)> {
    notice_files(&claims_dir.join(GIVEN))
        .into_iter()
        .map(|p| {
            let kept = read_json(&p);
            (p, kept)
        })
        .collect()
}

/// Every claim of this clone with what it was given, for `war plan impact`.
/// Nothing in a hosted or recording run: a claim is no member of a basis.
#[must_use]
pub fn held(store: &Store) -> Vec<(claim::Claim, Option<Kept>)> {
    if !crate::vfs::is_disk() || store.layout.is_none() {
        return Vec::new();
    }
    let Ok(claims) = store.claims() else {
        return Vec::new();
    };
    claims
        .into_iter()
        .filter_map(|(name, c)| {
            let c = c?;
            let kept = read_json::<Kept>(&kept_path(store, &name)).filter(|k| k.claim == c.claim);
            Some((c, kept))
        })
        .collect()
}

// ---- git, by what it costs ------------------------------------------------

/// `<common>/<name>`, loose or packed. No process.
fn ref_sha(common: &Utf8Path, name: &str) -> Option<String> {
    if let Ok(text) = std::fs::read_to_string(common.join(name)) {
        return Some(text.trim().to_owned());
    }
    let packed = std::fs::read_to_string(common.join("packed-refs")).ok()?;
    packed.lines().find_map(|l| {
        let (sha, r) = l.split_once(' ')?;
        (r == name).then(|| sha.to_owned())
    })
}

/// The further ahead of a branch here and on `origin`. Work in worktrees can
/// leave the local branch long behind what was fetched, so the local ref alone
/// would hear of nothing. One process, and only when the two differ.
fn newest_tip(common: &Utf8Path, root: &Utf8Path, branch: &str) -> Option<String> {
    let local = ref_sha(common, &format!("refs/heads/{branch}"));
    let fetched = ref_sha(common, &format!("refs/remotes/origin/{branch}"));
    match (local, fetched) {
        (Some(l), Some(f)) if l != f => {
            let behind = std::process::Command::new("git")
                .args(["-C", root.as_str(), "merge-base", "--is-ancestor", &l, &f])
                .output()
                .is_ok_and(|o| o.status.success());
            Some(if behind { f } else { l })
        }
        (Some(l), _) => Some(l),
        (None, f) => f,
    }
}

/// The branch `origin/HEAD` names, else `main`, else `master`, with its tip.
fn default_branch(common: &Utf8Path, root: &Utf8Path) -> Option<(String, String)> {
    let origin = std::fs::read_to_string(common.join("refs/remotes/origin/HEAD"))
        .ok()
        .and_then(|t| {
            t.trim()
                .strip_prefix("ref: refs/remotes/origin/")
                .map(str::to_owned)
        });
    origin
        .into_iter()
        .chain(["main".to_owned(), "master".to_owned()])
        .find_map(|b| newest_tip(common, root, &b).map(|sha| (b, sha)))
}

/// The branch checked out at `toplevel`, read from git's own files.
fn checked_out(toplevel: &Utf8Path) -> Option<String> {
    let dotgit = toplevel.join(".git");
    let head = if dotgit.is_dir() {
        dotgit.join("HEAD")
    } else {
        let text = std::fs::read_to_string(&dotgit).ok()?;
        Utf8PathBuf::from(text.trim().strip_prefix("gitdir:")?.trim()).join("HEAD")
    };
    let head = if head.is_absolute() {
        head
    } else {
        toplevel.join(head)
    };
    std::fs::read_to_string(head)
        .ok()?
        .trim()
        .strip_prefix("ref: refs/heads/")
        .map(str::to_owned)
}

/// `path` at `rev`: its bytes, `Some(None)` when it has none there, and `None`
/// when git could not be asked.
fn blob(dir: &Utf8Path, rev: &str, path: &str) -> Option<Option<Vec<u8>>> {
    let out = std::process::Command::new("git")
        .args(["-C", dir.as_str(), "cat-file", "blob"])
        .arg(format!("{rev}:{path}"))
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    Some(out.status.success().then_some(out.stdout))
}

fn digest_at(dir: &Utf8Path, rev: &str, path: &str) -> Option<String> {
    Some(blob(dir, rev, path)?.map_or_else(|| ABSENT.to_owned(), |b| digest(&b)))
}

fn merge_base(dir: &Utf8Path, a: &str, b: &str) -> Option<String> {
    let r = crate::go::git::git(dir, &["merge-base", a, b]);
    r.ok.then(|| r.stdout.trim().to_owned())
        .filter(|s| !s.is_empty())
}

// ---- what the default branch holds ----------------------------------------

/// The one tree a detector compares with.
struct View {
    branch: String,
    tip: String,
    /// The checkout on the default branch, when this is one. `None`: the tip.
    checkout: Option<Utf8PathBuf>,
    root: Utf8PathBuf,
    read: BTreeMap<String, Option<Vec<u8>>>,
}

impl View {
    /// The file's bytes in this view; `None` when that cannot be said.
    fn file(&mut self, source: &str) -> Option<&Option<Vec<u8>>> {
        if !self.read.contains_key(source) {
            let got = if let Some(checkout) = &self.checkout {
                match std::fs::read(checkout.join(source)) {
                    Ok(b) => Some(Some(b)),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(None),
                    Err(_) => None,
                }
            } else {
                blob(&self.root, &self.tip, source)
            };
            if let Some(g) = got {
                self.read.insert(source.to_owned(), g);
            }
        }
        self.read.get(source)
    }

    fn digest(&mut self, source: &str) -> Option<String> {
        let bytes = self.file(source)?;
        Some(
            bytes
                .as_ref()
                .map_or_else(|| ABSENT.to_owned(), |b| digest(b)),
        )
    }
}

fn notice_id(claim: &str, source: &str, file: &str) -> String {
    let d = digest(format!("{claim}\0{source}\0{file}").as_bytes());
    let hex = d.strip_prefix("sha256:").unwrap_or(&d);
    hex[..hex.len().min(32)].to_owned()
}

/// The units of `source` a change to `after` touched: those whose text is
/// gone, the innermost of them. With no text kept, every unit of the file.
fn touched(
    manifest: &[SourceEntry],
    units: &BTreeMap<String, String>,
    source: &str,
    after: Option<&[u8]>,
) -> Vec<String> {
    let after = after.map(String::from_utf8_lossy);
    let gone: Vec<(&str, Option<&str>)> = manifest
        .iter()
        .filter(|e| e.source == source)
        .filter_map(|e| {
            let text = units.get(&e.id).map(String::as_str);
            match (text, &after) {
                (Some(t), Some(a)) if a.contains(t) => None,
                _ => Some((e.id.as_str(), text)),
            }
        })
        .collect();
    gone.iter()
        .filter(|(id, text)| {
            !gone.iter().any(|(other, inner)| {
                other != id && matches!((text, inner), (Some(t), Some(i)) if t.contains(i))
            })
        })
        .map(|(id, _)| (*id).to_owned())
        .collect()
}

fn named(units: &[String], ticket: &str) -> String {
    let parts: Vec<&str> = units
        .iter()
        .filter(|u| !u.starts_with("doc:") && !u.starts_with("host:"))
        .map(|u| {
            u.strip_prefix(ticket)
                .map_or(u.as_str(), |r| r.trim_start_matches('/'))
        })
        .filter(|u| !u.is_empty())
        .collect();
    match parts.len() {
        0 => String::new(),
        n if n <= NAMED_UNITS => format!(" ({})", parts.join(", ")),
        n => format!(
            " ({}, +{})",
            parts[..NAMED_UNITS].join(", "),
            n - NAMED_UNITS
        ),
    }
}

/// What one comparison came to.
#[derive(Debug, Default)]
struct Scan {
    posted: usize,
    /// Files of the claim asked about that could not be compared.
    unread: Vec<String>,
}

/// Compare every live claim's watched files with the default branch, and post
/// what is news. Returns how many notices it posted.
pub fn detect(store: &Store) -> std::io::Result<usize> {
    scan(store, None).map(|s| s.posted)
}

/// [`detect`]. `asked` is a claim whose answer someone is waiting on: it is
/// compared even past its lease, and what could not be compared is said.
fn scan(store: &Store, asked: Option<&str>) -> std::io::Result<Scan> {
    let mut out = Scan::default();
    if !enabled() {
        return Ok(out);
    }
    let Some(layout) = &store.layout else {
        return Ok(out);
    };
    let kept = all_kept(&store.claims_dir);
    if kept.is_empty() {
        return Ok(out);
    }
    let dir = dir_of(layout);
    let claims = store.claims().map_err(std::io::Error::other)?;
    let mut live = Vec::new();
    for (path, k) in kept {
        let name = lock_of_kept(&path).unwrap_or_default();
        match (claims.get(&name), k) {
            (Some(Some(c)), Some(k)) if c.claim == k.claim => live.push((path, k, c.clone())),
            (Some(None), _) => {}
            // The claim ended, or another took its name.
            _ => {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    // No default branch: nothing has happened on it, so nothing is news.
    let Some((branch, tip)) = default_branch(&layout.common_dir, &layout.toplevel) else {
        return Ok(out);
    };
    let here = scopes::workspace(store);
    let on_default = checked_out(&layout.toplevel).as_deref() == Some(branch.as_str());
    let tip_file = dir.join(TIP_FILE);
    let tip_moved = std::fs::read_to_string(&tip_file).ok().as_deref() != Some(tip.as_str());
    if !on_default && !tip_moved {
        return Ok(out);
    }
    let mut view = View {
        branch,
        tip: tip.clone(),
        checkout: on_default.then(|| layout.toplevel.clone()),
        root: layout.toplevel.clone(),
        read: BTreeMap::new(),
    };
    let now = now_secs();
    let mut bases: BTreeMap<String, Option<String>> = BTreeMap::new();
    for (path, mut k, c) in live {
        let is_asked = asked == Some(k.claim.as_str());
        let Some(coord) = c
            .coordination
            .clone()
            .filter(|_| is_asked || !c.lease_expired(now))
        else {
            continue;
        };
        let same_workspace = coord.workspace == here;
        // An actor's own edit, in its own workspace, is no news to it.
        if same_workspace && c.actor == store.actor && view.checkout.is_some() {
            continue;
        }
        let mut learned = false;
        for w in &mut k.watched {
            let Some(now_digest) = view.digest(&w.source) else {
                if is_asked {
                    out.unread.push(w.source.clone());
                }
                continue;
            };
            if w.known.contains(&now_digest) {
                continue;
            }
            let id = notice_id(&k.claim, &w.source, &now_digest);
            let pending = dir.join(format!("{id}.json"));
            if pending.exists() || dir.join(DELIVERED).join(format!("{id}.json")).exists() {
                continue;
            }
            let from = coord.base_revision.as_ref().and_then(|b| {
                bases
                    .entry(b.clone())
                    .or_insert_with(|| merge_base(&view.root, b, &tip))
                    .clone()
            });
            // The branch did not move it since this claim's line left it.
            if from
                .as_deref()
                .and_then(|f| digest_at(&view.root, f, &w.source))
                .is_some_and(|d| d == now_digest)
            {
                w.known.push(now_digest);
                learned = true;
                continue;
            }
            // Its workspace holds these bytes already.
            if (view.checkout.is_none() || !same_workspace)
                && std::fs::read(Utf8Path::new(&coord.workspace).join(&w.source))
                    .is_ok_and(|b| digest(&b) == now_digest)
            {
                w.known.push(now_digest);
                learned = true;
                continue;
            }
            let committed = view.checkout.is_none()
                || digest_at(&view.root, &tip, &w.source).is_some_and(|d| d == now_digest);
            let place = if committed {
                format!("on {}", view.branch)
            } else {
                format!("uncommitted on {}", view.branch)
            };
            let after = view.file(&w.source).and_then(|b| b.as_deref());
            let units = if w.reach == "manifest" {
                touched(&k.source_manifest, &k.units, &w.source, after)
            } else {
                vec![w.because.clone()]
            };
            let verb = if after.is_some() {
                "changed"
            } else {
                "was removed"
            };
            let notice = Notice {
                schema: NOTICE_SCHEMA.to_owned(),
                line: format!(
                    "notice: {}{} {verb} {place}; run `war start {}`.",
                    w.source,
                    named(&units, &k.ticket),
                    k.ticket
                ),
                id: id.clone(),
                claim: k.claim.clone(),
                ticket: k.ticket.clone(),
                actor: c.actor.clone(),
                workspace: coord.workspace.clone(),
                source: w.source.clone(),
                units,
                reach: w.reach.clone(),
                place,
                digest: now_digest,
                from,
                to: committed.then(|| tip.clone()),
                checkout: view.checkout.as_ref().unwrap_or(&view.root).to_string(),
                at: rfc3339(now),
            };
            std::fs::create_dir_all(&dir)?;
            let body = serde_json::to_vec(&notice).map_err(std::io::Error::other)?;
            write_whole(&pending, &body)?;
            out.posted += 1;
        }
        if learned {
            let body = serde_json::to_vec(&k).map_err(std::io::Error::other)?;
            write_whole(&path, &body)?;
        }
    }
    if tip_moved {
        std::fs::create_dir_all(&dir)?;
        write_whole(&tip_file, tip.as_bytes())?;
    }
    Ok(out)
}

// ---- delivery -------------------------------------------------------------

/// Whether `n` is this store's actor's, in this workspace.
fn is_mine(store: &Store, n: &Notice) -> bool {
    n.actor == store.actor
        && n.workspace == scopes::workspace(store)
        && std::env::var("OPENWARRANT_CLAIM_ID")
            .ok()
            .is_none_or(|id| id == n.claim)
}

/// The notices in `dir` that are this actor's, in file order. A notice whose
/// claim has ended is removed.
fn mine_in(store: &Store, dir: &Utf8Path) -> Vec<(Utf8PathBuf, Notice)> {
    let files = notice_files(dir);
    if files.is_empty() {
        return Vec::new();
    }
    let Ok(claims) = store.claims() else {
        return Vec::new();
    };
    let live: BTreeSet<&str> = claims
        .values()
        .flatten()
        .map(|c| c.claim.as_str())
        .collect();
    let mut out = Vec::new();
    for path in files {
        let Some(n) = read_json::<Notice>(&path) else {
            continue;
        };
        if !live.contains(n.claim.as_str()) {
            let _ = std::fs::remove_file(&path);
        } else if is_mine(store, &n) {
            out.push((path, n));
        }
    }
    out
}

/// One line for any number of notices: the same file once, at its newest.
fn one_line<'a>(notices: impl Iterator<Item = &'a Notice>) -> Option<String> {
    let mut by_source: BTreeMap<&str, &Notice> = BTreeMap::new();
    for n in notices {
        let newest = by_source.entry(&n.source).or_insert(n);
        if n.at >= newest.at {
            *newest = n;
        }
    }
    let first = *by_source.values().next()?;
    let tickets: BTreeSet<&str> = by_source.values().map(|n| n.ticket.as_str()).collect();
    if by_source.len() == 1 {
        return Some(first.line.clone());
    }
    let shown: Vec<&str> = by_source.keys().take(2).copied().collect();
    let more = by_source.len() - shown.len();
    let also: Vec<&str> = tickets
        .iter()
        .copied()
        .filter(|t| *t != first.ticket)
        .collect();
    Some(format!(
        "notice: {} files changed ({}{}); run `war start {}`{}.",
        by_source.len(),
        shown.join(", "),
        if more > 0 {
            format!(", +{more}")
        } else {
            String::new()
        },
        first.ticket,
        if also.is_empty() {
            String::new()
        } else {
            format!(" (also {})", also.join(", "))
        }
    ))
}

/// Take this actor's undelivered notices: one line, once. Exactly one caller
/// wins each rename, and only what it won is said.
fn deliver(store: &Store) -> Option<String> {
    let dir = dir_of(store.layout.as_ref()?);
    let mine = mine_in(store, &dir);
    if mine.is_empty() {
        return None;
    }
    let delivered = dir.join(DELIVERED);
    std::fs::create_dir_all(&delivered).ok()?;
    let won: Vec<Notice> = mine
        .into_iter()
        .filter_map(|(path, n)| {
            let name = path.file_name()?;
            std::fs::rename(&path, delivered.join(name)).ok()?;
            Some(n)
        })
        .collect();
    one_line(won.iter())
}

/// The line for what this actor has not seen, told already or not, for
/// `war view prime`, and the end of what this process still owes.
#[must_use]
pub fn unseen_line(store: &Store) -> Option<String> {
    if !enabled() {
        return None;
    }
    let dir = dir_of(store.layout.as_ref()?);
    let mut all = mine_in(store, &dir);
    all.extend(mine_in(store, &dir.join(DELIVERED)));
    let line = one_line(all.iter().map(|(_, n)| n))?;
    let _ = std::fs::create_dir_all(dir.join(DELIVERED));
    for (path, _) in all {
        if let Some(name) = path.file_name()
            && path.parent() == Some(dir.as_path())
        {
            let _ = std::fs::rename(&path, dir.join(DELIVERED).join(name));
        }
    }
    Some(line)
}

/// What this process's caller is owed, taken: one line, once.
#[must_use]
pub fn take_line() -> Option<String> {
    let asking = ASKING.get()?;
    if !enabled() {
        return None;
    }
    let root = asking.root.as_deref();
    if notice_files(&dir_of(&layout_at(root)?)).is_empty() {
        return None;
    }
    deliver(&open_store(root, asking.actor.as_deref())?)
}

/// A long-running server serves one repository: every envelope it builds
/// collects that repository's line for itself.
pub fn serve(root: &Utf8Path) {
    let _ = ASKING.set(Asking {
        root: Some(root.to_owned()),
        actor: None,
    });
}

/// A server's write: look for changes, ignoring a problem.
pub fn after_served_write(store: &Store, command: &str) {
    if writes(command, false) {
        let _ = detect(store);
    }
}

// ---- `war start` ----------------------------------------------------------

/// What `war start` found changed for a claim it resumed.
#[derive(Debug, Clone, Serialize)]
pub struct Change {
    pub source: String,
    pub units: Vec<String>,
    pub place: String,
    /// The change itself, bounded.
    pub diff: String,
}

fn bounded_diff(n: &Notice) -> String {
    let Some(from) = &n.from else {
        return String::new();
    };
    let mut args = vec![
        "-C",
        n.checkout.as_str(),
        "diff",
        "--no-color",
        "--unified=1",
        from,
    ];
    if let Some(to) = &n.to {
        args.push(to);
    }
    args.extend(["--", n.source.as_str()]);
    let Ok(out) = std::process::Command::new("git")
        .args(args)
        .stderr(std::process::Stdio::null())
        .output()
    else {
        return String::new();
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let body: Vec<&str> = text.lines().skip_while(|l| !l.starts_with("@@")).collect();
    let shown = body.len().min(DIFF_LINES);
    let mut s = body[..shown].join("\n");
    if body.len() > shown {
        s.push_str(&format!(
            "\n... {} more line(s): git -C {} diff {from} {} -- {}",
            body.len() - shown,
            n.checkout,
            n.to.as_deref().unwrap_or(""),
            n.source
        ));
    }
    s
}

/// The human form of what a resumed `war start` found.
#[must_use]
pub fn render(ticket: &str, changes: &[Change]) -> String {
    let mut s = format!(
        "changed since you started {ticket} (`war view prime {ticket}` shows all of it):\n"
    );
    for c in changes {
        s.push_str(&format!(
            "- {}{} {}\n",
            c.source,
            named(&c.units, ticket),
            c.place
        ));
        if !c.diff.is_empty() {
            s.push_str(&c.diff);
            s.push('\n');
        }
    }
    s
}

fn own_digest(root: &Utf8Path, source: &str) -> Option<String> {
    match std::fs::read(root.join(source)) {
        Ok(b) => Some(digest(&b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(ABSENT.to_owned()),
        Err(_) => None,
    }
}

fn watching(root: &Utf8Path, source: &str, reach: &str, because: &str) -> Option<Watched> {
    let file = own_digest(root, source)?;
    Some(Watched {
        source: source.to_owned(),
        reach: reach.to_owned(),
        because: because.to_owned(),
        known: vec![file.clone()],
        file,
    })
}

/// The files of a packet and of what else reaches its claim, the first way
/// each was reached kept. Notes projected from a journal are not watched:
/// they name the journal, which every act moves.
fn watch_list(root: &Utf8Path, packet: &TaskPacket, reach: &[Reach]) -> Vec<Watched> {
    let mut out: Vec<Watched> = Vec::new();
    let manifest = packet
        .source_manifest
        .iter()
        .filter(|e| e.coverage != "derived-journal-notes")
        .map(|e| (e.source.as_str(), "manifest", e.id.as_str()));
    let others = reach
        .iter()
        .map(|r| (r.source.as_str(), r.via, r.because.as_str()));
    for (source, via, because) in manifest.chain(others) {
        if !out.iter().any(|w| w.source == source)
            && let Some(w) = watching(root, source, via, because)
        {
            out.push(w);
        }
    }
    out
}

/// Text for each unit that shares its file: a lone unit is the file's digest.
fn unit_texts(packet: &TaskPacket, text: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    for e in &packet.source_manifest {
        *count.entry(&e.source).or_default() += 1;
    }
    packet
        .source_manifest
        .iter()
        .filter(|e| count.get(e.source.as_str()).is_some_and(|n| *n > 1))
        .filter_map(|e| Some((e.id.clone(), text.get(&e.id)?.clone())))
        .collect()
}

/// `war start`, once its claim is held: keep what it was given and what the
/// item declares, and, resuming, take what changed since and record it as
/// seen. `root` is the workspace the packet was compiled in.
pub fn started(
    store: &Store,
    root: &Utf8Path,
    ticket: &str,
    packet: &TaskPacket,
    text: &BTreeMap<String, String>,
    reach: &[Reach],
    declared: Option<Allowed>,
) -> std::io::Result<Vec<Change>> {
    if !enabled() {
        return Ok(Vec::new());
    }
    let Some(layout) = &store.layout else {
        return Ok(Vec::new());
    };
    let claims = store.claims().map_err(std::io::Error::other)?;
    let Some((name, c)) = claims.iter().find_map(|(name, c)| {
        let c = c.as_ref()?;
        (c.ticket == ticket && super::owns_claim(store, c)).then_some((name, c))
    }) else {
        return Ok(Vec::new());
    };
    let path = kept_path(store, name);
    let Some(mut kept) = read_json::<Kept>(&path).filter(|k| k.claim == c.claim) else {
        let kept = Kept {
            schema: MANIFEST_SCHEMA.to_owned(),
            claim: c.claim.clone(),
            ticket: ticket.to_owned(),
            actor: c.actor.clone(),
            selection_digest: packet.selection_digest.clone(),
            source_manifest: packet.source_manifest.clone(),
            watched: watch_list(root, packet, reach),
            units: unit_texts(packet, text),
            seen: Vec::new(),
            taken: Vec::new(),
            declared,
        };
        let body = serde_json::to_vec(&kept).map_err(std::io::Error::other)?;
        std::fs::create_dir_all(store.claims_dir.join(GIVEN))?;
        write_whole(&path, &body)?;
        // The next detector compares this claim with the tip, moved or not.
        let _ = std::fs::remove_file(dir_of(layout).join(TIP_FILE));
        return Ok(Vec::new());
    };
    let dir = dir_of(layout);
    let _ = detect_for_self(store);
    let mut all = mine_in(store, &dir);
    all.extend(mine_in(store, &dir.join(DELIVERED)));
    all.retain(|(_, n)| n.claim == kept.claim);
    let mut newest: BTreeMap<String, &Notice> = BTreeMap::new();
    for (_, n) in &all {
        for w in kept.watched.iter_mut().filter(|w| w.source == n.source) {
            if !w.known.contains(&n.digest) {
                w.known.push(n.digest.clone());
            }
            w.file = n.digest.clone();
        }
        // Their text is the text that changed: nothing to narrow by.
        for u in &n.units {
            kept.units.remove(u);
        }
        kept.seen.push(n.id.clone());
        let e = newest.entry(n.source.clone()).or_insert(n);
        if n.at >= e.at {
            *e = n;
        }
    }
    for n in newest.values() {
        kept.taken.retain(|t| t.source != n.source);
        kept.taken.push(Taken {
            source: n.source.clone(),
            digest: n.digest.clone(),
            place: n.place.clone(),
            to: n.to.clone(),
        });
    }
    // Only a manifest kept before declarations were has none to protect.
    if kept.declared.is_none() {
        kept.declared = declared;
    }
    let changes: Vec<Change> = newest
        .values()
        .map(|n| Change {
            source: n.source.clone(),
            units: n.units.clone(),
            place: n.place.clone(),
            diff: bounded_diff(n),
        })
        .collect();
    // What it was not watching at its last start, and text it holds again.
    for w in watch_list(root, packet, reach) {
        if !kept.watched.iter().any(|k| k.source == w.source) {
            kept.watched.push(w);
        }
    }
    for e in &packet.source_manifest {
        if !kept.source_manifest.iter().any(|k| k.id == e.id) {
            kept.source_manifest.push(e.clone());
        }
    }
    for (id, t) in unit_texts(packet, text) {
        let source = packet.source_manifest.iter().find(|e| e.id == id);
        let current = source.is_some_and(|e| {
            let own = own_digest(root, &e.source);
            kept.watched
                .iter()
                .any(|w| w.source == e.source && Some(&w.file) == own.as_ref())
        });
        if current {
            kept.units.insert(id, t);
        }
    }
    let body = serde_json::to_vec(&kept).map_err(std::io::Error::other)?;
    write_whole(&path, &body)?;
    for (p, _) in &all {
        let _ = std::fs::remove_file(p);
    }
    Ok(changes)
}

/// [`detect`], with the tip compared whether it moved or not: a resumed start
/// asks now, and must not wait for a later write to have looked.
fn detect_for_self(store: &Store) -> std::io::Result<usize> {
    forget_tip(store);
    detect(store)
}

fn forget_tip(store: &Store) {
    if let Some(layout) = &store.layout {
        let _ = std::fs::remove_file(dir_of(layout).join(TIP_FILE));
    }
}

// ---- `war done` -----------------------------------------------------------

/// Where a claim stands when its work is said to be done.
#[derive(Debug)]
pub enum Standing {
    /// Nothing was kept for it: it was not started with `war start`, or
    /// nothing is on this machine to keep it in.
    Unrecorded,
    /// What was kept, or what became of it, could not be read.
    Unread(String),
    Read(Box<Given>),
}

/// What a started claim was given, and what has become of it.
#[derive(Debug)]
pub struct Given {
    pub kept: Kept,
    /// Changes it has not seen, one per file.
    pub unseen: Vec<Notice>,
    /// Changes it has seen and its workspace does not hold.
    pub behind: Vec<Taken>,
}

/// The notices for `claim` in `dir`. One that cannot be read may be its own.
fn claim_notices(dir: &Utf8Path, claim: &str) -> Result<Vec<Notice>, String> {
    if let Err(e) = std::fs::read_dir(dir)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        return Err(format!("{dir}: {e}"));
    }
    let mut out = Vec::new();
    for path in notice_files(dir) {
        match read_json::<Notice>(&path) {
            Some(n) if n.claim == claim => out.push(n),
            Some(_) => {}
            None => return Err(format!("{path} could not be read")),
        }
    }
    Ok(out)
}

/// Whether `root` holds a change: the same bytes, or the commit it came in,
/// with whatever was done on top of it.
fn holds(root: &Utf8Path, taken: &Taken) -> bool {
    own_digest(root, &taken.source).as_deref() == Some(taken.digest.as_str())
        || taken.to.as_ref().is_some_and(|to| {
            std::process::Command::new("git")
                .args(["-C", root.as_str(), "merge-base", "--is-ancestor", to])
                .arg("HEAD")
                .output()
                .is_ok_and(|o| o.status.success())
        })
}

/// What `store`'s actor's claim on `ticket` was given and what has become of
/// it, compared now: a change nobody has run `war` since is still a change.
#[must_use]
pub fn standing(store: &Store, ticket: &str) -> Standing {
    if !crate::vfs::is_disk() {
        return Standing::Unrecorded;
    }
    let Some(layout) = &store.layout else {
        return Standing::Unrecorded;
    };
    let claims = match store.claims() {
        Ok(claims) => claims,
        Err(e) => return Standing::Unread(e.to_string()),
    };
    let mut found = None;
    for (name, c) in &claims {
        let Some(c) = c.as_ref().filter(|c| c.ticket == ticket) else {
            continue;
        };
        if !super::owns_claim(store, c) {
            continue;
        }
        let path = kept_path(store, name);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Standing::Unread(format!("{path}: {e}")),
        };
        match serde_json::from_slice::<Kept>(&bytes) {
            Ok(k) if k.claim == c.claim => {
                found = Some(k);
                break;
            }
            Ok(_) => {}
            Err(e) => return Standing::Unread(format!("{path}: {e}")),
        }
    }
    let Some(kept) = found else {
        return Standing::Unrecorded;
    };
    if !enabled() {
        return Standing::Unread(format!("notices are off ({DISABLE_ENV})"));
    }
    forget_tip(store);
    match scan(store, Some(&kept.claim)) {
        Ok(s) if s.unread.is_empty() => {}
        Ok(s) => return Standing::Unread(format!("could not compare {}", s.unread.join(", "))),
        Err(e) => return Standing::Unread(e.to_string()),
    }
    let dir = dir_of(layout);
    let mut newest: BTreeMap<String, Notice> = BTreeMap::new();
    for place in [dir.clone(), dir.join(DELIVERED)] {
        let notices = match claim_notices(&place, &kept.claim) {
            Ok(notices) => notices,
            Err(why) => return Standing::Unread(why),
        };
        for n in notices {
            let seen = kept
                .watched
                .iter()
                .any(|w| w.source == n.source && w.known.contains(&n.digest));
            if !seen && newest.get(&n.source).is_none_or(|e| n.at >= e.at) {
                newest.insert(n.source.clone(), n);
            }
        }
    }
    let root = store.check_root.as_ref().unwrap_or(&store.root);
    let behind = kept
        .taken
        .iter()
        .filter(|t| !holds(root, t))
        .cloned()
        .collect();
    Standing::Read(Box::new(Given {
        kept,
        unseen: newest.into_values().collect(),
        behind,
    }))
}

/// A path pattern `was` allows everything `now` does. `src/a.rs` under
/// `src/**` is no widening, and neither is `src/x/**`; two patterns neither
/// of which plainly holds the other are taken as one, which errs towards
/// saying so.
fn path_covers(was: &str, now: &str) -> bool {
    let wild = |p: &str| p.contains(['*', '?', '[', '{']);
    let (was, now) = (was.trim_start_matches("./"), now.trim_start_matches("./"));
    if was == now || was == "**" {
        return true;
    }
    if !wild(now) {
        return crate::context::glob_match(was, now);
    }
    was.strip_suffix("/**")
        .or_else(|| was.strip_suffix('/'))
        .is_some_and(|dir| !wild(dir) && now.starts_with(&format!("{dir}/")))
}

/// What `now` allows that `was` did not, by name. Nothing declared means no
/// limit: there, nothing can widen, and to declare nothing is to lift it.
#[must_use]
pub fn widened(was: &Allowed, now: &Allowed) -> Vec<String> {
    fn axis(
        key: &str,
        was: &[String],
        now: &[String],
        covers: impl Fn(&str, &str) -> bool,
    ) -> Vec<String> {
        if was.is_empty() {
            return Vec::new();
        }
        if now.is_empty() {
            return vec![format!("{key}: limit removed")];
        }
        now.iter()
            .filter(|n| !was.iter().any(|w| covers(w, n)))
            .map(|n| format!("{key} + {n}"))
            .collect()
    }
    let same = |w: &str, n: &str| w == n;
    let mut out = axis("paths", &was.paths, &now.paths, path_covers);
    out.extend(axis(super::checks::TOOLS_KEY, &was.tools, &now.tools, same));
    out.extend(axis(
        super::checks::COMMANDS_KEY,
        &was.commands,
        &now.commands,
        same,
    ));
    out
}

// ---- the hook around a command --------------------------------------------

/// Whether `leaf` is a command that changes something a claim may read.
#[must_use]
pub fn writes(leaf: &str, changed: bool) -> bool {
    const WRITES: &[&str] = &[
        "add", "amend", "answer", "ask", "claim", "compile", "create", "deliver", "done", "edit",
        "new", "note", "promote", "release", "reopen", "start", "type",
    ];
    changed || WRITES.contains(&leaf)
}

/// Commands that never hear or look: a hosted or recording run reads nothing
/// but its basis, a person's signing is outside the agent loop, and `mcp`
/// collects per call.
fn silent(leaf: Option<&str>) -> bool {
    leaf.is_none_or(|l| {
        matches!(
            l,
            "host" | "mcp" | "tui" | "sign" | "init" | "update" | "version" | "install" | "sdk"
        ) || l.starts_with("__")
    })
}

fn layout_at(root: Option<&Utf8Path>) -> Option<claim::GitLayout> {
    let start = match root {
        Some(r) => r.to_owned(),
        None => Utf8PathBuf::from_path_buf(std::env::current_dir().ok()?).ok()?,
    };
    claim::git_layout(&start)
}

fn open_store(root: Option<&Utf8Path>, actor: Option<&str>) -> Option<Store> {
    let repo = Repository::discover(root.map(Utf8Path::to_owned)).ok()?;
    Store::open(&repo, actor).ok()
}

/// Before a command: say who is asking. It reads nothing.
pub fn asking(root: Option<&Utf8Path>, leaf: Option<&str>, actor: Option<&str>) {
    if !silent(leaf) {
        let _ = ASKING.set(Asking {
            root: root.map(Utf8Path::to_owned),
            actor: actor.map(str::to_owned),
        });
    }
}

/// After a command: look for changes if it wrote, then say what is owed, on
/// stderr. A `--json` caller reads its envelope, not stderr: what the envelope
/// could not carry waits for its next one. A problem on a write is one
/// warning; on anything else it is ignored.
pub fn after(
    root: Option<&Utf8Path>,
    leaf: Option<&str>,
    actor: Option<&str>,
    changed: bool,
    json: bool,
) {
    if !silent(leaf)
        && enabled()
        && let Some(leaf) = leaf
        && writes(leaf, changed)
        && let Some(layout) = layout_at(root)
    {
        // No claim watching anything: nothing to compare, nothing to open.
        let any = !notice_files(&layout.shared_claims_dir().join(GIVEN)).is_empty();
        if any
            && let Some(store) = open_store(root, actor)
            && let Err(e) = detect(&store)
            && !changed
        {
            eprintln!("warning (notices.unavailable): {e}");
        }
    }
    if !json && let Some(line) = take_line() {
        eprintln!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, source: &str) -> SourceEntry {
        SourceEntry {
            id: id.to_owned(),
            source: source.to_owned(),
            revision: String::new(),
            payload_sha256: String::new(),
            coverage: "record".to_owned(),
        }
    }

    fn kept(units: &[(&str, &str)]) -> Kept {
        Kept {
            schema: MANIFEST_SCHEMA.to_owned(),
            claim: "c".to_owned(),
            ticket: "t-1".to_owned(),
            actor: "a".to_owned(),
            selection_digest: String::new(),
            source_manifest: units.iter().map(|(id, _)| entry(id, "p.md")).collect(),
            watched: Vec::new(),
            units: units
                .iter()
                .map(|(id, t)| ((*id).to_owned(), (*t).to_owned()))
                .collect(),
            seen: Vec::new(),
            taken: Vec::new(),
            declared: None,
        }
    }

    fn notice(source: &str, at: &str, line: &str) -> Notice {
        Notice {
            schema: NOTICE_SCHEMA.to_owned(),
            id: String::new(),
            claim: "c".to_owned(),
            ticket: "t-1".to_owned(),
            actor: "a".to_owned(),
            workspace: String::new(),
            source: source.to_owned(),
            units: Vec::new(),
            reach: "manifest".to_owned(),
            place: "on main".to_owned(),
            digest: String::new(),
            from: None,
            to: None,
            checkout: String::new(),
            at: at.to_owned(),
            line: line.to_owned(),
        }
    }

    #[test]
    fn only_the_units_whose_text_is_gone_are_touched() {
        let k = kept(&[("t-1/S-a", "## A\none\n"), ("t-1/S-b", "## B\ntwo\n")]);
        let of = |after: &[u8]| touched(&k.source_manifest, &k.units, "p.md", Some(after));
        assert_eq!(of(b"## A\none\n## B\nTWO\n"), ["t-1/S-b"]);
        assert!(of(b"## A\none\n## B\ntwo\n").is_empty());
    }

    #[test]
    fn a_touched_section_stands_for_the_item_it_does_not_contain() {
        let k = kept(&[
            ("t-1/S-c", "## C\n- [ ] x\n- [ ] y\n"),
            ("t-1/i-1", "- [ ] x\n"),
        ]);
        assert_eq!(
            touched(
                &k.source_manifest,
                &k.units,
                "p.md",
                Some(b"## C\n- [x] x\n- [ ] y\n")
            ),
            ["t-1/i-1"]
        );
    }

    #[test]
    fn a_removed_file_touches_every_unit() {
        let k = kept(&[("t-1/S-a", "a"), ("t-1/S-b", "b")]);
        assert_eq!(touched(&k.source_manifest, &k.units, "p.md", None).len(), 2);
    }

    #[test]
    fn many_notices_are_one_line_and_one_file_is_its_newest_notice() {
        let same = [
            notice("a.md", "2026-01-01T00:00:00Z", "old"),
            notice("a.md", "2026-01-02T00:00:00Z", "new"),
        ];
        assert_eq!(one_line(same.iter()).as_deref(), Some("new"));
        let three = [
            notice("a.md", "t", "x"),
            notice("b.md", "t", "y"),
            notice("c.md", "t", "z"),
        ];
        assert_eq!(
            one_line(three.iter()).as_deref(),
            Some("notice: 3 files changed (a.md, b.md, +1); run `war start t-1`.")
        );
        assert_eq!(one_line(std::iter::empty()), None);
    }

    #[test]
    fn a_unit_list_is_short_and_drops_whole_file_ids() {
        let u = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(named(&u(&["t-1/S-a"]), "t-1"), " (S-a)");
        assert_eq!(named(&u(&["host:AGENTS.md"]), "t-1"), "");
        assert_eq!(named(&u(&["t-1"]), "t-1"), "");
        assert_eq!(
            named(&u(&["t-1/a", "t-1/b", "t-1/c", "t-1/d"]), "t-1"),
            " (a, b, c, +1)"
        );
    }

    fn allowed(paths: &[&str], tools: &[&str], commands: &[&str]) -> Allowed {
        let own = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect();
        Allowed {
            paths: own(paths),
            tools: own(tools),
            commands: own(commands),
        }
    }

    #[test]
    fn a_widening_is_named_and_a_narrowing_is_not_one() {
        let was = allowed(&["src/**"], &["Read"], &["cargo test"]);
        assert!(widened(&was, &was).is_empty());
        let wider = allowed(
            &["src/**", "docs/**"],
            &["Read", "Bash"],
            &["cargo test", "rm"],
        );
        assert_eq!(
            widened(&was, &wider),
            ["paths + docs/**", "x.tools + Bash", "x.commands + rm"]
        );
        // Inside what was declared: nothing added.
        let inside = allowed(&["src/a.rs", "src/x/**"], &["Read"], &["cargo test"]);
        assert!(widened(&was, &inside).is_empty());
    }

    #[test]
    fn lifting_a_limit_widens_and_declaring_a_first_one_does_not() {
        let was = allowed(&["src/**"], &["Read"], &[]);
        assert_eq!(
            widened(&was, &allowed(&[], &["Read"], &[])),
            ["paths: limit removed"]
        );
        assert!(widened(&allowed(&[], &[], &[]), &was).is_empty());
    }

    #[test]
    fn one_star_does_not_hold_two() {
        assert!(!path_covers("src/*", "src/**"));
        assert!(path_covers("src/*", "src/a.rs"));
        assert!(!path_covers("src/*", "src/x/a.rs"));
        assert!(!path_covers("src/a/**", "src/**"));
    }

    #[test]
    fn a_read_is_not_a_write_and_a_hosted_command_never_hears() {
        assert!(writes("note", false));
        assert!(!writes("prime", false));
        assert!(writes("check", true));
        assert!(silent(Some("host")));
        assert!(silent(None));
        assert!(!silent(Some("status")));
    }

    #[test]
    fn a_recording_or_hosted_run_is_off() {
        let tree = std::rc::Rc::new(crate::vfs::Tree::new(
            Utf8PathBuf::from("/virtual"),
            BTreeMap::new(),
            BTreeSet::new(),
        ));
        assert!(!crate::vfs::hosted(tree, enabled));
    }
}

#[cfg(test)]
mod newest_tip_tests {
    use super::*;

    fn git(root: &Utf8Path, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .args(["-C", root.as_str()])
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args(["-c", "commit.gpgsign=false"])
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "{args:?}");
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    }

    #[test]
    fn the_further_ahead_of_here_and_origin_is_the_tip_and_neither_alone_is_lost() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        git(root, &["init", "-q", "-b", "main", "."]);
        git(root, &["commit", "-q", "--allow-empty", "-m", "one"]);
        let one = git(root, &["rev-parse", "HEAD"]);
        git(root, &["commit", "-q", "--allow-empty", "-m", "two"]);
        let two = git(root, &["rev-parse", "HEAD"]);
        let common = root.join(".git");
        // Nothing fetched: the local branch.
        assert_eq!(newest_tip(&common, root, "main").unwrap(), two);
        // Origin behind: still the local branch, which holds unpushed work.
        git(root, &["update-ref", "refs/remotes/origin/main", &one]);
        assert_eq!(newest_tip(&common, root, "main").unwrap(), two);
        // Local behind what was fetched: origin's.
        git(root, &["update-ref", "refs/remotes/origin/main", &two]);
        git(root, &["reset", "-q", "--hard", &one]);
        assert_eq!(newest_tip(&common, root, "main").unwrap(), two);
        // No local branch of that name: what was fetched.
        git(root, &["update-ref", "refs/remotes/origin/other", &two]);
        assert_eq!(newest_tip(&common, root, "other").unwrap(), two);
        assert_eq!(newest_tip(&common, root, "none"), None);
    }
}
