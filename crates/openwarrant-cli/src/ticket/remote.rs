// SPDX-License-Identifier: Apache-2.0
//! Claims across machines, with no server (M11).
//!
//! With `[claims] remote = "<name>"` in `openwarrant.toml` (off by default),
//! a claim taken on this machine is also published to that git remote as a
//! ref, `refs/openwarrant/claims/<ticket>[--<item>]`, pointing at a commit
//! whose message is the claim. The push is `git push --atomic
//! --force-with-lease=<ref>:<expected>`: the remote updates the ref only if
//! it still holds what this machine read (nothing, for a new claim), so the
//! remote is the compare-and-set arbiter. Of two machines claiming one item,
//! exactly one push lands; the other reads the winner's claim and is refused
//! by name.
//!
//! The local lock is taken first, so the worktrees of one clone settle among
//! themselves before any push. A claim lost at the remote releases the local
//! lock it took, and so does a push that timed out: the claim may be on the
//! remote, but nothing established it, and the remote stays the arbiter. The
//! ref refuses other agents until its lease runs out, whatever this machine's
//! lock says. Finishing or releasing deletes the ref, again with a lease, so a
//! claim reclaimed meanwhile is never deleted by its previous holder.
//!
//! The commit is written with `git commit-tree --no-gpg-sign` under a fixed
//! identity: nothing here signs, and nothing reads the commit as anything but
//! a claim. Pushes skip hooks (`--no-verify`) and never prompt.

use std::io::Read as _;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use super::claim::Claim;

/// Where claim refs live on the remote.
pub const REF_PREFIX: &str = "refs/openwarrant/claims/";
/// Where the notices one clone leaves for the others live.
pub const NOTICE_PREFIX: &str = "refs/openwarrant/notices/";

/// How long one git call to the remote may take before it reads as
/// unreachable.
const DEADLINE: Duration = Duration::from_secs(30);
/// How long a view waits on the remote before its claims read as `UNKNOWN`.
const VIEW_DEADLINE: Duration = Duration::from_secs(10);
/// `[claims] remote_ttl`'s default: how many seconds the claims and notices
/// read from the remote serve every view before they are read again.
pub const DEFAULT_TTL_SECS: u64 = 30;
/// Under git's common directory: the remote's claims as last read.
const SEEN_FILE: &str = "openwarrant/remote-claims.json";

/// `[claims]` in `openwarrant.toml`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// The git remote claims are published to; absent, claims stay on this
    /// machine.
    #[serde(default)]
    pub remote: Option<String>,
    /// FORMAT-1.0 §11.3: a claim's lease, in minutes (default 30). Read
    /// before `[tickets] claim_lease_minutes`.
    #[serde(default)]
    pub claim_lease_minutes: Option<f64>,
    /// FORMAT-1.0 §11.3: how long since a live claim was taken before
    /// `--steal` may take it, in minutes (default 120). Read before
    /// `[tickets] claim_ttl_minutes`.
    #[serde(default)]
    pub claim_ttl_minutes: Option<u64>,
    /// How many seconds what was read from the remote (its claims, and the
    /// notices other clones left) serves before it is read again (default
    /// [`DEFAULT_TTL_SECS`]).
    #[serde(default)]
    pub remote_ttl: Option<u64>,
}

/// The ref a lock is published as.
#[must_use]
pub fn ref_name(lock_name: &str) -> String {
    format!(
        "{REF_PREFIX}{}",
        lock_name.strip_suffix(".lock").unwrap_or(lock_name)
    )
}

/// What this machine last published for a claim, kept beside its lock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Published {
    pub remote: String,
    #[serde(rename = "ref")]
    pub refname: String,
    pub commit: String,
    pub lease_until_unix: u64,
}

/// The file beside a lock that records its publication.
#[must_use]
pub fn sidecar(lock: &Utf8Path) -> Utf8PathBuf {
    Utf8PathBuf::from(format!("{lock}.remote"))
}

