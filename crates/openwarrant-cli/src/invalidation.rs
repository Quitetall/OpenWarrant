// SPDX-License-Identifier: Apache-2.0
//! `war gate invalidate` — §45's invalidation, request and ingest
//! (OW-WAR-0136, RQ-057, §91.10 test 75).
//!
//! Invalidating a Gate Definition disputes every resolution that materially
//! rests on it, transitively. This module computes that set over the corpus —
//! the receipts each resolution relied on (`gate_run_refs`), and the resolved
//! parents each Warrant names (§20.2) — through
//! [`openwarrant_core::gate_run::propagate_invalidation`].
//!
//! # The two halves (Q-001 (a))
//!
//! Invalidating is a human act, the same two-half seam as the others:
//!
//! - **The request** (`war gate invalidate <gate>@<version> --grounds …`)
//!   names the gate, its definition's digest, the grounds, every resolution
//!   the sweep would dispute, by alias, and who may sign. It writes nothing.
//! - **The response** is signed with `war sign <gate>@<version> --grounds …
//!   --ssh-sign` by a human holding `resolver`. It names the resolutions it
//!   disputes, so the signature covers exactly that list.
//! - **The ingest** refuses, before anything is written: an unknown or moved
//!   definition, no grounds, an agent (by kind, §27.2), an actor without
//!   `resolver`, the performer (`SelfAct`, Basis A-001), a second
//!   invalidation of the same version, a sweep that no longer reaches what
//!   was signed, and a response whose signature does not verify. Then it
//!   writes `docs/gates/invalidations/<gate>@<version>.toml` and one
//!   `docs/warrants/<alias>/disputes/DSP-NNN.toml` per reached resolution
//!   (§56.4's six fields; owner = the resolver who signed).
//!
//! # What it never writes
//!
//! The Gate Definition file (§43.3), any `resolution.toml`, receipt, run or
//! attestation. Standing is read from the disputes by `war check`
//! (`resolution.disputed`), never written into the record (§45 clause 4).
//!
//! # An unsigned invalidation never counts
//!
//! Every reader — a dispute's standing, a receipt's admissibility — goes
//! through [`counted_record`], which believes a record only when the response
//! it names is the one on disk, says the same gate, digest, grounds and
//! signer, and its signature verifies as a human holding `resolver` who is
//! not the performer. A record written by hand counts for nothing.

use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::digest::sha256_hex;
use openwarrant_core::GateDefinition;
use openwarrant_core::authority::{ActorKind, ActorRole};
use openwarrant_core::gate_run::{DependentResolution, propagate_invalidation};
use openwarrant_core::resolution::Dispute;
use openwarrant_core::{GateRun, ResolutionStanding};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

pub const REQUEST_SCHEMA: &str = "oh.war/invalidation-request/v1";
pub const RESPONSE_SCHEMA: &str = "oh.war/invalidation-response/v1";
pub const RECORD_SCHEMA: &str = "oh.war/invalidation/v1";
pub const DISPUTE_SCHEMA: &str = "oh.war/dispute/v1";
/// Under the gates directory: one record per invalidated version.
pub const RECORDS_DIR: &str = "invalidations";
/// Under a Warrant: one record per dispute of its resolution.
pub const DISPUTES_DIR: &str = "disputes";
/// The journal event each dispute is witnessed by, in the disputed Warrant.
pub const DISPUTE_RECORDED: &str = "dispute.recorded";
/// The only status a dispute has here. Closing one (§45 clause 6) is a later
/// Warrant's act.
pub const OPEN: &str = "open";

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

/// Whether `s` has the shape `<gate_id>@<version>` and nothing a file name
/// could be tricked with. The record's path is built from it.
#[must_use]
pub fn is_gate_ref(s: &str) -> bool {
    let Some((id, version)) = s.split_once('@') else {
        return false;
    };
    !id.is_empty()
        && !version.is_empty()
        && !version.contains('@')
        && !s.contains("..")
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '@' | '+'))
}

