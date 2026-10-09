// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::telemetry_metrics::{
    MeasurementUnit, MetricValue, ObservedTerm, derive_metric,
};

fn measured(value: u64, population: &str) -> ObservedTerm {
    ObservedTerm::Measured {
        value,
        unit: MeasurementUnit::Count,
        population: population.into(),
        source: "fixture://captured-events".into(),
    }
}

#[test]
fn exact_counts_preserve_noninteger_rates_and_large_values() {
    let MetricValue::Ratio(ratio) = derive_metric(
        "amendments per WAR",
        measured(5, "recorded amendments"),
        measured(2, "compiled Warrants"),
    )
    .unwrap() else {
        panic!("must measure")
    };
    assert_eq!((ratio.numerator, ratio.denominator), (5, 2));
    assert_eq!(ratio.denominator_population, "compiled Warrants");
    let MetricValue::Ratio(ratio) = derive_metric(
        "amendments per WAR",
        measured(u64::MAX, "recorded amendments"),
        measured(1, "compiled Warrants"),
    )
    .unwrap() else {
        panic!("must measure")
    };
    assert_eq!(ratio.numerator, u64::MAX);
}

#[test]
fn missing_observations_and_empty_populations_are_not_zero() {
    let value = derive_metric(
        "human control minutes per accepted WAR",
        ObservedTerm::Unavailable {
            reason: "no instrumented human duration".into(),
        },
        measured(2, "accepted Warrants"),
    )
    .unwrap();
    assert!(matches!(value, MetricValue::Unavailable { .. }));
    let value = derive_metric(
        "untracked-work rate",
        measured(0, "untracked candidate commits"),
        measured(0, "examined commits"),
    )
    .unwrap();
    assert!(matches!(value, MetricValue::Unavailable { .. }));
    let MetricValue::Ratio(ratio) = derive_metric(
        "untracked-work rate",
        measured(0, "untracked candidate commits"),
        measured(2, "examined commits"),
    )
    .unwrap() else {
        panic!("observed zero must remain measured")
    };
    assert_eq!((ratio.numerator, ratio.denominator), (0, 2));
}

#[test]
fn impossible_fraction_unknown_metric_and_unattributable_terms_are_refused() {
    assert!(
        derive_metric(
            "untracked-work rate",
            measured(3, "candidate commits"),
            measured(2, "examined commits")
        )
        .is_err()
    );
    assert!(
        derive_metric(
            "invented rate",
            measured(1, "events"),
            measured(2, "events")
        )
        .is_err()
    );
    assert!(
        derive_metric(
            "amendments per WAR",
            measured(1, ""),
            measured(2, "Warrants")
        )
        .is_err()
    );
    assert!(
        derive_metric(
            "amendments per WAR",
            ObservedTerm::Measured {
                value: 1,
                unit: MeasurementUnit::Count,
                population: "amendments".into(),
                source: String::new()
            },
            measured(2, "Warrants")
        )
        .is_err()
    );
    assert!(
        derive_metric(
            "untracked-work rate",
            ObservedTerm::Unavailable {
                reason: String::new()
            },
            measured(2, "commits")
        )
        .is_err()
    );
}

#[test]
fn all_eight_names_have_defined_quantity_or_fraction_semantics() {
    for name in openwarrant_core::lifecycle::DERIVED_METRICS {
        assert!(
            derive_metric(
                name,
                if name == "human control minutes per accepted WAR" {
                    ObservedTerm::Measured {
                        value: 30_000,
                        unit: MeasurementUnit::Milliseconds,
                        population: "human review duration".into(),
                        source: "fixture://timer".into(),
                    }
                } else {
                    measured(1, "numerator population")
                },
                measured(2, "denominator population")
            )
            .is_ok(),
            "{name}"
        );
    }
}

#[test]
fn subminute_duration_and_conversion_are_preserved_without_overflow() {
    let MetricValue::Ratio(ratio) = derive_metric(
        "human control minutes per accepted WAR",
        ObservedTerm::Measured {
            value: 30_000,
            unit: MeasurementUnit::Milliseconds,
            population: "human control time".into(),
            source: "fixture://timer".into(),
        },
        measured(1, "accepted Warrants"),
    )
    .unwrap() else {
        panic!("must measure")
    };
    assert_eq!(
        (ratio.numerator, ratio.denominator, ratio.denominator_scale),
        (30_000, 1, 60_000)
    );
    assert!(
        derive_metric(
            "human control minutes per accepted WAR",
            measured(1, "duration"),
            measured(1, "Warrants")
        )
        .is_err()
    );
    assert!(
        derive_metric(
            "amendments per WAR",
            measured(1, "amendments"),
            ObservedTerm::Measured {
                value: 2,
                unit: MeasurementUnit::Milliseconds,
                population: "wrong dimension".into(),
                source: "fixture://timer".into()
            }
        )
        .is_err()
    );
}
