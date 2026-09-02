// SPDX-License-Identifier: AGPL-3.0-or-later
//! SAS traceability (SAS §34, RQ-022).
//!
//! # The sentence this enforces
//!
//! §34.3: *"SAS requirement status is derived from linked WARs and evidence. The
//! SAS source itself SHALL NOT be edited merely to tick completion boxes."*
//!
//! Two consequences, both structural. Requirement status is a **derivation**, so
//! [`derive_status`] is a function of the linked Warrants rather than a field
//! anyone can set — there is no `set_status`. And §34.4 gives the only legitimate
//! route to changing a requirement: open an ADR, propose a controlled revision,
//! supersede affected Warrants, and preserve the original. [`ArchitectureChange`]
//! is those four steps, and it refuses to be complete with any of them missing.
//!
//! A Warrant claiming `complete` against a requirement while carrying no resolved
//! obligations is the shape of a ticked box, and [`derive_status`] does not let
//! the claim alone produce `satisfied`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::vocab::vocabulary;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TraceabilityError {
    #[error("unknown {vocabulary} {found:?}; SAS defines {known}")]
    UnknownTerm {
        vocabulary: &'static str,
        found: String,
        known: String,
    },
    #[error(
        "malformed requirement reference {found:?}; §34.1 requires a stable \
         identifier of the form <PREFIX>-SAS-RQ-<NNN>, optionally as sas://<id>: \
         an uppercase prefix and a number zero-padded to at least three digits, so \
         that RQ-1 and RQ-001 cannot be two spellings of one requirement"
    )]
    MalformedRequirementRef { found: String },
    #[error(
        "malformed roadmap reference {found:?}; expected \
         roadmap://<PREFIX>-PHASE-<N>[/<slug>] with an uppercase prefix, a phase \
         number 0..=10 written without padding, and a slug of [a-z0-9-]. This is \
         OpenWarrant's convention for §105's roadmap://<item-id>"
    )]
    MalformedRoadmapRef { found: String },
    #[error(
        "Warrant {warrant:?} claims contribution {contribution} to {requirement} but \
         is not resolved and has no evidence. §34.3: requirement status is derived \
         from linked WARs and evidence, not from the claim"
    )]
    UnevidencedClaim {
        warrant: String,
        requirement: String,
        contribution: Contribution,
    },
    #[error(
        "architecture change for {requirement} omits step {step}. §34.4 requires all \
         four: open an ADR, propose a controlled SAS revision, supersede or amend \
         affected WARs, and preserve the original requirement and evidence history"
    )]
    ArchitectureChangeIncomplete {
        requirement: String,
        step: &'static str,
    },
    #[error(
        "architecture change for {requirement} does not preserve the original \
         requirement and evidence history. §34.4 step 4 exists so that a \
         requirement that turned out to be wrong leaves a record of having been \
         believed"
    )]
    HistoryNotPreserved { requirement: String },
}

vocabulary!(
    /// §34.2's contribution values.
    Contribution, "contribution", TraceabilityError, {
        Partial => "partial",
        Complete => "complete",
        Validation => "validation",
        Investigation => "investigation",
        Supersession => "supersession",
    }
);

impl Contribution {
    /// Whether this contribution, once evidenced, can make a requirement
    /// satisfied on its own.
    ///
    /// `partial` cannot by definition. `investigation` cannot either: finding out
    /// about a requirement is not meeting it, and this is the distinction that
    /// stops a research Warrant closing a delivery requirement.
    #[must_use]
    pub const fn can_satisfy_alone(self) -> bool {
        matches!(self, Self::Complete | Self::Supersession)
    }
}

vocabulary!(
    /// Derived requirement status (§34.3). Derived — never assigned.
    RequirementStatus, "requirement status", TraceabilityError, {
        Unaddressed => "unaddressed",
        Claimed => "claimed",
        InProgress => "in_progress",
        Satisfied => "satisfied",
        Superseded => "superseded",
    }
);

