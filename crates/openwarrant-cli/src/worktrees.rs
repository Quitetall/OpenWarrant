// SPDX-License-Identifier: Apache-2.0
//! Conservative inventory and reclamation of tool-owned workspaces.
use crate::{
    go::git,
    repo::{RepoError, Repository},
    ticket::{self, Outcome},
};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};

const SCHEMA: &str = "oh.war/managed-workspace/v1";
/// A host that hides a same-user process cannot say a worktree is inactive.
/// Every reason that keeps a worktree for that alone starts with this.
pub(crate) const ACTIVITY_UNKNOWN: &str = "process activity unknown";
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Ownership {
    schema: String,
    id: String,
    path: String,
    branch: String,
    base: String,
    common: String,
}

fn registry(root: &Utf8Path) -> Result<(Utf8PathBuf, String), String> {
    let layout = ticket::claim::git_layout(root).ok_or("repository has no Git workspace layout")?;
    let common = layout
        .common_dir
        .canonicalize_utf8()
        .map_err(|e| e.to_string())?;
    Ok((common.join("openwarrant/workspaces"), common.to_string()))
}
/// The clone's git common directory, canonical: what every worktree of it
/// shares (`{common}` in `[resources.env]`).
#[must_use]
pub fn common_dir(root: &Utf8Path) -> Option<Utf8PathBuf> {
    ticket::claim::git_layout(root)?
        .common_dir
        .canonicalize_utf8()
        .ok()
}

fn marker(root: &Utf8Path, path: &Utf8Path) -> Result<Utf8PathBuf, String> {
    let (dir, _) = registry(root)?;
    Ok(dir.join(format!(
        "{}.json",
        openwarrant_compiler::sha256_hex(path.as_str().as_bytes())
    )))
}

fn local_marker(path: &Utf8Path) -> Result<Utf8PathBuf, String> {
    let result = git::git(path, &["rev-parse", "--absolute-git-dir"]);
    if !result.ok {
        return Err(result.why());
    }
    Ok(Utf8PathBuf::from(result.stdout.trim()).join("openwarrant-ownership.json"))
}

