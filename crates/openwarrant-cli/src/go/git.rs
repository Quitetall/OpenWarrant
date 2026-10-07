// SPDX-License-Identifier: Apache-2.0
//! The git `war evidence go` and `war start` run (OW-WAR-0148 M15): worktrees,
//! commits, merges and compare-and-set ref updates.
//!
//! Every call is an argv, never a shell string; none prompts (no terminal,
//! no askpass), none runs a hook it was not asked to, and none signs:
//! `commit.gpgSign`, `tag.gpgSign` and `merge.verifySignatures` are forced
//! off on every call, whatever the repository's configuration says, because
//! a scheduler that reached a signing key would be a signature nobody gave.

use std::process::{Command, Stdio};

use camino::{Utf8Path, Utf8PathBuf};

/// One git call's result.
#[derive(Debug)]
pub struct Ran {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

impl Ran {
    /// The first line of stderr, else of stdout, for a message.
    #[must_use]
    pub fn why(&self) -> String {
        self.stderr
            .lines()
            .chain(self.stdout.lines())
            .find(|l| !l.trim().is_empty())
            .unwrap_or("git said nothing")
            .trim()
            .to_owned()
    }
}

/// Run git in `dir`.
pub fn git(dir: &Utf8Path, args: &[&str]) -> Ran {
    git_env(dir, args, &[])
}

/// Run git in `dir` with extra environment.
pub fn git_env(dir: &Utf8Path, args: &[&str], env: &[(&str, &str)]) -> Ran {
    let mut cmd = Command::new("git");
    cmd.args([
        "-c",
        "commit.gpgSign=false",
        "-c",
        "tag.gpgSign=false",
        "-c",
        "merge.verifySignatures=false",
        "-c",
        "core.askPass=",
        "-c",
        "credential.interactive=never",
    ])
    .args(args)
    .current_dir(dir)
    .env("GIT_TERMINAL_PROMPT", "0")
    .env_remove("GIT_DIR")
    .env_remove("GIT_WORK_TREE")
    .env_remove("GIT_INDEX_FILE")
    .stdin(Stdio::null());
    for (k, v) in env {
        cmd.env(k, v);
    }
    match cmd.output() {
        Ok(out) => Ran {
            ok: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        },
        Err(e) => Ran {
            ok: false,
            stdout: String::new(),
            stderr: format!("could not run git: {e}"),
        },
    }
}

/// `git check-ref-format --branch`, without running git: the subset of the
/// rules a name written in a config file can break.
#[must_use]
pub fn is_branch_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(['-', '/'])
        && !name.ends_with(['/', '.'])
        && !name.ends_with(".lock")
        && !name.contains("..")
        && !name.contains("//")
        && !name.contains("@{")
        && !name
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || "~^:?*[\\".contains(c))
}

/// The branch a node's work is on: `war-go/<ticket>--<item>`, one level
/// below `war-go/` so it never collides with the integration branch or with
/// another node.
#[must_use]
pub fn node_branch(node: &str) -> String {
    format!("war-go/{}", slug(node))
}

/// A node id as one path segment: `t-x/i-y` → `t-x--i-y`.
#[must_use]
pub fn slug(node: &str) -> String {
    node.replace('/', "--")
}

/// The checkout's top level, when it is a git checkout.
#[must_use]
pub fn toplevel(dir: &Utf8Path) -> Option<Utf8PathBuf> {
    let r = git(dir, &["rev-parse", "--show-toplevel"]);
    r.ok.then(|| Utf8PathBuf::from(r.stdout.trim()))
}

/// The clone's common directory (shared by its worktrees), absolute.
#[must_use]
pub fn common_dir(dir: &Utf8Path) -> Option<Utf8PathBuf> {
    let r = git(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    );
    r.ok.then(|| Utf8PathBuf::from(r.stdout.trim()))
}

