// SPDX-License-Identifier: Apache-2.0
//! `war check --changed`: what the work changed, in what it puts in scope.
//!
//! An end-of-turn hook asked `war check` over the whole corpus every time an
//! agent stopped: seconds of work and, before the brief rendering, hundreds
//! of kilobytes it then had to read past. What the agent can have broken is
//! what it changed. Git is asked, over the paths OpenWarrant governs and no
//! other (issue #365: a pathspec left empty reads the whole tree), which of
//! them differ from a base commit (`HEAD`, or the commit the session started
//! at, so work the agent committed during the turn is not missed) or are new.
//!
//! - A Warrant's own record directory, or a file one of its deliverables
//!   pins, puts that Warrant in scope, and with it every Warrant that cites it
//!   as a parent: a parent's change is what makes its child's citation stale.
//! - A ticket under the tickets directory puts the tickets in scope; a file
//!   the `[documents] index` globs match puts the documents in scope.
//! - A change to what every Warrant reads (the SAS, the roadmap, ADRs, gates,
//!   the authority register, `openwarrant.toml`) puts the corpus in scope.

use std::collections::{BTreeMap, BTreeSet};

use crate::repo::{RepoError, Repository};

/// What a change needs checked. Empty when nothing governed moved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scope {
    /// Why the whole corpus is in scope, when it is: something every Warrant
    /// reads changed, or git could not say what did.
    pub corpus: Option<String>,
    /// The Warrants whose records or pinned files changed.
    pub warrants: BTreeSet<String>,
    /// The Warrants in scope because they cite a changed one as a parent.
    pub dependents: BTreeSet<String>,
    /// A ticket changed.
    pub tickets: bool,
    /// A document the `[documents] index` globs match changed.
    pub documents: bool,
}

impl Scope {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.corpus.is_none()
            && self.warrants.is_empty()
            && self.dependents.is_empty()
            && !self.tickets
            && !self.documents
    }

    /// Every Warrant to check: the changed ones and those citing them.
    #[must_use]
    pub fn all_warrants(&self) -> Vec<String> {
        self.warrants.union(&self.dependents).cloned().collect()
    }

    fn whole(why: impl Into<String>) -> Self {
        Self {
            corpus: Some(why.into()),
            ..Self::default()
        }
    }
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

/// `git <args> -- <literal paths> <globs>`, NUL-separated names; `None`
/// when git fails. Never run with no pathspec: that is the whole tree.
fn git_names(
    repo: &Repository,
    args: &[&str],
    literal: &[String],
    globs: &[String],
) -> Option<Vec<String>> {
    if literal.is_empty() && globs.is_empty() {
        return Some(Vec::new());
    }
    let out = std::process::Command::new("git")
        .args(args)
        .arg("--")
        .args(literal.iter().map(|s| format!(":(literal){s}")))
        .args(
            globs
                .iter()
                .map(|g| format!(":(glob){}", g.trim_start_matches("./"))),
        )
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

fn is_commit(repo: &Repository, rev: &str) -> bool {
    std::process::Command::new("git")
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{rev}^{{commit}}"),
        ])
        .current_dir(&repo.root)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn under(path: &str, dir: &str) -> bool {
    let dir = dir.trim_end_matches('/');
    path == dir || path.strip_prefix(dir).is_some_and(|r| r.starts_with('/'))
}

