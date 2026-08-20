// SPDX-License-Identifier: AGPL-3.0-or-later
//! Runtime and platform seams: Katana (§48), BLUT (§49), Knowledge Fabric
//! (§67), and source adapters (§82). RQ-044, RQ-062, RQ-063, RQ-061, RQ-076.
//!
//! # One idea, four times
//!
//! Each of these sections says the same thing about a different neighbour:
//! **the other system owns its own facts, and OpenWarrant records references to
//! them rather than reinterpreting them.**
//!
//! - §48.2: OpenWarrant *"records the PromptIR digest from the Katana receipt. It
//!   does not compile or reinterpret Katana's runtime conversation."*
//! - §48.5: *"Katana taint and influence labels remain Katana-owned runtime
//!   facts."*
//! - §49.3: *"BLUT execution lineage remains authoritative in BLUT. The WAR
//!   stores exact receipt references and relevant projections."*
//! - §82.3: parity is *measured* before cutover, not assumed.
//!
//! So the types here hold digests and references, not reconstructions. A
//! [`KatanaReceipt`] has a `prompt_ir_digest` and no PromptIR; a
//! [`BlutLineageReceipt`] has a lineage reference and no lineage. That is the
//! design, not an omission — a second copy of someone else's authoritative fact
//! is a second answer to a question that should have one.
//!
//! # Where the boundary actually gets enforced
//!
//! §48.3 splits authorization from realization: Knowledge Fabric and the WAR
//! contract authorize what may be done; Katana realizes and enforces the
//! low-level capability set. [`realized_within_authorization`] checks the one
//! direction that matters — a runtime that realized a capability nobody
//! authorized is a containment failure, and it is invisible unless someone
//! compares the two lists.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::vocab::vocabulary;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SeamError {
    #[error("unknown {vocabulary} {found:?}; SAS defines {known}")]
    UnknownTerm {
        vocabulary: &'static str,
        found: String,
        known: String,
    },
    #[error("Katana receipt {id:?} omits {field}, which §48.4 requires at minimum")]
    KatanaReceiptIncomplete { id: String, field: &'static str },
    #[error(
        "Katana receipt {id:?} answers dispatch {returned:?} but was recorded \
         against {expected:?}. A receipt for a different dispatch is evidence about \
         different work"
    )]
    ReceiptDispatchMismatch {
        id: String,
        expected: String,
        returned: String,
    },
    #[error(
        "the runtime realized capabilities nobody authorized: {extra}. §48.3 — the \
         contract and Knowledge Fabric authorize what may be done, and the runtime \
         realizes it; realizing MORE is a containment failure, not a detail"
    )]
    UnauthorizedCapabilityRealized { extra: String },
    #[error(
        "BLUT adapter rejected stage {stage:?}: {reason}. §49.2 requires the adapter \
         to reject incompatible kinds and unsupported conditions rather than \
         lowering them into something that runs but means something else"
    )]
    LoweringRejected { stage: String, reason: String },
    #[error(
        "BLUT adapter resolved stage {stage:?} against no pinned registry. §49.2: \
         stage names are resolved against a PINNED registry, or the same name means \
         different things on different days"
    )]
    UnpinnedRegistry { stage: String },
    #[error("controlled action {request_id:?} omits {field}, which §67.1 requires")]
    ActionEnvelopeIncomplete {
        request_id: String,
        field: &'static str,
    },
    #[error(
        "controlled action {request_id:?} sets recorded_at. §67.2: Knowledge Fabric \
         assigns recorded_at — a client-supplied value is a client asserting when \
         the server saw something"
    )]
    ClientAssignedRecordedAt { request_id: String },
    #[error(
        "controlled action {request_id:?} cites version {expected} but the object is \
         at {actual}. §67.3: drift FAILS rather than overwrites"
    )]
    VersionDrift {
        request_id: String,
        expected: u64,
        actual: u64,
    },
    #[error(
        "idempotency key {key:?} was reused with a different payload. §67.4: \
         equivalent retries replay the first committed result, and conflicting \
         reuse is rejected"
    )]
    IdempotencyConflict { key: String },
    #[error(
        "adapter parity not established: {differences}. §82.3 — the compatibility \
         corpus SHALL be compiled by BOTH adapters and compared before cutover"
    )]
    ParityNotEstablished { differences: String },
    #[error(
        "cutover attempted while parity is unmeasured or failing. §82.4 permits \
         cutover only once Liminal is QUALIFIED"
    )]
    CutoverWithoutQualification,
}