/// Where the invalidation of `gate` is recorded.
#[must_use]
pub fn record_path(repo: &Repository, gate: &str) -> Utf8PathBuf {
    repo.root
        .join(&repo.config.paths.gates)
        .join(RECORDS_DIR)
        .join(format!("{gate}.toml"))
}

/// One resolution the sweep reaches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dependent {
    pub warrant: String,
    pub resolution_id: String,
    /// Who signed the resolution.
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
    /// Humans holding `resolver` who are not the performer. Agents never appear.
    pub eligible_invalidators: Vec<String>,
    /// How the act is completed.
    pub ingest: String,
}

/// What the human signs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidationResponse {
    pub schema: String,
    pub gate: String,
    /// Must equal the definition file's digest at ingest.
    pub definition_digest: String,
    pub grounds: String,
    /// The aliases the signer disputes. Must equal the sweep at ingest, so
    /// the signature covers exactly the resolutions the act disputes.
    pub disputes: Vec<String>,
    pub invalidated_by: String,
    /// §27.4 — the role actually exercised.
    pub acting_role: String,
    pub meaning: String,
    pub effective_time: String,
    /// `tty` or `ssh` via `war sign`; absent when hand-written. Provenance only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signed_via: Option<String>,
}

/// One dispute an invalidation wrote, by path and the digest it was written at.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisputeRef {
    pub warrant: String,
    pub path: String,
    pub sha256: String,
}

/// `docs/gates/invalidations/<gate>@<version>.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidationRecord {
    pub schema: String,
    pub gate: String,
    pub definition_file: String,
    pub definition_digest: String,
    pub grounds: String,
    /// `person://<actor>`.
    pub invalidated_by: String,
    pub acting_role_ref: String,
    pub effective_at: String,
    pub recorded_at: String,
    /// The signed response, repository-relative, and its digest.
    pub response: String,
    pub response_digest: String,
    #[serde(default)]
    pub unaffected: Vec<String>,
    #[serde(default)]
    pub disputes: Vec<DisputeRef>,
}

/// `docs/warrants/<alias>/disputes/DSP-NNN.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeRecord {
    pub schema: String,
    pub warrant: String,
    pub gate: String,
    /// The invalidation record that wrote it, repository-relative.
    pub invalidation: String,
    pub status: String,
    pub recorded_at: String,
    /// §56.4's six fields.
    pub dispute: Dispute,
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

/// Humans holding `resolver` who are not the performer: who may invalidate.
fn eligible(repo: &Repository) -> Result<Vec<String>, RepoError> {
    let register = repo.load_authority_register()?;
    let performer = repo.performer();
    Ok(register
        .holders(ActorRole::Resolver)
        .filter(|a| a.actor_kind == ActorKind::Human && a.actor != performer)
        .map(|a| a.actor.clone())
        .collect())
}

fn sha256_file(path: &Utf8Path) -> Result<String, RepoError> {
    let bytes = std::fs::read(path).map_err(|source| RepoError::Io {
        context: format!("could not read {path}"),
        source,
    })?;
    Ok(format!("sha256:{}", sha256_hex(&bytes)))
}

