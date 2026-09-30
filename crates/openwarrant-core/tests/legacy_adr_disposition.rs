// SPDX-License-Identifier: AGPL-3.0-or-later

use openwarrant_core::legacy_disposition::{
    ContentBinding, GitObjectFormat, HumanReview, ImportBinding, ImportedBodyStatus,
    LEGACY_ADR_DISPOSITION_MANIFEST_KIND, LEGACY_ADR_DISPOSITION_MANIFEST_SCHEMA,
    LEGACY_ADR_DISPOSITION_PROPOSAL_KIND, LEGACY_ADR_DISPOSITION_PROPOSAL_SCHEMA,
    LEGACY_ADR_DISPOSITION_RECEIPT_KIND, LEGACY_ADR_DISPOSITION_RECEIPT_SCHEMA,
    LEGACY_ADR_DISPOSITION_REVIEW_RESPONSE_KIND, LEGACY_ADR_DISPOSITION_REVIEW_RESPONSE_SCHEMA,
    LEGACY_ADR_DISPOSITION_REVIEW_SUBJECT_KIND, LEGACY_ADR_DISPOSITION_REVIEW_SUBJECT_SCHEMA,
    LEGACY_ADR_FALSIFICATION_SCHEMA, LEGACY_ADR_SOURCE_CAPSULE_SCHEMA, LegacyAdrDisposition,
    LegacyAdrDispositionJudgment, LegacyAdrDispositionManifest,
    LegacyAdrDispositionManifestPayload, LegacyAdrDispositionProposal,
    LegacyAdrDispositionProposalPayload, LegacyAdrDispositionReceipt,
    LegacyAdrDispositionReceiptPayload, LegacyAdrDispositionReviewResponse,
    LegacyAdrFalsificationRecord, LegacyAdrSourceCapsule, LegacyAdrSuccessor, LegacyAdrSupport,
    LegacyDispositionError, LegacyExtensionNumberError, LegacyExtensionValue, LegacyJsonError,
    LegacyJsonRecord, MAX_LEGACY_JSON_BYTES, MAX_LEGACY_JSON_NESTING, MAX_LEGACY_JSON_TOKENS,
    SourceBinding, WarrantBinding, parse_legacy_json,
};
use openwarrant_core::{JudgmentAuthority, WarUuid};
use serde::Serialize;
use serde_json::{Value, json};

const SHA_F: &str = "ba9ed833faa9a52940d5e9d424566466e9066867";
const OTHER_COMMIT: &str = "1111111111111111111111111111111111111111";
const DIGEST_A: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DIGEST_B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const DIGEST_C: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const DIGEST_D: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const EXTENSION_NAME: &str = "example.review-context";
const WARRANT_UUID: &str = "018f22c2-7d00-7cc3-98c4-dc0c0c07398f";

fn assert_namespaced_extension_round_trip<T>(mut value: Value, record: &str)
where
    T: LegacyJsonRecord + Serialize,
{
    let extension = json!({
        "labels": ["preserved", "canonical"],
        "nested": {"enabled": true, "ratio": 1.25},
        "nullable": null
    });
    value[EXTENSION_NAME] = extension.clone();

    let decoded: T = parse_value(value.clone())
        .unwrap_or_else(|error| panic!("{record} rejected a namespaced extension: {error}"));
    let reencoded = serde_json::to_value(decoded)
        .unwrap_or_else(|error| panic!("{record} did not reserialize: {error}"));

    assert_eq!(reencoded, value, "{record} changed during round trip");
    assert_eq!(
        reencoded[EXTENSION_NAME], extension,
        "{record} dropped or changed its namespaced extension"
    );
}

fn assert_non_namespaced_unknown_rejected<T>(mut value: Value, record: &str)
where
    T: LegacyJsonRecord,
{
    value["unexpected"] = json!({"must_not_be_ignored": true});
    let error = parse_value::<T>(value)
        .err()
        .unwrap_or_else(|| panic!("{record} accepted a non-namespaced unknown field"));

    assert!(
        error
            .to_string()
            .contains("optional extensions must be namespaced"),
        "{record} failed for the wrong reason: {error}"
    );
}

fn parse_value<T>(value: Value) -> Result<T, LegacyJsonError>
where
    T: LegacyJsonRecord,
{
    let bytes = serde_json::to_vec(&value).expect("serialize strict JSON test value");
    parse_legacy_json(&bytes)
}

#[test]
fn required_gate_pass_support_refuses_the_obsolete_two_object_shape() {
    let old_shape = json!({
        "kind": "required_gate_pass",
        "run": {"reference": "ow:docs/receipts/GR-1.run.toml", "sha256": DIGEST_A},
        "receipt": {"reference": "ow:docs/receipts/GR-1.receipt.json", "sha256": DIGEST_B},
        "subject_source_sha256": DIGEST_C
    });

    assert!(
        parse_value::<LegacyAdrSupport>(old_shape).is_err(),
        "a Gate Run plus receipt cannot stand in for Definition, Binding, streams, and fixtures"
    );
}

#[test]
fn required_gate_pass_full_shape_round_trips() {
    let full_shape = json!({
        "kind": "required_gate_pass",
        "run": {"reference": "ow:docs/receipts/run-1.run.toml", "sha256": DIGEST_A},
        "definition": {"reference": "ow:docs/gates/gate-1.yaml", "sha256": DIGEST_B},
        "binding": {"reference": "ow:docs/receipts/run-1.binding.json", "sha256": DIGEST_C},
        "selection": {"reference": "ow:docs/receipts/run-1.selection.json", "sha256": DIGEST_B},
        "stdout": {"reference": "ow:docs/receipts/run-1.stdout.txt", "sha256": DIGEST_D},
        "stderr": {"reference": "ow:docs/receipts/run-1.stderr.txt", "sha256": DIGEST_A},
        "fixtures": [
            {"reference": "git:1111111111111111111111111111111111111111:fixtures/a.json", "sha256": DIGEST_B},
            {"reference": "git:1111111111111111111111111111111111111111:fixtures/b.json", "sha256": DIGEST_C}
        ],
        "receipt": {"reference": "ow:docs/receipts/run-1.receipt.json", "sha256": DIGEST_D},
        "subject_source_sha256": DIGEST_A,
        "example.receipt-context": {"qualification": "controlled"}
    });

    let decoded: LegacyAdrSupport =
        parse_value(full_shape.clone()).expect("full Gate receipt bundle shape");
    assert_eq!(
        serde_json::to_value(decoded).expect("serialize full Gate receipt bundle"),
        full_shape
    );
}

#[test]
fn required_gate_pass_rejects_duplicate_role_reference() {
    let repeated = "ow:docs/receipts/run-1.run.toml";
    let mut record = receipt("0001", SHA_F, "ADR 0001 has bounded evidence.", DIGEST_D);
    record.payload.evidence = vec![LegacyAdrSupport::RequiredGatePass {
        run: content(repeated, DIGEST_A),
        definition: content("ow:docs/gates/gate-1.yaml", DIGEST_B),
        binding: content("ow:docs/receipts/run-1.binding.json", DIGEST_C),
        selection: Box::new(content("ow:docs/receipts/run-1.selection.json", DIGEST_B)),
        stdout: Box::new(content("ow:docs/receipts/run-1.stdout.txt", DIGEST_D)),
        stderr: Box::new(content("ow:docs/receipts/run-1.stderr.txt", DIGEST_A)),
        fixtures: vec![],
        receipt: Box::new(content(repeated, DIGEST_B)),
        subject_source_sha256: DIGEST_A.to_owned(),
        extensions: Default::default(),
    }];

    assert!(matches!(
        record.validate_structure(),
        Err(LegacyDispositionError::ContentBindingsNotCanonical {
            field: "gate_bundle_references",
            ..
        })
    ));
}

