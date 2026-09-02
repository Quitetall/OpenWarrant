// SPDX-License-Identifier: AGPL-3.0-or-later
//! The corpus projection — where the whole body of Warrants stands, from
//! records (SAS §17.5 `status`, §34.3, §98, §102 decision 8).
//!
//! # Two trees, hinged on Warrant
//!
//! §102 decision 8 gives the hierarchy verbatim: *Vision → SAS and Roadmap →
//! WAR → Milestone → Stage/Dispatch → Artifact/Evidence → Resolution.* That is
//! two trees sharing one node:
//!
//! ```text
//! SPEC AXIS                              ROADMAP AXIS
//! Release   = an accepted SAS revision   Objective = SAS §98 phase 0..=10
//!   └─ Requirement (RQ-xxx, §106)          └─ Warrant (roadmap://…-PHASE-N/…)
//!         └─ implemented by ──►  WARRANT  ◄──┘
//!                                    └─ Milestone (§23.1)
//!                                         └─ Stage (§47)
//! ```
//!
//! "Requirements satisfied" and "objectives achieved" are therefore different
//! numbers measuring different trees, and both are legitimate. This module
//! names the levels — *Release, Objective, Warrant, Milestone, Stage,
//! Requirement* — avoiding three words the SAS already uses with other precise
//! meanings ("phase" is both §24 lifecycle and §98 roadmap; "milestone" and
//! "stage" are Warrant-internal).
//!
//! # Every count is a ladder, and there is no ratio
//!
//! This repository has refused, in four separate places, to print a number
//! that reads as progress. That refusal is made structural here: every
//! aggregate is a set of named rungs with the strictest first, and nothing in
//! this module or its renderer divides one number by another. A reader who
//! wants "69%" can compute it and will have to look at what they divided.
//!
//! The headline rung is always the strictest one. For requirements that is
//! `satisfied` (§34.3), which today is zero across the corpus, and the
//! projection says zero.
//!
//! # Derived, and labelled as such
//!
//! Nothing here reads a resolution record, because none exists — `war resolve`
//! refuses to write one without an authorizer. Every Warrant carries
//! [`Provenance::Derived`], and the document says once, at the top, why every
//! row reads the way it does.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::resolution::ResolutionChecks;
use crate::state::{Provenance, WarrantState};
use crate::traceability::{
    Contribution, Implements, RequirementRef, RequirementStatus, RoadmapRef,
};

pub const CORPUS_STATUS_SCHEMA: &str = "oh.war/corpus-status/v1";

/// SAS §98's eleven phases: number, heading, and the Exit sentence verbatim.
///
/// Phases 9 and 10 have no Exit in the SAS, and `None` here is that fact
/// carried forward rather than a criterion invented to fill the gap.
pub const PHASES: [(u8, &str, Option<&str>); 11] = [
    (
        0,
        "Telemetry shim",
        Some("real distributions for authoring cost, amendment types, and failure causes."),
    ),
    (
        1,
        "File-native WAR compiler",
        Some("OpenWarrant development uses WARs."),
    ),
    (
        2,
        "Agent planner",
        Some(
            "a vague engineering request produces a reviewable valid draft without direct model file mutation.",
        ),
    ),
    (
        3,
        "ADR federation",
        Some("no managed normative decision exists only inline."),
    ),
    (
        4,
        "Knowledge Fabric registration",
        Some(
            "registered WARs use KF as institutional authority while Git may remain Source Holder.",
        ),
    ),
    (
        5,
        "Dispatch and Katana execution",
        Some(
            "one WAR stage can be compiled, executed by a stateless Katana agent, and returned without authority confusion.",
        ),
    ),
    (
        6,
        "Gate Registry and assurance case",
        Some("a delivery can close only through bounded, provenance-preserving proof."),
    ),
    (
        7,
        "BLUT adapter",
        Some("compatible computational WARs execute without duplicating BLUT."),
    ),
    (
        8,
        "Liminal production compiler",
        Some("Liminal is the single production document semantic compiler."),
    ),
    (9, "High-assurance controls", None),
    (10, "Contractor Work Order profile", None),
];

