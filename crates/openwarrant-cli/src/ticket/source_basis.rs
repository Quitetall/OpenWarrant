// SPDX-License-Identifier: Apache-2.0
//! Bounded local source observations. These identify repository bytes, not
//! process isolation, external services, credentials or independent assurance.
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const SCHEMA: &str = "oh.war/local-check-basis/v1";
const MAX_FILES: usize = 20_000;
const MAX_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Basis {
    pub schema: String,
    pub commit: Option<String>,
    pub digest: String,
}

fn runtime(root: &Utf8Path, path: &Utf8Path, evidence: &Utf8Path) -> bool {
    path.starts_with(evidence)
        || path.starts_with(root.join(".git"))
        || path.starts_with(root.join(".openwarrant/state"))
        || path == root.join(".openwarrant/go-packet.json")
        || path == root.join(".openwarrant/session.json")
}

fn file_hash(root: &Utf8Path, path: &Utf8Path, budget: &mut usize) -> Result<String, String> {
    let size = std::fs::symlink_metadata(path)
        .map_err(|e| format!("{path}: {e}"))?
        .len();
    if size > *budget as u64 {
        return Err("local source byte budget exceeded (64 MiB); commit source to Git before rerunning the check".into());
    }
    let relative = path.strip_prefix(root).map_err(|e| e.to_string())?;
    let bytes =
        crate::vfs::read_anchored_limited(root.as_std_path(), relative.as_std_path(), *budget)
            .map_err(|e| format!("{path}: {e}"))?;
    *budget = budget
        .checked_sub(bytes.len())
        .ok_or("local source byte budget exceeded")?;
    Ok(openwarrant_core::format::sha256(&bytes))
}

fn contract_file(root: &Utf8Path, evidence: &Utf8Path) -> Option<Utf8PathBuf> {
    [
        evidence.join("ticket.md"),
        evidence.join("warrant.md"),
        evidence
            .parent()?
            .join(format!("{}.md", evidence.file_name()?)),
    ]
    .into_iter()
    .find(|p| p.starts_with(root) && p.is_file())
}

/// Progress bits and other checks have their own subject/definition revisions.
/// Keep the governing prose and constraints in the code basis without making
/// a checkbox update invalidate its own observation.
fn contract_digest(root: &Utf8Path, path: &Utf8Path) -> Result<String, String> {
    let relative = path.strip_prefix(root).map_err(|e| e.to_string())?;
    let bytes =
        crate::vfs::read_anchored_limited(root.as_std_path(), relative.as_std_path(), MAX_BYTES)
            .map_err(|e| e.to_string())?;
    let doc = openwarrant_core::format::parse(&bytes);
    if doc.refused() || doc.unknown() {
        return Err("work contract cannot be read as current".into());
    }
    let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    let mut units = Vec::new();
    for u in doc
        .units
        .iter()
        .filter(|u| !matches!(u.record_type.as_str(), "check" | "kpi" | "item"))
    {
        let item_lines: std::collections::BTreeSet<_> = u
            .items
            .iter()
            .filter_map(|i| text.get(i.start..i.end))
            .map(|l| l.trim_end_matches(['\r', '\n']))
            .collect();
        let body = u
            .body
            .lines()
            .filter(|l| !item_lines.contains(*l))
            .collect::<Vec<_>>()
            .join("\n");
        let relations: Vec<_> = u
            .relations
            .iter()
            .map(|r| (&r.kind, &r.target.id, &r.target.pin))
            .collect();
        units.push(serde_json::json!({"id":u.id,"type":u.record_type,"title":u.title,"fields":u.fields,"ext":u.ext,"relations":relations,"body":body}));
    }
    let value = serde_json::json!({"head":openwarrant_core::format::sha256(&bytes[..doc.head_end]),"units":units});
    let canonical = serde_jcs::to_vec(&value).map_err(|e| e.to_string())?;
    Ok(openwarrant_core::format::sha256(&canonical))
}

