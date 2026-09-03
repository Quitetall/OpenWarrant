// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war authorize` — the §28.4 authorization seam.
//!
//! # Two halves, and why this command cannot sign anything
//!
//! `war authorize <alias>` EMITS an authorization request. `war authorize
//! <alias> --response <file>` INGESTS what a human returned. Nothing in between
//! decides anything, for the same reason `war verify` is split: §27.2 says an
//! agent SHALL NOT authorize a proposed WAR, and a command that filled in an
//! authorizer would be doing exactly that with extra steps.
//!
//! The split is not merely a convention an agent is asked to respect. Ingestion
//! resolves the named authorizer against the repository's role register, and
//! [`RoleAssignment::may_authorize`] refuses every agent regardless of what the
//! response claims. An agent that wrote itself an authorization response would
//! have the record refused rather than written.
//!
//! # The digest is the point of requirement 1
//!
//! §56.1's first requirement is the *exact* authorized Contract Revision. So the
//! request carries the contract digest computed from the Warrant as it stands,
//! the response must echo that same digest back, and ingestion refuses a
//! mismatch. A signature returned against a digest that has since moved is a
//! signature on a document nobody is holding any more — which is the failure the
//! word "exact" is there to prevent.
//!
//! # What is written, and what is refused
//!
//! An admissible response writes `authorization.toml` (the authorized
//! [`ContractRevision`]) and, when judgments were returned, `judgments.toml`
//! (§42). A refused response writes NOTHING. A rejected authorization must not
//! become a file that later reads as authority, which is the same rule
//! `war verify` applies to refused verdicts.

use std::fs;

use camino::Utf8PathBuf;
use openwarrant_compiler::lower;
use openwarrant_core::authority::{ActorRole, AuthorityRegister, RoleAssignment};
use openwarrant_core::contract::{
    ActorKind, Authorization, ContractRevision, Independence, RevisionState,
};
use openwarrant_core::epistemic::Judgment;
use openwarrant_core::rationale::{Assumption, EpistemicStatus};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

pub const REQUEST_SCHEMA: &str = "oh.war/authorization-request/v1";
pub const RESPONSE_SCHEMA: &str = "oh.war/authorization-response/v1";
pub const AUTHORIZATION_SCHEMA: &str = "oh.war/authorization/v1";
pub const JUDGMENTS_SCHEMA: &str = "oh.war/judgments/v1";

/// What is put to the authorizer.
///
/// Deliberately carries no recommendation and no "suggested meaning": §42 says
/// an approval with no stated meaning is invalid, and pre-filling the meaning
/// would make the authorizer a signatory to text an agent wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationRequest {
    pub schema: String,
    pub warrant: String,
    pub title: String,
    pub assurance_level: String,
    /// The exact revision being put forward (§56.1 requirement 1).
    pub contract_digest: String,
    /// Which of §28.5's seventeen elements that digest actually covers
    /// (OW-ADR-0004). An authorizer is entitled to know they are signing a
    /// partial digest.
    pub contract_coverage: Vec<String>,
    /// §28.3 — who proposed it. Echoed so ingestion can detect
    /// self-authorization rather than trusting the response to admit it.
    pub proposer: String,
    pub obligations: Vec<RequestedObligation>,
    /// Assumptions carrying `accepted_residual_risk` (§36.2). Each needs a
    /// judgment from an actor with authority to accept it (§27.2).
    pub residual_risks: Vec<RequestedResidualRisk>,
    /// Actors the register says may authorize this. Informational — ingestion
    /// re-derives it rather than trusting the response to have used the list.
    pub eligible_authorizers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestedObligation {
    pub id: String,
    pub statement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestedResidualRisk {
    pub assumption_id: String,
    pub statement: String,
    /// §36.2 — a residual risk with no stated consequence is not a risk anyone
    /// can weigh. `rationale.rs` already refuses to validate one.
    pub consequence_if_false: String,
    /// The judgment the assumption points at, empty when it points at none.
    pub judgment_ref: String,
}

/// What the authorizer returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationResponse {
    pub schema: String,
    pub warrant: String,
    /// Must equal the request's. See the module docs on requirement 1.
    pub contract_digest: String,
    pub authorizer: String,
    /// §27.4 — the role ACTUALLY exercised, not every role held.
    pub acting_role: String,
    /// §28.4 — what authorizing meant here.
    pub meaning: String,
    pub effective_time: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_basis: Option<String>,
    pub independence: Independence,
    /// §42 judgments made at the same moment, including residual-risk
    /// acceptances. Optional: a Warrant may need none.
    #[serde(default)]
    pub judgment: Vec<Judgment>,
}

