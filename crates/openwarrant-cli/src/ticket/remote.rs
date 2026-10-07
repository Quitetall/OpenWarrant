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
//! lock it took. Finishing or releasing deletes the ref, again with a lease,
//! so a claim reclaimed meanwhile is never deleted by its previous holder.
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

/// How long one git call to the remote may take before it reads as
/// unreachable.
const DEADLINE: Duration = Duration::from_secs(30);

/// `[claims]` in `openwarrant.toml`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// The git remote claims are published to; absent, claims stay on this
    /// machine.
    #[serde(default)]
    pub remote: Option<String>,
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

/// One git call: its status and output, or why it could not finish.
struct Ran {
    ok: bool,
    stdout: String,
    stderr: String,
}

fn git(root: &Utf8Path, args: &[&str], stdin: Option<&[u8]>) -> Result<Ran, String> {
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
        .map_err(|e| format!("could not run git: {e}"))?;
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
            .map_err(|e| format!("could not wait for git: {e}"))?
        {
            break s;
        }
        if started.elapsed() > DEADLINE {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "git {} took more than {}s",
                args.first().unwrap_or(&""),
                DEADLINE.as_secs()
            ));
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
    let tree = git(
        root,
        &["hash-object", "-t", "tree", "-w", "--stdin"],
        Some(b""),
    )?;
    if !tree.ok {
        return Err(first_line(&tree.stderr));
    }
    let tree = tree.stdout.trim().to_owned();
    let message = serde_json::to_string(claim).map_err(|e| e.to_string())?;
    let commit = git(
        root,
        &["commit-tree", "--no-gpg-sign", &tree, "-m", &message],
        None,
    )?;
    if !commit.ok {
        return Err(first_line(&commit.stderr));
    }
    Ok(commit.stdout.trim().to_owned())
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
    let lease = format!("--force-with-lease={refname}:{}", expect.unwrap_or(""));
    let spec = format!("{}:{refname}", commit.unwrap_or(""));
    let ran = match git(
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
    ) {
        Ok(r) => r,
        Err(e) => return Push::Failed(e),
    };
    if ran.ok {
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
