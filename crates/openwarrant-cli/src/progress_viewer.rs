// SPDX-License-Identifier: Apache-2.0
//! A read-only consumer of live legacy status and explicit, untrusted work reports.
use crate::{
    overview,
    repo::{RepoError, Repository},
    status,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
mod roadmap;
mod server;
pub(crate) mod source;

const REPORT_SCHEMA: &str = "oh.war/viewer-work-report/v1";
const HTML: &str = include_str!("progress_viewer/view.html");
const MARKER: &str = "<!-- OpenWarrant progress viewer snapshot v1 -->";
const REPORT_LIMIT: usize = 64 * 1024;
const SOURCE_LIMIT: usize = 2 * 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum WorkState {
    InProgress,
    Completed,
    Blocked,
    Failed,
    Cancelled,
    Unknown,
}
/// Viewer-local attributed display claims, not an SDK lifecycle or assurance record.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WorkReport {
    schema: String,
    warrant: String,
    work_state: WorkState,
    reported_by: String,
    revision: String,
    summary: String,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    evidence: Vec<String>,
    #[serde(default)]
    next_steps: Vec<String>,
}
#[derive(Serialize)]
struct ReportEntry {
    report: Option<WorkReport>,
    error: Option<String>,
    source: String,
    source_digest: Option<String>,
}
#[derive(Serialize)]
pub(super) struct Snapshot {
    schema: &'static str,
    as_of_unix_ms: u128,
    source_revision: String,
    record_digest: String,
    legacy: overview::Overview,
    reports: BTreeMap<String, ReportEntry>,
    roadmap: Option<roadmap::Roadmap>,
    stage_frontier: Option<crate::frontier::Frontier>,
    stage_frontier_error: Option<String>,
    /// Relative repository names mapped to links. Live server serves only these sources.
    links: BTreeMap<String, String>,
    #[serde(skip)]
    sources: BTreeMap<String, PathBuf>,
    #[serde(skip)]
    root: PathBuf,
}
fn err(message: impl Into<String>) -> RepoError {
    RepoError::Message(message.into())
}
fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub(super) fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
fn bounded_read(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let parent = path
        .parent()
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let name = path.file_name().ok_or("Missing source filename")?;
    source::read(&parent, Path::new(name), limit)
}