#[test]
fn required_gate_pass_rejects_noncanonical_fixture_inventory() {
    let mut record = receipt("0001", SHA_F, "ADR 0001 has bounded evidence.", DIGEST_D);
    record.payload.evidence = vec![LegacyAdrSupport::RequiredGatePass {
        run: content("ow:docs/receipts/run-1.run.toml", DIGEST_A),
        definition: content("ow:docs/gates/gate-1.yaml", DIGEST_B),
        binding: content("ow:docs/receipts/run-1.binding.json", DIGEST_C),
        selection: Box::new(content("ow:docs/receipts/run-1.selection.json", DIGEST_B)),
        stdout: Box::new(content("ow:docs/receipts/run-1.stdout.txt", DIGEST_D)),
        stderr: Box::new(content("ow:docs/receipts/run-1.stderr.txt", DIGEST_A)),
        fixtures: vec![
            content(
                "git:1111111111111111111111111111111111111111:fixtures/b.json",
                DIGEST_B,
            ),
            content(
                "git:1111111111111111111111111111111111111111:fixtures/a.json",
                DIGEST_C,
            ),
        ],
        receipt: Box::new(content("ow:docs/receipts/run-1.receipt.json", DIGEST_D)),
        subject_source_sha256: DIGEST_A.to_owned(),
        extensions: Default::default(),
    }];

    assert!(matches!(
        record.validate_structure(),
        Err(LegacyDispositionError::ContentBindingsNotCanonical {
            field: "gate_fixtures",
            ..
        })
    ));
}

fn content(reference: impl Into<String>, digest: &str) -> ContentBinding {
    ContentBinding {
        reference: reference.into(),
        sha256: digest.to_owned(),
        extensions: Default::default(),
    }
}

fn warrant() -> WarrantBinding {
    WarrantBinding {
        warrant_id: "OW-WAR-0043".to_owned(),
        authorized_revision: 2,
        contract_digest: "a".repeat(64),
        authorization_ref: "docs/warrants/OW-WAR-0043/authorization.json".to_owned(),
        authorization_sha256: DIGEST_B.to_owned(),
        extensions: Default::default(),
    }
}

fn source(id: &str, commit: &str) -> SourceBinding {
    SourceBinding {
        adr_id: id.to_owned(),
        repository_identity: "github.com/Quitetall/LamQuant".to_owned(),
        git_object_format: GitObjectFormat::Sha1,
        exact_ref: if commit == SHA_F {
            "refs/heads/integration/consolidate".to_owned()
        } else {
            "refs/heads/integration/off-branch".to_owned()
        },
        ref_tip_at_review: commit.to_owned(),
        commit_sha: commit.to_owned(),
        path: format!("docs/decisions/{id}-fixture.md"),
        source_sha256: DIGEST_A.to_owned(),
        extensions: Default::default(),
    }
}

fn migrated() -> ImportBinding {
    ImportBinding::Migrated {
        artifact: content("artifacts/lamquant-adr-import.json", DIGEST_B),
        imported_body_sha256: DIGEST_A.to_owned(),
        extensions: Default::default(),
    }
}

fn resolved_warrant_successor() -> LegacyAdrSuccessor {
    LegacyAdrSuccessor::ResolvedWarrant {
        warrant_uuid: WARRANT_UUID.parse::<WarUuid>().expect("UUIDv7 fixture"),
        warrant: warrant(),
        resolution: content("docs/warrants/OW-WAR-0043/resolution.json", DIGEST_C),
        resolution_digest: DIGEST_D.to_owned(),
        extensions: Default::default(),
    }
}

fn receipt(id: &str, commit: &str, reason: &str, digest: &str) -> LegacyAdrDispositionReceipt {
    LegacyAdrDispositionReceipt {
        schema: LEGACY_ADR_DISPOSITION_RECEIPT_SCHEMA.to_owned(),
        kind: LEGACY_ADR_DISPOSITION_RECEIPT_KIND.to_owned(),
        payload: LegacyAdrDispositionReceiptPayload {
            warrant: warrant(),
            source: source(id, commit),
            migration: migrated(),
            review: HumanReview {
                actor: "human:quitetall".to_owned(),
                role_assignment_ref: "docs/warrants/OW-WAR-0043/roles/human.json".to_owned(),
                role_assignment_sha256: DIGEST_C.to_owned(),
                reviewed_at: "2026-08-26T12:00:00Z".to_owned(),
                response: content("reviews/adr0186-response.json", DIGEST_B),
                judgment_id: format!("ADR0186-{id}"),
                extensions: Default::default(),
            },
            disposition: LegacyAdrDisposition::Complete,
            reason: reason.to_owned(),
            evidence: vec![LegacyAdrSupport::ImplementationArtifact {
                artifact: content(format!("git:{OTHER_COMMIT}:src/{id}.rs"), DIGEST_D),
                extensions: Default::default(),
            }],
            successors: vec![],
            proposal_digest: None,
            extensions: Default::default(),
        },
        receipt_digest: digest.to_owned(),
        extensions: Default::default(),
    }
}

fn judgment(id: &str, adr_id: &str, authority: JudgmentAuthority) -> LegacyAdrDispositionJudgment {
    LegacyAdrDispositionJudgment {
        id: id.to_owned(),
        adr_id: adr_id.to_owned(),
        review_subject_digest: DIGEST_A.to_owned(),
        statement: format!("ADR {adr_id} disposition reviewed against frozen source."),
        actor: "human:quitetall".to_owned(),
        acting_role: "Judge".to_owned(),
        meaning: "Authorizes this bounded legacy disposition only.".to_owned(),
        authority,
        limitations: vec!["No claim beyond frozen source and cited evidence.".to_owned()],
        extensions: Default::default(),
    }
}

fn review_response() -> LegacyAdrDispositionReviewResponse {
    LegacyAdrDispositionReviewResponse {
        schema: LEGACY_ADR_DISPOSITION_REVIEW_RESPONSE_SCHEMA.to_owned(),
        kind: LEGACY_ADR_DISPOSITION_REVIEW_RESPONSE_KIND.to_owned(),
        warrant: warrant(),
        reviewed_at: "2026-08-26T12:00:00Z".to_owned(),
        judgments: vec![
            judgment("ADR0186-0001", "0001", JudgmentAuthority::Authorized),
            judgment("ADR0186-0002", "0002", JudgmentAuthority::Authorized),
        ],
        extensions: Default::default(),
    }
}

fn proposal() -> LegacyAdrDispositionProposal {
    LegacyAdrDispositionProposal {
        schema: LEGACY_ADR_DISPOSITION_PROPOSAL_SCHEMA.to_owned(),
        kind: LEGACY_ADR_DISPOSITION_PROPOSAL_KIND.to_owned(),
        payload: LegacyAdrDispositionProposalPayload {
            warrant: warrant(),
            source: source("0001", SHA_F),
            migration: migrated(),
            proposed_disposition: LegacyAdrDisposition::Complete,
            reason: "Machine-generated triage proposal for later human review.".to_owned(),
            evidence: vec![LegacyAdrSupport::ImplementationArtifact {
                artifact: content(format!("git:{OTHER_COMMIT}:src/proposal-0001.rs"), DIGEST_D),
                extensions: Default::default(),
            }],
            successors: vec![],
            extensions: Default::default(),
        },
        proposal_digest: DIGEST_C.to_owned(),
        extensions: Default::default(),
    }
}