/// Look up a §98 phase by number.
#[must_use]
pub fn phase(n: u8) -> Option<(&'static str, Option<&'static str>)> {
    PHASES
        .iter()
        .find(|(p, _, _)| *p == n)
        .map(|(_, t, e)| (*t, *e))
}

/// Where one Warrant stands, strictest rung last so that `Ord` reads as
/// "further along".
///
/// `Resolved` is reachable only from a §56.2 record. Nothing derives it, and
/// nothing may: a resolver that inferred resolution from the record's shape
/// would be manufacturing the false completion this whole system refuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarrantRung {
    /// The manifest did not validate; nothing downstream can be trusted.
    Invalid,
    /// At least one of §56.1's thirteen is unmet.
    Draft,
    /// All thirteen met; §38.6 would NOT resolve satisfied (or is unknown).
    ReadyToResolve,
    /// All thirteen met and §38.6 would resolve satisfied. Awaiting a human.
    WouldSatisfy,
    /// A recorded §56.2 resolution.
    Resolved,
}

impl WarrantRung {
    /// Derive the rung. `resolved` comes only from a record.
    #[must_use]
    pub fn derive(
        valid: bool,
        checks: Option<&ResolutionChecks>,
        would_satisfy: Option<bool>,
        resolved: bool,
    ) -> Self {
        if !valid {
            return Self::Invalid;
        }
        if resolved {
            return Self::Resolved;
        }
        match checks {
            Some(c) if c.unmet().is_empty() => {
                if would_satisfy == Some(true) {
                    Self::WouldSatisfy
                } else {
                    Self::ReadyToResolve
                }
            }
            _ => Self::Draft,
        }
    }
}

/// Named rung counts for a set of Warrants. Never a ratio.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarrantLadder {
    pub invalid: usize,
    pub draft: usize,
    pub ready_to_resolve: usize,
    pub would_satisfy: usize,
    pub resolved: usize,
}

impl WarrantLadder {
    pub fn count(&mut self, rung: WarrantRung) {
        match rung {
            WarrantRung::Invalid => self.invalid += 1,
            WarrantRung::Draft => self.draft += 1,
            WarrantRung::ReadyToResolve => self.ready_to_resolve += 1,
            WarrantRung::WouldSatisfy => self.would_satisfy += 1,
            WarrantRung::Resolved => self.resolved += 1,
        }
    }

    #[must_use]
    pub fn total(&self) -> usize {
        self.invalid + self.draft + self.ready_to_resolve + self.would_satisfy + self.resolved
    }
}

/// Named rung counts for the requirement axis (§34.3). Never a ratio.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementCounts {
    pub unaddressed: usize,
    pub claimed: usize,
    pub in_progress: usize,
    pub satisfied: usize,
    pub superseded: usize,
}

impl RequirementCounts {
    pub fn count(&mut self, status: RequirementStatus) {
        match status {
            RequirementStatus::Unaddressed => self.unaddressed += 1,
            RequirementStatus::Claimed => self.claimed += 1,
            RequirementStatus::InProgress => self.in_progress += 1,
            RequirementStatus::Satisfied => self.satisfied += 1,
            RequirementStatus::Superseded => self.superseded += 1,
        }
    }

    #[must_use]
    pub fn total(&self) -> usize {
        self.unaddressed + self.claimed + self.in_progress + self.satisfied + self.superseded
    }
}

/// Whether an Objective (a §98 phase) is achieved.
///
/// Achievement is defined by the phase's `exit` Warrant — the one whose
/// `roadmap://` slug is `exit`, which discharges §98's Exit criterion. The Exit
/// sentence itself is carried as text and never evaluated; no tool can judge
/// "Liminal is the single production document semantic compiler".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum Achieved {
    /// No Exit criterion in §98 (phases 9, 10), or no `exit` Warrant, or no
    /// member Warrants at all. Said so rather than reported as blocked.
    NotDerivable { why: String },
    /// Named members are below `WouldSatisfy`.
    Blocked { by: Vec<String> },
    /// The exit Warrant would resolve satisfied; a human has not recorded it.
    ExitWarrantWouldSatisfy,
    /// A recorded resolution of the exit Warrant.
    Recorded,
}