/// `rev` as a commit id, when it names one.
#[must_use]
pub fn rev(dir: &Utf8Path, rev: &str) -> Option<String> {
    let spec = format!("{rev}^{{commit}}");
    let r = git(dir, &["rev-parse", "--verify", "--quiet", &spec]);
    r.ok.then(|| r.stdout.trim().to_owned())
        .filter(|s| !s.is_empty())
}

/// The commit `refs/heads/<branch>` points at.
#[must_use]
pub fn branch_tip(dir: &Utf8Path, branch: &str) -> Option<String> {
    rev(dir, &format!("refs/heads/{branch}"))
}

/// Whether `a` is an ancestor of `b` (or is `b`).
#[must_use]
pub fn is_ancestor(dir: &Utf8Path, a: &str, b: &str) -> bool {
    git(dir, &["merge-base", "--is-ancestor", a, b]).ok
}

/// Make `branch` at HEAD when it does not exist, and return its tip.
pub fn ensure_branch(dir: &Utf8Path, branch: &str) -> Result<String, String> {
    if let Some(tip) = branch_tip(dir, branch) {
        return Ok(tip);
    }
    let head = rev(dir, "HEAD").ok_or_else(|| {
        "this checkout has no commit yet; a worktree is made from a commit".to_owned()
    })?;
    let r = git(dir, &["branch", branch, &head]);
    if !r.ok {
        return Err(format!("could not create branch {branch}: {}", r.why()));
    }
    Ok(head)
}

/// Add a worktree at `path` on a new branch `branch` from `base`. A worktree
/// or branch of that name left by an earlier attempt is removed first: an
/// attempt starts from the integration branch, never from a dead one.
pub fn add_worktree(
    root: &Utf8Path,
    path: &Utf8Path,
    branch: &str,
    base: &str,
) -> Result<(), String> {
    remove_worktree(root, path, Some(branch));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("could not create {parent}: {e}"))?;
    }
    let r = git(
        root,
        &["worktree", "add", "-q", "-b", branch, path.as_str(), base],
    );
    if !r.ok {
        return Err(format!("could not add the worktree {path}: {}", r.why()));
    }
    Ok(())
}

/// Add a worktree at `path` on an existing `branch`.
pub fn add_worktree_on(root: &Utf8Path, path: &Utf8Path, branch: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("could not create {parent}: {e}"))?;
    }
    let r = git(root, &["worktree", "add", "-q", path.as_str(), branch]);
    if !r.ok {
        return Err(format!("could not add the worktree {path}: {}", r.why()));
    }
    Ok(())
}

/// Remove the worktree at `path` (forcefully: its work has landed or been
/// given up) and, when named, its branch. Best effort: what is already gone
/// is fine.
pub fn remove_worktree(root: &Utf8Path, path: &Utf8Path, branch: Option<&str>) {
    if path.exists() {
        let _ = git(
            root,
            &["worktree", "remove", "--force", "--force", path.as_str()],
        );
        if path.exists() {
            let _ = std::fs::remove_dir_all(path);
        }
    }
    let _ = git(root, &["worktree", "prune"]);
    if let Some(b) = branch
        && branch_tip(root, b).is_some()
    {
        let _ = git(root, &["branch", "-D", b]);
    }
}

/// The identity a commit by the scheduler is written under: the
/// repository's own `user.name`/`user.email` when it has them, else `war go`.
fn identity(dir: &Utf8Path) -> Vec<(&'static str, String)> {
    let get = |k: &str| {
        let r = git(dir, &["config", "--get", k]);
        r.ok.then(|| r.stdout.trim().to_owned())
            .filter(|s| !s.is_empty())
    };
    let mut env = Vec::new();
    if get("user.name").is_none() {
        env.push(("GIT_AUTHOR_NAME", "war go".to_owned()));
        env.push(("GIT_COMMITTER_NAME", "war go".to_owned()));
    }
    if get("user.email").is_none() {
        env.push(("GIT_AUTHOR_EMAIL", "war-go@openwarrant.invalid".to_owned()));
        env.push((
            "GIT_COMMITTER_EMAIL",
            "war-go@openwarrant.invalid".to_owned(),
        ));
    }
    env
}