#[test]
fn optional_namespaced_extensions_survive_round_trip_for_every_protocol_record() {
    assert_namespaced_extension_round_trip::<ContentBinding>(
        serde_json::to_value(content("artifact.json", DIGEST_A)).expect("content binding value"),
        "content binding",
    );
    assert_namespaced_extension_round_trip::<WarrantBinding>(
        serde_json::to_value(warrant()).expect("warrant binding value"),
        "warrant binding",
    );
    assert_namespaced_extension_round_trip::<SourceBinding>(
        serde_json::to_value(source("0001", SHA_F)).expect("source binding value"),
        "source binding",
    );
    assert_namespaced_extension_round_trip::<ImportBinding>(
        serde_json::to_value(migrated()).expect("import binding value"),
        "import binding",
    );
    assert_namespaced_extension_round_trip::<LegacyAdrSupport>(
        json!({
            "kind": "historical_context",
            "artifact": content("history.json", DIGEST_A)
        }),
        "support",
    );
    assert_namespaced_extension_round_trip::<LegacyAdrSuccessor>(
        json!({
            "kind": "adr",
            "source": source("0002", OTHER_COMMIT)
        }),
        "successor",
    );

    let receipt_record = receipt("0001", SHA_F, "ADR 0001 has bounded evidence.", DIGEST_D);
    assert_namespaced_extension_round_trip::<HumanReview>(
        serde_json::to_value(&receipt_record.payload.review).expect("human review value"),
        "human review",
    );
    assert_namespaced_extension_round_trip::<LegacyAdrDispositionJudgment>(
        serde_json::to_value(judgment(
            "ADR0186-0001",
            "0001",
            JudgmentAuthority::Authorized,
        ))
        .expect("judgment value"),
        "judgment",
    );
    assert_namespaced_extension_round_trip::<LegacyAdrDispositionReviewResponse>(
        serde_json::to_value(review_response()).expect("review response value"),
        "review response",
    );
    assert_namespaced_extension_round_trip::<LegacyAdrDispositionProposalPayload>(
        serde_json::to_value(&proposal().payload).expect("proposal payload value"),
        "proposal payload",
    );
    assert_namespaced_extension_round_trip::<LegacyAdrDispositionProposal>(
        serde_json::to_value(proposal()).expect("proposal value"),
        "proposal",
    );
    assert_namespaced_extension_round_trip::<LegacyAdrDispositionReceiptPayload>(
        serde_json::to_value(&receipt_record.payload).expect("receipt payload value"),
        "receipt payload",
    );
    assert_namespaced_extension_round_trip::<LegacyAdrDispositionReceipt>(
        serde_json::to_value(receipt_record).expect("receipt value"),
        "receipt",
    );
    assert_namespaced_extension_round_trip::<LegacyAdrDispositionManifestPayload>(
        serde_json::to_value(&complete_manifest().payload).expect("manifest payload value"),
        "manifest payload",
    );
    assert_namespaced_extension_round_trip::<LegacyAdrDispositionManifest>(
        serde_json::to_value(complete_manifest()).expect("manifest value"),
        "manifest",
    );
}

#[test]
fn non_namespaced_unknown_fields_fail_closed_for_every_protocol_record() {
    assert_non_namespaced_unknown_rejected::<ContentBinding>(
        serde_json::to_value(content("artifact.json", DIGEST_A)).expect("content binding value"),
        "content binding",
    );
    assert_non_namespaced_unknown_rejected::<WarrantBinding>(
        serde_json::to_value(warrant()).expect("warrant binding value"),
        "warrant binding",
    );
    assert_non_namespaced_unknown_rejected::<SourceBinding>(
        serde_json::to_value(source("0001", SHA_F)).expect("source binding value"),
        "source binding",
    );
    assert_non_namespaced_unknown_rejected::<ImportBinding>(
        serde_json::to_value(migrated()).expect("import binding value"),
        "import binding",
    );
    assert_non_namespaced_unknown_rejected::<LegacyAdrSupport>(
        json!({
            "kind": "historical_context",
            "artifact": content("history.json", DIGEST_A)
        }),
        "support",
    );
    assert_non_namespaced_unknown_rejected::<LegacyAdrSuccessor>(
        json!({
            "kind": "adr",
            "source": source("0002", OTHER_COMMIT)
        }),
        "successor",
    );

    let receipt_record = receipt("0001", SHA_F, "ADR 0001 has bounded evidence.", DIGEST_D);
    assert_non_namespaced_unknown_rejected::<HumanReview>(
        serde_json::to_value(&receipt_record.payload.review).expect("human review value"),
        "human review",
    );
    assert_non_namespaced_unknown_rejected::<LegacyAdrDispositionJudgment>(
        serde_json::to_value(judgment(
            "ADR0186-0001",
            "0001",
            JudgmentAuthority::Authorized,
        ))
        .expect("judgment value"),
        "judgment",
    );
    assert_non_namespaced_unknown_rejected::<LegacyAdrDispositionReviewResponse>(
        serde_json::to_value(review_response()).expect("review response value"),
        "review response",
    );
    assert_non_namespaced_unknown_rejected::<LegacyAdrDispositionProposalPayload>(
        serde_json::to_value(&proposal().payload).expect("proposal payload value"),
        "proposal payload",
    );
    assert_non_namespaced_unknown_rejected::<LegacyAdrDispositionProposal>(
        serde_json::to_value(proposal()).expect("proposal value"),
        "proposal",
    );
    assert_non_namespaced_unknown_rejected::<LegacyAdrDispositionReceiptPayload>(
        serde_json::to_value(&receipt_record.payload).expect("receipt payload value"),
        "receipt payload",
    );
    assert_non_namespaced_unknown_rejected::<LegacyAdrDispositionReceipt>(
        serde_json::to_value(receipt_record).expect("receipt value"),
        "receipt",
    );
    assert_non_namespaced_unknown_rejected::<LegacyAdrDispositionManifestPayload>(
        serde_json::to_value(&complete_manifest().payload).expect("manifest payload value"),
        "manifest payload",
    );
    assert_non_namespaced_unknown_rejected::<LegacyAdrDispositionManifest>(
        serde_json::to_value(complete_manifest()).expect("manifest value"),
        "manifest",
    );
}

