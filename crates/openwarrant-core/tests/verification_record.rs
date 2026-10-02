// SPDX-License-Identifier: Apache-2.0
//! The public record decoder preserves the v1 boundary and identifies v2.
use openwarrant_core::verification_record::{
    DecodedVerificationRecord, VerificationRecordError, decode,
};

const V1: &str = r#"
obligation = "OBL-001"
disposition = "established"
evidence = "receipt://test/one"
performer = "worker"
[verifier]
actor = "reviewer"
kind = "agent"
[verifier.independence]
performer_transcript_blind = false
performer_rationale_blind = false
separate_writable_workspace = false
cannot_modify_subject_artifacts = false
cannot_modify_gate_definition = false
cannot_modify_gate_fixtures = false
separate_context_compilation = false
distinct_model_required = false
distinct_human_required = false
"#;

#[test]
fn sdk_reads_bound_v2_and_preserves_unbound_v1_without_promoting_it() {
    let legacy = decode(V1).expect("frozen v1 is readable");
    assert!(matches!(legacy, DecodedVerificationRecord::Legacy(_)));
    assert_eq!(legacy.observation().obligation, "OBL-001");

    let v2 = format!(
        r#"
schema = "oh.war/verification/v2"
verification_protocol = "oh.war/verification-response/v2"
[reviewed_subject]
contract_digest = "contract-one"
[reviewed_subject.artifacts]
"src/lib.rs" = "bytes-one"
[[reviewed_packets]]
path = "verifications/bundle-one.json"
digest = "packet-one"
[verification]
{V1}
"#
    )
    .replace("[verifier]", "[verification.verifier]")
    .replace(
        "[verifier.independence]",
        "[verification.verifier.independence]",
    );
    let bound = match decode(&v2).expect("public v2 wire record is readable") {
        DecodedVerificationRecord::V2(bound) => bound,
        DecodedVerificationRecord::Legacy(_) => panic!("v2 cannot decode as legacy"),
    };
    let encoded = toml::to_string(&bound).expect("v2 SDK value serializes");
    assert!(matches!(
        decode(&encoded),
        Ok(DecodedVerificationRecord::V2(_))
    ));
    let subject = bound.reviewed_subject.clone().unwrap();
    assert!(bound.binds(&subject, &bound.reviewed_packets));
    assert!(!legacy.binds(&subject, &bound.reviewed_packets));
    let mut changed = subject.clone();
    changed.contract_digest = "contract-two".into();
    assert!(!bound.binds(&changed, &bound.reviewed_packets));
    assert!(!bound.binds(&subject, &[]));
    // Serde may ignore unknown v1 fields; the explicit format selector must not.
    let future = v2.replace("oh.war/verification/v2", "oh.war/verification/v3");
    assert!(
        matches!(decode(&future), Err(VerificationRecordError::UnsupportedSchema { schema })
        if schema == "oh.war/verification/v3")
    );
    assert!(matches!(
        decode(&format!("unexpected = true\n{v2}")),
        Err(VerificationRecordError::Malformed(_))
    ));
}

#[cfg(feature = "schema")]
#[test]
fn published_candidate_schema_is_explicit_v2_and_keeps_the_active_pack_frozen() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = std::fs::read(root.join("schemas/oh.war/verification/v2.json"))
        .expect("candidate schema is published beside v1");
    let published: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(published["$id"], "oh.war/verification/v2");
    assert_eq!(
        published["properties"]["schema"]["const"],
        "oh.war/verification/v2"
    );
    assert_eq!(published["additionalProperties"], false);
    assert!(
        published["required"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("verification"))
    );
    assert_eq!(
        published,
        serde_json::to_value(openwarrant_core::verification_record::schema()).unwrap()
    );
    use sha2::Digest;
    let v1 = std::fs::read(root.join("schemas/oh.war/verification/v1.json")).unwrap();
    assert_eq!(
        sha2::Sha256::digest(v1)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "72f0adee007391244d82d00b2346724c4cb68d8de21a39b0f59cfd538b83787c",
        "frozen v1 bytes stay unchanged"
    );
    let active: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("schemas/pack.json")).unwrap()).unwrap();
    assert_eq!(active["version"], "0.2.0", "publication is not adoption");
    assert!(active["files"].get("verification/v2").is_none());
}
