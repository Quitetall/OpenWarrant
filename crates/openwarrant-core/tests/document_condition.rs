// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::document::{
    MetadataValue,
    condition::{ConditionLimits, validate_condition},
};

#[test]
fn condition_syntax_accepts_f4_fields_and_refuses_unknown_operators() {
    let valid: MetadataValue =
        "stage = ['implementation']\nsubsystem = ['identity']\npath = ['src/**', 'lib/*.rs']"
            .parse()
            .unwrap();
    assert_eq!(
        validate_condition(&valid, ConditionLimits::default())
            .unwrap()
            .as_metadata(),
        &valid
    );
    let invalid: MetadataValue = "not = ['implementation']".parse().unwrap();
    assert_eq!(
        validate_condition(&invalid, ConditionLimits::default())
            .unwrap_err()
            .code,
        "condition-invalid"
    );
}

#[test]
fn malformed_conditions_never_become_unknown_applicability() {
    for source in [
        "",
        "stage=[]",
        "stage=['']",
        "stage='implementation'",
        "stage=[1]",
        "path=['a/**b']",
        "path=['a?']",
        "path=['[a]']",
        "path=['{a,b}']",
        "path=['../x']",
        "path=['/x']",
        "path=['x//y']",
        "path=['x#unit']",
        "other=['x']",
    ] {
        let value: MetadataValue = source.parse().unwrap();
        assert_eq!(
            validate_condition(&value, ConditionLimits::default())
                .unwrap_err()
                .code,
            "condition-invalid",
            "{source}"
        );
    }
    for source in [
        "path=['**','src/**','src/*.rs','é/*']",
        "stage=['Implementation']",
        "subsystem=['identity','payments']",
    ] {
        let value: MetadataValue = source.parse().unwrap();
        assert!(
            validate_condition(&value, ConditionLimits::default()).is_ok(),
            "{source}"
        );
    }
}

#[test]
fn limits_count_entries_across_fields_and_utf8_bytes_without_cloning() {
    let value: MetadataValue = "stage=['é']\nsubsystem=['x']".parse().unwrap();
    let exact = ConditionLimits {
        entries: 2,
        string_bytes: 3,
    };
    let checked = validate_condition(&value, exact).unwrap();
    assert!(std::ptr::eq(checked.as_metadata(), &value));
    for limits in [
        ConditionLimits {
            entries: 1,
            ..exact
        },
        ConditionLimits {
            string_bytes: 2,
            ..exact
        },
        ConditionLimits {
            entries: 0,
            ..exact
        },
        ConditionLimits {
            string_bytes: 0,
            ..exact
        },
    ] {
        assert_eq!(
            validate_condition(&value, limits).unwrap_err().code,
            "resource-limit"
        );
    }
}