vocabulary!(
    /// §67's controlled-action vocabulary — contract group.
    ContractAction, "contract action", SeamError, {
        CreateWarrantDraft => "create_warrant_draft",
        ReviseWarrantDraft => "revise_warrant_draft",
        SubmitWarrant => "submit_warrant",
        AuthorizeWarrantContract => "authorize_warrant_contract",
        WithdrawWarrantProposal => "withdraw_warrant_proposal",
        ProposeWarrantAmendment => "propose_warrant_amendment",
        AuthorizeWarrantAmendment => "authorize_warrant_amendment",
        RejectWarrantAmendment => "reject_warrant_amendment",
    }
);

vocabulary!(
    /// §67's execution group.
    ExecutionAction, "execution action", SeamError, {
        RecordWarrantPreflight => "record_warrant_preflight",
        AuthorizeWarrantDispatch => "authorize_warrant_dispatch",
        AttachWarrantRuntimeReceipt => "attach_warrant_runtime_receipt",
        RegisterWarrantSubmission => "register_warrant_submission",
        OpenWarrantBlocker => "open_warrant_blocker",
        ResolveWarrantBlocker => "resolve_warrant_blocker",
        PauseWarrant => "pause_warrant",
        ResumeWarrant => "resume_warrant",
        ProposeWarrantDeviation => "propose_warrant_deviation",
        ApproveWarrantDeviation => "approve_warrant_deviation",
        RejectWarrantDeviation => "reject_warrant_deviation",
        RecordWarrantDiscoveredGap => "record_warrant_discovered_gap",
    }
);

vocabulary!(
    /// §67's evidence group.
    EvidenceAction, "evidence action", SeamError, {
        RegisterWarrantArtifact => "register_warrant_artifact",
        RegisterWarrantEvidence => "register_warrant_evidence",
        AttachWarrantGateRun => "attach_warrant_gate_run",
        RecordWarrantInference => "record_warrant_inference",
        RecordWarrantJudgment => "record_warrant_judgment",
        RequestWarrantResolution => "request_warrant_resolution",
    }
);

vocabulary!(
    /// §67's terminal and administrative group.
    TerminalAction, "terminal action", SeamError, {
        ResolveWarrant => "resolve_warrant",
        DisputeWarrantResolution => "dispute_warrant_resolution",
        ResolveWarrantDispute => "resolve_warrant_dispute",
        AnnulWarrantResolution => "annul_warrant_resolution",
        SupersedeWarrant => "supersede_warrant",
        DeprecateWarrant => "deprecate_warrant",
    }
);

/// How many controlled actions §67 defines in total.
pub const CONTROLLED_ACTION_COUNT: usize = ContractAction::ALL.len()
    + ExecutionAction::ALL.len()
    + EvidenceAction::ALL.len()
    + TerminalAction::ALL.len();

/// Every controlled action name, in §67's group order.
#[must_use]
pub fn all_controlled_actions() -> Vec<&'static str> {
    let mut out = Vec::new();
    out.extend(ContractAction::ALL.iter().map(|a| a.as_str()));
    out.extend(ExecutionAction::ALL.iter().map(|a| a.as_str()));
    out.extend(EvidenceAction::ALL.iter().map(|a| a.as_str()));
    out.extend(TerminalAction::ALL.iter().map(|a| a.as_str()));
    out
}

/// §67.1's controlled-action envelope.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ActionEnvelope {
    pub action_type: String,
    pub actor_id: String,
    pub acting_role_id: String,
    pub organization_id: String,
    #[serde(default)]
    pub target_ids: Vec<String>,
    #[serde(default)]
    pub payload: String,
    pub reason: String,
    pub idempotency_key: String,
    pub request_id: String,
    /// §67.3 — the version the client READ. Drift fails rather than overwrites.
    pub expected_version: u64,
    pub effective_at: String,
    pub max_classification: String,
    /// §67.2 — Knowledge Fabric assigns this. A client setting it is asserting
    /// when the server saw something, which is not the client's to say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recorded_at: Option<String>,
}

