// SPDX-License-Identifier: AGPL-3.0-or-later
//! Warrant-bound Bonsai evidence.
//!
//! OpenWarrant owns authorization and evidence identity; Bonsai remains the
//! repository checker. This adapter deliberately takes an explicit executable
//! from its caller, never from authored Warrant text.

use std::collections::BTreeSet;
use std::process::Command;

use camino::Utf8Path;
use openwarrant_compiler::{CompilationBasis, lower, sha256_hex};
use openwarrant_core::obligation;
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
struct ScopeEntry {
    path_glob: String,
    obligation_refs: Vec<String>,
}

impl ScopeContract {
    fn validate(&self, known_obligations: &BTreeSet<String>) -> Result<(), String> {
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
            for obligation in &entry.obligation_refs {
                if !known_obligations.contains(obligation) {
                    return Err(format!(
                        "scope path_glob {:?} refers to undeclared obligation {:?}",
                        entry.path_glob, obligation
                    ));
                }
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
    digest.strip_prefix("sha256:").is_some_and(is_digest_hex)
}

fn is_digest_hex(hex: &str) -> bool {
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_git_commit(commit: &str) -> bool {
    commit.len() == 40
        && commit
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceVerdict {
    Pass,
    Fail,
    Unknown,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BonsaiEvidence {
    pub schema: String,
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WarrantBinding {
    pub alias: String,
    pub contract_digest: String,
    pub scope_source: String,
    pub scope_source_digest: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitBinding {
    pub repository: String,
    pub base: String,
    pub head: String,
    pub tree: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyBinding {
    pub path: String,
    pub digest: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeFinding {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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

/// Validate the optional Bonsai scope sidecar during `war check`.
///
/// A scope entry names obligations, so accepting a dangling name would allow a
/// passing adapter report to claim coverage no assurance atom declares.
pub(crate) fn validate_scope(alias: &str, basis: &CompilationBasis) -> Result<(), RepoError> {
    load_scope(alias, basis).map(|_| ())
}

fn load_scope(alias: &str, basis: &CompilationBasis) -> Result<ScopeContract, RepoError> {
    let scope_source = basis.scope.as_ref().ok_or_else(|| {
        RepoError::Message(format!("{alias}: Bonsai requires a bound scope.toml"))
    })?;
    // Never reread this path. The Compilation Basis is the evidence boundary:
    // using current filesystem bytes here could enforce a different policy than
    // the digest embedded in the Warrant contract.
    let scope: ScopeContract = toml::from_str(&String::from_utf8_lossy(&scope_source.bytes))
        .map_err(|source| {
            RepoError::Message(format!(
                "{alias}: could not parse bound {}: {source}",
                scope_source.source
            ))
        })?;
    let known_obligations = declared_obligations(basis).map_err(|message| {
        RepoError::Message(format!(
            "{alias}: invalid Bonsai assurance basis: {message}"
        ))
    })?;
    scope.validate(&known_obligations).map_err(|message| {
        RepoError::Message(format!(
            "{alias}: invalid bound {}: {message}",
            scope_source.source
        ))
    })?;
    Ok(scope)
}

fn declared_obligations(basis: &CompilationBasis) -> Result<BTreeSet<String>, String> {
    let mut known = BTreeSet::new();
    for atom in basis.atoms.iter().filter(|atom| atom.role == "assurance") {
        let source = String::from_utf8_lossy(&atom.bytes);
        let obligations =
            obligation::parse(&source).map_err(|error| format!("{}: {error}", atom.source))?;
        known.extend(obligations.ids().into_iter().map(str::to_owned));
    }
    Ok(known)
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

    let scope = load_scope(alias, basis)?;

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

    let bonsai_asked = valid_bonsai_report(&bonsai);
    // Finding classification has no meaning until the complete machine result
    // is validated. In particular, a malformed document cannot turn a string
    // that resembles an architecture rule into a false failure.
    let architecture_findings: Vec<Value> = if bonsai_asked {
        raw_findings
            .iter()
            .filter(|finding| is_architecture_error(finding))
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    let advisory_findings: Vec<Value> = if bonsai_asked {
        raw_findings
            .iter()
            .filter(|finding| !is_architecture_error(finding))
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    let verdict = evidence_verdict(
        !scope_findings.is_empty(),
        bonsai_asked,
        !architecture_findings.is_empty(),
    );

    Ok(BonsaiEvidence {
        schema: SCHEMA.to_owned(),
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

fn evidence_verdict(
    has_scope_findings: bool,
    bonsai_asked: bool,
    has_architecture_findings: bool,
) -> EvidenceVerdict {
    if has_scope_findings || (bonsai_asked && has_architecture_findings) {
        EvidenceVerdict::Fail
    } else if bonsai_asked {
        EvidenceVerdict::Pass
    } else {
        EvidenceVerdict::Unknown
    }
}

fn is_architecture_error(finding: &Value) -> bool {
    finding.get("severity").and_then(Value::as_str) == Some("error")
        && finding
            .get("rule")
            .and_then(Value::as_str)
            .is_some_and(|rule| ARCHITECTURE_RULES.contains(&rule))
}

fn valid_bonsai_report(run: &BonsaiRun) -> bool {
    run.binary_digest.is_some()
        && run.raw_output.as_ref().is_some_and(|report| {
            let Some(findings) = report.get("findings").and_then(Value::as_array) else {
                return false;
            };
            let has_error = findings
                .iter()
                .any(|finding| finding.get("severity").and_then(Value::as_str) == Some("error"));
            matches!(
                (run.exit_code, has_error),
                (Some(0), false) | (Some(1), true)
            ) && report.get("tool").and_then(Value::as_str) == Some("bonsai")
                && report
                    .get("version")
                    .and_then(Value::as_str)
                    .is_some_and(|version| !version.is_empty())
                && findings.iter().all(valid_finding)
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
                .is_none_or(|line| line.as_u64().is_some_and(|line| line <= u32::MAX.into()))
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

/// Read and validate a pass-worthy Bonsai evidence document from this repository.
///
/// This validates evidence syntax and its internal consistency. It does not run
/// Bonsai; `war bonsai check` remains the producer of a fresh observation.
pub(crate) fn verify_evidence_file(
    repo: &Repository,
    evidence_path: &Utf8Path,
) -> Result<(), RepoError> {
    if evidence_path.is_absolute() || !safe_relative(evidence_path.as_str()) {
        return Err(RepoError::Message(
            "Bonsai evidence path must be a safe repository-relative path".to_owned(),
        ));
    }
    let bytes = std::fs::read(repo.root.join(evidence_path)).map_err(|source| RepoError::Io {
        context: format!("could not read Bonsai evidence {evidence_path}"),
        source,
    })?;
    validate_passing_evidence_bytes(&bytes)
        .map(|_| ())
        .map_err(RepoError::Message)
}

/// Validate the bytes attached to a gate receipt and return their typed model.
pub(crate) fn validate_passing_evidence_bytes(bytes: &[u8]) -> Result<BonsaiEvidence, String> {
    let evidence: BonsaiEvidence = serde_json::from_slice(bytes)
        .map_err(|error| format!("Bonsai evidence is not valid v1 JSON: {error}"))?;
    if evidence.schema != SCHEMA {
        return Err(format!("Bonsai evidence schema must be {SCHEMA:?}"));
    }
    if evidence.verdict != EvidenceVerdict::Pass {
        return Err("Bonsai evidence verdict must be pass".to_owned());
    }
    if evidence.warrant.alias.trim().is_empty()
        || !is_digest_hex(&evidence.warrant.contract_digest)
        || !safe_relative(&evidence.warrant.scope_source)
        || !is_sha256(&evidence.warrant.scope_source_digest)
    {
        return Err("Bonsai evidence has an invalid Warrant binding".to_owned());
    }
    if !valid_github_identity(&evidence.git.repository)
        || !is_git_commit(&evidence.git.base)
        || !is_git_commit(&evidence.git.head)
        || !is_git_commit(&evidence.git.tree)
    {
        return Err("Bonsai evidence has an invalid Git binding".to_owned());
    }
    if !safe_relative(&evidence.policy.path) || !is_sha256(&evidence.policy.digest) {
        return Err("Bonsai evidence has an invalid policy binding".to_owned());
    }
    if evidence
        .changed_paths
        .iter()
        .any(|path| !safe_relative(path))
    {
        return Err("Bonsai evidence contains an unsafe changed path".to_owned());
    }
    if !evidence.scope_findings.is_empty() || !evidence.architecture_findings.is_empty() {
        return Err("passing Bonsai evidence cannot contain blocking findings".to_owned());
    }
    if evidence.bonsai.spawn_error.is_some()
        || !is_sha256(evidence.bonsai.binary_digest.as_deref().unwrap_or_default())
        || evidence.bonsai.expected_source != "github:Quitetall/bonsai"
        || !is_git_commit(&evidence.bonsai.expected_revision)
        || !valid_bonsai_report(&evidence.bonsai)
    {
        return Err("Bonsai evidence does not contain a valid completed Bonsai run".to_owned());
    }

    let findings = evidence
        .bonsai
        .raw_output
        .as_ref()
        .and_then(|output| output.get("findings"))
        .and_then(Value::as_array)
        .ok_or_else(|| "Bonsai evidence has no typed findings array".to_owned())?;
    let expected_architecture = findings
        .iter()
        .filter(|finding| is_architecture_error(finding))
        .cloned()
        .collect::<Vec<_>>();
    let expected_advisory = findings
        .iter()
        .filter(|finding| !is_architecture_error(finding))
        .cloned()
        .collect::<Vec<_>>();
    if evidence.architecture_findings != expected_architecture
        || evidence.advisory_findings != expected_advisory
    {
        return Err("Bonsai evidence finding classifications do not match raw output".to_owned());
    }
    Ok(evidence)
}

fn valid_github_identity(repository: &str) -> bool {
    let Some(path) = repository.strip_prefix("github:") else {
        return false;
    };
    let mut parts = path.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.is_empty() && !name.is_empty()
    )
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

    fn clean_evidence() -> BonsaiEvidence {
        BonsaiEvidence {
            schema: SCHEMA.to_owned(),
            warrant: WarrantBinding {
                alias: "OW-WAR-0050".to_owned(),
                contract_digest: "a".repeat(64),
                scope_source: "docs/warrants/OW-WAR-0050/scope.toml".to_owned(),
                scope_source_digest: format!("sha256:{}", "b".repeat(64)),
            },
            git: GitBinding {
                repository: "github:Quitetall/OpenWarrant".to_owned(),
                base: "c".repeat(40),
                head: "d".repeat(40),
                tree: "e".repeat(40),
            },
            policy: PolicyBinding {
                path: "bonsai.toml".to_owned(),
                digest: format!("sha256:{}", "f".repeat(64)),
            },
            changed_paths: vec!["crates/openwarrant-cli/src/bonsai.rs".to_owned()],
            scope_findings: vec![],
            bonsai: BonsaiRun {
                executable: "target/release/bonsai".to_owned(),
                binary_digest: Some(format!("sha256:{}", "1".repeat(64))),
                expected_source: "github:Quitetall/bonsai".to_owned(),
                expected_revision: "2".repeat(40),
                version: Some("bonsai fixture".to_owned()),
                exit_code: Some(0),
                raw_output: Some(serde_json::json!({
                    "tool": "bonsai",
                    "version": "fixture",
                    "findings": []
                })),
                stderr: String::new(),
                spawn_error: None,
            },
            architecture_findings: vec![],
            advisory_findings: vec![],
            verdict: EvidenceVerdict::Pass,
        }
    }

    #[test]
    fn passing_evidence_requires_complete_consistent_observation() {
        let evidence = clean_evidence();
        let bytes = serde_json::to_vec(&evidence).expect("serialize evidence");
        validate_passing_evidence_bytes(&bytes).expect("valid clean evidence");

        let mut non_passing = clean_evidence();
        non_passing.verdict = EvidenceVerdict::Unknown;
        let bytes = serde_json::to_vec(&non_passing).expect("serialize evidence");
        assert!(validate_passing_evidence_bytes(&bytes).is_err());

        let mut inconsistent = clean_evidence();
        inconsistent.advisory_findings.push(serde_json::json!({
            "rule": "leanness-ratchet",
            "severity": "error",
            "message": "not present in raw output",
            "location": {"file": "bonsai.toml"}
        }));
        let bytes = serde_json::to_vec(&inconsistent).expect("serialize evidence");
        assert!(validate_passing_evidence_bytes(&bytes).is_err());
    }

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

    #[test]
    fn unreadable_bonsai_binary_is_not_an_asked_check() {
        let run = BonsaiRun {
            executable: "bonsai".to_owned(),
            binary_digest: None,
            expected_source: "github:Quitetall/bonsai".to_owned(),
            expected_revision: "0".repeat(40),
            version: Some("bonsai 0.1.0".to_owned()),
            exit_code: Some(0),
            raw_output: Some(serde_json::json!({
                "tool": "bonsai",
                "version": "0.1.0",
                "findings": []
            })),
            stderr: String::new(),
            spawn_error: None,
        };
        assert!(!valid_bonsai_report(&run));
    }

    #[test]
    fn oversized_bonsai_line_is_not_well_formed() {
        let finding = serde_json::json!({
            "rule": "contract-forbid",
            "severity": "error",
            "message": "forbidden dependency",
            "location": {"file": "src/lib.rs", "line": 4_294_967_296u64}
        });
        assert!(!valid_finding(&finding));
    }

    #[test]
    fn bonsai_exit_code_must_match_error_findings() {
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
                "findings": []
            })),
            stderr: String::new(),
            spawn_error: None,
        };
        assert!(!valid_bonsai_report(&run));
    }

    #[test]
    fn malformed_architecture_output_is_unknown_not_fail() {
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
                "findings": [{"rule": "contract-seal", "severity": "error"}]
            })),
            stderr: String::new(),
            spawn_error: None,
        };
        assert!(!valid_bonsai_report(&run));
        assert_eq!(
            evidence_verdict(false, valid_bonsai_report(&run), true),
            EvidenceVerdict::Unknown
        );
    }

    #[cfg(unix)]
    mod qualification_plants {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        use std::path::{Path, PathBuf};
        use std::process::Command;
        use std::sync::atomic::{AtomicUsize, Ordering};

        use camino::{Utf8Path, Utf8PathBuf};

        use super::{EvidenceVerdict, Repository, check};

        static FIXTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);

        #[derive(Clone, Copy)]
        enum BonsaiOutput {
            Clean,
            ArchitectureError,
            AdvisoryError,
            MalformedArchitecture,
        }

        struct Fixture {
            source: PathBuf,
            root: Utf8PathBuf,
            scratch: PathBuf,
        }

        impl Fixture {
            fn new() -> Self {
                let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .and_then(Path::parent)
                    .expect("workspace root")
                    .to_owned();
                let unique = format!(
                    "openwarrant-bonsai-qualification-{}-{}",
                    std::process::id(),
                    FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
                );
                let scratch = std::env::temp_dir().join(unique);
                let root = scratch.join("worktree");
                fs::create_dir_all(&scratch).expect("fixture directory");
                run_git(
                    &source,
                    &[
                        "worktree",
                        "add",
                        "--detach",
                        root.to_str().expect("UTF-8 path"),
                        "HEAD",
                    ],
                );
                run_git(
                    &root,
                    &[
                        "remote",
                        "set-url",
                        "origin",
                        "https://github.com/Quitetall/OpenWarrant.git",
                    ],
                );
                let base = git_output(&root, &["rev-parse", "HEAD"]);
                run_git(&root, &["update-ref", "refs/remotes/origin/main", &base]);
                Self {
                    source,
                    root: Utf8PathBuf::from_path_buf(root).expect("UTF-8 path"),
                    scratch,
                }
            }

            fn repo(&self) -> Repository {
                Repository::discover(Some(self.root.clone())).expect("fixture repository")
            }

            fn base(&self) -> String {
                git_output(self.root.as_std_path(), &["rev-parse", "origin/main"])
            }

            fn head(&self) -> String {
                git_output(self.root.as_std_path(), &["rev-parse", "HEAD"])
            }

            fn commit(&self, path: &str, contents: &str) {
                let target = self.root.join(path);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).expect("fixture parent directory");
                }
                fs::write(&target, contents).expect("fixture source");
                run_git(self.root.as_std_path(), &["add", path]);
                run_git(
                    self.root.as_std_path(),
                    &[
                        "-c",
                        "user.name=Qualification Plant",
                        "-c",
                        "user.email=qualification@example.invalid",
                        "commit",
                        "-m",
                        "qualification fixture",
                    ],
                );
            }

            fn bonsai(&self, output: BonsaiOutput) -> Utf8PathBuf {
                let binary = self.scratch.join("bonsai-fixture");
                let (report, status) = match output {
                    BonsaiOutput::Clean => {
                        (r#"{"tool":"bonsai","version":"fixture","findings":[]}"#, 0)
                    }
                    BonsaiOutput::ArchitectureError => (
                        r#"{"tool":"bonsai","version":"fixture","findings":[{"rule":"contract-forbid","severity":"error","message":"forbidden dependency","location":{"file":"crates/openwarrant-cli/src/fixture.rs","line":1}}]}"#,
                        1,
                    ),
                    BonsaiOutput::AdvisoryError => (
                        r#"{"tool":"bonsai","version":"fixture","findings":[{"rule":"leanness-ratchet","severity":"error","message":"advisory regression","location":{"file":"crates/openwarrant-cli/src/fixture.rs","line":1}}]}"#,
                        1,
                    ),
                    BonsaiOutput::MalformedArchitecture => (
                        r#"{"tool":"bonsai","version":"fixture","findings":[{"rule":"contract-forbid","severity":"error"}]}"#,
                        1,
                    ),
                };
                fs::write(
                    &binary,
                    format!(
                        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf 'bonsai fixture\\n'\n  exit 0\nfi\nprintf '%s\\n' '{report}'\nexit {status}\n"
                    ),
                )
                .expect("fixture Bonsai binary");
                fs::set_permissions(&binary, fs::Permissions::from_mode(0o700))
                    .expect("fixture Bonsai permissions");
                Utf8PathBuf::from_path_buf(binary).expect("UTF-8 path")
            }
        }

        impl Drop for Fixture {
            fn drop(&mut self) {
                let root = self.root.as_str();
                let _ = Command::new("git")
                    .arg("-C")
                    .arg(&self.source)
                    .args(["worktree", "remove", "--force", root])
                    .status();
                let _ = fs::remove_dir_all(&self.scratch);
            }
        }

        fn run_git(root: &Path, args: &[&str]) {
            let output = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()
                .expect("run git");
            assert!(
                output.status.success(),
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        fn git_output(root: &Path, args: &[&str]) -> String {
            let output = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()
                .expect("run git");
            assert!(output.status.success(), "git {} failed", args.join(" "));
            String::from_utf8(output.stdout)
                .expect("UTF-8 git output")
                .trim()
                .to_owned()
        }

        #[test]
        fn pilot_qualification_plants_are_observed() {
            let scoped = Fixture::new();
            scoped.commit(
                "crates/openwarrant-cli/src/qualification_fixture.rs",
                "// scope-covered qualification change\n",
            );
            let evidence = check(
                &scoped.repo(),
                "OW-WAR-0050",
                &scoped.base(),
                &scoped.head(),
                Utf8Path::new(scoped.bonsai(BonsaiOutput::Clean).as_str()),
            )
            .expect("scope-covered candidate reports evidence");
            assert_eq!(evidence.verdict, EvidenceVerdict::Pass);

            let dangling_obligation = Fixture::new();
            let scope_path = dangling_obligation
                .root
                .join("docs/warrants/OW-WAR-0050/scope.toml");
            let altered_scope = fs::read_to_string(&scope_path)
                .expect("read qualification scope")
                .replacen("OBL-001", "OBL-999", 1);
            dangling_obligation.commit("docs/warrants/OW-WAR-0050/scope.toml", &altered_scope);
            let report = crate::check::run(&dangling_obligation.repo(), Some("OW-WAR-0050"), false)
                .expect("scope check completes");
            assert!(report.diagnostics.iter().any(|diagnostic| {
                diagnostic.rule == "bonsai-scope.invalid"
                    && diagnostic
                        .message
                        .contains("undeclared obligation \"OBL-999\"")
            }));

            let out_of_scope = Fixture::new();
            out_of_scope.commit("outside-warrant.txt", "must be refused\n");
            let evidence = check(
                &out_of_scope.repo(),
                "OW-WAR-0050",
                &out_of_scope.base(),
                &out_of_scope.head(),
                Utf8Path::new(out_of_scope.bonsai(BonsaiOutput::Clean).as_str()),
            )
            .expect("out-of-scope candidate still produces evidence");
            assert_eq!(evidence.verdict, EvidenceVerdict::Fail);
            assert_eq!(evidence.scope_findings[0].path, "outside-warrant.txt");

            let policy_drift = Fixture::new();
            policy_drift.commit("bonsai.toml", "# changed policy bytes\n");
            let error = check(
                &policy_drift.repo(),
                "OW-WAR-0050",
                &policy_drift.base(),
                &policy_drift.head(),
                Utf8Path::new(policy_drift.bonsai(BonsaiOutput::Clean).as_str()),
            )
            .expect_err("policy digest drift must refuse evidence");
            assert!(error.to_string().contains("policy digest differs"));

            let non_head = Fixture::new();
            non_head.commit(
                "crates/openwarrant-cli/src/non_head_fixture.rs",
                "// candidate differs from checkout\n",
            );
            let error = check(
                &non_head.repo(),
                "OW-WAR-0050",
                &non_head.base(),
                &non_head.base(),
                Utf8Path::new(non_head.bonsai(BonsaiOutput::Clean).as_str()),
            )
            .expect_err("non-HEAD candidate must refuse evidence");
            assert!(error.to_string().contains("is not checked out"));

            let unavailable = Fixture::new();
            unavailable.commit(
                "crates/openwarrant-cli/src/unavailable_fixture.rs",
                "// unavailable Bonsai still has scoped change\n",
            );
            let evidence = check(
                &unavailable.repo(),
                "OW-WAR-0050",
                &unavailable.base(),
                &unavailable.head(),
                Utf8Path::new(
                    unavailable
                        .scratch
                        .join("missing-bonsai")
                        .to_str()
                        .expect("UTF-8 path"),
                ),
            )
            .expect("unavailable Bonsai produces unknown evidence");
            assert_eq!(evidence.verdict, EvidenceVerdict::Unknown);
            assert!(evidence.bonsai.spawn_error.is_some());

            let architecture = Fixture::new();
            architecture.commit(
                "crates/openwarrant-cli/src/architecture_fixture.rs",
                "// architecture finding candidate\n",
            );
            let evidence = check(
                &architecture.repo(),
                "OW-WAR-0050",
                &architecture.base(),
                &architecture.head(),
                Utf8Path::new(
                    architecture
                        .bonsai(BonsaiOutput::ArchitectureError)
                        .as_str(),
                ),
            )
            .expect("architecture finding produces evidence");
            assert_eq!(evidence.verdict, EvidenceVerdict::Fail);
            assert_eq!(evidence.architecture_findings.len(), 1);

            let advisory = Fixture::new();
            advisory.commit(
                "crates/openwarrant-cli/src/advisory_fixture.rs",
                "// advisory finding candidate\n",
            );
            let evidence = check(
                &advisory.repo(),
                "OW-WAR-0050",
                &advisory.base(),
                &advisory.head(),
                Utf8Path::new(advisory.bonsai(BonsaiOutput::AdvisoryError).as_str()),
            )
            .expect("advisory finding produces evidence");
            assert_eq!(evidence.verdict, EvidenceVerdict::Pass);
            assert_eq!(evidence.architecture_findings.len(), 0);
            assert_eq!(evidence.advisory_findings.len(), 1);

            let malformed = Fixture::new();
            malformed.commit(
                "crates/openwarrant-cli/src/malformed_fixture.rs",
                "// malformed Bonsai candidate\n",
            );
            let evidence = check(
                &malformed.repo(),
                "OW-WAR-0050",
                &malformed.base(),
                &malformed.head(),
                Utf8Path::new(
                    malformed
                        .bonsai(BonsaiOutput::MalformedArchitecture)
                        .as_str(),
                ),
            )
            .expect("malformed output produces evidence");
            assert_eq!(evidence.verdict, EvidenceVerdict::Unknown);
            assert!(evidence.architecture_findings.is_empty());
            assert!(evidence.advisory_findings.is_empty());
        }
    }
}
