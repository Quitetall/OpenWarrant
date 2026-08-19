// SPDX-License-Identifier: AGPL-3.0-or-later
//! Contract revisions and their immutability (SAS §28, §29; RQ-030, RQ-031,
//! RQ-033, RQ-034).
//!
//! Law 5: authorized contract revisions are immutable. Law 6: progress cannot
//! amend the contract. §28.7: "An authorized contract is never patched."
//!
//! Those three sentences are enforced structurally here rather than by
//! convention. [`ContractRevision`] exposes no way to mutate an authorized
//! revision — amending produces a NEW revision with the old one as predecessor,
//! and there is no method that takes `&mut self` on an authorized revision at
//! all.
//!
//! # Coverage is declared (OW-ADR-0004)
//!
//! §28.5 lists seventeen elements a contract digest SHALL cover. Most do not
//! exist as typed fields yet. Rather than compute a digest that sounds like
//! §28.5's and is not, [`ContractCoverage`] records which elements a given
//! digest actually covered, and it is part of the digest preimage — so a
//! four-element digest and a seventeen-element digest are distinguishable by
//! inspection and can never be confused.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ContractError {
    #[error(
        "an authorized contract revision is immutable (Law 5, §28.7). Amend by \
         creating a new revision; an authorized contract is never patched"
    )]
    Immutable,
    #[error(
        "an agent may not authorize a proposal it produced (§27.2). \
         Authorizer {authorizer:?} is an agent and is also the proposer"
    )]
    AgentSelfAuthorization { authorizer: String },
    #[error(
        "cannot authorize a revision in state {state}; only a proposed revision may be authorized (§28.4)"
    )]
    NotProposed { state: RevisionState },
    #[error("cannot propose a revision in state {state}; only a draft may be proposed (§28.3)")]
    NotDraft { state: RevisionState },
    #[error("an authorization must record the acting role (§27.4)")]
    MissingActingRole,
    #[error("an authorization must record its meaning (§28.4)")]
    MissingMeaning,
    #[error("progress cannot amend the contract (Law 6, RQ-031): {detail}")]
    ProgressAmendment { detail: String },
}

/// Where a revision sits in §28's lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RevisionState {
    /// §28.2 — atoms may still change.
    Draft,
    /// §28.3 — an immutable proposed revision.
    Proposed,
    /// §28.4 — an immutable authorized revision.
    Authorized,
}

impl fmt::Display for RevisionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Draft => "draft",
            Self::Proposed => "proposed",
            Self::Authorized => "authorized",
        })
    }
}

/// What kind of actor is acting (§27).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorKind {
    Human,
    Agent,
    /// §27.3 — a separately identified policy service.
    PolicyService,
}

impl fmt::Display for ActorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Human => "human",
            Self::Agent => "agent",
            Self::PolicyService => "policy_service",
        })
    }
}

/// Independence of an authorization from the work it authorizes (§27.4, §46).
///
/// `None` is a recorded value, never an omission. §27.4: "Role separation by one
/// person is not organizational independence. Human views SHALL not claim
/// four-eyes review when none occurred."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Independence {
    /// The authorizer also produced the work. Recorded, not hidden.
    None,
    /// A distinct identity, but the same person or organization.
    SeparateRole,
    /// Organizationally independent.
    Organizational,
}

impl Independence {
    /// Whether this level may be described as four-eyes review (§27.4).
    #[must_use]
    pub const fn is_four_eyes(self) -> bool {
        match self {
            // §27.4 is explicit: role separation by one person is NOT
            // organizational independence, so SeparateRole does not qualify.
            Self::None | Self::SeparateRole => false,
            Self::Organizational => true,
        }
    }
}

impl fmt::Display for Independence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::SeparateRole => "separate_role",
            Self::Organizational => "organizational",
        })
    }
}

/// The §28.5 elements a contract digest covers.
///
/// Recorded in the digest preimage so a partial digest is distinguishable from a
/// complete one (OW-ADR-0004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractElement {
    Intent,
    Scope,
    BasisRequirements,
    Assumptions,
    Constraints,
    AdrReferences,
    Deliverables,
    Milestones,
    Stages,
    Capabilities,
    Autonomy,
    Resources,
    Gates,
    Obligations,
    Rollback,
    AmendmentPolicy,
    AssuranceRequirements,
}

