// SPDX-License-Identifier: Apache-2.0
//! `oh.war/draft-proposal/v2` — a Draft Proposal whose operations carry what
//! they operate on (OW-ADR-0013).
//!
//! v1's `atom_operations` is a list of bare operation NAMES. It proved the
//! closed list (§74.3) and the deny-unknown-fields boundary (§91.8 tests 53,
//! 54), and it can never be applied: `create_atom` with no role, ordinal or
//! body creates nothing. v2 keeps every v1 rule and gives each operation a
//! payload. v1 still parses and validates; only `--apply` refuses it, by name.
//!
//! The struct is still the agent's ENTIRE output surface. `deny_unknown_fields`
//! stays: a proposal carrying `authorized_by` or `enterprise_id` is refused at
//! parse, not silently dropped.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::drafting::{AtomOperation, DraftError, DurableChoice, EvidenceClaim, InterviewQuestion};

/// v2's own refusals, beside v1's (which it wraps). `drafting.rs` is a pinned
/// deliverable of resolved Warrants; v2 adds nothing to its error type.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DraftV2Error {
    #[error(
        "draft proposal declares api_version {found:?}; this is the v2 reader ({})",
        API_VERSION_V2
    )]
    WrongApiVersion { found: String },
    #[error("operation {op} needs `{field}`; without it there is nothing to apply")]
    OperationMissingField { op: String, field: &'static str },
    #[error("atom path {path:?} escapes the atoms/ directory; a path is a file name, not a route")]
    OperationPathEscapes { path: String },
    #[error("unknown relation kind {kind:?}; known: implements, roadmap, parent")]
    UnknownRelationKind { kind: String },
    #[error("two create_atom operations name {path:?}; one file is written once")]
    DuplicateAtomPath { path: String },
    #[error("{0}")]
    V1(#[from] DraftError),
}

pub const API_VERSION_V2: &str = "oh.war/draft-proposal/v2";

/// What the proposal proposes to be.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedIdentity {
    pub title: String,
    #[serde(default = "default_profile")]
    pub profile: String,
    #[serde(default = "default_assurance")]
    pub assurance: String,
}

fn default_profile() -> String {
    "delivery".to_owned()
}
fn default_assurance() -> String {
    "basic".to_owned()
}

/// One §74.3 operation with its payload. Fields not meaningful for an
/// operation are simply empty; `validate` says which ones an operation needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtomOperationRequest {
    pub op: AtomOperation,
    /// `intent`, `basis`, `work_order`, `milestones`, `assurance`, `adr`, …
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub ordinal: u32,
    /// Relative to the Warrant's `atoms/` directory, e.g. `10-intent.md`.
    #[serde(default)]
    pub path: String,
    /// The atom body (without frontmatter — the tool writes that).
    #[serde(default)]
    pub body: String,
    /// For revise/retire: the atom path being changed.
    #[serde(default)]
    pub target: String,
    /// For add_relation: `implements` | `roadmap` | `parent`.
    #[serde(default)]
    pub relation_kind: String,
    /// For add_relation: `sas://…`, `roadmap://…`, `war://…`.
    #[serde(default)]
    pub relation_ref: String,
    /// For propose_adr.
    #[serde(default)]
    pub adr_title: String,
}

impl AtomOperationRequest {
    /// The fields each operation cannot do without. Empty where irrelevant.
    pub fn validate(&self) -> Result<(), DraftV2Error> {
        let need = |field: &'static str, value: &str| {
            if value.trim().is_empty() {
                Err(DraftV2Error::OperationMissingField {
                    op: self.op.to_string(),
                    field,
                })
            } else {
                Ok(())
            }
        };
        match self.op {
            AtomOperation::CreateAtom => {
                need("role", &self.role)?;
                need("path", &self.path)?;
                need("body", &self.body)?;
                if self.ordinal == 0 {
                    return Err(DraftV2Error::OperationMissingField {
                        op: self.op.to_string(),
                        field: "ordinal",
                    });
                }
                if self.path.contains('/') || self.path.contains("..") {
                    return Err(DraftV2Error::OperationPathEscapes {
                        path: self.path.clone(),
                    });
                }
            }
            AtomOperation::ReviseAtom => {
                need("target", &self.target)?;
                need("body", &self.body)?;
            }
            AtomOperation::RetireAtom => need("target", &self.target)?,
            AtomOperation::AddBinding | AtomOperation::RemoveBinding => {
                need("target", &self.target)?;
            }
            AtomOperation::AddRelation => {
                need("relation_kind", &self.relation_kind)?;
                need("relation_ref", &self.relation_ref)?;
                if !["implements", "roadmap", "parent"].contains(&self.relation_kind.as_str()) {
                    return Err(DraftV2Error::UnknownRelationKind {
                        kind: self.relation_kind.clone(),
                    });
                }
            }
            AtomOperation::ProposeAdr => {
                need("adr_title", &self.adr_title)?;
                need("body", &self.body)?;
            }
        }
        Ok(())
    }
}

