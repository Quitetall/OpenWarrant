// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR relations and currency (SAS §19.2, §19.4, §21), read projections
//! (§17.5), telemetry (§94), and untracked-work detection (§95).
//! RQ-020, RQ-024, RQ-025, RQ-073.
//!
//! # Nothing is silently carried forward
//!
//! §21.5: *"A superseding WAR SHALL explicitly identify which unresolved child
//! WARs, deliverables, evidence, or obligations it adopts. Nothing is silently
//! carried forward."*
//!
//! [`Supersession::validate`] takes the list of unresolved items and refuses a
//! supersession that leaves any of them unaddressed. Every item is either
//! **adopted** or **explicitly dropped with a reason** — there is no third
//! option, because the third option is the silence §21.5 forbids.
//!
//! §21.4 pairs with it: *"Superseded and deprecated WARs SHALL remain available
//! for audit and relation traversal."* Supersession changes currency; it removes
//! nothing.
//!
//! # A decision is an ADR when a future reader would need the reason
//!
//! §19.2 lists six things that do NOT need an ADR, then gives the test:
//!
//! > Would a future executor need to know why one alternative was chosen over
//! > another, or would this choice constrain future work?
//!
//! [`needs_adr`] applies that test, and — like [`crate::autonomy::classify`] —
//! returns `Unknown` rather than guessing. §19.2's exemptions are a closed list;
//! anything outside it has not been shown to be exempt, and the safe direction
//! for an unrecognised decision is to ask.
//!
//! # §95 is a signal, not an accusation
//!
//! *"This is a diagnostic and governance signal. It SHALL not fabricate a
//! relationship after the fact without review."* [`UntrackedWork`] produces
//! candidates, and [`UntrackedWork::attach_relation`] refuses without a recorded
//! reviewer.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::vocab::vocabulary;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LifecycleError {
    #[error("unknown {vocabulary} {found:?}; SAS defines {known}")]
    UnknownTerm {
        vocabulary: &'static str,
        found: String,
        known: String,
    },
    #[error(
        "supersession of {superseded:?} leaves {count} unresolved item(s) \
         unaddressed: {items}. §21.5 — a superseding WAR SHALL explicitly identify \
         what it adopts, and nothing is silently carried forward"
    )]
    SilentCarryForward {
        superseded: String,
        count: usize,
        items: String,
    },
    #[error(
        "supersession of {superseded:?} drops {item:?} without a reason. Dropping is \
         a legitimate choice and an unexplained drop is the same silence §21.5 \
         forbids, one step later"
    )]
    DropWithoutReason { superseded: String, item: String },
    #[error("supersession of {superseded:?} states no reason (§21.1)")]
    SupersessionWithoutReason { superseded: String },
    #[error(
        "a Warrant with currency {currency} was deleted or made unavailable. §21.4: \
         superseded and deprecated WARs SHALL remain available for audit and \
         relation traversal"
    )]
    RetiredWarrantRemoved { currency: Currency },
    #[error(
        "untracked-work candidate {scope:?} was related to {warrant:?} with no \
         recorded reviewer. §95: this SHALL not fabricate a relationship after the \
         fact without review"
    )]
    RelationFabricated { scope: String, warrant: String },
    #[error("telemetry omits {measure:?}, which §94 requires the system to measure")]
    TelemetryIncomplete { measure: &'static str },
}

vocabulary!(
    /// §21's currency values.
    Currency, "currency", LifecycleError, {
        Current => "current",
        Superseded => "superseded",
        Deprecated => "deprecated",
    }
);

impl Currency {
    /// §21.2 and §21.3 — retired for new execution, still authoritative for what
    /// it described.
    #[must_use]
    pub const fn retired_for_new_execution(self) -> bool {
        matches!(self, Self::Superseded | Self::Deprecated)
    }

    /// §21.4 — retired is never removed.
    #[must_use]
    pub const fn remains_available(self) -> bool {
        true
    }

    /// §21.2's required banner, verbatim.
    #[must_use]
    pub const fn banner(self) -> Option<&'static str> {
        match self {
            Self::Superseded => Some(
                "**Superseded and deprecated for new execution. See WAR X. \
                 Historical artifacts, evidence, and resolution remain authoritative \
                 for the period and basis they describe.**",
            ),
            Self::Deprecated | Self::Current => None,
        }
    }
}

vocabulary!(
    /// §19.4's ADR relations to a Warrant.
    AdrRelation, "ADR relation", LifecycleError, {
        OriginatesFrom => "originates_from",
        Governs => "governs",
        ImplementedBy => "implemented_by",
        Supersedes => "supersedes",
        AmendsSas => "amends_sas",
        AuthorizesAmendmentClass => "authorizes_amendment_class",
        RecordsPhaseGateOutcome => "records_phase_gate_outcome",
    }
);

