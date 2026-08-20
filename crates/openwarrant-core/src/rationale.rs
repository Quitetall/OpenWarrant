// SPDX-License-Identifier: AGPL-3.0-or-later
//! The rationale model (SAS §35) and assumptions (§36).
//!
//! # Two rules with teeth
//!
//! §35.3: *"A priority SHALL NOT be presented as an empirical fact."* The node
//! classes are separate types, so "we prefer reversibility" cannot be filed as a
//! [`NodeClass::Fact`] — and a fact must cite a source (§35.2), which a priority
//! has no way to produce.
//!
//! §36.4: *"An assumption cannot be validated by a gate whose meaning depends on
//! that assumption. The claim/evidence graph SHALL be acyclic."* This is the
//! circular-validation prohibition, and it is the one that actually needs an
//! algorithm — [`ClaimGraph::validate`] runs a cycle detection and names the
//! cycle it finds.
//!
//! §36.5's claim narrowing is enforced by [`Assumption`] rather than described:
//! a `blocking_unknown` must state its resolution requirement, and an
//! `accepted_residual_risk` must state what follows if it is false. Both are the
//! sentence you cannot write while overstating a claim.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::vocab::vocabulary;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RationaleError {
    #[error("unknown {vocabulary} {found:?}; SAS defines {known}")]
    UnknownTerm {
        vocabulary: &'static str,
        found: String,
        known: String,
    },
    #[error(
        "node {id:?} is a fact and cites no source. §35.2: a fact cites an \
         authoritative source or empirical observation — without one it is a \
         priority or a forecast wearing a fact's clothes (§35.3)"
    )]
    FactWithoutSource { id: String },
    #[error(
        "node {id:?} is a forecast and omits {field}. §35.4 requires method, \
         assumptions, uncertainty, time horizon, and source — a forecast missing \
         them is an assertion about the future with nothing behind it"
    )]
    ForecastIncomplete { id: String, field: &'static str },
    #[error(
        "node {id:?} is an option and states no {field}. §35.5: every considered \
         option SHOULD identify its shape, benefit, cost, risk, affected \
         requirements, and why it was selected or rejected"
    )]
    OptionIncomplete { id: String, field: &'static str },
    #[error(
        "the decision for node {id:?} is stated inline. §35.6: the decision itself \
         lives in an ADR; the rationale section BINDS the ADR and renders what is \
         relevant"
    )]
    DecisionNotInAdr { id: String },
    #[error(
        "assumption {id:?} is a blocking unknown and states no resolution \
         requirement (§36.3). An unknown with no stated way to resolve it is an \
         unknown nobody has to act on"
    )]
    BlockingUnknownWithoutResolution { id: String },
    #[error(
        "assumption {id:?} is an accepted residual risk and states no consequence \
         if false (§36.2). Accepting a risk whose cost is unstated is not accepting \
         a risk"
    )]
    ResidualRiskWithoutConsequence { id: String },
    #[error(
        "assumption {id:?} is an evidenced premise and cites no evidence (§36.1). \
         The word 'evidenced' is doing all the work"
    )]
    EvidencedPremiseWithoutEvidence { id: String },
    #[error(
        "circular validation: {cycle}. §36.4 — an assumption cannot be validated \
         by a gate whose meaning depends on that assumption, and the claim/evidence \
         graph SHALL be acyclic"
    )]
    CircularValidation { cycle: String },
    #[error("rationale edge references {missing:?}, which is not a declared node")]
    DanglingEdge { missing: String },
}

vocabulary!(
    /// §35.1's eight node classes.
    NodeClass, "rationale node class", RationaleError, {
        Fact => "fact",
        Priority => "priority",
        Constraint => "constraint",
        Option => "option",
        Forecast => "forecast",
        Tradeoff => "tradeoff",
        Decision => "decision",
        Consequence => "consequence",
    }
);

vocabulary!(
    /// §35.7's eight rationale edges.
    RationaleEdge, "rationale edge", RationaleError, {
        Supports => "supports",
        Refutes => "refutes",
        Constrains => "constrains",
        DependsOn => "depends_on",
        TradesOffAgainst => "trades_off_against",
        Causes => "causes",
        Qualifies => "qualifies",
        SelectedOver => "selected_over",
    }
);

