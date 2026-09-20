// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::document::{Dialect, ParseLimits, parse_document};

const MINIMAL: &[u8] = include_bytes!("../../../conformance/sdk/document/minimal-rc3.md");

#[test]
fn minimal_document_preserves_original_bytes_and_exact_units() {
    let document = parse_document(MINIMAL, Dialect::Rc3, ParseLimits::default()).unwrap();
    assert_eq!(document.original(), MINIMAL);
    assert_eq!(
        document.metadata()["title"].as_str(),
        Some("Make signup easier")
    );
    assert_eq!(document.units().len(), 3);
    assert_eq!(
        document.unit("scope"),
        Some(
            "## Scope\n\nExplore the signup form. Payment and account deletion are outside this draft.\n\n"
        )
    );
    assert!(
        document
            .unit("context")
            .unwrap()
            .ends_with("recorded here.\n\n")
    );
    assert_eq!(document.unit("absent"), None);
}

#[test]
fn metadata_subset_refuses_multiline_and_nonportable_values_before_producing_document() {
    let original = std::str::from_utf8(MINIMAL).unwrap();
    for payload in [
        "1.5",
        "-1",
        "9007199254740992",
        "1979-05-27",
        "\"\"\"multi\nline\"\"\"",
        "'''multi\nline'''",
        "[true, { nested = -2 }]",
    ] {
        let input = original.replace(
            "state = \"draft\"",
            &format!("state = \"draft\"\n[extensions]\n\"example:value\" = {payload}"),
        );
        let error =
            parse_document(input.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap_err();
        assert_eq!(error.code, "source-invalid", "{payload}");
        assert!(error.span.start < input.len());
    }
    let valid = original.replace("state = \"draft\"", "state = \"draft\"\n# \"\"\" comment is not a string\n[extensions]\n\"example:value\" = [true, { nested = 9007199254740991 }]");
    assert!(parse_document(valid.as_bytes(), Dialect::Rc3, ParseLimits::default()).is_ok());
}

#[test]
fn validation_distinguishes_kind_requirements_from_external_readiness() {
    use openwarrant_core::document::{Evaluation, ValidationOptions, Validity, validate_document};
    let document = parse_document(MINIMAL, Dialect::Rc3, ParseLimits::default()).unwrap();
    let report = validate_document(&document, &ValidationOptions::default());
    assert_eq!(report.validity, Validity::Valid);
    assert_eq!(report.context_resolution, Evaluation::NotEvaluated);
    assert_eq!(report.readiness, Evaluation::NotEvaluated);
    let original = std::str::from_utf8(MINIMAL).unwrap();
    for (before, after, expected) in [
        (
            "outcome binding",
            "unrelated binding",
            "Required binding unit outcome",
        ),
        (
            "scope binding",
            "scope background",
            "Required binding unit scope",
        ),
        ("state = \"draft\"", "state = \"accepted\"", "state"),
        ("revision = 1", "revision = 0", "revision"),
        (
            "id = \"example:minimal-signup\"",
            "id = \"NotQualified\"",
            "id",
        ),
        ("kind = \"warrant\"", "kind = \"adr\"", "decision"),
        (
            "state = \"draft\"",
            "state = \"draft\"\nunknown = true",
            "Unknown core field",
        ),
        (
            "# Make signup easier",
            "# A different visible title",
            "title",
        ),
    ] {
        let input = original.replace(before, after);
        let document =
            parse_document(input.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap();
        let report = validate_document(&document, &ValidationOptions::default());
        assert_eq!(report.validity, Validity::Invalid, "{after}");
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.message.contains(expected)),
            "{report:?}"
        );
    }
}

#[test]
fn unknown_extensions_preserve_content_and_block_only_required_interpretation() {
    use openwarrant_core::document::{
        SemanticSupport, ValidationOptions, Validity, validate_document,
    };
    let source = std::str::from_utf8(MINIMAL).unwrap().replace(
        "state = \"draft\"",
        "state = \"draft\"\n[extensions]\n\"example:future\" = { detail = \"retain me\" }",
    );
    let document = parse_document(source.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap();
    let report = validate_document(&document, &ValidationOptions::default());
    assert_eq!(report.validity, Validity::Valid);
    assert_eq!(report.semantic_support, SemanticSupport::Supported);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "extension-unknown")
    );
    assert_eq!(
        document.metadata()["extensions"]["example:future"]["detail"].as_str(),
        Some("retain me")
    );
    let required = source.replace(
        "state = \"draft\"",
        "state = \"draft\"\nrequires_extensions = [\"example:future\"]",
    );
    let document =
        parse_document(required.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap();
    let report = validate_document(&document, &ValidationOptions::default());
    assert_eq!(report.validity, Validity::Valid);
    assert_eq!(report.semantic_support, SemanticSupport::Unsupported);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "extension-required")
    );
    let mut supported = ValidationOptions::default();
    supported
        .supported_extensions
        .insert("example:future".into());
    assert_eq!(
        validate_document(&document, &supported).semantic_support,
        SemanticSupport::Supported
    );
    let duplicate = required.replace(
        "[\"example:future\"]",
        "[\"example:future\", \"example:future\"]",
    );
    let document =
        parse_document(duplicate.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap();
    assert_eq!(
        validate_document(&document, &supported).validity,
        Validity::Invalid
    );
}

