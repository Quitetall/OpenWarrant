// SPDX-License-Identifier: Apache-2.0
//! `war verify <alias> --bundle` / `--run` — the verification bundle and a
//! configured verifier (slice C3; SAS §46, §75.2).
//!
//! The request `war verify` emits names obligations; a blind verifier then
//! has to go and find the atoms, the deliverables, the receipts and the
//! plants by itself, in a context that is not the performer's. Sessions did
//! that with a script. The bundle is that script inside the tool: one
//! canonical JSON document (`oh.war/verification-bundle/v1`) carrying the
//! request with the authorized contract digest filled in, every basis atom,
//! each deliverable's bytes (whole under a size cap, else the head with the
//! full digest and `truncated: true`), the plants that name the alias, the
//! `#[test]` names in Rust deliverables, the committed gate runs and the
//! prior verifications — and its own token estimate. Written to
//! `verifications/bundle-<digest>.json` under a digest domain of its own.
//!
//! `--run` hands the bundle path to `[verify] verifier_argv`, under a
//! deadline, and ingests what it prints on stdout through the unchanged
//! `verify::ingest` — so every refusal that seam has (self-verification,
//! wrong Warrant, malformed envelope) applies to a configured verifier too.

use camino::Utf8PathBuf;
use openwarrant_compiler::digest::sha256_hex;
use openwarrant_compiler::{DigestDomain, sha256_digest};
use serde::Serialize;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

pub const SCHEMA: &str = "oh.war/verification-bundle/v1";

