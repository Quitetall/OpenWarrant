// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::document::*;

fn fields() -> DocumentFields {
    DocumentFields {
        metadata: vec![
            ("schema".into(), Dialect::Rc3.schema().into()),
            ("kind".into(), "context".into()),
            ("id".into(), "example:author".into()),
            ("revision".into(), 1.into()),
            ("title".into(), "Author é".into()),
            ("state".into(), "draft".into()),
        ],
        units: vec![AuthoredUnit {
            id: "context".into(),
            kind: UnitKind::Binding,
            text: "# Author é\n\nKeep $(shell) and `text` literal.\n".into(),
        }],
    }
}

#[test]
fn author_round_trip_and_edit_preserves_other_bytes() {
    let source =
        author_document(&fields(), AuthorOptions::default(), ParseLimits::default()).unwrap();
    assert!(source.starts_with(b"<!-- ow:unit context binding -->\n# Author"));
    let doc = parse_document(&source, Dialect::Rc3, ParseLimits::default()).unwrap();
    assert_eq!(
        validate_document(&doc, &ValidationOptions::default()).validity,
        Validity::Valid
    );
    assert_eq!(doc.unit("context"), Some(fields().units[0].text.as_str()));
    assert_eq!(
        edit_document(&doc, &[], ParseLimits::default()).unwrap(),
        source
    );
    let updated = edit_document(
        &doc,
        &[DocumentEdit::Metadata {
            key: "revision".into(),
            value: Some(2.into()),
        }],
        ParseLimits::default(),
    )
    .unwrap();
    let edited = parse_document(&updated, Dialect::Rc3, ParseLimits::default()).unwrap();
    assert_eq!(edited.unit("context"), doc.unit("context"));
    assert_eq!(edited.metadata()["revision"].as_integer(), Some(2));
    assert_eq!(doc.metadata()["revision"].as_integer(), Some(1));
}

#[test]
fn title_edit_updates_heading_preserves_crlf_and_refuses_conflict() {
    let mut f = fields();
    f.units[0].text = f.units[0].text.replace('\n', "\r\n");
    f.units.push(AuthoredUnit {
        id: "notes".into(),
        kind: UnitKind::Background,
        text: "## Notes\r\n\r\nUntouched é.\r\n".into(),
    });
    let bytes = author_document(
        &f,
        AuthorOptions {
            line_ending: LineEnding::CrLf,
            wrap_metadata: true,
        },
        ParseLimits::default(),
    )
    .unwrap();
    let doc = parse_document(&bytes, Dialect::Rc3, ParseLimits::default()).unwrap();
    let title = DocumentEdit::Metadata {
        key: "title".into(),
        value: Some("New title".into()),
    };
    let changed =
        edit_document(&doc, std::slice::from_ref(&title), ParseLimits::default()).unwrap();
    let new_doc = parse_document(&changed, Dialect::Rc3, ParseLimits::default()).unwrap();
    assert_eq!(
        new_doc.unit("context"),
        Some("# New title\r\n\r\nKeep $(shell) and `text` literal.\r\n")
    );
    assert_eq!(new_doc.unit("notes"), doc.unit("notes"));
    assert_eq!(new_doc.metadata()["id"], doc.metadata()["id"]);
    assert_eq!(new_doc.metadata()["revision"], doc.metadata()["revision"]);
    assert!(
        String::from_utf8(changed)
            .unwrap()
            .contains("<details>\r\n")
    );
    assert!(
        edit_document(
            &doc,
            &[
                title,
                DocumentEdit::Unit {
                    id: "context".into(),
                    text: "# Conflicting title\r\n\r\nBody\r\n".into()
                }
            ],
            ParseLimits::default()
        )
        .is_err()
    );
    assert_eq!(doc.original(), bytes);
}

