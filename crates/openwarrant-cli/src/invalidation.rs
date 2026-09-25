// SPDX-License-Identifier: Apache-2.0
//! `war gate invalidate` — the request half of §45's invalidation
//! (OW-WAR-0136, RQ-057, §91.10 test 75).
//!
//! Invalidating a Gate Definition disputes every resolution that materially
//! rests on it, transitively. This module computes that set over the corpus —
//! the receipts each resolution relied on (`gate_run_refs`), and the resolved
//! parents each Warrant names (§20.2) — through
//! [`openwarrant_core::gate_run::propagate_invalidation`], and emits the
//! request: the gate, its definition's digest, the grounds, and every
//! resolution the sweep would dispute, by alias.
//!
//! # What this does not do
//!
//! It writes nothing. The ingest — who may invalidate, whether that is a
//! signed act, and the invalidation and dispute records it writes — waits on
//! OW-WAR-0136 Q-001, which the owner has not answered. Until then no record
//! is written, no resolution is disputed, and `war check` reports standing
//! exactly as the resolutions say. The Gate Definition file is never edited
//! (§43.3), and no `resolution.toml` is read for anything but its fields.

use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8PathBuf;
use openwarrant_compiler::digest::sha256_hex;
use openwarrant_core::GateDefinition;
use openwarrant_core::gate_run::{DependentResolution, propagate_invalidation};
use openwarrant_core::{GateRun, ResolutionStanding};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

pub const REQUEST_SCHEMA: &str = "oh.war/invalidation-request/v1";

/// The Gate Definition file for `<gate_id>@<version>`, and the definition.
/// A file that does not parse is skipped here; `war check` reports it.
pub fn find_definition(repo: &Repository, key: &str) -> Option<(Utf8PathBuf, GateDefinition)> {
    let dir = repo.root.join(&repo.config.paths.gates);
    let mut paths: Vec<Utf8PathBuf> = dir
        .read_dir_utf8()
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.into_path())
        .filter(|p| p.extension().is_some_and(|e| e == "yaml" || e == "yml"))
        .collect();
    paths.sort();
    paths.into_iter().find_map(|path| {
        let text = std::fs::read_to_string(&path).ok()?;
        let doc = openwarrant_core::structured::parse(&text).ok()?;
        let def = openwarrant_core::gate::definition_from_structured(&doc).ok()?;
        (def.key() == key).then_some((path, def))
    })
}

/// One resolution the sweep reaches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dependent {
    pub warrant: String,
    pub resolution_id: String,
    /// Who signed the resolution: §56.4's owner of the dispute.
    pub resolved_by: String,
    /// Why it is reached: `receipt <path>` for a run of the gate it relied
    /// on, `parent <alias>` for a disputed resolution it rests on.
    pub via: Vec<String>,
}

/// What a signature would invalidate, and what that would dispute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidationRequest {
    pub schema: String,
    pub gate: String,
    pub definition_file: String,
    /// sha256 of the definition file's bytes as they are now. §43.3: the
    /// file is not edited by an invalidation, so this digest outlives it.
    pub definition_digest: String,
    pub lifecycle: String,
    pub grounds: String,
    /// Every resolution the sweep would dispute, transitively, by alias.
    pub disputes: Vec<Dependent>,
    /// Resolutions the sweep leaves standing.
    pub unaffected: Vec<String>,
    /// What happens to this request: nothing, until Q-001 is answered.
    pub ingest: String,
}

struct Resolved {
    alias: String,
    id: String,
    resolved_by: String,
    /// (receipt path, `<gate>@<version>` of its run).
    runs: Vec<(String, String)>,
    parent_uuids: Vec<String>,
}

