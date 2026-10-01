// SPDX-License-Identifier: Apache-2.0
//! `war verify` — the independent-verification seam (SAS §46, §38.5, §75.2).
//!
//! # Two halves, and why this command cannot verify anything itself
//!
//! `war verify <alias>` EMITS a verification request. `war verify <alias>
//! --response <file>` INGESTS the verdicts something else returned. Nothing in
//! between runs a model, and that is the design rather than a gap.
//!
//! The reason is §46 and it is not negotiable by convenience: the actor that
//! produced the work cannot be the actor that clears it. If this command called
//! a model itself, the resulting verdict would still have been produced inside
//! the performer's process, holding the performer's context. Emitting a request
//! and consuming a response keeps the verifier genuinely out-of-process, which
//! is what makes `separate_writable_workspace` and the two blindness dimensions
//! true statements rather than assertions.
//!
//! # What the request may contain
//!
//! Exactly §46.2's admissible inputs, carried by
//! [`BlindVerifierInput`] — a type with no field for the performer's narrative,
//! so the exclusion is structural rather than a rule someone must remember.
//!
//! # What ingestion refuses
//!
//! Every response is checked by [`Verification::admissible_for`] before it is
//! written. A returned verdict that self-verifies, cites no evidence, or claims
//! independence it did not have is refused and NOT recorded — a rejected verdict
//! must not become a file that later reads as a verification.

use std::collections::BTreeMap;
use std::fs;

use camino::Utf8PathBuf;
use openwarrant_core::independence::BlindVerifierInput;
use openwarrant_core::obligation;
use openwarrant_core::verification::Verification;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

/// The canonical request handed to an independent verifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationRequest {
    pub schema: String,
    pub warrant: String,
    pub assurance_level: String,
    /// The actor whose work is under verification. Echoed back in the response
    /// so [`Verification::admissible_for`] can detect self-verification instead
    /// of trusting the responder to declare it.
    pub performer: String,
    pub obligations: Vec<RequestedObligation>,
    pub inputs: BlindVerifierInput,
    /// Exact subject facts to echo in the response; this is not an approval.
    pub reviewed_subject: ReviewedSubject,
}

/// A review binds the compiled contract and actual delivered file bytes.
/// Digests use existing contract canonicalization and ordinary file SHA-256;
/// no new semantic digest domain or replacement canonicalizer is introduced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedSubject {
    pub contract_digest: String,
    pub artifacts: BTreeMap<String, String>,
}

pub fn subject(repo: &Repository, one: &crate::repo::Loaded) -> Result<ReviewedSubject, RepoError> {
    let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
        return Err(RepoError::Message(
            "verify.subject-unavailable: the Warrant does not compile".to_owned(),
        ));
    };
    let contract_digest = openwarrant_compiler::lower(basis, validated)
        .map_err(|e| RepoError::Message(format!("verify.subject-unavailable: {e}")))?
        .contract_digest()
        .map_err(|e| RepoError::Message(format!("verify.subject-unavailable: {e}")))?;
    let mut artifacts = BTreeMap::new();
    let deliveries = repo.load_deliverables(&one.dir)?;
    if !deliveries.failures.is_empty() {
        return Err(RepoError::Message(format!(
            "verify.subject-unavailable: deliverable declarations did not parse: {:?}",
            deliveries.failures
        )));
    }
    for delivery in deliveries.records {
        let relative = camino::Utf8Path::new(&delivery.target_ref);
        if relative.is_absolute()
            || relative
                .components()
                .any(|c| matches!(c, camino::Utf8Component::ParentDir))
        {
            return Err(RepoError::Message(format!(
                "verify.subject-unavailable: artifact reference {:?} is not repository-relative",
                delivery.target_ref
            )));
        }
        let path = repo.root.join(relative);
        let mut walked = repo.root.clone();
        for component in relative.components() {
            walked.push(component.as_str());
            match fs::symlink_metadata(&walked) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(RepoError::Message(format!(
                        "verify.subject-unavailable: symlink at {walked}"
                    )));
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
                Err(source) => {
                    return Err(RepoError::Io {
                        context: format!("could not inspect {walked}"),
                        source,
                    });
                }
                _ => {}
            }
        }
        let digest = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => {
                let bytes = fs::read(&path).map_err(|source| RepoError::Io {
                    context: format!("could not read {path}"),
                    source,
                })?;
                format!(
                    "sha256:{}",
                    openwarrant_compiler::digest::sha256_hex(&bytes)
                )
            }
            Ok(_) => {
                return Err(RepoError::Message(format!(
                    "verify.subject-unavailable: {path} is not a regular file"
                )));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => "missing".to_owned(),
            Err(source) => {
                return Err(RepoError::Io {
                    context: format!("could not inspect {path}"),
                    source,
                });
            }
        };
        artifacts.insert(delivery.target_ref, digest);
    }
    Ok(ReviewedSubject {
        contract_digest,
        artifacts,
    })
}

