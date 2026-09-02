// SPDX-License-Identifier: AGPL-3.0-or-later
//! §56.2 resolution: the third two-half seam.
//!
//! `war authorize` (§28.4) and `war sas accept` (§101.2) established the shape:
//! an agent may EMIT a request that names exactly what a human would be
//! signing, and only a human's RESPONSE — ingested through the authority
//! register, refused for every agent regardless of what it claims — writes a
//! record. Resolution is the act §27.2 names most plainly ("an agent SHALL NOT
//! resolve a delivery"), and until this module existed `war resolve` could only
//! report whether the thirteen were met; nothing could record that they were.
//!
//! # What ingestion refuses
//!
//! - any of §56.1's thirteen unmet, re-evaluated at ingest, never trusted from
//!   the request;
//! - a `contract_digest` that is not the one the Warrant compiles to now;
//! - a resolver the register does not know, or knows as an agent, or who does
//!   not hold `resolver`, or who is the performer (`may_resolve`);
//! - `common_outcome = satisfied` when §38.6 says the obligations are not
//!   established — a resolver may close a Warrant, and may not close it as
//!   *satisfied* over the verifiers' heads;
//! - `falsified` on a profile with no falsifiable claim (§56.3, via
//!   [`Resolution::validate`]);
//! - a Warrant that already carries a resolution. §56 has dispute and annulment
//!   for changing one; overwriting is neither.
//!
//! # What the record binds
//!
//! The contract digest, the assurance-case snapshot digest (the compiled
//! `assurance_case` section under `DigestDomain::AssuranceCaseSnapshot`), the
//! artifact manifest digest (the bytes of `deliverables.toml`), and the
//! repository-relative paths of every admissible gate run, judgment and residual
//! risk it relied on. When the contract later moves, `war check` reports the
//! resolution as stale rather than letting a record about revision N read as a
//! record about revision N+1.

use camino::Utf8Path;
use openwarrant_compiler::canonical::sha256_digest;
use openwarrant_compiler::digest::{DigestDomain, sha256_hex};
use openwarrant_core::WarUuid;
use openwarrant_core::authority::{ActorKind, ActorRole, PolicyResolutionContext};
use openwarrant_core::resolution::{CommonOutcome, Resolution};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};
use crate::resolve::{assess, declared_obligations};

pub const REQUEST_SCHEMA: &str = "oh.war/resolution-request/v1";
pub const RESPONSE_SCHEMA: &str = "oh.war/resolution-response/v1";
pub const RECORD_SCHEMA: &str = "oh.war/resolution/v1";

/// What an agent may emit: the state of the thirteen, what a signature would
/// bind, and who may sign. No recommendation, no suggested wording.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionRequest {
    pub schema: String,
    pub warrant: String,
    pub title: String,
    pub contract_digest: String,
    pub contract_revision: u32,
    /// All thirteen met right now. When false, `unmet` says which.
    pub requirements_met: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unmet: Vec<String>,
    /// §38.6, beside the thirteen: `None` when no obligation is declared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub would_resolve_satisfied: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub established: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unestablished: Vec<String>,
    /// The outcomes a signature may name, given §38.6. `satisfied` appears only
    /// when every obligation is established.
    pub permitted_outcomes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gate_run_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub judgment_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub residual_risk_refs: Vec<String>,
    /// Humans the register lets resolve this Warrant. Agents never appear.
    pub eligible_resolvers: Vec<String>,
}

/// What the resolver returns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResponse {
    pub schema: String,
    pub warrant: String,
    /// Must equal the digest the Warrant compiles to at ingest.
    pub contract_digest: String,
    pub resolved_by: String,
    /// §27.4 — the role actually exercised.
    pub acting_role: String,
    pub common_outcome: CommonOutcome,
    /// The profile's own word for it (§56.2's example: `delivered`).
    pub profile_outcome: String,
    /// §56.2 — what accepting asserts. Not optional.
    pub meaning: String,
    pub effective_time: String,
}

/// The persisted record: `docs/warrants/<alias>/resolution.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionRecord {
    pub schema: String,
    pub warrant: String,
    pub resolution: Resolution,
}

fn load_response(path: &Utf8Path) -> Result<ResolutionResponse, RepoError> {
    let text = std::fs::read_to_string(path).map_err(|source| RepoError::Io {
        context: format!("could not read {path}"),
        source,
    })?;
    toml::from_str(&text).map_err(|e| RepoError::Message(format!("{path}: {e}")))
}

/// Which outcomes §38.6 leaves open.
fn permitted_outcomes(would_satisfy: Option<bool>) -> Vec<String> {
    let mut v = vec![];
    if would_satisfy == Some(true) {
        v.push(CommonOutcome::Satisfied.to_string());
    }
    v.push(CommonOutcome::NotSatisfied.to_string());
    v.push(CommonOutcome::Cancelled.to_string());
    v.push(CommonOutcome::Blocked.to_string());
    v
}