#[test]
fn namespaced_extensions_participate_in_logical_digest_preimages() {
    let extension = json!({"purpose": "digest-bound", "ordinal": 7.25});

    let base_proposal = proposal();
    let base_proposal_preimage =
        serde_json::to_value(base_proposal.digest_preimage()).expect("proposal preimage");
    let mut extended_proposal_value = serde_json::to_value(&base_proposal).expect("proposal value");
    extended_proposal_value[EXTENSION_NAME] = extension.clone();
    let extended_proposal: LegacyAdrDispositionProposal =
        parse_value(extended_proposal_value).expect("extended proposal");
    let extended_proposal_preimage = serde_json::to_value(extended_proposal.digest_preimage())
        .expect("extended proposal preimage");
    assert_ne!(base_proposal_preimage, extended_proposal_preimage);
    assert_eq!(extended_proposal_preimage[EXTENSION_NAME], extension);

    let mut extended_proposal_payload_value =
        serde_json::to_value(base_proposal).expect("proposal value");
    extended_proposal_payload_value["payload"][EXTENSION_NAME] = extension.clone();
    let extended_proposal_payload: LegacyAdrDispositionProposal =
        parse_value(extended_proposal_payload_value).expect("extended proposal payload");
    let extended_proposal_payload_preimage =
        serde_json::to_value(extended_proposal_payload.digest_preimage())
            .expect("extended proposal payload preimage");
    assert_ne!(
        base_proposal_preimage, extended_proposal_payload_preimage,
        "proposal payload extensions must be content-bound"
    );
    assert_eq!(
        extended_proposal_payload_preimage["payload"][EXTENSION_NAME],
        extension
    );

    let base_receipt = receipt("0001", SHA_F, "ADR 0001 has bounded evidence.", DIGEST_D);
    let base_receipt_preimage =
        serde_json::to_value(base_receipt.digest_preimage()).expect("receipt preimage");
    let mut extended_receipt_value = serde_json::to_value(base_receipt).expect("receipt value");
    extended_receipt_value[EXTENSION_NAME] = extension.clone();
    extended_receipt_value["payload"][EXTENSION_NAME] = extension.clone();
    extended_receipt_value["payload"]["evidence"][0]["artifact"][EXTENSION_NAME] =
        extension.clone();
    let extended_receipt: LegacyAdrDispositionReceipt =
        parse_value(extended_receipt_value).expect("extended receipt");
    let extended_receipt_preimage = serde_json::to_value(extended_receipt.digest_preimage())
        .expect("extended receipt preimage");
    assert_ne!(base_receipt_preimage, extended_receipt_preimage);
    assert_eq!(extended_receipt_preimage[EXTENSION_NAME], extension);
    assert_eq!(
        extended_receipt_preimage["payload"][EXTENSION_NAME],
        extension
    );
    assert_eq!(
        extended_receipt_preimage["payload"]["evidence"][0]["artifact"][EXTENSION_NAME],
        extension
    );
    let review_subject = serde_json::to_value(extended_receipt.payload.review_subject_preimage())
        .expect("extended review subject");
    assert_eq!(review_subject["payload"][EXTENSION_NAME], extension);

    let base_manifest = complete_manifest();
    let base_manifest_preimage =
        serde_json::to_value(base_manifest.digest_preimage()).expect("manifest preimage");
    let mut extended_manifest_value = serde_json::to_value(&base_manifest).expect("manifest value");
    extended_manifest_value[EXTENSION_NAME] = extension.clone();
    let extended_manifest: LegacyAdrDispositionManifest =
        parse_value(extended_manifest_value).expect("extended manifest");
    let extended_manifest_preimage = serde_json::to_value(extended_manifest.digest_preimage())
        .expect("extended manifest preimage");
    assert_ne!(base_manifest_preimage, extended_manifest_preimage);
    assert_eq!(extended_manifest_preimage[EXTENSION_NAME], extension);

    let mut extended_manifest_payload_value =
        serde_json::to_value(base_manifest).expect("manifest value");
    extended_manifest_payload_value["payload"][EXTENSION_NAME] = extension.clone();
    let extended_manifest_payload: LegacyAdrDispositionManifest =
        parse_value(extended_manifest_payload_value).expect("extended manifest payload");
    let extended_manifest_payload_preimage =
        serde_json::to_value(extended_manifest_payload.digest_preimage())
            .expect("extended manifest payload preimage");
    assert_ne!(
        base_manifest_preimage, extended_manifest_payload_preimage,
        "manifest payload extensions must be content-bound"
    );
    assert_eq!(
        extended_manifest_payload_preimage["payload"][EXTENSION_NAME],
        extension
    );
}

#[test]
fn digest_preimages_are_domain_separated() {
    let proposal = proposal();
    let receipt = receipt("0001", SHA_F, "ADR 0001 has bounded evidence.", DIGEST_D);
    let manifest = complete_manifest();
    let tagged_preimages = [
        (
            serde_json::to_value(proposal.digest_preimage()).expect("proposal preimage"),
            LEGACY_ADR_DISPOSITION_PROPOSAL_SCHEMA,
            LEGACY_ADR_DISPOSITION_PROPOSAL_KIND,
        ),
        (
            serde_json::to_value(receipt.digest_preimage()).expect("receipt preimage"),
            LEGACY_ADR_DISPOSITION_RECEIPT_SCHEMA,
            LEGACY_ADR_DISPOSITION_RECEIPT_KIND,
        ),
        (
            serde_json::to_value(receipt.payload.review_subject_preimage())
                .expect("review subject preimage"),
            LEGACY_ADR_DISPOSITION_REVIEW_SUBJECT_SCHEMA,
            LEGACY_ADR_DISPOSITION_REVIEW_SUBJECT_KIND,
        ),
        (
            serde_json::to_value(manifest.digest_preimage()).expect("manifest preimage"),
            LEGACY_ADR_DISPOSITION_MANIFEST_SCHEMA,
            LEGACY_ADR_DISPOSITION_MANIFEST_KIND,
        ),
    ];

    for (index, (preimage, schema, kind)) in tagged_preimages.iter().enumerate() {
        assert_eq!(&preimage["schema"], schema);
        assert_eq!(&preimage["kind"], kind);
        for (other_preimage, _, _) in &tagged_preimages[index + 1..] {
            assert_ne!(preimage, other_preimage, "digest domains must not collide");
        }
    }
}