/// Every resolution in the corpus, with what it rests on.
fn corpus(
    repo: &Repository,
    report: &mut Report,
) -> Result<(Vec<Resolved>, BTreeMap<String, String>), RepoError> {
    let mut resolved = Vec::new();
    let mut alias_of_uuid = BTreeMap::new();
    for dir in repo.warrant_dirs()? {
        let Some(alias) = dir.file_name().map(str::to_owned) else {
            continue;
        };
        let manifest: Option<toml::Value> = std::fs::read_to_string(dir.join("manifest.toml"))
            .ok()
            .and_then(|t| toml::from_str(&t).ok());
        let uuid = manifest
            .as_ref()
            .and_then(|m| m.get("uuid"))
            .and_then(toml::Value::as_str)
            .map(str::to_owned);
        if let Some(u) = &uuid {
            alias_of_uuid.insert(u.clone(), alias.clone());
        }
        let parent_uuids: Vec<String> = manifest
            .as_ref()
            .and_then(|m| m.get("parents"))
            .and_then(toml::Value::as_array)
            .map(|ps| {
                ps.iter()
                    .filter_map(|p| p.get("ref").and_then(toml::Value::as_str))
                    .filter_map(|r| r.strip_prefix("war://"))
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let record = match repo.load_resolution(&dir) {
            Ok(Some(r)) => r,
            Ok(None) => continue,
            Err(e) => {
                // An unreadable resolution cannot be said to be unaffected.
                report.push(Diagnostic::unknown(
                    "invalidation.resolution-unreadable",
                    repo.relative(&dir.join("resolution.toml")),
                    format!("{alias}: {e}; whether the sweep reaches it is UNKNOWN"),
                ));
                continue;
            }
        };
        let runs = record
            .resolution
            .gate_run_refs
            .iter()
            .map(|receipt| {
                let gate = receipt
                    .strip_suffix(".receipt.json")
                    .map(|stem| repo.root.join(format!("{stem}.run.toml")))
                    .and_then(|p| std::fs::read_to_string(p).ok())
                    .and_then(|t| toml::from_str::<GateRun>(&t).ok())
                    .map(|r| r.gate)
                    .unwrap_or_default();
                (receipt.clone(), gate)
            })
            .collect::<Vec<_>>();
        for (receipt, gate) in &runs {
            if gate.is_empty() {
                report.push(Diagnostic::unknown(
                    "invalidation.run-unreadable",
                    receipt.clone(),
                    format!(
                        "{alias}: no run beside {receipt} names its gate; whether the sweep \
                         reaches it through this receipt is UNKNOWN"
                    ),
                ));
            }
        }
        resolved.push(Resolved {
            alias,
            id: record.resolution.id.clone(),
            resolved_by: record.resolution.resolved_by_ref.clone(),
            runs,
            parent_uuids,
        });
    }
    Ok((resolved, alias_of_uuid))
}

/// §45's sweep for `gate`: every resolution it would dispute, with why.
pub fn sweep(
    repo: &Repository,
    gate: &str,
    report: &mut Report,
) -> Result<(Vec<Dependent>, Vec<String>), RepoError> {
    let (resolved, alias_of_uuid) = corpus(repo, report)?;
    let resolved_aliases: BTreeSet<&str> = resolved.iter().map(|r| r.alias.as_str()).collect();
    // Run ids are qualified by the receipt path: two Warrants' runs of one
    // gate share an id (`GR-<gate>`), and the sweep must tell them apart.
    let mut runs = Vec::new();
    let mut deps = Vec::new();
    for r in &resolved {
        for (receipt, g) in &r.runs {
            runs.push(GateRun {
                id: receipt.clone(),
                gate: g.clone(),
                askability: openwarrant_core::Askability::Askable,
                execution_status: openwarrant_core::ExecutionStatus::Completed,
                verdict: openwarrant_core::Verdict::Pass,
                reason_code: None,
            });
        }
        deps.push(DependentResolution {
            id: r.alias.clone(),
            rests_on_runs: r.runs.iter().map(|(p, _)| p.clone()).collect(),
            rests_on_resolutions: r
                .parent_uuids
                .iter()
                .filter_map(|u| alias_of_uuid.get(u))
                .filter(|a| resolved_aliases.contains(a.as_str()))
                .cloned()
                .collect(),
            standing: ResolutionStanding::Valid,
        });
    }
    let disputed = propagate_invalidation(gate, &runs, &deps);
    let mut out = Vec::new();
    let mut unaffected = Vec::new();
    for r in &resolved {
        if !disputed.contains(&r.alias) {
            unaffected.push(r.alias.clone());
            continue;
        }
        let mut via: Vec<String> = r
            .runs
            .iter()
            .filter(|(_, g)| g == gate)
            .map(|(p, _)| format!("receipt {p}"))
            .collect();
        via.extend(
            r.parent_uuids
                .iter()
                .filter_map(|u| alias_of_uuid.get(u))
                .filter(|a| disputed.contains(*a))
                .map(|a| format!("parent {a}")),
        );
        out.push(Dependent {
            warrant: r.alias.clone(),
            resolution_id: r.id.clone(),
            resolved_by: r.resolved_by.clone(),
            via,
        });
    }
    Ok((out, unaffected))
}

/// `war gate invalidate <gate>@<version> --grounds <text>`: the request.
pub fn request(
    repo: &Repository,
    gate: &str,
    grounds: &str,
) -> Result<(Report, Option<InvalidationRequest>), RepoError> {
    let mut report = Report::default();
    if !gate.contains('@') || gate.starts_with('@') || gate.ends_with('@') {
        report.push(Diagnostic::error(
            "invalidation.gate-ref",
            gate.to_owned(),
            format!(
                "{gate:?} is not `<gate_id>@<version>`: a Gate Definition version is what is \
                 invalidated, never a gate across its versions"
            ),
        ));
        return Ok((report, None));
    }
    if grounds.trim().is_empty() {
        report.push(Diagnostic::error(
            "invalidation.no-grounds",
            gate.to_owned(),
            "§56.4: a dispute states its grounds, and an invalidation that disputes \
             resolutions states them first"
                .to_owned(),
        ));
        return Ok((report, None));
    }
    let Some((path, def)) = find_definition(repo, gate) else {
        report.push(Diagnostic::error(
            "invalidation.unknown-gate",
            gate.to_owned(),
            format!(
                "no Gate Definition {gate} under {}; only a registered definition can be \
                 invalidated",
                repo.config.paths.gates
            ),
        ));
        return Ok((report, None));
    };
    let bytes = std::fs::read(&path).map_err(|source| RepoError::Io {
        context: format!("could not read {path}"),
        source,
    })?;
    let (disputes, unaffected) = sweep(repo, gate, &mut report)?;
    report.push(Diagnostic::pass(
        "invalidation.requested",
        format!(
            "{gate}: the sweep would dispute {} resolution(s){} and leave {} standing. \
             Nothing is written",
            disputes.len(),
            if disputes.is_empty() {
                String::new()
            } else {
                format!(
                    " ({})",
                    disputes
                        .iter()
                        .map(|d| d.warrant.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            },
            unaffected.len()
        ),
    ));
    report.note(
        "the ingest is not implemented: who may invalidate a gate, and whether it is a signed \
         act, is OW-WAR-0136 Q-001, unanswered. No invalidation or dispute record is written"
            .to_owned(),
    );
    let request = InvalidationRequest {
        schema: REQUEST_SCHEMA.to_owned(),
        gate: gate.to_owned(),
        definition_file: repo.relative(&path),
        definition_digest: format!("sha256:{}", sha256_hex(&bytes)),
        lifecycle: def.lifecycle.to_string(),
        grounds: grounds.to_owned(),
        disputes,
        unaffected,
        ingest: "none: OW-WAR-0136 Q-001 (who may invalidate, and whether it is signed) is \
                 unanswered; this request records nothing and disputes nothing"
            .to_owned(),
    };
    Ok((report, Some(request)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gate_ref_without_a_version_or_grounds_is_refused_before_anything_is_read() {
        let repo_root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize_utf8()
            .unwrap();
        let repo = Repository::open(repo_root).expect("repository opens");
        let (r, req) = request(&repo, "software.repo.war-check", "g").unwrap();
        assert!(req.is_none());
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.rule == "invalidation.gate-ref")
        );
        let (r, req) = request(&repo, "software.repo.war-check@1.0.0", "  ").unwrap();
        assert!(req.is_none());
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.rule == "invalidation.no-grounds")
        );
        let (r, req) = request(&repo, "no.such.gate@9.9.9", "g").unwrap();
        assert!(req.is_none());
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.rule == "invalidation.unknown-gate")
        );
    }
}
