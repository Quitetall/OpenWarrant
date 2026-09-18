// SPDX-License-Identifier: Apache-2.0
//! Bounded local Git history capture. No checkout, signing, lazy fetch or authority act.
use crate::repo::Repository;
use openwarrant_compiler::preservation::Error;
use std::{
    collections::BTreeMap,
    io::Read,
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

fn git(repo: &Repository, args: &[&str], limit: usize) -> Result<Vec<u8>, Error> {
    let mut child = Command::new("git")
        .current_dir(&repo.root)
        .args(["--no-pager", "--no-replace-objects"])
        .args(args)
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| Error(format!("history Git unavailable: {e}")))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error("history stdout unavailable".into()))?;
    let (send, receive) = mpsc::sync_channel(1);
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = send.send(result);
    });
    let bytes = match receive.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(bytes)) if bytes.len() <= limit => bytes,
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err(Error(
                "history read failed, timed out or exceeded limit".into(),
            ));
        }
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| Error(e.to_string()))? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err(Error(
                "history process did not terminate within limit".into(),
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let _ = reader.join();
    if !status.success() {
        return Err(Error("required local Git history unavailable".into()));
    }
    Ok(bytes)
}
fn text(bytes: Vec<u8>) -> Result<String, Error> {
    String::from_utf8(bytes).map_err(|_| Error("history metadata is not UTF-8".into()))
}
fn oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

pub(super) fn capture(
    repo: &Repository,
    relative: &str,
    byte_limit: usize,
    record_limit: usize,
) -> Result<BTreeMap<String, Vec<u8>>, Error> {
    if text(git(repo, &["rev-parse", "--is-shallow-repository"], 16)?)?.trim() != "false" {
        return Err(Error(
            "shallow history cannot establish preservation".into(),
        ));
    }
    if !text(git(repo, &["rev-parse", "--show-prefix"], 4096)?)?
        .trim()
        .is_empty()
    {
        return Err(Error("history capture requires repository root".into()));
    }
    let head = text(git(repo, &["rev-parse", "--verify", "HEAD^{commit}"], 128)?)?
        .trim()
        .to_owned();
    if !oid(&head) {
        return Err(Error("invalid history HEAD identity".into()));
    }
    let commits = text(git(
        repo,
        &[
            "log",
            "--full-history",
            "--format=%H",
            "--max-count=257",
            &head,
            "--",
            relative,
        ],
        32768,
    )?)?;
    let mut commits: Vec<_> = commits.lines().collect();
    if commits.len() > 256 {
        return Err(Error("history exceeds 256-commit bound".into()));
    }
    commits.sort_unstable();
    let mut records = BTreeMap::new();
    let mut remaining = byte_limit;
    let mut index = Vec::new();
    for commit in commits {
        if !oid(commit) {
            return Err(Error("invalid history commit identity".into()));
        }
        let prefix = format!("__ow_archive__/history/{commit}");
        let commit_bytes = git(repo, &["cat-file", "commit", commit], remaining)?;
        remaining = remaining
            .checked_sub(commit_bytes.len())
            .ok_or_else(|| Error("history byte limit exceeded".into()))?;
        records.insert(format!("{prefix}/commit.txt"), commit_bytes);
        let tree = git(
            repo,
            &["ls-tree", "-r", "-z", "--full-tree", commit, "--", relative],
            remaining.min(2 * 1024 * 1024),
        )?;
        let mut files = Vec::new();
        for entry in tree.split(|b| *b == 0).filter(|b| !b.is_empty()) {
            let entry =
                std::str::from_utf8(entry).map_err(|_| Error("non-UTF8 historical path".into()))?;
            let (metadata, source) = entry
                .split_once('\t')
                .ok_or_else(|| Error("invalid history tree entry".into()))?;
            let fields: Vec<_> = metadata.split(' ').collect();
            if fields.len() != 3
                || !matches!(fields[0], "100644" | "100755")
                || fields[1] != "blob"
                || !oid(fields[2])
            {
                return Err(Error("historical source is not a regular Git blob".into()));
            }
            if !source.starts_with(&format!("{relative}/")) {
                return Err(Error("historical path escaped requested Warrant".into()));
            }
            if records.len() >= record_limit {
                return Err(Error("history record count exceeds limit".into()));
            }
            let size: usize = text(git(repo, &["cat-file", "-s", fields[2]], 64)?)?
                .trim()
                .parse()
                .map_err(|_| Error("invalid historical blob size".into()))?;
            if size > remaining {
                return Err(Error("history byte limit exceeded".into()));
            }
            let bytes = git(repo, &["cat-file", "blob", fields[2]], size)?;
            if bytes.len() != size {
                return Err(Error("historical blob size mismatch".into()));
            }
            remaining -= bytes.len();
            let record = format!("{prefix}/{source}");
            records.insert(record.clone(), bytes);
            files.push(serde_json::json!({"source":source,"mode":fields[0],"git_blob":fields[2],"record":record}));
        }
        index.push(serde_json::json!({"commit":commit,"commit_record":format!("{prefix}/commit.txt"),"files":files}));
    }
    let manifest = openwarrant_compiler::to_canonical_bytes(&serde_json::json!({"schema":"oh.war/preservation-history/v1-draft.1","head":head,"reachable_from":"HEAD","warrant_path":relative,"commits":index,"other_refs_included":false})).map_err(|e| Error(e.to_string()))?;
    if manifest.len() > remaining || records.len() >= record_limit {
        return Err(Error("history manifest exceeds limits".into()));
    }
    records.insert("__ow_archive__/history.json".into(), manifest);
    Ok(records)
}