#[test]
fn local_reference_syntax_is_checked_without_resolving_missing_sources() {
    use openwarrant_core::document::{ValidationOptions, Validity, validate_document};
    let original = std::str::from_utf8(MINIMAL).unwrap();
    let valid = "\ncontext = [{id = \"missing-rule\", target = \"unknown.md#rule\", required = true, when = {stage = [\"build\"], path = [\"src/**/file*.rs\"]}}]\ndependencies = [{unit = \"scope\", target = \"#outcome\"}, {unit = \"scope\", target = \"elsewhere.md#*\"}]\nscope = {subsystems = [], paths = []}";
    let source = original.replace("state = \"draft\"", &format!("state = \"draft\"{valid}"));
    let document = parse_document(source.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap();
    assert_eq!(
        validate_document(&document, &ValidationOptions::default()).validity,
        Validity::Valid
    );
    for (before, after, code) in [
        ("unknown.md#rule", "../outside.md#rule", "target-invalid"),
        ("unknown.md#rule", "https://remote/rule", "target-invalid"),
        ("unknown.md#rule", "file.md#", "target-invalid"),
        ("unknown.md#rule", "file.md#Upper", "target-invalid"),
        ("unit = \"scope\"", "unit = \"absent\"", "source-invalid"),
        ("required = true", "required = \"true\"", "source-invalid"),
        ("stage = [\"build\"]", "stage = []", "condition-invalid"),
        (
            "stage = [\"build\"]",
            "mode = [\"build\"]",
            "condition-invalid",
        ),
        ("src/**/file*.rs", "src/fo**o.rs", "condition-invalid"),
        ("src/**/file*.rs", "src/file?.rs", "condition-invalid"),
        (
            "scope = {subsystems = [], paths = []}",
            "scope = {surprise = true}",
            "source-invalid",
        ),
    ] {
        let input = source.replace(before, after);
        let document =
            parse_document(input.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap();
        let report = validate_document(&document, &ValidationOptions::default());
        assert_eq!(report.validity, Validity::Invalid, "{after}");
        assert!(
            report.diagnostics.iter().any(|d| d.code == code),
            "{report:?}"
        );
    }
}

#[test]
fn framing_preserves_fenced_markers_crlf_and_explicit_legacy_ranges() {
    let original = std::str::from_utf8(MINIMAL).unwrap();
    let fenced = original.replace("## Scope\n", "## Scope\n\n~~~~markdown\n<!-- ow:unit phantom binding -->\n<!-- ow:metadata -->\n~~~\n~~~~\n");
    let document = parse_document(fenced.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap();
    assert_eq!(document.units().len(), 3);
    assert!(document.unit("scope").unwrap().contains("ow:unit phantom"));
    let crlf = original
        .replace("## Scope", "## Périmètre 🦀")
        .replace('\n', "\r\n");
    let document = parse_document(crlf.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap();
    let scope = &document.units()[1];
    assert_eq!(
        &crlf.as_bytes()[scope.span.clone()],
        document.unit("scope").unwrap().as_bytes()
    );
    assert_eq!(scope.span.start, crlf.find("## Périmètre 🦀").unwrap());
    assert_eq!(scope.span.end, crlf.find("<!-- ow:unit context").unwrap());
    let legacy = include_bytes!("../../../conformance/sdk/document/minimal-rc2.md");
    let document = parse_document(legacy, Dialect::Rc2, ParseLimits::default()).unwrap();
    assert_eq!(document.original(), legacy);
    assert_eq!(document.units().last().unwrap().span.end, legacy.len());
    assert!(parse_document(legacy, Dialect::Rc3, ParseLimits::default()).is_err());
    assert!(parse_document(MINIMAL, Dialect::Rc2, ParseLimits::default()).is_err());
}

#[test]
fn malformed_footer_and_unit_framing_never_yield_partial_documents() {
    let original = std::str::from_utf8(MINIMAL).unwrap();
    let malformed = [
        original.replace("<!-- ow:metadata -->", "<!-- ow:metadata broken -->"),
        original.replace(
            "<!-- ow:unit scope binding -->",
            "<!-- ow:unit scope binding --> ",
        ),
        original.replace(
            "<!-- ow:unit scope binding -->",
            "<!-- ow:unit outcome binding -->",
        ),
        original.replace("## Scope\n", "not a heading\n"),
        original.replace("\n<!-- /ow:metadata -->", "\n<!-- /ow:metadata broken -->"),
        original.replace("```toml\n", "```TOML\n"),
        original.replace("\n```\n<!-- /ow:metadata -->", "\n<!-- /ow:metadata -->"),
        original.replace("## Scope\n", "## Scope\n\n```unclosed\n"),
        original.replace("<!-- ow:metadata -->\n", ""),
        format!("{original}extra text"),
        format!("{original}<!-- ow:metadata -->\n"),
        format!("{original}\u{a0}"),
        original.trim_end_matches('\n').to_string() + "\r",
    ];
    for input in malformed {
        let error =
            parse_document(input.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap_err();
        assert!(matches!(error.code, "source-invalid" | "unit-duplicate"));
        assert!(error.span.end <= input.len());
    }
}

#[test]
fn context_kind_requires_at_least_one_nonempty_unit() {
    use openwarrant_core::document::{ValidationOptions, Validity, validate_document};
    let source = "<!-- ow:unit notes background -->\n# Notes\n\n<!-- ow:metadata -->\n```toml\nschema = \"oh.war/document/1.0.0-rc.3\"\nkind = \"context\"\nid = \"example:notes\"\nrevision = 1\ntitle = \"Notes\"\nstate = \"draft\"\n```\n<!-- /ow:metadata -->\n";
    let document = parse_document(source.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap();
    assert_eq!(
        validate_document(&document, &ValidationOptions::default()).validity,
        Validity::Invalid
    );
    let background = source.replace("# Notes\n", "# Notes\n\nNo sources known yet.\n");
    let document =
        parse_document(background.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap();
    assert_eq!(
        validate_document(&document, &ValidationOptions::default()).validity,
        Validity::Invalid
    );
    let populated = background.replace("notes background", "notes binding");
    let document =
        parse_document(populated.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap();
    assert_eq!(
        validate_document(&document, &ValidationOptions::default()).validity,
        Validity::Valid
    );
}

#[test]
fn footer_wrappers_and_dependency_spellings_preserve_meaning_not_bytes() {
    let original = std::str::from_utf8(MINIMAL).unwrap();
    let compact = original.replace("state = \"draft\"", "state = \"draft\"\ndependencies = [{unit = \"scope\", target = \"#outcome\"}, {unit = \"scope\", target = \"#context\"}]");
    let expanded = original.replace("state = \"draft\"", "state = \"draft\"\n[[dependencies]]\nunit = \"scope\"\ntarget = \"#outcome\"\n[[dependencies]]\nunit = \"scope\"\ntarget = \"#context\"");
    let left = parse_document(compact.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap();
    let right = parse_document(expanded.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap();
    assert_eq!(left.metadata(), right.metadata());
    assert_ne!(left.original(), right.original());
    for blank in ["", "\n"] {
        let wrapped = original.replace("<!-- ow:metadata -->\n", &format!("<!-- ow:metadata -->\n<details>\n<summary>OpenWarrant metadata</summary>\n{blank}")).replace("\n<!-- /ow:metadata -->", &format!("\n{blank}</details>\n<!-- /ow:metadata -->"));
        let document =
            parse_document(wrapped.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap();
        assert_eq!(
            document.metadata(),
            parse_document(MINIMAL, Dialect::Rc3, ParseLimits::default())
                .unwrap()
                .metadata()
        );
        assert_eq!(document.unit("context"), left.unit("context"));
        let bad = wrapped.replace("<details>", "<details open>");
        assert_eq!(
            parse_document(bad.as_bytes(), Dialect::Rc3, ParseLimits::default())
                .unwrap_err()
                .code,
            "source-invalid"
        );
    }
}

#[test]
fn limits_are_positive_inclusive_and_refuse_without_truncation() {
    let mut limits = ParseLimits {
        source_bytes: MINIMAL.len(),
        ..ParseLimits::default()
    };
    assert!(parse_document(MINIMAL, Dialect::Rc3, limits).is_ok());
    limits.source_bytes -= 1;
    assert_eq!(
        parse_document(MINIMAL, Dialect::Rc3, limits)
            .unwrap_err()
            .code,
        "resource-limit"
    );
    let document = parse_document(MINIMAL, Dialect::Rc3, ParseLimits::default()).unwrap();
    limits = ParseLimits {
        metadata_bytes: document.metadata_span().len(),
        units: 3,
        ..ParseLimits::default()
    };
    assert!(parse_document(MINIMAL, Dialect::Rc3, limits).is_ok());
    limits.metadata_bytes -= 1;
    assert_eq!(
        parse_document(MINIMAL, Dialect::Rc3, limits)
            .unwrap_err()
            .code,
        "resource-limit"
    );
    for limit in [0, 2] {
        limits = ParseLimits {
            units: limit,
            ..ParseLimits::default()
        };
        assert_eq!(
            parse_document(MINIMAL, Dialect::Rc3, limits)
                .unwrap_err()
                .code,
            "resource-limit"
        );
    }
    let deep = std::str::from_utf8(MINIMAL).unwrap().replace(
        "state = \"draft\"",
        "state = \"draft\"\n[extensions]\n\"example:depth\" = [[[[true]]]]",
    );
    limits = ParseLimits {
        metadata_depth: 3,
        ..ParseLimits::default()
    };
    assert_eq!(
        parse_document(deep.as_bytes(), Dialect::Rc3, limits)
            .unwrap_err()
            .code,
        "resource-limit"
    );
    for input in [
        vec![255, 254],
        [b"\xef\xbb\xbf".as_slice(), MINIMAL].concat(),
        [MINIMAL, b"\0"].concat(),
    ] {
        assert_eq!(
            parse_document(&input, Dialect::Rc3, ParseLimits::default())
                .unwrap_err()
                .code,
            "source-invalid"
        );
    }
}

#[test]
fn toml_10_and_core_field_contract_refusals_are_explicit() {
    use openwarrant_core::document::{ValidationOptions, Validity, validate_document};
    let original = std::str::from_utf8(MINIMAL).unwrap();
    for payload in [
        "\"\\e\"",
        "\"\\x41\"",
        "{\n nested = true\n}",
        "{ nested = true, }",
    ] {
        let source = original.replace(
            "state = \"draft\"",
            &format!("state = \"draft\"\n[extensions]\n\"example:value\" = {payload}"),
        );
        assert_eq!(
            parse_document(source.as_bytes(), Dialect::Rc3, ParseLimits::default())
                .unwrap_err()
                .code,
            "source-invalid"
        );
    }
    for (from, to) in [
        (
            "Explore the signup form. Payment and account deletion are outside this draft.",
            "",
        ),
        ("state = \"draft\"", ""),
        ("title = \"Make signup easier\"", "title = \"\""),
        ("state = \"draft\"", "state = \"draft\"\nalias = \"\""),
        (
            "state = \"draft\"",
            "state = \"draft\"\n[extensions]\nbad = true",
        ),
        (
            "state = \"draft\"",
            "state = \"draft\"\ncontext = [{id = \"x\", target = \"#scope\", required = true}, {id = \"x\", target = \"#context\", required = false}]",
        ),
        (
            "state = \"draft\"",
            "state = \"draft\"\ndependencies = [{unit = \"scope\", target = \"#outcome\", when = {stage = [\"build\"]}}]",
        ),
    ] {
        let source = original.replace(from, to);
        let document =
            parse_document(source.as_bytes(), Dialect::Rc3, ParseLimits::default()).unwrap();
        assert_eq!(
            validate_document(&document, &ValidationOptions::default()).validity,
            Validity::Invalid,
            "{to}"
        );
    }
    let legacy = include_bytes!("../../../conformance/sdk/document/minimal-rc2.md");
    let header_end = std::str::from_utf8(legacy).unwrap()[4..]
        .find("+++")
        .unwrap()
        + 4;
    assert_eq!(
        parse_document(&legacy[..header_end], Dialect::Rc2, ParseLimits::default())
            .unwrap_err()
            .code,
        "source-invalid"
    );
}

#[test]
fn combined_dotted_and_inline_depth_is_bounded_before_recursive_allocation() {
    let original = std::str::from_utf8(MINIMAL).unwrap();
    for width in [8, 32, 63] {
        let mut value = "true".to_string();
        for _ in 0..width {
            value = format!("{{{} = {value}}}", vec!["a"; width].join("."));
        }
        let source = original.replace(
            "state = \"draft\"",
            &format!("state = \"draft\"\n[extensions]\n\"example:deep\" = {value}"),
        );
        assert_eq!(
            parse_document(source.as_bytes(), Dialect::Rc3, ParseLimits::default())
                .unwrap_err()
                .code,
            "resource-limit"
        );
    }
}

#[test]
fn nul_diagnostic_identifies_actual_byte_after_unicode_crlf() {
    let source = std::str::from_utf8(MINIMAL)
        .unwrap()
        .replace("## Scope", "## Étendue 🦀")
        .replace('\n', "\r\n");
    let mut bytes = source.as_bytes().to_vec();
    bytes.push(0);
    let error = parse_document(&bytes, Dialect::Rc3, ParseLimits::default()).unwrap_err();
    assert_eq!(error.code, "source-invalid");
    assert_eq!(error.span, source.len()..source.len() + 1);
}