#[test]
fn refuses_injection_duplicates_invalid_values_and_limits_without_partial_output() {
    let original = fields();
    for (key, value) in [
        ("title", MetadataValue::from("Wrong heading")),
        ("revision", MetadataValue::from(0)),
        ("state", MetadataValue::from("verified")),
        ("schema", MetadataValue::from(Dialect::Rc2.schema())),
        ("revision", MetadataValue::Float(1.5)),
        ("revision", MetadataValue::from(9_007_199_254_740_992_i64)),
    ] {
        let mut f = original.clone();
        f.metadata.retain(|(k, _)| k != key);
        f.metadata.push((key.into(), value));
        assert!(
            author_document(&f, AuthorOptions::default(), ParseLimits::default()).is_err(),
            "{key}"
        );
    }
    let mut duplicate = original.clone();
    duplicate.metadata.push(("title".into(), "Author é".into()));
    assert_eq!(
        author_document(&duplicate, AuthorOptions::default(), ParseLimits::default())
            .unwrap_err()
            .code,
        "field-duplicate"
    );
    let mut injected = original.clone();
    injected.units[0]
        .text
        .push_str("<!-- ow:unit hidden binding -->\n## Hidden\nInjected rule.\n");
    assert_eq!(
        author_document(&injected, AuthorOptions::default(), ParseLimits::default())
            .unwrap_err()
            .code,
        "unit-injection"
    );
    for tail in [
        "<!-- ow:metadata broken -->\n",
        "```\n",
        "<!-- ow:unit broken -->\n",
        "\0\n",
    ] {
        let mut f = original.clone();
        f.units[0].text.push_str(tail);
        assert!(author_document(&f, AuthorOptions::default(), ParseLimits::default()).is_err());
    }
    for limits in [
        ParseLimits {
            source_bytes: 10,
            ..ParseLimits::default()
        },
        ParseLimits {
            metadata_bytes: 10,
            ..ParseLimits::default()
        },
        ParseLimits {
            units: 0,
            ..ParseLimits::default()
        },
        ParseLimits {
            metadata_depth: 129,
            ..ParseLimits::default()
        },
    ] {
        assert_eq!(
            author_document(&original, AuthorOptions::default(), limits)
                .unwrap_err()
                .code,
            "resource-limit"
        );
    }
    let bytes =
        author_document(&original, AuthorOptions::default(), ParseLimits::default()).unwrap();
    let doc = parse_document(&bytes, Dialect::Rc3, ParseLimits::default()).unwrap();
    let edit = DocumentEdit::Metadata {
        key: "revision".into(),
        value: Some(2.into()),
    };
    assert_eq!(
        edit_document(&doc, &[edit.clone(), edit], ParseLimits::default())
            .unwrap_err()
            .code,
        "field-duplicate"
    );
    assert_eq!(
        edit_document(
            &doc,
            &[DocumentEdit::Unit {
                id: "absent".into(),
                text: "## Missing\nBody\n".into()
            }],
            ParseLimits::default()
        )
        .unwrap_err()
        .code,
        "unit-missing"
    );
    assert_eq!(doc.original(), bytes);
}

#[test]
fn preserves_unknown_extensions_compact_dependencies_and_fenced_markers() {
    let mut f = fields();
    f.units[0].text.push_str(
        "```sh\n<!-- ow:unit fake binding -->\n<!-- ow:metadata -->\n$(touch /not-executed)\n```\n",
    );
    let extra: MetadataValue = "extensions = { 'example:extra' = { text = 'quotes \" backslash \\ shell $()' } }\ndependencies = [{unit='context', target='#context'}, {unit='context', target='other.md'}]\nrequires_extensions = ['example:extra']\n".parse().unwrap();
    for (k, v) in extra.as_table().unwrap() {
        f.metadata.push((k.clone(), v.clone()));
    }
    let bytes = author_document(&f, AuthorOptions::default(), ParseLimits::default()).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("[[dependencies]]"));
    let doc = parse_document(&bytes, Dialect::Rc3, ParseLimits::default()).unwrap();
    assert_eq!(doc.metadata()["extensions"], extra["extensions"]);
    assert_eq!(doc.metadata()["dependencies"], extra["dependencies"]);
    assert_eq!(
        validate_document(&doc, &ValidationOptions::default()).semantic_support,
        SemanticSupport::Unsupported
    );
    let edited = edit_document(
        &doc,
        &[DocumentEdit::Unit {
            id: "context".into(),
            text: "# Author é\n\nChanged only unit.\n".into(),
        }],
        ParseLimits::default(),
    )
    .unwrap();
    let result = parse_document(&edited, Dialect::Rc3, ParseLimits::default()).unwrap();
    assert_eq!(&edited[result.metadata_span()], &bytes[doc.metadata_span()]);
    assert_eq!(result.metadata(), doc.metadata());
}

#[test]
fn explicit_rc2_edits_keep_adapter_and_original_bytes() {
    let original = include_bytes!("../../../conformance/sdk/document/minimal-rc2.md");
    let doc = parse_document(original, Dialect::Rc2, ParseLimits::default()).unwrap();
    assert_eq!(
        edit_document(&doc, &[], ParseLimits::default()).unwrap(),
        original
    );
    let changed = edit_document(
        &doc,
        &[DocumentEdit::Metadata {
            key: "revision".into(),
            value: Some(2.into()),
        }],
        ParseLimits::default(),
    )
    .unwrap();
    assert!(changed.starts_with(b"+++\n"));
    let edited = parse_document(&changed, Dialect::Rc2, ParseLimits::default()).unwrap();
    for unit in doc.units() {
        assert_eq!(edited.unit(&unit.id), doc.unit(&unit.id));
    }
    assert!(parse_document(&changed, Dialect::Rc3, ParseLimits::default()).is_err());
    assert_eq!(doc.original(), original);
}

#[test]
fn edit_batch_is_bounded_before_copying_values() {
    let bytes =
        author_document(&fields(), AuthorOptions::default(), ParseLimits::default()).unwrap();
    let doc = parse_document(&bytes, Dialect::Rc3, ParseLimits::default()).unwrap();
    let limits = ParseLimits {
        source_bytes: bytes.len(),
        ..ParseLimits::default()
    };
    let edits = [
        DocumentEdit::Metadata {
            key: "alias".into(),
            value: Some("x".repeat(bytes.len() / 2).into()),
        },
        DocumentEdit::Metadata {
            key: "title".into(),
            value: Some("y".repeat(bytes.len() / 2).into()),
        },
    ];
    assert_eq!(
        edit_document(&doc, &edits, limits).unwrap_err().code,
        "resource-limit"
    );
    assert_eq!(doc.original(), bytes);
}
