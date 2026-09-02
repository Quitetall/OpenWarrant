// SPDX-License-Identifier: AGPL-3.0-or-later
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

    let vdir = dir.join("verifications");
    let mut written = 0usize;
    let mut refused = 0usize;

    for v in &response.verifications {
        match v.admissible_for(&assurance) {
            Ok(()) => {
                fs::create_dir_all(&vdir).map_err(|source| RepoError::Io {
                    context: format!("could not create {vdir}"),
                    source,
                })?;
                let path = vdir.join(format!("{}.toml", v.obligation));
                let rendered = toml::to_string_pretty(v).map_err(|e| RepoError::Io {
                    context: format!("could not serialize verification for {}", v.obligation),
                    source: std::io::Error::other(e.to_string()),
                })?;
                fs::write(&path, rendered).map_err(|source| RepoError::Io {
                    context: format!("could not write {path}"),
                    source,
                })?;
                written += 1;
                if let Some(vm) = &one.validated {
                    crate::journal_cmd::record(
                        &dir,
                        &vm.uuid.to_string(),
                        crate::journal_cmd::VERIFICATION_RECORDED,
                        &format!("{}://{}", v.verifier.kind, v.verifier.actor),
                        &format!(
                            "{{\"obligation\":\"{}\",\"disposition\":\"{}\"}}",
                            v.obligation, v.disposition
                        ),
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
            Err(why) => {
                // NOT written. A refused verdict must not become a file that
                // later reads as a verification.
                refused += 1;
                report.push(Diagnostic::error(
                    "verify.inadmissible",
                    response_path.to_string(),
                    why.to_string(),
                ));
            }
        }
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