/// A stable SAS requirement identifier (§34.1).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RequirementRef {
    /// The system prefix: `WAR`, `LIM`, `LMQ`.
    pub prefix: String,
    pub number: u32,
}

impl RequirementRef {
    /// Parse `WAR-SAS-RQ-001`, or `sas://WAR-SAS-RQ-001`.
    pub fn parse(text: &str) -> Result<Self, TraceabilityError> {
        let bare = text.trim().strip_prefix("sas://").unwrap_or(text.trim());
        let malformed = || TraceabilityError::MalformedRequirementRef {
            found: text.to_owned(),
        };
        let (prefix, rest) = bare.split_once("-SAS-RQ-").ok_or_else(malformed)?;
        if prefix.is_empty() || !prefix.bytes().all(|b| b.is_ascii_uppercase()) {
            return Err(malformed());
        }
        // Fixed-width, zero-padded. `RQ-1` and `RQ-001` naming the same
        // requirement would make the identifier unstable, which §34.1's word
        // "stable" rules out.
        if rest.len() < 3 || !rest.bytes().all(|b| b.is_ascii_digit()) {
            return Err(malformed());
        }
        Ok(Self {
            prefix: prefix.to_owned(),
            number: rest.parse().map_err(|_| malformed())?,
        })
    }

    #[must_use]
    pub fn canonical(&self) -> String {
        format!("{}-SAS-RQ-{:03}", self.prefix, self.number)
    }
}

impl std::fmt::Display for RequirementRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.canonical())
    }
}

/// A reference into the Production Roadmap (§6.3, §105 `roadmap://<item-id>`).
///
/// # Why this has a grammar at all
///
/// §105 gives only the URI scheme; the item-id's shape is left to the
/// repository. Until this type existed the manifest carried the ref as an
/// unparsed `String` and `war show` said outright that only `war://` was
/// resolved — so `roadmap://OW-PHASE-11/x` and `roadmap://typo` were both
/// accepted, and nothing could group Warrants by phase because nothing could
/// read the phase.
///
/// The grammar is OpenWarrant's convention, and is said so in the error text:
/// `<PREFIX>-PHASE-<N>`, N in 0..=10 (SAS §98's eleven phases), then an optional
/// `/slug`. The SAS's own example (`roadmap://LIM-PHASE-1/M4`) uses a milestone
/// id as the slug; this corpus uses topic words. Both parse. The one slug with
/// meaning is `exit` — the Warrant that discharges §98's Exit criterion for its
/// phase — and [`RoadmapRef::is_exit`] names it so a projection can ask.
///
/// The phase number is unpadded on purpose. `PHASE-1` and `PHASE-01` naming one
/// phase would make the reference unstable, and §34.1's argument for
/// zero-padding requirement numbers runs the other way here: there are eleven
/// phases, they are written `Phase 1` in the SAS, and a leading zero would be a
/// second spelling rather than a fixed width.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RoadmapRef {
    /// The system prefix: `OW`, `LIM`.
    pub prefix: String,
    /// SAS §98 phase, 0..=10.
    pub phase: u8,
    /// The item within the phase. `None` names the phase itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
}

impl RoadmapRef {
    /// The last phase §98 defines. A reference past it names nothing.
    pub const MAX_PHASE: u8 = 10;

