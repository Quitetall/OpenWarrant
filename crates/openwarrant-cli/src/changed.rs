// SPDX-License-Identifier: Apache-2.0
//! `war check --changed`: what the working tree changed, in Warrants.
//!
//! An end-of-turn hook asked `war check` over the whole corpus every time an
//! agent stopped: seconds of work and, before the brief rendering, hundreds
//! of kilobytes it then had to read past. What the agent can have broken is
//! what it changed. Git is asked, over the paths OpenWarrant governs and no
//! other (issue #365: a pathspec left empty reads the whole tree), which of
//! them differ from `HEAD` or are new. A Warrant's own record directory or
//! a file one of its deliverables pins puts that Warrant in scope; a change
//! to what every Warrant reads (the SAS, the roadmap, ADRs, gates, the
//! authority register, `openwarrant.toml`) puts the corpus in scope.

use std::collections::{BTreeMap, BTreeSet};

use crate::repo::{RepoError, Repository};

/// What a change needs checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Nothing OpenWarrant governs differs from `HEAD`.
    Nothing,
    /// These Warrants' records or pinned files changed, and nothing shared.
    Warrants(BTreeSet<String>),
    /// Something every Warrant reads changed, or git could not say.
    Corpus(String),
}

/// The trees and files every Warrant reads.
fn shared_paths(repo: &Repository) -> Vec<String> {
    let p = &repo.config.paths;
    let mut out = vec![
        p.sas.clone(),
        p.roadmap.clone(),
        p.adrs.clone(),
        p.gates.clone(),
        "docs/authority".to_owned(),
        crate::init::CONFIG_FILE.to_owned(),
    ];
    out.retain(|s| !s.trim_matches('/').is_empty());
    out
}

fn git_names(repo: &Repository, args: &[&str], specs: &[String]) -> Option<Vec<String>> {
    let out = std::process::Command::new("git")
        .args(args)
        .arg("--")
        .args(specs.iter().map(|s| format!(":(literal){s}")))
        .current_dir(&repo.root)
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    out.status.success().then(|| {
        String::from_utf8_lossy(&out.stdout)
            .split('\0')
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect()
    })
}

fn under(path: &str, dir: &str) -> bool {
    let dir = dir.trim_end_matches('/');
    path == dir || path.strip_prefix(dir).is_some_and(|r| r.starts_with('/'))
}

/// Which Warrants, or whether the whole corpus, the working tree's
/// difference from `HEAD` puts in scope.
pub fn scope(repo: &Repository) -> Result<Scope, RepoError> {
    let has_head = std::process::Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", "HEAD"])
        .current_dir(&repo.root)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if !has_head {
        return Ok(Scope::Corpus(
            "no commit to compare against, so every Warrant is new".to_owned(),
        ));
    }
    let mut pinned: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for pin in crate::pins::list(repo, false)?.pins {
        let owner = match &pin.governed_by {
            Some(g) => g.split('/').next().unwrap_or(g).to_owned(),
            None => pin.warrant.clone(),
        };
        pinned.entry(pin.path).or_default().insert(owner);
    }
    let warrants = repo.config.paths.warrants.trim_end_matches('/').to_owned();
    let shared = shared_paths(repo);
    let mut specs: Vec<String> = shared.clone();
    specs.push(warrants.clone());
    specs.extend(pinned.keys().cloned());
    specs.sort();
    specs.dedup();

    let tracked = git_names(
        repo,
        &["diff", "--name-only", "--no-renames", "-z", "HEAD"],
        &specs,
    );
    let untracked = git_names(
        repo,
        &["ls-files", "--others", "--exclude-standard", "-z"],
        &specs,
    );
    let (Some(tracked), Some(untracked)) = (tracked, untracked) else {
        return Ok(Scope::Corpus(
            "git could not say what changed, so nothing is assumed unchanged".to_owned(),
        ));
    };

    let mut touched: BTreeSet<String> = BTreeSet::new();
    for path in tracked.iter().chain(&untracked) {
        if let Some(dir) = shared.iter().find(|d| under(path, d)) {
            return Ok(Scope::Corpus(format!(
                "{path} changed, and every Warrant reads {dir}"
            )));
        }
        if let Some(rest) = path
            .strip_prefix(&warrants)
            .and_then(|r| r.strip_prefix('/'))
            && let Some((alias, _)) = rest.split_once('/')
            && repo
                .warrants_dir()
                .join(alias)
                .join("manifest.toml")
                .is_file()
        {
            touched.insert(alias.to_owned());
        }
        if let Some(owners) = pinned.get(path) {
            touched.extend(owners.iter().cloned());
        }
    }
    Ok(if touched.is_empty() {
        Scope::Nothing
    } else {
        Scope::Warrants(touched)
    })
}

#[cfg(test)]
mod tests {
    use super::under;

    #[test]
    fn a_path_is_under_a_directory_only_at_a_segment_boundary() {
        assert!(under("docs/sas/SAS.md", "docs/sas"));
        assert!(under("docs/sas", "docs/sas/"));
        assert!(!under("docs/sas-notes.md", "docs/sas"));
        assert!(!under("docs", "docs/sas"));
    }
}