impl ActionEnvelope {
    pub fn validate(&self) -> Result<(), SeamError> {
        for (field, value) in [
            ("action_type", &self.action_type),
            ("actor_id", &self.actor_id),
            ("acting_role_id", &self.acting_role_id),
            ("organization_id", &self.organization_id),
            ("reason", &self.reason),
            ("idempotency_key", &self.idempotency_key),
            ("request_id", &self.request_id),
            ("effective_at", &self.effective_at),
            ("max_classification", &self.max_classification),
        ] {
            if value.trim().is_empty() {
                return Err(SeamError::ActionEnvelopeIncomplete {
                    request_id: self.request_id.clone(),
                    field,
                });
            }
        }
        if self.recorded_at.is_some() {
            return Err(SeamError::ClientAssignedRecordedAt {
                request_id: self.request_id.clone(),
            });
        }
        if !all_controlled_actions().contains(&self.action_type.as_str()) {
            return Err(SeamError::ActionEnvelopeIncomplete {
                request_id: self.request_id.clone(),
                field: "action_type (not in §67's vocabulary)",
            });
        }
        Ok(())
    }

    /// §67.3 — optimistic concurrency. Drift FAILS.
    pub fn check_version(&self, actual: u64) -> Result<(), SeamError> {
        if self.expected_version == actual {
            Ok(())
        } else {
            Err(SeamError::VersionDrift {
                request_id: self.request_id.clone(),
                expected: self.expected_version,
                actual,
            })
        }
    }

    /// §67.4 — an equivalent retry replays; a conflicting reuse is rejected.
    ///
    /// Equivalence is by payload, so retrying the same action is safe and
    /// reusing a key for a different action is refused. Without the payload
    /// comparison an idempotency key would suppress a genuinely different write.
    pub fn check_idempotency(
        &self,
        prior_payload_for_key: Option<&str>,
    ) -> Result<Replay, SeamError> {
        match prior_payload_for_key {
            None => Ok(Replay::FirstUse),
            Some(prior) if prior == self.payload => Ok(Replay::ReplayFirstResult),
            Some(_) => Err(SeamError::IdempotencyConflict {
                key: self.idempotency_key.clone(),
            }),
        }
    }
}

/// What §67.4 says should happen to a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Replay {
    FirstUse,
    ReplayFirstResult,
}

/// §48.4's runtime receipt, at minimum.
///
/// Holds the PromptIR *digest*, never PromptIR. §48.2 is explicit that
/// OpenWarrant does not compile or reinterpret Katana's runtime conversation.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct KatanaReceipt {
    pub session_id: String,
    pub dispatch_digest: String,
    pub prompt_ir_digest: String,
    pub provider_model_identity: String,
    pub runtime_event_log_head: String,
    #[serde(default)]
    pub realized_capabilities: Vec<String>,
    pub confinement: String,
    pub usage: String,
    #[serde(default)]
    pub artifact_refs: Vec<String>,
    pub terminal_runtime_status: String,
    pub receipt_digest: String,
    /// §48.5 — Katana-owned runtime facts, REFERENCED not recomputed.
    #[serde(default)]
    pub taint_label_refs: Vec<String>,
}

impl KatanaReceipt {
    /// §48.4's minimum fields, and §48.1's binding to the dispatch it answers.
    pub fn validate(&self, expected_dispatch_digest: &str) -> Result<(), SeamError> {
        for (field, value) in [
            ("session_id", &self.session_id),
            ("dispatch_digest", &self.dispatch_digest),
            ("prompt_ir_digest", &self.prompt_ir_digest),
            ("provider_model_identity", &self.provider_model_identity),
            ("runtime_event_log_head", &self.runtime_event_log_head),
            ("confinement", &self.confinement),
            ("usage", &self.usage),
            ("terminal_runtime_status", &self.terminal_runtime_status),
            ("receipt_digest", &self.receipt_digest),
        ] {
            if value.trim().is_empty() {
                return Err(SeamError::KatanaReceiptIncomplete {
                    id: self.session_id.clone(),
                    field,
                });
            }
        }
        if self.dispatch_digest != expected_dispatch_digest {
            return Err(SeamError::ReceiptDispatchMismatch {
                id: self.session_id.clone(),
                expected: expected_dispatch_digest.to_owned(),
                returned: self.dispatch_digest.clone(),
            });
        }
        Ok(())
    }
}