/// Called only after OpenWarrant creates the worktree. A name/prefix alone
/// never establishes ownership, and existing unrelated worktrees are not adopted.
pub fn register(root: &Utf8Path, path: &Utf8Path, branch: &str, base: &str) -> Result<(), String> {
    let path = path.canonicalize_utf8().map_err(|e| e.to_string())?;
    let (dir, common) = registry(root)?;
    let base = git::rev(root, base).ok_or("workspace base does not name an available commit")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let record = Ownership {
        schema: SCHEMA.into(),
        id: openwarrant_core::WarUuid::mint().to_string(),
        path: path.to_string(),
        branch: branch.into(),
        base,
        common,
    };
    let name = marker(root, &path)?;
    if name.exists() {
        let old: Ownership =
            serde_json::from_slice(&std::fs::read(&name).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if old.path == record.path
            && old.branch == record.branch
            && old.common == record.common
            && ownership(root, &path, Some(branch)).is_some()
        {
            return Ok(());
        }
        return Err("workspace ownership record does not match; retained".into());
    }
    use std::io::Write as _;
    let bytes = serde_json::to_vec(&record).map_err(|e| e.to_string())?;
    for target in [local_marker(&path)?, name] {
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&target)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub path: String,
    pub branch: Option<String>,
    pub head: String,
    pub base: Option<String>,
    pub managed: bool,
    pub claims: Vec<String>,
    pub prunable: bool,
    pub reasons: Vec<String>,
    /// What established inactivity where the process probe could not:
    /// `activity unknown; idle 3d, past 24h`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity: Option<String>,
}

/// Same-user process references are observed once per inventory, not once per
/// worktree. Inaccessible observations remain unknown and prevent deletion.
fn process_references() -> (BTreeSet<PathBuf>, bool) {
    let (paths, known) = observe_processes();
    // A debug build's hook: behave as a host that hides a process, whatever
    // this one does, so a plant is deterministic. What is seen still counts.
    #[cfg(debug_assertions)]
    if std::env::var("OPENWARRANT_TEST_PROCESS_PROBE").as_deref() == Ok("hidden") {
        return (paths, false);
    }
    (paths, known)
}

fn observe_processes() -> (BTreeSet<PathBuf>, bool) {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::MetadataExt;
        let mut paths = BTreeSet::new();
        let mut known = true;
        let Ok(processes) = std::fs::read_dir("/proc") else {
            return (paths, false);
        };
        let uid = rustix::process::geteuid().as_raw();
        for process in processes.flatten() {
            if process
                .file_name()
                .to_string_lossy()
                .parse::<u32>()
                .is_err()
                || process.file_name() == std::process::id().to_string().as_str()
            {
                continue;
            }
            if process.metadata().ok().is_none_or(|m| m.uid() != uid) {
                continue;
            }
            match std::fs::read_link(process.path().join("cwd")) {
                Ok(path) => {
                    paths.insert(path);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => known = false,
            }
            match std::fs::read_dir(process.path().join("fd")) {
                Ok(files) => {
                    for file in files.flatten() {
                        if let Ok(path) = std::fs::read_link(file.path())
                            && path.is_absolute()
                        {
                            paths.insert(path);
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => known = false,
            }
        }
        (paths, known)
    }
    #[cfg(target_os = "macos")]
    {
        let mut command = std::process::Command::new("/usr/sbin/lsof");
        command.args([
            "-nP",
            "-F0pn",
            "-a",
            "-u",
            &rustix::process::geteuid().as_raw().to_string(),
        ]);
        match crate::runtime_capture::process::observe_with_stdin(
            command,
            std::time::Duration::from_secs(10),
            8 * 1024 * 1024,
            std::process::Stdio::null(),
        ) {
            Ok(observed) if observed.success && observed.stderr.is_empty() => {
                let mut paths = BTreeSet::new();
                for field in observed.stdout.split(|b| matches!(*b, 0 | b'\n')) {
                    if let Some(name) = field.strip_prefix(b"n") {
                        let Ok(name) = std::str::from_utf8(name) else {
                            return (paths, false);
                        };
                        let path = PathBuf::from(name);
                        if path.is_absolute() {
                            paths.insert(path);
                        }
                    }
                }
                (paths, true)
            }
            _ => (BTreeSet::new(), false),
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        (BTreeSet::new(), false)
    }
}

pub fn is_managed(root: &Utf8Path, path: &Utf8Path, branch: &str) -> bool {
    path.canonicalize_utf8()
        .ok()
        .is_some_and(|path| ownership(root, &path, Some(branch)).is_some())
}

fn ownership(root: &Utf8Path, path: &Utf8Path, branch: Option<&str>) -> Option<Ownership> {
    let (_, common) = registry(root).ok()?;
    let name = marker(root, path).ok()?;
    let data = crate::vfs::read_regular(name.parent()?.as_std_path(), name.as_std_path()).ok()?;
    let record: Ownership = serde_json::from_slice(&data).ok()?;
    let local_path = local_marker(path).ok()?;
    let local =
        crate::vfs::read_regular(local_path.parent()?.as_std_path(), local_path.as_std_path())
            .ok()?;
    let local: Ownership = serde_json::from_slice(&local).ok()?;
    (record.schema == SCHEMA
        && record.id == local.id
        && record.path == local.path
        && record.branch == local.branch
        && record.base == local.base
        && record.common == local.common
        && record.path == path.as_str()
        && record.common == common
        && branch == Some(record.branch.as_str()))
    .then_some(record)
}

fn ignored_disposable(path: &Utf8Path, file: &str) -> bool {
    if file.starts_with("target/") {
        return std::fs::read_to_string(path.join("target/CACHEDIR.TAG"))
            .is_ok_and(|t| t.starts_with("Signature: 8a477f597d28d172789f06886806bc55"));
    }
    // A session's own files, and the output of checks run there, whose
    // digests are in the journal.
    if file == ".openwarrant/session.json"
        || file == ".openwarrant/start.log"
        || file == ".openwarrant/go-packet.json"
        || file == crate::resources::ENV_FILE
        || file == crate::ticket::next_file::FILE
        || file.starts_with(".openwarrant/state/checks/")
    {
        return true;
    }
    if file == ".claude/settings.local.json" {
        let session = std::fs::read_to_string(path.join(".openwarrant/session.json"))
            .ok()
            .and_then(|s| serde_json::from_str::<crate::go::start::Session>(&s).ok());
        return session.and_then(|s| s.settings_sha256).is_some_and(|sha| {
            std::fs::read(path.join(file))
                .is_ok_and(|b| openwarrant_compiler::sha256_hex(&b) == sha)
        });
    }
    false
}

/// `[resources] idle_after`: seconds and as written; `None` when it is 0.
type Idle = Result<Option<(u64, String)>, String>;

fn idle_policy(root: &Utf8Path) -> Idle {
    let config = crate::resources::load(root)?;
    let text = config
        .idle_after
        .clone()
        .unwrap_or_else(|| crate::resources::IDLE_AFTER.into());
    Ok(config.idle_after()?.map(|s| (s, text)))
}

fn age(secs: u64) -> String {
    match secs {
        86_400.. => format!("{}d", secs / 86_400),
        3600.. => format!("{}h", secs / 3600),
        _ => format!("{}m", secs / 60),
    }
}

/// The newest modification time of anything in the worktree but its `.git`,
/// the worktree directory included; symlinks are not followed.
fn newest_change(path: &Utf8Path) -> Result<(std::time::SystemTime, String), String> {
    let mut newest = (
        std::fs::symlink_metadata(path)
            .and_then(|m| m.modified())
            .map_err(|e| e.to_string())?,
        ".".to_owned(),
    );
    let mut stack = vec![path.as_std_path().to_owned()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let p = entry.path();
            if dir == path.as_std_path() && entry.file_name() == ".git" {
                continue;
            }
            let meta =
                std::fs::symlink_metadata(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            let modified = meta.modified().map_err(|e| e.to_string())?;
            if modified > newest.0 {
                let rel = p.strip_prefix(path.as_std_path()).unwrap_or(&p);
                newest = (modified, rel.to_string_lossy().into_owned());
            }
            if meta.is_dir() {
                stack.push(p);
            }
        }
    }
    Ok(newest)
}

/// Where the process probe is UNKNOWN: idle when nothing in the worktree
/// changed for `idle_after`. `Ok` says it was; `Err` says why it is kept.
fn idle_signal(path: &Utf8Path, idle: &Idle) -> Result<String, String> {
    let (after, text) = match idle {
        Ok(Some(policy)) => policy,
        Ok(None) => return Err(format!("{ACTIVITY_UNKNOWN}; idle_after 0")),
        Err(why) => return Err(format!("{ACTIVITY_UNKNOWN}; {why}")),
    };
    let (when, file) =
        newest_change(path).map_err(|e| format!("{ACTIVITY_UNKNOWN}; file times UNKNOWN: {e}"))?;
    let Ok(since) = std::time::SystemTime::now().duration_since(when) else {
        return Err(format!("{ACTIVITY_UNKNOWN}; {file} changed after now"));
    };
    let since = since.as_secs();
    if since > *after {
        Ok(format!(
            "activity unknown; idle {}, past {text}",
            age(since)
        ))
    } else {
        Err(format!(
            "{ACTIVITY_UNKNOWN}; {file} changed {} ago, within idle_after {text}",
            age(since)
        ))
    }
}

#[allow(clippy::too_many_arguments)]
fn inspect(
    repo: &Repository,
    path: &Utf8Path,
    branch: Option<String>,
    head: String,
    store: &ticket::Store,
    activity: &(BTreeSet<PathBuf>, bool),
    idle: Option<&Idle>,
    integration: &str,
) -> Row {
    let (references, known) = activity;
    let own = ownership(&repo.root, path, branch.as_deref());
    let mut row = Row {
        path: path.to_string(),
        branch,
        head,
        base: own.as_ref().map(|o| o.base.clone()),
        managed: own.is_some(),
        claims: Vec::new(),
        prunable: false,
        reasons: Vec::new(),
        activity: None,
    };
    if own.is_none() {
        row.reasons.push("foreign or ownership unknown".into());
    }
    if path.canonicalize_utf8().ok() == repo.root.canonicalize_utf8().ok() {
        row.reasons.push("current worktree".into());
    }
    if path.join(".git").is_dir() {
        row.reasons.push("primary checkout".into());
    }
    let status = git::git(path, &["status", "--porcelain=v1", "--untracked-files=all"]);
    if !status.ok || !status.stdout.trim().is_empty() {
        row.reasons.push("dirty or unreadable worktree".into());
    }
    let ignored = git::git(
        path,
        &["ls-files", "--others", "--ignored", "--exclude-standard"],
    );
    if !ignored.ok || ignored.stdout.lines().any(|p| !ignored_disposable(path, p)) {
        row.reasons
            .push("ignored non-disposable files or unreadable ignored state".into());
    }
    if !git::git(
        &repo.root,
        &["merge-base", "--is-ancestor", &row.head, integration],
    )
    .ok
    {
        row.reasons
            .push("unique commits or integration revision unknown".into());
    }
    match store.claims() {
        Ok(claims) => {
            if claims.values().any(Option::is_none) {
                row.reasons.push("unreadable claim retained".into());
            }
            for claim in claims.values().flatten() {
                if claim
                    .coordination
                    .as_ref()
                    .is_some_and(|c| Utf8Path::new(&c.workspace).starts_with(path))
                {
                    row.claims
                        .push(format!("{}: {}", claim.actor, claim.target()));
                }
            }
        }
        Err(_) => row.reasons.push("claim inventory unknown".into()),
    }
    if !row.claims.is_empty() {
        row.reasons
            .push("writer claim retained; expiry alone does not establish inactivity".into());
    }
    let seen = references.iter().any(|p| p.starts_with(path.as_std_path()));
    if !*known {
        // The second signal is asked only of a worktree nothing else keeps.
        match idle.filter(|_| row.reasons.is_empty() && !seen) {
            Some(idle) => match idle_signal(path, idle) {
                Ok(signal) => row.activity = Some(signal),
                Err(why) => row.reasons.push(why),
            },
            None => row.reasons.push(ACTIVITY_UNKNOWN.into()),
        }
    }
    if seen {
        row.reasons.push("active process reference".into());
    }
    row.prunable = row.reasons.is_empty();
    row
}

pub fn inventory(repo: &Repository, probe: bool, integration: &str) -> Result<Vec<Row>, RepoError> {
    let store = ticket::Store::open(repo, None)?;
    let listed = git::git(&repo.root, &["worktree", "list", "--porcelain"]);
    if !listed.ok {
        return Err(RepoError::Message(format!(
            "worktrees.inventory: {}",
            listed.why()
        )));
    }
    let activity = if probe {
        process_references()
    } else {
        (BTreeSet::new(), false)
    };
    // Unprobed is not "hidden": the idle fallback answers only a real probe.
    let idle = probe.then(|| idle_policy(&repo.root));
    let mut rows = Vec::new();
    for block in listed.stdout.split("\n\n") {
        let value = |key: &str| {
            block
                .lines()
                .find_map(|l| l.strip_prefix(key))
                .map(str::to_owned)
        };
        let Some(path) = value("worktree ") else {
            continue;
        };
        let path = Utf8PathBuf::from(path);
        rows.push(inspect(
            repo,
            &path,
            value("branch ").map(|b| b.trim_start_matches("refs/heads/").to_owned()),
            value("HEAD ").unwrap_or_default(),
            &store,
            &activity,
            idle.as_ref(),
            integration,
        ));
    }
    Ok(rows)
}

/// Cleanup of a worktree someone else may be in. Never force, recursively
/// delete, or discard a branch with unique commits. Unknown state is retained.
pub fn remove_if_safe(root: &Utf8Path, path: &Utf8Path, integration: &str) -> bool {
    reclaim(root, path, integration, |row| row.prunable)
}

/// Cleanup by the run that made a worktree, once what it started there has
/// ended. Every rule of [`remove_if_safe`] holds but one: a process the host
/// hides is not a reason to keep it. The run knows what it started, a process
/// it sees in the worktree still retains it, and what is removed is clean and
/// reachable at `reachable_at`. Where that hidden process is a signing agent,
/// the rule would otherwise retain every worktree a run ever made.
pub fn remove_own(root: &Utf8Path, path: &Utf8Path, reachable_at: &str) -> bool {
    reclaim(root, path, reachable_at, |row| {
        row.reasons.iter().all(|r| r.starts_with(ACTIVITY_UNKNOWN))
    })
}

fn reclaim(
    root: &Utf8Path,
    path: &Utf8Path,
    integration: &str,
    removable: impl Fn(&Row) -> bool,
) -> bool {
    let Ok(path) = path.canonicalize_utf8() else {
        return false;
    };
    let path = path.as_path();
    let Ok(repo) = Repository::open(root.to_owned()) else {
        return false;
    };
    let Ok(rows) = inventory(&repo, true, integration) else {
        return false;
    };
    if !rows.iter().any(|r| r.path == path.as_str() && removable(r)) {
        return false;
    }
    let result = git::git(root, &["worktree", "remove", path.as_str()]);
    if !result.ok {
        return false;
    }
    if let Ok(name) = marker(root, path) {
        // Retain the generation's ownership evidence without adopting a future
        // checkout created at the same path.
        let archive = name.with_extension(format!(
            "retired-{}.json",
            openwarrant_core::WarUuid::mint()
        ));
        let _ = std::fs::rename(name, archive);
    }
    true
}

pub fn run(
    repo: &Repository,
    prune: bool,
    apply: bool,
    integration: &str,
) -> Result<Outcome, RepoError> {
    let rows = inventory(repo, prune, integration)?;
    let mut removed = Vec::new();
    let mut retained = Vec::new();
    if apply && !prune {
        return Ok(Outcome::refused(
            "worktrees.apply-needs-prune",
            String::new(),
            "use --prune --apply after inspecting the dry run",
        ));
    }
    if prune && apply {
        for row in rows.iter().filter(|r| r.prunable) {
            // Recheck every condition immediately before a non-forced removal.
            let current = inventory(repo, true, integration)?
                .into_iter()
                .find(|r| r.path == row.path);
            if current.is_none_or(|r| !r.prunable || r.head != row.head) {
                retained.push(row.path.clone());
                continue;
            }
            if remove_if_safe(&repo.root, Utf8Path::new(&row.path), integration) {
                removed.push(row.path.clone());
            } else {
                retained.push(row.path.clone());
            }
        }
    }
    let human = rows
        .iter()
        .map(|r| {
            format!(
                "{}  {}  {}",
                if r.prunable { "eligible" } else { "retain" },
                r.path,
                r.activity
                    .as_ref()
                    .map_or_else(|| r.reasons.join("; "), Clone::clone)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Outcome::ok(
        human,
        serde_json::json!({"schema":"oh.war/worktrees/v1","dry_run":!apply,"integration":integration,"worktrees":rows,"removed":removed,"retained_after_recheck":retained}),
    ))
}

#[cfg(test)]
mod idle_tests {
    use super::*;

    #[test]
    fn an_aged_tree_is_idle_and_one_fresh_file_or_zero_keeps_it() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_owned()).unwrap();
        std::fs::create_dir(root.join("src")).unwrap();
        std::fs::write(root.join("src/a.rs"), b"a").unwrap();
        std::fs::write(root.join(".git"), b"gitdir: x").unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3 * 86_400);
        for p in ["src/a.rs", "src", "."] {
            std::fs::File::options()
                .read(true)
                .open(root.join(p))
                .and_then(|f| f.set_modified(old))
                .unwrap();
        }
        let day: Idle = Ok(Some((86_400, "24h".into())));
        assert_eq!(
            idle_signal(&root, &day).unwrap(),
            "activity unknown; idle 3d, past 24h",
            "a fresh .git is not activity"
        );
        assert_eq!(
            idle_signal(&root, &Ok(None)).unwrap_err(),
            "process activity unknown; idle_after 0"
        );
        std::fs::write(root.join("src/a.rs"), b"b").unwrap();
        let kept = idle_signal(&root, &day).unwrap_err();
        assert!(kept.starts_with(ACTIVITY_UNKNOWN), "{kept}");
        assert!(
            kept.contains("src/a.rs changed 0m ago, within idle_after 24h"),
            "{kept}"
        );
        assert!(
            idle_signal(&root.join("absent"), &day)
                .unwrap_err()
                .contains("file times UNKNOWN")
        );
    }
}