fn relative_source(root: &Path, name: &str) -> Result<PathBuf, String> {
    if name.is_empty()
        || Path::new(name)
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
    {
        return Err("Source path must be relative without traversal".into());
    }
    let path = root.join(name);
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    if !canonical.starts_with(root) || !canonical.is_file() {
        return Err("Source must be a file inside repository".into());
    }
    Ok(canonical)
}
fn file_url(path: &Path) -> String {
    let mut result = String::from("file://");
    for b in path.to_string_lossy().as_bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~".contains(b) {
            result.push(*b as char)
        } else {
            result.push_str(&format!("%{b:02X}"));
        }
    }
    result
}
fn link(snapshot: &mut Snapshot, root: &Path, name: &str, live: bool) -> Result<(), String> {
    let path = relative_source(root, name)?;
    let key = sha(name.as_bytes());
    let url = if live {
        format!("/source/{key}")
    } else {
        file_url(&path)
    };
    snapshot.links.insert(name.into(), url);
    snapshot.sources.insert(key, path);
    Ok(())
}
fn read_report(root: &Path, path: &str, alias: &str) -> Result<(WorkReport, String), String> {
    let source = relative_source(root, path)?;
    let bytes = source::read(
        root,
        source.strip_prefix(root).map_err(|e| e.to_string())?,
        REPORT_LIMIT,
    )?;
    let report: WorkReport = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if report.schema != REPORT_SCHEMA || report.warrant != alias {
        return Err("Report schema or Warrant identity does not match".into());
    }
    if report.reported_by.is_empty()
        || report.reported_by.len() > 256
        || report.summary.is_empty()
        || report.summary.len() > 4096
        || !matches!(report.revision.len(), 40 | 64)
        || !report.revision.bytes().all(|c| c.is_ascii_hexdigit())
        || report.evidence.len() > 32
        || report.next_steps.len() > 32
        || report.next_steps.iter().any(|s| s.len() > 4096)
    {
        return Err("Invalid or over-limit work report fields".into());
    }
    for name in report.evidence.iter().chain(report.notes.iter()) {
        relative_source(root, name)?;
    }
    Ok((report, sha(&bytes)))
}
fn capture(repo: &Repository, live: bool) -> Result<Snapshot, RepoError> {
    let root = repo.root.canonicalize().map_err(|e| err(e.to_string()))?;
    let legacy = overview::build(status::build(repo)?, true);
    let git = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&root)
        .output()
        .map_err(|e| err(e.to_string()))?;
    let revision = if git.status.success() {
        String::from_utf8_lossy(&git.stdout).trim().to_owned()
    } else {
        "unknown".into()
    };
    let mut snapshot = Snapshot {
        schema: "oh.war/progress-view/v1",
        as_of_unix_ms: now(),
        source_revision: revision,
        record_digest: String::new(),
        legacy,
        reports: BTreeMap::new(),
        roadmap: None,
        stage_frontier: None,
        stage_frontier_error: None,
        links: BTreeMap::new(),
        sources: BTreeMap::new(),
        root: root.clone(),
    };
    let mut known_aliases = std::collections::BTreeSet::new();
    // The configured Warrant directories, not a hard-coded docs path, define scope.
    for dir in repo.warrant_dirs()? {
        let manifest = repo.load_warrant(&dir)?;
        let Some(basis) = manifest.basis else {
            continue;
        };
        let alias = basis.manifest.local_alias.to_string();
        known_aliases.insert(alias.clone());
        let path = dir.join("implementation/progress.json");
        if !path.try_exists().map_err(|e| err(e.to_string()))? {
            continue;
        }
        let relative = repo.relative(&path);
        let result = read_report(&root, &relative, &alias);
        let entry = match result {
            Ok((report, digest)) => {
                let links: Vec<_> = report
                    .evidence
                    .iter()
                    .chain(report.notes.iter())
                    .cloned()
                    .chain(std::iter::once(relative.clone()))
                    .collect();
                let mut problem = None;
                for name in links {
                    if let Err(e) = link(&mut snapshot, &root, &name, live) {
                        problem = Some(e);
                        break;
                    }
                }
                ReportEntry {
                    report: if problem.is_none() {
                        Some(report)
                    } else {
                        None
                    },
                    error: problem,
                    source: relative,
                    source_digest: Some(digest),
                }
            }
            Err(e) => ReportEntry {
                report: None,
                error: Some(e),
                source: relative,
                source_digest: None,
            },
        };
        snapshot.reports.insert(alias, entry);
    }
    // OW-ADR-0023: the roadmap record when the program has one; the authored
    // `view.json` only for a program that has not adopted the record.
    let from_record = match crate::roadmap_cmd::view(repo) {
        Ok((_, v)) => Some(roadmap::from_record(&v)),
        Err(_) => None,
    };
    let path = root.join("docs/roadmap/view.json");
    if from_record.is_some() || path.try_exists().map_err(|e| err(e.to_string()))? {
        let roadmap = match from_record {
            Some(r) => r,
            None => {
                let bytes = bounded_read(&path, SOURCE_LIMIT).map_err(err)?;
                let aliases = known_aliases.iter().map(String::as_str).collect();
                roadmap::parse(&bytes, &aliases).map_err(err)?
            }
        };
        for name in roadmap.nodes.iter().flat_map(|node| &node.documents) {
            link(&mut snapshot, &root, name, live).map_err(err)?;
        }
        snapshot.roadmap = Some(roadmap);
        match crate::frontier::run(repo, None) {
            Ok((report, frontier)) if report.is_ready() => snapshot.stage_frontier = Some(frontier),
            Ok((report, _)) => {
                snapshot.stage_frontier_error = Some(format!(
                    "{}: {:?}",
                    report.verdict_line(),
                    report.diagnostics
                ))
            }
            Err(error) => snapshot.stage_frontier_error = Some(error.to_string()),
        }
    }
    snapshot.record_digest = sha(&serde_json::to_vec(&(
        &snapshot.legacy,
        &snapshot.reports,
        &snapshot.roadmap,
        &snapshot.stage_frontier,
        &snapshot.stage_frontier_error,
    ))
    .map_err(|e| err(e.to_string()))?);
    Ok(snapshot)
}
fn html(snapshot: &Snapshot, live: bool, interval: u64) -> Result<String, RepoError> {
    let json = serde_json::to_string(snapshot)
        .map_err(|e| err(e.to_string()))?
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029");
    Ok(HTML
        .replace(
            "__ROADMAP_MODEL__",
            include_str!("progress_viewer/roadmap-model.js"),
        )
        .replace("__LIVE__", if live { "true" } else { "false" })
        .replace("__INTERVAL__", &interval.to_string())
        .replace("__SNAPSHOT__", &json))
}
/// Atomic, explicit snapshot output. Only a prior viewer snapshot may be replaced.
pub fn export(repo: &Repository, destination: &Path) -> Result<(), RepoError> {
    let snapshot = capture(repo, false)?;
    let contents = html(&snapshot, false, 5)?;
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| err(e.to_string()))?;
    if let Ok(metadata) = std::fs::symlink_metadata(destination) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(err(
                "Snapshot destination must be a regular file, not a symlink",
            ));
        }
        let existing = bounded_read(destination, 16 * 1024 * 1024).map_err(err)?;
        if !existing.starts_with(format!("<!doctype html>\n{MARKER}\n").as_bytes()) {
            return Err(err(
                "Refusing to overwrite a file that is not a progress viewer snapshot",
            ));
        }
    }
    use std::io::Write;
    let temporary = parent.join(format!(
        ".war-progress-{}-{}.tmp",
        std::process::id(),
        now()
    ));
    // A failed create_new means this path belongs to someone else. Never clean
    // it up unless this invocation successfully created and owns the file.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| err(e.to_string()))?;
    let result = (|| {
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temporary, destination)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(|e| err(e.to_string()))
}
pub fn serve(
    repo: Repository,
    port: u16,
    interval: u64,
    mode: crate::output::Mode,
) -> Result<(), RepoError> {
    server::serve(repo, port, interval, mode)
}

/// The same validated snapshot used by both HTML consumers. No files are written.
pub fn json_snapshot(repo: &Repository) -> Result<serde_json::Value, RepoError> {
    serde_json::to_value(capture(repo, true)?).map_err(|e| err(e.to_string()))
}