struct Bound {
    contract_digest: String,
    contract_revision: u32,
    assurance_case_snapshot_digest: String,
    artifact_manifest_digest: String,
}

fn bind(alias: &str, one: &crate::repo::Loaded) -> Result<Bound, RepoError> {
    let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
        return Err(RepoError::Message(format!(
            "{alias}: the manifest did not validate, so there is no contract to resolve"
        )));
    };
    let ir = openwarrant_compiler::lower(basis, validated)
        .map_err(|e| RepoError::Message(format!("{alias}: could not compile contract: {e}")))?;
    let contract_digest = ir
        .contract_digest()
        .map_err(|e| RepoError::Message(format!("{alias}: could not digest contract: {e}")))?;
    // §56.2's assurance-case snapshot. The IR carries no `assurance_case`
    // section today (the compiler lowers it as `None`), so the snapshot is
    // taken over the assurance ATOMS as compiled: (ordinal, role, sha256 of
    // bytes) for each, under the snapshot domain. A record with no assurance
    // atom to bind is refused — "absent" would be a digest of nothing that
    // reads like a digest of something.
    let snapshot = match &ir.assurance_case {
        Some(a) => sha256_digest(DigestDomain::AssuranceCaseSnapshot, a)
            .map_err(|e| RepoError::Message(format!("{alias}: assurance snapshot: {e}")))?,
        None => {
            let atoms: Vec<(u32, String, String)> = basis
                .atoms
                .iter()
                .filter(|a| a.role == "assurance")
                .map(|a| (a.ordinal, a.role.clone(), sha256_hex(&a.bytes)))
                .collect();
            if atoms.is_empty() {
                return Err(RepoError::Message(format!(
                    "{alias}: no assurance atom to snapshot; §56.2 binds the assurance case and there is none"
                )));
            }
            sha256_digest(DigestDomain::AssuranceCaseSnapshot, &atoms)
                .map_err(|e| RepoError::Message(format!("{alias}: assurance snapshot: {e}")))?
        }
    };
    let snapshot = format!("sha256:{snapshot}");
    let manifest_path = one.dir.join("deliverables.toml");
    let artifact_manifest_digest = std::fs::read(&manifest_path)
        .map(|b| format!("sha256:{}", sha256_hex(&b)))
        .unwrap_or_else(|_| "absent".to_owned());
    Ok(Bound {
        contract_digest,
        contract_revision: ir.contract_revision,
        assurance_case_snapshot_digest: snapshot,
        artifact_manifest_digest,
    })
}

/// `war resolve <alias>` with no flag: the request.
pub fn request(repo: &Repository, alias: &str) -> Result<ResolutionRequest, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let bound = bind(alias, &one)?;
    let a = assess(repo, &one)?;
    let register = repo.load_authority_register()?;
    let performer = repo.performer();
    let declared = declared_obligations(&one);
    let assumptions = repo.load_rationale(&dir)?;
    let context = PolicyResolutionContext {
        policy_allows: repo.config.policy.allow_automated_resolution,
        assurance_level: &one
            .validated
            .as_ref()
            .map(|v| v.assurance_level.to_string())
            .unwrap_or_else(|| "basic".to_owned()),
        all_obligations_mechanical: declared.is_empty(),
        residual_risk_judgment_required: assumptions
            .as_deref()
            .is_none_or(|a| !crate::authorize::residual_risks_in(a).is_empty()),
    };
    let eligible_resolvers: Vec<String> = register
        .holders(ActorRole::Resolver)
        .filter(|r| r.actor_kind == ActorKind::Human)
        .filter(|r| {
            r.may_resolve(
                &performer,
                PolicyResolutionContext {
                    assurance_level: context.assurance_level,
                    ..context
                },
            )
            .is_ok()
        })
        .map(|r| r.actor.clone())
        .collect();
    let evidence = crate::evidence::load(repo, &dir)?;
    let gate_run_refs = crate::evidence::admissible_runs(&evidence, Some(&bound.contract_digest))
        .iter()
        .filter_map(|run| {
            evidence
                .iter()
                .find(|e| e.run.id == run.id && e.run.gate == run.gate)
                .map(|e| repo.relative(&e.receipt_path))
        })
        .collect();
    let judgment_refs = repo
        .load_judgments(&dir)?
        .iter()
        .map(|j| format!("judgment://{}", j.id))
        .collect();
    let residual_risk_refs = assumptions
        .as_deref()
        .map(|a| {
            crate::authorize::residual_risks_in(a)
                .iter()
                .map(|r| format!("assumption://{}", r.assumption_id))
                .collect()
        })
        .unwrap_or_default();
    let unmet: Vec<String> = a.checks.unmet().into_iter().map(str::to_owned).collect();
    Ok(ResolutionRequest {
        schema: REQUEST_SCHEMA.to_owned(),
        warrant: alias.to_owned(),
        title: one
            .validated
            .as_ref()
            .map(|v| v.raw.title.clone())
            .unwrap_or_default(),
        contract_digest: bound.contract_digest,
        contract_revision: bound.contract_revision,
        requirements_met: unmet.is_empty(),
        unmet,
        would_resolve_satisfied: a.would_resolve_satisfied,
        established: a.established,
        unestablished: a.unestablished,
        permitted_outcomes: permitted_outcomes(a.would_resolve_satisfied),
        gate_run_refs,
        judgment_refs,
        residual_risk_refs,
        eligible_resolvers,
    })
}