/// The persisted authorization record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationRecord {
    pub schema: String,
    pub warrant: String,
    pub revision: ContractRevision,
    /// §14 — the SAS revision the authorized contract was compiled against.
    /// Fixed here so that a LATER SAS revision does not move this Warrant's
    /// contract digest out from under its signature: an authorized contract
    /// keeps its Basis until an amendment re-authorizes it. Absent only on
    /// records written before this field existed; those read as the latest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sas_revision: Option<String>,
}

/// The persisted judgment set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgmentRecord {
    pub schema: String,
    pub warrant: String,
    #[serde(default)]
    pub judgment: Vec<Judgment>,
}

/// Build the request for one Warrant.
pub fn request(repo: &Repository, alias: &str) -> Result<AuthorizationRequest, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
        return Err(RepoError::Message(format!(
            "{alias}: the manifest did not validate, so there is no contract to authorize"
        )));
    };

    let ir = lower(basis, validated)
        .map_err(|e| RepoError::Message(format!("{alias}: could not compile contract: {e}")))?;
    let contract_digest = ir
        .contract_digest()
        .map_err(|e| RepoError::Message(format!("{alias}: could not digest contract: {e}")))?;

    let obligations = obligations_of(basis)
        .into_iter()
        .map(|o| RequestedObligation {
            id: o.id,
            statement: o.statement,
        })
        .collect();

    let register = repo.load_authority_register()?;
    let assumptions = repo.load_rationale(&dir)?.unwrap_or_default();

    Ok(AuthorizationRequest {
        schema: REQUEST_SCHEMA.to_owned(),
        warrant: alias.to_owned(),
        title: validated.raw.title.clone(),
        assurance_level: validated.assurance_level.to_string(),
        contract_digest,
        contract_coverage: ir
            .contract_coverage
            .covered()
            .map(ToString::to_string)
            .collect(),
        proposer: repo.performer(),
        obligations,
        residual_risks: residual_risks_in(&assumptions),
        // Filtered by `may_authorize`, not by `holds`. Holding the authorizer
        // role is necessary and not sufficient — an agent holding it is still
        // refused by §27.2, as is a human who proposed this Warrant. Listing
        // either here would tell the reader to route the request to somebody
        // whose signature ingestion is guaranteed to reject.
        eligible_authorizers: register
            .holders(ActorRole::Authorizer)
            .filter(|a| a.may_authorize(&repo.performer()).is_ok())
            .map(|a| a.actor.clone())
            .collect(),
    })
}

/// Obligations declared across a Warrant's assurance atoms.
fn obligations_of(
    basis: &openwarrant_compiler::CompilationBasis,
) -> Vec<openwarrant_core::Obligation> {
    basis
        .atoms
        .iter()
        .filter(|a| a.role == "assurance")
        .filter_map(|a| {
            openwarrant_core::obligation::parse(&String::from_utf8_lossy(&a.bytes)).ok()
        })
        .flat_map(|set| set.obligations)
        .collect()
}

/// The assumptions carrying `accepted_residual_risk` (§36.2).
///
/// Takes the declared assumptions rather than reading them, so the caller
/// decides what an ABSENT `rationale.toml` means. That distinction is the whole
/// reason this is not a one-liner: a Warrant that declared no assumptions and a
/// Warrant that was never asked both produce an empty list here, and only the
/// caller knows which it is holding.
#[must_use]
pub fn residual_risks_in(assumptions: &[Assumption]) -> Vec<RequestedResidualRisk> {
    assumptions
        .iter()
        .filter(|a| a.epistemic_status == EpistemicStatus::AcceptedResidualRisk)
        .map(|a| RequestedResidualRisk {
            assumption_id: a.id.clone(),
            statement: a.statement.clone(),
            consequence_if_false: a.consequence_if_false.clone(),
            judgment_ref: a.judgment_ref.clone(),
        })
        .collect()
}

