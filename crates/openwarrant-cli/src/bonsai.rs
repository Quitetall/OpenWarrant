// SPDX-License-Identifier: AGPL-3.0-or-later
//! Warrant-bound Bonsai evidence.
//!
//! OpenWarrant owns authorization and evidence identity; Bonsai remains the
//! repository checker. This adapter deliberately takes an explicit executable
//! from its caller, never from authored Warrant text.

use std::process::Command;

use camino::Utf8Path;
use openwarrant_compiler::{lower, sha256_hex};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::repo::{RepoError, Repository};

const SCHEMA: &str = "oh.war/bonsai-evidence/v1";
const SCOPE_SCHEMA: &str = "oh.war/bonsai-scope/v1";
const ARCHITECTURE_RULES: &[&str] = &[
    "contract-anchor",
    "contract-forbid",
    "contract-seal",
    "conformance-layering",
    "conformance-cycle",
    "conformance-placement",
    "blueprint-contract",
];

/// Authored machine scope for one Warrant.
#[derive(Debug, Deserialize)]
struct ScopeContract {
    schema: String,
    repository: String,
    base_ref: String,
    policy_path: String,
    policy_digest: String,
    bonsai_source: String,
    bonsai_revision: String,
    #[serde(default)]
    scope: Vec<ScopeEntry>,
}

#[derive(Debug, Deserialize)]
struct ScopeEntry {
    path_glob: String,
    obligation_refs: Vec<String>,
}

impl ScopeContract {
    fn validate(&self) -> Result<(), String> {
        if self.schema != SCOPE_SCHEMA {
            return Err(format!(
                "scope schema must be {SCOPE_SCHEMA:?}, found {:?}",
                self.schema
            ));
        }
        if self.repository.trim().is_empty() {
            return Err("scope repository must not be empty".to_owned());
        }
        if self.base_ref.trim().is_empty() {
            return Err("scope base_ref must not be empty".to_owned());
        }
        if !safe_ref(&self.base_ref) {
            return Err("scope base_ref contains an unsafe ref name".to_owned());
        }
        // Bonsai discovers only the root `bonsai.toml`. Binding any other file
        // would make evidence claim one policy while executing another.
        if self.policy_path != "bonsai.toml" {
            return Err("scope policy_path must be the effective root bonsai.toml".to_owned());
        }
        if !safe_relative(&self.policy_path) {
            return Err(format!(
                "scope policy_path {:?} must be a safe repository-relative path",
                self.policy_path
            ));
        }
        if !is_sha256(&self.policy_digest) {
            return Err(
                "scope policy_digest must be sha256:<64 lowercase hex characters>".to_owned(),
            );
        }
        if self.bonsai_source != "github:Quitetall/bonsai" {
            return Err("scope bonsai_source must identify the approved Bonsai source".to_owned());
        }
        if !is_git_commit(&self.bonsai_revision) {
            return Err("scope bonsai_revision must be a full lowercase Git commit SHA".to_owned());
        }
        if self.scope.is_empty() {
            return Err("scope must declare at least one path_glob".to_owned());
        }
        for entry in &self.scope {
            if !safe_glob(&entry.path_glob) {
                return Err(format!(
                    "scope path_glob {:?} must be an exact path or a path ending in /**",
                    entry.path_glob
                ));
            }
            if entry.obligation_refs.is_empty()
                || entry
                    .obligation_refs
                    .iter()
                    .any(|reference| reference.trim().is_empty())
            {
                return Err(format!(
                    "scope path_glob {:?} must name one or more obligation_refs",
                    entry.path_glob
                ));
            }
        }
        Ok(())
    }

    fn matches(&self, path: &str) -> bool {
        self.scope.iter().any(|entry| {
            entry
                .path_glob
                .strip_suffix("/**")
                .map_or(entry.path_glob == path, |prefix| {
                    path == prefix || path.starts_with(&format!("{prefix}/"))
                })
        })
    }
}

fn safe_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn safe_glob(glob: &str) -> bool {
    let stem = glob.strip_suffix("/**").unwrap_or(glob);
    safe_relative(stem) && !stem.contains('*') && !glob[..stem.len()].contains('?')
}

fn safe_ref(reference: &str) -> bool {
    !reference.is_empty()
        && !reference.starts_with('-')
        && !reference.contains("..")
        && reference
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'_' | b'.' | b'-'))
}