vocabulary!(
    /// §17.5's read projections.
    Projection, "projection", LifecycleError, {
        FullWarrant => "full_warrant",
        WorkOrder => "work_order",
        AdrSection => "adr_section",
        AdrOverview => "adr_overview",
        StageDispatch => "stage_dispatch",
        AssuranceCase => "assurance_case",
        Status => "status",
        Audit => "audit",
        CanonicalJson => "canonical_json",
    }
);

/// §19.2's six exemptions, verbatim.
pub const NOT_A_NEW_ADR: [&str; 6] = [
    "a private variable name",
    "a local refactor preserving all declared behavior",
    "selection among equivalent permitted tools",
    "a mechanical application of a governing ADR",
    "an auto-authorized amendment class whose governing policy already made the normative decision",
    "a factual correction with no semantic choice",
];

/// The answer to §19.2's test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdrNeed {
    /// §19.2 exempts it explicitly.
    Exempt,
    /// A future executor would need the reason, or it constrains future work.
    Required,
    /// Neither exempt nor obviously normative. Ask.
    Unknown,
}

/// §19.2's test, applied.
///
/// `constrains_future_work` and `future_reader_needs_the_reason` come from the
/// author, not from this function — §19.2 asks a question about consequences,
/// and no string match can answer it. What this does is refuse to guess: a
/// decision that is neither on the exemption list nor declared constraining
/// returns `Unknown`, and the safe response to `Unknown` is to ask.
#[must_use]
pub fn needs_adr(
    decision: &str,
    constrains_future_work: bool,
    future_reader_needs_the_reason: bool,
) -> AdrNeed {
    if constrains_future_work || future_reader_needs_the_reason {
        // The test in §19.2 is decisive in this direction even for an item that
        // resembles an exemption: "if yes, create an ADR".
        return AdrNeed::Required;
    }
    let d = decision.trim().to_lowercase();
    if NOT_A_NEW_ADR.iter().any(|e| *e == d) {
        return AdrNeed::Exempt;
    }
    AdrNeed::Unknown
}

/// What a superseding Warrant does with one unresolved item (§21.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Adoption {
    pub item: String,
    pub adopted: bool,
    /// Required when `adopted` is false. An unexplained drop is the same silence
    /// §21.5 forbids, one step later.
    #[serde(default)]
    pub drop_reason: String,
}

/// §21.1's supersession relation.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Supersession {
    pub superseding: String,
    pub superseded: String,
    pub reason: String,
    #[serde(default)]
    pub adoptions: Vec<Adoption>,
}

impl Supersession {
    /// §21.1 and §21.5.
    ///
    /// `unresolved` is the superseded Warrant's actual unresolved items, so the
    /// superseding Warrant cannot satisfy this by listing only what it felt like
    /// addressing.
    pub fn validate(&self, unresolved: &[String]) -> Result<(), LifecycleError> {
        if self.reason.trim().is_empty() {
            return Err(LifecycleError::SupersessionWithoutReason {
                superseded: self.superseded.clone(),
            });
        }
        let addressed: BTreeSet<&str> = self.adoptions.iter().map(|a| a.item.as_str()).collect();
        let missing: Vec<&str> = unresolved
            .iter()
            .map(String::as_str)
            .filter(|i| !addressed.contains(i))
            .collect();
        if !missing.is_empty() {
            return Err(LifecycleError::SilentCarryForward {
                superseded: self.superseded.clone(),
                count: missing.len(),
                items: missing.join(", "),
            });
        }
        for a in &self.adoptions {
            if !a.adopted && a.drop_reason.trim().is_empty() {
                return Err(LifecycleError::DropWithoutReason {
                    superseded: self.superseded.clone(),
                    item: a.item.clone(),
                });
            }
        }
        Ok(())
    }

    /// §21.2 — what the superseded Warrant's currency becomes.
    #[must_use]
    pub const fn superseded_currency() -> Currency {
        Currency::Superseded
    }
}

/// §94's measures, verbatim.
pub const TELEMETRY_MEASURES: [&str; 18] = [
    "human authoring minutes",
    "interview questions",
    "clarification count",
    "escalation count and class",
    "amendments",
    "auto-authorizable fraction",
    "replay, repair, restart",
    "gate failure cause",
    "adequacy counterexamples",
    "wall time",
    "compute and model cost",
    "time to first usable artifact",
    "reopenings",
    "untracked commits or artifacts",
    "work completed outside WAR",
    "evidence and gate reuse",
    "post-resolution escapes",
    "gate library reuse",
];

