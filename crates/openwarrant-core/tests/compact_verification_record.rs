// SPDX-License-Identifier: Apache-2.0
//! Candidate metadata bindings, not file validation or independent assurance.
use openwarrant_core::compact_verification_record::{
    self as compact, ReviewedView, StoredCompactVerification,
};
use openwarrant_core::verification_record::{
    self as legacy, ReviewedPacket, ReviewedSubject, VerificationRecordError,
};

fn candidate() -> StoredCompactVerification {
    let observation = legacy::decode(
        r#"
obligation = "OBL-001"
disposition = "not_established"
evidence = "synthetic codec fixture; no execution observation"
performer = "fixture-worker"
[verifier]
actor = "fixture-reviewer"
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
"#,
    )
    .unwrap()
    .observation()
    .clone();
    StoredCompactVerification {
        schema: compact::SCHEMA.into(),
        verification_protocol: compact::RESPONSE_SCHEMA.into(),
        verification: observation,
        reviewed_subject: serde_json::from_value::<ReviewedSubject>(serde_json::json!({"contract_digest":"contract-one","artifacts":{"src/lib.rs":"source-one"}})).unwrap(),
        reviewed_packets: vec![ReviewedPacket { path: "verifications/bundle-aaaaaaaaaaaaaaaa.json".into(), digest: "a".repeat(64) }],
        reviewed_views: vec![ReviewedView { path: format!("verifications/responses/views/view-{}.json", "b".repeat(64)), sha256: "b".repeat(64) }],
    }
}

fn response_text() -> String {
    let mut value = toml::Value::try_from(candidate()).unwrap();
    let root = value.as_table_mut().unwrap();
    let observation = root.remove("verification").unwrap();
    root.remove("verification_protocol");
    root.insert("schema".into(), compact::RESPONSE_SCHEMA.into());
    root.insert("warrant".into(), "FIX-WAR-0001".into());
    root.insert(
        "verifications".into(),
        toml::Value::Array(vec![observation]),
    );
    toml::to_string(&value).unwrap()
}

#[test]
fn compact_response_prepares_bound_observations_without_promoting_them() {
    let text = response_text();
    let response = compact::decode_response(&text).unwrap();
    let records: Vec<_> = response.prepare_records().unwrap().collect();
    assert_eq!(records.len(), 1);
    let expected = candidate();
    assert_eq!(records[0].reviewed_subject, expected.reviewed_subject);
    assert_eq!(records[0].reviewed_packets, expected.reviewed_packets);
    assert_eq!(records[0].reviewed_views, expected.reviewed_views);
    assert_eq!(records[0].verification_protocol, compact::RESPONSE_SCHEMA);
    assert_eq!(records[0].verification.obligation, "OBL-001");
    assert_eq!(
        serde_json::to_value(&records[0].verification).unwrap(),
        serde_json::to_value(&expected.verification).unwrap()
    );
    assert!(matches!(
        legacy::decode(&toml::to_string(&records[0]).unwrap()),
        Err(VerificationRecordError::UnsupportedSchema { .. })
    ));
    for kind in [
        "duplicate",
        "missing-view",
        "invalid-hash",
        "no-observations",
        "no-warrant",
    ] {
        let mut response = compact::decode_response(&text).unwrap();
        match kind {
            "duplicate" => response
                .verifications
                .push(response.verifications[0].clone()),
            "missing-view" => response.reviewed_views.clear(),
            "invalid-hash" => response.reviewed_views[0].sha256 = "invalid".into(),
            "no-observations" => response.verifications.clear(),
            "no-warrant" => response.warrant.clear(),
            _ => unreachable!(),
        }
        assert!(response.prepare_records().is_err(), "{kind}");
    }
    assert!(
        compact::decode_response(&format!("performer_rationale = 'trust me'\n{text}")).is_err()
    );
    for schema in [legacy::RESPONSE_SCHEMA, "oh.war/verification-response/v4"] {
        let input = text.replace(compact::RESPONSE_SCHEMA, schema);
        assert!(matches!(
            compact::decode_response(&input),
            Err(VerificationRecordError::UnsupportedSchema { .. })
        ));
    }
}

#[cfg(feature = "schema")]
#[test]
fn response_schema_is_published_without_adopting_the_protocol() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let published: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("schemas/oh.war/verification-response/v3.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(published["$id"], compact::RESPONSE_SCHEMA);
    assert_eq!(
        published["properties"]["schema"]["const"],
        compact::RESPONSE_SCHEMA
    );
    assert_eq!(published["additionalProperties"], false);
    for key in [
        "warrant",
        "reviewed_subject",
        "reviewed_packets",
        "reviewed_views",
        "verifications",
    ] {
        assert!(
            published["required"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!(key))
        );
    }
    assert_eq!(
        published,
        serde_json::to_value(compact::response_schema()).unwrap()
    );
    let pack: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("schemas/pack.json")).unwrap()).unwrap();
    assert_eq!(pack["version"], "0.2.0");
    assert!(pack["files"].get("verification-response/v3").is_none());
}

