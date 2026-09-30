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

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use camino::Utf8PathBuf;
use openwarrant_compiler::{DigestDomain, lower, sha256_digest};
use openwarrant_core::authority::{ActorRole, AuthorityRegister, RoleAssignment};
use openwarrant_core::contract::{
    ActorKind, Authorization, ContractRevision, Independence, RevisionState,
};
use openwarrant_core::epistemic::Judgment;
use openwarrant_core::gate::{GateBinding, GateRef};
use openwarrant_core::rationale::{Assumption, EpistemicStatus};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository, read_repository_regular_bounded};

pub const REQUEST_SCHEMA: &str = "oh.war/authorization-request/v2";
pub const RESPONSE_SCHEMA: &str = "oh.war/authorization-response/v2";
pub const AUTHORIZATION_SCHEMA: &str = "oh.war/authorization/v2";
pub const LEGACY_AUTHORIZATION_SCHEMA: &str = "oh.war/authorization/v1";
pub const JUDGMENTS_SCHEMA: &str = "oh.war/judgments/v1";
const MAX_AUTHORIZATION_GATE_BINDINGS: usize = 256;
const MAX_AUTHORIZATION_GATE_BINDING_BYTES: u64 = 1024 * 1024;

/// Exact §43.5 Gate Binding identity authorized alongside one Contract Revision.
///
/// This lives in the authorization envelope rather than the contract preimage:
/// a Binding may contain the resulting contract digest as one of its subjects,
/// so putting the Binding digest inside that same preimage would be circular.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizedGateBinding {
    pub id: String,
    /// Canonical `<gate-id>@<version>` key.
    pub gate: String,
    /// Domain-separated `oh.war/gate-binding/v1` logical digest.
    pub digest: String,
    /// Repository-relative immutable source object reviewed by the authorizer.
    pub source_ref: String,
}

/// What is put to the authorizer.
///
/// Deliberately carries no recommendation and no "suggested meaning": §42 says
/// an approval with no stated meaning is invalid, and pre-filling the meaning
/// would make the authorizer a signatory to text an agent wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// Exact candidate Bindings presented to the human authorizer. The response
    /// must select one candidate for every gate cited by this Contract Revision.
    pub candidate_gate_bindings: Vec<AuthorizedGateBinding>,
    /// Gate keys cited by this exact Contract Revision. Human selection must
    /// cover each key exactly once.
    pub required_gates: Vec<String>,
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
#[serde(deny_unknown_fields)]
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
    /// Exact Gate Bindings authorized for this Contract Revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_bindings: Option<Vec<AuthorizedGateBinding>>,
    /// §42 judgments made at the same moment, including residual-risk
    /// acceptances. Optional: a Warrant may need none.
    #[serde(default)]
    pub judgment: Vec<Judgment>,
}

/// The persisted authorization record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationRecord {
    pub schema: String,
    pub warrant: String,
    pub revision: ContractRevision,
    /// Empty only on preserved v1 history. A revision citing a gate cannot use
    /// such history as current authorization for Gate evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_bindings: Option<Vec<AuthorizedGateBinding>>,
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
    let required_gates = required_gate_keys(basis)?;
    let candidate_gate_bindings =
        load_candidate_gate_bindings(repo, alias, &contract_digest, &required_gates)?;

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
        candidate_gate_bindings,
        required_gates: required_gates.into_iter().collect(),
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

fn required_gate_keys(
    basis: &openwarrant_compiler::CompilationBasis,
) -> Result<BTreeSet<String>, RepoError> {
    let mut keys = BTreeSet::new();
    for atom in basis.atoms.iter().filter(|atom| atom.role == "assurance") {
        for uri in openwarrant_core::gate::cited_gate_uris(&String::from_utf8_lossy(&atom.bytes)) {
            let gate = GateRef::parse_uri(&uri).map_err(|error| {
                RepoError::Message(format!("malformed required Gate citation {uri:?}: {error}"))
            })?;
            keys.insert(gate.key());
        }
    }
    Ok(keys)
}

