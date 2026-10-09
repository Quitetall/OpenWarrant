// SPDX-License-Identifier: Apache-2.0
//! Synthetic whole-basis controls, not native provider qualification.
use openwarrant_compiler::{
    AtomSource, CompilationBasis, DigestDomain, lower, runtime_basis::*, sha256_digest,
};
use openwarrant_core::{
    Manifest, ValidatedManifest,
    document::{records::raw_digest, runtime::*},
    execution::{DISPATCH_API_VERSION, StageDispatch},
    seam::KatanaReceipt,
};

const RAW: &[u8] = b"synthetic receipt";

fn fixture(kind: &str) -> (CompilationBasis, ValidatedManifest) {
    let roles = ["intent", "basis", "work_order", "milestones", "assurance"];
    let manifest: Manifest = serde_json::from_value(serde_json::json!({
        "schema": openwarrant_core::MANIFEST_SCHEMA,
        "uuid": "01a018db-19fc-7f2a-8e39-69730f255e33", "local_alias": "OW-WAR-0001",
        "title": "Synthetic runtime basis", "profile": "delivery", "assurance_level": "basic",
        "atoms": roles.iter().enumerate().map(|(i,r)| serde_json::json!({"ordinal":(i+1)*10,"role":r,"path":format!("atoms/{r}"),"required":true})).collect::<Vec<_>>()
    })).unwrap();
    let validated = manifest.validate(Some("OW")).unwrap();
    let graph = format!(
        "schema: oh.war/milestones/v1\nmilestones:\n  - id: M1\n    stage_refs: [S1, S2]\n    obligation_refs: [OBL-001]\nstages:\n  - id: S1\n    executor_kind: {kind}\n    responsibility_tier: T3\n    executor_ref: synthetic\n  - id: S2\n    executor_kind: {kind}\n    responsibility_tier: T3\n    executor_ref: synthetic\n"
    );
    let atoms = manifest
        .atoms
        .iter()
        .map(|a| AtomSource {
            ordinal: a.ordinal,
            role: a.role.clone(),
            source: a.path.clone().unwrap(),
            required: a.required,
            jurisdiction: "authored".into(),
            bytes: if a.role == "milestones" {
                graph.as_bytes().to_vec()
            } else {
                b"synthetic source".to_vec()
            },
        })
        .collect();
    (
        CompilationBasis {
            manifest,
            manifest_bytes: b"synthetic manifest capture".to_vec(),
            manifest_source: "manifest.toml".into(),
            atoms,
            scope: None,
            sas: None,
        },
        validated,
    )
}