/// Historical verdicts remain readable. Only a matching observed review may
/// qualify the current subject. Never infer review from a file's timestamp.
pub fn current_records(
    repo: &Repository,
    one: &crate::repo::Loaded,
    records: &[Verification],
) -> Vec<Verification> {
    let Ok(journal) = crate::journal_cmd::load(&one.dir) else {
        return vec![];
    };
    let bound: Vec<_> = journal
        .events
        .iter()
        .filter_map(|event| {
            if event.event_type != crate::journal_cmd::VERIFICATION_RECORDED {
                return None;
            }
            let payload: serde_json::Value = serde_json::from_str(&event.payload).ok()?;
            let reviewed: ReviewedSubject =
                serde_json::from_value(payload.get("reviewed_subject")?.clone()).ok()?;
            Some((event, payload, reviewed))
        })
        .collect();
    if bound.is_empty() {
        return vec![];
    }
    let Ok(current) = subject(repo, one) else {
        return vec![];
    };
    records
        .iter()
        .filter(|record| {
            let path = one
                .dir
                .join("verifications")
                .join(format!("{}.toml", record.obligation));
            let Ok(bytes) = fs::read(path) else {
                return false;
            };
            let digest = format!(
                "sha256:{}",
                openwarrant_compiler::digest::sha256_hex(&bytes)
            );
            let actor = format!("{}://{}", record.verifier.kind, record.verifier.actor);
            bound.iter().any(|(event, payload, reviewed)| {
                event.actor_ref == actor
                    && one
                        .validated
                        .as_ref()
                        .is_some_and(|v| event.warrant_uuid == v.uuid.to_string())
                    && reviewed == &current
                    && payload["obligation"] == record.obligation
                    && payload["record_digest"] == digest
            })
        })
        .cloned()
        .collect()
}

/// One obligation put to the verifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestedObligation {
    pub id: String,
    pub statement: String,
    /// §38.4's bound. A verifier that cannot see the scope cannot tell an
    /// overclaim from a claim.
    pub scope: String,
    pub evidence: String,
}

pub const REQUEST_SCHEMA: &str = "oh.war/verification-request/v1";

/// What a verifier returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationResponse {
    pub schema: String,
    pub warrant: String,
    pub verifications: Vec<Verification>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_subject: Option<ReviewedSubject>,
}

pub const RESPONSE_SCHEMA: &str = "oh.war/verification-response/v1";

