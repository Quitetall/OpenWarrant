// SPDX-License-Identifier: AGPL-3.0-or-later
//! Prerequisites and readiness (SAS §32, RQ-035).
//!
//! # What Preflight does and does not mean
//!
//! §32.7, in full: *"Preflight proves only that the work and its verification can
//! validly be attempted. It does not prove the deliverable correct."*
//!
//! That sentence is carried in the type. A [`PreflightReceipt`] grants
//! [`Readiness::Ready`] and nothing else; there is no method on it that says
//! anything about a deliverable, and [`PreflightReceipt::meaning`] returns §32.7
//! verbatim so that a caller rendering a receipt renders the disclaimer with it.
//!
//! # Fail-closed, by construction
//!
//! §32 lists 41 checks across six groups. A receipt is `ready` only when every
//! one has been **run and passed** — an unrun check is not a passed check, which
//! is why [`CheckOutcome`] has a `NotRun` variant and why `NotRun` blocks exactly
//! as `Failed` does. A missing measurement must never read as a pass; that is
//! the failure mode this whole project is about.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::vocab::vocabulary;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PreflightError {
    #[error("unknown {vocabulary} {found:?}; SAS defines {known}")]
    UnknownTerm {
        vocabulary: &'static str,
        found: String,
        known: String,
    },
    #[error(
        "Preflight for {warrant:?} declares itself ready, but {count} of §32's {total} \
         checks did not pass: {failing}"
    )]
    NotActuallyReady {
        warrant: String,
        count: usize,
        total: usize,
        failing: String,
    },
    #[error(
        "Preflight for {warrant:?} records no outcome for {check:?}. §32 is \
         fail-closed: an unrun check is not a passed check"
    )]
    CheckNotRun { warrant: String, check: String },
    #[error(
        "a WAR becomes `ready` only through a SUCCESSFUL Preflight receipt (§32). \
         {warrant:?} was marked ready with no receipt at all"
    )]
    ReadyWithoutReceipt { warrant: String },
}

vocabulary!(
    /// §32's six check groups.
    CheckGroup, "preflight group", PreflightError, {
        Contract => "contract",
        Context => "context",
        Graph => "graph",
        Runtime => "runtime",
        Gates => "gates",
        Authority => "authority",
    }
);

vocabulary!(
    /// What happened to one check.
    ///
    /// `NotRun` exists so that "we did not look" is representable and therefore
    /// refusable. Without it, an unrun check has to be recorded as something
    /// else, and the convenient something else is always `Passed`.
    CheckOutcome, "check outcome", PreflightError, {
        Passed => "passed",
        Failed => "failed",
        NotRun => "not_run",
        NotApplicable => "not_applicable",
    }
);

impl CheckOutcome {
    /// Whether this outcome permits readiness.
    ///
    /// Only `Passed` and `NotApplicable`. `NotRun` blocks exactly as `Failed`
    /// does — that equivalence is the fail-closed property, and
    /// `not_run_blocks_exactly_as_failed_does` pins it.
    #[must_use]
    pub const fn permits_readiness(self) -> bool {
        matches!(self, Self::Passed | Self::NotApplicable)
    }
}

vocabulary!(
    /// Whether a Warrant may be attempted.
    Readiness, "readiness", PreflightError, {
        NotReady => "not_ready",
        Ready => "ready",
    }
);

/// §32.1 — contract checks, verbatim.
pub const CONTRACT_CHECKS: [&str; 6] = [
    "schema pack known",
    "profile valid",
    "contract digest reproducible",
    "required atoms present",
    "required ADRs accepted or explicitly permitted",
    "authorization valid",
];

/// §32.2 — context checks, verbatim.
pub const CONTEXT_CHECKS: [&str; 6] = [
    "required context resolves",
    "normative references are immutable",
    "no unresolved conflict",
    "no unauthorized omission",
    "classification policy satisfied",
    "exact Workspace Basis captured",
];

/// §32.3 — graph checks, verbatim.
pub const GRAPH_CHECKS: [&str; 5] = [
    "no stage or milestone cycle",
    "required stages reachable",
    "named ports compatible",
    "outputs consumed or delivered",
    "supported executor and condition semantics",
];

/// §32.4 — runtime checks, verbatim.
pub const RUNTIME_CHECKS: [&str; 10] = [
    "target repository or workspace available",
    "base revision available",
    "actor identity and role valid",
    "tools available",
    "capabilities realizable",
    "provider available",
    "actual network path usable from the actor environment",
    "resource envelope available",
    "output destinations writable",
    "required secrets resolvable by reference",
];

/// §32.5 — gate checks, verbatim.
pub const GATE_CHECKS: [&str; 9] = [
    "Gate Definition exists",
    "version and digest match",
    "lifecycle permits use",
    "qualification meets assurance level",
    "fixtures exist",
    "performer cannot modify protected gate assets",
    "verifier environment can execute the gate",
    "selectors are valid and nonempty",
    "negative controls remain valid",
];