/// Why a response was refused before anything was written.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    UnknownSchema {
        found: String,
    },
    WrongWarrant {
        named: String,
        ingesting: String,
    },
    /// The signature is against a contract revision that is no longer the one
    /// on disk (§56.1 requirement 1).
    StaleDigest {
        signed: String,
        current: String,
    },
    /// The named authorizer holds no role assignment at all.
    UnknownActor {
        actor: String,
    },
    /// The register refused the act — agent, self-authorization, or a role the
    /// actor does not hold.
    NotPermitted {
        detail: String,
    },
}

impl std::fmt::Display for Refusal {
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
            Self::StaleDigest { signed, current } => write!(
                f,
                "the response authorizes contract {signed} but the Warrant now compiles \
                 to {current}. §56.1 requires the EXACT authorized revision, so this \
                 signature covers a document that no longer exists. Re-issue the request \
                 and have it signed again"
            ),
            Self::UnknownActor { actor } => write!(
                f,
                "{actor:?} holds no role assignment in docs/authority/roles.toml, so \
                 there is no authority to exercise. Authority comes from a record a \
                 human wrote, never from a name supplied in the response itself"
            ),
            Self::NotPermitted { detail } => f.write_str(detail),
        }
    }
}

/// Check the response envelope and the authorizer's standing, writing nothing.
///
/// Separated from [`ingest`] so every refusal is testable without a repository
/// on disk — including the two that matter most, an agent signing and a
/// signature against a stale digest.
pub fn validate_response(
    response: &AuthorizationResponse,
    ingesting: &str,
    current_digest: &str,
    proposer: &str,
    register: &AuthorityRegister,
) -> Result<(), Refusal> {
    if response.schema != RESPONSE_SCHEMA {
        return Err(Refusal::UnknownSchema {
            found: response.schema.clone(),
        });
    }
    if response.warrant != ingesting {
        return Err(Refusal::WrongWarrant {
            named: response.warrant.clone(),
            ingesting: ingesting.to_owned(),
        });
    }
    if response.contract_digest != current_digest {
        return Err(Refusal::StaleDigest {
            signed: response.contract_digest.clone(),
            current: current_digest.to_owned(),
        });
    }
    let assignment = register
        .actor(&response.authorizer)
        .ok_or_else(|| Refusal::UnknownActor {
            actor: response.authorizer.clone(),
        })?;
    assignment
        .may_authorize(proposer)
        .map_err(|e| Refusal::NotPermitted {
            detail: e.to_string(),
        })?;
    Ok(())
}

/// Whether a judgment's actor may make it (§42, §27.2).
///
/// A residual-risk acceptance needs `risk_acceptor`; anything else needs
/// `judge`. Both are refused to agents by [`RoleAssignment`], so a judgment an
/// agent "made" cannot be recorded here even if the response asserts it.
fn judgment_is_permitted(judgment: &Judgment, register: &AuthorityRegister) -> Result<(), String> {
    judgment.validate().map_err(|e| e.to_string())?;
    let assignment: &RoleAssignment = register.actor(&judgment.actor).ok_or_else(|| {
        format!(
            "{:?} holds no role assignment, so it cannot make a judgment (§42)",
            judgment.actor
        )
    })?;
    // Agent-hood is checked BEFORE the role, and the order is deliberate. It is
    // the stronger statement: granting an agent the judge role would not make it
    // eligible, so reporting "does not hold the judge role" first would suggest
    // a fix that does not work.
    //
    // §27.1 lets an agent RECOMMEND a judgment. Recording one as MADE is a
    // different act, and that distinction is the whole of §42.
    if assignment.actor_kind == ActorKind::Agent {
        return Err(format!(
            "{:?} is an agent. §27.1 permits recommending a judgment, not making one — \
             record it as a recommendation. Granting the judge role would not change \
             this",
            judgment.actor
        ));
    }
    if judgment.kind == "residual_risk_acceptance" {
        assignment
            .may_accept_residual_risk()
            .map_err(|e| e.to_string())
    } else if assignment.holds(ActorRole::Judge) {
        Ok(())
    } else {
        Err(format!(
            "{:?} does not hold the judge role (§27.4)",
            judgment.actor
        ))
    }
}