fn contract_subject(contract_digest: &str) -> Result<String, RepoError> {
    let digest = contract_digest
        .strip_prefix("sha256:")
        .unwrap_or(contract_digest);
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(RepoError::Message(format!(
            "contract digest {contract_digest:?} is not canonical sha256"
        )));
    }
    Ok(format!("contract:sha256:{digest}"))
}

/// Read exact Warrant-local Gate Binding candidates presented for authorization.
///
/// Candidate discovery is deliberately confined to the Warrant directory. A
/// receipt sibling written by a previous run cannot silently become a candidate
/// for a later authorization merely because both live under repository control.
fn load_candidate_gate_bindings(
    repo: &Repository,
    alias: &str,
    contract_digest: &str,
    required_gates: &BTreeSet<String>,
) -> Result<Vec<AuthorizedGateBinding>, RepoError> {
    let warrant_dir = repo.warrant_dir(alias)?;
    let dir = warrant_dir.join("gate-bindings");
    if required_gates.is_empty() {
        if dir.exists() {
            return Err(RepoError::Message(format!(
                "{alias} declares no required gates but has Gate Binding candidates in {dir}"
            )));
        }
        return Ok(Vec::new());
    }
    let entries = dir.read_dir_utf8().map_err(|source| RepoError::Io {
        context: format!(
            "{alias} cites gates but its Gate Binding candidate directory {dir} is unavailable"
        ),
        source,
    })?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| RepoError::Io {
            context: format!("could not enumerate Gate Binding candidates in {dir}"),
            source,
        })?;
        let path = entry.into_path();
        if path
            .file_name()
            .is_some_and(|name| name.ends_with(".binding.json"))
        {
            paths.push(path);
        }
    }
    paths.sort();
    if paths.len() > MAX_AUTHORIZATION_GATE_BINDINGS {
        return Err(RepoError::Message(format!(
            "{alias} has {} Gate Binding candidates; limit is {MAX_AUTHORIZATION_GATE_BINDINGS}",
            paths.len()
        )));
    }

    let mut gate_report = Report::default();
    let registry = crate::check::load_gate_registry(repo, &mut gate_report);
    if !gate_report.is_ready() {
        return Err(RepoError::Message(
            "Gate Binding authorization requires a fully admissible Gate Definition registry"
                .to_owned(),
        ));
    }
    let required_contract_subject = contract_subject(contract_digest)?;
    let mut candidates = Vec::with_capacity(paths.len());
    let mut seen_ids = BTreeSet::new();
    let mut seen_sources = BTreeSet::new();
    for path in paths {
        let bytes = read_repository_regular_bounded(
            &path,
            &repo.root,
            "authorization Gate Binding",
            MAX_AUTHORIZATION_GATE_BINDING_BYTES,
        )?;
        let binding: GateBinding = openwarrant_core::legacy_disposition::parse_strict_json(&bytes)
            .map_err(|error| {
                RepoError::Message(format!(
                    "cannot parse Gate Binding candidate {path}: {error}"
                ))
            })?;
        if binding.id.trim().is_empty() || binding.id.trim() != binding.id {
            return Err(RepoError::Message(format!(
                "Gate Binding candidate {path} has a blank or noncanonical id"
            )));
        }
        let gate = binding.gate.key();
        if !required_gates.contains(&gate) {
            return Err(RepoError::Message(format!(
                "Gate Binding candidate {path} binds uncited gate {gate}"
            )));
        }
        registry.validate_binding(&binding).map_err(|error| {
            RepoError::Message(format!(
                "Gate Binding candidate {path} is inadmissible: {error}"
            ))
        })?;
        if !binding
            .subjects
            .iter()
            .any(|subject| subject == &required_contract_subject)
        {
            return Err(RepoError::Message(format!(
                "Gate Binding candidate {path} does not bind exact subject {required_contract_subject}"
            )));
        }
        let source_ref = repo.relative(&path);
        if source_ref.starts_with('/')
            || source_ref.contains('\\')
            || source_ref.contains(':')
            || source_ref
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(RepoError::Message(format!(
                "Gate Binding candidate source {source_ref:?} is not a portable repository-relative path"
            )));
        }
        let digest = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::GateBinding, &binding)
                .map_err(|error| RepoError::Message(error.to_string()))?
        );
        if !seen_ids.insert(binding.id.clone()) {
            return Err(RepoError::Message(format!(
                "Gate Binding id {:?} appears more than once for {alias}",
                binding.id
            )));
        }
        if !seen_sources.insert(source_ref.clone()) {
            return Err(RepoError::Message(format!(
                "Gate Binding source {source_ref:?} appears more than once for {alias}"
            )));
        }
        candidates.push(AuthorizedGateBinding {
            id: binding.id,
            gate,
            digest,
            source_ref,
        });
    }
    candidates.sort();
    let covered: BTreeSet<&str> = candidates
        .iter()
        .map(|binding| binding.gate.as_str())
        .collect();
    let missing: Vec<&str> = required_gates
        .iter()
        .map(String::as_str)
        .filter(|gate| !covered.contains(gate))
        .collect();
    if !missing.is_empty() {
        return Err(RepoError::Message(format!(
            "{alias} has no Gate Binding candidate for required gate(s): {}",
            missing.join(", ")
        )));
    }
    Ok(candidates)
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
    /// Human response did not authorize an exact candidate Binding set for the
    /// gates cited by this Contract Revision.
    InvalidGateBindings {
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
            Self::InvalidGateBindings { detail } => f.write_str(detail),
        }
    }
}