/// Commit everything in the worktree. The files a session writes for itself
/// are in the clone's `info/exclude` ([`exclude`]), so they are never
/// staged; any of them already staged by hand is unstaged here. `Ok(None)`
/// when there was nothing to commit.
pub fn commit_all(wt: &Utf8Path, message: &str, skip: &[&str]) -> Result<Option<String>, String> {
    let r = git(wt, &["add", "-A"]);
    if !r.ok {
        return Err(format!("git add: {}", r.why()));
    }
    for s in skip {
        let _ = git(
            wt,
            &["rm", "-r", "-q", "--cached", "--ignore-unmatch", "--", s],
        );
    }
    if git(wt, &["diff", "--cached", "--quiet"]).ok {
        return Ok(None);
    }
    let env = identity(wt);
    let env: Vec<(&str, &str)> = env.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let r = git_env(wt, &["commit", "-q", "--no-gpg-sign", "-m", message], &env);
    if !r.ok {
        return Err(format!("git commit: {}", r.why()));
    }
    Ok(rev(wt, "HEAD"))
}

/// Merge `what` into the worktree's branch. `Err(conflicted files)` when it
/// does not merge cleanly; the merge is aborted and the worktree is left as
/// it was.
pub fn merge(wt: &Utf8Path, what: &str, message: &str) -> Result<(), Vec<String>> {
    let env = identity(wt);
    let env: Vec<(&str, &str)> = env.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let r = git_env(
        wt,
        &[
            "merge",
            "--no-ff",
            "--no-edit",
            "--no-gpg-sign",
            "-m",
            message,
            what,
        ],
        &env,
    );
    if r.ok {
        return Ok(());
    }
    let unmerged = git(wt, &["diff", "--name-only", "--diff-filter=U"]);
    let mut files: Vec<String> = unmerged
        .stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect();
    if files.is_empty() {
        files.push(r.why());
    }
    let _ = git(wt, &["merge", "--abort"]);
    Err(files)
}

/// Move `refs/heads/<branch>` from `old` to `new`, only if it is still at
/// `old`: the compare-and-set that lands work.
pub fn update_branch(dir: &Utf8Path, branch: &str, new: &str, old: &str) -> Result<(), String> {
    let refname = format!("refs/heads/{branch}");
    let r = git(
        dir,
        &["update-ref", "-m", "war go: land", &refname, new, old],
    );
    if r.ok { Ok(()) } else { Err(r.why()) }
}

/// Add `pattern` to the clone's `info/exclude` when it is not there: a
/// worktree under the root, or a file a session writes for itself, never
/// reads as a change to the checkout.
pub fn exclude(dir: &Utf8Path, pattern: &str) {
    let Some(common) = common_dir(dir) else {
        return;
    };
    let path = common.join("info").join("exclude");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    if text.lines().any(|l| l.trim() == pattern) {
        return;
    }
    let _ = std::fs::create_dir_all(common.join("info"));
    let mut next = text;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str(pattern);
    next.push('\n');
    let _ = crate::compile::atomic::write(&path, next.into_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_branches_are_one_level_below_war_go() {
        assert_eq!(node_branch("t-3f2a/i-9c01"), "war-go/t-3f2a--i-9c01");
        assert_eq!(
            node_branch("OW-WAR-0001/STAGE-001"),
            "war-go/OW-WAR-0001--STAGE-001"
        );
        assert!(is_branch_name(&node_branch("t-3f2a/i-9c01")));
    }

    #[test]
    fn a_name_git_refuses_is_refused() {
        for bad in [
            "", "-x", "a..b", "a b", "a~1", "a:b", "x.lock", "a/", "@{u}",
        ] {
            assert!(!is_branch_name(bad), "{bad:?}");
        }
        assert!(is_branch_name("war-go/integration"));
    }
}