/// Ingest an authorization response, writing records only if it is admissible.
pub fn ingest(
    repo: &Repository,
    alias: &str,
    response_path: &Utf8PathBuf,
) -> Result<Report, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let mut report = Report::default();

    let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
        report.push(Diagnostic::error(
            "authorize.not-compilable",
            dir.to_string(),
            format!("{alias}: the manifest did not validate, so there is no contract to authorize"),
        ));
        return Ok(report);
    };
    let ir = lower(basis, validated)
        .map_err(|e| RepoError::Message(format!("{alias}: could not compile contract: {e}")))?;
    let current_digest = ir
        .contract_digest()
        .map_err(|e| RepoError::Message(format!("{alias}: could not digest contract: {e}")))?;

    let text = fs::read_to_string(response_path).map_err(|source| RepoError::Io {
        context: format!("could not read {response_path}"),
        source,
    })?;
    let response: AuthorizationResponse = match toml::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            report.push(Diagnostic::error(
                "authorize.response-malformed",
                response_path.to_string(),
                e.to_string(),
            ));
            return Ok(report);
        }
    };

    let register = repo.load_authority_register()?;
    let proposer = repo.performer();

    if let Err(refusal) = validate_response(&response, alias, &current_digest, &proposer, &register)
    {
        let rule = match refusal {
            Refusal::UnknownSchema { .. } => "authorize.response-schema",
            Refusal::WrongWarrant { .. } => "authorize.response-warrant",
            Refusal::StaleDigest { .. } => "authorize.stale-digest",
            Refusal::UnknownActor { .. } => "authorize.unknown-actor",
            Refusal::NotPermitted { .. } => "authorize.not-permitted",
        };
        report.push(Diagnostic::error(
            rule,
            response_path.to_string(),
            refusal.to_string(),
        ));
        return Ok(report);
    }

    // Every judgment is checked BEFORE anything is written. A response that
    // authorizes correctly but carries one inadmissible judgment must not leave
    // a valid authorization next to a silently dropped judgment — that would
    // read, later, as a Warrant whose judgments were simply never needed.
    let mut judgment_failures = Vec::new();
    for j in &response.judgment {
        if let Err(detail) = judgment_is_permitted(j, &register) {
            judgment_failures.push(format!("{}: {detail}", j.id));
        }
    }
    if !judgment_failures.is_empty() {
        for detail in judgment_failures {
            report.push(Diagnostic::error(
                "authorize.judgment-not-permitted",
                response_path.to_string(),
                detail,
            ));
        }
        return Ok(report);
    }

    let authorization = Authorization {
        authorizer: response.authorizer.clone(),
        // Unreachable after `validate_response`, which refuses `UnknownActor`
        // before this point. Kept rather than unwrapped so a future edit that
        // reorders the guard degrades to the most restricted kind instead of
        // panicking — `Human` is the fallback because an agent reaching here
        // would already have been refused for being one.
        actor_kind: register
            .actor(&response.authorizer)
            .map_or(ActorKind::Human, |a| a.actor_kind),
        acting_role: response.acting_role.clone(),
        meaning: response.meaning.clone(),
        effective_time: response.effective_time.clone(),
        policy_basis: response.policy_basis.clone(),
        independence: response.independence,
    };

    // A first authorization is revision 1. A later one, at a moved digest, is
    // §28's amendment: revision N+1 whose predecessor is the authorized digest
    // — and §31 says every revision after authorization SHALL carry an
    // amendment record, so one must exist under `amendments/` before the
    // signature is accepted. Re-signing the SAME digest is refused: an
    // authorized revision is immutable (§28.3), and there is nothing to add.
    let existing = repo.load_authorization(&dir)?;
    let revision = match existing {
        Some(prev) if prev.revision.contract_digest == current_digest => {
            report.push(Diagnostic::error(
                "authorize.already-authorized",
                response_path.to_string(),
                format!(
                    "{alias}: revision {} is already authorized at this digest; an authorized \
                     revision is immutable (§28.3)",
                    prev.revision.revision
                ),
            ));
            return Ok(report);
        }
        Some(prev) => {
            // Revision N+1 needs N amendment records: one per revision after
            // the first. Counting, rather than "any file exists", is what stops
            // the record written for revision 2 from carrying revision 3 through
            // (found by review). The count is of `.yaml` files under amendments/;
            // each is separately validated by `war check` as a §31 record.
            let amendments = dir
                .join("amendments")
                .read_dir_utf8()
                .map(|it| {
                    it.flatten()
                        .filter(|e| e.path().as_str().ends_with(".yaml"))
                        .count()
                })
                .unwrap_or(0);
            let needed = prev.revision.revision as usize;
            if amendments < needed {
                report.push(Diagnostic::error(
                    "authorize.no-amendment",
                    response_path.to_string(),
                    format!(
                        "{alias}: the contract moved from {} to {} after revision {} was authorized. \
                         §31: every revision after authorization carries an amendment record — \
                         revision {} needs {needed} under amendments/ and {amendments} exist. \
                         Write AM-{:03} before re-signing",
                        prev.revision.contract_digest,
                        current_digest,
                        prev.revision.revision,
                        prev.revision.revision + 1,
                        needed
                    ),
                ));
                return Ok(report);
            }
            prev.revision
                .amend(current_digest.clone(), ir.contract_coverage.clone())
                .and_then(|draft| draft.propose(proposer))
                .and_then(|proposed| proposed.authorize(authorization))
                .map_err(|e| RepoError::Message(format!("{alias}: {e}")))?
        }
        None => ContractRevision::draft(current_digest.clone(), ir.contract_coverage.clone())
            .propose(proposer)
            .and_then(|proposed| proposed.authorize(authorization))
            .map_err(|e| RepoError::Message(format!("{alias}: {e}")))?,
    };

    let record = AuthorizationRecord {
        schema: AUTHORIZATION_SCHEMA.to_owned(),
        warrant: alias.to_owned(),
        revision,
        sas_revision: basis.sas.as_ref().map(|p| p.version.clone()),
    };
    write_toml(&dir.join("authorization.toml"), &record)?;
    if let Some(v) = &one.validated {
        crate::journal_cmd::record(
            &dir,
            &v.uuid.to_string(),
            crate::journal_cmd::AUTHORIZATION_RECORDED,
            &format!("person://{}", response.authorizer),
            &format!(
                "{{\"contract_digest\":\"{}\",\"acting_role\":\"{}\"}}",
                response.contract_digest, response.acting_role
            ),
        )?;
    }
    report.push(Diagnostic::pass(
        "authorize.recorded",
        format!(
            "{alias}: contract {current_digest} authorized by {} acting as {} → {}",
            response.authorizer,
            response.acting_role,
            dir.join("authorization.toml")
        ),
    ));

    if !response.judgment.is_empty() {
        let judgments = JudgmentRecord {
            schema: JUDGMENTS_SCHEMA.to_owned(),
            warrant: alias.to_owned(),
            judgment: response.judgment.clone(),
        };
        write_toml(&dir.join("judgments.toml"), &judgments)?;
        report.push(Diagnostic::pass(
            "authorize.judgments-recorded",
            format!(
                "{alias}: {} judgment(s) recorded → {}",
                response.judgment.len(),
                dir.join("judgments.toml")
            ),
        ));
    }

    Ok(report)
}