/// §48.3 — what was realized must be within what was authorized.
///
/// The asymmetry is the point. Realizing FEWER capabilities than authorized is
/// fine, and often correct. Realizing more is a containment failure, and it is
/// invisible unless the two lists are actually compared.
pub fn realized_within_authorization(
    authorized: &[String],
    realized: &[String],
) -> Result<(), SeamError> {
    let allowed: BTreeSet<&str> = authorized.iter().map(String::as_str).collect();
    let extra: Vec<&str> = realized
        .iter()
        .map(String::as_str)
        .filter(|c| !allowed.contains(c))
        .collect();
    if extra.is_empty() {
        Ok(())
    } else {
        Err(SeamError::UnauthorizedCapabilityRealized {
            extra: extra.join(", "),
        })
    }
}

/// §49.2's adapter duties, as a checklist that must be discharged.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BlutLowering {
    pub stage: String,
    /// §49.2 — resolved against a PINNED registry.
    pub registry_digest: String,
    #[serde(default)]
    pub port_mappings: Vec<PortMapping>,
    pub backend_identity: String,
    pub stage_identity: String,
    #[serde(default)]
    pub resource_envelope_mapped: bool,
    pub plan_provenance: String,
}

/// One named WAR port mapped to a typed BLUT input or output (§49.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortMapping {
    pub war_port: String,
    pub blut_kind: String,
    /// Whether the two kinds are compatible. §49.2 requires the adapter to
    /// REJECT incompatible kinds rather than lower them into something that runs
    /// and means something else.
    pub compatible: bool,
}

impl BlutLowering {
    pub fn validate(&self) -> Result<(), SeamError> {
        if self.registry_digest.trim().is_empty() {
            return Err(SeamError::UnpinnedRegistry {
                stage: self.stage.clone(),
            });
        }
        if let Some(bad) = self.port_mappings.iter().find(|m| !m.compatible) {
            return Err(SeamError::LoweringRejected {
                stage: self.stage.clone(),
                reason: format!(
                    "port {:?} maps to incompatible kind {:?}",
                    bad.war_port, bad.blut_kind
                ),
            });
        }
        for (field, value) in [
            ("backend_identity", &self.backend_identity),
            ("stage_identity", &self.stage_identity),
            ("plan_provenance", &self.plan_provenance),
        ] {
            if value.trim().is_empty() {
                return Err(SeamError::LoweringRejected {
                    stage: self.stage.clone(),
                    reason: format!("{field} is not pinned"),
                });
            }
        }
        Ok(())
    }
}

/// §49.3 — BLUT's lineage stays authoritative in BLUT; this is the reference.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BlutLineageReceipt {
    pub status: String,
    #[serde(default)]
    pub artifact_refs: Vec<String>,
    /// A REFERENCE. Copying BLUT's lineage here would create a second answer to
    /// a question §49.3 says BLUT owns.
    pub lineage_ref: String,
    pub receipt_digest: String,
}

/// §82.3's parity comparison between two source adapters.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AdapterParity {
    pub corpus_size: usize,
    pub compiled_by_both: usize,
    /// Observable differences found. §82.3 says "declared observable parity", so
    /// what counts as observable is declared up front rather than argued after.
    #[serde(default)]
    pub differences: Vec<String>,
    #[serde(default)]
    pub declared_observables: Vec<String>,
}