/// §74.2's Draft Proposal, v2.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftProposalV2 {
    pub api_version: String,
    #[serde(default)]
    pub proposed_identity: ProposedIdentity,
    #[serde(default)]
    pub operations: Vec<AtomOperationRequest>,
    #[serde(default)]
    pub proposed_relations: Vec<String>,
    #[serde(default)]
    pub evidence_claims: Vec<EvidenceClaim>,
    #[serde(default)]
    pub durable_choices: Vec<DurableChoice>,
    #[serde(default)]
    pub unresolved_questions: Vec<InterviewQuestion>,
    #[serde(default)]
    pub risk_assessment: String,
    #[serde(default)]
    pub adequacy_attacks: Vec<String>,
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

impl DraftProposalV2 {
    /// Every v1 rule, plus each operation's payload.
    pub fn validate(&self) -> Result<(), DraftV2Error> {
        if self.api_version != API_VERSION_V2 {
            return Err(DraftV2Error::WrongApiVersion {
                found: self.api_version.clone(),
            });
        }
        if self.proposed_identity.title.trim().is_empty() {
            return Err(DraftV2Error::OperationMissingField {
                op: "proposed_identity".to_owned(),
                field: "title",
            });
        }
        for claim in &self.evidence_claims {
            claim.validate()?;
        }
        for choice in &self.durable_choices {
            if choice.proposed_adr_draft.trim().is_empty() {
                return Err(DraftError::DurableChoiceBuried {
                    choice: choice.statement.clone(),
                }
                .into());
            }
        }
        for op in &self.operations {
            op.validate()?;
        }
        // Two create_atom operations on one path would write twice.
        let mut seen = BTreeSet::new();
        for op in &self.operations {
            if op.op == AtomOperation::CreateAtom && !seen.insert(op.path.as_str()) {
                return Err(DraftV2Error::DuplicateAtomPath {
                    path: op.path.clone(),
                });
            }
        }
        Ok(())
    }

    /// The `war://` references the proposal names, for resolution against the
    /// corpus (§74.4 step 3).
    #[must_use]
    pub fn cited_warrants(&self) -> Vec<&str> {
        self.proposed_relations
            .iter()
            .map(String::as_str)
            .chain(
                self.operations
                    .iter()
                    .filter(|o| o.op == AtomOperation::AddRelation)
                    .map(|o| o.relation_ref.as_str()),
            )
            .filter(|r| r.starts_with("war://"))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal() -> DraftProposalV2 {
        DraftProposalV2 {
            api_version: API_VERSION_V2.to_owned(),
            proposed_identity: ProposedIdentity {
                title: "Add a CHANGELOG".to_owned(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn v2_parses_and_validates_and_v1_is_refused_by_version() {
        let json = r##"{"api_version":"oh.war/draft-proposal/v2","proposed_identity":{"title":"t"},
            "operations":[{"op":"create_atom","role":"intent","ordinal":10,"path":"10-intent.md","body":"# Intent"}]}"##;
        let p: DraftProposalV2 = serde_json::from_str(json).unwrap();
        p.validate().unwrap();
        let v1 = DraftProposalV2 {
            api_version: "oh.war/draft-proposal/v1".to_owned(),
            ..minimal()
        };
        assert!(matches!(
            v1.validate(),
            Err(DraftV2Error::WrongApiVersion { .. })
        ));
    }

    #[test]
    fn the_output_surface_is_still_closed() {
        let json = r#"{"api_version":"oh.war/draft-proposal/v2","authorized_by":"me"}"#;
        assert!(serde_json::from_str::<DraftProposalV2>(json).is_err());
        let json = r#"{"api_version":"oh.war/draft-proposal/v2","operations":[{"op":"write_file","path":"x"}]}"#;
        assert!(
            serde_json::from_str::<DraftProposalV2>(json).is_err(),
            "write_file is not an operation"
        );
    }

    #[test]
    fn each_operation_names_what_it_cannot_do_without() {
        let mut p = minimal();
        p.operations.push(AtomOperationRequest {
            op: AtomOperation::CreateAtom,
            role: "intent".to_owned(),
            ordinal: 10,
            path: "10-intent.md".to_owned(),
            body: String::new(),
            target: String::new(),
            relation_kind: String::new(),
            relation_ref: String::new(),
            adr_title: String::new(),
        });
        assert!(matches!(
            p.validate(),
            Err(DraftV2Error::OperationMissingField { field: "body", .. })
        ));
        p.operations[0].body = "x".to_owned();
        p.operations[0].path = "../escape.md".to_owned();
        assert!(matches!(
            p.validate(),
            Err(DraftV2Error::OperationPathEscapes { .. })
        ));
        p.operations[0].path = "10-intent.md".to_owned();
        p.operations.push(p.operations[0].clone());
        assert!(matches!(
            p.validate(),
            Err(DraftV2Error::DuplicateAtomPath { .. })
        ));
        let mut q = minimal();
        q.operations.push(AtomOperationRequest {
            op: AtomOperation::AddRelation,
            relation_kind: "friend".to_owned(),
            relation_ref: "war://x".to_owned(),
            role: String::new(),
            ordinal: 0,
            path: String::new(),
            body: String::new(),
            target: String::new(),
            adr_title: String::new(),
        });
        assert!(matches!(
            q.validate(),
            Err(DraftV2Error::UnknownRelationKind { .. })
        ));
        assert_eq!(q.cited_warrants(), vec!["war://x"]);
    }
}