impl ContractElement {
    /// All seventeen elements §28.5 requires.
    pub const ALL: [Self; 17] = [
        Self::Intent,
        Self::Scope,
        Self::BasisRequirements,
        Self::Assumptions,
        Self::Constraints,
        Self::AdrReferences,
        Self::Deliverables,
        Self::Milestones,
        Self::Stages,
        Self::Capabilities,
        Self::Autonomy,
        Self::Resources,
        Self::Gates,
        Self::Obligations,
        Self::Rollback,
        Self::AmendmentPolicy,
        Self::AssuranceRequirements,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Intent => "intent",
            Self::Scope => "scope",
            Self::BasisRequirements => "basis_requirements",
            Self::Assumptions => "assumptions",
            Self::Constraints => "constraints",
            Self::AdrReferences => "adr_references",
            Self::Deliverables => "deliverables",
            Self::Milestones => "milestones",
            Self::Stages => "stages",
            Self::Capabilities => "capabilities",
            Self::Autonomy => "autonomy",
            Self::Resources => "resources",
            Self::Gates => "gates",
            Self::Obligations => "obligations",
            Self::Rollback => "rollback",
            Self::AmendmentPolicy => "amendment_policy",
            Self::AssuranceRequirements => "assurance_requirements",
        }
    }
}

impl fmt::Display for ContractElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Which §28.5 elements a digest actually covered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractCoverage {
    covered: BTreeSet<ContractElement>,
}

impl ContractCoverage {
    #[must_use]
    pub fn new(covered: impl IntoIterator<Item = ContractElement>) -> Self {
        Self {
            covered: covered.into_iter().collect(),
        }
    }

    /// The elements §28.5 requires that this digest does NOT cover.
    #[must_use]
    pub fn missing(&self) -> Vec<ContractElement> {
        ContractElement::ALL
            .into_iter()
            .filter(|e| !self.covered.contains(e))
            .collect()
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.missing().is_empty()
    }

    /// No `#[must_use]`: `Iterator` carries one, and a redundant attribute is
    /// itself a clippy error. Same shape as `Frontmatter::keys`.
    pub fn covered(&self) -> impl Iterator<Item = &ContractElement> {
        self.covered.iter()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.covered.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.covered.is_empty()
    }
}

/// An authorization record (§28.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authorization {
    pub authorizer: String,
    pub actor_kind: ActorKind,
    /// §27.4 — the role ACTUALLY exercised, not the roles held.
    pub acting_role: String,
    /// §28.4 — what authorizing meant here.
    pub meaning: String,
    /// §28.4. Recorded as supplied; server time is authoritative once KF exists
    /// (§67.2), so a locally stamped time is provisional.
    pub effective_time: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_basis: Option<String>,
    /// Recorded, never omitted (OW-ADR-0004).
    pub independence: Independence,
}

impl Authorization {
    fn validate(&self, proposer: Option<&str>) -> Result<(), ContractError> {
        if self.acting_role.trim().is_empty() {
            return Err(ContractError::MissingActingRole);
        }
        if self.meaning.trim().is_empty() {
            return Err(ContractError::MissingMeaning);
        }
        // §27.2: an AGENT may not authorize its own proposed WAR. A human may
        // (§27.4), provided the acting role is recorded — which the checks
        // above already require.
        if self.actor_kind == ActorKind::Agent && proposer == Some(self.authorizer.as_str()) {
            return Err(ContractError::AgentSelfAuthorization {
                authorizer: self.authorizer.clone(),
            });
        }
        Ok(())
    }
}

/// One contract revision (§28).
///
/// Immutability is structural: there is no method that mutates an authorized
/// revision. `propose` and `authorize` consume `self` and return a new value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractRevision {
    /// §28.1 — the Warrant identity persists across revisions; this numbers the
    /// revision within it.
    pub revision: u32,
    pub state: RevisionState,
    /// The digest of the contract content, over `coverage`.
    pub contract_digest: String,
    pub coverage: ContractCoverage,
    /// §28.6 — every revision identifies its predecessor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predecessor_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization: Option<Authorization>,
}

impl ContractRevision {
    /// A first draft revision (§28.2).
    #[must_use]
    pub fn draft(contract_digest: String, coverage: ContractCoverage) -> Self {
        Self {
            revision: 1,
            state: RevisionState::Draft,
            contract_digest,
            coverage,
            predecessor_digest: None,
            proposer: None,
            authorization: None,
        }
    }

    /// §28.3 — submitting creates an immutable proposed revision.
    pub fn propose(self, proposer: impl Into<String>) -> Result<Self, ContractError> {
        if self.state != RevisionState::Draft {
            return Err(ContractError::NotDraft { state: self.state });
        }
        Ok(Self {
            state: RevisionState::Proposed,
            proposer: Some(proposer.into()),
            ..self
        })
    }