pub fn read_sidecar(lock: &Utf8Path) -> Option<Published> {
    std::fs::read(sidecar(lock))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
}

pub fn write_sidecar(lock: &Utf8Path, p: &Published) {
    if let Ok(bytes) = serde_json::to_vec(p) {
        let _ = crate::compile::atomic::write(&sidecar(lock), bytes);
    }
}

pub fn remove_sidecar(lock: &Utf8Path) {
    let _ = std::fs::remove_file(sidecar(lock));
}

/// One git call: its status and output.
struct Ran {
    ok: bool,
    stdout: String,
    stderr: String,
}

/// Why a git call has no answer.
enum Unanswered {
    /// It never started: whatever it was to do, it did not do.
    NotStarted(String),
    /// It started and was not seen to finish: whatever it was to do may have
    /// been done.
    Unfinished(String),
}

impl From<Unanswered> for String {
    fn from(u: Unanswered) -> Self {
        match u {
            Unanswered::NotStarted(why) | Unanswered::Unfinished(why) => why,
        }
    }
}

fn git(root: &Utf8Path, args: &[&str], stdin: Option<&[u8]>) -> Result<Ran, Unanswered> {
    git_within(root, args, stdin, DEADLINE)
}

fn git_within(
    root: &Utf8Path,
    args: &[&str],
    stdin: Option<&[u8]>,
    deadline: Duration,
) -> Result<Ran, Unanswered> {
    let started = Instant::now();
    let mut child = Command::new("git")
        .args(["-c", "core.askPass=", "-c", "credential.interactive=never"])
        .args(args)
        .current_dir(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_AUTHOR_NAME", "war")
        .env("GIT_AUTHOR_EMAIL", "war@openwarrant.invalid")
        .env("GIT_COMMITTER_NAME", "war")
        .env("GIT_COMMITTER_EMAIL", "war@openwarrant.invalid")
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Unanswered::NotStarted(format!("could not run git: {e}")))?;
    if let (Some(bytes), Some(mut pipe)) = (stdin, child.stdin.take()) {
        use std::io::Write as _;
        let _ = pipe.write_all(bytes);
    }
    let mut out = child.stdout.take().expect("piped");
    let mut err = child.stderr.take().expect("piped");
    let reader = std::thread::spawn(move || {
        let mut o = String::new();
        let _ = out.read_to_string(&mut o);
        let mut e = String::new();
        let _ = err.read_to_string(&mut e);
        (o, e)
    });
    let mut pause = Duration::from_millis(1);
    let status = loop {
        if let Some(s) = child
            .try_wait()
            .map_err(|e| Unanswered::Unfinished(format!("could not wait for git: {e}")))?
        {
            break s;
        }
        if started.elapsed() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Unanswered::Unfinished(format!(
                "git {} took more than {}s",
                args.first().unwrap_or(&""),
                deadline.as_secs()
            )));
        }
        std::thread::sleep(pause);
        pause = (pause * 2).min(Duration::from_millis(20));
    };
    let (stdout, stderr) = reader.join().unwrap_or_default();
    Ok(Ran {
        ok: status.success(),
        stdout,
        stderr,
    })
}

fn first_line(s: &str) -> String {
    s.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .chars()
        .take(200)
        .collect()
}

/// The commit that carries `claim`: the empty tree, the claim as its
/// message, a fixed identity, never signed.
pub fn commit_for(root: &Utf8Path, claim: &Claim) -> Result<String, String> {
    let message = serde_json::to_string(claim).map_err(|e| e.to_string())?;
    commit_of(root, &message)
}

/// The empty tree with `message` on it, under the fixed identity.
fn commit_of(root: &Utf8Path, message: &str) -> Result<String, String> {
    let tree = git(
        root,
        &["hash-object", "-t", "tree", "-w", "--stdin"],
        Some(b""),
    )?;
    if !tree.ok {
        return Err(first_line(&tree.stderr));
    }
    let tree = tree.stdout.trim().to_owned();
    let commit = git(
        root,
        &["commit-tree", "--no-gpg-sign", &tree, "-m", message],
        None,
    )?;
    if !commit.ok {
        return Err(first_line(&commit.stderr));
    }
    Ok(commit.stdout.trim().to_owned())
}

