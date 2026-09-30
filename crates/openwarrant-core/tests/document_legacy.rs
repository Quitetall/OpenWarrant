// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::document::legacy::*;
use std::collections::BTreeMap;
#[test]
fn explicit_import_roundtrips_original_bytes_without_new_authority() {
    let bytes = include_bytes!("../../../conformance/sdk/document/minimal-rc3.md").to_vec();
    let files = BTreeMap::from([("draft.md".into(), bytes.clone())]);
    let bundle = capture("rc3", ADAPTER_VERSION, "draft.md", files, Limits::default()).unwrap();
    assert_eq!(bundle.files()["draft.md"], bytes);
    assert!(!bundle.report().qualification_established);
    let exported = export(&bundle, Limits::default()).unwrap();
    let restored = import(&exported, Limits::default()).unwrap();
    assert_eq!(export(&restored, Limits::default()).unwrap(), exported);
    assert_eq!(restored.files()["draft.md"], bytes);
}
#[test]
fn signed_legacy_inventory_keeps_resolution_subjects_and_refuses_tampering() {
    let source = include_bytes!("../../../conformance/sdk/legacy/resolved.json");
    let saved = import(source, Limits::default()).unwrap();
    assert_eq!(
        saved.report().original_identity,
        "01a018db-19fc-7f34-92db-54b2dca5446d"
    );
    assert_eq!(
        saved.report().original_state.as_deref(),
        Some("resolved-record-present")
    );
    let path = "docs/warrants/OW-WAR-0002/attestations/correct-1.dsse.json";
    assert_eq!(
        saved.files()[path],
        include_bytes!("../../../docs/warrants/OW-WAR-0002/attestations/correct-1.dsse.json")
    );
    assert!(!saved.report().qualification_established);
    let encoded = export(&saved, Limits::default()).unwrap();
    assert_eq!(
        export(
            &import(&encoded, Limits::default()).unwrap(),
            Limits::default()
        )
        .unwrap(),
        encoded
    );
    let mut value: serde_json::Value = serde_json::from_slice(source).unwrap();
    value["files"][path][0] = serde_json::json!(0);
    assert_eq!(
        import(&serde_json::to_vec(&value).unwrap(), Limits::default())
            .unwrap_err()
            .code,
        "legacy.integrity"
    );
}
#[test]
fn successor_keeps_both_versions_without_copying_old_authority() {
    let source = include_bytes!("../../../conformance/sdk/document/minimal-rc3.md");
    let old = capture(
        "rc3",
        ADAPTER_VERSION,
        "warrant.md",
        BTreeMap::from([
            ("warrant.md".into(), source.to_vec()),
            ("src/lib.rs".into(), b"old".to_vec()),
        ]),
        Limits::default(),
    )
    .unwrap();
    let text = String::from_utf8(source.to_vec()).unwrap();
    let next = text.replace("example:minimal", "example:successor");
    assert_ne!(next, text, "fixture ID replacement must change identity");
    let new = capture(
        "rc3",
        ADAPTER_VERSION,
        "warrant.md",
        BTreeMap::from([
            ("warrant.md".into(), next.into_bytes()),
            ("src/lib.rs".into(), b"new".to_vec()),
        ]),
        Limits::default(),
    )
    .unwrap();
    let link = check_successor(&old, &new, &["src/lib.rs"], Limits::default()).unwrap();
    assert!(!link.authority_transferred);
    assert_eq!(old.files()["src/lib.rs"], b"old");
    assert_eq!(new.files()["src/lib.rs"], b"new");
    assert_eq!(
        check_successor(&old, &old, &["src/lib.rs"], Limits::default())
            .unwrap_err()
            .code,
        "legacy.lineage"
    );
}
#[test]
fn corruption_missing_files_escaping_paths_and_unknown_editions_refuse() {
    let source = include_bytes!("../../../conformance/sdk/legacy/resolved.json");
    let original: serde_json::Value = serde_json::from_slice(source).unwrap();
    for (field, value, code) in [
        ("source_dialect", "guess-markdown", "legacy.dialect"),
        ("adapter_version", "future/99", "legacy.version"),
        ("entry", "../manifest.toml", "legacy.path"),
    ] {
        let mut v = original.clone();
        v[field] = serde_json::json!(value);
        assert_eq!(
            import(&serde_json::to_vec(&v).unwrap(), Limits::default())
                .unwrap_err()
                .code,
            code
        );
    }
    let mut missing = original.clone();
    missing["files"]
        .as_object_mut()
        .unwrap()
        .remove("docs/warrants/OW-WAR-0002/resolution.toml");
    assert_eq!(
        import(&serde_json::to_vec(&missing).unwrap(), Limits::default())
            .unwrap_err()
            .code,
        "legacy.inventory"
    );
    assert_eq!(
        import(
            source,
            Limits {
                wire_bytes: 1,
                ..Limits::default()
            }
        )
        .unwrap_err()
        .code,
        "resource-limit"
    );
    let saved = import(source, Limits::default()).unwrap();
    assert_eq!(
        export(
            &saved,
            Limits {
                wire_bytes: 1,
                ..Limits::default()
            }
        )
        .unwrap_err()
        .code,
        "resource-limit"
    );
    assert_eq!(
        import(br#"{"schema":"x","schema":"y"}"#, Limits::default())
            .unwrap_err()
            .code,
        "legacy.syntax"
    );
    assert_eq!(
        import(b"[]", Limits::default()).unwrap_err().code,
        "legacy.syntax"
    );
}
#[test]
fn unrelated_unresolved_history_does_not_gate_new_work_but_named_failure_does() {
    use openwarrant_core::document::schedule::*;
    let node = |name: &str| WorkScope {
        warrant: name.into(),
        contract_digest: format!("sha256:{}", "a".repeat(64)),
        stage: None,
    };
    let nodes = vec![node("legacy:unresolved"), node("new:work")];
    let report = evaluate_schedule(&nodes, &[], &[], ScheduleLimits::default()).unwrap();
    assert!(report.ready.contains(&1));
    let edge = Dependency {
        predecessor: 0,
        successor: 1,
        requirement: RequiredResult::ChecksPassed,
        result_digest: None,
        reason: "Explicit prerequisite".into(),
    };
    let fact = ResultFact {
        scope: nodes[0].clone(),
        requirement: RequiredResult::ChecksPassed,
        result_digest: format!("sha256:{}", "b".repeat(64)),
        state: Satisfaction::Unmet,
    };
    let blocked = evaluate_schedule(&nodes, &[edge], &[fact], ScheduleLimits::default()).unwrap();
    assert!(!blocked.ready.contains(&1));
    assert_eq!(blocked.waiting[0].state, Satisfaction::Unmet);
}
#[test]
fn historical_colon_names_are_preserved_but_drive_and_traversal_paths_refuse() {
    let source = include_bytes!("../../../conformance/sdk/document/minimal-rc3.md").to_vec();
    let signed_name = "docs/authority/sha256:5.response.sig";
    let saved = capture(
        "rc3",
        ADAPTER_VERSION,
        "warrant.md",
        BTreeMap::from([
            ("warrant.md".into(), source.clone()),
            (signed_name.into(), b"signed bytes".to_vec()),
        ]),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(saved.files()[signed_name], b"signed bytes");
    for path in [
        "C:escape",
        "C:/escape",
        "/absolute",
        "a/../escape",
        "a\\escape",
    ] {
        assert_eq!(
            capture(
                "rc3",
                ADAPTER_VERSION,
                path,
                BTreeMap::from([(path.into(), source.clone())]),
                Limits::default()
            )
            .unwrap_err()
            .code,
            "legacy.path"
        );
    }
}
#[test]
fn tiny_limits_refuse_import_before_retaining_blobs_and_metadata() {
    let source = include_bytes!("../../../conformance/sdk/legacy/resolved.json");
    for limits in [
        Limits {
            file_bytes: 1,
            ..Limits::default()
        },
        Limits {
            files: 1,
            ..Limits::default()
        },
        Limits {
            total_bytes: 1,
            ..Limits::default()
        },
        Limits {
            file_bytes: 0,
            ..Limits::default()
        },
    ] {
        assert_eq!(import(source, limits).unwrap_err().code, "resource-limit");
    }
    let huge = "x".repeat(500_000);
    let failure = capture(
        &huge,
        ADAPTER_VERSION,
        "a",
        BTreeMap::new(),
        Limits::default(),
    )
    .unwrap_err();
    assert_eq!(failure.code, "legacy.dialect");
    assert!(failure.message.len() < 100);
}
#[test]
fn oversized_json_strings_keys_and_escapes_refuse_before_decoding() {
    let limits = Limits {
        total_bytes: 1000,
        ..Limits::default()
    };
    for input in [
        format!("{{\"source_dialect\":\"{}\"}}", "x".repeat(500_000)),
        format!("{{\"{}\":0}}", "x".repeat(500_000)),
        format!("{{\"entry\":\"{}\"}}", r"\u0061".repeat(100_000)),
    ] {
        assert_eq!(
            import(input.as_bytes(), limits).unwrap_err().code,
            "resource-limit"
        );
    }
}