/// §94's derived metrics, verbatim.
pub const DERIVED_METRICS: [&str; 8] = [
    "human control minutes per accepted WAR",
    "amendments per WAR",
    "safe auto-amendment fraction",
    "gate-failure-to-repair success rate",
    "post-resolution escape rate",
    "untracked-work rate",
    "adequacy-review catch rate",
    "gate-library reuse rate",
];

/// §95's tracking identifiers that a tracked change should carry.
pub const TRACKING_IDENTIFIERS: [&str; 4] = [
    "WAR UUID or enterprise ID",
    "contract digest",
    "Dispatch ID",
    "runtime run ID",
];

/// §95's candidate — a change to a tracked scope with no Warrant relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UntrackedWork {
    pub scope: String,
    #[serde(default)]
    pub carried_identifiers: Vec<String>,
    /// A relation attached AFTER the fact, with the reviewer who approved it.
    #[serde(default)]
    pub related_warrant: String,
    #[serde(default)]
    pub reviewed_by: String,
}

impl UntrackedWork {
    /// §95 — a change carrying none of the tracking identifiers is a candidate.
    #[must_use]
    pub fn is_candidate(&self) -> bool {
        self.carried_identifiers.is_empty()
    }

    /// §95 — attaching a relation after the fact requires review.
    ///
    /// The whole value of this signal is that it is honest about what it does
    /// not know. A tool that quietly attributed orphan commits to whichever
    /// Warrant was open at the time would turn a diagnostic into a fabrication.
    pub fn attach_relation(&mut self, warrant: &str, reviewer: &str) -> Result<(), LifecycleError> {
        if reviewer.trim().is_empty() {
            return Err(LifecycleError::RelationFabricated {
                scope: self.scope.clone(),
                warrant: warrant.to_owned(),
            });
        }
        self.related_warrant = warrant.to_owned();
        self.reviewed_by = reviewer.to_owned();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    /// §17.5's nine projections, transcribed.
    #[test]
    fn the_projections_match_the_sas() {
        assert_eq!(
            Projection::ALL
                .iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>(),
            [
                "full_warrant",
                "work_order",
                "adr_section",
                "adr_overview",
                "stage_dispatch",
                "assurance_case",
                "status",
                "audit",
                "canonical_json",
            ]
        );
    }

    /// §19.4's seven relations, transcribed.
    #[test]
    fn the_adr_relations_match_the_sas() {
        assert_eq!(AdrRelation::ALL.len(), 7);
        assert_eq!(AdrRelation::ALL[0].as_str(), "originates_from");
        assert_eq!(AdrRelation::ALL[6].as_str(), "records_phase_gate_outcome");
    }

    // ---- §21.5, the rule this module exists for --------------------------

    /// Nothing is silently carried forward.
    #[test]
    fn a_supersession_cannot_leave_unresolved_work_unaddressed() {
        let s = Supersession {
            superseding: "OW-WAR-0099".into(),
            superseded: "OW-WAR-0042".into(),
            reason: "approach replaced".into(),
            adoptions: vec![Adoption {
                item: "OBL-001".into(),
                adopted: true,
                drop_reason: String::new(),
            }],
        };
        let err = s
            .validate(&["OBL-001".to_owned(), "DEL-002".to_owned()])
            .unwrap_err();
        assert!(
            matches!(err, LifecycleError::SilentCarryForward { .. }),
            "{err}"
        );
        assert!(err.to_string().contains("DEL-002"), "{err}");
    }

    /// Dropping is legitimate; dropping silently is not.
    #[test]
    fn dropping_an_item_requires_a_reason() {
        let mut s = Supersession {
            superseding: "OW-WAR-0099".into(),
            superseded: "OW-WAR-0042".into(),
            reason: "approach replaced".into(),
            adoptions: vec![Adoption {
                item: "DEL-002".into(),
                adopted: false,
                drop_reason: String::new(),
            }],
        };
        assert!(matches!(
            s.validate(&["DEL-002".to_owned()]),
            Err(LifecycleError::DropWithoutReason { .. })
        ));

        s.adoptions[0].drop_reason = "the deliverable was withdrawn from scope".into();
        assert_eq!(s.validate(&["DEL-002".to_owned()]), Ok(()));
    }

    #[test]
    fn a_supersession_states_its_reason() {
        let s = Supersession {
            superseding: "OW-WAR-0099".into(),
            superseded: "OW-WAR-0042".into(),
            reason: String::new(),
            adoptions: vec![],
        };
        assert!(matches!(
            s.validate(&[]),
            Err(LifecycleError::SupersessionWithoutReason { .. })
        ));
    }

    /// §21.2's banner, verbatim, and only on `superseded`.
    #[test]
    fn the_superseded_banner_matches_the_sas() {
        let b = Currency::Superseded
            .banner()
            .expect("superseded has a banner");
        assert!(b.contains("Superseded and deprecated for new execution"));
        assert!(b.contains("Historical artifacts, evidence, and resolution remain authoritative"));
        assert!(Currency::Deprecated.banner().is_none());
        assert!(Currency::Current.banner().is_none());
    }

    /// §21.4 — retirement is not removal.
    #[test]
    fn retired_warrants_remain_available() {
        for c in [Currency::Superseded, Currency::Deprecated] {
            assert!(c.retired_for_new_execution(), "{c}");
            assert!(c.remains_available(), "{c} was treated as deletable");
        }
        assert!(!Currency::Current.retired_for_new_execution());
        assert_eq!(Supersession::superseded_currency(), Currency::Superseded);
    }

    // ---- §19.2 -----------------------------------------------------------

    /// The six exemptions, and the fact that an unrecognised decision is not one.
    #[test]
    fn an_unrecognised_decision_is_not_assumed_exempt() {
        assert_eq!(NOT_A_NEW_ADR.len(), 6);
        assert_eq!(
            needs_adr("a private variable name", false, false),
            AdrNeed::Exempt
        );
        for unknown in [
            "switching the default compression level",
            "renaming a public method",
            "picking a serialization format",
        ] {
            assert_eq!(
                needs_adr(unknown, false, false),
                AdrNeed::Unknown,
                "{unknown:?} was assumed exempt"
            );
        }
    }

    /// §19.2's test is decisive: "if yes, create an ADR" — even for something
    /// that superficially resembles an exemption.
    #[test]
    fn the_consequence_test_overrides_the_exemption_list() {
        assert_eq!(
            needs_adr(
                "a local refactor preserving all declared behavior",
                false,
                false
            ),
            AdrNeed::Exempt
        );
        assert_eq!(
            needs_adr(
                "a local refactor preserving all declared behavior",
                true,
                false
            ),
            AdrNeed::Required,
            "a choice that constrains future work needs an ADR whatever it is called"
        );
        assert_eq!(
            needs_adr("selection among equivalent permitted tools", false, true),
            AdrNeed::Required,
            "if a future executor needs the reason, it needs an ADR"
        );
    }

    // ---- §94, §95 --------------------------------------------------------

    #[test]
    fn the_telemetry_measures_and_derived_metrics_match_the_sas() {
        assert_eq!(DERIVED_METRICS.len(), 8);
        assert_eq!(DERIVED_METRICS[0], "human control minutes per accepted WAR");
        assert_eq!(DERIVED_METRICS[7], "gate-library reuse rate");
        assert!(TELEMETRY_MEASURES.contains(&"adequacy counterexamples"));
        assert!(TELEMETRY_MEASURES.contains(&"work completed outside WAR"));

        let mut names = TELEMETRY_MEASURES.to_vec();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "duplicate telemetry measure");
    }