impl AdapterParity {
    /// §82.3 — parity is MEASURED across the whole corpus, not sampled.
    pub fn validate(&self) -> Result<(), SeamError> {
        if self.corpus_size == 0 || self.compiled_by_both != self.corpus_size {
            return Err(SeamError::ParityNotEstablished {
                differences: format!(
                    "{} of {} compiled by both adapters",
                    self.compiled_by_both, self.corpus_size
                ),
            });
        }
        if !self.differences.is_empty() {
            return Err(SeamError::ParityNotEstablished {
                differences: self.differences.join("; "),
            });
        }
        if self.declared_observables.is_empty() {
            return Err(SeamError::ParityNotEstablished {
                differences: "no observables were declared, so 'parity' asserts nothing".to_owned(),
            });
        }
        Ok(())
    }

    /// §82.4 — cutover only once parity is established.
    pub fn permit_cutover(&self) -> Result<(), SeamError> {
        self.validate()
            .map_err(|_| SeamError::CutoverWithoutQualification)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    /// §67's four groups, transcribed. 32 actions in total.
    #[test]
    fn the_controlled_action_vocabulary_matches_the_sas() {
        assert_eq!(ContractAction::ALL.len(), 8);
        assert_eq!(ExecutionAction::ALL.len(), 12);
        assert_eq!(EvidenceAction::ALL.len(), 6);
        assert_eq!(TerminalAction::ALL.len(), 6);
        assert_eq!(CONTROLLED_ACTION_COUNT, 32);

        let all = all_controlled_actions();
        assert_eq!(all.len(), CONTROLLED_ACTION_COUNT);
        // No name may appear in two groups: the group decides who may perform it.
        let mut sorted = all.clone();
        sorted.sort_unstable();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), before, "an action name appears in two groups");

