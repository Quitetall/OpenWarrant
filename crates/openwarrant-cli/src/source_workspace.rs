// SPDX-License-Identifier: Apache-2.0
//! A source checkpoint in the same local Git repository. Authority and retention
//! remain with the original repository; no checkout or source mutation occurs.
use crate::{
    diagnostic::Severity,
    repo::{RepoError, Repository},
};
use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::CompilationBasis;

pub(crate) struct SourceWorkspace {
    pub repository: Repository,
    pub revision: String,
}

fn git(repo: &Repository, args: &[&str], limit: usize) -> Result<String, RepoError> {
    let bytes = crate::preservation::local_git(repo, args, limit).map_err(|e| {
        RepoError::ObservationUnavailable {
            rule: "workspace.git-unavailable",
            message: e.to_string(),
        }
    })?;
    String::from_utf8(bytes)
        .map(|s| s.trim().to_owned())
        .map_err(|e| RepoError::Message(format!("workspace.git-metadata: {e}")))
}

impl SourceWorkspace {
    pub fn open(
        repo: &Repository,
        path: &Utf8Path,
        alias: &str,
        basis: &CompilationBasis,
    ) -> Result<Self, RepoError> {
        let canonical =
            std::fs::canonicalize(path).map_err(|e| RepoError::ObservationUnavailable {
                rule: "workspace.unavailable",
                message: e.to_string(),
            })?;
        let root = Utf8PathBuf::from_path_buf(canonical)
            .map_err(|_| RepoError::Message("workspace.non-utf8-path".into()))?;
        let source = Repository::open(root)?;
        let common = |r: &Repository| -> Result<std::path::PathBuf, RepoError> {
            let p = git(
                r,
                &["rev-parse", "--path-format=absolute", "--git-common-dir"],
                4096,
            )?;
            std::fs::canonicalize(p).map_err(|e| RepoError::ObservationUnavailable {
                rule: "workspace.git-unavailable",
                message: e.to_string(),
            })
        };
        if common(repo)? != common(&source)?
            || repo.config.project.name != source.config.project.name
            || !git(&source, &["rev-parse", "--show-prefix"], 4096)?.is_empty()
        {
            return Err(RepoError::Message(
                "workspace.repository-mismatch: select a worktree of the original repository"
                    .into(),
            ));
        }
        let revision = git(&source, &["rev-parse", "--verify", "HEAD^{commit}"], 128)?;
        if !matches!(revision.len(), 40 | 64)
            || !revision
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(RepoError::Message("workspace.invalid-checkpoint".into()));
        }
        let loaded = source.load_warrant(&source.warrant_dir(alias)?)?;
        if loaded
            .report
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Unknown)
        {
            return Err(RepoError::ObservationUnavailable {
                rule: "workspace.basis-unavailable",
                message: "Source contract cannot be established".into(),
            });
        }
        if loaded
            .report
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
            || loaded.basis.as_ref() != Some(basis)
        {
            return Err(RepoError::Message(
                "workspace.contract-mismatch: source contract differs from the original project"
                    .into(),
            ));
        }
        let selected = Self {
            repository: source,
            revision,
        };
        selected.recheck()?;
        Ok(selected)
    }

    pub fn recheck(&self) -> Result<(), RepoError> {
        if git(
            &self.repository,
            &["rev-parse", "--verify", "HEAD^{commit}"],
            128,
        )? != self.revision
        {
            return Err(RepoError::Message("workspace.changed-checkpoint".into()));
        }
        if !git(
            &self.repository,
            &["status", "--porcelain", "--untracked-files=all"],
            65536,
        )?
        .is_empty()
        {
            return Err(RepoError::Message(
                "workspace.dirty: commit the source checkpoint first".into(),
            ));
        }
        Ok(())
    }
}