    /// §28.4 — authorization creates an immutable authorized revision.
    pub fn authorize(self, authorization: Authorization) -> Result<Self, ContractError> {
        if self.state != RevisionState::Proposed {
            return Err(ContractError::NotProposed { state: self.state });
        }
        authorization.validate(self.proposer.as_deref())?;
        Ok(Self {
            state: RevisionState::Authorized,
            authorization: Some(authorization),
            ..self
        })
    }

    /// §28.7 / RQ-033 — amending produces a NEW revision, never a patch.
    ///
    /// Takes `&self` and returns a new draft whose predecessor is this
    /// revision's digest. There is deliberately no `amend_in_place`.
    pub fn amend(
        &self,
        new_digest: String,
        coverage: ContractCoverage,
    ) -> Result<Self, ContractError> {
        if self.state != RevisionState::Authorized {
            // Amending something not yet authorized is just editing the draft
            // (§28.2), which needs no revision at all.
            return Err(ContractError::Immutable);
        }
        Ok(Self {
            revision: self.revision + 1,
            state: RevisionState::Draft,
            contract_digest: new_digest,
            coverage,
            predecessor_digest: Some(self.contract_digest.clone()),
            proposer: None,
            authorization: None,
        })
    }

    /// Whether this revision is immutable (§28.3, §28.4).
    #[must_use]
    pub const fn is_immutable(&self) -> bool {
        matches!(
            self.state,
            RevisionState::Proposed | RevisionState::Authorized
        )
    }