vocabulary!(
    /// §36's three assumption statuses. Every assumption SHALL use one.
    EpistemicStatus, "epistemic status", RationaleError, {
        EvidencedPremise => "evidenced_premise",
        AcceptedResidualRisk => "accepted_residual_risk",
        BlockingUnknown => "blocking_unknown",
    }
);

impl EpistemicStatus {
    /// Whether this status stops a Warrant becoming ready (§36.3).
    #[must_use]
    pub const fn blocks_readiness(self) -> bool {
        matches!(self, Self::BlockingUnknown)
    }
}

/// §35.4's forecast fields.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Forecast {
    pub method: String,
    pub assumptions: String,
    pub uncertainty: String,
    pub time_horizon: String,
    pub source: String,
}

/// §35.5's alternative.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AlternativeShape {
    pub implementation_shape: String,
    pub expected_benefit: String,
    pub cost: String,
    pub risk: String,
    #[serde(default)]
    pub affected_requirements: Vec<String>,
    pub reason_selected_or_rejected: String,
}

/// A node in the rationale graph (§35.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RationaleNode {
    pub id: String,
    pub class: NodeClass,
    pub statement: String,
    /// §35.2 — required for a fact, meaningless for a priority.
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub forecast: Option<Forecast>,
    #[serde(default)]
    pub alternative: Option<AlternativeShape>,
    /// §35.6 — a decision node binds an ADR rather than stating the decision.
    #[serde(default)]
    pub adr_ref: String,
}

impl RationaleNode {
    pub fn validate(&self) -> Result<(), RationaleError> {
        match self.class {
            // §35.2 and §35.3 together: this is what stops a preference being
            // filed as a finding.
            NodeClass::Fact if self.source.trim().is_empty() => {
                Err(RationaleError::FactWithoutSource {
                    id: self.id.clone(),
                })
            }
            NodeClass::Forecast => {
                let f = self.forecast.clone().unwrap_or_default();
                for (field, value) in [
                    ("method", &f.method),
                    ("assumptions", &f.assumptions),
                    ("uncertainty", &f.uncertainty),
                    ("time_horizon", &f.time_horizon),
                    ("source", &f.source),
                ] {
                    if value.trim().is_empty() {
                        return Err(RationaleError::ForecastIncomplete {
                            id: self.id.clone(),
                            field,
                        });
                    }
                }
                Ok(())
            }
            NodeClass::Option => {
                let a = self.alternative.clone().unwrap_or_default();
                for (field, value) in [
                    ("implementation_shape", &a.implementation_shape),
                    ("expected_benefit", &a.expected_benefit),
                    ("cost", &a.cost),
                    ("risk", &a.risk),
                    (
                        "reason_selected_or_rejected",
                        &a.reason_selected_or_rejected,
                    ),
                ] {
                    if value.trim().is_empty() {
                        return Err(RationaleError::OptionIncomplete {
                            id: self.id.clone(),
                            field,
                        });
                    }
                }
                Ok(())
            }
            // §35.6 — the decision lives in an ADR, and this node points at it.
            NodeClass::Decision if self.adr_ref.trim().is_empty() => {
                Err(RationaleError::DecisionNotInAdr {
                    id: self.id.clone(),
                })
            }
            _ => Ok(()),
        }
    }
}

/// §36's assumption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assumption {
    pub id: String,
    pub statement: String,
    pub epistemic_status: EpistemicStatus,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub judgment_ref: String,
    #[serde(default)]
    pub consequence_if_false: String,
    #[serde(default)]
    pub resolution_requirement: String,
    /// Gate runs cited as validating this assumption. §36.4 makes this the
    /// interesting field: a gate whose meaning depends on the assumption cannot
    /// validate it.
    #[serde(default)]
    pub validated_by: Vec<String>,
}

impl Assumption {
    pub fn validate(&self) -> Result<(), RationaleError> {
        match self.epistemic_status {
            EpistemicStatus::EvidencedPremise if self.evidence_refs.is_empty() => {
                Err(RationaleError::EvidencedPremiseWithoutEvidence {
                    id: self.id.clone(),
                })
            }
            EpistemicStatus::AcceptedResidualRisk
                if self.consequence_if_false.trim().is_empty() =>
            {
                Err(RationaleError::ResidualRiskWithoutConsequence {
                    id: self.id.clone(),
                })
            }
            EpistemicStatus::BlockingUnknown if self.resolution_requirement.trim().is_empty() => {
                Err(RationaleError::BlockingUnknownWithoutResolution {
                    id: self.id.clone(),
                })
            }
            _ => Ok(()),
        }
    }
}