/// `war resolve <alias> --response <file>`: ingest a human's resolution.
pub fn ingest(repo: &Repository, alias: &str, path: &Utf8Path) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let refuse = |report: &mut Report, rule: &'static str, why: String| {
        report.push(Diagnostic::error(rule, path.to_string(), why));
    };
    let response = load_response(path)?;
    if response.schema != RESPONSE_SCHEMA {
        refuse(
            &mut report,
            "resolution.schema",
            format!(
                "{alias}: expected {RESPONSE_SCHEMA}, found {:?}",
                response.schema
            ),
        );
        return Ok(report);
    }
    if response.warrant != alias {
        refuse(
            &mut report,
            "resolution.wrong-warrant",
            format!(
                "the response names {:?}; ingesting for {alias}",
                response.warrant
            ),
        );
        return Ok(report);
    }

    let dir = repo.warrant_dir(alias)?;
    if dir.join("resolution.toml").is_file() {
        refuse(
            &mut report,
            "resolution.exists",
            format!(
                "{alias} already carries a resolution. §56.4 dispute and §56.5 annulment change one; \
             overwriting is neither"
            ),
        );
        return Ok(report);
    }
    let one = repo.load_warrant(&dir)?;
    let bound = bind(alias, &one)?;
    if response.contract_digest != bound.contract_digest {
        refuse(
            &mut report,
            "resolution.stale-digest",
            format!(
                "{alias}: the response resolves contract {} and the Warrant now compiles to {} — \
             §56.1 asks for the EXACT authorized revision",
                response.contract_digest, bound.contract_digest
            ),
        );
        return Ok(report);
    }

    let a = assess(repo, &one)?;
    let unmet = a.checks.unmet();
    if !unmet.is_empty() {
        refuse(
            &mut report,
            "resolution.requirements-unmet",
            format!(
                "{alias}: {} of 13 §56.1 requirements unmet: {}",
                unmet.len(),
                unmet.join("; ")
            ),
        );
        return Ok(report);
    }

    // The resolver, through the register. Never through the response's claims.
    let register = repo.load_authority_register()?;
    let Some(assignment) = register.actor(&response.resolved_by) else {
        refuse(
            &mut report,
            "resolution.unknown-resolver",
            format!(
                "{:?} holds no role assignment in docs/authority/roles.toml",
                response.resolved_by
            ),
        );
        return Ok(report);
    };
    if assignment.actor_kind == ActorKind::Agent {
        refuse(
            &mut report,
            "resolution.agent",
            format!(
                "{:?} is an agent. §27.2: an agent SHALL NOT resolve a delivery",
                response.resolved_by
            ),
        );
        return Ok(report);
    }
    let declared = declared_obligations(&one);
    let assumptions = repo.load_rationale(&dir)?;
    let assurance = one
        .validated
        .as_ref()
        .map(|v| v.assurance_level.to_string())
        .unwrap_or_else(|| "basic".to_owned());
    if let Err(e) = assignment.may_resolve(
        &repo.performer(),
        PolicyResolutionContext {
            policy_allows: repo.config.policy.allow_automated_resolution,
            assurance_level: &assurance,
            all_obligations_mechanical: declared.is_empty(),
            residual_risk_judgment_required: assumptions
                .as_deref()
                .is_none_or(|a| !crate::authorize::residual_risks_in(a).is_empty()),
        },
    ) {
        refuse(
            &mut report,
            "resolution.not-permitted",
            format!("{}: {e}", response.resolved_by),
        );
        return Ok(report);
    }
    if !assignment.holds(ActorRole::Resolver) {
        refuse(
            &mut report,
            "resolution.not-permitted",
            format!("{:?} does not hold the resolver role", response.resolved_by),
        );
        return Ok(report);
    }

    // §38.6 bounds the outcome a signature may name.
    if response.common_outcome == CommonOutcome::Satisfied
        && a.would_resolve_satisfied != Some(true)
    {
        refuse(
            &mut report,
            "resolution.outcome-unsupported",
            format!(
                "{alias}: `satisfied` needs every declared obligation established by an admissible \
             verification, and {} are not: {}",
                a.unestablished.len(),
                if a.unestablished.is_empty() {
                    "none declared".to_owned()
                } else {
                    a.unestablished.join(", ")
                }
            ),
        );
        return Ok(report);
    }
    if response.meaning.trim().is_empty() {
        refuse(
            &mut report,
            "resolution.no-meaning",
            format!("{alias}: §56.2 — a resolution with no stated meaning is not a resolution"),
        );
        return Ok(report);
    }

    let req = request(repo, alias)?;
    let resolution = Resolution {
        id: WarUuid::mint().to_string(),
        common_outcome: response.common_outcome,
        profile_outcome: response.profile_outcome.clone(),
        contract_revision: bound.contract_revision,
        contract_digest: bound.contract_digest.clone(),
        assurance_case_snapshot_digest: bound.assurance_case_snapshot_digest,
        artifact_manifest_digest: bound.artifact_manifest_digest,
        gate_run_refs: req.gate_run_refs,
        judgment_refs: req.judgment_refs,
        residual_risk_refs: req.residual_risk_refs,
        resolved_by_ref: format!("person://{}", response.resolved_by),
        acting_role_ref: format!(
            "role-assignment://{}/{}",
            response.resolved_by, response.acting_role
        ),
        meaning: response.meaning.clone(),
        effective_at: response.effective_time.clone(),
        recorded_at: crate::gate_cmd::receipt::now_rfc3339_public(),
        standing: openwarrant_core::ResolutionStanding::Valid,
    };
    let falsifiable = one
        .validated
        .as_ref()
        .is_some_and(|v| matches!(v.profile.as_str(), "experiment" | "feasibility"));
    if let Err(e) = resolution.validate(a.checks, falsifiable) {
        refuse(&mut report, "resolution.invalid", format!("{alias}: {e}"));
        return Ok(report);
    }

    let record = ResolutionRecord {
        schema: RECORD_SCHEMA.to_owned(),
        warrant: alias.to_owned(),
        resolution,
    };
    let out = dir.join("resolution.toml");
    let body = toml::to_string_pretty(&record)
        .map_err(|e| RepoError::Message(format!("could not render the resolution: {e}")))?;
    std::fs::write(&out, body).map_err(|source| RepoError::Io {
        context: format!("could not write {out}"),
        source,
    })?;
    if let Some(v) = &one.validated {
        crate::journal_cmd::record(
            &dir,
            &v.uuid.to_string(),
            crate::journal_cmd::RESOLUTION_RECORDED,
            &format!("person://{}", response.resolved_by),
            &format!(
                "{{\"common_outcome\":\"{}\",\"contract_digest\":\"{}\"}}",
                response.common_outcome, bound.contract_digest
            ),
        )?;
    }
    report.push(Diagnostic::pass(
        "resolution.recorded",
        format!(
            "{alias}: {} · {} by {} acting as {} → {}",
            response.common_outcome,
            response.profile_outcome,
            response.resolved_by,
            response.acting_role,
            repo.relative(&out)
        ),
    ));
    Ok(report)
}