#[test]
fn extension_capture_keeps_known_fields_and_duplicate_keys_strict() {
    let mut wrong_known_type =
        serde_json::to_value(content("artifact.json", DIGEST_A)).expect("content binding value");
    wrong_known_type["sha256"] = json!(7);
    let wrong_known_type_error = parse_value::<ContentBinding>(wrong_known_type)
        .expect_err("known field with wrong type must fail");
    assert!(
        wrong_known_type_error
            .to_string()
            .contains("expected a string"),
        "known field failed for the wrong reason: {wrong_known_type_error}"
    );

    let duplicate_known =
        format!(r#"{{"reference":"first","reference":"second","sha256":"{DIGEST_A}"}}"#);
    let duplicate_known_error = parse_legacy_json::<ContentBinding>(duplicate_known.as_bytes())
        .expect_err("duplicate known field must fail");
    assert!(
        duplicate_known_error
            .to_string()
            .contains("duplicate object key \"reference\" is not admitted"),
        "duplicate known field failed for the wrong reason: {duplicate_known_error}"
    );

    let duplicate_extension = format!(
        r#"{{"reference":"artifact.json","sha256":"{DIGEST_A}","example.value":1,"example.value":2}}"#
    );
    let duplicate_extension_error =
        parse_legacy_json::<ContentBinding>(duplicate_extension.as_bytes())
            .expect_err("duplicate extension field must fail");
    assert!(
        duplicate_extension_error
            .to_string()
            .contains("duplicate object key \"example.value\" is not admitted"),
        "duplicate extension failed for the wrong reason: {duplicate_extension_error}"
    );

    let duplicate_nested = format!(
        r#"{{"reference":"artifact.json","sha256":"{DIGEST_A}","example.value":{{"same":1,"same":2}}}}"#
    );
    let duplicate_nested_error = parse_legacy_json::<ContentBinding>(duplicate_nested.as_bytes())
        .expect_err("duplicate nested extension key must fail");
    assert!(
        duplicate_nested_error
            .to_string()
            .contains("duplicate object key \"same\" is not admitted"),
        "duplicate nested key failed for the wrong reason: {duplicate_nested_error}"
    );
}

#[test]
fn extension_object_cannot_alias_serde_json_number_transport() {
    let object_json = format!(
        r#"{{"reference":"artifact.json","sha256":"{DIGEST_A}","example.value":{{"$serde_json::private::Number":"1"}}}}"#
    );
    let object = parse_legacy_json::<ContentBinding>(object_json.as_bytes())
        .expect("private transport token remains ordinary extension data");
    let number_json =
        format!(r#"{{"reference":"artifact.json","sha256":"{DIGEST_A}","example.value":1}}"#);
    let number =
        parse_legacy_json::<ContentBinding>(number_json.as_bytes()).expect("numeric extension");

    assert_ne!(
        object, number,
        "a legitimate extension object must not alias numeric extension content"
    );
    let canonical = serde_jcs::to_string(&object).expect("canonical extension object");
    assert!(
        canonical.contains(r#""example.value":{"$serde_json::private::Number":"1"}"#),
        "extension object shape changed during round-trip: {canonical}"
    );
}

#[test]
fn extension_numbers_normalize_only_inside_the_rfc_8785_domain() {
    assert!(matches!(
        LegacyExtensionValue::try_from_f64(f64::NAN),
        Err(LegacyExtensionNumberError::NonFinite { found }) if found == "NaN"
    ));

    let parse = |number: &str| {
        let json = format!(
            r#"{{"reference":"artifact.json","sha256":"{DIGEST_A}","example.value":{number}}}"#
        );
        parse_legacy_json::<ContentBinding>(json.as_bytes())
    };
    let canonical_one =
        serde_jcs::to_string(&parse("1").expect("canonical integer")).expect("canonical JSON");
    for equivalent in ["1.0", "1e0"] {
        let record = parse(equivalent).expect("equivalent number");
        assert_eq!(
            serde_jcs::to_string(&record).expect("canonical JSON"),
            canonical_one,
            "{equivalent} must normalize to the same semantic number as 1"
        );
    }
    let negative_zero = parse("-0").expect("negative zero is in the f64 domain");
    assert!(
        serde_jcs::to_string(&negative_zero)
            .expect("canonical negative zero")
            .contains("\"example.value\":0"),
        "RFC 8785 must normalize negative zero"
    );
    parse("0.1").expect("ordinary finite decimal remains stable");
    parse("9007199254740992").expect("exact IEEE-754 integer boundary remains stable");

    for lossy in [
        "0.10000000000000001",
        "1.0000000000000001",
        "9007199254740993",
        "18446744073709551616",
        "18446744073709551617",
        "1e-324",
    ] {
        let error = parse(lossy).expect_err("lossy number must fail closed");
        assert!(
            error
                .to_string()
                .contains("would change mathematical value under RFC 8785 canonicalization"),
            "{lossy} failed for the wrong reason: {error}"
        );
    }

    let overflow = parse("1e400").expect_err("non-finite conversion must fail closed");
    assert!(
        overflow
            .to_string()
            .contains("outside the finite IEEE-754 range required by RFC 8785"),
        "overflow failed for the wrong reason: {overflow}"
    );
}

#[test]
fn legacy_json_admission_bounds_nesting_before_typed_deserialization() {
    let nested =
        (0..=MAX_LEGACY_JSON_NESTING).fold("0".to_owned(), |value, _| format!("[{value}]"));
    let json = format!(
        r#"{{"reference":"artifact.json","sha256":"{DIGEST_A}","example.value":{nested}}}"#
    );
    let error = parse_legacy_json::<ContentBinding>(json.as_bytes())
        .expect_err("excessive nesting must fail before recursive typed decoding");
    assert!(
        error.to_string().contains("nesting exceeds"),
        "nesting failed for wrong reason: {error}"
    );
}

#[test]
fn legacy_json_admission_refuses_byte_budget_before_tree_allocation() {
    let bytes = vec![b' '; MAX_LEGACY_JSON_BYTES + 1];
    let error = parse_legacy_json::<ContentBinding>(&bytes)
        .expect_err("oversized JSON must fail before tree allocation");
    assert_eq!(
        error.to_string(),
        format!(
            "legacy disposition JSON exceeds structural limits: byte length {} exceeds {MAX_LEGACY_JSON_BYTES}",
            bytes.len()
        )
    );
}

#[test]
fn legacy_json_admission_refuses_value_token_budget_before_tree_allocation() {
    let mut bytes = Vec::with_capacity(MAX_LEGACY_JSON_TOKENS.saturating_mul(5) + 8);
    bytes.push(b'[');
    for _ in 0..MAX_LEGACY_JSON_TOKENS {
        bytes.extend_from_slice(b"null,");
    }
    bytes.extend_from_slice(b"null]");

    let error = parse_legacy_json::<LegacyExtensionValue>(&bytes)
        .expect_err("excessive value count must fail before tree allocation");
    assert_eq!(
        error.to_string(),
        format!(
            "legacy disposition JSON exceeds structural limits: token count exceeds {MAX_LEGACY_JSON_TOKENS} before tree allocation"
        )
    );
}

#[test]
fn manifest_verification_as_of_is_content_bound_canonical_utc_syntax() {
    let mut manifest = complete_manifest();
    let original_preimage =
        serde_json::to_value(manifest.digest_preimage()).expect("manifest preimage");

    // Core validates declaration shape only. The application verifier owns the
    // comparison with authorization and review times.
    manifest.payload.verification_as_of = "2025-01-01T00:00:00Z".to_owned();
    manifest
        .validate_structure()
        .expect("canonical syntax is sufficient in core");
    let changed_preimage =
        serde_json::to_value(manifest.digest_preimage()).expect("changed manifest preimage");
    assert_ne!(
        original_preimage, changed_preimage,
        "verification_as_of must participate in the manifest digest preimage"
    );

    manifest.payload.verification_as_of = "2026-08-27T00:00:00+00:00".to_owned();
    assert!(matches!(
        manifest.validate_structure(),
        Err(LegacyDispositionError::MalformedManifestVerificationTime { .. })
    ));
}

#[test]
fn proposal_is_explicitly_nonauthoritative_and_cannot_deserialize_as_receipt() {
    proposal().validate_structure().expect("valid proposal");

    let encoded = serde_json::to_value(proposal()).expect("serialize proposal");
    assert_eq!(
        encoded["schema"],
        Value::String(LEGACY_ADR_DISPOSITION_PROPOSAL_SCHEMA.to_owned())
    );
    assert_eq!(
        encoded["kind"],
        Value::String(LEGACY_ADR_DISPOSITION_PROPOSAL_KIND.to_owned())
    );
    assert!(encoded["payload"].get("review").is_none());
    assert_eq!(encoded["payload"]["warrant"]["warrant_id"], "OW-WAR-0043");
    assert!(encoded.get("receipt_digest").is_none());
    assert!(parse_value::<LegacyAdrDispositionReceipt>(encoded).is_err());
}

#[test]
fn proposal_cannot_deserialize_as_dedicated_human_review_response() {
    let encoded = serde_json::to_value(proposal()).expect("serialize proposal");
    assert!(
        parse_value::<LegacyAdrDispositionReviewResponse>(encoded).is_err(),
        "a triage proposal must never be accepted as a human response"
    );
}

#[test]
fn review_response_requires_authorized_sorted_unique_judgments() {
    review_response()
        .validate_structure()
        .expect("valid authorized response");

    let mut recommendation = review_response();
    recommendation.judgments[0].authority = JudgmentAuthority::AgentRecommendation;
    assert!(matches!(
        recommendation.validate_structure(),
        Err(LegacyDispositionError::UnauthorizedReviewJudgment { .. })
    ));

    let mut unsorted = review_response();
    unsorted.judgments.swap(0, 1);
    assert!(matches!(
        unsorted.validate_structure(),
        Err(LegacyDispositionError::ReviewJudgmentsNotCanonical)
    ));

    let mut duplicate_id = review_response();
    duplicate_id.judgments[1].id = duplicate_id.judgments[0].id.clone();
    assert!(matches!(
        duplicate_id.validate_structure(),
        Err(LegacyDispositionError::DuplicateReviewJudgmentId { .. })
    ));

    let mut duplicate_adr = review_response();
    duplicate_adr.judgments[1].adr_id = duplicate_adr.judgments[0].adr_id.clone();
    assert!(matches!(
        duplicate_adr.validate_structure(),
        Err(LegacyDispositionError::DuplicateReviewedAdr { .. })
    ));
}

#[test]
fn review_response_rejects_unknown_nested_fields_and_malformed_exact_bindings() {
    let mut unknown = serde_json::to_value(review_response()).expect("serialize response");
    unknown["judgments"][0]["unknown"] = json!(true);
    assert!(
        parse_value::<LegacyAdrDispositionReviewResponse>(unknown).is_err(),
        "unknown nested judgment field must fail closed"
    );

    let mut malformed = review_response();
    malformed.judgments[0].review_subject_digest = "sha256:approximate".to_owned();
    assert!(matches!(
        malformed.validate_structure(),
        Err(LegacyDispositionError::MalformedIdentity {
            field: "review_subject_digest",
            ..
        })
    ));

    let mut bad_time = review_response();
    bad_time.reviewed_at = "2026-08-26T12:00:00+00:00".to_owned();
    assert!(matches!(
        bad_time.validate_structure(),
        Err(LegacyDispositionError::MalformedResponseReviewTime { .. })
    ));
}

#[test]
fn receipt_digest_is_outside_payload_and_unsigned_preimage_keeps_schema_and_kind() {
    let receipt = receipt(
        "0001",
        SHA_F,
        "ADR 0001 has bounded, independently reviewable evidence.",
        DIGEST_D,
    );
    receipt.validate_structure().expect("valid receipt");

    let payload = serde_json::to_value(&receipt.payload).expect("serialize payload");
    assert!(payload.get("receipt_digest").is_none());
    let unsigned = serde_json::to_value(receipt.digest_preimage()).expect("serialize preimage");
    assert_eq!(unsigned["schema"], LEGACY_ADR_DISPOSITION_RECEIPT_SCHEMA);
    assert_eq!(unsigned["kind"], LEGACY_ADR_DISPOSITION_RECEIPT_KIND);
    assert_eq!(unsigned["payload"], payload);
    assert!(unsigned.get("receipt_digest").is_none());
}

#[test]
fn receipt_review_subject_excludes_human_review_but_binds_disposition() {
    let original = receipt(
        "0001",
        SHA_F,
        "ADR 0001 has bounded, independently reviewable evidence.",
        DIGEST_D,
    );
    let subject = serde_json::to_value(original.payload.review_subject_preimage())
        .expect("serialize review subject");
    assert_eq!(
        subject["schema"],
        LEGACY_ADR_DISPOSITION_REVIEW_SUBJECT_SCHEMA
    );
    assert_eq!(subject["kind"], LEGACY_ADR_DISPOSITION_REVIEW_SUBJECT_KIND);
    assert!(subject["payload"].get("review").is_none());
    assert_eq!(subject["payload"]["source"]["adr_id"], "0001");

    let mut changed_review = original.clone();
    changed_review.payload.review.actor = "human:a-second-recorder".to_owned();
    changed_review.payload.review.reviewed_at = "2026-08-27T12:00:00Z".to_owned();
    changed_review.payload.review.response = content("reviews/rebound.json", DIGEST_C);
    changed_review.payload.review.judgment_id = "ADR0186-0001-REBOUND".to_owned();
    assert_eq!(
        subject,
        serde_json::to_value(changed_review.payload.review_subject_preimage())
            .expect("serialize rebound subject"),
        "review metadata must not alter the subject the human judged"
    );

    let mut changed_disposition = original;
    changed_disposition.payload.disposition = LegacyAdrDisposition::Rejected;
    assert_ne!(
        subject,
        serde_json::to_value(changed_disposition.payload.review_subject_preimage())
            .expect("serialize changed subject"),
        "disposition mutation must change the human review subject"
    );
}

#[test]
fn nested_unknown_fields_are_refused_in_all_deserializable_receipt_records() {
    let base = serde_json::to_value(receipt(
        "0001",
        SHA_F,
        "ADR 0001 has bounded evidence.",
        DIGEST_D,
    ))
    .expect("serialize receipt");

    type Mutation = (&'static str, Box<dyn Fn(&mut Value)>);
    let mut mutations: Vec<Mutation> = vec![
        ("envelope", Box::new(|v| v["unknown"] = json!(true))),
        (
            "payload",
            Box::new(|v| v["payload"]["unknown"] = json!(true)),
        ),
        (
            "warrant",
            Box::new(|v| v["payload"]["warrant"]["unknown"] = json!(true)),
        ),
        (
            "source",
            Box::new(|v| v["payload"]["source"]["unknown"] = json!(true)),
        ),
        (
            "migration",
            Box::new(|v| v["payload"]["migration"]["unknown"] = json!(true)),
        ),
        (
            "review",
            Box::new(|v| v["payload"]["review"]["unknown"] = json!(true)),
        ),
        (
            "content binding",
            Box::new(|v| v["payload"]["evidence"][0]["unknown"] = json!(true)),
        ),
    ];

    for (record, mutate) in mutations.drain(..) {
        let mut value = base.clone();
        mutate(&mut value);
        assert!(
            parse_value::<LegacyAdrDispositionReceipt>(value).is_err(),
            "unknown field accepted in {record}"
        );
    }
}

#[test]
fn review_timestamp_is_real_canonical_gregorian_utc_but_not_an_authority_claim() {
    let mut record = receipt("0001", SHA_F, "ADR 0001 has bounded evidence.", DIGEST_D);
    record.payload.review.reviewed_at = "2024-02-29T23:59:59Z".to_owned();
    record.validate_structure().expect("valid leap day");

    for invalid in [
        "2023-02-29T12:00:00Z",
        "2024-04-31T12:00:00Z",
        "2024-00-01T12:00:00Z",
        "2024-13-01T12:00:00Z",
        "2024-01-00T12:00:00Z",
        "2024-01-01T24:00:00Z",
        "2024-01-01T23:60:00Z",
        "2024-01-01T23:59:60Z",
        "2024-01-01T00:00:00+00:00",
    ] {
        record.payload.review.reviewed_at = invalid.to_owned();
        assert!(matches!(
            record.validate_structure(),
            Err(LegacyDispositionError::MalformedReviewTime { .. })
        ));
    }

    // Core checks declaration shape and bindings only. It does not open the role
    // assignment or authorization artifacts, authenticate actor, or grant authority.
    record.payload.review.reviewed_at = "2026-08-26T12:00:00Z".to_owned();
    record.payload.review.actor = "human:opaque-declaration".to_owned();
    record
        .validate_structure()
        .expect("structurally valid only");
}

#[test]
fn deliberately_excluded_import_requires_explicit_not_applicable_status() {
    let binding = ImportBinding::DeliberatelyExcluded {
        source_capsule: content("artifacts/excluded/0187.json", DIGEST_C),
        imported_body_status: ImportedBodyStatus::NotApplicable,
        extensions: Default::default(),
    };
    let value = serde_json::to_value(&binding).expect("excluded import value");
    assert_eq!(value["imported_body_status"], "not_applicable");

    let mut missing = value.clone();
    missing
        .as_object_mut()
        .expect("import object")
        .remove("imported_body_status");
    let missing_error = parse_value::<ImportBinding>(missing)
        .expect_err("excluded imports must declare body status");
    assert!(
        missing_error
            .to_string()
            .contains("missing field `imported_body_status`"),
        "missing status failed for the wrong reason: {missing_error}"
    );

    let mut unknown = value;
    unknown["imported_body_status"] = json!("unmeasured");
    let unknown_error =
        parse_value::<ImportBinding>(unknown).expect_err("unknown body status must fail");
    assert!(
        unknown_error
            .to_string()
            .contains("unknown variant `unmeasured`"),
        "unknown status failed for the wrong reason: {unknown_error}"
    );

    let mut record = receipt("0187", OTHER_COMMIT, "ADR 0187 was excluded.", DIGEST_D);
    record.payload.migration = binding;
    let preimage = serde_json::to_value(record.digest_preimage()).expect("receipt preimage");
    assert_eq!(
        preimage["payload"]["migration"]["imported_body_status"], "not_applicable",
        "the explicit status must be content-bound"
    );
}

#[test]
fn resolved_warrant_successor_round_trips_and_is_digest_bound() {
    let extension = json!({"basis": "independent-resolution"});
    let mut value = serde_json::to_value(resolved_warrant_successor()).expect("successor value");
    value[EXTENSION_NAME] = extension.clone();
    let successor: LegacyAdrSuccessor =
        parse_value(value.clone()).expect("resolved Warrant successor");
    assert_eq!(
        serde_json::to_value(&successor).expect("round-trip successor"),
        value
    );

    let mut unknown = value;
    unknown["unexpected"] = json!(true);
    let error = parse_value::<LegacyAdrSuccessor>(unknown)
        .expect_err("non-namespaced successor field must fail");
    assert!(
        error
            .to_string()
            .contains("optional extensions must be namespaced"),
        "resolved successor failed for the wrong reason: {error}"
    );

    let mut record = receipt("0001", SHA_F, "ADR 0001 has bounded evidence.", DIGEST_D);
    let base_preimage = serde_json::to_value(record.digest_preimage()).expect("base preimage");
    record.payload.successors = vec![successor];
    record
        .validate_structure()
        .expect("valid resolved Warrant successor");
    let successor_preimage =
        serde_json::to_value(record.digest_preimage()).expect("successor preimage");
    assert_ne!(base_preimage, successor_preimage);
    assert_eq!(
        successor_preimage["payload"]["successors"][0]["warrant_uuid"],
        WARRANT_UUID
    );
    assert_eq!(
        successor_preimage["payload"]["successors"][0][EXTENSION_NAME],
        extension
    );
    let review_subject =
        serde_json::to_value(record.payload.review_subject_preimage()).expect("review subject");
    assert_eq!(
        review_subject["payload"]["successors"][0]["resolution_digest"],
        DIGEST_D
    );
}

#[test]
fn sealed_source_capsule_round_trips_every_field_and_rejects_unknown_fields() {
    let capsule = LegacyAdrSourceCapsule {
        schema: LEGACY_ADR_SOURCE_CAPSULE_SCHEMA.to_owned(),
        source: source("0187", OTHER_COMMIT),
        exact_source: "# ADR 0187\n\nFrozen source bytes.\n".to_owned(),
    };
    let encoded = serde_json::to_value(&capsule).expect("serialize source capsule");
    let decoded: LegacyAdrSourceCapsule =
        parse_value(encoded.clone()).expect("parse sealed source capsule");
    assert_eq!(decoded, capsule, "sealed source capsule changed fields");
    assert_eq!(
        serde_json::to_value(decoded).expect("reserialize source capsule"),
        encoded,
        "sealed source capsule did not round-trip exactly"
    );

    let mut unknown = encoded;
    unknown["unexpected"] = json!("must fail closed");
    let error = parse_value::<LegacyAdrSourceCapsule>(unknown)
        .expect_err("source capsule accepted an unknown top-level field");
    assert!(
        error.to_string().contains("unknown field `unexpected`"),
        "source capsule failed for the wrong reason: {error}"
    );
}

#[test]
fn sealed_falsification_record_round_trips_every_field_and_rejects_unknown_fields() {
    let record = LegacyAdrFalsificationRecord {
        schema: LEGACY_ADR_FALSIFICATION_SCHEMA.to_owned(),
        subject_source_sha256: DIGEST_A.to_owned(),
        statement: "No implementation artifact exists in the reviewed snapshot.".to_owned(),
        observations: vec![
            content("artifacts/search/adr-0187-paths.json", DIGEST_B),
            content("artifacts/search/adr-0187-symbols.json", DIGEST_C),
        ],
    };
    let encoded = serde_json::to_value(&record).expect("serialize falsification record");
    let decoded: LegacyAdrFalsificationRecord =
        parse_value(encoded.clone()).expect("parse sealed falsification record");
    assert_eq!(
        decoded, record,
        "sealed falsification record changed fields"
    );
    assert_eq!(
        serde_json::to_value(decoded).expect("reserialize falsification record"),
        encoded,
        "sealed falsification record did not round-trip exactly"
    );

    let mut unknown = encoded;
    unknown["unexpected"] = json!({"must": "fail closed"});
    let error = parse_value::<LegacyAdrFalsificationRecord>(unknown)
        .expect_err("falsification record accepted an unknown top-level field");
    assert!(
        error.to_string().contains("unknown field `unexpected`"),
        "falsification record failed for the wrong reason: {error}"
    );
}

#[test]
fn resolved_warrant_successor_validates_uuid_bindings_and_semantic_digest() {
    let mut invalid_uuid_value =
        serde_json::to_value(resolved_warrant_successor()).expect("successor value");
    invalid_uuid_value["warrant_uuid"] = json!("f47ac10b-58cc-4372-a567-0e02b2c3d479");
    let invalid_uuid = parse_value::<LegacyAdrSuccessor>(invalid_uuid_value)
        .expect_err("non-v7 successor identity must fail during admission");
    assert!(invalid_uuid.to_string().contains("requires UUIDv7"));

    for non_rfc_uuid_v7 in [
        "018f22c2-7d00-7cc3-18c4-dc0c0c07398f",
        "018f22c2-7d00-7cc3-d8c4-dc0c0c07398f",
    ] {
        let mut value =
            serde_json::to_value(resolved_warrant_successor()).expect("successor value");
        value["warrant_uuid"] = json!(non_rfc_uuid_v7);
        let error = parse_value::<LegacyAdrSuccessor>(value)
            .expect_err("non-RFC4122 successor identity must fail during admission");
        assert!(error.to_string().contains("requires the RFC 4122 variant"));
    }

    let mut record = receipt("0001", SHA_F, "ADR 0001 has bounded evidence.", DIGEST_D);
    let mut invalid_warrant = resolved_warrant_successor();
    let LegacyAdrSuccessor::ResolvedWarrant { warrant, .. } = &mut invalid_warrant else {
        unreachable!("resolved successor fixture")
    };
    warrant.authorized_revision = 0;
    record.payload.successors = vec![invalid_warrant];
    assert!(matches!(
        record.validate_structure(),
        Err(LegacyDispositionError::InvalidAuthorizedRevision { .. })
    ));

    let mut invalid_resolution = resolved_warrant_successor();
    let LegacyAdrSuccessor::ResolvedWarrant { resolution, .. } = &mut invalid_resolution else {
        unreachable!("resolved successor fixture")
    };
    resolution.reference.clear();
    record.payload.successors = vec![invalid_resolution];
    assert!(matches!(
        record.validate_structure(),
        Err(LegacyDispositionError::BlankField {
            field: "content_reference",
            ..
        })
    ));

    let mut invalid_digest = resolved_warrant_successor();
    let LegacyAdrSuccessor::ResolvedWarrant {
        resolution_digest, ..
    } = &mut invalid_digest
    else {
        unreachable!("resolved successor fixture")
    };
    *resolution_digest = "sha256:approximate".to_owned();
    record.payload.successors = vec![invalid_digest];
    assert!(matches!(
        record.validate_structure(),
        Err(LegacyDispositionError::MalformedIdentity {
            field: "successor_resolution_digest",
            ..
        })
    ));
}

#[test]
fn successors_require_unique_semantic_identities_and_canonical_order() {
    let adr = LegacyAdrSuccessor::Adr {
        source: source("0002", OTHER_COMMIT),
        extensions: Default::default(),
    };
    let resolved = resolved_warrant_successor();
    let mut record = receipt("0001", SHA_F, "ADR 0001 has bounded evidence.", DIGEST_D);
    record.payload.successors = vec![resolved.clone(), adr.clone()];
    assert!(matches!(
        record.validate_structure(),
        Err(LegacyDispositionError::ContentBindingsNotCanonical {
            field: "successors",
            ..
        })
    ));
    record.payload.successors.sort();
    record
        .validate_structure()
        .expect("canonical mixed successor inventory");

    let mut duplicate_adr = LegacyAdrSuccessor::Adr {
        source: source("0002", SHA_F),
        extensions: Default::default(),
    };
    if duplicate_adr == adr {
        let LegacyAdrSuccessor::Adr { source, .. } = &mut duplicate_adr else {
            unreachable!("ADR successor fixture")
        };
        source.path = "docs/decisions/0002-different.md".to_owned();
    }
    record.payload.successors = vec![adr, duplicate_adr];
    record.payload.successors.sort();
    assert!(matches!(
        record.validate_structure(),
        Err(LegacyDispositionError::DuplicateSuccessorAdr { adr_id }) if adr_id == "0002"
    ));

    let mut duplicate_warrant = resolved.clone();
    let LegacyAdrSuccessor::ResolvedWarrant { resolution, .. } = &mut duplicate_warrant else {
        unreachable!("resolved successor fixture")
    };
    resolution.reference = "docs/warrants/OW-WAR-0043/alternate-resolution.json".to_owned();
    record.payload.successors = vec![resolved, duplicate_warrant];
    record.payload.successors.sort();
    assert!(matches!(
        record.validate_structure(),
        Err(LegacyDispositionError::DuplicateSuccessorWarrant { warrant_uuid })
            if warrant_uuid == WARRANT_UUID
    ));
}

#[test]
fn receipt_requires_sorted_unique_typed_support_and_successor_for_supersession() {
    let mut record = receipt(
        "0001",
        SHA_F,
        "ADR 0001 moved to an exact successor.",
        DIGEST_D,
    );
    record.payload.disposition = LegacyAdrDisposition::Superseded;
    record.payload.evidence.clear();
    assert!(matches!(
        record.validate_structure(),
        Err(LegacyDispositionError::SuccessorRequired { .. })
    ));

    record.payload.successors = vec![
        LegacyAdrSuccessor::Adr {
            source: source("0003", OTHER_COMMIT),
            extensions: Default::default(),
        },
        LegacyAdrSuccessor::Adr {
            source: source("0002", OTHER_COMMIT),
            extensions: Default::default(),
        },
    ];
    assert!(matches!(
        record.validate_structure(),
        Err(LegacyDispositionError::ContentBindingsNotCanonical { .. })
    ));

    record.payload.successors.sort();
    record.validate_structure().expect("supported supersession");

    record
        .payload
        .successors
        .push(record.payload.successors[1].clone());
    assert!(matches!(
        record.validate_structure(),
        Err(LegacyDispositionError::DuplicateSuccessorAdr { adr_id }) if adr_id == "0003"
    ));
}

fn complete_manifest() -> LegacyAdrDispositionManifest {
    let sha_f_ids = vec!["0001".to_owned(), "0002".to_owned()];
    let off_branch_ids = vec!["0187".to_owned()];
    let mut off_branch = receipt(
        "0187",
        OTHER_COMMIT,
        "ADR 0187 was reviewed as an off-branch duplicate.",
        DIGEST_C,
    );
    off_branch.payload.migration = ImportBinding::DeliberatelyExcluded {
        source_capsule: content("artifacts/excluded/0187.json", DIGEST_C),
        imported_body_status: ImportedBodyStatus::NotApplicable,
        extensions: Default::default(),
    };

    LegacyAdrDispositionManifest {
        schema: LEGACY_ADR_DISPOSITION_MANIFEST_SCHEMA.to_owned(),
        kind: LEGACY_ADR_DISPOSITION_MANIFEST_KIND.to_owned(),
        payload: LegacyAdrDispositionManifestPayload {
            final_adr_id: "0186".to_owned(),
            warrant: warrant(),
            sha_f: SHA_F.to_owned(),
            verification_as_of: "2026-08-27T00:00:00Z".to_owned(),
            source_import: content("artifacts/lamquant-adr-import.json", DIGEST_B),
            sha_f_predecessor_ids: sha_f_ids,
            off_branch_ids,
            receipts: vec![
                receipt(
                    "0001",
                    SHA_F,
                    "ADR 0001 has its own bounded evidence.",
                    DIGEST_A,
                ),
                receipt(
                    "0002",
                    SHA_F,
                    "ADR 0002 has different bounded evidence.",
                    DIGEST_B,
                ),
                off_branch,
            ],
            extensions: Default::default(),
        },
        manifest_digest: DIGEST_D.to_owned(),
        extensions: Default::default(),
    }
}

#[test]
fn manifest_structurally_covers_supplied_inventories_without_inventing_corpus_size() {
    let manifest = complete_manifest();
    manifest
        .validate_structure()
        .expect("exact supplied coverage");

    let payload = serde_json::to_value(&manifest.payload).expect("serialize payload");
    assert!(payload.get("manifest_digest").is_none());
    let unsigned = serde_json::to_value(manifest.digest_preimage()).expect("serialize preimage");
    assert_eq!(unsigned["schema"], LEGACY_ADR_DISPOSITION_MANIFEST_SCHEMA);
    assert_eq!(unsigned["kind"], LEGACY_ADR_DISPOSITION_MANIFEST_KIND);
    assert_eq!(unsigned["payload"], payload);
    assert!(unsigned.get("manifest_digest").is_none());

    let mut missing = complete_manifest();
    missing.payload.receipts.pop();
    assert!(matches!(
        missing.validate_structure(),
        Err(LegacyDispositionError::ReceiptCoverageMismatch { .. })
    ));
}

#[test]
fn manifest_requires_sorted_receipts_and_unique_ids_digests_and_source_bindings() {
    let mut unsorted = complete_manifest();
    unsorted.payload.receipts.swap(0, 1);
    assert!(matches!(
        unsorted.validate_structure(),
        Err(LegacyDispositionError::ReceiptInventoryNotCanonical)
    ));

    let mut duplicate_id = complete_manifest();
    let mut duplicate = duplicate_id.payload.receipts[0].clone();
    duplicate.payload.source.path = "docs/decisions/0001-other.md".to_owned();
    duplicate.receipt_digest = DIGEST_C.to_owned();
    duplicate_id.payload.receipts.insert(1, duplicate);
    assert!(matches!(
        duplicate_id.validate_structure(),
        Err(LegacyDispositionError::DuplicateReceipt { .. })
    ));

    let mut duplicate_digest = complete_manifest();
    duplicate_digest.payload.receipts[1].receipt_digest = DIGEST_A.to_owned();
    assert!(matches!(
        duplicate_digest.validate_structure(),
        Err(LegacyDispositionError::DuplicateReceiptDigest { .. })
    ));

    let mut duplicate_source = complete_manifest();
    let duplicate = duplicate_source.payload.receipts[0].clone();
    duplicate_source.payload.receipts.insert(1, duplicate);
    assert!(matches!(
        duplicate_source.validate_structure(),
        Err(LegacyDispositionError::DuplicateSourceBinding { .. })
    ));
}

#[test]
fn manifest_nested_unknown_fields_are_refused() {
    let mut value = serde_json::to_value(complete_manifest()).expect("serialize manifest");
    value["payload"]["receipts"][0]["payload"]["review"]["unknown"] = json!(true);
    assert!(parse_value::<LegacyAdrDispositionManifest>(value).is_err());
}