/// `war gate invalidate <gate>@<version> --grounds <text>`: the request.
pub fn request(
    repo: &Repository,
    gate: &str,
    grounds: &str,
) -> Result<(Report, Option<InvalidationRequest>), RepoError> {
    let mut report = Report::default();
    if !is_gate_ref(gate) {
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
    let digest = sha256_file(&path)?;
    let (disputes, unaffected) = sweep(repo, gate, &mut report)?;
    let eligible = eligible(repo)?;
    let recorded = record_path(repo, gate);
    if recorded.exists() {
        report.push(Diagnostic::error(
            "invalidation.exists",
            repo.relative(&recorded),
            format!(
                "{gate} is already invalidated; an invalidation is recorded once and never \
                 overwritten"
            ),
        ));
    }
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
    if eligible.is_empty() {
        report.note(
            "nobody may sign this: docs/authority/roles.toml grants `resolver` to no human who \
             is not the performer"
                .to_owned(),
        );
    }
    let request = InvalidationRequest {
        schema: REQUEST_SCHEMA.to_owned(),
        gate: gate.to_owned(),
        definition_file: repo.relative(&path),
        definition_digest: digest,
        lifecycle: def.lifecycle.to_string(),
        grounds: grounds.to_owned(),
        disputes,
        unaffected,
        eligible_invalidators: eligible,
        ingest: format!(
            "a human holding `resolver` who is not the performer signs it: `war sign {gate} \
             --grounds <text> --ssh-sign`. Until then nothing is recorded and nothing disputed"
        ),
    };
    Ok((report, Some(request)))
}

/// The pending act `war sign <gate>@<version> --grounds <text>` signs, or the
/// report saying why there is none.
pub fn pending(
    repo: &Repository,
    gate: &str,
    grounds: Option<&str>,
) -> Result<Result<crate::sign::Pending, Report>, RepoError> {
    let (report, request) = request(repo, gate, grounds.unwrap_or(""))?;
    Ok(match request {
        Some(request) if report.is_ready() => Ok(crate::sign::Pending::Invalidate {
            gate: gate.to_owned(),
            request,
        }),
        _ => Err(report),
    })
}

/// Whether a gate's invalidation counts, as every reader asks it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateInvalidation {
    /// A signed record stands. The string says so in a sentence that names it.
    Counted(String),
    /// A record exists and is not believed. The string says why.
    NotCounted(String),
}

/// A signed invalidation of `gate`, or why the record on disk does not
/// count. `None` when there is no record.
pub fn counted_record(repo: &Repository, gate: &str) -> Option<Result<InvalidationRecord, String>> {
    let path = record_path(repo, gate);
    if !path.is_file() {
        return None;
    }
    Some(judge_record(repo, gate, &path))
}

fn judge_record(
    repo: &Repository,
    gate: &str,
    path: &Utf8Path,
) -> Result<InvalidationRecord, String> {
    let rel = repo.relative(path);
    let text = std::fs::read_to_string(path).map_err(|e| format!("{rel}: {e}"))?;
    let record: InvalidationRecord =
        toml::from_str(&text).map_err(|e| format!("{rel} is not an invalidation record: {e}"))?;
    if record.schema != RECORD_SCHEMA || record.gate != gate {
        return Err(format!(
            "{rel} is {} for {}, not {RECORD_SCHEMA} for {gate}",
            record.schema, record.gate
        ));
    }
    let response_path = repo.root.join(&record.response);
    let bytes = std::fs::read(&response_path).map_err(|e| {
        format!(
            "{rel} names the response {}, and it cannot be read: {e}",
            record.response
        )
    })?;
    if format!("sha256:{}", sha256_hex(&bytes)) != record.response_digest {
        return Err(format!(
            "{rel} names the response {} at {}, and its bytes are not those",
            record.response, record.response_digest
        ));
    }
    let response: InvalidationResponse = toml::from_str(&String::from_utf8_lossy(&bytes))
        .map_err(|e| format!("{} is not an invalidation response: {e}", record.response))?;
    if response.schema != RESPONSE_SCHEMA
        || response.gate != record.gate
        || response.definition_digest != record.definition_digest
        || response.grounds != record.grounds
        || format!("person://{}", response.invalidated_by) != record.invalidated_by
    {
        return Err(format!(
            "{rel} does not say what its signed response {} says (gate, definition digest, \
             grounds and signer must agree)",
            record.response
        ));
    }
    let verdict = crate::authority_check::verify(
        repo,
        crate::authority_check::Act::Invalidate,
        gate,
        &response.invalidated_by,
        Some(&record.definition_digest),
    );
    match &verdict {
        crate::authority_check::Verdict::Signed { response: r, .. }
            if r.starts_with(&record.response) => {}
        crate::authority_check::Verdict::Signed { response: r, .. } => {
            return Err(format!(
                "the signature that verifies is over {r}, not the response {rel} names ({})",
                record.response
            ));
        }
        v => {
            return Err(format!(
                "an unsigned invalidation never counts — {}: {}",
                v.rule(),
                v.why()
            ));
        }
    }
    let register = repo.load_authority_register().map_err(|e| e.to_string())?;
    let Some(a) = register.actor(&response.invalidated_by) else {
        return Err(format!(
            "{} holds no role in docs/authority/roles.toml",
            response.invalidated_by
        ));
    };
    if a.actor_kind != ActorKind::Human || !a.holds(ActorRole::Resolver) {
        return Err(format!(
            "{} is not a human holding `resolver` in docs/authority/roles.toml",
            response.invalidated_by
        ));
    }
    if a.actor == repo.performer() {
        return Err(format!(
            "SelfAct: {} is the performer, and the performer may not invalidate a gate its work \
             rests on",
            a.actor
        ));
    }
    Ok(record)
}