        assert_eq!(all[0], "create_warrant_draft");
        assert_eq!(all[all.len() - 1], "deprecate_warrant");
    }

    fn envelope() -> ActionEnvelope {
        ActionEnvelope {
            action_type: "authorize_warrant_contract".into(),
            actor_id: "person://quitetall".into(),
            acting_role_id: "role://authorizer".into(),
            organization_id: "org://openhuman".into(),
            target_ids: vec!["war://uuid".into()],
            payload: r#"{"revision":3}"#.into(),
            reason: "contract reviewed".into(),
            idempotency_key: "key-1".into(),
            request_id: "req-1".into(),
            expected_version: 12,
            effective_at: "2026-08-20T00:00:00Z".into(),
            max_classification: "internal".into(),
            recorded_at: None,
        }
    }

    /// §67.2 — the server assigns `recorded_at`. A client setting it is
    /// asserting when the server saw something.
    #[test]
    fn a_client_cannot_assign_recorded_at() {
        let mut e = envelope();
        e.recorded_at = Some("2026-08-20T00:00:00Z".into());
        assert!(matches!(
            e.validate(),
            Err(SeamError::ClientAssignedRecordedAt { .. })
        ));
    }

    /// §67.3 — drift FAILS rather than overwrites.
    #[test]
    fn version_drift_fails_rather_than_overwrites() {
        let e = envelope();
        assert_eq!(e.check_version(12), Ok(()));
        let err = e.check_version(13).unwrap_err();
        assert!(matches!(err, SeamError::VersionDrift { .. }), "{err}");
        assert!(err.to_string().contains("FAILS rather than overwrites"));
    }

    /// §67.4 — a retry replays; a conflicting reuse is rejected.
    #[test]
    fn idempotency_replays_equivalents_and_rejects_conflicts() {
        let e = envelope();
        assert_eq!(e.check_idempotency(None), Ok(Replay::FirstUse));
        assert_eq!(
            e.check_idempotency(Some(r#"{"revision":3}"#)),
            Ok(Replay::ReplayFirstResult)
        );
        assert!(
            matches!(
                e.check_idempotency(Some(r#"{"revision":4}"#)),
                Err(SeamError::IdempotencyConflict { .. })
            ),
            "a key was reused for a DIFFERENT action and accepted"
        );
    }

    #[test]
    fn an_action_outside_the_vocabulary_is_refused() {
        let mut e = envelope();
        e.action_type = "just_do_it".into();
        assert!(matches!(
            e.validate(),
            Err(SeamError::ActionEnvelopeIncomplete { .. })
        ));
    }

    // ---- §48 -------------------------------------------------------------

    fn receipt() -> KatanaReceipt {
        KatanaReceipt {
            session_id: "katana://run-1".into(),
            dispatch_digest: "sha256:d".into(),
            prompt_ir_digest: "sha256:p".into(),
            provider_model_identity: "anthropic/claude-opus-5".into(),
            runtime_event_log_head: "sha256:e".into(),
            realized_capabilities: vec!["fs:read".into()],
            confinement: "bubblewrap, cwd-rw, no network".into(),
            usage: "1200 input, 800 output".into(),
            artifact_refs: vec![],
            terminal_runtime_status: "completed".into(),
            receipt_digest: "sha256:r".into(),
            taint_label_refs: vec!["katana://taint/1".into()],
        }
    }

    /// §48.2 — OpenWarrant records the DIGEST and never PromptIR itself.
    #[test]
    fn the_receipt_holds_a_prompt_ir_digest_and_not_prompt_ir() {
        let json = serde_json::to_string(&receipt()).expect("serialize");
        assert!(json.contains("prompt_ir_digest"));
        // A field carrying the IR itself would be a second, divergent copy of a
        // fact §48.2 says Katana owns.
        for forbidden in [
            "prompt_ir\":",
            "prompt_ir_body",
            "conversation",
            "transcript",
        ] {
            assert!(!json.contains(forbidden), "receipt carries {forbidden}");
        }
    }

    #[test]
    fn a_receipt_for_a_different_dispatch_is_refused() {
        let r = receipt();
        assert_eq!(r.validate("sha256:d"), Ok(()));
        assert!(matches!(
            r.validate("sha256:other"),
            Err(SeamError::ReceiptDispatchMismatch { .. })
        ));
    }

    #[test]
    fn each_missing_minimum_receipt_field_is_named() {
        type Blank = (&'static str, fn(&mut KatanaReceipt));
        let blanks: [Blank; 8] = [
            ("dispatch_digest", |r| r.dispatch_digest.clear()),
            ("prompt_ir_digest", |r| r.prompt_ir_digest.clear()),
            ("provider_model_identity", |r| {
                r.provider_model_identity.clear();
            }),
            ("runtime_event_log_head", |r| {
                r.runtime_event_log_head.clear();
            }),
            ("confinement", |r| r.confinement.clear()),
            ("usage", |r| r.usage.clear()),
            ("terminal_runtime_status", |r| {
                r.terminal_runtime_status.clear();
            }),
            ("receipt_digest", |r| r.receipt_digest.clear()),
        ];
        for (name, blank) in blanks {
            let mut r = receipt();
            blank(&mut r);
            match r.validate("sha256:d") {
                Err(SeamError::KatanaReceiptIncomplete { field, .. }) => {
                    assert_eq!(field, name);
                }
                Err(SeamError::ReceiptDispatchMismatch { .. }) if name == "dispatch_digest" => {}
                other => panic!("receipt without {name} was accepted: {other:?}"),
            }
        }
    }

    /// §48.3 — the asymmetry. Realizing fewer capabilities is fine; realizing
    /// more is a containment failure.
    #[test]
    fn realizing_more_than_was_authorized_is_a_containment_failure() {
        let authorized = ["fs:read".to_owned(), "fs:write".to_owned()];

        // Fewer is fine.
        assert_eq!(
            realized_within_authorization(&authorized, &["fs:read".to_owned()]),
            Ok(())
        );
        // Exactly is fine.
        assert_eq!(
            realized_within_authorization(&authorized, &authorized),
            Ok(())
        );
        // More is not.
        let err = realized_within_authorization(
            &authorized,
            &["fs:read".to_owned(), "net:egress".to_owned()],
        )
        .unwrap_err();
        assert!(err.to_string().contains("net:egress"), "{err}");
    }

    // ---- §49 -------------------------------------------------------------

    fn lowering() -> BlutLowering {
        BlutLowering {
            stage: "STAGE-003".into(),
            registry_digest: "sha256:reg".into(),
            port_mappings: vec![PortMapping {
                war_port: "encoded".into(),
                blut_kind: "artifact/bytes".into(),
                compatible: true,
            }],
            backend_identity: "blut://backend/local".into(),
            stage_identity: "blut://stage/train".into(),
            resource_envelope_mapped: true,
            plan_provenance: "planspec://p1".into(),
        }
    }

    /// §49.2 — reject incompatible kinds rather than lowering them into
    /// something that runs and means something else.
    #[test]
    fn an_incompatible_port_kind_is_rejected_not_coerced() {
        let mut l = lowering();
        l.port_mappings[0].compatible = false;
        let err = l.validate().unwrap_err();
        assert!(matches!(err, SeamError::LoweringRejected { .. }), "{err}");
        assert!(err.to_string().contains("encoded"), "{err}");
    }

    /// §49.2 — an unpinned registry means the same stage name means different
    /// things on different days.
    #[test]
    fn an_unpinned_registry_is_refused() {
        let mut l = lowering();
        l.registry_digest.clear();
        assert!(matches!(
            l.validate(),
            Err(SeamError::UnpinnedRegistry { .. })
        ));
    }

    #[test]
    fn unpinned_identities_are_refused() {
        for (name, mut l) in [
            ("backend_identity", lowering()),
            ("stage_identity", lowering()),
            ("plan_provenance", lowering()),
        ] {
            match name {
                "backend_identity" => l.backend_identity.clear(),
                "stage_identity" => l.stage_identity.clear(),
                _ => l.plan_provenance.clear(),
            }
            let err = l.validate().unwrap_err();
            assert!(err.to_string().contains(name), "{name}: {err}");
        }
    }

    /// §49.3 — the lineage is referenced, not copied.
    #[test]
    fn blut_lineage_is_referenced_not_reproduced() {
        let r = BlutLineageReceipt {
            status: "succeeded".into(),
            artifact_refs: vec!["artifact://a".into()],
            lineage_ref: "blut://lineage/42".into(),
            receipt_digest: "sha256:r".into(),
        };
        let json = serde_json::to_string(&r).expect("serialize");
        assert!(json.contains("lineage_ref"));
        assert!(
            !json.contains("lineage_events") && !json.contains("lineage_graph"),
            "a copy of BLUT's authoritative lineage would be a second answer"
        );
    }

    // ---- §82 -------------------------------------------------------------

    /// §82.3 — parity is measured across the WHOLE corpus, not sampled.
    #[test]
    fn parity_must_cover_the_whole_corpus() {
        let sampled = AdapterParity {
            corpus_size: 40,
            compiled_by_both: 12,
            differences: vec![],
            declared_observables: vec!["canonical bytes".into()],
        };
        let err = sampled.validate().unwrap_err();
        assert!(err.to_string().contains("12 of 40"), "{err}");

        let full = AdapterParity {
            corpus_size: 40,
            compiled_by_both: 40,
            differences: vec![],
            declared_observables: vec!["canonical bytes".into()],
        };
        assert_eq!(full.validate(), Ok(()));
    }

    /// "Parity" with no declared observables asserts nothing at all.
    #[test]
    fn parity_with_no_declared_observables_asserts_nothing() {
        let p = AdapterParity {
            corpus_size: 40,
            compiled_by_both: 40,
            differences: vec![],
            declared_observables: vec![],
        };
        let err = p.validate().unwrap_err();
        assert!(err.to_string().contains("asserts nothing"), "{err}");
    }

    /// §82.4 — cutover only once qualified.
    #[test]
    fn cutover_is_refused_until_parity_holds() {
        let failing = AdapterParity {
            corpus_size: 40,
            compiled_by_both: 40,
            differences: vec!["OW-WAR-0007 canonical bytes differ".into()],
            declared_observables: vec!["canonical bytes".into()],
        };
        assert_eq!(
            failing.permit_cutover(),
            Err(SeamError::CutoverWithoutQualification)
        );

        let passing = AdapterParity {
            differences: vec![],
            ..failing
        };
        assert_eq!(passing.permit_cutover(), Ok(()));
    }

    #[test]
    fn vocabularies_round_trip() {
        for &a in ContractAction::ALL {
            assert_eq!(ContractAction::from_str(a.as_str()), Ok(a));
        }
        for &a in TerminalAction::ALL {
            assert_eq!(TerminalAction::from_str(a.as_str()), Ok(a));
        }
        assert!(ContractAction::from_str("yolo_warrant").is_err());
    }
}