/// §32.6 — authority checks, verbatim.
pub const AUTHORITY_CHECKS: [&str; 5] = [
    "performer assigned",
    "verifier assigned",
    "resolver available",
    "required independence achievable",
    "side-effect authority sufficient",
];

/// Every §32 check, grouped.
#[must_use]
pub fn all_checks() -> Vec<(CheckGroup, &'static str)> {
    let mut out = Vec::new();
    for c in CONTRACT_CHECKS {
        out.push((CheckGroup::Contract, c));
    }
    for c in CONTEXT_CHECKS {
        out.push((CheckGroup::Context, c));
    }
    for c in GRAPH_CHECKS {
        out.push((CheckGroup::Graph, c));
    }
    for c in RUNTIME_CHECKS {
        out.push((CheckGroup::Runtime, c));
    }
    for c in GATE_CHECKS {
        out.push((CheckGroup::Gates, c));
    }
    for c in AUTHORITY_CHECKS {
        out.push((CheckGroup::Authority, c));
    }
    out
}

/// How many checks §32 defines in total.
pub const TOTAL_CHECKS: usize = CONTRACT_CHECKS.len()
    + CONTEXT_CHECKS.len()
    + GRAPH_CHECKS.len()
    + RUNTIME_CHECKS.len()
    + GATE_CHECKS.len()
    + AUTHORITY_CHECKS.len();

/// §32's receipt. A Warrant becomes `ready` only through one of these.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PreflightReceipt {
    pub warrant: String,
    /// check name -> outcome. A check absent from this map is `NotRun`.
    #[serde(default)]
    pub outcomes: BTreeMap<String, CheckOutcome>,
    #[serde(default)]
    pub performed_at: String,
    #[serde(default)]
    pub receipt_digest: String,
}

impl PreflightReceipt {
    /// The outcome of one check. Absent means `NotRun`, never `Passed`.
    #[must_use]
    pub fn outcome(&self, check: &str) -> CheckOutcome {
        self.outcomes
            .get(check)
            .copied()
            .unwrap_or(CheckOutcome::NotRun)
    }

    /// Every check that does not permit readiness, in §32's order.
    #[must_use]
    pub fn blocking(&self) -> Vec<(CheckGroup, &'static str, CheckOutcome)> {
        all_checks()
            .into_iter()
            .map(|(g, c)| (g, c, self.outcome(c)))
            .filter(|(_, _, o)| !o.permits_readiness())
            .collect()
    }

    /// §32 — readiness, derived from the outcomes and nothing else.
    #[must_use]
    pub fn readiness(&self) -> Readiness {
        if self.blocking().is_empty() {
            Readiness::Ready
        } else {
            Readiness::NotReady
        }
    }

    /// Refuse a receipt that claims readiness it has not earned.
    pub fn validate_claim(&self, claimed: Readiness) -> Result<(), PreflightError> {
        let blocking = self.blocking();
        if claimed == Readiness::Ready && !blocking.is_empty() {
            return Err(PreflightError::NotActuallyReady {
                warrant: self.warrant.clone(),
                count: blocking.len(),
                total: TOTAL_CHECKS,
                failing: blocking
                    .iter()
                    .map(|(g, c, o)| format!("{g}/{c} ({o})"))
                    .collect::<Vec<_>>()
                    .join("; "),
            });
        }
        Ok(())
    }

    /// §32.7, verbatim.
    ///
    /// Returned as a value rather than left in a doc comment so that anything
    /// rendering a receipt renders the limit alongside it. A ready Preflight is
    /// the most over-read signal in this system.
    #[must_use]
    pub const fn meaning() -> &'static str {
        "Preflight proves only that the work and its verification can validly be \
         attempted. It does not prove the deliverable correct."
    }
}

