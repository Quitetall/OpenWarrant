// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::document::Dialect;
use openwarrant_core::document::source::BoundReference;
use openwarrant_core::document::source::check_sources;
use openwarrant_core::document::source::{
    Holder, HolderKind, SourceLimits, SourceMetadata, describe_source,
};
use openwarrant_core::document::source::{
    decode_bound_reference, decode_source_descriptor, encode_bound_reference,
    encode_source_descriptor,
};
use std::collections::BTreeMap;

fn holder() -> Holder {
    Holder {
        kind: HolderKind::Example,
        locator: "example:fixture".into(),
        commit: None,
    }
}

#[test]
fn parsed_identity_comes_from_validated_original_document() {
    let bytes = include_bytes!("../../../conformance/sdk/document/minimal-rc3.md");
    let d = describe_source(
        "task.md",
        holder(),
        SourceMetadata::unestablished("untrusted", "internal"),
        bytes,
        Some(Dialect::Rc3),
        SourceLimits::default(),
    )
    .unwrap();
    let identity = d.document.unwrap();
    assert_eq!(identity.schema, "oh.war/document/1.0.0-rc.3");
    assert_eq!(identity.kind, "warrant");
    assert_eq!(identity.revision, 1);
    assert_eq!(d.byte_length, bytes.len());
    assert_eq!(
        describe_source(
            "task.md",
            holder(),
            SourceMetadata::unestablished("untrusted", "internal"),
            b"not a document",
            Some(Dialect::Rc3),
            SourceLimits::default()
        )
        .unwrap_err()
        .code,
        "source-invalid"
    );
}

#[test]
fn source_descriptions_refuse_unsafe_paths_and_invalid_provenance() {
    for path in [
        "../secret",
        "/root",
        "dir//file",
        "./file",
        "C:/file",
        "dir\\file",
        "file#unit",
        "",
        "a\nb",
    ] {
        assert_eq!(
            describe_source(
                path,
                holder(),
                SourceMetadata::unestablished("untrusted", "internal"),
                b"abc",
                None,
                SourceLimits::default()
            )
            .unwrap_err()
            .code,
            "target-invalid",
            "{path:?}"
        );
    }
    let mut bad = holder();
    bad.commit = Some("main".into());
    assert_eq!(
        describe_source(
            "file",
            bad,
            SourceMetadata::unestablished("untrusted", "internal"),
            b"abc",
            None,
            SourceLimits::default()
        )
        .unwrap_err()
        .code,
        "source-invalid"
    );
    let mut metadata = SourceMetadata::unestablished("untrusted", "internal");
    metadata.authority = "verified".into();
    assert_eq!(
        describe_source(
            "file",
            holder(),
            metadata,
            b"abc",
            None,
            SourceLimits::default()
        )
        .unwrap_err()
        .code,
        "source-invalid"
    );
}

#[test]
fn snapshot_table_checks_actual_blobs_and_rejects_conflicting_claims() {
    let bytes = include_bytes!("../../../conformance/sdk/document/minimal-rc3.md");
    let descriptor = describe_source(
        "task.md",
        holder(),
        SourceMetadata::unestablished("untrusted", "internal"),
        bytes,
        Some(Dialect::Rc3),
        SourceLimits::default(),
    )
    .unwrap();
    let mut blobs = BTreeMap::from([(descriptor.source_digest.clone(), bytes.to_vec())]);
    let mut rows = vec![descriptor];
    assert_eq!(
        check_sources(&rows, &blobs, SourceLimits::default())
            .unwrap()
            .len(),
        1
    );
    rows[0].document.as_mut().unwrap().id = "example:forged".into();
    assert_eq!(
        check_sources(&rows, &blobs, SourceLimits::default())
            .unwrap_err()
            .code,
        "source-mismatch"
    );
    rows[0].document.as_mut().unwrap().id = "example:minimal-signup".into();
    blobs.values_mut().next().unwrap()[0] = b'!';
    assert_eq!(
        check_sources(&rows, &blobs, SourceLimits::default())
            .unwrap_err()
            .code,
        "digest-mismatch"
    );
    blobs.clear();
    assert_eq!(
        check_sources(&rows, &blobs, SourceLimits::default())
            .unwrap_err()
            .code,
        "source-missing"
    );
}