/// The claim/evidence graph §36.4 requires to be acyclic.
///
/// Edges run *from a claim to what it depends on*. An assumption validated by a
/// gate that itself depends on that assumption closes a loop, and the loop is the
/// circular validation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClaimGraph {
    /// node id -> what it depends on.
    #[serde(default)]
    pub depends_on: BTreeMap<String, Vec<String>>,
}

impl ClaimGraph {
    pub fn add(&mut self, from: &str, to: &str) {
        self.depends_on
            .entry(from.to_owned())
            .or_default()
            .push(to.to_owned());
    }

    /// §36.4 — the graph SHALL be acyclic.
    ///
    /// Iterative depth-first search with an explicit stack. Recursion would blow
    /// the native stack on a deep chain, and a validator that crashes on hostile
    /// input is not a validator.
    pub fn validate(&self) -> Result<(), RationaleError> {
        let mut done: BTreeSet<&str> = BTreeSet::new();

        for root in self.depends_on.keys() {
            if done.contains(root.as_str()) {
                continue;
            }
            // (node, index of the next child to visit)
            let mut stack: Vec<(&str, usize)> = vec![(root.as_str(), 0)];
            let mut on_path: Vec<&str> = vec![root.as_str()];

            while let Some((node, child_index)) = stack.pop() {
                let children = self.depends_on.get(node).map(Vec::as_slice).unwrap_or(&[]);
                if child_index >= children.len() {
                    done.insert(node);
                    on_path.pop();
                    continue;
                }
                stack.push((node, child_index + 1));
                let child = children[child_index].as_str();

                if let Some(start) = on_path.iter().position(|n| *n == child) {
                    let mut cycle: Vec<&str> = on_path[start..].to_vec();
                    cycle.push(child);
                    return Err(RationaleError::CircularValidation {
                        cycle: cycle.join(" -> "),
                    });
                }
                if done.contains(child) {
                    continue;
                }
                on_path.push(child);
                stack.push((child, 0));
            }
        }
        Ok(())
    }