/// For a receipt's admissibility: whether `gate` is invalidated, and how
/// that is said. `None` when there is no record.
#[must_use]
pub fn gate_invalidation(repo: &Repository, gate: &str) -> Option<GateInvalidation> {
    let rel = repo.relative(&record_path(repo, gate));
    counted_record(repo, gate).map(|r| match r {
        Ok(rec) => GateInvalidation::Counted(format!(
            "{gate} was invalidated by {} at {} ({rel}): {}. A receipt of an invalidated gate \
             is a record of a run that happened, not admissible evidence (§45, RQ-057)",
            rec.invalidated_by.trim_start_matches("person://"),
            rec.effective_at,
            rec.grounds
        )),
        Err(why) => GateInvalidation::NotCounted(format!(
            "{rel} does not count, so {gate} is not treated as invalidated: {why}"
        )),
    })
}

/// Every `DSP-NNN.toml` under a Warrant, parsed or with why not.
#[must_use]
pub fn load_disputes(dir: &Utf8Path) -> Vec<(Utf8PathBuf, Result<DisputeRecord, String>)> {
    let ddir = dir.join(DISPUTES_DIR);
    let Ok(entries) = ddir.read_dir_utf8() else {
        return Vec::new();
    };
    let mut paths: Vec<Utf8PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.into_path())
        .filter(|p| p.extension() == Some("toml"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let parsed = std::fs::read_to_string(&p)
                .map_err(|e| e.to_string())
                .and_then(|t| toml::from_str::<DisputeRecord>(&t).map_err(|e| e.to_string()));
            (p, parsed)
        })
        .collect()
}