#[test]
fn bound_reference_must_match_named_unit_exactly() {
    let bytes = include_bytes!("../../../conformance/sdk/document/minimal-rc3.md");
    let descriptor = describe_source(
        "task.md",
        holder(),
        SourceMetadata::unestablished("untrusted", "internal"),
        bytes,
        Some(Dialect::Rc3),
        SourceLimits::default(),
    )
    .unwrap();
    let blobs = BTreeMap::from([(descriptor.source_digest.clone(), bytes.to_vec())]);
    let mut reference = BoundReference {
        source_digest: descriptor.source_digest.clone(),
        unit: "outcome".into(),
        start: 34,
        end: 160,
    };
    // Byte ranges are independently specified by the literal fixture content.
    let text = std::str::from_utf8(bytes).unwrap();
    reference.start = text.find("# Make signup easier").unwrap();
    reference.end = text.find("<!-- ow:unit scope").unwrap();
    let rows = [descriptor];
    let checked = check_sources(&rows, &blobs, SourceLimits::default()).unwrap();
    assert_eq!(checked.check_reference(&reference).unwrap(), b"# Make signup easier\n\n## Desired outcome\n\nReduce the information a new user must enter to create an account.\n\n");
    reference.start += 1;
    assert_eq!(
        checked.check_reference(&reference).unwrap_err().code,
        "range-mismatch"
    );
    reference.unit = "missing".into();
    assert_eq!(
        checked.check_reference(&reference).unwrap_err().code,
        "target-missing"
    );
    reference.unit = "*".into();
    reference.start = 0;
    reference.end = bytes.len();
    assert_eq!(checked.check_reference(&reference).unwrap(), bytes);
    reference.source_digest = format!("sha256:{}", "0".repeat(64));
    assert_eq!(
        checked.check_reference(&reference).unwrap_err().code,
        "source-missing"
    );
}

#[test]
fn source_lock_is_order_independent_and_refuses_ambiguous_capture() {
    let descriptor = |path: &str, bytes: &[u8]| {
        describe_source(
            path,
            holder(),
            SourceMetadata::unestablished("untrusted", "internal"),
            bytes,
            None,
            SourceLimits::default(),
        )
        .unwrap()
    };
    let a = descriptor("a", b"abc");
    let b = descriptor("b", b"xyz");
    let blobs = BTreeMap::from([
        (a.source_digest.clone(), b"abc".to_vec()),
        (b.source_digest.clone(), b"xyz".to_vec()),
    ]);
    let first = [a.clone(), b.clone()];
    let second = [b.clone(), a.clone()];
    assert_eq!(
        check_sources(&first, &blobs, SourceLimits::default())
            .unwrap()
            .canonical_lock()
            .unwrap(),
        check_sources(&second, &blobs, SourceLimits::default())
            .unwrap()
            .canonical_lock()
            .unwrap()
    );
    assert_eq!(
        check_sources(&[a.clone(), a], &blobs, SourceLimits::default())
            .unwrap_err()
            .code,
        "source-ambiguous"
    );
    let original = include_bytes!("../../../conformance/sdk/document/minimal-rc3.md");
    let changed = std::str::from_utf8(original)
        .unwrap()
        .replace("Payment", "Payments");
    let a = describe_source(
        "old.md",
        holder(),
        SourceMetadata::unestablished("untrusted", "internal"),
        original,
        Some(Dialect::Rc3),
        SourceLimits::default(),
    )
    .unwrap();
    let b = describe_source(
        "other.md",
        holder(),
        SourceMetadata::unestablished("untrusted", "internal"),
        changed.as_bytes(),
        Some(Dialect::Rc3),
        SourceLimits::default(),
    )
    .unwrap();
    let blobs = BTreeMap::from([
        (a.source_digest.clone(), original.to_vec()),
        (b.source_digest.clone(), changed.into_bytes()),
    ]);
    assert_eq!(
        check_sources(&[a, b], &blobs, SourceLimits::default())
            .unwrap_err()
            .code,
        "source-ambiguous"
    );
}