/// What the working tree's difference from `since` (default `HEAD`) puts in
/// scope. A `since` that names no commit is refused, never read as "nothing
/// changed".
pub fn scope(corpus: &crate::corpus::Corpus, since: Option<&str>) -> Result<Scope, RepoError> {
    let repo = corpus.repo();
    let base = since.unwrap_or("HEAD");
    if !is_commit(repo, base) {
        if let Some(rev) = since {
            return Err(RepoError::Message(format!(
                "--since {rev}: names no commit, so what changed since it cannot be said"
            )));
        }
        return Ok(Scope::whole(
            "no commit to compare against, so every Warrant is new",
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
    let tickets = crate::ticket::Store::open(repo, None)
        .ok()
        .map(|s| repo.relative(&s.dir).trim_end_matches('/').to_owned())
        .filter(|d| !d.is_empty());
    let (doc_globs, _) = crate::doc_index::globs(repo);
    let shared = shared_paths(repo);
    let mut literal: Vec<String> = shared.clone();
    literal.push(warrants.clone());
    literal.extend(tickets.clone());
    literal.extend(pinned.keys().cloned());
    literal.sort();
    literal.dedup();

    let tracked = git_names(
        repo,
        &["diff", "--name-only", "--no-renames", "-z", base],
        &literal,
        &doc_globs,
    );
    let untracked = git_names(
        repo,
        &["ls-files", "--others", "--exclude-standard", "-z"],
        &literal,
        &doc_globs,
    );
    let (Some(tracked), Some(untracked)) = (tracked, untracked) else {
        return Ok(Scope::whole(
            "git could not say what changed, so nothing is assumed unchanged",
        ));
    };

    let mut scope = Scope::default();
    for path in tracked.iter().chain(&untracked) {
        if let Some(dir) = shared.iter().find(|d| under(path, d)) {
            return Ok(Scope::whole(format!(
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
            scope.warrants.insert(alias.to_owned());
        }
        if let Some(owners) = pinned.get(path) {
            scope.warrants.extend(owners.iter().cloned());
        }
        if tickets.as_deref().is_some_and(|d| under(path, d)) {
            scope.tickets = true;
        }
        // The document index never walks the Warrant or ticket trees, a
        // `generated/` projection or a hidden directory (doc_index.rs), so a
        // change there is not a document's.
        let indexed = !under(path, &warrants)
            && !tickets.as_deref().is_some_and(|d| under(path, d))
            && !path
                .split('/')
                .any(|seg| seg == "generated" || seg.starts_with('.'));
        if indexed
            && doc_globs
                .iter()
                .any(|g| crate::gate_cmd::source::glob_matches(g, path))
        {
            scope.documents = true;
        }
    }
    if !scope.warrants.is_empty() {
        scope.dependents = dependents(corpus, &scope.warrants)?;
    }
    Ok(scope)
}

/// The Warrants, not in `changed`, whose manifests cite one of `changed` as a
/// parent (`[[parents]] ref = "war://<uuid>"`).
fn dependents(
    corpus: &crate::corpus::Corpus,
    changed: &BTreeSet<String>,
) -> Result<BTreeSet<String>, RepoError> {
    let all = corpus.loaded_all()?;
    let uuids: BTreeSet<String> = all
        .iter()
        .filter(|l| changed.contains(&l.alias()))
        .filter_map(|l| l.validated.as_ref().map(|v| v.uuid.to_string()))
        .collect();
    Ok(all
        .iter()
        .filter(|l| !changed.contains(&l.alias()))
        .filter(|l| {
            l.basis.as_ref().is_some_and(|b| {
                b.manifest
                    .parents
                    .iter()
                    .any(|p| uuids.contains(p.r#ref.strip_prefix("war://").unwrap_or(&p.r#ref)))
            })
        })
        .map(|l| l.alias())
        .collect())
}

/// The headline of a scoped run: what was checked, and why.
#[must_use]
pub fn headline(scope: &Scope) -> String {
    fn names(set: &BTreeSet<String>) -> String {
        let shown: Vec<&str> = set.iter().take(8).map(String::as_str).collect();
        match set.len() {
            n if n > 8 => format!("{} and {} more", shown.join(", "), n - 8),
            _ => shown.join(", "),
        }
    }
    let mut parts = Vec::new();
    if !scope.warrants.is_empty() {
        parts.push(names(&scope.warrants));
    }
    if !scope.dependents.is_empty() {
        parts.push(format!(
            "{} (they cite a changed parent)",
            names(&scope.dependents)
        ));
    }
    if scope.tickets {
        parts.push("the tickets".to_owned());
    }
    if scope.documents {
        parts.push("the documents".to_owned());
    }
    format!("checked what changed: {}", parts.join("; "))
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
