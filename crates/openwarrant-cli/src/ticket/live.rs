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
//! any other worktree. A side branch is a proposal, and two agents touching the
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
//! Nothing here signs or touches a signed record, and nothing here is
//! authority: a claim, and so a notice, is coordination. It is off in a hosted
//! or recording run and under `OPENWARRANT_NO_NOTICES`. A notice problem never
//! fails a command: a read ignores it, and a write says so in one warning.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use super::{Store, claim, now_secs, rfc3339, scopes};
use crate::context::task::{SourceEntry, TaskPacket};
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

/// `<common>/refs/heads/<branch>`, loose or packed. No process.
fn ref_sha(common: &Utf8Path, branch: &str) -> Option<String> {
    let name = format!("refs/heads/{branch}");
    if let Ok(text) = std::fs::read_to_string(common.join(&name)) {
        return Some(text.trim().to_owned());
    }
    let packed = std::fs::read_to_string(common.join("packed-refs")).ok()?;
    packed.lines().find_map(|l| {
        let (sha, r) = l.split_once(' ')?;
        (r == name).then(|| sha.to_owned())
    })
}

/// The branch `origin/HEAD` names, else `main`, else `master`, with its tip.
fn default_branch(common: &Utf8Path) -> Option<(String, String)> {
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
        .find_map(|b| ref_sha(common, &b).map(|sha| (b, sha)))
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

/// Compare every live claim's watched files with the default branch, and post
/// what is news. Returns how many notices it posted.
pub fn detect(store: &Store) -> std::io::Result<usize> {
    if !enabled() {
        return Ok(0);
    }
    let Some(layout) = &store.layout else {
        return Ok(0);
    };
    let kept = all_kept(&store.claims_dir);
    if kept.is_empty() {
        return Ok(0);
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
    let Some((branch, tip)) = default_branch(&layout.common_dir) else {
        return Ok(0);
    };
    let here = scopes::workspace(store);
    let on_default = checked_out(&layout.toplevel).as_deref() == Some(branch.as_str());
    let tip_file = dir.join(TIP_FILE);
    let tip_moved = std::fs::read_to_string(&tip_file).ok().as_deref() != Some(tip.as_str());
    if !on_default && !tip_moved {
        return Ok(0);
    }
    let mut view = View {
        branch,
        tip: tip.clone(),
        checkout: on_default.then(|| layout.toplevel.clone()),
        root: layout.toplevel.clone(),
        read: BTreeMap::new(),
    };
    let now = now_secs();
    let mut posted = 0;
    let mut bases: BTreeMap<String, Option<String>> = BTreeMap::new();
    for (path, mut k, c) in live {
        let Some(coord) = c.coordination.clone().filter(|_| !c.lease_expired(now)) else {
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
            posted += 1;
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
    Ok(posted)
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

/// `war start`, once its claim is held: keep what it was given, and, resuming,
/// take what changed since and record it as seen. `root` is the workspace the
/// packet was compiled in.
pub fn started(
    store: &Store,
    root: &Utf8Path,
    ticket: &str,
    packet: &TaskPacket,
    text: &BTreeMap<String, String>,
    reach: &[Reach],
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
    if let Some(layout) = &store.layout {
        let _ = std::fs::remove_file(dir_of(layout).join(TIP_FILE));
    }
    detect(store)
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