/// Where the tickets one clone announces to the others live (R2).
pub const TICKET_PREFIX: &str = "refs/openwarrant/tickets/";

/// [`commit_of`] with a tree: each `(name, text)` a blob at the tree's top,
/// `message` on the commit. What a ticket is announced as: its record in the
/// message, its document a blob any git can read.
pub fn commit_with_blobs(
    root: &Utf8Path,
    message: &str,
    blobs: &[(String, String)],
) -> Result<String, String> {
    let mut listing = String::new();
    for (name, text) in blobs {
        let blob = git(
            root,
            &["hash-object", "-w", "--stdin"],
            Some(text.as_bytes()),
        )?;
        if !blob.ok {
            return Err(first_line(&blob.stderr));
        }
        listing.push_str(&format!("100644 blob {}\t{name}\n", blob.stdout.trim()));
    }
    let tree = git(root, &["mktree"], Some(listing.as_bytes()))?;
    if !tree.ok {
        return Err(first_line(&tree.stderr));
    }
    let commit = git(
        root,
        &[
            "commit-tree",
            "--no-gpg-sign",
            tree.stdout.trim(),
            "-m",
            message,
        ],
        None,
    )?;
    if !commit.ok {
        return Err(first_line(&commit.stderr));
    }
    Ok(commit.stdout.trim().to_owned())
}

/// Every ref under `prefix` the remote holds, as `(commit, ref)`: one
/// `ls-remote`, inside a view's deadline.
pub fn list_refs(
    root: &Utf8Path,
    remote: &str,
    prefix: &str,
) -> Result<Vec<(String, String)>, String> {
    let pattern = format!("{prefix}*");
    let listed = git_within(root, &["ls-remote", remote, &pattern], None, VIEW_DEADLINE)?;
    if !listed.ok {
        return Err(first_line(&listed.stderr));
    }
    Ok(listed
        .stdout
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .filter(|(_, r)| r.starts_with(prefix))
        .map(|(c, r)| (c.to_owned(), r.to_owned()))
        .collect())
}

/// Fetch the commits of `refs` from `remote`, into no ref of this clone.
pub fn fetch_refs(root: &Utf8Path, remote: &str, refs: &[&str]) -> Result<(), String> {
    if refs.is_empty() {
        return Ok(());
    }
    let mut args = vec![
        "fetch",
        "--quiet",
        "--no-tags",
        "--no-write-fetch-head",
        remote,
    ];
    args.extend(refs);
    let fetched = git_within(root, &args, None, VIEW_DEADLINE)?;
    if fetched.ok {
        Ok(())
    } else {
        Err(first_line(&fetched.stderr))
    }
}

/// The message of `commit`, when this clone has it.
pub fn message_of(root: &Utf8Path, commit: &str) -> Result<Option<String>, String> {
    let shown = git(root, &["cat-file", "commit", commit], None)?;
    Ok(shown.ok.then(|| {
        shown
            .stdout
            .split_once("\n\n")
            .map_or("", |(_, m)| m)
            .trim()
            .to_owned()
    }))
}

/// The blob `name` at the top of `commit`'s tree.
pub fn blob_of(root: &Utf8Path, commit: &str, name: &str) -> Result<String, String> {
    let shown = git(
        root,
        &["cat-file", "blob", &format!("{commit}:{name}")],
        None,
    )?;
    if shown.ok {
        Ok(shown.stdout)
    } else {
        Err(first_line(&shown.stderr))
    }
}