/// The next `DSP-NNN` under a Warrant.
fn next_dispute_number(dir: &Utf8Path) -> u32 {
    load_disputes(dir)
        .iter()
        .filter_map(|(p, _)| {
            p.file_stem()?
                .strip_prefix("DSP-")
                .and_then(|n| n.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0)
        + 1
}

fn load_response(path: &Utf8Path) -> Result<InvalidationResponse, RepoError> {
    let text = std::fs::read_to_string(path).map_err(|source| RepoError::Io {
        context: format!("could not read {path}"),
        source,
    })?;
    toml::from_str(&text).map_err(|e| RepoError::Message(format!("{path}: {e}")))
}

/// `war gate invalidate <gate>@<version> --response <file>`: ingest a
/// human's signed invalidation. Every refusal happens before anything is
/// written.
pub fn ingest(repo: &Repository, gate: &str, path: &Utf8Path) -> Result<Report, RepoError> {
    ingest_with(repo, gate, path, crate::sign::IngestMode::Record)
}

/// The dispute §56.4 asks for, for one reached resolution.
fn dispute_for(d: &Dependent, id: &str, gate: &str, signer: &str, grounds: &str) -> Dispute {
    let parents: Vec<&str> = d
        .via
        .iter()
        .filter_map(|v| v.strip_prefix("parent "))
        .collect();
    let direct = d.via.iter().any(|v| v.starts_with("receipt "));
    Dispute {
        id: id.to_owned(),
        challenged_resolution: d.resolution_id.clone(),
        grounds: if direct {
            format!("Gate Definition {gate} was invalidated by {signer}: {grounds}")
        } else {
            format!(
                "It rests on the resolution of {} (§20.2), disputed by the invalidation of \
                 {gate} by {signer}: {grounds}",
                parents.join(", ")
            )
        },
        affected_evidence_or_judgment: d
            .via
            .iter()
            .map(|v| match v.strip_prefix("parent ") {
                Some(p) => format!("the resolution of {p}"),
                None => v.trim_start_matches("receipt ").to_owned(),
            })
            .collect(),
        reliance_policy: "Do not rely on this resolution while the dispute is open. The \
                          resolution record, its receipts and its attestations are unchanged \
                          and remain history (§45: no historical evidence is rewritten)."
            .to_owned(),
        owner: format!("person://{signer}"),
        required_re_verification: if direct {
            format!(
                "Re-run the obligations that cited {gate} under a new, qualified version of \
                 the gate, then resolve this dispute or annul the resolution (§45 clause 6)."
            )
        } else {
            format!(
                "Close the dispute of {} first, then re-verify what this Warrant took from \
                 it, and resolve this dispute or annul the resolution (§45 clause 6).",
                parents.join(", ")
            )
        },
    }
}

/// One dispute, rendered and not yet written.
struct Rendered {
    file: Utf8PathBuf,
    body: String,
    reference: DisputeRef,
    warrant_dir: Utf8PathBuf,
}

/// [`ingest`], or the same judgment with the write withheld (see
/// `sign::IngestMode`): every refusal runs; `DryRun` does not judge the
/// signature — a dry run's draft is unsigned by construction — and stops
/// with `invalidation.would-record`.
#[allow(clippy::too_many_lines)]
pub fn ingest_with(
    repo: &Repository,
    gate: &str,
    path: &Utf8Path,
    mode: crate::sign::IngestMode,
) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let at = path.to_string();
    let refuse = |report: &mut Report, rule: &'static str, why: String| {
        report.push(Diagnostic::error(rule, at.clone(), why));
    };
    let response = load_response(path)?;
    if response.schema != RESPONSE_SCHEMA {
        refuse(
            &mut report,
            "invalidation.schema",
            format!("expected {RESPONSE_SCHEMA}, found {:?}", response.schema),
        );
        return Ok(report);
    }
    if response.gate != gate || !is_gate_ref(gate) {
        refuse(
            &mut report,
            "invalidation.wrong-gate",
            format!("the response names {}; ingesting for {gate}", response.gate),
        );
        return Ok(report);
    }
    let Some((def_path, _)) = find_definition(repo, gate) else {
        refuse(
            &mut report,
            "invalidation.unknown-gate",
            format!("no Gate Definition {gate} is registered"),
        );
        return Ok(report);
    };
    let digest = sha256_file(&def_path)?;
    if response.definition_digest != digest {
        refuse(
            &mut report,
            "invalidation.stale",
            format!(
                "{gate}: the response invalidates the definition at {} and {} is now {digest} — \
                 a signature over other bytes invalidates nothing",
                response.definition_digest,
                repo.relative(&def_path)
            ),
        );
        return Ok(report);
    }
    if response.grounds.trim().is_empty() {
        refuse(
            &mut report,
            "invalidation.no-grounds",
            format!("{gate}: §56.4 — a dispute with no grounds is one nobody can answer"),
        );
        return Ok(report);
    }
    if let Err(e) = openwarrant_core::timestamp::validate_rfc3339_utc(&response.effective_time) {
        refuse(
            &mut report,
            "invalidation.effective-time",
            format!("{gate}: effective_time {e}"),
        );
        return Ok(report);
    }

    // The signer, through the register. Never through the response's claims.
    let register = repo.load_authority_register()?;
    let Some(assignment) = register.actor(&response.invalidated_by) else {
        refuse(
            &mut report,
            "invalidation.unknown-actor",
            format!(
                "{:?} holds no role assignment in docs/authority/roles.toml",
                response.invalidated_by
            ),
        );
        return Ok(report);
    };
    if assignment.actor_kind == ActorKind::Agent {
        refuse(
            &mut report,
            "invalidation.agent",
            format!(
                "{:?} is an agent. §27.2: an invalidation disputes resolutions a human signed, \
                 and an agent SHALL NOT make that judgment, whatever the response file says",
                response.invalidated_by
            ),
        );
        return Ok(report);
    }
    if !assignment.holds(ActorRole::Resolver) || response.acting_role != "resolver" {
        refuse(
            &mut report,
            "invalidation.not-permitted",
            format!(
                "{} must hold and act as `resolver` (acting as {:?}); the resolver owns standing",
                response.invalidated_by, response.acting_role
            ),
        );
        return Ok(report);
    }
    if assignment.actor == repo.performer() {
        refuse(
            &mut report,
            "invalidation.self-act",
            format!(
                "SelfAct: {} is the performer of every Warrant this invalidation reaches \
                 (Basis A-001), and no step of the exit is performed by the actor who produced \
                 the work (§27.2, §98 Phase 9)",
                assignment.actor
            ),
        );
        return Ok(report);
    }
    let record_file = record_path(repo, gate);
    if record_file.exists() {
        refuse(
            &mut report,
            "invalidation.exists",
            format!(
                "{}: {gate} is already invalidated; an invalidation is recorded once and never \
                 overwritten",
                repo.relative(&record_file)
            ),
        );
        return Ok(report);
    }
    let (reached, unaffected) = sweep(repo, gate, &mut report)?;
    let reached_aliases: BTreeSet<&str> = reached.iter().map(|d| d.warrant.as_str()).collect();
    let signed_aliases: BTreeSet<&str> = response.disputes.iter().map(String::as_str).collect();
    if reached_aliases != signed_aliases {
        refuse(
            &mut report,
            "invalidation.sweep-moved",
            format!(
                "{gate}: the response disputes [{}] and the sweep now reaches [{}]; the \
                 signature covers the list it names, and nothing is disputed that it does not",
                signed_aliases
                    .iter()
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", "),
                reached_aliases
                    .iter()
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
        return Ok(report);
    }
    if !report.is_ready() {
        // The sweep could not read a resolution: whether it is reached is
        // UNKNOWN, and an invalidation recorded over an unknown is a guess.
        return Ok(report);
    }
    if mode == crate::sign::IngestMode::Record {
        let verdict = crate::authority_check::verify(
            repo,
            crate::authority_check::Act::Invalidate,
            gate,
            &response.invalidated_by,
            Some(&response.definition_digest),
        );
        let this = repo.relative(path);
        let signed_here = matches!(
            &verdict,
            crate::authority_check::Verdict::Signed { response: r, .. } if r.starts_with(&this)
        );
        if !signed_here {
            refuse(
                &mut report,
                "invalidation.unsigned",
                format!(
                    "{gate}: an unsigned invalidation never counts, and nothing is written. {} — \
                     {}. Sign it with `war sign {gate} --grounds <text> --ssh-sign`",
                    verdict.rule(),
                    verdict.why()
                ),
            );
            return Ok(report);
        }
    }

    // Everything is judged. Render every file before writing any.
    let now = crate::gate_cmd::receipt::now_rfc3339_public();
    let signer = response.invalidated_by.clone();
    let record_rel = repo.relative(&record_file);
    let mut disputes: Vec<Rendered> = Vec::new();
    for d in &reached {
        let warrant_dir = repo.warrant_dir(&d.warrant)?;
        let id = format!("DSP-{:03}", next_dispute_number(&warrant_dir));
        let file = warrant_dir.join(DISPUTES_DIR).join(format!("{id}.toml"));
        if file.exists() {
            refuse(
                &mut report,
                "invalidation.dispute-exists",
                format!(
                    "{}: already exists; a dispute is never overwritten",
                    repo.relative(&file)
                ),
            );
            return Ok(report);
        }
        let record = DisputeRecord {
            schema: DISPUTE_SCHEMA.to_owned(),
            warrant: d.warrant.clone(),
            gate: gate.to_owned(),
            invalidation: record_rel.clone(),
            status: OPEN.to_owned(),
            recorded_at: now.clone(),
            dispute: dispute_for(d, &id, gate, &signer, &response.grounds),
        };
        if let Err(e) = record.dispute.validate() {
            refuse(
                &mut report,
                "invalidation.dispute-incomplete",
                format!("{}: {e}", d.warrant),
            );
            return Ok(report);
        }
        let body = toml::to_string_pretty(&record)
            .map_err(|e| RepoError::Message(format!("could not render {id}: {e}")))?;
        let sha256 = format!("sha256:{}", sha256_hex(body.as_bytes()));
        disputes.push(Rendered {
            reference: DisputeRef {
                warrant: d.warrant.clone(),
                path: repo.relative(&file),
                sha256,
            },
            file,
            body,
            warrant_dir,
        });
    }
    let record = InvalidationRecord {
        schema: RECORD_SCHEMA.to_owned(),
        gate: gate.to_owned(),
        definition_file: repo.relative(&def_path),
        definition_digest: digest,
        grounds: response.grounds.clone(),
        invalidated_by: format!("person://{signer}"),
        acting_role_ref: format!("role://{}", response.acting_role),
        effective_at: response.effective_time.clone(),
        recorded_at: now,
        response: repo.relative(path),
        response_digest: sha256_file(path)?,
        unaffected,
        disputes: disputes.iter().map(|r| r.reference.clone()).collect(),
    };
    let body = toml::to_string_pretty(&record)
        .map_err(|e| RepoError::Message(format!("could not render the invalidation: {e}")))?;
    let names = disputes
        .iter()
        .map(|r| r.reference.warrant.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    if mode == crate::sign::IngestMode::DryRun {
        report.push(Diagnostic::pass(
            "invalidation.would-record",
            format!(
                "{gate}: would be recorded at {record_rel}, disputing {} resolution(s){}. The \
                 signature is not judged by a dry run. Not written",
                disputes.len(),
                if names.is_empty() {
                    String::new()
                } else {
                    format!(" ({names})")
                }
            ),
        ));
        return Ok(report);
    }
    // The record first, `create_new`: it is what claims this gate version.
    write_new(&record_file, &body)?;
    for r in &disputes {
        write_new(&r.file, &r.body)?;
        if let Some(v) = repo.load_warrant(&r.warrant_dir)?.validated {
            crate::journal_cmd::record(
                &r.warrant_dir,
                &v.uuid.to_string(),
                DISPUTE_RECORDED,
                &format!("person://{signer}"),
                &serde_json::json!({
                    "gate": gate,
                    "path": r.reference.path,
                    "sha256": r.reference.sha256,
                    "invalidation": record_rel,
                })
                .to_string(),
            )?;
        }
        report.push(Diagnostic::pass(
            "dispute.recorded",
            format!(
                "{}: {} — {gate} invalidated",
                r.reference.warrant, r.reference.path
            ),
        ));
    }
    report.push(Diagnostic::pass(
        "invalidation.recorded",
        format!(
            "{gate}: invalidated by {signer} → {record_rel}; {} resolution(s) disputed. The \
             definition file and every resolution are unchanged",
            disputes.len()
        ),
    ));
    Ok(report)
}

fn write_new(path: &Utf8Path, body: &str) -> Result<(), RepoError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| RepoError::Io {
            context: format!("could not create {parent}"),
            source,
        })?;
    }
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| RepoError::Io {
            context: format!("{path} could not be created (never overwritten)"),
            source,
        })?;
    use std::io::Write as _;
    f.write_all(body.as_bytes())
        .map_err(|source| RepoError::Io {
            context: format!("could not write {path}"),
            source,
        })
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

    #[test]
    fn a_gate_ref_cannot_name_a_path() {
        assert!(is_gate_ref("ops.exit-demo@1.0.0"));
        for bad in [
            "ops.exit-demo",
            "@1.0.0",
            "ops@",
            "../x@1",
            "a/b@1",
            "a@1@2",
            "a@1 b",
        ] {
            assert!(!is_gate_ref(bad), "{bad}");
        }
    }

    #[test]
    fn a_dispute_states_all_six_things_directly_and_through_a_parent() {
        let direct = Dependent {
            warrant: "A".into(),
            resolution_id: "RES-A".into(),
            resolved_by: "person://x".into(),
            via: vec!["receipt docs/warrants/A/gate-runs/g.receipt.json".into()],
        };
        let d = dispute_for(&direct, "DSP-001", "g@1", "Signer", "unsound");
        assert_eq!(d.validate(), Ok(()));
        assert_eq!(d.owner, "person://Signer");
        assert_eq!(d.challenged_resolution, "RES-A");
        assert_eq!(
            d.affected_evidence_or_judgment,
            vec!["docs/warrants/A/gate-runs/g.receipt.json".to_owned()]
        );
        let child = Dependent {
            warrant: "C".into(),
            resolution_id: "RES-C".into(),
            resolved_by: "person://x".into(),
            via: vec!["parent A".into()],
        };
        let d = dispute_for(&child, "DSP-001", "g@1", "Signer", "unsound");
        assert_eq!(d.validate(), Ok(()));
        assert!(d.grounds.contains("resolution of A"), "{}", d.grounds);
        assert_eq!(
            d.affected_evidence_or_judgment,
            vec!["the resolution of A".to_owned()]
        );
    }

    #[test]
    fn records_render_as_toml_and_read_back() {
        let rec = InvalidationRecord {
            schema: RECORD_SCHEMA.into(),
            gate: "g@1".into(),
            definition_file: "docs/gates/g@1.yaml".into(),
            definition_digest: "sha256:00".into(),
            grounds: "unsound".into(),
            invalidated_by: "person://S".into(),
            acting_role_ref: "role://resolver".into(),
            effective_at: "2026-09-25T00:00:00Z".into(),
            recorded_at: "2026-09-25T00:00:00Z".into(),
            response: "docs/authority/responses/g@1.invalidation.response.toml".into(),
            response_digest: "sha256:11".into(),
            unaffected: vec!["D".into()],
            disputes: vec![DisputeRef {
                warrant: "A".into(),
                path: "docs/warrants/A/disputes/DSP-001.toml".into(),
                sha256: "sha256:22".into(),
            }],
        };
        let text = toml::to_string_pretty(&rec).unwrap();
        let back: InvalidationRecord = toml::from_str(&text).unwrap();
        assert_eq!(back.disputes, rec.disputes);
        let dsp = DisputeRecord {
            schema: DISPUTE_SCHEMA.into(),
            warrant: "A".into(),
            gate: "g@1".into(),
            invalidation: "docs/gates/invalidations/g@1.toml".into(),
            status: OPEN.into(),
            recorded_at: "2026-09-25T00:00:00Z".into(),
            dispute: dispute_for(
                &Dependent {
                    warrant: "A".into(),
                    resolution_id: "RES-A".into(),
                    resolved_by: "person://x".into(),
                    via: vec!["receipt r".into()],
                },
                "DSP-001",
                "g@1",
                "S",
                "unsound",
            ),
        };
        let text = toml::to_string_pretty(&dsp).unwrap();
        let back: DisputeRecord = toml::from_str(&text).unwrap();
        assert_eq!(back.dispute, dsp.dispute);
    }
}