#[test]
fn limits_cover_empty_calls_metadata_total_bytes_and_output() {
    let defaults = SourceLimits::default();
    assert_eq!(
        describe_source(
            "file",
            holder(),
            SourceMetadata::unestablished("untrusted", "internal"),
            b"abc",
            None,
            SourceLimits {
                total_bytes: 2,
                ..defaults
            }
        )
        .unwrap_err()
        .code,
        "resource-limit"
    );
    assert_eq!(
        check_sources(
            &[],
            &BTreeMap::new(),
            SourceLimits {
                sources: 0,
                ..defaults
            }
        )
        .unwrap_err()
        .code,
        "resource-limit"
    );
    assert_eq!(
        describe_source(
            "file",
            holder(),
            SourceMetadata::unestablished(&"x".repeat(2048), "internal"),
            b"abc",
            None,
            SourceLimits {
                metadata_bytes: 1024,
                ..defaults
            }
        )
        .unwrap_err()
        .code,
        "resource-limit"
    );
    let d = describe_source(
        "file",
        holder(),
        SourceMetadata::unestablished("untrusted", "internal"),
        b"abc",
        None,
        defaults,
    )
    .unwrap();
    let blobs = BTreeMap::from([(d.source_digest.clone(), b"abc".to_vec())]);
    let rows = [d];
    assert_eq!(
        check_sources(
            &rows,
            &blobs,
            SourceLimits {
                total_bytes: 2,
                ..defaults
            }
        )
        .unwrap_err()
        .code,
        "resource-limit"
    );
    assert_eq!(
        check_sources(
            &rows,
            &blobs,
            SourceLimits {
                output_bytes: 2,
                ..defaults
            }
        )
        .unwrap()
        .canonical_lock()
        .unwrap_err()
        .code,
        "resource-limit"
    );
}

#[test]
fn codecs_refuse_duplicate_fields_wrong_domains_and_unsafe_ranges() {
    let limits = SourceLimits::default();
    let d = describe_source(
        "file",
        holder(),
        SourceMetadata::unestablished("untrusted", "internal"),
        b"abc",
        None,
        limits,
    )
    .unwrap();
    let encoded = encode_source_descriptor(&d, limits).unwrap();
    assert_eq!(decode_source_descriptor(&encoded, limits).unwrap(), d);
    let text = String::from_utf8(encoded).unwrap();
    let duplicate = text.replacen(
        "\"path\":\"file\"",
        "\"path\":\"file\",\"path\":\"other\"",
        1,
    );
    assert_eq!(
        decode_source_descriptor(duplicate.as_bytes(), limits)
            .unwrap_err()
            .code,
        "source-invalid"
    );
    let wrong_domain = text.replace("sha256:", "sha512:");
    assert_eq!(
        decode_source_descriptor(wrong_domain.as_bytes(), limits)
            .unwrap_err()
            .code,
        "digest-invalid"
    );
    let source_digest = &d.source_digest;
    let reference = serde_json::json!({"source_digest":source_digest,"unit":"*","start":3,"end":2});
    assert_eq!(
        decode_bound_reference(&serde_json::to_vec(&reference).unwrap(), limits)
            .unwrap_err()
            .code,
        "range-mismatch"
    );
    let reference = serde_json::json!({"source_digest":source_digest,"unit":"*","start":0,"end":3,"verified":true});
    assert_eq!(
        decode_bound_reference(&serde_json::to_vec(&reference).unwrap(), limits)
            .unwrap_err()
            .code,
        "source-invalid"
    );
}