/// Leave a notice for the other clones. A ref that is there already holds the
/// same change: it is named by it, and a lost race changes nothing.
pub fn publish_notice(root: &Utf8Path, remote: &str, key: &str, body: &str) -> Result<(), String> {
    let commit = commit_of(root, body)?;
    let refname = format!("{NOTICE_PREFIX}{key}");
    match push(root, remote, &refname, Some(&commit), None) {
        Push::Won | Push::Lost => Ok(()),
        Push::Failed(why) | Push::Unsettled(why) => Err(why),
    }
}

/// Every notice the remote holds: the message of each ref under
/// [`NOTICE_PREFIX`], fetching the commits this clone lacks.
pub fn read_notices(root: &Utf8Path, remote: &str) -> Result<Vec<String>, String> {
    let pattern = format!("{NOTICE_PREFIX}*");
    let listed = git(root, &["ls-remote", remote, &pattern], None)?;
    if !listed.ok {
        return Err(first_line(&listed.stderr));
    }
    let mut out = Vec::new();
    for line in listed.stdout.lines() {
        let Some((commit, refname)) = line.split_once('\t') else {
            continue;
        };
        let have = git(
            root,
            &["cat-file", "-e", &format!("{commit}^{{commit}}")],
            None,
        )?;
        if !have.ok {
            let fetched = git(
                root,
                &[
                    "fetch",
                    "--quiet",
                    "--no-tags",
                    "--no-write-fetch-head",
                    remote,
                    refname,
                ],
                None,
            )?;
            if !fetched.ok {
                continue;
            }
        }
        let shown = git(root, &["cat-file", "commit", commit], None)?;
        if shown.ok {
            let message = shown.stdout.split_once("\n\n").map_or("", |(_, m)| m);
            out.push(message.trim().to_owned());
        }
    }
    Ok(out)
}

/// One claim the remote holds: its lock name here, and the claim (`None`:
/// the ref's commit is not a claim, which holds all the same).
pub type RemoteClaim = (String, Option<Claim>);

/// Every claim the remote holds: one `ls-remote`, one fetch of the commits
/// this clone lacks. Each claim is marked as read from `remote`.
pub fn read_claims(root: &Utf8Path, remote: &str) -> Result<Vec<RemoteClaim>, String> {
    let pattern = format!("{REF_PREFIX}*");
    let listed = git_within(root, &["ls-remote", remote, &pattern], None, VIEW_DEADLINE)?;
    if !listed.ok {
        return Err(first_line(&listed.stderr));
    }
    let refs: Vec<(String, String)> = listed
        .stdout
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .filter(|(_, r)| r.starts_with(REF_PREFIX))
        .map(|(c, r)| (c.to_owned(), r.to_owned()))
        .collect();
    let mut missing = Vec::new();
    for (commit, refname) in &refs {
        let have = git(
            root,
            &["cat-file", "-e", &format!("{commit}^{{commit}}")],
            None,
        )?;
        if !have.ok {
            missing.push(refname.as_str());
        }
    }
    if !missing.is_empty() {
        let mut args = vec![
            "fetch",
            "--quiet",
            "--no-tags",
            "--no-write-fetch-head",
            remote,
        ];
        args.extend(missing);
        let fetched = git_within(root, &args, None, VIEW_DEADLINE)?;
        if !fetched.ok {
            return Err(first_line(&fetched.stderr));
        }
    }
    let mut out = Vec::new();
    for (commit, refname) in refs {
        let shown = git(root, &["cat-file", "commit", &commit], None)?;
        if !shown.ok {
            return Err(first_line(&shown.stderr));
        }
        let message = shown
            .stdout
            .split_once("\n\n")
            .map_or("", |(_, m)| m)
            .trim();
        let claim = serde_json::from_str::<Claim>(message).ok().map(|mut c| {
            c.elsewhere = Some(remote.to_owned());
            c
        });
        let name = format!("{}.lock", &refname[REF_PREFIX.len()..]);
        out.push((name, claim));
    }
    Ok(out)
}