fn is_sha256(digest: &str) -> bool {
    digest.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn is_git_commit(commit: &str) -> bool {
    commit.len() == 40
        && commit
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceVerdict {
    Pass,
    Fail,
    Unknown,
}

#[derive(Debug, Serialize)]
pub struct BonsaiEvidence {
    pub schema: &'static str,
    pub warrant: WarrantBinding,
    pub git: GitBinding,
    pub policy: PolicyBinding,
    pub changed_paths: Vec<String>,
    pub scope_findings: Vec<ScopeFinding>,
    pub bonsai: BonsaiRun,
    pub architecture_findings: Vec<Value>,
    pub advisory_findings: Vec<Value>,
    pub verdict: EvidenceVerdict,
}

#[derive(Debug, Serialize)]
pub struct WarrantBinding {
    pub alias: String,
    pub contract_digest: String,
    pub scope_source: String,
    pub scope_source_digest: String,
}

#[derive(Debug, Serialize)]
pub struct GitBinding {
    pub repository: String,
    pub base: String,
    pub head: String,
    pub tree: String,
}

#[derive(Debug, Serialize)]
pub struct PolicyBinding {
    pub path: String,
    pub digest: String,
}

#[derive(Debug, Serialize)]
pub struct ScopeFinding {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct BonsaiRun {
    pub executable: String,
    pub binary_digest: Option<String>,
    /// Source identity the Warrant requires. This is not a claim that the
    /// supplied binary was built from it; `binary_digest` is the observed fact.
    pub expected_source: String,
    pub expected_revision: String,
    pub version: Option<String>,
    pub exit_code: Option<i32>,
    pub raw_output: Option<Value>,
    pub stderr: String,
    pub spawn_error: Option<String>,
}

/// Execute Bonsai against a Warrant-bounded diff and emit its evidence model.
///
/// The candidate commit must be the checked-out `HEAD`. This closes a common
/// evidence hole: a report over the worktree cannot truthfully bind an arbitrary
/// commit supplied on a command line.
pub fn check(
    repo: &Repository,
    alias: &str,
    base: &str,
    head: &str,
    binary: &Utf8Path,
) -> Result<BonsaiEvidence, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let loaded = repo.load_warrant(&dir)?;
    let (Some(basis), Some(validated)) = (&loaded.basis, &loaded.validated) else {
        return Err(RepoError::Message(format!(
            "{alias}: Warrant is not compilable"
        )));
    };
    if !loaded.report.is_ready() {
        return Err(RepoError::Message(format!(
            "{alias}: Warrant has loading diagnostics"
        )));
    }

    let scope_path = dir.join("scope.toml");
    let scope_bytes = std::fs::read(&scope_path).map_err(|source| RepoError::Io {
        context: format!("{alias}: Bonsai requires {scope_path}"),
        source,
    })?;
    let scope: ScopeContract =
        toml::from_str(&String::from_utf8_lossy(&scope_bytes)).map_err(|source| {
            RepoError::Message(format!("{alias}: could not parse {scope_path}: {source}"))
        })?;
    scope.validate().map_err(|message| {
        RepoError::Message(format!("{alias}: invalid {scope_path}: {message}"))
    })?;

    let worktree = git(repo, &["status", "--porcelain=v1", "--untracked-files=all"])?;
    if !worktree.is_empty() {
        return Err(RepoError::Message(
            "Bonsai evidence requires a clean worktree; check a committed candidate".to_owned(),
        ));
    }

    let contract = lower(basis, validated).map_err(|source| {
        RepoError::Message(format!("{alias}: could not compile contract: {source}"))
    })?;
    let contract_digest = contract.contract_digest().map_err(|source| {
        RepoError::Message(format!("{alias}: could not digest contract: {source}"))
    })?;
    let scope_source = contract
        .source_and_composition
        .scope
        .as_ref()
        .ok_or_else(|| {
            RepoError::Message(format!(
                "{alias}: scope was not captured in the compilation basis"
            ))
        })?;

    let current_head = git(repo, &["rev-parse", "HEAD"])?;
    let canonical_head = git(
        repo,
        &["rev-parse", "--verify", &format!("{head}^{{commit}}")],
    )?;
    if current_head != canonical_head {
        return Err(RepoError::Message(format!(
            "candidate head {canonical_head} is not checked out (HEAD is {current_head})"
        )));
    }
    let canonical_base = git(
        repo,
        &["rev-parse", "--verify", &format!("{base}^{{commit}}")],
    )?;
    let scoped_base = git(
        repo,
        &[
            "rev-parse",
            "--verify",
            &format!("origin/{}^{{commit}}", scope.base_ref),
        ],
    )?;
    let expected_base = git(repo, &["merge-base", &canonical_head, &scoped_base])?;
    if canonical_base != expected_base {
        return Err(RepoError::Message(format!(
            "candidate base {canonical_base} is not merge-base({canonical_head}, origin/{}) = {expected_base}",
            scope.base_ref
        )));
    }
    let tree = git(repo, &["rev-parse", &format!("{canonical_head}^{{tree}}")])?;
    let remote = git(repo, &["remote", "get-url", "origin"])?;
    let repository = github_identity(&remote).ok_or_else(|| {
        RepoError::Message(format!(
            "origin {remote:?} is not a recognizable GitHub repository"
        ))
    })?;
    if scope.repository != repository {
        return Err(RepoError::Message(format!(
            "{alias}: scope repository {:?} does not match origin {:?}",
            scope.repository, repository
        )));
    }

    let policy_path = repo.root.join(&scope.policy_path);
    let policy_bytes = std::fs::read(&policy_path).map_err(|source| RepoError::Io {
        context: format!("{alias}: could not read Bonsai policy {policy_path}"),
        source,
    })?;
    let policy_digest = format!("sha256:{}", sha256_hex(&policy_bytes));
    if policy_digest != scope.policy_digest {
        return Err(RepoError::Message(format!(
            "{alias}: policy digest differs from bound scope (expected {}, got {})",
            scope.policy_digest, policy_digest
        )));
    }

    let changed_paths = git_z(
        repo,
        &[
            "diff",
            "--name-only",
            "--no-renames",
            "--diff-filter=ACDMRT",
            "-z",
            &canonical_base,
            &canonical_head,
        ],
    )?;
    let scope_findings: Vec<ScopeFinding> = changed_paths
        .iter()
        .filter(|path| !scope.matches(path))
        .map(|path| ScopeFinding {
            path: path.clone(),
            message: "changed path is outside Warrant machine scope".to_owned(),
        })
        .collect();

    let version = Command::new(binary.as_std_path())
        .arg("--version")
        .current_dir(repo.root.as_std_path())
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned());
    // An unreadable binary is an unaskable check, not a diagnostic about the
    // Warrant. Keep the missing digest explicit in the Unknown evidence.
    let binary_digest = std::fs::read(binary)
        .ok()
        .map(|bytes| format!("sha256:{}", sha256_hex(&bytes)));

    let launched = Command::new(binary.as_std_path())
        .args([
            "check",
            "--root",
            repo.root.as_str(),
            "--no-cache",
            "--format",
            "json",
            "--since",
            &canonical_base,
        ])
        .current_dir(repo.root.as_std_path())
        .output();

    let (bonsai, raw_findings) = match launched {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parsed = serde_json::from_str::<Value>(&stdout).ok();
            let findings = parsed
                .as_ref()
                .and_then(|value| value.get("findings"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            (
                BonsaiRun {
                    executable: binary.to_string(),
                    binary_digest,
                    expected_source: scope.bonsai_source.clone(),
                    expected_revision: scope.bonsai_revision.clone(),
                    version,
                    exit_code: output.status.code(),
                    raw_output: parsed,
                    stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                    spawn_error: None,
                },
                findings,
            )
        }
        Err(error) => (
            BonsaiRun {
                executable: binary.to_string(),
                binary_digest,
                expected_source: scope.bonsai_source.clone(),
                expected_revision: scope.bonsai_revision.clone(),
                version: None,
                exit_code: None,
                raw_output: None,
                stderr: String::new(),
                spawn_error: Some(error.to_string()),
            },
            Vec::new(),
        ),
    };

    let architecture_findings: Vec<Value> = raw_findings
        .iter()
        .filter(|finding| is_architecture_error(finding))
        .cloned()
        .collect();
    let advisory_findings: Vec<Value> = raw_findings
        .iter()
        .filter(|finding| !is_architecture_error(finding))
        .cloned()
        .collect();
    let bonsai_asked = valid_bonsai_report(&bonsai);
    let verdict = if !scope_findings.is_empty() || !architecture_findings.is_empty() {
        EvidenceVerdict::Fail
    } else if bonsai_asked {
        EvidenceVerdict::Pass
    } else {
        EvidenceVerdict::Unknown
    };

    Ok(BonsaiEvidence {
        schema: SCHEMA,
        warrant: WarrantBinding {
            alias: alias.to_owned(),
            contract_digest,
            scope_source: scope_source.source.clone(),
            scope_source_digest: format!("sha256:{}", scope_source.scope_source_digest),
        },
        git: GitBinding {
            repository,
            base: canonical_base,
            head: canonical_head,
            tree,
        },
        policy: PolicyBinding {
            path: scope.policy_path,
            digest: policy_digest,
        },
        changed_paths,
        scope_findings,
        bonsai,
        architecture_findings,
        advisory_findings,
        verdict,
    })
}

fn is_architecture_error(finding: &Value) -> bool {
    finding.get("severity").and_then(Value::as_str) == Some("error")
        && finding
            .get("rule")
            .and_then(Value::as_str)
            .is_some_and(|rule| ARCHITECTURE_RULES.contains(&rule))
}

fn valid_bonsai_report(run: &BonsaiRun) -> bool {
    matches!(run.exit_code, Some(0 | 1))
        && run.raw_output.as_ref().is_some_and(|report| {
            report.get("tool").and_then(Value::as_str) == Some("bonsai")
                && report
                    .get("version")
                    .and_then(Value::as_str)
                    .is_some_and(|version| !version.is_empty())
                && report
                    .get("findings")
                    .and_then(Value::as_array)
                    .is_some_and(|findings| findings.iter().all(valid_finding))
        })
}

fn valid_finding(finding: &Value) -> bool {
    let Some(object) = finding.as_object() else {
        return false;
    };
    let valid_text = |field| {
        object
            .get(field)
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty())
    };
    let valid_location = |location: &Value| {
        location
            .get("file")
            .and_then(Value::as_str)
            .is_some_and(|file| !file.is_empty())
            && location
                .get("line")
                .is_none_or(|line| line.as_u64().is_some())
    };
    valid_text("rule")
        && valid_text("message")
        && matches!(
            object.get("severity").and_then(Value::as_str),
            Some("error" | "warning" | "info")
        )
        && object.get("location").is_some_and(valid_location)
        && object.get("related").is_none_or(|related| {
            related
                .as_array()
                .is_some_and(|locations| locations.iter().all(valid_location))
        })
        && object
            .get("fix")
            .is_none_or(|fix| fix.as_str().is_some_and(|value| !value.is_empty()))
}

fn git(repo: &Repository, args: &[&str]) -> Result<String, RepoError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo.root.as_str())
        .args(args)
        .output()
        .map_err(|source| RepoError::Io {
            context: "could not run git for Bonsai evidence".to_owned(),
            source,
        })?;
    if !output.status.success() {
        return Err(RepoError::Message(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|_| {
            RepoError::Message(format!("git {} returned non-UTF-8 output", args.join(" ")))
        })
}

fn git_z(repo: &Repository, args: &[&str]) -> Result<Vec<String>, RepoError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo.root.as_str())
        .args(args)
        .output()
        .map_err(|source| RepoError::Io {
            context: "could not run git for Bonsai evidence".to_owned(),
            source,
        })?;
    if !output.status.success() {
        return Err(RepoError::Message(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let output = String::from_utf8(output.stdout)
        .map_err(|_| RepoError::Message("git diff returned non-UTF-8 path output".to_owned()))?;
    Ok(output
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect())
}

fn github_identity(remote: &str) -> Option<String> {
    let without_suffix = remote.trim().strip_suffix(".git").unwrap_or(remote.trim());
    let path = without_suffix
        .strip_prefix("git@github.com:")
        .or_else(|| without_suffix.strip_prefix("https://github.com/"))
        .or_else(|| without_suffix.strip_prefix("ssh://git@github.com/"))?;
    let mut parts = path.split('/');
    let owner = parts.next()?;
    let name = parts.next()?;
    if owner.is_empty() || name.is_empty() || parts.next().is_some() {
        return None;
    }
    Some(format!("github:{owner}/{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_paths_are_small_and_unambiguous() {
        assert!(safe_glob("crates/openwarrant-cli/**"));
        assert!(safe_glob("bonsai.toml"));
        assert!(!safe_glob("../outside/**"));
        assert!(!safe_glob("crates/*/src/**"));
        assert!(!safe_glob("/absolute/**"));
    }

    #[test]
    fn only_full_commit_ids_are_accepted() {
        assert!(is_git_commit("4c8cc1043cbc4d30d2c41cbb25ba1afe25c6ad7c"));
        assert!(!is_git_commit("4c8cc10"));
    }

    #[test]
    fn github_remote_is_normalized() {
        assert_eq!(
            github_identity("git@github.com:Quitetall/OpenWarrant.git"),
            Some("github:Quitetall/OpenWarrant".to_owned())
        );
        assert_eq!(
            github_identity("https://github.com/Quitetall/OpenWarrant.git"),
            Some("github:Quitetall/OpenWarrant".to_owned())
        );
    }

    #[test]
    fn only_architecture_errors_block() {
        let architecture = serde_json::json!({"rule":"contract-seal", "severity":"error"});
        let advisory = serde_json::json!({"rule":"leanness-ratchet", "severity":"error"});
        assert!(is_architecture_error(&architecture));
        assert!(!is_architecture_error(&advisory));
    }

    #[test]
    fn malformed_bonsai_output_is_not_an_asked_check() {
        let run = BonsaiRun {
            executable: "bonsai".to_owned(),
            binary_digest: Some("sha256:0".to_owned()),
            expected_source: "github:Quitetall/bonsai".to_owned(),
            expected_revision: "0".repeat(40),
            version: Some("bonsai 0.1.0".to_owned()),
            exit_code: Some(1),
            raw_output: Some(serde_json::json!({
                "tool": "bonsai",
                "version": "0.1.0",
                "findings": [null]
            })),
            stderr: String::new(),
            spawn_error: None,
        };
        assert!(!valid_bonsai_report(&run));
    }
}
