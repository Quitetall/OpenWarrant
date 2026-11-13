// SPDX-License-Identifier: Apache-2.0
//! A known legacy contract is not an unknown daily-loop work item.

use super::{Outcome, Store};
use crate::repo::{RepoError, Repository};

/// Preserve the legacy submission/resolution boundary; never tick or migrate it.
pub(super) fn refusal(store: &Store, query: &str) -> Result<Option<Outcome>, RepoError> {
    let (alias, stage) = query
        .trim()
        .split_once('/')
        .map_or((query.trim(), None), |(alias, stage)| (alias, Some(stage)));
    let repo = Repository::open(store.root.clone())?;
    let dir = match repo.warrant_dir(alias) {
        Ok(dir) if dir.join("manifest.toml").is_file() => dir,
        Ok(_) | Err(RepoError::UnknownWarrant { .. }) => return Ok(None),
        Err(error) => return Err(error),
    };
    if let Some(stage) = stage {
        let loaded = repo.load_warrant(&dir)?;
        let graph = loaded
            .basis
            .as_ref()
            .and_then(|basis| basis.atoms.iter().find(|atom| atom.role == "milestones"))
            .and_then(|atom| std::str::from_utf8(&atom.bytes).ok())
            .map(openwarrant_core::milestones::parse)
            .transpose()
            .map_err(|error| RepoError::Message(format!("{alias}: invalid milestones: {error}")))?;
        if !graph.is_some_and(|graph| graph.stages.iter().any(|entry| entry.id == stage)) {
            return Ok(Some(Outcome::refused(
                "done.legacy-stage-unknown",
                repo.relative(&dir),
                format!(
                    "{alias} exists, but does not declare stage {stage:?}; inspect `war view context {alias}`"
                ),
            )));
        }
        return Ok(Some(Outcome::refused(
            "done.legacy-stage",
            repo.relative(&dir),
            format!(
                "{query} is a legacy directory stage, not a daily-loop item. \
                 `war admin dispatch {alias} {stage}` compiles its Dispatch subject to the legacy \
                 action gates; `war evidence submit {alias} <submission-file>` records work against \
                 that Dispatch. No completion, authorization or resolution was recorded. \
                 `war sign resolve --dry-run {alias}` lists the remaining legacy closure requirements; \
                 independent evidence and required human acts are not replaced by `war done`."
            ),
        )));
    }
    Ok(Some(Outcome::refused(
        "done.legacy-warrant",
        repo.relative(&dir),
        format!(
            "{alias} is a legacy directory Warrant. `war done` cannot replace its resolution \
             workflow. Run `war sign resolve --dry-run {alias}` to inspect the remaining \
             requirements, and follow `docs/agents/legacy-warrant-workflow.md`. \
             No completion, authorization or resolution was recorded."
        ),
    )))
}