/// The remote's claims as a view sees them: read at most once per `ttl`
/// seconds for every worktree of the clone, the answer kept under git's
/// common directory. A remote that could not be read is kept as such for as
/// long, so a dead remote costs one wait per `ttl`, not one per view.
pub fn seen_claims(
    root: &Utf8Path,
    common_dir: &Utf8Path,
    remote: &str,
    ttl: u64,
    now: u64,
) -> Result<Vec<RemoteClaim>, String> {
    #[derive(Serialize, Deserialize)]
    struct Seen {
        remote: String,
        read_unix: u64,
        #[serde(default)]
        claims: Vec<RemoteClaim>,
        #[serde(default)]
        unknown: Option<String>,
    }
    let file = common_dir.join(SEEN_FILE);
    let cached = std::fs::read(&file)
        .ok()
        .and_then(|b| serde_json::from_slice::<Seen>(&b).ok())
        .filter(|s| s.remote == remote && now >= s.read_unix && now - s.read_unix < ttl);
    let seen = cached.unwrap_or_else(|| {
        let read = read_claims(root, remote);
        let seen = Seen {
            remote: remote.to_owned(),
            read_unix: now,
            unknown: read.as_ref().err().cloned(),
            claims: read.unwrap_or_default(),
        };
        if let Ok(bytes) = serde_json::to_vec(&seen) {
            if let Some(dir) = file.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = crate::compile::atomic::write(&file, bytes);
        }
        seen
    });
    match seen.unknown {
        Some(why) => Err(why),
        None => Ok(seen.claims),
    }
}

/// Fetch `branch` from `remote` into a ref of this clone's own,
/// `refs/openwarrant/upstream/<branch>`, touching no branch or remote-tracking
/// ref of the person's. Returns its commit.
pub fn fetch_branch(root: &Utf8Path, remote: &str, branch: &str) -> Result<String, String> {
    let local = format!("refs/openwarrant/upstream/{branch}");
    let spec = format!("+refs/heads/{branch}:{local}");
    let fetched = git_within(
        root,
        &[
            "fetch",
            "--quiet",
            "--no-tags",
            "--no-write-fetch-head",
            remote,
            &spec,
        ],
        None,
        VIEW_DEADLINE,
    )?;
    if !fetched.ok {
        return Err(first_line(&fetched.stderr));
    }
    let sha = git(root, &["rev-parse", "--verify", "--quiet", &local], None)?;
    if !sha.ok {
        return Err(format!("{local} is not there after the fetch"));
    }
    Ok(sha.stdout.trim().to_owned())
}

/// What a push found.
#[derive(Debug)]
pub enum Push {
    /// The remote took it.
    Won,
    /// The ref was not what this machine expected: someone else moved it.
    Lost,
    /// The remote could not be reached, or refused for another reason.
    Failed(String),
    /// The push was not seen to finish (it ran past [`DEADLINE`]), and it may
    /// have landed: the remote can apply what it was sent after git is
    /// killed here. Whether the ref moved is unknown.
    Unsettled(String),
}

/// Set `refname` on `remote` to `commit` (or delete it, `commit` `None`) if
/// it is `expect` there now (`None`: absent).
pub fn push(
    root: &Utf8Path,
    remote: &str,
    refname: &str,
    commit: Option<&str>,
    expect: Option<&str>,
) -> Push {
    push_within(root, remote, refname, commit, expect, DEADLINE)
}