fn walk(
    root: &Utf8Path,
    evidence: &Utf8Path,
    here: &Utf8Path,
    paths: &mut Vec<Utf8PathBuf>,
    nodes: &mut usize,
    depth: usize,
) -> Result<(), String> {
    if depth > 64 {
        return Err("local source directory depth exceeds 64".into());
    }
    for entry in std::fs::read_dir(here).map_err(|e| format!("{here}: {e}"))? {
        *nodes = nodes
            .checked_sub(1)
            .ok_or("local source entry budget exceeded (20,000)")?;
        let entry = entry.map_err(|e| e.to_string())?;
        let path =
            Utf8PathBuf::from_path_buf(entry.path()).map_err(|_| "source path is not UTF-8")?;
        if runtime(root, &path, evidence) {
            continue;
        }
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            let tag = path.join("CACHEDIR.TAG");
            if crate::vfs::read_anchored_limited(
                root.as_std_path(),
                tag.strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .as_std_path(),
                4096,
            )
            .is_ok_and(|s| s.starts_with(b"Signature: 8a477f597d28d172789f06886806bc55"))
            {
                continue;
            }
            walk(root, evidence, &path, paths, nodes, depth + 1)?;
        } else {
            if paths.len() >= MAX_FILES {
                return Err("local source file budget exceeded (20,000)".into());
            }
            paths.push(path);
        }
    }
    Ok(())
}

/// Git captures the committed base, tracked differences and nonignored new
/// files. A non-Git workspace uses bounded regular files, excluding declared
/// cache directories. The check's evidence output is excluded from its inputs.
pub fn capture(root: &Utf8Path, evidence: &Utf8Path) -> Result<Basis, String> {
    let canonical = root.canonicalize_utf8().map_err(|e| e.to_string())?;
    let evidence = evidence
        .strip_prefix(root)
        .map(|p| canonical.join(p))
        .unwrap_or_else(|_| evidence.to_owned());
    let root = canonical.as_path();
    let evidence = evidence.as_path();
    let contract = contract_file(root, evidence);
    let contract_hash = contract
        .as_deref()
        .map(|p| contract_digest(root, p))
        .transpose()?;
    let commit = super::ladder::head_commit(root);
    let mut files = Vec::new();
    let mut patch = String::new();
    if commit.is_some() {
        let mut excluded = vec![
            ":(exclude,literal).openwarrant/state".to_owned(),
            ":(exclude,literal).openwarrant/go-packet.json".to_owned(),
            ":(exclude,literal).openwarrant/session.json".to_owned(),
        ];
        if let Ok(relative) = evidence.strip_prefix(root) {
            excluded.push(format!(":(exclude,literal){relative}"));
        }
        if let Some(path) = &contract {
            excluded.push(format!(
                ":(exclude,literal){}",
                path.strip_prefix(root).map_err(|e| e.to_string())?
            ));
        }
        let mut args = vec![
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--binary",
            "HEAD",
            "--",
            ".",
        ];
        args.extend(excluded.iter().map(String::as_str));
        let diff = crate::go::git::git_raw(root, &args)
            .map_err(|e| format!("local source diff unavailable: {e:?}"))?;
        if !diff.success {
            return Err(format!(
                "local source diff unavailable: {}",
                String::from_utf8_lossy(&diff.stderr)
            ));
        }
        patch = openwarrant_core::format::sha256(&diff.stdout);
        let mut args = vec![
            "ls-files",
            "--others",
            "--exclude-standard",
            "-z",
            "--",
            ".",
        ];
        args.extend(excluded.iter().map(String::as_str));
        let listed = crate::go::git::git_raw(root, &args)
            .map_err(|e| format!("local source listing unavailable: {e:?}"))?;
        if !listed.success {
            return Err(format!(
                "local source listing unavailable: {}",
                String::from_utf8_lossy(&listed.stderr)
            ));
        }
        let names =
            std::str::from_utf8(&listed.stdout).map_err(|_| "source paths are not UTF-8")?;
        for name in names.split('\0').filter(|s| !s.is_empty()) {
            if files.len() >= MAX_FILES {
                return Err("local source file budget exceeded (20,000)".into());
            }
            let relative = Utf8Path::new(name);
            if relative.is_absolute() || relative.components().any(|c| c.as_str() == "..") {
                return Err("local source listing escaped workspace".into());
            }
            files.push(root.join(relative));
        }
    } else {
        let mut nodes = MAX_FILES;
        walk(root, evidence, root, &mut files, &mut nodes, 0)?;
    }
    let mut budget = MAX_BYTES;
    let mut entries = BTreeMap::new();
    for path in files {
        if runtime(root, &path, evidence) || contract.as_ref() == Some(&path) {
            continue;
        }
        let key = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_string();
        entries.insert(key, file_hash(root, &path, &mut budget)?);
    }
    let value = serde_json::json!({"schema":SCHEMA,"commit":commit,"contract":contract_hash,"tracked_diff":patch,"files":entries});
    let canonical = serde_jcs::to_vec(&value).map_err(|e| e.to_string())?;
    Ok(Basis {
        schema: SCHEMA.into(),
        commit,
        digest: openwarrant_core::format::sha256(&canonical),
    })
}