fn write_toml<T: Serialize>(path: &camino::Utf8Path, value: &T) -> Result<(), RepoError> {
    let rendered = toml::to_string_pretty(value).map_err(|e| RepoError::Message(e.to_string()))?;
    fs::write(path, rendered).map_err(|source| RepoError::Io {
        context: format!("could not write {path}"),
        source,
    })
}

/// Whether a persisted authorization covers the contract as it stands now.
///
/// §56.1 requirement 1 in one function. Both halves matter: an authorization in
/// any state other than `Authorized` is a proposal, and one whose digest has
/// moved covers a revision that no longer exists.
#[must_use]
pub fn authorizes_current_contract(record: &AuthorizationRecord, current_digest: &str) -> bool {
    record.revision.state == RevisionState::Authorized
        && record.revision.contract_digest == current_digest
        && record.revision.authorization.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwarrant_core::authority::RoleAssignment;

    fn register() -> AuthorityRegister {
        AuthorityRegister::new(vec![
            RoleAssignment {
                actor: "brian".to_owned(),
                actor_kind: ActorKind::Human,
                roles: [ActorRole::Authorizer, ActorRole::Judge, ActorRole::Resolver]
                    .into_iter()
                    .collect(),
                assigned_by: "owner".to_owned(),
                effective_time: "2026-08-25T00:00:00Z".to_owned(),
                note: None,
            },
            RoleAssignment {
                actor: "claude".to_owned(),
                actor_kind: ActorKind::Agent,
                roles: [
                    ActorRole::Performer,
                    ActorRole::Authorizer,
                    ActorRole::Judge,
                    ActorRole::RiskAcceptor,
                ]
                .into_iter()
                .collect(),
                assigned_by: "owner".to_owned(),
                effective_time: "2026-08-25T00:00:00Z".to_owned(),
                // Deliberately over-granted. The agent refusals below must fire
                // because it IS an agent, not because a role is missing — a
                // fixture that withheld the roles would pass against an
                // implementation that had no agent check at all.
                note: Some("over-granted so the agent refusals are the reason".to_owned()),
            },
        ])
    }

    fn response(authorizer: &str, digest: &str) -> AuthorizationResponse {
        AuthorizationResponse {
            schema: RESPONSE_SCHEMA.to_owned(),
            warrant: "OW-WAR-0014".to_owned(),
            contract_digest: digest.to_owned(),
            authorizer: authorizer.to_owned(),
            acting_role: "owner".to_owned(),
            meaning: "Accept the declared deliverables against the bounded obligations.".to_owned(),
            effective_time: "2026-08-25T12:00:00Z".to_owned(),
            policy_basis: None,
            independence: Independence::None,
            judgment: vec![],
        }
    }

    #[test]
    fn a_well_formed_human_authorization_is_accepted() {
        assert!(
            validate_response(
                &response("brian", "sha256:abc"),
                "OW-WAR-0014",
                "sha256:abc",
                "claude",
                &register(),
            )
            .is_ok(),
            "the positive case must pass, or the refusals below prove nothing"
        );
    }

    #[test]
    fn an_agent_cannot_authorize_even_holding_the_role() {
        let refusal = validate_response(
            &response("claude", "sha256:abc"),
            "OW-WAR-0014",
            "sha256:abc",
            "brian",
            &register(),
        )
        .expect_err("an agent must be refused");
        assert!(
            matches!(refusal, Refusal::NotPermitted { .. }),
            "got {refusal:?}"
        );
    }

    #[test]
    fn an_authorizer_who_proposed_it_is_refused() {
        let refusal = validate_response(
            &response("brian", "sha256:abc"),
            "OW-WAR-0014",
            "sha256:abc",
            "brian",
            &register(),
        )
        .expect_err("self-authorization must be refused");
        assert!(matches!(refusal, Refusal::NotPermitted { .. }));
    }

    #[test]
    fn a_signature_against_a_moved_digest_is_refused() {
        let refusal = validate_response(
            &response("brian", "sha256:old"),
            "OW-WAR-0014",
            "sha256:new",
            "claude",
            &register(),
        )
        .expect_err("a stale digest must be refused");
        assert!(matches!(refusal, Refusal::StaleDigest { .. }));
    }

    #[test]
    fn an_actor_with_no_assignment_has_no_authority() {
        let refusal = validate_response(
            &response("someone", "sha256:abc"),
            "OW-WAR-0014",
            "sha256:abc",
            "claude",
            &register(),
        )
        .expect_err("an unknown actor must be refused");
        assert!(matches!(refusal, Refusal::UnknownActor { .. }));
    }

    #[test]
    fn a_response_for_another_warrant_is_refused() {
        let mut r = response("brian", "sha256:abc");
        r.warrant = "OW-WAR-0001".to_owned();
        let refusal = validate_response(&r, "OW-WAR-0014", "sha256:abc", "claude", &register())
            .expect_err("a mismatched warrant must be refused");
        assert!(matches!(refusal, Refusal::WrongWarrant { .. }));
    }

    #[test]
    fn an_unknown_schema_is_refused() {
        let mut r = response("brian", "sha256:abc");
        r.schema = "oh.war/something-else/v1".to_owned();
        let refusal = validate_response(&r, "OW-WAR-0014", "sha256:abc", "claude", &register())
            .expect_err("an unknown schema must be refused");
        assert!(matches!(refusal, Refusal::UnknownSchema { .. }));
    }

    fn judgment(actor: &str, kind: &str) -> Judgment {
        Judgment {
            id: "J-001".to_owned(),
            kind: kind.to_owned(),
            statement: "The residual risk is acceptable for this release.".to_owned(),
            actor: actor.to_owned(),
            acting_role: "owner".to_owned(),
            meaning: "Accepted knowingly, with the consequence stated.".to_owned(),
            basis_refs: vec!["assumption://A-1".to_owned()],
            authority: openwarrant_core::JudgmentAuthority::Authorized,
            limitations: vec![],
        }
    }

    #[test]
    fn a_human_risk_acceptance_needs_the_risk_acceptor_role() {
        // brian holds authorizer, judge and resolver — but NOT risk_acceptor.
        let err =
            judgment_is_permitted(&judgment("brian", "residual_risk_acceptance"), &register())
                .expect_err("holding judge is not holding risk_acceptor");
        assert!(err.contains("risk_acceptor"), "got {err}");
    }

    #[test]
    fn an_agent_may_recommend_a_judgment_but_not_make_one() {
        let err = judgment_is_permitted(&judgment("claude", "adequacy"), &register())
            .expect_err("an agent must not record a made judgment");
        assert!(err.contains("recommend"), "got {err}");
    }

    /// A scratch repository, removed when the test ends.
    struct Scratch(Utf8PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A real repository containing one real Warrant, copied from this tree.
    ///
    /// The Warrant is COPIED rather than hand-built so the round trip below runs
    /// against a manifest that actually compiles. A hand-rolled fixture would
    /// drift from the real format, and the first thing to break would be the
    /// contract digest — the one value this whole seam turns on.
    fn scratch_repo() -> (Scratch, Repository) {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);

        let root = Utf8PathBuf::from_path_buf(std::env::temp_dir().join(format!(
            "war-authorize-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
        .expect("temp dir path is utf8");
        let repo_root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize_utf8()
            .expect("workspace root");

        fs::create_dir_all(root.join("docs/warrants/OW-WAR-0014")).expect("create scratch");
        fs::copy(
            repo_root.join("openwarrant.toml"),
            root.join("openwarrant.toml"),
        )
        .expect("copy config");
        copy_tree(
            &repo_root.join("docs/warrants/OW-WAR-0014"),
            &root.join("docs/warrants/OW-WAR-0014"),
        );

        let repo = Repository::open(root.clone()).expect("scratch repository opens");
        (Scratch(root), repo)
    }

    fn copy_tree(from: &camino::Utf8Path, to: &camino::Utf8Path) {
        fs::create_dir_all(to).expect("create dir");
        for entry in fs::read_dir(from).expect("read dir") {
            let entry = entry.expect("entry");
            let name = entry.file_name();
            let name = name.to_str().expect("utf8 name");
            let src = from.join(name);
            let dst = to.join(name);
            if entry.file_type().expect("file type").is_dir() {
                copy_tree(&src, &dst);
            } else {
                fs::copy(&src, &dst).expect("copy file");
            }
        }
    }

    fn write_register(repo: &Repository) {
        fs::create_dir_all(repo.root.join("docs/authority")).expect("create authority dir");
        #[derive(Serialize)]
        struct File<'a> {
            assignment: &'a [RoleAssignment],
        }
        let register = register();
        let rendered = toml::to_string_pretty(&File {
            assignment: &register.assignments,
        })
        .expect("render register");
        fs::write(repo.root.join("docs/authority/roles.toml"), rendered).expect("write register");
    }

    fn write_response(repo: &Repository, response: &AuthorizationResponse) -> Utf8PathBuf {
        let path = repo.root.join("response.toml");
        fs::write(&path, toml::to_string_pretty(response).expect("render")).expect("write");
        path
    }

    /// The whole seam, end to end, against a repository on disk.
    ///
    /// Everything above this point tests `validate_response` in isolation, which
    /// cannot catch a wiring mistake — a working validator called with the wrong
    /// digest, or an ingest that writes before it checks. This drives the real
    /// command.
    #[test]
    fn a_signed_response_becomes_a_record_and_a_refused_one_writes_nothing() {
        let (_scratch, repo) = scratch_repo();
        write_register(&repo);
        // The scratch repo is a copy of the real corpus, which since 2026-09-02
        // carries the owner's records. This test is about a Warrant with NONE,
        // so start from that state explicitly rather than assuming it.
        let warrant_dir = repo.root.join("docs/warrants/OW-WAR-0014");
        for f in ["authorization.toml", "judgments.toml"] {
            let _ = fs::remove_file(warrant_dir.join(f));
        }

        let request = request(&repo, "OW-WAR-0014").expect("request builds");
        assert_eq!(
            request.eligible_authorizers,
            vec!["brian".to_owned()],
            "only the human holds the authorizer role in an admissible way"
        );
        assert_eq!(
            request.proposer, "claude",
            "the performer identity is what self-authorization is measured against"
        );

        // Refused first, so the assertion that nothing was written cannot be
        // satisfied by a file this test has not yet created.
        let mut bad = response("claude", &request.contract_digest);
        let path = write_response(&repo, &bad);
        let report = ingest(&repo, "OW-WAR-0014", &path).expect("ingest runs");
        assert!(!report.is_ready(), "an agent's signature must be refused");
        let record_path = repo
            .root
            .join("docs/warrants/OW-WAR-0014/authorization.toml");
        assert!(
            !record_path.exists(),
            "a refused authorization must not leave a file that later reads as authority"
        );

        // A human, but signing a digest that has moved.
        bad = response("brian", "sha256:0000000000000000");
        let path = write_response(&repo, &bad);
        let report = ingest(&repo, "OW-WAR-0014", &path).expect("ingest runs");
        assert!(!report.is_ready(), "a stale digest must be refused");
        assert!(!record_path.exists(), "still nothing written");

        // The real thing.
        let good = response("brian", &request.contract_digest);
        let path = write_response(&repo, &good);
        let report = ingest(&repo, "OW-WAR-0014", &path).expect("ingest runs");
        assert!(
            report.is_ready(),
            "a human authorizer signing the current digest must be accepted: {report:?}"
        );
        assert!(record_path.exists(), "the record must be written");

        let written = repo
            .load_authorization(&repo.root.join("docs/warrants/OW-WAR-0014"))
            .expect("record loads")
            .expect("record is present");
        assert!(
            authorizes_current_contract(&written, &request.contract_digest),
            "the round trip must satisfy §56.1 requirement 1"
        );

        // And the digest binding is not decoration: move the contract and the
        // same record must stop satisfying requirement 1.
        assert!(
            !authorizes_current_contract(&written, "sha256:something-else"),
            "an authorization must not survive the contract moving underneath it"
        );
    }

    #[test]
    fn an_authorization_must_be_authorized_and_current() {
        let record = AuthorizationRecord {
            schema: AUTHORIZATION_SCHEMA.to_owned(),
            warrant: "OW-WAR-0014".to_owned(),
            revision: ContractRevision::draft(
                "sha256:abc".to_owned(),
                openwarrant_core::ContractCoverage::new([]),
            ),
            sas_revision: None,
        };
        assert!(
            !authorizes_current_contract(&record, "sha256:abc"),
            "a draft revision is not an authorization, however current its digest"
        );
    }
}