fn dispatch(b: &CompilationBasis, v: &ValidatedManifest, stage: &str) -> StageDispatch {
    let ir = lower(b, v).unwrap();
    let mut d = StageDispatch {
        api_version: DISPATCH_API_VERSION.into(),
        warrant_ref: format!("war://{}", ir.identity.uuid),
        contract_digest: ir.contract_digest().unwrap(),
        workspace_basis_digest: ir.integrity.workspace_basis_digest,
        stage_id: stage.into(),
        milestone_id: "M1".into(),
        attempt_id: format!("attempt-{stage}"),
        attempt_basis_digest: "synthetic-attempt-basis".into(),
        objective: "Synthetic control".into(),
        workspace_basis_ref: "basis://synthetic".into(),
        context_manifest_ref: "context://synthetic".into(),
        context_manifest_digest: "context-digest".into(),
        submission_schema_ref: "schema://synthetic".into(),
        ..Default::default()
    };
    d.dispatch_digest = sha256_digest(DigestDomain::Dispatch, &d).unwrap();
    d
}
struct SyntheticVerifier {
    facts: VerifiedReceipt,
}
impl ReceiptVerifier for SyntheticVerifier {
    fn interface(&self) -> &ProviderInterface {
        &self.facts.interface
    }
    fn verify(&self, _: &[u8]) -> Result<VerifiedReceipt, ProviderFailure> {
        Ok(self.facts.clone())
    }
}
fn verifier(d: &StageDispatch) -> SyntheticVerifier {
    SyntheticVerifier {
        facts: VerifiedReceipt {
            interface: ProviderInterface {
                kind: ProviderKind::Katana,
                identity: "synthetic-provider".into(),
                version: "synthetic/v1".into(),
            },
            binding: RuntimeBinding::from_dispatch(d),
            raw_digest: raw_digest(RAW),
            receipt: NativeReceipt::Katana(KatanaReceipt {
                session_id: "synthetic-session".into(),
                dispatch_digest: d.dispatch_digest.clone(),
                prompt_ir_digest: "synthetic-prompt".into(),
                provider_model_identity: "synthetic-model".into(),
                runtime_event_log_head: "synthetic-events".into(),
                confinement: "synthetic-confinement".into(),
                usage: "unknown".into(),
                terminal_runtime_status: "completed".into(),
                receipt_digest: "synthetic-provider-seal".into(),
                ..Default::default()
            }),
            outcome: RuntimeOutcome::Completed,
            execution: Observation::Established,
            confinement: Observation::Established,
            metered_cost: Observation::Unknown,
            spend_cap: Observation::Unknown,
            registry_digest: None,
        },
    }
}
fn evidence<'a>(d: &'a StageDispatch, v: &'a SyntheticVerifier) -> RuntimeStageEvidence<'a> {
    RuntimeStageEvidence {
        expectation: RuntimeExpectation {
            dispatch: d,
            provider: v.interface(),
            registry_digest: None,
            authorized_capabilities: Some(&[]),
            confinement_required: true,
            hard_spend_cap_required: false,
            max_receipt_bytes: 100,
        },
        raw_receipt: RAW,
        verifier: Some(v),
    }
}

#[test]
fn one_receipt_cannot_satisfy_two_stages_and_cost_stays_unknown() {
    let (b, v) = fixture("katana");
    let d1 = dispatch(&b, &v, "S1");
    let d2 = dispatch(&b, &v, "S2");
    let p1 = verifier(&d1);
    let p2 = verifier(&d2);
    let partial = assess_runtime_basis(&b, &v, &[evidence(&d1, &p1)]);
    assert_eq!(partial.standing, ReceiptStanding::Unknown);
    assert_eq!(
        partial.stages[1].receipt.code,
        "runtime-basis.receipt-missing"
    );
    let full = assess_runtime_basis(&b, &v, &[evidence(&d1, &p1), evidence(&d2, &p2)]);
    assert_eq!(full.standing, ReceiptStanding::Matches);
    assert!(
        full.stages
            .iter()
            .all(|s| s.receipt.cost_observation == Observation::Unknown)
    );
}

#[test]
fn modified_dispatch_or_current_contract_is_refused() {
    let (b, v) = fixture("katana");
    for field in ["digest", "milestone", "contract", "workspace", "warrant"] {
        let mut d = dispatch(&b, &v, "S1");
        match field {
            "digest" => d.dispatch_digest = "tampered".into(),
            "milestone" => d.milestone_id = "absent".into(),
            "contract" => d.contract_digest = "old".into(),
            "workspace" => d.workspace_basis_digest = "old".into(),
            _ => d.warrant_ref = "war://other".into(),
        }
        if field != "digest" {
            d.dispatch_digest.clear();
            d.dispatch_digest = sha256_digest(DigestDomain::Dispatch, &d).unwrap();
        }
        // Even a fresh provider seal over an altered dispatch cannot establish its currency.
        let p = verifier(&d);
        let a = assess_runtime_basis(&b, &v, &[evidence(&d, &p)]);
        assert_eq!(a.standing, ReceiptStanding::Refused, "{field}");
        assert_eq!(a.stages[0].receipt.code, "runtime-basis.dispatch-mismatch");
    }
    let d = dispatch(&b, &v, "S1");
    let p = verifier(&d);
    let mut changed = b.clone();
    changed.atoms[0].bytes.push(b'!');
    assert_eq!(
        assess_runtime_basis(&changed, &v, &[evidence(&d, &p)]).standing,
        ReceiptStanding::Refused
    );
}