/// One §98 phase and the Warrants that name it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectiveStatus {
    /// `None` for the synthetic `unassigned` group: Warrants declaring no
    /// `[[roadmap]]`. Listed last, flagged, and never actionable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roadmap_ref: Option<RoadmapRef>,
    pub title: String,
    /// §98's Exit sentence, verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_criterion: Option<String>,
    /// The member whose slug is `exit`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_warrant: Option<String>,
    /// Member aliases, sorted.
    pub warrants: Vec<String>,
    pub ladder: WarrantLadder,
    pub achieved: Achieved,
}

/// Whether the manifest validated. An invalid Warrant is LISTED, not omitted:
/// a denominator that quietly shrinks is the cheap way to look further along.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum Validity {
    Valid,
    Invalid { reason: String },
}

/// A `[[implements]]` claim, parsed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImplementsClaim {
    pub requirement: RequirementRef,
    pub contribution: Contribution,
}

/// Whether a Milestone (§23.1) is reached, derived from verifications.
///
/// Authoritative "reached" per §72.6 needs stage submissions and receipts,
/// which nothing records yet. This is what the verification records support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum Reached {
    /// Cites no obligations, so there is nothing to evidence.
    Declared,
    /// A dependency is not yet evidenced.
    BlockedOn { milestones: Vec<String> },
    /// Dependencies evidenced; this one is not.
    Unblocked,
    /// Every cited obligation has an admissible verification that permits
    /// satisfaction (§38.6).
    Evidenced { obligations: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MilestoneState {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub depends_on: Vec<String>,
    pub stages: Vec<String>,
    pub obligations: Vec<String>,
    pub reached: Reached,
}

/// One Warrant, as much as the records support.
/// A recorded §56.2 resolution, as the projection reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionSummary {
    pub common_outcome: String,
    pub profile_outcome: String,
    pub resolved_by_ref: String,
    pub effective_at: String,
    /// The record binds the contract as it compiles now. When false the record
    /// is reported and counted for nothing: a resolution of an earlier revision
    /// is not a resolution of this one.
    pub binds_current_contract: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarrantStatus {
    pub alias: String,
    /// Present when `resolution.toml` exists. Read, never derived.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<ResolutionSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub validity: Validity,
    pub rung: WarrantRung,
    pub roadmap: Vec<RoadmapRef>,
    pub implements: Vec<ImplementsClaim>,
    /// §24's five dimensions with provenance. Derived until the journal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<WarrantState>,
    /// The thirteen, by name. `None` when the Warrant could not be evaluated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checks: Option<ResolutionChecks>,
    /// The unmet requirement names — names, so a reader knows what to fix.
    pub unmet: Vec<String>,
    /// §38.6, beside the thirteen and never folded into them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub would_resolve_satisfied: Option<bool>,
    /// Obligation ids on record as not established or refuted.
    pub unestablished: Vec<String>,
    /// §36.3 blocking unknowns, by assumption id.
    pub blocking_unknowns: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub milestones: Option<Vec<MilestoneState>>,
}

/// One §106 requirement and its §34.3 status, derived.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementLadder {
    pub requirement: RequirementRef,
    /// The §106 text, when the SAS could be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// §34.3, from [`crate::traceability::derive_status`]. `satisfied` needs a
    /// resolved Warrant; nothing derives it from "would satisfy".
    pub status: RequirementStatus,
    /// Implementers at `WouldSatisfy`. Forward-looking, and kept apart from
    /// `status` so the two cannot be read as one.
    pub would_satisfy: usize,
    pub links: Vec<Implements>,
}

/// A stage an agent could pick up now, and the one-sentence reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageRef {
    pub warrant: String,
    pub milestone: String,
    pub stage: String,
    pub objective: RoadmapRef,
    pub why: String,
}

/// Why `next_actionable` is empty, when it is. Never silently empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NothingActionable {
    pub objective: Option<RoadmapRef>,
    pub blocked_by: Vec<String>,
    pub why: String,
}

/// The Release axis. One entry, versionless, until OW-WAR-0058 records
/// accepted SAS revisions; the projection does not invent a version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseSummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    pub requirements: RequirementCounts,
    pub note: String,
}