    /// Build the graph implied by a set of assumptions and what validates them.
    #[must_use]
    pub fn from_assumptions(
        assumptions: &[Assumption],
        gate_depends_on: &BTreeMap<String, Vec<String>>,
    ) -> Self {
        let mut g = Self::default();
        for a in assumptions {
            for gate in &a.validated_by {
                // The claim depends on the gate that validates it...
                g.add(&a.id, gate);
            }
        }
        for (gate, deps) in gate_depends_on {
            for d in deps {
                // ...and the gate depends on whatever gives it meaning.
                g.add(gate, d);
            }
        }
        g
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    /// §35.1, transcribed.
    #[test]
    fn the_node_classes_match_the_sas() {
        assert_eq!(
            NodeClass::ALL
                .iter()
                .map(|n| n.as_str())
                .collect::<Vec<_>>(),
            [
                "fact",
                "priority",
                "constraint",
                "option",
                "forecast",
                "tradeoff",
                "decision",
                "consequence",
            ]
        );
    }

    /// §35.7, transcribed.
    #[test]
    fn the_rationale_edges_match_the_sas() {
        assert_eq!(
            RationaleEdge::ALL
                .iter()
                .map(|e| e.as_str())
                .collect::<Vec<_>>(),
            [
                "supports",
                "refutes",
                "constrains",
                "depends_on",
                "trades_off_against",
                "causes",
                "qualifies",
                "selected_over",
            ]
        );
    }

    /// §36, transcribed.
    #[test]
    fn the_epistemic_statuses_match_the_sas() {
        assert_eq!(
            EpistemicStatus::ALL
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
            [
                "evidenced_premise",
                "accepted_residual_risk",
                "blocking_unknown"
            ]
        );
        assert!(EpistemicStatus::BlockingUnknown.blocks_readiness());
        assert!(!EpistemicStatus::AcceptedResidualRisk.blocks_readiness());
    }

    fn node(id: &str, class: NodeClass) -> RationaleNode {
        RationaleNode {
            id: id.into(),
            class,
            statement: "something".into(),
            source: String::new(),
            forecast: None,
            alternative: None,
            adr_ref: String::new(),
        }
    }

    /// §35.3 — a priority is not a fact, and the type system plus §35.2's source
    /// requirement is what keeps them apart.
    #[test]
    fn a_priority_cannot_be_filed_as_a_fact() {
        let mut as_fact = node("R-001", NodeClass::Fact);
        as_fact.statement = "we prefer reversibility".into();
        assert!(
            matches!(
                as_fact.validate(),
                Err(RationaleError::FactWithoutSource { .. })
            ),
            "a value judgment passed as an empirical fact"
        );

        // Filed correctly, it needs no source.
        let mut as_priority = node("R-001", NodeClass::Priority);
        as_priority.statement = "we prefer reversibility".into();
        assert_eq!(as_priority.validate(), Ok(()));
    }

    #[test]
    fn a_fact_with_a_source_validates() {
        let mut f = node("R-002", NodeClass::Fact);
        f.source = "benchmark run 2026-08-19, evidence://run-12".into();
        assert_eq!(f.validate(), Ok(()));
    }

    /// §35.4 — each of the five fields, one at a time.
    #[test]
    fn a_forecast_missing_any_field_is_named() {
        let full = Forecast {
            method: "linear extrapolation".into(),
            assumptions: "load grows as it did last quarter".into(),
            uncertainty: "wide; two data points".into(),
            time_horizon: "six months".into(),
            source: "internal capacity model v3".into(),
        };
        let mut n = node("R-003", NodeClass::Forecast);
        n.forecast = Some(full.clone());
        assert_eq!(n.validate(), Ok(()));

        for (name, blank) in [
            (
                "method",
                (|f: &mut Forecast| f.method.clear()) as fn(&mut Forecast),
            ),
            ("assumptions", |f| f.assumptions.clear()),
            ("uncertainty", |f| f.uncertainty.clear()),
            ("time_horizon", |f| f.time_horizon.clear()),
            ("source", |f| f.source.clear()),
        ] {
            let mut f = full.clone();
            blank(&mut f);
            let mut n = node("R-003", NodeClass::Forecast);
            n.forecast = Some(f);
            match n.validate() {
                Err(RationaleError::ForecastIncomplete { field, .. }) => assert_eq!(field, name),
                other => panic!("forecast without {name} was accepted: {other:?}"),
            }
        }
    }

    /// §35.6 — the decision lives in the ADR, not in the rationale node.
    #[test]
    fn a_decision_node_must_bind_an_adr() {
        let mut d = node("R-004", NodeClass::Decision);
        assert!(matches!(
            d.validate(),
            Err(RationaleError::DecisionNotInAdr { .. })
        ));
        d.adr_ref = "adr://OW-ADR-0006".into();
        assert_eq!(d.validate(), Ok(()));
    }

    #[test]
    fn an_option_missing_a_required_part_is_named() {
        let mut n = node("R-005", NodeClass::Option);
        n.alternative = Some(AlternativeShape {
            implementation_shape: "use a restricted reader".into(),
            expected_benefit: "no implicit typing".into(),
            cost: "we maintain a parser".into(),
            risk: "the subset is too small".into(),
            affected_requirements: vec!["WAR-SAS-RQ-056".into()],
            reason_selected_or_rejected: String::new(),
        });
        match n.validate() {
            Err(RationaleError::OptionIncomplete { field, .. }) => {
                assert_eq!(field, "reason_selected_or_rejected");
            }
            other => panic!("accepted an option with no reason: {other:?}"),
        }
    }

    // ---- §36 -------------------------------------------------------------

    fn assumption(id: &str, status: EpistemicStatus) -> Assumption {
        Assumption {
            id: id.into(),
            statement: "the thing holds".into(),
            epistemic_status: status,
            evidence_refs: vec![],
            judgment_ref: String::new(),
            consequence_if_false: String::new(),
            resolution_requirement: String::new(),
            validated_by: vec![],
        }
    }

    /// Each status must supply the field that makes it honest.
    #[test]
    fn each_status_requires_the_field_that_earns_its_name() {
        assert!(matches!(
            assumption("A-1", EpistemicStatus::EvidencedPremise).validate(),
            Err(RationaleError::EvidencedPremiseWithoutEvidence { .. })
        ));
        assert!(matches!(
            assumption("A-2", EpistemicStatus::AcceptedResidualRisk).validate(),
            Err(RationaleError::ResidualRiskWithoutConsequence { .. })
        ));
        assert!(matches!(
            assumption("A-3", EpistemicStatus::BlockingUnknown).validate(),
            Err(RationaleError::BlockingUnknownWithoutResolution { .. })
        ));

        let mut ok = assumption("A-1", EpistemicStatus::EvidencedPremise);
        ok.evidence_refs = vec!["evidence://run-12".into()];
        assert_eq!(ok.validate(), Ok(()));
    }

    // ---- §36.4, circular validation --------------------------------------

    /// The prohibition, directly: an assumption validated by a gate that depends
    /// on that assumption.
    #[test]
    fn an_assumption_validated_by_a_gate_that_depends_on_it_is_refused() {
        let mut a = assumption("A-1", EpistemicStatus::EvidencedPremise);
        a.evidence_refs = vec!["evidence://x".into()];
        a.validated_by = vec!["gate://g1".into()];

        let mut gate_deps = BTreeMap::new();
        // The gate's meaning depends on the very assumption it validates.
        gate_deps.insert("gate://g1".to_owned(), vec!["A-1".to_owned()]);

        let g = ClaimGraph::from_assumptions(std::slice::from_ref(&a), &gate_deps);
        let err = g.validate().unwrap_err();
        assert!(
            matches!(err, RationaleError::CircularValidation { .. }),
            "{err}"
        );
        assert!(err.to_string().contains("A-1"), "the cycle is named: {err}");
    }

    #[test]
    fn an_acyclic_claim_graph_validates() {
        let mut g = ClaimGraph::default();
        g.add("A-1", "gate://g1");
        g.add("gate://g1", "fixture://f1");
        g.add("A-2", "gate://g1");
        assert_eq!(g.validate(), Ok(()));
    }

    /// A longer loop is still a loop.
    #[test]
    fn an_indirect_cycle_is_found_and_named() {
        let mut g = ClaimGraph::default();
        g.add("A", "B");
        g.add("B", "C");
        g.add("C", "D");
        g.add("D", "A");
        let err = g.validate().unwrap_err();
        let msg = err.to_string();
        for n in ["A", "B", "C", "D"] {
            assert!(msg.contains(n), "cycle should name {n}: {msg}");
        }
    }

    /// A diamond is not a cycle. A validator that reported one would make
    /// ordinary shared evidence unusable.
    #[test]
    fn a_diamond_is_not_a_cycle() {
        let mut g = ClaimGraph::default();
        g.add("top", "left");
        g.add("top", "right");
        g.add("left", "bottom");
        g.add("right", "bottom");
        assert_eq!(g.validate(), Ok(()));
    }

    /// A deep chain must not blow the stack. Recursion here would crash on
    /// hostile input, and a validator that crashes is not a validator.
    ///
    /// 10,000 is far past the depth at which naive recursion overflows the
    /// default stack, and cheap enough to run on every commit.
    #[test]
    fn a_deep_chain_does_not_overflow_the_stack() {
        let mut g = ClaimGraph::default();
        for i in 0..10_000 {
            g.add(&format!("n{i}"), &format!("n{}", i + 1));
        }
        assert_eq!(g.validate(), Ok(()));

        // ...and a cycle at the far end of a deep chain is still caught.
        g.add("n10000", "n0");
        assert!(matches!(
            g.validate(),
            Err(RationaleError::CircularValidation { .. })
        ));
    }

    #[test]
    fn a_self_loop_is_a_cycle() {
        let mut g = ClaimGraph::default();
        g.add("A-1", "A-1");
        assert!(matches!(
            g.validate(),
            Err(RationaleError::CircularValidation { .. })
        ));
    }

    #[test]
    fn vocabularies_round_trip() {
        for &n in NodeClass::ALL {
            assert_eq!(NodeClass::from_str(n.as_str()), Ok(n));
        }
        for &e in RationaleEdge::ALL {
            assert_eq!(RationaleEdge::from_str(e.as_str()), Ok(e));
        }
        for &s in EpistemicStatus::ALL {
            assert_eq!(EpistemicStatus::from_str(s.as_str()), Ok(s));
        }
        assert!(NodeClass::from_str("opinion").is_err());
    }
}