#[test]
fn duplicate_attempts_and_unrelated_receipts_are_refused() {
    let (b, v) = fixture("katana");
    let d = dispatch(&b, &v, "S1");
    let p = verifier(&d);
    assert_eq!(
        assess_runtime_basis(&b, &v, &[evidence(&d, &p), evidence(&d, &p)]).code,
        "runtime-basis.ambiguous-attempt"
    );
    let unrelated = dispatch(&b, &v, "other");
    let p = verifier(&unrelated);
    assert_eq!(
        assess_runtime_basis(&b, &v, &[evidence(&unrelated, &p)]).code,
        "runtime-basis.unexpected-stage"
    );
}

#[test]
fn valid_nonruntime_inventory_passes_but_missing_or_malformed_sources_do_not() {
    let (b, v) = fixture("human");
    assert_eq!(
        assess_runtime_basis(&b, &v, &[]).standing,
        ReceiptStanding::Matches
    );
    let mut missing = b.clone();
    missing.atoms.retain(|a| a.role != "milestones");
    assert_eq!(
        assess_runtime_basis(&missing, &v, &[]).code,
        "runtime-basis.source-missing"
    );
    for bytes in [b"not yaml".to_vec(), vec![255]] {
        let mut bad = b.clone();
        bad.atoms[3].bytes = bytes;
        assert_eq!(
            assess_runtime_basis(&bad, &v, &[]).code,
            "runtime-basis.invalid-milestones"
        );
    }
    let mut mismatched = v.clone();
    mismatched.raw.title = "different".into();
    assert_eq!(
        assess_runtime_basis(&b, &mismatched, &[]).code,
        "runtime-basis.manifest-mismatch"
    );
}

#[test]
fn wrong_provider_missing_verifier_and_failed_execution_stay_distinct() {
    let (b, v) = fixture("katana");
    let d = dispatch(&b, &v, "S1");
    let mut p = verifier(&d);
    p.facts.interface.kind = ProviderKind::Blut;
    assert_eq!(
        assess_runtime_basis(&b, &v, &[evidence(&d, &p)]).stages[0]
            .receipt
            .code,
        "runtime-basis.dispatch-mismatch"
    );
    let mut p = verifier(&d);
    let mut e = evidence(&d, &p);
    e.verifier = None;
    assert_eq!(
        assess_runtime_basis(&b, &v, &[e]).stages[0].receipt.code,
        "runtime.verifier-unavailable"
    );
    p.facts.outcome = RuntimeOutcome::Failed;
    assert_eq!(
        assess_runtime_basis(&b, &v, &[evidence(&d, &p)]).standing,
        ReceiptStanding::Refused
    );
}

#[test]
fn duplicate_stage_ids_across_sources_do_not_collapse_into_one_requirement() {
    let (mut b, _) = fixture("katana");
    let mut entry = b.manifest.atoms[3].clone();
    entry.ordinal = 90;
    entry.path = Some("atoms/other-milestones".into());
    let mut atom = b.atoms[3].clone();
    atom.ordinal = entry.ordinal;
    atom.source = entry.path.clone().unwrap();
    atom.bytes = String::from_utf8(atom.bytes)
        .unwrap()
        .replace("id: M1", "id: M2")
        .into_bytes();
    b.manifest.atoms.push(entry);
    b.atoms.push(atom);
    let v = b.manifest.validate(Some("OW")).unwrap();
    assert_eq!(
        assess_runtime_basis(&b, &v, &[]).code,
        "runtime-basis.duplicate-stage"
    );
}

#[test]
fn hard_spend_cap_cannot_be_satisfied_by_an_unknown_cost() {
    let (b, v) = fixture("katana");
    let d = dispatch(&b, &v, "S1");
    let p = verifier(&d);
    let mut e = evidence(&d, &p);
    e.expectation.hard_spend_cap_required = true;
    let a = assess_runtime_basis(&b, &v, &[e]);
    assert_eq!(a.standing, ReceiptStanding::Unknown);
    assert_eq!(a.stages[0].receipt.code, "runtime.accounting-unestablished");
}