fn push_within(
    root: &Utf8Path,
    remote: &str,
    refname: &str,
    commit: Option<&str>,
    expect: Option<&str>,
    deadline: Duration,
) -> Push {
    let lease = format!("--force-with-lease={refname}:{}", expect.unwrap_or(""));
    let spec = format!("{}:{refname}", commit.unwrap_or(""));
    let ran = match git_within(
        root,
        &[
            "push",
            "--porcelain",
            "--atomic",
            "--no-verify",
            &lease,
            remote,
            &spec,
        ],
        None,
        deadline,
    ) {
        Ok(r) => r,
        Err(Unanswered::NotStarted(e)) => return Push::Failed(e),
        Err(Unanswered::Unfinished(e)) => return Push::Unsettled(e),
    };
    if ran.ok {
        // What the views read of the remote is stale now: read it again.
        if refname.starts_with(REF_PREFIX)
            && let Some(layout) = super::claim::git_layout(root)
        {
            let _ = std::fs::remove_file(layout.common_dir.join(SEEN_FILE));
        }
        return Push::Won;
    }
    // `--porcelain`: a line per ref, `!` when it was rejected. A stale lease,
    // or a ref another push holds locked, is a race lost, not a fault.
    let rejected = ran
        .stdout
        .lines()
        .any(|l| l.starts_with('!') && l.contains(refname));
    let said = format!("{}\n{}", ran.stdout, ran.stderr);
    let raced = [
        "stale info",
        "fetch first",
        "non-fast-forward",
        "already exists",
        "cannot lock ref",
        "failed to update ref",
        "failed to lock",
        "incorrect old value",
    ]
    .iter()
    .any(|s| said.contains(s));
    if rejected && raced {
        Push::Lost
    } else {
        Push::Failed(
            [first_line(&ran.stderr), first_line(&ran.stdout)]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("; "),
        )
    }
}

/// What the remote holds for a ref.
#[derive(Debug)]
pub enum Held {
    Absent,
    /// The commit, and the claim its message carries (`None`: not a claim).
    At(String, Option<Box<Claim>>),
}