#[derive(Debug, Clone, Serialize)]
pub struct BundledAtom {
    pub path: String,
    pub role: String,
    pub sha256: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BundledDeliverable {
    pub id: String,
    pub title: String,
    pub target_ref: String,
    pub sha256: String,
    pub bytes: u64,
    pub truncated: bool,
    pub text: String,
    /// `#[test]` function names, for a Rust source deliverable.
    pub test_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BundledPlant {
    pub file: String,
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Bundle {
    pub schema: String,
    pub warrant: String,
    pub authorized_contract_digest: String,
    pub request: crate::verify::VerificationRequest,
    pub atoms: Vec<BundledAtom>,
    pub deliverables: Vec<BundledDeliverable>,
    pub plants: Vec<BundledPlant>,
    pub gate_runs: Vec<serde_json::Value>,
    pub prior_verifications: Vec<openwarrant_core::verification::Verification>,
    pub estimated_tokens: u64,
    pub token_method: String,
}

/// `#[test]` function names, by a line scan that needs no parser.
fn test_names(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut armed = false;
    for line in source.lines() {
        let t = line.trim();
        if t == "#[test]" {
            armed = true;
            continue;
        }
        if armed && let Some(rest) = t.strip_prefix("fn ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                out.push(name);
            }
            armed = false;
        } else if armed && !t.starts_with("//") && !t.starts_with('#') {
            armed = false;
        }
    }
    out
}

/// The plant lines naming `alias`, across the battery.
fn plants_naming(repo: &Repository, alias: &str) -> Vec<BundledPlant> {
    let mut out = Vec::new();
    let dir = repo.root.join("conformance/plants.d");
    let mut files: Vec<Utf8PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
                .filter(|p| p.extension() == Some("sh"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            if line.contains(alias) && (line.contains("plant") || line.contains("printf 'ok")) {
                out.push(BundledPlant {
                    file: repo.relative(&f),
                    line: i + 1,
                    text: line.trim().to_owned(),
                });
            }
        }
    }
    out
}

pub fn build(repo: &Repository, alias: &str, performer: &str) -> Result<Bundle, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let request = crate::verify::request(repo, alias, performer)?;
    let authorized_contract_digest = repo
        .load_authorization(&dir)
        .ok()
        .flatten()
        .map(|a| a.revision.contract_digest)
        .unwrap_or_default();
    let mut atoms = Vec::new();
    if let Some(basis) = &one.basis {
        for a in &basis.atoms {
            atoms.push(BundledAtom {
                path: a.source.clone(),
                role: a.role.clone(),
                sha256: sha256_hex(&a.bytes),
                text: String::from_utf8_lossy(&a.bytes).into_owned(),
            });
        }
    }
    let cap = repo.config.verify.max_excerpt_bytes();
    let mut deliverables = Vec::new();
    if let Ok(set) = repo.load_deliverables(&dir) {
        for d in set.records {
            let full = repo.root.join(&d.target_ref);
            let (sha, len, text, truncated) = match std::fs::read(&full) {
                Ok(bytes) => {
                    let sha = sha256_hex(&bytes);
                    let len = bytes.len() as u64;
                    let truncated = bytes.len() > cap;
                    let head = if truncated { &bytes[..cap] } else { &bytes[..] };
                    (
                        sha,
                        len,
                        String::from_utf8_lossy(head).into_owned(),
                        truncated,
                    )
                }
                Err(e) => (String::new(), 0, format!("<unreadable: {e}>"), false),
            };
            let names = if d.target_ref.ends_with(".rs") {
                std::fs::read_to_string(&full)
                    .map(|s| test_names(&s))
                    .unwrap_or_default()
            } else {
                vec![]
            };
            deliverables.push(BundledDeliverable {
                id: d.id,
                title: d.title,
                target_ref: d.target_ref,
                sha256: sha,
                bytes: len,
                truncated,
                text,
                test_names: names,
            });
        }
    }
    let gate_runs: Vec<serde_json::Value> = crate::evidence::load(repo, &dir)
        .map(|ev| {
            ev.iter()
                .map(|e| {
                    serde_json::json!({
                        "gate": e.run.gate,
                        "run_id": e.run.id,
                        "run": serde_json::to_value(&e.run).unwrap_or_default(),
                        "receipt": e.receipt.as_ref().map(|r| serde_json::to_value(r).unwrap_or_default()),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let prior_verifications = repo
        .load_verifications(&dir)
        .map(|v| v.records)
        .unwrap_or_default();
    let plants = plants_naming(repo, alias);
    let mut bundle = Bundle {
        schema: SCHEMA.to_owned(),
        warrant: alias.to_owned(),
        authorized_contract_digest,
        request,
        atoms,
        deliverables,
        plants,
        gate_runs,
        prior_verifications,
        estimated_tokens: 0,
        token_method: openwarrant_core::tokens::METHOD.to_owned(),
    };
    // The estimate counts what a verifier READS — atom text, deliverable
    // bytes, plant lines — not the request, the gate-run records or the JSON
    // punctuation around them; it is a reading cost, labelled as such.
    let bytes: u64 = bundle
        .atoms
        .iter()
        .map(|a| a.text.len() as u64)
        .sum::<u64>()
        + bundle
            .deliverables
            .iter()
            .map(|d| d.text.len() as u64)
            .sum::<u64>()
        + bundle
            .plants
            .iter()
            .map(|p| p.text.len() as u64)
            .sum::<u64>();
    bundle.estimated_tokens = openwarrant_core::tokens::estimate(bytes);
    Ok(bundle)
}

/// Canonical bytes and the domain-prefixed digest.
pub fn canonical(bundle: &Bundle) -> Result<(String, String), RepoError> {
    let text = serde_jcs::to_string(bundle)
        .map_err(|e| RepoError::Message(format!("could not canonicalise the bundle: {e}")))?;
    let digest = sha256_digest(DigestDomain::VerificationBundle, bundle)
        .map_err(|e| RepoError::Message(e.to_string()))?;
    Ok((text, digest))
}

/// Build and write `verifications/bundle-<digest16>.json`; returns its path.
pub fn write(
    repo: &Repository,
    alias: &str,
    performer: &str,
) -> Result<(Utf8PathBuf, Bundle, String), RepoError> {
    let bundle = build(repo, alias, performer)?;
    let (text, digest) = canonical(&bundle)?;
    let dir = repo.warrant_dir(alias)?.join("verifications");
    std::fs::create_dir_all(&dir).map_err(|source| RepoError::Io {
        context: format!("could not create {dir}"),
        source,
    })?;
    let short = digest
        .trim_start_matches("sha256:")
        .chars()
        .take(16)
        .collect::<String>();
    let path = dir.join(format!("bundle-{short}.json"));
    std::fs::write(&path, text + "\n").map_err(|source| RepoError::Io {
        context: format!("could not write {path}"),
        source,
    })?;
    Ok((path, bundle, digest))
}

/// `war verify <alias> --bundle`: write it and say what it holds.
pub fn emit(repo: &Repository, alias: &str, performer: &str) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let (path, bundle, digest) = write(repo, alias, performer)?;
    report.push(Diagnostic::pass(
        "verify.bundle",
        format!(
            "{alias}: bundle {digest} written to {} — {} obligation(s), {} atom(s), {} deliverable(s) ({} truncated), {} plant line(s), {} gate run(s), {} prior verification(s), ~{} tokens ({})",
            repo.relative(&path),
            bundle.request.obligations.len(),
            bundle.atoms.len(),
            bundle.deliverables.len(),
            bundle.deliverables.iter().filter(|d| d.truncated).count(),
            bundle.plants.len(),
            bundle.gate_runs.len(),
            bundle.prior_verifications.len(),
            bundle.estimated_tokens,
            bundle.token_method
        ),
    ));
    Ok(report)
}

/// `war verify <alias> --run`: the configured verifier, then the unchanged ingest.
pub fn run(repo: &Repository, alias: &str, performer: &str) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let argv = &repo.config.verify.verifier_argv;
    if argv.is_empty() {
        report.push(Diagnostic::error(
            "verify.no-verifier",
            "openwarrant.toml".to_owned(),
            "no verifier is configured: set `[verify] verifier_argv` to a command that reads a \
             bundle path and prints an oh.war/verification-response/v1 document on stdout. A seam \
             with nothing on the other side says so (§75.2)"
                .to_owned(),
        ));
        return Ok(report);
    }
    let (bundle_path, bundle, digest) = write(repo, alias, performer)?;
    report.push(Diagnostic::pass(
        "verify.bundle",
        format!(
            "{alias}: bundle {digest} ({} tokens) written to {}",
            bundle.estimated_tokens,
            repo.relative(&bundle_path)
        ),
    ));
    let timeout = repo.config.verify.timeout_secs();
    let mut child = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .arg(bundle_path.as_str())
        .current_dir(&repo.root)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            RepoError::Message(format!(
                "verify.verifier-failed: could not run {:?}: {e}",
                argv[0]
            ))
        })?;
    let started = std::time::Instant::now();
    let deadline = std::time::Duration::from_secs(timeout);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if started.elapsed() > deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
            Err(e) => return Err(RepoError::Message(format!("verify.verifier-failed: {e}"))),
        }
    };
    let Some(status) = status else {
        report.push(Diagnostic::error(
            "verify.verifier-timeout",
            argv.join(" "),
            format!("the verifier was killed after {timeout}s; nothing was ingested"),
        ));
        return Ok(report);
    };
    let mut stdout = String::new();
    let mut stderr = String::new();
    use std::io::Read;
    if let Some(mut o) = child.stdout.take() {
        let _ = o.read_to_string(&mut stdout);
    }
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_string(&mut stderr);
    }
    if !status.success() {
        report.push(Diagnostic::error(
            "verify.verifier-failed",
            argv.join(" "),
            format!(
                "exit {status}: {}",
                stderr.trim().chars().take(400).collect::<String>()
            ),
        ));
        return Ok(report);
    }
    let response_path = repo.warrant_dir(alias)?.join("verifications").join(format!(
        "response-{}.toml",
        digest
            .trim_start_matches("sha256:")
            .chars()
            .take(16)
            .collect::<String>()
    ));
    std::fs::write(&response_path, &stdout).map_err(|source| RepoError::Io {
        context: format!("could not write {response_path}"),
        source,
    })?;
    let ingested = crate::verify::ingest(repo, alias, &response_path)?;
    // The response file was the seam's input; the verifications it produced
    // are the records. Keep it beside them only when the ingest accepted it.
    if !ingested.is_ready() {
        let _ = std::fs::remove_file(&response_path);
    }
    for d in ingested.diagnostics {
        report.push(d);
    }
    for n in ingested.notes {
        report.note(n);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_names_are_read_from_test_attributes_only() {
        let src = "fn helper() {}\n#[test]\nfn one_works() {}\n#[test]\n#[ignore]\nfn two_is_ignored() {}\nfn three() {}\n";
        assert_eq!(test_names(src), vec!["one_works", "two_is_ignored"]);
    }
}
