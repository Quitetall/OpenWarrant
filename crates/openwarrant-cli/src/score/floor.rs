// SPDX-License-Identifier: Apache-2.0
//! The CI floor (OW-WAR-0148 M17, decision 23): a change never lowers the
//! level.
//!
//! Opt-in: `war check --floor <base>` compares the working tree with any
//! commit, and `war check --pr <n>` compares the PR with its base when the
//! base branch's openwarrant.toml says `[score] floor = true`.
//!
//! The base is scored in a detached worktree of that commit (its records,
//! its ledger, its history), removed afterwards. A base that cannot be
//! scored, or a head whose history cannot be read, is UNKNOWN: never a pass.

use serde::Serialize;

use super::{Score, compute, measure};
use crate::diagnostic::{Diagnostic, Report};
use crate::go::git::git;
use crate::repo::Repository;

/// The floor's answer.
#[derive(Debug, Clone, Serialize)]
pub struct Answer {
    pub schema: &'static str,
    pub base: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_score: Option<Score>,
    pub head_score: Score,
    /// `pass`, `refused` or `unknown`.
    pub verdict: &'static str,
}

pub const SCHEMA: &str = "oh.war/score-floor/v1";

/// Score `rev` in a throwaway worktree.
///
/// # Errors
/// Why it could not: the commit is unknown here, or the worktree or the
/// repository would not open.
pub fn score_at(repo: &Repository, rev: &str) -> Result<Score, String> {
    let sha = git(
        &repo.root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{rev}^{{commit}}"),
        ],
    );
    if !sha.ok {
        return Err(format!(
            "{rev:?} is not a commit this clone has (a CI checkout needs `fetch-depth: 0`)"
        ));
    }
    let sha = sha.stdout.trim().to_owned();
    let tmp = std::env::temp_dir().join(format!("war-score-{}-{}", std::process::id(), &sha[..12]));
    let Some(tmp_s) = tmp.to_str().map(str::to_owned) else {
        return Err("the temporary directory's path is not UTF-8".to_owned());
    };
    let add = git(
        &repo.root,
        &["worktree", "add", "--detach", "--quiet", &tmp_s, &sha],
    );
    if !add.ok {
        return Err(format!("git could not check out {sha}: {}", add.why()));
    }
    let out = Repository::open(camino::Utf8PathBuf::from(&tmp_s))
        .map(|r| compute(&measure(&r)))
        .map_err(|e| format!("the base's records would not open: {e}"));
    let _ = git(&repo.root, &["worktree", "remove", "--force", &tmp_s]);
    let _ = std::fs::remove_dir_all(&tmp);
    out
}

/// Compare the working tree's level with `base`'s.
#[must_use]
pub fn check(repo: &Repository, base: &str) -> (Report, Answer) {
    let mut report = Report::default();
    let head = compute(&measure(repo));
    let mut answer = Answer {
        schema: SCHEMA,
        base: base.to_owned(),
        base_score: None,
        head_score: head.clone(),
        verdict: "unknown",
    };
    let base_score = match score_at(repo, base) {
        Ok(s) => s,
        Err(why) => {
            report.push(Diagnostic::unknown(
                "score.floor",
                "war check --floor".to_owned(),
                format!(
                    "the base could not be scored ({why}), so whether this change lowers the \
                     level is UNKNOWN, never a pass"
                ),
            ));
            return (report, answer);
        }
    };
    let unknown_now: Vec<&str> = head
        .dimensions
        .iter()
        .zip(&base_score.dimensions)
        .filter(|(h, b)| h.status == super::Status::Unknown && b.status == super::Status::Measured)
        .map(|(h, _)| h.id.as_str())
        .collect();
    if head.level.number < base_score.level.number {
        let mut drops: Vec<(i64, &str)> = head
            .dimensions
            .iter()
            .zip(&base_score.dimensions)
            .map(|(h, b)| (i64::from(h.points) - i64::from(b.points), h.id.as_str()))
            .filter(|(d, _)| *d < 0)
            .collect();
        drops.sort();
        report.push(Diagnostic::error(
            "score.floor",
            "war check --floor".to_owned(),
            format!(
                "this change lowers the level from {} ({}, score {}) to {} ({}, score {}); the \
                 floor keeps the level. Biggest drops: {}. To raise it: {}",
                base_score.level.number,
                base_score.level.name,
                base_score.score,
                head.level.number,
                head.level.name,
                head.score,
                drops
                    .iter()
                    .take(3)
                    .map(|(d, id)| format!("{id} {d}"))
                    .collect::<Vec<_>>()
                    .join(", "),
                head.next_steps
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "see `war status`".to_owned())
            ),
        ));
        answer.verdict = "refused";
    } else if !unknown_now.is_empty() {
        report.push(Diagnostic::unknown(
            "score.floor",
            "war check --floor".to_owned(),
            format!(
                "{} measured at the base but UNKNOWN here; the level holds only by the \
                 dimensions that could be read, so the floor is UNKNOWN, never a pass",
                unknown_now.join(", ")
            ),
        ));
    } else {
        report.push(Diagnostic::pass(
            "score.floor",
            format!(
                "the level holds: {} ({}) at the base, {} ({}) here; score {} -> {}",
                base_score.level.number,
                base_score.level.name,
                head.level.number,
                head.level.name,
                base_score.score,
                head.score
            ),
        ));
        answer.verdict = "pass";
    }
    answer.base_score = Some(base_score);
    (report, answer)
}