/// Build the request for one Warrant.
pub fn request(
    repo: &Repository,
    alias: &str,
    performer: &str,
) -> Result<VerificationRequest, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;

    let assurance = one
        .validated
        .as_ref()
        .map(|v| v.assurance_level.to_string())
        .unwrap_or_else(|| "basic".to_owned());

    let mut obligations = Vec::new();
    if let Some(basis) = one.basis.as_ref() {
        for atom in basis.atoms.iter().filter(|a| a.role == "assurance") {
            if let Ok(set) = obligation::parse(&String::from_utf8_lossy(&atom.bytes)) {
                for o in set.obligations {
                    obligations.push(RequestedObligation {
                        id: o.id,
                        statement: o.statement,
                        scope: o.scope,
                        evidence: o.evidence,
                    });
                }
            }
        }
    }

    // Artifact references are repository paths, deliberately not contents: a
    // verifier reads the tree itself, so nothing here can be a curated excerpt
    // chosen by the performer.
    //
    // Two sources, and the second one matters more. The Warrant's own atoms say
    // what was PROMISED. The declared deliverables (§37) say what was
    // PRODUCED — and an obligation is a claim about the latter. Sending only the
    // atoms asks the verifier whether `independence.rs` implements §46.1 without
    // showing it `independence.rs`, which is not a question anyone can answer
    // honestly. That was the shape of the request until deliverables existed to
    // name the artifacts.
    let mut artifact_refs: Vec<String> = one
        .basis
        .as_ref()
        .map(|b| b.atoms.iter().map(|a| a.source.clone()).collect())
        .unwrap_or_default();
    for deliverable in repo.load_deliverables(&dir)?.records {
        if !artifact_refs.contains(&deliverable.target_ref) {
            artifact_refs.push(deliverable.target_ref);
        }
    }

    Ok(VerificationRequest {
        schema: REQUEST_SCHEMA.to_owned(),
        warrant: alias.to_owned(),
        assurance_level: assurance,
        performer: performer.to_owned(),
        obligations,
        inputs: BlindVerifierInput {
            authorized_contract_digest: String::new(),
            artifact_refs,
            gate_binding_refs: vec![],
            evidence_refs: vec![],
            required_context_refs: vec![],
        },
        reviewed_subject: subject(repo, &one)?,
    })
}

/// Why a response envelope was refused before any verdict was considered.
#[derive(Debug, PartialEq, Eq)]
pub enum EnvelopeRefusal {
    UnknownSchema { found: String },
    WrongWarrant { named: String, ingesting: String },
}

impl std::fmt::Display for EnvelopeRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSchema { found } => write!(
                f,
                "unknown response schema {found:?}; expected {RESPONSE_SCHEMA:?}"
            ),
            Self::WrongWarrant { named, ingesting } => write!(
                f,
                "response names {named:?} but is being ingested for {ingesting}"
            ),
        }
    }
}

/// Check the envelope before any verdict inside it is looked at.
///
/// Separate from [`ingest`] so both refusals are testable without a repository
/// on disk. The warrant check is the load-bearing one: a response for a
/// different Warrant, written into this one's directory, would attach verdicts
/// to work the verifier never examined.
pub fn validate_envelope(
    response: &VerificationResponse,
    ingesting: &str,
) -> Result<(), EnvelopeRefusal> {
    if response.schema != RESPONSE_SCHEMA {
        return Err(EnvelopeRefusal::UnknownSchema {
            found: response.schema.clone(),
        });
    }
    if response.warrant != ingesting {
        return Err(EnvelopeRefusal::WrongWarrant {
            named: response.warrant.clone(),
            ingesting: ingesting.to_owned(),
        });
    }
    Ok(())
}