    /// Parse `roadmap://OW-PHASE-6/gate-runs`, `roadmap://OW-PHASE-6`, or the
    /// same without the scheme.
    pub fn parse(text: &str) -> Result<Self, TraceabilityError> {
        let malformed = || TraceabilityError::MalformedRoadmapRef {
            found: text.to_owned(),
        };
        let bare = text.trim();
        let bare = bare.strip_prefix("roadmap://").unwrap_or(bare);
        let (head, slug) = match bare.split_once('/') {
            Some((h, s)) => (h, Some(s)),
            None => (bare, None),
        };

        let (prefix, number) = head.split_once("-PHASE-").ok_or_else(malformed)?;
        if prefix.is_empty() || !prefix.bytes().all(|b| b.is_ascii_uppercase()) {
            return Err(malformed());
        }
        // Unpadded: "0" is fine, "01" is not, "" is not.
        if number.is_empty()
            || !number.bytes().all(|b| b.is_ascii_digit())
            || (number.len() > 1 && number.starts_with('0'))
        {
            return Err(malformed());
        }
        let phase: u8 = number.parse().map_err(|_| malformed())?;
        if phase > Self::MAX_PHASE {
            return Err(malformed());
        }

        let slug = match slug {
            None => None,
            Some(s) => {
                if s.is_empty()
                    || !s
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                {
                    return Err(malformed());
                }
                Some(s.to_owned())
            }
        };

        Ok(Self {
            prefix: prefix.to_owned(),
            phase,
            slug,
        })
    }

    /// The reference to the phase alone, with any slug dropped.
    #[must_use]
    pub fn phase_ref(&self) -> Self {
        Self {
            prefix: self.prefix.clone(),
            phase: self.phase,
            slug: None,
        }
    }

    /// Whether this names the phase's exit Warrant — the one that discharges
    /// §98's Exit criterion. The only slug this corpus gives a meaning to.
    #[must_use]
    pub fn is_exit(&self) -> bool {
        self.slug.as_deref() == Some("exit")
    }

    #[must_use]
    pub fn canonical(&self) -> String {
        match &self.slug {
            Some(s) => format!("roadmap://{}-PHASE-{}/{s}", self.prefix, self.phase),
            None => format!("roadmap://{}-PHASE-{}", self.prefix, self.phase),
        }
    }
}

impl std::fmt::Display for RoadmapRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.canonical())
    }
}

/// §34.2's link from a Warrant to a requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Implements {
    pub warrant: String,
    pub requirement_ref: RequirementRef,
    pub intended_contribution: Contribution,
    /// Whether the Warrant is resolved. Status is derived from this, not from
    /// the contribution claim.
    #[serde(default)]
    pub warrant_resolved: bool,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

impl Implements {
    /// Whether this link contributes evidence rather than only a claim.
    #[must_use]
    pub fn is_evidenced(&self) -> bool {
        self.warrant_resolved && !self.evidence_refs.is_empty()
    }
}

/// §34.3 — derive a requirement's status from its links.
///
/// There is deliberately no setter. A requirement becomes `satisfied` because a
/// resolved Warrant with evidence claims `complete` or `supersession`; a claim
/// with nothing behind it reaches `claimed` and stops there, which is exactly the
/// ticked box §34.3 forbids, left visible rather than counted.
#[must_use]
pub fn derive_status(links: &[Implements]) -> RequirementStatus {
    if links.is_empty() {
        return RequirementStatus::Unaddressed;
    }
    if links
        .iter()
        .any(|l| l.intended_contribution == Contribution::Supersession && l.is_evidenced())
    {
        return RequirementStatus::Superseded;
    }
    if links
        .iter()
        .any(|l| l.intended_contribution.can_satisfy_alone() && l.is_evidenced())
    {
        return RequirementStatus::Satisfied;
    }
    if links.iter().any(Implements::is_evidenced) {
        // Something real happened, but nothing that closes it.
        return RequirementStatus::InProgress;
    }
    RequirementStatus::Claimed
}

/// Derive every requirement's status from a corpus of links.
#[must_use]
pub fn derive_all(links: &[Implements]) -> BTreeMap<String, RequirementStatus> {
    let mut by_req: BTreeMap<String, Vec<Implements>> = BTreeMap::new();
    for l in links {
        by_req
            .entry(l.requirement_ref.canonical())
            .or_default()
            .push(l.clone());
    }
    by_req
        .into_iter()
        .map(|(k, v)| (k, derive_status(&v)))
        .collect()
}

