// SPDX-License-Identifier: Apache-2.0
//! Read retained, record-bound contract snapshots. No checkout or signing act.
use crate::{
    authorize::AuthorizationRecord,
    repo::{RepoError, Repository},
    sdk::wire,
};
use openwarrant_compiler::ir::WarIr;
use serde_json::Value;
use std::{
    io::Read,
    process::{Command, Stdio},
};

fn error(s: impl Into<String>) -> RepoError {
    RepoError::Message(s.into())
}
fn git(repo: &Repository, args: &[&str], limit: usize) -> Result<Vec<u8>, RepoError> {
    let mut child = Command::new("git")
        .current_dir(&repo.root)
        .args(["--no-pager", "--no-replace-objects"])
        .args(args)
        .env("GIT_NO_LAZY_FETCH", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| error(format!("history Git command unavailable: {e}")))?;
    let mut bytes = Vec::new();
    let result = child
        .stdout
        .take()
        .expect("piped stdout")
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes);
    if result.is_err() || bytes.len() > limit {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error(
            "history Git output exceeds resource limit or could not be read",
        ));
    }
    if !child.wait().map_err(|e| error(e.to_string()))?.success() {
        return Err(error(
            "history Git command failed; retained evidence unavailable",
        ));
    }
    Ok(bytes)
}
fn text(bytes: Vec<u8>) -> Result<String, RepoError> {
    String::from_utf8(bytes).map_err(|_| error("history Git output is not UTF-8"))
}
fn blob(
    repo: &Repository,
    commit: &str,
    path: &str,
    budget: &mut usize,
) -> Result<Option<Vec<u8>>, RepoError> {
    let entry = text(git(repo, &["ls-tree", commit, "--", path], 16_384)?)?;
    if entry.is_empty() {
        return Ok(None);
    }
    let fields: Vec<_> = entry.split_whitespace().take(3).collect();
    if fields.len() != 3 || !matches!(fields[0], "100644" | "100755") || fields[1] != "blob" {
        return Err(error("history source must be a regular Git blob"));
    }
    let size = text(git(repo, &["cat-file", "-s", fields[2]], 64)?)?
        .trim()
        .parse::<usize>()
        .map_err(|_| error("invalid Git blob size"))?;
    if size > 4 * 1024 * 1024 {
        return Err(error("history source exceeds 4 MiB"));
    }
    *budget = budget
        .checked_sub(size)
        .ok_or_else(|| error("history scan exceeds 32 MiB"))?;
    Ok(Some(git(repo, &["cat-file", "blob", fields[2]], size)?))
}

/// Preserve supported semantics before applying the current digest definition.
/// Unknown extension payloads round-trip; unknown typed fields cannot vanish.
pub fn parse_ir(value: &Value) -> Result<WarIr, RepoError> {
    fn retained(original: &Value, typed: &Value) -> bool {
        match (original, typed) {
            (Value::Object(a), Value::Object(b)) => a
                .iter()
                .all(|(key, value)| b.get(key).is_some_and(|typed| retained(value, typed))),
            (Value::Array(a), Value::Array(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(a, b)| retained(a, b))
            }
            _ => original == typed,
        }
    }
    let ir: WarIr = serde_json::from_value(value.clone())
        .map_err(|e| error(format!("invalid contract IR: {e}")))?;
    if ir.api_version != "oh.war/v1" || ir.kind != "work_authorization_record" {
        return Err(error("unsupported contract IR identity or version"));
    }
    let typed = serde_json::to_value(&ir).map_err(|e| error(e.to_string()))?;
    if !retained(value, &typed) {
        return Err(error("unsupported IR field or non-round-tripping value"));
    }
    Ok(ir)
}

pub fn selector(s: &str) -> Result<Option<u32>, RepoError> {
    let Some(number) = s.strip_prefix("contract:") else {
        return Ok(None);
    };
    let revision = number
        .parse::<u32>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == number)
        .ok_or_else(|| {
            error("contract selector requires canonical positive revision: contract:N")
        })?;
    Ok(Some(revision))
}