/// `war check` rules for a recorded resolution.
pub fn check(
    repo: &Repository,
    warrant_dir: &Utf8Path,
    alias: &str,
    current_contract_digest: Option<&str>,
    report: &mut Report,
) {
    let path = warrant_dir.join("resolution.toml");
    match repo.load_resolution(warrant_dir) {
        Ok(None) => {}
        Ok(Some(r)) => {
            if r.warrant != alias {
                report.push(Diagnostic::error(
                    "resolution.wrong-warrant",
                    repo.relative(&path),
                    format!("{alias}: the record names {:?}", r.warrant),
                ));
            } else if Some(r.resolution.contract_digest.as_str()) == current_contract_digest {
                report.push(Diagnostic::pass(
                    "resolution.recorded",
                    format!(
                        "{alias}: {} · resolved by {} at {}",
                        r.resolution.common_outcome,
                        r.resolution.resolved_by_ref,
                        r.resolution.effective_at
                    ),
                ));
            } else {
                report.push(Diagnostic::error(
                    "resolution.stale",
                    repo.relative(&path),
                    format!(
                        "{alias}: the resolution binds contract {} and the Warrant now compiles to {} — \
                         a resolution of an earlier revision is not a resolution of this one. §45: \
                         dispute or annul it; do not edit around it",
                        r.resolution.contract_digest,
                        current_contract_digest.unwrap_or("nothing (the manifest does not validate)")
                    ),
                ));
            }
        }
        Err(e) => report.push(Diagnostic::error(
            "resolution.malformed",
            repo.relative(&path),
            format!("{alias}: {e} — an unreadable resolution is not an absent one"),
        )),
    }
}