/// §32 — a Warrant is ready only through a successful receipt.
pub fn readiness_of(
    warrant: &str,
    receipt: Option<&PreflightReceipt>,
) -> Result<Readiness, PreflightError> {
    match receipt {
        None => Err(PreflightError::ReadyWithoutReceipt {
            warrant: warrant.to_owned(),
        }),
        Some(r) => Ok(r.readiness()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn all_passing(warrant: &str) -> PreflightReceipt {
        PreflightReceipt {
            warrant: warrant.into(),
            outcomes: all_checks()
                .into_iter()
                .map(|(_, c)| (c.to_owned(), CheckOutcome::Passed))
                .collect(),
            performed_at: "2026-08-20T00:00:00Z".into(),
            receipt_digest: "sha256:r".into(),
        }
    }

    /// §32's six groups, transcribed with their counts.
    #[test]
    fn the_check_groups_and_counts_match_the_sas() {
        assert_eq!(
            CheckGroup::ALL
                .iter()
                .map(|g| g.as_str())
                .collect::<Vec<_>>(),
            [
                "contract",
                "context",
                "graph",
                "runtime",
                "gates",
                "authority"
            ]
        );
        assert_eq!(CONTRACT_CHECKS.len(), 6, "§32.1");
        assert_eq!(CONTEXT_CHECKS.len(), 6, "§32.2");
        assert_eq!(GRAPH_CHECKS.len(), 5, "§32.3");
        assert_eq!(RUNTIME_CHECKS.len(), 10, "§32.4");
        assert_eq!(GATE_CHECKS.len(), 9, "§32.5");
        assert_eq!(AUTHORITY_CHECKS.len(), 5, "§32.6");
        assert_eq!(TOTAL_CHECKS, 41);
        assert_eq!(
            all_checks().len(),
            TOTAL_CHECKS,
            "a group was added to the SAS lists but not to all_checks()"
        );
        // Names must be unique: a receipt is keyed by check name, so a duplicate
        // would let one outcome silently answer for two checks.
        let mut names: Vec<&str> = all_checks().into_iter().map(|(_, c)| c).collect();
        names.sort_unstable();
        let total = names.len();
        names.dedup();
        assert_eq!(names.len(), total, "duplicate check name across §32 groups");
    }

    /// THE fail-closed property: an unrun check blocks exactly as a failed one.
    #[test]
    fn not_run_blocks_exactly_as_failed_does() {
        assert!(!CheckOutcome::NotRun.permits_readiness());
        assert!(!CheckOutcome::Failed.permits_readiness());
        assert!(CheckOutcome::Passed.permits_readiness());
        assert!(CheckOutcome::NotApplicable.permits_readiness());

        // And in practice, on a receipt.
        let mut r = all_passing("OW-WAR-0011");
        assert_eq!(r.readiness(), Readiness::Ready);

        r.outcomes
            .insert("tools available".into(), CheckOutcome::NotRun);
        assert_eq!(
            r.readiness(),
            Readiness::NotReady,
            "an unrun check permitted readiness"
        );

        r.outcomes
            .insert("tools available".into(), CheckOutcome::Failed);
        assert_eq!(r.readiness(), Readiness::NotReady);
    }

    /// A check simply absent from the map must not read as passed.
    #[test]
    fn an_absent_check_is_not_run_not_passed() {
        let empty = PreflightReceipt {
            warrant: "OW-WAR-0011".into(),
            ..PreflightReceipt::default()
        };
        assert_eq!(empty.outcome("tools available"), CheckOutcome::NotRun);
        assert_eq!(empty.readiness(), Readiness::NotReady);
        assert_eq!(
            empty.blocking().len(),
            TOTAL_CHECKS,
            "an empty receipt blocks on everything"
        );
    }

    /// A receipt cannot claim more than its outcomes support.
    #[test]
    fn a_receipt_cannot_claim_readiness_it_has_not_earned() {
        let mut r = all_passing("OW-WAR-0011");
        r.outcomes
            .insert("verifier assigned".into(), CheckOutcome::Failed);

        let err = r.validate_claim(Readiness::Ready).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("verifier assigned"), "{msg}");
        assert!(msg.contains("authority"), "the group is named: {msg}");

        // Claiming not-ready is always honest.
        assert_eq!(r.validate_claim(Readiness::NotReady), Ok(()));
    }

    /// §32 — readiness comes only through a receipt.
    #[test]
    fn readiness_requires_a_receipt_to_exist() {
        assert!(matches!(
            readiness_of("OW-WAR-0011", None),
            Err(PreflightError::ReadyWithoutReceipt { .. })
        ));
        assert_eq!(
            readiness_of("OW-WAR-0011", Some(&all_passing("OW-WAR-0011"))),
            Ok(Readiness::Ready)
        );
    }

    /// §32.7, carried in the type rather than left in prose.
    #[test]
    fn the_receipt_states_what_it_does_not_prove() {
        let m = PreflightReceipt::meaning();
        assert!(m.contains("can validly be attempted"));
        assert!(
            m.contains("does not prove the deliverable correct"),
            "§32.7's limit must travel with the receipt"
        );
    }

    #[test]
    fn blocking_checks_are_reported_in_the_specifications_order() {
        let empty = PreflightReceipt::default();
        let blocking = empty.blocking();
        assert_eq!(blocking[0].0, CheckGroup::Contract);
        assert_eq!(blocking[0].1, "schema pack known");
        assert_eq!(blocking[blocking.len() - 1].0, CheckGroup::Authority);
        assert_eq!(
            blocking[blocking.len() - 1].1,
            "side-effect authority sufficient"
        );
    }

    #[test]
    fn vocabularies_round_trip() {
        for &g in CheckGroup::ALL {
            assert_eq!(CheckGroup::from_str(g.as_str()), Ok(g));
        }
        for &o in CheckOutcome::ALL {
            assert_eq!(CheckOutcome::from_str(o.as_str()), Ok(o));
        }
        for &r in Readiness::ALL {
            assert_eq!(Readiness::from_str(r.as_str()), Ok(r));
        }
        assert!(CheckOutcome::from_str("probably_fine").is_err());
    }

    #[test]
    fn a_receipt_round_trips_through_json() {
        let r = all_passing("OW-WAR-0011");
        let s = serde_json::to_string(&r).expect("serialize");
        assert_eq!(
            serde_json::from_str::<PreflightReceipt>(&s).expect("deserialize"),
            r
        );
    }
}