pub fn resolve(
    repo: &Repository,
    alias: &str,
    revision: u32,
) -> Result<(Value, String), RepoError> {
    if text(git(repo, &["rev-parse", "--is-shallow-repository"], 16)?)?.trim() != "false" {
        return Err(error("contract history unavailable: shallow repository"));
    }
    let head = text(git(repo, &["rev-parse", "--verify", "HEAD^{commit}"], 128)?)?;
    let prefix = text(git(repo, &["rev-parse", "--show-prefix"], 4096)?)?;
    // --full-tree makes names relative to Git's root even for a nested project.
    if !prefix.trim().is_empty() {
        return Err(error(
            "contract history requires OpenWarrant at Git repository root",
        ));
    }
    let dir = repo.warrant_dir(alias)?;
    let relative = dir
        .strip_prefix(&repo.root)
        .map_err(|e| error(e.to_string()))?;
    let ir_path = format!("{relative}/generated/WAR.json");
    let auth_path = format!("{relative}/authorization.toml");
    let commits = text(git(
        repo,
        &[
            "log",
            "--full-history",
            "--format=%H",
            "--max-count=1025",
            head.trim(),
            "--",
            &ir_path,
            &auth_path,
        ],
        80_000,
    )?)?;
    let commits: Vec<_> = commits.lines().collect();
    if commits.len() > 1024 {
        return Err(error("contract history exceeds 1024 snapshot scan limit"));
    }
    let mut budget = 32 * 1024 * 1024;
    let mut found: Option<(Value, String, String)> = None;
    for commit in commits {
        let Some(auth) = blob(repo, commit, &auth_path, &mut budget)? else {
            continue;
        };
        let auth: AuthorizationRecord =
            toml::from_str(std::str::from_utf8(&auth).map_err(|e| error(e.to_string()))?)
                .map_err(|e| error(format!("invalid retained authorization at {commit}: {e}")))?;
        if auth.revision.revision != revision {
            continue;
        }
        if auth.schema != "oh.war/authorization/v1"
            || auth.warrant != alias
            || auth.revision.state != openwarrant_core::contract::RevisionState::Authorized
        {
            return Err(error("retained authorization identity or state mismatch"));
        }
        let bytes = blob(repo, commit, &ir_path, &mut budget)?
            .ok_or_else(|| error("retained contract IR unavailable"))?;
        let value = wire::decode_value(&bytes).map_err(|e| error(e.to_string()))?;
        let ir = parse_ir(&value)?;
        if ir.api_version != "oh.war/v1"
            || ir.kind != "work_authorization_record"
            || ir.identity.local_alias != alias
        {
            return Err(error("retained contract IR identity or version mismatch"));
        }
        let digest = ir.contract_digest().map_err(|e| error(e.to_string()))?;
        if digest != auth.revision.contract_digest || ir.contract_coverage != auth.revision.coverage
        {
            return Err(error(format!(
                "retained contract digest mismatch at {commit}"
            )));
        }
        // Legacy compilers emitted IR revision 1 even after authorization changed.
        // The retained authorization supplies the revision number; never rewrite IR.
        if let Some((previous, _, previous_digest)) = &found {
            if previous_digest != &digest
                || previous["identity"]["uuid"] != value["identity"]["uuid"]
            {
                return Err(error(format!(
                    "ambiguous contract:{revision}: conflicting retained records"
                )));
            }
        } else {
            found = Some((value, commit.into(), digest));
        }
    }
    found.map(|(value, commit, digest)| (value, format!("contract:{revision} at {commit}, digest {digest}; record-bound comparison, not signature verification")))
        .ok_or_else(|| error(format!("contract:{revision} unavailable in retained history of {}", head.trim())))
}
