// SPDX-License-Identifier: Apache-2.0
//! Synthetic public SDK boundary controls, not actual provider qualification.
use openwarrant_core::{
    document::{records::raw_digest, runtime::*},
    execution::{DISPATCH_API_VERSION, StageDispatch},
    seam::{BlutLineageReceipt, KatanaReceipt},
};
use std::cell::Cell;

const RAW: &[u8] = b"synthetic provider-owned receipt bytes";

fn dispatch() -> StageDispatch {
    StageDispatch {
        api_version: DISPATCH_API_VERSION.into(),
        warrant_ref: "war://synthetic".into(),
        contract_digest: "contract-A".into(),
        dispatch_digest: "dispatch-A".into(),
        stage_id: "STAGE-001".into(),
        attempt_id: "attempt-A".into(),
        attempt_basis_digest: "basis-A".into(),
        objective: "Exercise the synthetic receipt boundary".into(),
        workspace_basis_ref: "git://synthetic".into(),
        workspace_basis_digest: "workspace-A".into(),
        context_manifest_ref: "context://synthetic".into(),
        context_manifest_digest: "context-A".into(),
        submission_schema_ref: "schema://synthetic".into(),
        ..Default::default()
    }
}

fn interface(kind: ProviderKind) -> ProviderInterface {
    ProviderInterface {
        kind,
        identity: "synthetic-provider-build".into(),
        version: "synthetic-interface/v1".into(),
    }
}

fn facts(kind: ProviderKind) -> VerifiedReceipt {
    VerifiedReceipt {
        interface: interface(kind),
        raw_digest: raw_digest(RAW),
        binding: RuntimeBinding::from_dispatch(&dispatch()),
        receipt: match kind {
            ProviderKind::Katana => NativeReceipt::Katana(KatanaReceipt {
                session_id: "session-A".into(),
                dispatch_digest: "dispatch-A".into(),
                prompt_ir_digest: "provider-prompt-A".into(),
                provider_model_identity: "local-synthetic-model".into(),
                runtime_event_log_head: "provider-log-A".into(),
                realized_capabilities: vec!["read".into()],
                confinement: "provider-observed-confinement".into(),
                usage: "unknown-cost".into(),
                terminal_runtime_status: "completed".into(),
                receipt_digest: "provider-defined-opaque-seal".into(),
                ..Default::default()
            }),
            ProviderKind::Blut => NativeReceipt::Blut(BlutLineageReceipt {
                status: "completed".into(),
                artifact_refs: vec!["artifact://job-A".into()],
                lineage_ref: "blut://job-A/lineage".into(),
                receipt_digest: "provider-defined-opaque-seal".into(),
            }),
        },
        outcome: RuntimeOutcome::Completed,
        execution: Observation::Established,
        confinement: Observation::Established,
        metered_cost: Observation::Unknown,
        spend_cap: Observation::Unknown,
        registry_digest: Some("registry-A".into()),
    }
}

struct Verifier {
    identity: ProviderInterface,
    response: Result<VerifiedReceipt, ProviderFailure>,
    calls: Cell<usize>,
}
impl ReceiptVerifier for Verifier {
    fn interface(&self) -> &ProviderInterface {
        &self.identity
    }
    fn verify(&self, _: &[u8]) -> Result<VerifiedReceipt, ProviderFailure> {
        self.calls.set(self.calls.get() + 1);
        self.response.clone()
    }
}
fn verifier(facts: VerifiedReceipt) -> Verifier {
    Verifier {
        identity: facts.interface.clone(),
        response: Ok(facts),
        calls: Cell::new(0),
    }
}
fn assess(facts: VerifiedReceipt) -> ReceiptAssessment {
    let provider = facts.interface.clone();
    let verifier = verifier(facts);
    assess_runtime_receipt(
        &RuntimeExpectation {
            dispatch: &dispatch(),
            provider: &provider,
            registry_digest: Some("registry-A"),
            authorized_capabilities: Some(&["read".into()]),
            confinement_required: true,
            hard_spend_cap_required: false,
            max_receipt_bytes: 1024,
        },
        RAW,
        Some(&verifier),
    )
}

#[test]
fn exact_verified_facts_match_without_becoming_an_assurance_mark_or_zero_cost() {
    for kind in [ProviderKind::Katana, ProviderKind::Blut] {
        let result = assess(facts(kind));
        assert_eq!(result.standing, ReceiptStanding::Matches);
        assert_eq!(result.raw_digest, Some(raw_digest(RAW)));
        assert_eq!(
            result.provider_receipt_digest.as_deref(),
            Some("provider-defined-opaque-seal")
        );
        assert_eq!(result.cost_observation, Observation::Unknown);
    }
}