#[test]
fn compact_codec_preserves_both_reference_sets_and_old_reader_refusal() {
    let record = candidate();
    let text = toml::to_string(&record).unwrap();
    let decoded = compact::decode(&text).unwrap();
    assert_eq!(decoded.reviewed_views, record.reviewed_views);
    assert_eq!(decoded.reviewed_packets, record.reviewed_packets);
    assert!(decoded.binds(
        &record.reviewed_subject,
        &record.reviewed_packets,
        &record.reviewed_views
    ));
    assert!(
        matches!(legacy::decode(&text), Err(VerificationRecordError::UnsupportedSchema { schema }) if schema == compact::SCHEMA)
    );
    assert!(
        toml::from_str::<openwarrant_core::verification::Verification>(&text).is_err(),
        "the bare v1 reader must not mistake an envelope for a verdict"
    );
    let root: toml::Value = toml::from_str(&text).unwrap();
    assert!(root.get("obligation").is_none());
    assert_eq!(
        root["verification"]["disposition"].as_str(),
        Some("not_established")
    );
}

#[test]
fn matching_metadata_requires_exact_nonempty_distinct_view_and_archive_references() {
    let expected = candidate();
    let matches = |r: &StoredCompactVerification| {
        r.binds(
            &expected.reviewed_subject,
            &expected.reviewed_packets,
            &expected.reviewed_views,
        )
    };
    let mut changed = expected.clone();
    changed.reviewed_subject.contract_digest = "different-contract".into();
    assert!(!matches(&changed));
    changed = expected.clone();
    changed.reviewed_views[0].sha256 = "c".repeat(64);
    assert!(!matches(&changed));
    changed = expected.clone();
    changed.reviewed_packets[0].digest = "d".repeat(64);
    assert!(!matches(&changed));
    assert!(!expected.binds(&expected.reviewed_subject, &expected.reviewed_packets, &[]));
    assert!(!expected.binds(&expected.reviewed_subject, &[], &expected.reviewed_views));
    for which in [
        "duplicate-view",
        "duplicate-archive",
        "malformed-view-hash",
        "same-file",
        "wrong-protocol",
    ] {
        let mut r = expected.clone();
        match which {
            "duplicate-view" => r.reviewed_views.push(r.reviewed_views[0].clone()),
            "duplicate-archive" => r.reviewed_packets.push(r.reviewed_packets[0].clone()),
            "malformed-view-hash" => r.reviewed_views[0].sha256 = "not-a-sha256".into(),
            "same-file" => r.reviewed_views[0].path = r.reviewed_packets[0].path.clone(),
            "wrong-protocol" => r.verification_protocol = legacy::RESPONSE_SCHEMA.into(),
            _ => unreachable!(),
        }
        assert!(
            !r.binds(&r.reviewed_subject, &r.reviewed_packets, &r.reviewed_views),
            "{which}"
        );
    }
    for path in [
        "../escape",
        "/absolute",
        "file\\name",
        "https://example.invalid/view",
        "view#fragment",
    ] {
        let mut r = expected.clone();
        r.reviewed_views[0].path = path.into();
        assert!(
            !r.binds(&r.reviewed_subject, &r.reviewed_packets, &r.reviewed_views),
            "{path}"
        );
    }
}

#[test]
fn compact_codec_refuses_unknown_fields_and_future_formats() {
    let text = toml::to_string(&candidate()).unwrap();
    assert!(matches!(
        compact::decode(&format!("performer_narrative = 'trust me'\n{text}")),
        Err(VerificationRecordError::Malformed(_))
    ));
    let future = text.replace(compact::SCHEMA, "oh.war/verification/v4");
    assert!(
        matches!(compact::decode(&future), Err(VerificationRecordError::UnsupportedSchema { schema }) if schema == "oh.war/verification/v4")
    );
    assert!(matches!(
        compact::decode("schema = 3"),
        Err(VerificationRecordError::Malformed(_))
    ));
    let mut missing: toml::Value = toml::from_str(&text).unwrap();
    missing.as_table_mut().unwrap().remove("reviewed_views");
    assert!(matches!(
        compact::decode(&toml::to_string(&missing).unwrap()),
        Err(VerificationRecordError::Malformed(_))
    ));
}

#[cfg(feature = "schema")]
#[test]
fn candidate_schema_requires_both_reference_sets_without_activating_the_pack() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let published: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("schemas/oh.war/verification/v3.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(published["$id"], compact::SCHEMA);
    assert_eq!(published["properties"]["schema"]["const"], compact::SCHEMA);
    assert_eq!(published["additionalProperties"], false);
    for field in [
        "verification",
        "reviewed_subject",
        "reviewed_packets",
        "reviewed_views",
    ] {
        assert!(
            published["required"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!(field))
        );
    }
    assert_eq!(published, serde_json::to_value(compact::schema()).unwrap());
    let pack: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("schemas/pack.json")).unwrap()).unwrap();
    assert_eq!(pack["version"], "0.2.0");
    assert!(pack["files"].get("verification/v3").is_none());
}