#[test]
fn reference_encoding_matches_canonical_vector_and_retains_exact_revision() {
    let reference = BoundReference {
        source_digest: "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            .into(),
        unit: "*".into(),
        start: 0,
        end: 3,
    };
    assert_eq!(encode_bound_reference(&reference, SourceLimits::default()).unwrap(), br#"{"end":3,"source_digest":"sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad","start":0,"unit":"*"}"#);
    let old = include_str!("../../../conformance/sdk/document/minimal-rc3.md");
    let new = old.replace("Make signup easier", "Simplify signup");
    let describe = |bytes: &[u8]| {
        describe_source(
            "task.md",
            holder(),
            SourceMetadata::unestablished("untrusted", "internal"),
            bytes,
            Some(Dialect::Rc3),
            SourceLimits::default(),
        )
        .unwrap()
    };
    let old_descriptor = describe(old.as_bytes());
    let new_descriptor = describe(new.as_bytes());
    assert_eq!(old_descriptor.document, new_descriptor.document);
    assert_ne!(old_descriptor.source_digest, new_descriptor.source_digest);
    let old_reference = BoundReference {
        source_digest: old_descriptor.source_digest,
        unit: "*".into(),
        start: 0,
        end: old.len(),
    };
    let blobs = BTreeMap::from([(new_descriptor.source_digest.clone(), new.into_bytes())]);
    let rows = [new_descriptor];
    assert_eq!(
        check_sources(&rows, &blobs, SourceLimits::default())
            .unwrap()
            .check_reference(&old_reference)
            .unwrap_err()
            .code,
        "source-missing"
    );
}

#[test]
fn one_digest_cannot_have_incompatible_parsed_and_opaque_interpretations() {
    let bytes = include_bytes!("../../../conformance/sdk/document/minimal-rc3.md");
    let parsed = describe_source(
        "parsed.md",
        holder(),
        SourceMetadata::unestablished("untrusted", "internal"),
        bytes,
        Some(Dialect::Rc3),
        SourceLimits::default(),
    )
    .unwrap();
    let opaque = describe_source(
        "opaque.md",
        holder(),
        SourceMetadata::unestablished("untrusted", "internal"),
        bytes,
        None,
        SourceLimits::default(),
    )
    .unwrap();
    let blobs = BTreeMap::from([(parsed.source_digest.clone(), bytes.to_vec())]);
    assert_eq!(
        check_sources(&[parsed, opaque], &blobs, SourceLimits::default())
            .unwrap_err()
            .code,
        "source-ambiguous"
    );
}

#[test]
fn aliases_share_checked_bytes_but_each_length_claim_is_checked() {
    let first = describe_source(
        "first",
        holder(),
        SourceMetadata::unestablished("untrusted", "internal"),
        b"abc",
        None,
        SourceLimits::default(),
    )
    .unwrap();
    let mut second = first.clone();
    second.path = "second".into();
    let blobs = BTreeMap::from([(first.source_digest.clone(), b"abc".to_vec())]);
    assert_eq!(
        check_sources(
            &[first.clone(), second.clone()],
            &blobs,
            SourceLimits::default()
        )
        .unwrap()
        .len(),
        2
    );
    second.byte_length = 2;
    assert_eq!(
        check_sources(&[first, second], &blobs, SourceLimits::default())
            .unwrap_err()
            .code,
        "source-mismatch"
    );
}

#[test]
fn supplied_opaque_bytes_get_exact_identity_without_io_or_authority() {
    let descriptor = describe_source(
        "fixtures/input.bin",
        Holder {
            kind: HolderKind::Example,
            locator: "example:fixture".into(),
            commit: None,
        },
        SourceMetadata::unestablished("untrusted", "internal"),
        b"abc",
        None,
        SourceLimits::default(),
    )
    .unwrap();
    assert_eq!(
        descriptor.source_digest,
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(descriptor.byte_length, 3);
    assert!(descriptor.document.is_none());
    assert_eq!(descriptor.metadata.authority, "unestablished");
}