/// The whole corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusStatus {
    pub schema: String,
    pub provenance: Provenance,
    /// One line explaining why every Warrant reads the way it does.
    pub provenance_note: String,
    pub release: ReleaseSummary,
    pub objectives: Vec<ObjectiveStatus>,
    pub warrants: Vec<WarrantStatus>,
    pub requirements: Vec<RequirementLadder>,
    pub next_actionable: Vec<StageRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nothing_actionable: Option<NothingActionable>,
    /// Facts the projection found that a reader should know before reading the
    /// numbers. The roadmap document's hand-written "resolved" column is one.
    pub caveats: Vec<String>,
}

impl CorpusStatus {
    #[must_use]
    pub fn warrant_ladder(&self) -> WarrantLadder {
        let mut l = WarrantLadder::default();
        for w in &self.warrants {
            l.count(w.rung);
        }
        l
    }

    #[must_use]
    pub fn requirement_counts(&self) -> RequirementCounts {
        let mut c = RequirementCounts::default();
        for r in &self.requirements {
            c.count(r.status);
        }
        c
    }

    /// How many Warrants are blocked on each unmet requirement, by name.
    ///
    /// A histogram of NAMES, which is the form that says what to fix — not a
    /// count of met requirements, which would say only how far along to feel.
    #[must_use]
    pub fn blockers_by_requirement(&self) -> BTreeMap<String, usize> {
        let mut m = BTreeMap::new();
        for w in &self.warrants {
            for u in &w.unmet {
                *m.entry(u.clone()).or_insert(0) += 1;
            }
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases_cover_zero_through_ten_and_only_nine_have_exits() {
        assert_eq!(PHASES.len(), 11);
        for (i, (n, _, _)) in PHASES.iter().enumerate() {
            assert_eq!(*n as usize, i);
        }
        let with_exit = PHASES.iter().filter(|(_, _, e)| e.is_some()).count();
        assert_eq!(
            with_exit, 9,
            "§98 gives phases 0..=8 an Exit; 9 and 10 have none"
        );
        assert!(phase(11).is_none());
    }

    #[test]
    fn resolved_is_reachable_only_from_a_record() {
        let all = ResolutionChecks::all_met();
        assert_eq!(
            WarrantRung::derive(true, Some(&all), Some(true), false),
            WarrantRung::WouldSatisfy,
            "thirteen met and §38.6 true is still not resolved"
        );
        assert_eq!(
            WarrantRung::derive(true, Some(&all), Some(true), true),
            WarrantRung::Resolved
        );
        assert_eq!(
            WarrantRung::derive(true, Some(&all), Some(false), false),
            WarrantRung::ReadyToResolve
        );
        assert_eq!(
            WarrantRung::derive(true, Some(&ResolutionChecks::default()), Some(true), false),
            WarrantRung::Draft,
            "§38.6 cannot lift a Warrant past an unmet requirement"
        );
        assert_eq!(
            WarrantRung::derive(false, Some(&all), Some(true), true),
            WarrantRung::Invalid
        );
    }

    #[test]
    fn rungs_order_from_least_to_most_done() {
        assert!(WarrantRung::Invalid < WarrantRung::Draft);
        assert!(WarrantRung::Draft < WarrantRung::ReadyToResolve);
        assert!(WarrantRung::ReadyToResolve < WarrantRung::WouldSatisfy);
        assert!(WarrantRung::WouldSatisfy < WarrantRung::Resolved);
    }

    #[test]
    fn ladders_count_every_rung_and_nothing_else() {
        let mut l = WarrantLadder::default();
        for r in [
            WarrantRung::Invalid,
            WarrantRung::Draft,
            WarrantRung::Draft,
            WarrantRung::WouldSatisfy,
        ] {
            l.count(r);
        }
        assert_eq!(l.total(), 4);
        assert_eq!(l.draft, 2);
        assert_eq!(l.resolved, 0);

        let mut c = RequirementCounts::default();
        c.count(RequirementStatus::Claimed);
        c.count(RequirementStatus::Unaddressed);
        assert_eq!(c.total(), 2);
        assert_eq!(c.satisfied, 0);
    }
}