#[test]
fn every_dispatch_binding_component_and_raw_capture_must_match() {
    for field in 0..6 {
        let mut receipt = facts(ProviderKind::Katana);
        match field {
            0 => receipt.binding.warrant_ref.push_str("-wrong"),
            1 => receipt.binding.contract_digest.push_str("-wrong"),
            2 => receipt.binding.dispatch_digest.push_str("-wrong"),
            3 => receipt.binding.stage_id.push_str("-wrong"),
            4 => receipt.binding.attempt_id.push_str("-wrong"),
            _ => receipt.raw_digest = raw_digest(b"different receipt bytes"),
        }
        let result = assess(receipt);
        assert_eq!(result.standing, ReceiptStanding::Refused);
        assert_eq!(
            result.code,
            if field == 5 {
                "runtime.raw-mismatch"
            } else {
                "runtime.binding-mismatch"
            }
        );
    }
}

#[test]
fn provider_identity_version_native_kind_and_minimum_fields_are_checked() {
    let provider = interface(ProviderKind::Katana);
    let expect = RuntimeExpectation {
        dispatch: &dispatch(),
        provider: &provider,
        registry_digest: None,
        authorized_capabilities: Some(&[]),
        confinement_required: false,
        hard_spend_cap_required: false,
        max_receipt_bytes: 1024,
    };
    for field in 0..3 {
        let mut adapter = verifier(facts(ProviderKind::Katana));
        match field {
            0 => adapter.identity.identity.push_str("-wrong"),
            1 => adapter.identity.version.push_str("-wrong"),
            _ => adapter.identity.kind = ProviderKind::Blut,
        }
        let result = assess_runtime_receipt(&expect, RAW, Some(&adapter));
        assert_eq!(result.code, "runtime.provider-mismatch");
        assert_eq!(adapter.calls.get(), 0);
    }
    let mut receipt = facts(ProviderKind::Katana);
    receipt.interface.version.push_str("-wrong");
    let mut adapter = verifier(receipt);
    adapter.identity = provider.clone();
    assert_eq!(
        assess_runtime_receipt(&expect, RAW, Some(&adapter)).code,
        "runtime.provider-mismatch"
    );
    let mut receipt = facts(ProviderKind::Katana);
    receipt.receipt = facts(ProviderKind::Blut).receipt;
    assert_eq!(assess(receipt).code, "runtime.provider-mismatch");
    let mut receipt = facts(ProviderKind::Katana);
    if let NativeReceipt::Katana(native) = &mut receipt.receipt {
        native.prompt_ir_digest.clear();
    }
    assert_eq!(assess(receipt).code, "runtime.invalid-native-receipt");
}

#[test]
fn absence_unsupported_and_unavailable_remain_unknown_but_bad_seals_refuse() {
    let provider = interface(ProviderKind::Katana);
    let expect = RuntimeExpectation {
        dispatch: &dispatch(),
        provider: &provider,
        registry_digest: None,
        authorized_capabilities: Some(&[]),
        confinement_required: false,
        hard_spend_cap_required: false,
        max_receipt_bytes: 1024,
    };
    assert_eq!(
        assess_runtime_receipt(&expect, RAW, None).standing,
        ReceiptStanding::Unknown
    );
    for (failure, standing, code) in [
        (
            ProviderFailure::Unsupported("unknown protocol".into()),
            ReceiptStanding::Unknown,
            "runtime.unsupported",
        ),
        (
            ProviderFailure::Unavailable("provider offline".into()),
            ReceiptStanding::Unknown,
            "runtime.unavailable",
        ),
        (
            ProviderFailure::Rejected("altered native seal".into()),
            ReceiptStanding::Refused,
            "runtime.provider-rejected",
        ),
    ] {
        let adapter = Verifier {
            identity: provider.clone(),
            response: Err(failure),
            calls: Cell::new(0),
        };
        let result = assess_runtime_receipt(&expect, RAW, Some(&adapter));
        assert_eq!((result.standing, result.code), (standing, code));
        assert_eq!(adapter.calls.get(), 1);
    }
}