/// Ingest a verifier's response, writing only the verdicts that are admissible.
pub fn ingest(
    repo: &Repository,
    alias: &str,
    response_path: &Utf8PathBuf,
) -> Result<Report, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let assurance = one
        .validated
        .as_ref()
        .map(|v| v.assurance_level.to_string())
        .unwrap_or_else(|| "basic".to_owned());

    let mut report = Report::default();

    let text = fs::read_to_string(response_path).map_err(|source| RepoError::Io {
        context: format!("could not read {response_path}"),
        source,
    })?;
    let response: VerificationResponse = match toml::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            report.push(Diagnostic::error(
                "verify.response-malformed",
                response_path.to_string(),
                e.to_string(),
            ));
            return Ok(report);
        }
    };

    if let Err(refusal) = validate_envelope(&response, alias) {
        let rule = match refusal {
            EnvelopeRefusal::UnknownSchema { .. } => "verify.response-schema",
            EnvelopeRefusal::WrongWarrant { .. } => "verify.response-warrant",
        };
        report.push(Diagnostic::error(
            rule,
            response_path.to_string(),
            refusal.to_string(),
        ));
        return Ok(report);
    }

    // A response may address only declared obligations. Check the entire set
    // before constructing paths or writing any verdict, including valid peers.
    let declared = crate::resolve::declared_obligations(&one);
    for verdict in &response.verifications {
        if !declared.contains(&verdict.obligation) {
            report.push(Diagnostic::error(
                "verify.unknown-obligation",
                response_path.to_string(),
                format!(
                    "{:?} is not a declared obligation of {alias}; nothing was written",
                    verdict.obligation
                ),
            ));
        }
    }
    if report
        .diagnostics
        .iter()
        .any(|d| d.rule == "verify.unknown-obligation")
    {
        return Ok(report);
    }

    if let Some(reviewed) = &response.reviewed_subject {
        let current = subject(repo, &one)?;
        if reviewed != &current {
            report.push(Diagnostic::error("verify.subject-stale", response_path.to_string(), "the response reviewed a different contract or artifact snapshot; nothing was written".to_owned()));
            return Ok(report);
        }
    } else {
        report.push(Diagnostic::unknown("verify.subject-unbound", response_path.to_string(), "legacy verdicts are retained as historical records, but no reviewed subject was supplied; they cannot qualify current work".to_owned()));
    }

    // OW-WAR-0137: on a Warrant whose authorization signed an assignment,
    // only the assigned verifier's verdict is recorded, and only while the
    // register still grants that actor `verifier`. An assignment that is not
    // the one signed refuses the whole response with nothing written.
    // Unassigned Warrants take none of this path.
    let assigned: Option<Vec<String>> = match crate::authorize::assignment_standing(repo, &dir)?
        .for_act(crate::authorize::assignment::Act::Verify)
    {
        Ok(list) => list.map(<[String]>::to_vec),
        Err(finding) => {
            report.push(Diagnostic::error(
                finding.rule,
                response_path.to_string(),
                format!("{alias}: {}", finding.message),
            ));
            report.note("the assignment refused the response whole; nothing was written");
            return Ok(report);
        }
    };
    let register = match &assigned {
        Some(_) => Some(repo.load_authority_register()?),
        None => None,
    };
    let not_assigned = |v: &Verification| -> Option<(&'static str, String)> {
        let (list, register) = (assigned.as_ref()?, register.as_ref()?);
        let actor = &v.verifier.actor;
        if !list.contains(actor) {
            return Some((
                "verify.not-assigned",
                format!(
                    "{}: {actor:?} is not the assigned verifier of {alias}; the assignment the \
                     authorizer signed names {}. Not recorded",
                    v.obligation,
                    list.join(", ")
                ),
            ));
        }
        let entry = register.actor(actor);
        if !entry.is_some_and(|e| e.holds(openwarrant_core::ActorRole::Verifier)) {
            return Some((
                "verify.role-missing",
                format!(
                    "{}: {actor:?} is assigned and does not hold `verifier` in \
                     docs/authority/roles.toml. An assignment narrows who may verify; it never \
                     grants the role. Not recorded",
                    v.obligation
                ),
            ));
        }
        // Two vocabularies: the register's `policy_service` is a verdict's
        // `service`. Anything else must match by name.
        let same_kind = |e: &openwarrant_core::authority::RoleAssignment| {
            matches!(
                (e.actor_kind, v.verifier.kind),
                (
                    openwarrant_core::ActorKind::Human,
                    openwarrant_core::verification::ActorKind::Human
                ) | (
                    openwarrant_core::ActorKind::Agent,
                    openwarrant_core::verification::ActorKind::Agent
                ) | (
                    openwarrant_core::ActorKind::PolicyService,
                    openwarrant_core::verification::ActorKind::Service
                )
            )
        };
        if entry.is_some_and(|e| !same_kind(e)) {
            return Some((
                "verify.kind-mismatch",
                format!(
                    "{}: the verdict says {actor:?} is {:?} and the register says otherwise. \
                     Not recorded",
                    v.obligation, v.verifier.kind
                ),
            ));
        }
        None
    };

    let vdir = dir.join("verifications");
    let mut written = 0usize;
    let mut refused = 0usize;
    let mut replayed = 0usize;

    // OW-WAR-0130 (§67.4): every admissible verdict is rendered and its
    // journal key looked up BEFORE the first write. A verdict already
    // recorded by the same verifier, whose record still holds exactly these
    // bytes, replays: nothing is written and the act exits 0. One recorded
    // by a different actor is a conflicting reuse of the key, and it refuses
    // the whole response with nothing written.
    struct Planned<'a> {
        v: &'a Verification,
        path: Utf8PathBuf,
        rendered: String,
        actor: String,
        payload: String,
        replay: bool,
    }
    let mut planned: Vec<Planned<'_>> = Vec::new();
    let mut conflicts = 0usize;
    for v in &response.verifications {
        if v.admissible_for(&assurance).is_err() || not_assigned(v).is_some() {
            continue;
        }
        let path = vdir.join(format!("{}.toml", v.obligation));
        let rendered = toml::to_string_pretty(v).map_err(|e| RepoError::Io {
            context: format!("could not serialize verification for {}", v.obligation),
            source: std::io::Error::other(e.to_string()),
        })?;
        let record_digest = openwarrant_compiler::digest::sha256_hex(rendered.as_bytes());
        let actor = format!("{}://{}", v.verifier.kind, v.verifier.actor);
        // The record's own digest is in the payload so that a re-verification
        // reaching the same disposition on new evidence is a new event, not a
        // refused duplicate.
        let payload = match &response.reviewed_subject {
            Some(reviewed) => serde_json::to_string(&serde_json::json!({"obligation":v.obligation,"disposition":v.disposition,"record_digest":format!("sha256:{record_digest}"),"reviewed_subject":reviewed})).map_err(|e| RepoError::Message(e.to_string()))?,
            None => format!("{{\"obligation\":\"{}\",\"disposition\":\"{}\",\"record_digest\":\"sha256:{}\"}}", v.obligation, v.disposition, record_digest),
        };
        let replay = match crate::journal_cmd::already_recorded(
            &dir,
            crate::journal_cmd::VERIFICATION_RECORDED,
            &payload,
            &actor,
        )? {
            crate::journal_cmd::Prior::Fresh => false,
            crate::journal_cmd::Prior::Equivalent { .. } => {
                fs::read(&path).is_ok_and(|on_disk| on_disk == rendered.as_bytes())
            }
            crate::journal_cmd::Prior::Conflict { recorded_by } => {
                conflicts += 1;
                report.push(crate::journal_cmd::conflict(
                    repo.relative(&dir.join(crate::journal_cmd::FILE)),
                    crate::journal_cmd::VERIFICATION_RECORDED,
                    &recorded_by,
                    &actor,
                ));
                false
            }
        };
        planned.push(Planned {
            v,
            path,
            rendered,
            actor,
            payload,
            replay,
        });
    }
    if conflicts > 0 {
        report.note(format!(
            "{conflicts} verdict(s) conflict with the journal; the response is refused whole and \
             nothing was written"
        ));
        return Ok(report);
    }

    for v in &response.verifications {
        let why = match v.admissible_for(&assurance) {
            Err(why) => why.to_string(),
            Ok(()) => {
                if let Some((rule, why)) = not_assigned(v) {
                    refused += 1;
                    report.push(Diagnostic::error(rule, response_path.to_string(), why));
                }
                continue;
            }
        };
        // NOT written. A refused verdict must not become a file that later
        // reads as a verification.
        refused += 1;
        report.push(Diagnostic::error(
            "verify.inadmissible",
            response_path.to_string(),
            why,
        ));
    }

    for p in planned {
        let v = p.v;
        if p.replay {
            replayed += 1;
            report.push(Diagnostic::pass(
                "verify.replayed",
                format!(
                    "{}: {} by {} is already recorded with these exact bytes → {}; an \
                     equivalent retry replays and writes nothing",
                    v.obligation,
                    v.disposition,
                    v.verifier.actor,
                    repo.relative(&p.path)
                ),
            ));
            continue;
        }
        fs::create_dir_all(&vdir).map_err(|source| RepoError::Io {
            context: format!("could not create {vdir}"),
            source,
        })?;
        // OW-WAR-0121: temp, fsync, rename. A crash leaves the prior verdict
        // whole, never half of either; a symlink in the record's place is
        // refused by name.
        if let Err(storage) = crate::compile::atomic::write(&p.path, &p.rendered) {
            report.push(storage.diagnostic());
            refused += 1;
            continue;
        }
        written += 1;
        if let Some(vm) = &one.validated {
            crate::journal_cmd::record(
                &dir,
                &vm.uuid.to_string(),
                crate::journal_cmd::VERIFICATION_RECORDED,
                &p.actor,
                &p.payload,
            )?;
        }
        report.push(Diagnostic::pass(
            "verify.recorded",
            format!(
                "{}: {} by {}",
                v.obligation, v.disposition, v.verifier.actor
            ),
        ));
    }

    if replayed > 0 {
        report.note(format!(
            "{replayed} verification(s) replayed, nothing written for them"
        ));
    }
    report.note(format!(
        "{written} verification(s) recorded, {refused} refused. A refused verdict is \
         not written to disk at all — recording it would let an inadmissible verdict \
         read as a verification on the next run."
    ));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_request_carries_no_performer_narrative() {
        // Structural, not a rule: BlindVerifierInput has no field that could
        // hold one, so a request cannot leak the performer's reasoning even by
        // mistake. This test pins the field set against §46.2.
        let json = serde_json::to_string(&BlindVerifierInput::default()).expect("serializes");
        for excluded in ["narrative", "reasoning", "transcript", "rationale"] {
            assert!(
                !json.contains(excluded),
                "§46.2 excludes {excluded}; the request must have no field for it"
            );
        }
    }

    fn response(warrant: &str, schema: &str) -> VerificationResponse {
        VerificationResponse {
            reviewed_subject: None,
            schema: schema.to_owned(),
            warrant: warrant.to_owned(),
            verifications: vec![],
        }
    }

    #[test]
    fn a_well_formed_envelope_is_accepted() {
        assert_eq!(
            validate_envelope(&response("OW-WAR-0014", RESPONSE_SCHEMA), "OW-WAR-0014"),
            Ok(())
        );
    }

    /// A response for a different Warrant would attach verdicts to work the
    /// verifier never examined.
    #[test]
    fn a_response_for_another_warrant_is_refused() {
        assert_eq!(
            validate_envelope(&response("OW-WAR-0099", RESPONSE_SCHEMA), "OW-WAR-0014"),
            Err(EnvelopeRefusal::WrongWarrant {
                named: "OW-WAR-0099".to_owned(),
                ingesting: "OW-WAR-0014".to_owned()
            })
        );
    }

    #[test]
    fn an_unknown_response_schema_is_refused() {
        assert_eq!(
            validate_envelope(
                &response("OW-WAR-0014", "oh.war/verification-response/v2"),
                "OW-WAR-0014"
            ),
            Err(EnvelopeRefusal::UnknownSchema {
                found: "oh.war/verification-response/v2".to_owned()
            })
        );
    }

    #[test]
    fn schemas_are_versioned() {
        assert!(REQUEST_SCHEMA.ends_with("/v1"));
        assert!(RESPONSE_SCHEMA.ends_with("/v1"));
        assert_ne!(REQUEST_SCHEMA, RESPONSE_SCHEMA);
    }
}
