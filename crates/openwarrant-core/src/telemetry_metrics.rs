// SPDX-License-Identifier: Apache-2.0
//! Exact derived measurements over caller-supplied observations. No I/O,
//! discovery, baseline inference, authorization or assurance is performed.
//! Sources and populations are disclosed labels, not authenticated provenance.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementUnit {
    Count,
    Milliseconds,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservedTerm {
    Measured {
        value: u64,
        unit: MeasurementUnit,
        population: String,
        source: String,
    },
    Unavailable {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactRatio {
    pub numerator: u64,
    pub denominator: u64,
    /// Exact unit conversion: value = numerator / (denominator_scale * denominator).
    /// Kept separate so even u64::MAX counts never overflow a product.
    pub denominator_scale: u64,
    pub numerator_unit: MeasurementUnit,
    pub numerator_population: String,
    pub denominator_population: String,
    pub numerator_source: String,
    pub denominator_source: String,
    pub method: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum MetricValue {
    Ratio(ExactRatio),
    Unavailable { not_measurable_yet: String },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("{0}")]
pub struct MetricError(pub String);

fn validate(term: &ObservedTerm) -> Result<(), MetricError> {
    match term {
        ObservedTerm::Measured {
            population, source, ..
        } if population.trim().is_empty() || source.trim().is_empty() => Err(MetricError(
            "measured terms require a population and source".into(),
        )),
        ObservedTerm::Unavailable { reason } if reason.trim().is_empty() => {
            Err(MetricError("unavailable terms require a reason".into()))
        }
        _ => Ok(()),
    }
}

/// Preserve raw counts without floating-point conversion, rounding, overflow or
/// reduction that would hide population size. Five rates describe subsets; their
/// numerator cannot exceed the denominator. Per-unit quantities can exceed one.
/// The caller must establish compatible populations and observed event meaning.
/// A nonzero denominator is essential; zero events does not mean a zero rate.
pub fn derive_metric(
    name: &str,
    numerator: ObservedTerm,
    denominator: ObservedTerm,
) -> Result<MetricValue, MetricError> {
    let bounded = match name {
        "human control minutes per accepted WAR"
        | "amendments per WAR"
        | "gate-library reuse rate" => false,
        "safe auto-amendment fraction"
        | "gate-failure-to-repair success rate"
        | "post-resolution escape rate"
        | "untracked-work rate"
        | "adequacy-review catch rate" => true,
        _ => return Err(MetricError(format!("unknown derived metric {name:?}"))),
    };
    validate(&numerator)?;
    validate(&denominator)?;
    let expected_unit = if name == "human control minutes per accepted WAR" {
        MeasurementUnit::Milliseconds
    } else {
        MeasurementUnit::Count
    };
    for (term, unit) in [
        (&numerator, expected_unit),
        (&denominator, MeasurementUnit::Count),
    ] {
        if let ObservedTerm::Measured { unit: actual, .. } = term
            && *actual != unit
        {
            return Err(MetricError(
                "observed term has the wrong measurement unit".into(),
            ));
        }
    }
    let (n, unit, np, ns) = match numerator {
        ObservedTerm::Measured {
            value,
            unit,
            population,
            source,
        } => (value, unit, population, source),
        ObservedTerm::Unavailable { reason } => {
            return Ok(MetricValue::Unavailable {
                not_measurable_yet: format!("numerator unavailable: {reason}"),
            });
        }
    };
    let (d, dp, ds) = match denominator {
        ObservedTerm::Measured {
            value,
            unit: MeasurementUnit::Count,
            population,
            source,
        } => (value, population, source),
        ObservedTerm::Measured { .. } => {
            return Err(MetricError(
                "denominator must count its declared population".into(),
            ));
        }
        ObservedTerm::Unavailable { reason } => {
            return Ok(MetricValue::Unavailable {
                not_measurable_yet: format!("denominator unavailable: {reason}"),
            });
        }
    };
    let denominator_scale = match (name, unit) {
        ("human control minutes per accepted WAR", MeasurementUnit::Milliseconds) => 60_000,
        ("human control minutes per accepted WAR", _) => {
            return Err(MetricError(
                "human control time must be observed in milliseconds".into(),
            ));
        }
        (_, MeasurementUnit::Count) => 1,
        _ => {
            return Err(MetricError(
                "this metric requires counted events, not elapsed time".into(),
            ));
        }
    };
    if d == 0 {
        return Ok(MetricValue::Unavailable {
            not_measurable_yet: format!(
                "denominator population {dp:?} is empty; a rate over no observations is undefined"
            ),
        });
    }
    if bounded && n > d {
        return Err(MetricError(format!(
            "{name}: subset numerator {n} exceeds denominator {d}"
        )));
    }
    Ok(MetricValue::Ratio(ExactRatio {
        numerator: n,
        denominator: d,
        denominator_scale,
        numerator_unit: unit,
        numerator_population: np,
        denominator_population: dp,
        numerator_source: ns,
        denominator_source: ds,
        method: format!(
            "{name}: numerator divided by (denominator_scale times denominator); exact observations, no trend or assurance claim"
        ),
    }))
}