#[test]
fn policy_registry_execution_terminal_and_limits_cannot_be_inferred_from_receipt_text() {
    for outcome in [
        RuntimeOutcome::Failed,
        RuntimeOutcome::Halted,
        RuntimeOutcome::Cancelled,
        RuntimeOutcome::Running,
    ] {
        let mut receipt = facts(ProviderKind::Katana);
        receipt.outcome = outcome;
        assert_eq!(assess(receipt).code, "runtime.not-completed");
    }
    let mut receipt = facts(ProviderKind::Katana);
    receipt.outcome = RuntimeOutcome::Unknown;
    assert_eq!(assess(receipt).standing, ReceiptStanding::Unknown);
    for observed in [Observation::Unknown, Observation::Refuted] {
        let mut receipt = facts(ProviderKind::Katana);
        receipt.execution = observed;
        assert_eq!(assess(receipt).code, "runtime.execution-unestablished");
        let mut receipt = facts(ProviderKind::Katana);
        receipt.confinement = observed;
        assert_eq!(assess(receipt).code, "runtime.confinement-unestablished");
    }
    let mut receipt = facts(ProviderKind::Katana);
    if let NativeReceipt::Katana(native) = &mut receipt.receipt {
        native.realized_capabilities.push("write".into());
    }
    assert_eq!(assess(receipt).code, "runtime.capability-refused");
    let mut receipt = facts(ProviderKind::Blut);
    receipt.registry_digest = None;
    assert_eq!(assess(receipt).standing, ReceiptStanding::Unknown);
    let mut receipt = facts(ProviderKind::Blut);
    receipt.registry_digest = Some("different-job-registry".into());
    assert_eq!(assess(receipt).code, "runtime.registry-mismatch");
}

#[test]
fn hard_cap_needs_observed_accounting_and_enforcement_and_bounds_precede_provider_call() {
    let provider = interface(ProviderKind::Katana);
    let dispatch = dispatch();
    let allowed = ["read".into()];
    let mut expect = RuntimeExpectation {
        dispatch: &dispatch,
        provider: &provider,
        registry_digest: None,
        authorized_capabilities: Some(&allowed),
        confinement_required: false,
        hard_spend_cap_required: true,
        max_receipt_bytes: 1024,
    };
    let mut receipt = facts(ProviderKind::Katana);
    let adapter = verifier(receipt.clone());
    assert_eq!(
        assess_runtime_receipt(&expect, RAW, Some(&adapter)).code,
        "runtime.accounting-unestablished"
    );
    receipt.metered_cost = Observation::Established;
    assert_eq!(
        assess_runtime_receipt(&expect, RAW, Some(&verifier(receipt.clone()))).code,
        "runtime.spend-cap-unestablished"
    );
    receipt.spend_cap = Observation::Established;
    assert_eq!(
        assess_runtime_receipt(&expect, RAW, Some(&verifier(receipt))).standing,
        ReceiptStanding::Matches
    );
    let adapter = verifier(facts(ProviderKind::Katana));
    expect.max_receipt_bytes = 1;
    assert_eq!(
        assess_runtime_receipt(&expect, RAW, Some(&adapter)).code,
        "runtime.receipt-limit"
    );
    assert_eq!(adapter.calls.get(), 0);
    expect.max_receipt_bytes = 1024;
    expect.authorized_capabilities = None;
    assert_eq!(
        assess_runtime_receipt(&expect, RAW, Some(&adapter)).code,
        "runtime.capabilities-unknown"
    );
}

#[test]
fn incomplete_expectations_and_empty_capture_never_call_provider() {
    let provider = interface(ProviderKind::Katana);
    let mut dispatch = dispatch();
    dispatch.attempt_id.clear();
    let adapter = verifier(facts(ProviderKind::Katana));
    let mut expect = RuntimeExpectation {
        dispatch: &dispatch,
        provider: &provider,
        registry_digest: None,
        authorized_capabilities: Some(&[]),
        confinement_required: false,
        hard_spend_cap_required: false,
        max_receipt_bytes: 1024,
    };
    assert_eq!(
        assess_runtime_receipt(&expect, RAW, Some(&adapter)).code,
        "runtime.invalid-expectation"
    );
    let valid = self::dispatch();
    expect.dispatch = &valid;
    assert_eq!(
        assess_runtime_receipt(&expect, b"", Some(&adapter)).code,
        "runtime.receipt-limit"
    );
    assert_eq!(adapter.calls.get(), 0);
    let mut receipt = facts(ProviderKind::Blut);
    receipt.registry_digest = Some("  ".into());
    assert_eq!(assess(receipt).code, "runtime.registry-unknown");
}
