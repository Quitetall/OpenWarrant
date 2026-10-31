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
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let record = Ownership {
        schema: SCHEMA.into(),
        id: openwarrant_core::WarUuid::mint().to_string(),
        path: path.to_string(),
        branch: branch.into(),
        base: base.into(),
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
}

/// Same-user process references are observed once per inventory, not once per
/// worktree. Inaccessible observations remain unknown and prevent deletion.
fn process_references() -> (BTreeSet<PathBuf>, bool) {
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
    #[cfg(not(target_os = "linux"))]
    {
        (BTreeSet::new(), false)
    }
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
    if file == ".openwarrant/session.json" {
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

fn inspect(
    repo: &Repository,
    path: &Utf8Path,
    branch: Option<String>,
    head: String,
    store: &ticket::Store,
    references: &BTreeSet<PathBuf>,
    known: bool,
    integration: &str,
) -> Row {
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
                    .is_some_and(|c| c.workspace == path.as_str())
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
    if !known {
        row.reasons.push("process activity unknown".into());
    }
    if references.iter().any(|p| p.starts_with(path.as_std_path())) {
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
    let (refs, known) = if probe {
        process_references()
    } else {
        (BTreeSet::new(), false)
    };
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
            &refs,
            known,
            integration,
        ));
    }
    Ok(rows)
}

/// Cleanup used by the execution lifecycle too. Never force, recursively
/// delete, or discard a branch with unique commits. Unknown state is retained.
pub fn remove_if_safe(root: &Utf8Path, path: &Utf8Path, integration: &str) -> bool {
    let Ok(repo) = Repository::open(root.to_owned()) else {
        return false;
    };
    let Ok(rows) = inventory(&repo, true, integration) else {
        return false;
    };
    if !rows.iter().any(|r| r.path == path.as_str() && r.prunable) {
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
                r.reasons.join("; ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Outcome::ok(
        human,
        serde_json::json!({"schema":"oh.war/worktrees/v1","dry_run":!apply,"integration":integration,"worktrees":rows,"removed":removed,"retained_after_recheck":retained}),
    ))
}