    #[test]
    fn the_tracking_identifiers_match_the_sas() {
        assert_eq!(
            TRACKING_IDENTIFIERS,
            [
                "WAR UUID or enterprise ID",
                "contract digest",
                "Dispatch ID",
                "runtime run ID",
            ]
        );
    }

    /// §95 — a signal, not an accusation. A relation attached after the fact
    /// needs a reviewer, or the diagnostic becomes a fabrication.
    #[test]
    fn a_relation_cannot_be_fabricated_after_the_fact() {
        let mut u = UntrackedWork {
            scope: "src/encoder.rs".into(),
            carried_identifiers: vec![],
            related_warrant: String::new(),
            reviewed_by: String::new(),
        };
        assert!(u.is_candidate());

        assert!(matches!(
            u.attach_relation("OW-WAR-0042", "  "),
            Err(LifecycleError::RelationFabricated { .. })
        ));
        assert!(
            u.related_warrant.is_empty(),
            "a relation was written anyway"
        );

        assert_eq!(u.attach_relation("OW-WAR-0042", "QuiteTall"), Ok(()));
        assert_eq!(u.reviewed_by, "QuiteTall");
    }

    #[test]
    fn work_carrying_a_tracking_identifier_is_not_a_candidate() {
        let u = UntrackedWork {
            scope: "src/encoder.rs".into(),
            carried_identifiers: vec!["contract digest".into()],
            related_warrant: String::new(),
            reviewed_by: String::new(),
        };
        assert!(!u.is_candidate());
    }

    #[test]
    fn vocabularies_round_trip() {
        for &c in Currency::ALL {
            assert_eq!(Currency::from_str(c.as_str()), Ok(c));
        }
        for &p in Projection::ALL {
            assert_eq!(Projection::from_str(p.as_str()), Ok(p));
        }
        for &r in AdrRelation::ALL {
            assert_eq!(AdrRelation::from_str(r.as_str()), Ok(r));
        }
    }
}