    /// Reject an attempt to fold execution progress into the contract
    /// (Law 6, RQ-031).
    pub fn reject_progress_amendment(&self, detail: &str) -> ContractError {
        ContractError::ProgressAmendment {
            detail: detail.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coverage() -> ContractCoverage {
        // What the IR can actually cover today (OW-ADR-0004).
        ContractCoverage::new([
            ContractElement::Intent,
            ContractElement::Scope,
            ContractElement::AdrReferences,
            ContractElement::Milestones,
            ContractElement::Stages,
        ])
    }

    fn human_auth() -> Authorization {
        Authorization {
            authorizer: "brian".to_owned(),
            actor_kind: ActorKind::Human,
            acting_role: "maintainer".to_owned(),
            meaning: "authorized for execution under the alpha roadmap".to_owned(),
            effective_time: "2026-08-19T00:00:00Z".to_owned(),
            policy_basis: None,
            independence: Independence::None,
        }
    }

    #[test]
    fn the_draft_propose_authorize_chain_works() {
        let r = ContractRevision::draft("d1".to_owned(), coverage());
        assert_eq!(r.state, RevisionState::Draft);
        assert!(!r.is_immutable());

        let r = r.propose("brian").expect("proposable");
        assert_eq!(r.state, RevisionState::Proposed);
        assert!(r.is_immutable(), "§28.3 — a proposal is immutable");

        let r = r.authorize(human_auth()).expect("authorizable");
        assert_eq!(r.state, RevisionState::Authorized);
        assert!(r.is_immutable());
        assert_eq!(r.revision, 1);
    }

    /// §27.4 — a human may authorize work they proposed, provided the acting
    /// role is recorded and independence is not overstated.
    #[test]
    fn a_human_may_authorize_their_own_proposal() {
        let r = ContractRevision::draft("d1".to_owned(), coverage())
            .propose("brian")
            .expect("proposable")
            .authorize(human_auth())
            .expect("§27.4 permits this for a human");
        let auth = r.authorization.expect("recorded");
        assert_eq!(auth.independence, Independence::None);
        assert!(
            !auth.independence.is_four_eyes(),
            "§27.4 — this must never read as four-eyes review"
        );
    }

    /// §27.2 — an AGENT may not authorize its own proposed WAR.
    #[test]
    fn an_agent_may_not_authorize_its_own_proposal() {
        let mut auth = human_auth();
        auth.actor_kind = ActorKind::Agent;
        auth.authorizer = "drafter-agent".to_owned();

        let err = ContractRevision::draft("d1".to_owned(), coverage())
            .propose("drafter-agent")
            .expect("proposable")
            .authorize(auth)
            .expect_err("§27.2 forbids this");
        assert_eq!(
            err,
            ContractError::AgentSelfAuthorization {
                authorizer: "drafter-agent".to_owned()
            }
        );
    }

    /// An agent authorizing SOMEONE ELSE'S proposal is not self-authorization.
    #[test]
    fn an_agent_may_authorize_a_proposal_it_did_not_make() {
        let mut auth = human_auth();
        auth.actor_kind = ActorKind::Agent;
        auth.authorizer = "reviewer-agent".to_owned();
        ContractRevision::draft("d1".to_owned(), coverage())
            .propose("brian")
            .expect("proposable")
            .authorize(auth)
            .expect("not self-authorization");
    }

    #[test]
    fn an_authorization_must_record_its_acting_role_and_meaning() {
        let mut no_role = human_auth();
        no_role.acting_role = "  ".to_owned();
        assert_eq!(
            ContractRevision::draft("d".to_owned(), coverage())
                .propose("brian")
                .expect("ok")
                .authorize(no_role),
            Err(ContractError::MissingActingRole)
        );

        let mut no_meaning = human_auth();
        no_meaning.meaning = String::new();
        assert_eq!(
            ContractRevision::draft("d".to_owned(), coverage())
                .propose("brian")
                .expect("ok")
                .authorize(no_meaning),
            Err(ContractError::MissingMeaning)
        );
    }

    /// §28.7 / Law 5 — amending produces a new revision with ancestry.
    #[test]
    fn amendment_creates_a_new_revision_with_a_predecessor() {
        let authorized = ContractRevision::draft("d1".to_owned(), coverage())
            .propose("brian")
            .expect("ok")
            .authorize(human_auth())
            .expect("ok");

        let amended = authorized.amend("d2".to_owned(), coverage()).expect("ok");
        assert_eq!(amended.revision, 2);
        assert_eq!(amended.state, RevisionState::Draft);
        assert_eq!(amended.predecessor_digest.as_deref(), Some("d1"));
        assert!(
            amended.authorization.is_none(),
            "a new revision is not carried by the old authorization"
        );

        // RQ-034 — the prior revision is untouched.
        assert_eq!(authorized.revision, 1);
        assert_eq!(authorized.contract_digest, "d1");
        assert_eq!(authorized.state, RevisionState::Authorized);
    }

    #[test]
    fn out_of_order_transitions_are_refused() {
        let draft = ContractRevision::draft("d".to_owned(), coverage());
        assert!(matches!(
            draft.clone().authorize(human_auth()),
            Err(ContractError::NotProposed { .. })
        ));

        let proposed = draft.propose("brian").expect("ok");
        assert!(matches!(
            proposed.clone().propose("brian"),
            Err(ContractError::NotDraft { .. })
        ));

        // Amending a draft is not amendment — it is editing (§28.2).
        assert_eq!(
            proposed.amend("x".to_owned(), coverage()),
            Err(ContractError::Immutable)
        );
    }

    // --- coverage (OW-ADR-0004) ---

    /// §28.5 lists seventeen elements; today's digest covers five.
    #[test]
    fn coverage_reports_what_is_missing() {
        let c = coverage();
        assert!(!c.is_complete());
        assert_eq!(c.len(), 5);
        let missing = c.missing();
        assert_eq!(missing.len(), 12);
        assert!(missing.contains(&ContractElement::Gates));
        assert!(missing.contains(&ContractElement::Obligations));
        assert!(missing.contains(&ContractElement::Deliverables));
    }

    #[test]
    fn full_coverage_is_recognised() {
        let full = ContractCoverage::new(ContractElement::ALL);
        assert!(full.is_complete());
        assert_eq!(full.missing(), vec![]);
        assert_eq!(full.len(), 17);
    }

    /// The whole point: two digests over different coverage are different
    /// objects and must not be interchangeable.
    #[test]
    fn partial_and_complete_coverage_are_distinguishable() {
        let partial = ContractRevision::draft("same-digest".to_owned(), coverage());
        let complete = ContractRevision::draft(
            "same-digest".to_owned(),
            ContractCoverage::new(ContractElement::ALL),
        );
        assert_ne!(
            partial, complete,
            "identical digest bytes over different coverage must not compare equal"
        );
    }

    /// §28.5's element list, transcribed as an external expectation.
    #[test]
    fn the_seventeen_elements_match_the_sas() {
        assert_eq!(
            ContractElement::ALL
                .iter()
                .map(|e| e.as_str())
                .collect::<Vec<_>>(),
            [
                "intent",
                "scope",
                "basis_requirements",
                "assumptions",
                "constraints",
                "adr_references",
                "deliverables",
                "milestones",
                "stages",
                "capabilities",
                "autonomy",
                "resources",
                "gates",
                "obligations",
                "rollback",
                "amendment_policy",
                "assurance_requirements",
            ]
        );
    }

    /// §27.4 — role separation by one person is NOT organizational independence.
    #[test]
    fn separate_role_is_not_four_eyes() {
        assert!(!Independence::None.is_four_eyes());
        assert!(
            !Independence::SeparateRole.is_four_eyes(),
            "§27.4 says this explicitly"
        );
        assert!(Independence::Organizational.is_four_eyes());
    }
}