/// §34.4's four steps for changing a requirement that turned out to be wrong.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ArchitectureChange {
    pub requirement: String,
    /// Step 1 — open an ADR.
    pub adr_ref: String,
    /// Step 2 — propose a controlled SAS revision.
    pub proposed_revision_ref: String,
    /// Step 3 — supersede or amend affected WARs.
    #[serde(default)]
    pub affected_warrants: Vec<String>,
    /// Step 4 — preserve the original requirement and evidence history.
    #[serde(default)]
    pub original_preserved: bool,
    #[serde(default)]
    pub preserved_evidence_refs: Vec<String>,
}

impl ArchitectureChange {
    pub fn validate(&self) -> Result<(), TraceabilityError> {
        for (step, value) in [
            ("1 (open an ADR)", &self.adr_ref),
            (
                "2 (propose a controlled SAS revision)",
                &self.proposed_revision_ref,
            ),
        ] {
            if value.trim().is_empty() {
                return Err(TraceabilityError::ArchitectureChangeIncomplete {
                    requirement: self.requirement.clone(),
                    step,
                });
            }
        }
        if self.affected_warrants.is_empty() {
            return Err(TraceabilityError::ArchitectureChangeIncomplete {
                requirement: self.requirement.clone(),
                step: "3 (supersede or amend affected WARs)",
            });
        }
        if !self.original_preserved {
            return Err(TraceabilityError::HistoryNotPreserved {
                requirement: self.requirement.clone(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    /// §34.2's list, transcribed.
    #[test]
    fn the_contribution_values_match_the_sas() {
        assert_eq!(
            Contribution::ALL
                .iter()
                .map(|c| c.as_str())
                .collect::<Vec<_>>(),
            [
                "partial",
                "complete",
                "validation",
                "investigation",
                "supersession"
            ]
        );
    }

    /// §34.1's three worked identifiers.
    #[test]
    fn the_sas_example_identifiers_parse() {
        for (text, prefix, number) in [
            ("WAR-SAS-RQ-001", "WAR", 1),
            ("LIM-SAS-RQ-042", "LIM", 42),
            ("LMQ-SAS-RQ-117", "LMQ", 117),
        ] {
            let r = RequirementRef::parse(text).expect(text);
            assert_eq!(r.prefix, prefix);
            assert_eq!(r.number, number);
            assert_eq!(r.canonical(), text);
        }
        // §34.2 writes them as URIs.
        assert_eq!(
            RequirementRef::parse("sas://LIM-SAS-RQ-042").expect("uri form"),
            RequirementRef {
                prefix: "LIM".into(),
                number: 42
            }
        );
    }

    /// "Stable" rules out two spellings of one requirement.
    #[test]
    fn unstable_or_malformed_identifiers_are_refused() {
        for bad in [
            "WAR-SAS-RQ-1",   // not zero-padded: a second spelling of RQ-001
            "war-SAS-RQ-001", // prefix must be uppercase
            "-SAS-RQ-001",    // no prefix
            "WAR-RQ-001",     // not a SAS requirement reference
            "WAR-SAS-RQ-",    // no number
            "WAR-SAS-RQ-abc",
            "",
        ] {
            assert!(
                RequirementRef::parse(bad).is_err(),
                "{bad:?} should not parse"
            );
        }
    }

    fn link(warrant: &str, c: Contribution, resolved: bool, evidence: bool) -> Implements {
        Implements {
            warrant: warrant.into(),
            requirement_ref: RequirementRef {
                prefix: "WAR".into(),
                number: 22,
            },
            intended_contribution: c,
            warrant_resolved: resolved,
            evidence_refs: if evidence {
                vec!["evidence://run-1".into()]
            } else {
                vec![]
            },
        }
    }

    /// §34.3 — the ticked box. A claim with nothing behind it reaches `claimed`
    /// and goes no further.
    #[test]
    fn a_claim_alone_does_not_satisfy_a_requirement() {
        let claimed_only = [link("OW-WAR-0001", Contribution::Complete, false, false)];
        assert_eq!(derive_status(&claimed_only), RequirementStatus::Claimed);

        let resolved_no_evidence = [link("OW-WAR-0001", Contribution::Complete, true, false)];
        assert_eq!(
            derive_status(&resolved_no_evidence),
            RequirementStatus::Claimed,
            "resolved but unevidenced is still only a claim"
        );

        let evidenced = [link("OW-WAR-0001", Contribution::Complete, true, true)];
        assert_eq!(derive_status(&evidenced), RequirementStatus::Satisfied);
    }

    /// A partial contribution cannot close a requirement however well evidenced.
    #[test]
    fn partial_and_investigation_cannot_satisfy_alone() {
        for c in [
            Contribution::Partial,
            Contribution::Investigation,
            Contribution::Validation,
        ] {
            assert!(!c.can_satisfy_alone(), "{c} claimed to satisfy alone");
            let links = [link("OW-WAR-0001", c, true, true)];
            assert_eq!(
                derive_status(&links),
                RequirementStatus::InProgress,
                "{c} closed a requirement"
            );
        }
    }

    #[test]
    fn no_links_means_unaddressed() {
        assert_eq!(derive_status(&[]), RequirementStatus::Unaddressed);
    }

    #[test]
    fn an_evidenced_supersession_supersedes() {
        let links = [
            link("OW-WAR-0001", Contribution::Partial, true, true),
            link("OW-WAR-0002", Contribution::Supersession, true, true),
        ];
        assert_eq!(derive_status(&links), RequirementStatus::Superseded);
    }

    /// There is no setter. This test exists to state that as an expectation
    /// rather than leaving it to be noticed.
    #[test]
    fn status_is_derived_and_cannot_be_assigned() {
        let links = [link("OW-WAR-0001", Contribution::Complete, false, false)];
        let derived = derive_status(&links);
        assert_eq!(derived, RequirementStatus::Claimed);
        // The only way to change it is to change the facts.
        let with_evidence = [link("OW-WAR-0001", Contribution::Complete, true, true)];
        assert_eq!(derive_status(&with_evidence), RequirementStatus::Satisfied);
    }

    #[test]
    fn statuses_derive_across_a_corpus() {
        let mut a = link("OW-WAR-0001", Contribution::Complete, true, true);
        a.requirement_ref = RequirementRef {
            prefix: "WAR".into(),
            number: 22,
        };
        let mut b = link("OW-WAR-0002", Contribution::Partial, false, false);
        b.requirement_ref = RequirementRef {
            prefix: "WAR".into(),
            number: 54,
        };
        let all = derive_all(&[a, b]);
        assert_eq!(all["WAR-SAS-RQ-022"], RequirementStatus::Satisfied);
        assert_eq!(all["WAR-SAS-RQ-054"], RequirementStatus::Claimed);
    }

    // ---- §34.4 -----------------------------------------------------------

    fn change() -> ArchitectureChange {
        ArchitectureChange {
            requirement: "WAR-SAS-RQ-022".into(),
            adr_ref: "adr://OW-ADR-0007".into(),
            proposed_revision_ref: "revision://sas-v0.2.0-draft.1".into(),
            affected_warrants: vec!["OW-WAR-0013".into()],
            original_preserved: true,
            preserved_evidence_refs: vec!["evidence://original".into()],
        }
    }

    #[test]
    fn all_four_steps_are_required() {
        assert_eq!(change().validate(), Ok(()));

        let mut no_adr = change();
        no_adr.adr_ref.clear();
        assert!(matches!(
            no_adr.validate(),
            Err(TraceabilityError::ArchitectureChangeIncomplete { .. })
        ));

        let mut no_revision = change();
        no_revision.proposed_revision_ref.clear();
        assert!(matches!(
            no_revision.validate(),
            Err(TraceabilityError::ArchitectureChangeIncomplete { .. })
        ));

        let mut no_warrants = change();
        no_warrants.affected_warrants.clear();
        assert!(matches!(
            no_warrants.validate(),
            Err(TraceabilityError::ArchitectureChangeIncomplete { .. })
        ));
    }

    /// §34.4 step 4: a requirement that turned out to be wrong still leaves a
    /// record of having been believed.
    #[test]
    fn history_must_be_preserved() {
        let mut c = change();
        c.original_preserved = false;
        assert!(matches!(
            c.validate(),
            Err(TraceabilityError::HistoryNotPreserved { .. })
        ));
    }

    #[test]
    fn vocabularies_round_trip() {
        for &c in Contribution::ALL {
            assert_eq!(Contribution::from_str(c.as_str()), Ok(c));
        }
        for &s in RequirementStatus::ALL {
            assert_eq!(RequirementStatus::from_str(s.as_str()), Ok(s));
        }
        assert!(Contribution::from_str("done").is_err());
    }

    #[test]
    fn a_roadmap_ref_parses_with_and_without_a_slug() {
        let r = RoadmapRef::parse("roadmap://OW-PHASE-6/gate-runs").expect("parses");
        assert_eq!(r.prefix, "OW");
        assert_eq!(r.phase, 6);
        assert_eq!(r.slug.as_deref(), Some("gate-runs"));
        assert_eq!(r.canonical(), "roadmap://OW-PHASE-6/gate-runs");

        let p = RoadmapRef::parse("OW-PHASE-0").expect("scheme is optional");
        assert_eq!(p.phase, 0);
        assert_eq!(p.slug, None);
        assert_eq!(p.canonical(), "roadmap://OW-PHASE-0");
        assert_eq!(
            r.phase_ref(),
            RoadmapRef::parse("roadmap://OW-PHASE-6").expect("ok")
        );
    }

    /// §98 defines phases 0..=10. Each malformed form below is a distinct way
    /// a reference could name nothing while looking like it names something,
    /// and each must be refused on its own — a test that only checked one
    /// would pass against a parser that accepted the rest.
    #[test]
    fn a_roadmap_ref_outside_the_grammar_is_refused() {
        for bad in [
            "roadmap://OW-PHASE-11/x",   // past the last phase
            "roadmap://OW-PHASE-01/x",   // padded: a second spelling of phase 1
            "roadmap://OW-PHASE-/x",     // no number
            "roadmap://ow-PHASE-1/x",    // lowercase prefix
            "roadmap://OW-PHASE-1/",     // empty slug
            "roadmap://OW-PHASE-1/Gate", // uppercase slug
            "roadmap://OW-PHASE-1/a b",  // space
            "roadmap://OW-STAGE-1/x",    // wrong keyword
            "roadmap://typo",
            "",
        ] {
            assert!(
                matches!(
                    RoadmapRef::parse(bad),
                    Err(TraceabilityError::MalformedRoadmapRef { .. })
                ),
                "{bad:?} must be refused"
            );
        }
    }

    #[test]
    fn only_the_exit_slug_has_a_meaning() {
        assert!(
            RoadmapRef::parse("roadmap://OW-PHASE-5/exit")
                .expect("ok")
                .is_exit()
        );
        assert!(
            !RoadmapRef::parse("roadmap://OW-PHASE-5/dispatch")
                .expect("ok")
                .is_exit()
        );
        assert!(
            !RoadmapRef::parse("roadmap://OW-PHASE-5")
                .expect("ok")
                .is_exit()
        );
    }

    #[test]
    fn roadmap_refs_order_by_phase_then_slug() {
        let mut refs: Vec<RoadmapRef> = [
            "OW-PHASE-6/exit",
            "OW-PHASE-1/compiler",
            "OW-PHASE-6/adequacy",
        ]
        .iter()
        .map(|t| RoadmapRef::parse(t).expect("ok"))
        .collect();
        refs.sort();
        let order: Vec<String> = refs.iter().map(RoadmapRef::canonical).collect();
        assert_eq!(
            order,
            [
                "roadmap://OW-PHASE-1/compiler",
                "roadmap://OW-PHASE-6/adequacy",
                "roadmap://OW-PHASE-6/exit"
            ]
        );
    }
}