fn canonical_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_binding_identity(binding: &AuthorizedGateBinding) -> bool {
    !binding.id.is_empty()
        && binding.id.trim() == binding.id
        && GateRef::parse_uri(&format!("gate://{}", binding.gate)).is_ok()
        && canonical_sha256(&binding.digest)
        && !binding.source_ref.is_empty()
        && !binding.source_ref.starts_with('/')
        && !binding.source_ref.contains('\\')
        && !binding.source_ref.contains(':')
        && binding
            .source_ref
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn validate_gate_binding_selection(
    selected: &[AuthorizedGateBinding],
    candidates: &[AuthorizedGateBinding],
    required_gates: &BTreeSet<String>,
) -> Result<(), Refusal> {
    if selected.len() > MAX_AUTHORIZATION_GATE_BINDINGS {
        return Err(Refusal::InvalidGateBindings {
            detail: format!(
                "authorization selects {} Gate Bindings; limit is {MAX_AUTHORIZATION_GATE_BINDINGS}",
                selected.len()
            ),
        });
    }
    if selected
        .iter()
        .any(|binding| !canonical_binding_identity(binding))
        || selected.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(Refusal::InvalidGateBindings {
            detail:
                "authorized Gate Binding inventory is not canonical, strictly sorted, and unique"
                    .to_owned(),
        });
    }
    let candidate_set: BTreeSet<&AuthorizedGateBinding> = candidates.iter().collect();
    if let Some(binding) = selected
        .iter()
        .find(|binding| !candidate_set.contains(binding))
    {
        return Err(Refusal::InvalidGateBindings {
            detail: format!(
                "Gate Binding {} at {} was not one of the exact candidates presented to the authorizer",
                binding.id, binding.digest
            ),
        });
    }
    let mut selected_by_gate = BTreeMap::<&str, usize>::new();
    for binding in selected {
        *selected_by_gate.entry(binding.gate.as_str()).or_default() += 1;
    }
    if let Some(gate) = selected_by_gate
        .keys()
        .find(|gate| !required_gates.contains(**gate))
    {
        return Err(Refusal::InvalidGateBindings {
            detail: format!("authorization selects a Binding for uncited gate {gate}"),
        });
    }
    let invalid: Vec<String> = required_gates
        .iter()
        .filter_map(|gate| match selected_by_gate.get(gate.as_str()).copied() {
            Some(1) => None,
            Some(count) => Some(format!("{gate} ({count} selected)")),
            None => Some(format!("{gate} (missing)")),
        })
        .collect();
    if !invalid.is_empty() {
        return Err(Refusal::InvalidGateBindings {
            detail: format!(
                "authorization must select exactly one Gate Binding for each cited gate: {}",
                invalid.join(", ")
            ),
        });
    }
    Ok(())
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
    candidate_gate_bindings: &[AuthorizedGateBinding],
    required_gates: &BTreeSet<String>,
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
    let selected =
        response
            .gate_bindings
            .as_deref()
            .ok_or_else(|| Refusal::InvalidGateBindings {
                detail: "authorization-response/v2 omits required gate_bindings inventory"
                    .to_owned(),
            })?;
    validate_gate_binding_selection(selected, candidate_gate_bindings, required_gates)
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
    let required_gates = required_gate_keys(basis)?;
    let candidate_gate_bindings =
        load_candidate_gate_bindings(repo, alias, &current_digest, &required_gates)?;

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

    if let Err(refusal) = validate_response(
        &response,
        alias,
        &current_digest,
        &proposer,
        &register,
        &candidate_gate_bindings,
        &required_gates,
    ) {
        let rule = match refusal {
            Refusal::UnknownSchema { .. } => "authorize.response-schema",
            Refusal::WrongWarrant { .. } => "authorize.response-warrant",
            Refusal::StaleDigest { .. } => "authorize.stale-digest",
            Refusal::UnknownActor { .. } => "authorize.unknown-actor",
            Refusal::NotPermitted { .. } => "authorize.not-permitted",
            Refusal::InvalidGateBindings { .. } => "authorize.gate-bindings",
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

    let revision = ContractRevision::draft(current_digest.clone(), ir.contract_coverage.clone())
        .propose(proposer)
        .and_then(|proposed| proposed.authorize(authorization))
        .map_err(|e| RepoError::Message(format!("{alias}: {e}")))?;

    let record = AuthorizationRecord {
        schema: AUTHORIZATION_SCHEMA.to_owned(),
        warrant: alias.to_owned(),
        revision,
        gate_bindings: response.gate_bindings.clone(),
    };
    let authorization_path = dir.join("authorization.toml");
    let authorization_created = write_immutable_toml(&authorization_path, &record)?;
    report.push(Diagnostic::pass(
        if authorization_created {
            "authorize.recorded"
        } else {
            "authorize.idempotent"
        },
        format!(
            "{alias}: contract {current_digest} authorized by {} acting as {} → {}",
            response.authorizer, response.acting_role, authorization_path
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

/// Publish an immutable authorization. A byte-identical replay is idempotent;
/// a different response for the same revision must become a new revision, not
/// overwrite the authorization someone already relied on.
fn write_immutable_toml<T: Serialize>(
    path: &camino::Utf8Path,
    value: &T,
) -> Result<bool, RepoError> {
    use std::io::Write;

    let rendered = toml::to_string_pretty(value)
        .map_err(|error| RepoError::Message(error.to_string()))?
        .into_bytes();
    let compare_existing = || -> Result<bool, RepoError> {
        let existing = fs::read(path).map_err(|source| RepoError::Io {
            context: format!("could not read existing immutable authorization {path}"),
            source,
        })?;
        if existing == rendered {
            Ok(false)
        } else {
            Err(RepoError::Message(format!(
                "refusing to replace immutable authorization {path}; amend to a new Contract Revision"
            )))
        }
    };
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            if let Err(source) = file.write_all(&rendered).and_then(|()| file.sync_all()) {
                drop(file);
                let _ = fs::remove_file(path);
                return Err(RepoError::Io {
                    context: format!("could not publish complete immutable authorization {path}"),
                    source,
                });
            }
            Ok(true)
        }
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => compare_existing(),
        Err(source) => Err(RepoError::Io {
            context: format!("could not exclusively create authorization {path}"),
            source,
        }),
    }
}

/// Whether a persisted authorization covers the contract as it stands now.
///
/// §56.1 requirement 1 in one function. Both halves matter: an authorization in
/// any state other than `Authorized` is a proposal, and one whose digest has
/// moved covers a revision that no longer exists.
#[must_use]
pub fn authorizes_current_contract(record: &AuthorizationRecord, current_digest: &str) -> bool {
    let schema_is_admissible = match record.schema.as_str() {
        AUTHORIZATION_SCHEMA => record.gate_bindings.is_some(),
        LEGACY_AUTHORIZATION_SCHEMA => true,
        _ => false,
    };
    schema_is_admissible
        && record.revision.state == RevisionState::Authorized
        && record.revision.contract_digest == current_digest
        && record.revision.authorization.is_some()
}

/// Exact Gate Binding commitments usable by a resolver or recorded executor.
///
/// v1 authorizations remain readable proof of historical contract authority,
/// but cannot authorize Gate evidence because that wire carried no Binding
/// commitment. Unknown, draft, and stale records likewise return no authority.
#[must_use]
pub fn gate_bindings_for_current<'a>(
    record: &'a AuthorizationRecord,
    current_digest: &str,
) -> Option<&'a [AuthorizedGateBinding]> {
    if record.schema != AUTHORIZATION_SCHEMA || !authorizes_current_contract(record, current_digest)
    {
        return None;
    }
    record.gate_bindings.as_deref()
}

/// Refuse recorded execution unless exact Binding was selected by current human
/// authorization for named Warrant.
pub(crate) fn require_authorized_gate_binding(
    repo: &Repository,
    alias: &str,
    binding: &AuthorizedGateBinding,
    binding_subjects: &[String],
) -> Result<(), RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
        return Err(RepoError::Message(format!(
            "{alias}: cannot authorize Gate execution because its contract does not compile"
        )));
    };
    let current_digest = lower(basis, validated)
        .map_err(|error| RepoError::Message(format!("{alias}: {error}")))?
        .contract_digest()
        .map_err(|error| RepoError::Message(format!("{alias}: {error}")))?;
    let current_subject = contract_subject(&current_digest)?;
    if !binding_subjects
        .iter()
        .any(|subject| subject == &current_subject)
    {
        return Err(RepoError::Message(format!(
            "{alias}: Gate Binding {} does not bind exact current contract subject {current_subject}",
            binding.id
        )));
    }
    let record = repo.load_authorization(&dir)?.ok_or_else(|| {
        RepoError::Message(format!(
            "{alias}: recorded Gate execution requires current authorization/v2"
        ))
    })?;
    let authorized = gate_bindings_for_current(&record, &current_digest).ok_or_else(|| {
        RepoError::Message(format!(
            "{alias}: authorization is legacy, stale, malformed, or does not authorize Gate Bindings"
        ))
    })?;
    if !authorized.iter().any(|candidate| candidate == binding) {
        return Err(RepoError::Message(format!(
            "{alias}: Gate Binding {} at {} was not selected by exact current human authorization",
            binding.id, binding.digest
        )));
    }
    Ok(())
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
            gate_bindings: Some(vec![]),
            judgment: vec![],
        }
    }

    fn validate_no_gate_response(
        response: &AuthorizationResponse,
        ingesting: &str,
        current_digest: &str,
        proposer: &str,
        register: &AuthorityRegister,
    ) -> Result<(), Refusal> {
        validate_response(
            response,
            ingesting,
            current_digest,
            proposer,
            register,
            &[],
            &BTreeSet::new(),
        )
    }

    fn authorized_binding(id: &str, digest_byte: char) -> AuthorizedGateBinding {
        AuthorizedGateBinding {
            id: id.to_owned(),
            gate: "fixture.pass@1.0.0".to_owned(),
            digest: format!("sha256:{}", digest_byte.to_string().repeat(64)),
            source_ref: format!("docs/warrants/OW-WAR-0014/gate-bindings/{id}.binding.json"),
        }
    }

    #[test]
    fn a_well_formed_human_authorization_is_accepted() {
        assert!(
            validate_no_gate_response(
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
    fn exact_presented_gate_binding_selection_is_accepted() {
        let candidate = authorized_binding("GB-001", 'a');
        let mut signed = response("brian", "sha256:abc");
        signed.gate_bindings = Some(vec![candidate.clone()]);
        assert!(
            validate_response(
                &signed,
                "OW-WAR-0014",
                "sha256:abc",
                "claude",
                &register(),
                std::slice::from_ref(&candidate),
                &BTreeSet::from(["fixture.pass@1.0.0".to_owned()]),
            )
            .is_ok()
        );
    }

    #[test]
    fn v2_response_must_carry_explicit_gate_binding_inventory_even_when_empty() {
        let mut signed = response("brian", "sha256:abc");
        signed.gate_bindings = None;
        let refusal =
            validate_no_gate_response(&signed, "OW-WAR-0014", "sha256:abc", "claude", &register())
                .expect_err("omitted v2 inventory must differ from explicit empty inventory");
        assert!(matches!(refusal, Refusal::InvalidGateBindings { .. }));
    }

    #[test]
    fn unknown_v2_response_field_fails_closed() {
        let rendered =
            toml::to_string_pretty(&response("brian", "sha256:abc")).expect("render response");
        let planted = format!("{rendered}\nunknown_binding_authority = true\n");
        assert!(toml::from_str::<AuthorizationResponse>(&planted).is_err());
    }

    #[test]
    fn actor_selected_unpresented_gate_binding_is_refused() {
        let candidate = authorized_binding("GB-001", 'a');
        let mut signed = response("brian", "sha256:abc");
        signed.gate_bindings = Some(vec![authorized_binding("GB-ATTACKER", 'b')]);
        let refusal = validate_response(
            &signed,
            "OW-WAR-0014",
            "sha256:abc",
            "claude",
            &register(),
            &[candidate],
            &BTreeSet::from(["fixture.pass@1.0.0".to_owned()]),
        )
        .expect_err("a response may not introduce a Binding the request did not present");
        assert!(matches!(refusal, Refusal::InvalidGateBindings { .. }));
    }

    #[test]
    fn missing_or_ambiguous_required_gate_binding_is_refused() {
        let first = authorized_binding("GB-001", 'a');
        let second = authorized_binding("GB-002", 'b');
        let required = BTreeSet::from(["fixture.pass@1.0.0".to_owned()]);

        let missing = validate_response(
            &response("brian", "sha256:abc"),
            "OW-WAR-0014",
            "sha256:abc",
            "claude",
            &register(),
            &[first.clone(), second.clone()],
            &required,
        )
        .expect_err("a required gate without an authorized Binding must fail closed");
        assert!(matches!(missing, Refusal::InvalidGateBindings { .. }));

        let mut signed = response("brian", "sha256:abc");
        signed.gate_bindings = Some(vec![first.clone(), second.clone()]);
        let ambiguous = validate_response(
            &signed,
            "OW-WAR-0014",
            "sha256:abc",
            "claude",
            &register(),
            &[first, second],
            &required,
        )
        .expect_err("two Bindings for one required gate must fail closed");
        assert!(matches!(ambiguous, Refusal::InvalidGateBindings { .. }));
    }

    #[test]
    fn an_agent_cannot_authorize_even_holding_the_role() {
        let refusal = validate_no_gate_response(
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
        let refusal = validate_no_gate_response(
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
        let refusal = validate_no_gate_response(
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
        let refusal = validate_no_gate_response(
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
        let refusal =
            validate_no_gate_response(&r, "OW-WAR-0014", "sha256:abc", "claude", &register())
                .expect_err("a mismatched warrant must be refused");
        assert!(matches!(refusal, Refusal::WrongWarrant { .. }));
    }

    #[test]
    fn an_unknown_schema_is_refused() {
        let mut r = response("brian", "sha256:abc");
        r.schema = "oh.war/something-else/v1".to_owned();
        let refusal =
            validate_no_gate_response(&r, "OW-WAR-0014", "sha256:abc", "claude", &register())
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
        copy_tree(&repo_root.join("docs/gates"), &root.join("docs/gates"));

        let repo = Repository::open(root.clone()).expect("scratch repository opens");
        let warrant_dir = root.join("docs/warrants/OW-WAR-0014");
        let loaded = repo
            .load_warrant(&warrant_dir)
            .expect("load copied Warrant");
        let basis = loaded.basis.as_ref().expect("copied compilation basis");
        let validated = loaded.validated.as_ref().expect("copied manifest");
        let digest = lower(basis, validated)
            .expect("lower copied Warrant")
            .contract_digest()
            .expect("digest copied Warrant");
        let subject = contract_subject(&digest).expect("canonical contract subject");
        let required = required_gate_keys(basis).expect("required Gate keys");
        let mut gate_report = Report::default();
        let registry = crate::check::load_gate_registry(&repo, &mut gate_report);
        assert!(gate_report.is_ready(), "copied Gate registry must load");
        let binding_dir = warrant_dir.join("gate-bindings");
        fs::create_dir_all(&binding_dir).expect("create candidate Binding directory");
        for (index, gate) in required.iter().enumerate() {
            let definition = registry.get(gate).expect("required Gate Definition");
            for variant in ['A', 'B'] {
                let binding = GateBinding {
                    id: format!("GB-AUTH-{}-{variant}", index + 1),
                    gate: openwarrant_core::gate::GateRef {
                        id: definition.gate_id.clone(),
                        version: definition.version.clone(),
                        digest: definition.digest.clone(),
                    },
                    subjects: vec![subject.clone()],
                    fixtures: vec![],
                    parameters: BTreeMap::new(),
                    pass_predicate: BTreeMap::new(),
                    evidence_policy: openwarrant_core::gate::EvidencePolicy {
                        producer: "gate_runner".to_owned(),
                        performer_authored_report_admissible: true,
                    },
                };
                fs::write(
                    binding_dir.join(format!("{:03}-{variant}.binding.json", index + 1)),
                    serde_json::to_vec_pretty(&binding).expect("serialize candidate Binding"),
                )
                .expect("write candidate Binding");
            }
        }
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
        let select_variant = |variant: &str| {
            request
                .required_gates
                .iter()
                .map(|gate| {
                    request
                        .candidate_gate_bindings
                        .iter()
                        .find(|binding| binding.gate == *gate && binding.id.ends_with(variant))
                        .expect("candidate variant for required gate")
                        .clone()
                })
                .collect::<Vec<_>>()
        };
        let mut good = response("brian", &request.contract_digest);
        good.gate_bindings = Some(select_variant("-A"));
        let path = write_response(&repo, &good);
        let report = ingest(&repo, "OW-WAR-0014", &path).expect("ingest runs");
        assert!(
            report.is_ready(),
            "a human authorizer signing the current digest must be accepted: {report:?}"
        );
        assert!(record_path.exists(), "the record must be written");

        let replay = ingest(&repo, "OW-WAR-0014", &path).expect("identical replay is admissible");
        assert!(
            replay
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule == "authorize.idempotent")
        );

        let written = repo
            .load_authorization(&repo.root.join("docs/warrants/OW-WAR-0014"))
            .expect("record loads")
            .expect("record is present");
        assert!(
            authorizes_current_contract(&written, &request.contract_digest),
            "the round trip must satisfy §56.1 requirement 1"
        );

        let immutable_bytes = fs::read(&record_path).expect("read immutable authorization");
        let mut replacement = response("brian", &request.contract_digest);
        replacement.gate_bindings = Some(select_variant("-B"));
        let replacement_path = write_response(&repo, &replacement);
        let error = ingest(&repo, "OW-WAR-0014", &replacement_path)
            .expect_err("a different Binding selection cannot replace one authorized revision");
        assert!(
            error
                .to_string()
                .contains("refusing to replace immutable authorization")
        );
        assert_eq!(
            fs::read(&record_path).expect("authorization survives replacement attempt"),
            immutable_bytes
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
            gate_bindings: Some(vec![]),
        };
        assert!(
            !authorizes_current_contract(&record, "sha256:abc"),
            "a draft revision is not an authorization, however current its digest"
        );
    }

    #[test]
    fn v1_authorization_remains_historical_contract_authority_but_authorizes_no_gate_binding() {
        let record = AuthorizationRecord {
            schema: LEGACY_AUTHORIZATION_SCHEMA.to_owned(),
            warrant: "OW-WAR-0014".to_owned(),
            revision: ContractRevision::draft(
                "sha256:abc".to_owned(),
                openwarrant_core::ContractCoverage::new([]),
            )
            .propose("claude")
            .expect("propose legacy fixture")
            .authorize(Authorization {
                authorizer: "brian".to_owned(),
                actor_kind: ActorKind::Human,
                acting_role: ActorRole::Authorizer.to_string(),
                meaning: "Historical v1 authorization.".to_owned(),
                effective_time: "2026-08-25T00:00:00Z".to_owned(),
                policy_basis: None,
                independence: Independence::SeparateRole,
            })
            .expect("authorize legacy fixture"),
            gate_bindings: None,
        };
        assert!(authorizes_current_contract(&record, "sha256:abc"));
        assert!(gate_bindings_for_current(&record, "sha256:abc").is_none());
    }
}