/// Read `refname` on `remote`, fetching its commit when this clone lacks it.
pub fn read(root: &Utf8Path, remote: &str, refname: &str) -> Result<Held, String> {
    let listed = git(root, &["ls-remote", remote, refname], None)?;
    if !listed.ok {
        return Err(first_line(&listed.stderr));
    }
    let Some(commit) = listed
        .stdout
        .lines()
        .find_map(|l| l.split_once('\t').filter(|(_, r)| *r == refname))
        .map(|(sha, _)| sha.to_owned())
    else {
        return Ok(Held::Absent);
    };
    let have = git(
        root,
        &["cat-file", "-e", &format!("{commit}^{{commit}}")],
        None,
    )?;
    if !have.ok {
        let fetched = git(
            root,
            &[
                "fetch",
                "--quiet",
                "--no-tags",
                "--no-write-fetch-head",
                remote,
                refname,
            ],
            None,
        )?;
        if !fetched.ok {
            return Err(first_line(&fetched.stderr));
        }
    }
    let shown = git(root, &["cat-file", "commit", &commit], None)?;
    if !shown.ok {
        return Err(first_line(&shown.stderr));
    }
    let message = shown
        .stdout
        .split_once("\n\n")
        .map_or("", |(_, m)| m)
        .trim();
    Ok(Held::At(
        commit,
        serde_json::from_str::<Claim>(message).ok().map(Box::new),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_in(dir: &std::path::Path, args: &[&str]) {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}: {out:?}");
    }

    /// A clone whose remote answers a push by running `receive_pack`.
    fn clone_pushing_through(top: &std::path::Path, receive_pack: &str) -> (Utf8PathBuf, String) {
        use std::os::unix::fs::PermissionsExt as _;
        let bare = top.join("bare.git");
        let clone = top.join("clone");
        std::fs::create_dir(&bare).unwrap();
        std::fs::create_dir(&clone).unwrap();
        git_in(&bare, &["init", "-q", "--bare"]);
        git_in(&clone, &["init", "-q"]);
        git_in(&clone, &["remote", "add", "origin", bare.to_str().unwrap()]);
        git_in(&clone, &["commit", "-q", "--allow-empty", "-m", "c"]);
        let script = top.join("receive-pack");
        std::fs::write(&script, format!("#!/bin/sh\n{receive_pack}\n")).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        git_in(
            &clone,
            &[
                "config",
                "remote.origin.receivepack",
                script.to_str().unwrap(),
            ],
        );
        let head = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&clone)
            .output()
            .unwrap();
        (
            Utf8PathBuf::from_path_buf(clone).unwrap(),
            String::from_utf8(head.stdout).unwrap().trim().to_owned(),
        )
    }

    const REF: &str = "refs/openwarrant/claims/t-1";

    fn a_claim(actor: &str) -> Claim {
        Claim {
            schema: super::super::claim::CLAIM_SCHEMA.to_owned(),
            claim: format!("c-{actor}"),
            ticket: "t-1".to_owned(),
            item: Some("i-1".to_owned()),
            actor: actor.to_owned(),
            since: "2026-11-07T00:00:00Z".to_owned(),
            since_unix: 1,
            lease_until: None,
            lease_until_unix: Some(2),
            coordination: None,
            elsewhere: None,
        }
    }

    #[test]
    fn the_remotes_claims_are_read_marked_and_kept_for_their_ttl() {
        let top = tempfile::tempdir().unwrap();
        let (clone, _) = clone_pushing_through(top.path(), "exit 1");
        git_in(
            clone.as_std_path(),
            &["config", "--unset", "remote.origin.receivepack"],
        );
        let common = clone.join(".git");
        let refname = "refs/openwarrant/claims/t-1--i-1";
        let commit = commit_for(&clone, &a_claim("bob")).unwrap();
        assert!(matches!(
            push(&clone, "origin", refname, Some(&commit), None),
            Push::Won
        ));
        let seen = seen_claims(&clone, &common, "origin", 30, 100).unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, "t-1--i-1.lock");
        let c = seen[0].1.as_ref().unwrap();
        assert_eq!(
            (c.actor.as_str(), c.elsewhere.as_deref()),
            ("bob", Some("origin"))
        );
        // Another machine's push moves nothing until the TTL has passed.
        let other = commit_for(&clone, &a_claim("carol")).unwrap();
        let r2 = "refs/openwarrant/claims/t-2";
        Command::new("git")
            .args(["push", "-q", "origin", &format!("{other}:{r2}")])
            .current_dir(&clone)
            .output()
            .unwrap();
        assert_eq!(
            seen_claims(&clone, &common, "origin", 30, 120)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            seen_claims(&clone, &common, "origin", 30, 131)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn a_remote_that_cannot_be_read_is_unknown_not_empty() {
        let top = tempfile::tempdir().unwrap();
        let (clone, _) = clone_pushing_through(top.path(), "exit 1");
        let common = clone.join(".git");
        git_in(
            clone.as_std_path(),
            &[
                "remote",
                "set-url",
                "origin",
                "file:///nonexistent/war-no-remote.git",
            ],
        );
        let read = seen_claims(&clone, &common, "origin", 30, 100);
        assert!(read.is_err(), "{read:?}");
        // Kept as unknown for the TTL: a dead remote is one wait, not one per view.
        assert!(seen_claims(&clone, &common, "origin", 30, 110).is_err());
    }

    #[test]
    fn a_push_past_its_deadline_is_unsettled_not_failed() {
        let top = tempfile::tempdir().unwrap();
        // `exec`: the deadline's kill reaches the sleep, not a shell above it.
        let (clone, head) = clone_pushing_through(top.path(), "exec sleep 20");
        let started = Instant::now();
        let pushed = push_within(
            &clone,
            "origin",
            REF,
            Some(&head),
            None,
            Duration::from_millis(100),
        );
        assert!(matches!(pushed, Push::Unsettled(_)), "{pushed:?}");
        assert!(started.elapsed() < Duration::from_secs(15));
    }

    #[test]
    fn a_push_the_remote_answers_with_a_failure_stays_failed() {
        let top = tempfile::tempdir().unwrap();
        let (clone, head) = clone_pushing_through(top.path(), "exit 1");
        let pushed = push_within(
            &clone,
            "origin",
            REF,
            Some(&head),
            None,
            Duration::from_secs(20),
        );
        assert!(matches!(pushed, Push::Failed(_)), "{pushed:?}");
    }
}
