// SPDX-License-Identifier: Apache-2.0
//! `war export --progress <dir>` — the progress bundle (slice D5).
//!
//! Everything a tracker needs, copied byte-for-byte from the committed
//! projections into one directory with a manifest of sha256 digests:
//! the corpus status (JSON, Markdown, HTML), the timeline, the pending set,
//! the SAS normative projection, and every Warrant's compiled `WAR.json`.
//! Nothing is rendered or rewritten here — a bundle that could differ from
//! the committed tree would be a second source of truth. §68's export of a
//! single Warrant with its evidence is untouched (`export.rs`).

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::digest::sha256_hex;
use serde::Serialize;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

pub const SCHEMA: &str = "oh.war/progress-bundle/v1";

#[derive(Debug, Clone, Serialize)]
pub struct BundleFile {
    /// Path inside the bundle.
    pub path: String,
    /// Repository-relative source it was copied from.
    pub source: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Manifest {
    pub schema: String,
    pub war_version: String,
    pub files: Vec<BundleFile>,
    /// sha256 over the sorted `path:sha256` lines — one digest for the bundle.
    pub bundle_digest: String,
}

/// Copy `source` (repository-relative) to `out/<path>`, recording it.
fn copy(
    repo: &Repository,
    out: &Utf8Path,
    source: &Utf8Path,
    path: &str,
    files: &mut Vec<BundleFile>,
) -> Result<(), RepoError> {
    let bytes = std::fs::read(source).map_err(|e| {
        RepoError::Message(format!(
            "progress.missing: {} is not in the tree ({e}); run `war compile` first",
            repo.relative(source)
        ))
    })?;
    let dest = out.join(path);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|source| RepoError::Io {
            context: format!("could not create {parent}"),
            source,
        })?;
    }
    std::fs::write(&dest, &bytes).map_err(|source| RepoError::Io {
        context: format!("could not write {dest}"),
        source,
    })?;
    files.push(BundleFile {
        path: path.to_owned(),
        source: repo.relative(source),
        sha256: sha256_hex(&bytes),
        bytes: bytes.len() as u64,
    });
    Ok(())
}

pub fn export(repo: &Repository, out: &Utf8Path) -> Result<Report, RepoError> {
    let mut report = Report::default();
    if out.exists()
        && std::fs::read_dir(out)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
    {
        report.push(Diagnostic::error(
            "progress.not-empty",
            out.to_string(),
            "the bundle directory exists and is not empty; a bundle is written whole into an empty \
             directory so nothing stale can sit beside it"
                .to_owned(),
        ));
        return Ok(report);
    }
    std::fs::create_dir_all(out).map_err(|source| RepoError::Io {
        context: format!("could not create {out}"),
        source,
    })?;
    let generated = repo
        .root
        .join(&repo.config.paths.warrants)
        .join("generated");
    let sas_gen = repo.root.join(&repo.config.paths.sas).join("generated");
    let mut files = Vec::new();
    for name in [
        "CORPUS_STATUS.json",
        "CORPUS_STATUS.md",
        "CORPUS_STATUS.html",
        "CORPUS_TIMELINE.json",
        "CORPUS_PENDING.json",
    ] {
        copy(repo, out, &generated.join(name), name, &mut files)?;
    }
    for name in ["NORMATIVE.md", "NORMATIVE.json"] {
        let src = sas_gen.join(name);
        if src.is_file() {
            copy(repo, out, &src, name, &mut files)?;
        } else {
            report.push(Diagnostic::warn(
                "progress.no-sas-projection",
                repo.relative(&src),
                "no SAS normative projection to bundle (no SAS document, or not compiled)"
                    .to_owned(),
            ));
        }
    }
    let mut dirs = repo.warrant_dirs()?;
    dirs.sort();
    let mut warrants = 0usize;
    for dir in dirs {
        let src = dir.join("generated/WAR.json");
        if !src.is_file() {
            continue;
        }
        let alias = dir.file_name().unwrap_or("").to_owned();
        // A directory name is an alias; one that is not (`..`, a separator) is
        // not bundled, so no bundle path can leave the bundle.
        if alias.is_empty() || alias.contains("..") || alias.contains('/') {
            continue;
        }
        copy(
            repo,
            out,
            &src,
            &format!("warrants/{alias}/WAR.json"),
            &mut files,
        )?;
        warrants += 1;
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let lines: Vec<String> = files
        .iter()
        .map(|f| format!("{}:{}", f.path, f.sha256))
        .collect();
    let bundle_digest = sha256_hex(lines.join("\n").as_bytes());
    let manifest = Manifest {
        schema: SCHEMA.to_owned(),
        war_version: env!("CARGO_PKG_VERSION").to_owned(),
        files,
        bundle_digest: bundle_digest.clone(),
    };
    let text = serde_jcs::to_string(&manifest)
        .map_err(|e| RepoError::Message(format!("could not canonicalise MANIFEST.json: {e}")))?
        + "\n";
    let mpath = out.join("MANIFEST.json");
    std::fs::write(&mpath, text).map_err(|source| RepoError::Io {
        context: format!("could not write {mpath}"),
        source,
    })?;
    report.push(Diagnostic::pass(
        "progress.exported",
        format!(
            "{}: {} file(s) — the corpus projections and {warrants} Warrant(s) — bundle sha256:{}",
            out,
            manifest.files.len(),
            &bundle_digest[..16]
        ),
    ));
    Ok(report)
}

/// Check a bundle against its manifest: every file present and matching.
pub fn verify(dir: &Utf8Path) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let mpath = dir.join("MANIFEST.json");
    let text = std::fs::read_to_string(&mpath)
        .map_err(|e| RepoError::Message(format!("progress.no-manifest: {mpath}: {e}")))?;
    let manifest: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| RepoError::Message(format!("progress.manifest-malformed: {mpath}: {e}")))?;
    if manifest.get("schema").and_then(serde_json::Value::as_str) != Some(SCHEMA) {
        report.push(Diagnostic::error(
            "progress.manifest-schema",
            mpath.to_string(),
            format!("schema is not {SCHEMA}"),
        ));
        return Ok(report);
    }
    let mut checked = 0usize;
    for f in manifest
        .get("files")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let path = f
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let want = f
            .get("sha256")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        // A manifest is data: a path that leaves the bundle is refused, not read.
        if path.is_empty() || path.starts_with('/') || path.split('/').any(|seg| seg == "..") {
            report.push(Diagnostic::error(
                "progress.manifest-path",
                path.to_owned(),
                "a bundle path must be relative and inside the bundle".to_owned(),
            ));
            continue;
        }
        match std::fs::read(dir.join(path)) {
            Ok(bytes) if sha256_hex(&bytes) == want => checked += 1,
            Ok(_) => report.push(Diagnostic::error(
                "progress.file-drift",
                path.to_owned(),
                "bytes differ from the manifest's digest".to_owned(),
            )),
            Err(e) => report.push(Diagnostic::error(
                "progress.file-missing",
                path.to_owned(),
                e.to_string(),
            )),
        }
    }
    report.push(Diagnostic::pass(
        "progress.verified",
        format!("{checked} file(s) match the manifest"),
    ));
    Ok(report)
}

#[allow(dead_code)]
pub fn manifest_path(dir: &Utf8Path) -> Utf8PathBuf {
    dir.join("MANIFEST.json")
}
