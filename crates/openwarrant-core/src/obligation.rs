// SPDX-License-Identifier: AGPL-3.0-or-later
//! Acceptance obligations and bounded claims (SAS §38; RQ-050, RQ-051).
//!
//! §38.1: "A completion summary SHALL be decomposed into bounded acceptance
//! obligations." Not one prose claim — a set of them, each bounded, each
//! separately disposable.
//!
//! # Designed against the real corpus
//!
//! The schema here was fitted to the 134 obligations already written in this
//! repository rather than derived from §38.2 in the abstract. Measured across
//! them: **134 of 134 carry both a `scope` and an `evidence` statement**, three
//! carry a `note`, two carry a prose rationale. That unanimity is why `scope`
//! and `evidence` are required fields and everything else is optional — the
//! authors converged on that shape before any parser existed.
//!
//! §38.2's fuller YAML schema (criticality, verification methods, gate binding
//! refs, known gaps, residual risk refs) is representable but not yet required,
//! because gate bindings do not exist until OW-WAR-0019 and requiring a
//! reference to a thing that cannot exist would make every obligation invalid.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ObligationError {
    #[error("obligation {id:?} declares no scope; §38.4 requires a claim to be bounded")]
    MissingScope { id: String },
    #[error("obligation {id:?} declares no evidence; an obligation with no evidence is a wish")]
    MissingEvidence { id: String },
    #[error("duplicate obligation id {id:?}")]
    DuplicateId { id: String },
    #[error("unknown scope kind {found:?}; expected one of {known}")]
    UnknownScopeKind { found: String, known: String },
    #[error("unknown disposition {found:?}; expected one of {known}")]
    UnknownDisposition { found: String, known: String },
    #[error(
        "obligation {id:?} claims universal scope but its verification is {method:?}. \
         §38.4: a universal claim requires an argument capable of supporting universal \
         scope — sampling alone is insufficient"
    )]
    UniversalBySampling { id: String, method: String },
    #[error("obligation {id:?} is disposed not_applicable with no authorized reason (§38.5)")]
    NotApplicableWithoutReason { id: String },
    #[error("milestone {milestone:?} references obligation {obligation:?}, which is not declared")]
    DanglingObligationRef {
        milestone: String,
        obligation: String,
    },
}

/// How far a claim reaches (SAS §38.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeKind {
    SingleInstance,
    EnumeratedSet,
    BoundedDomain,
    BoundedCorpus,
    SampledPopulation,
    TemporalWindow,
    Existential,
    Universal,
    FormalModel,
}

impl ScopeKind {
    pub const ALL: [Self; 9] = [
        Self::SingleInstance,
        Self::EnumeratedSet,
        Self::BoundedDomain,
        Self::BoundedCorpus,
        Self::SampledPopulation,
        Self::TemporalWindow,
        Self::Existential,
        Self::Universal,
        Self::FormalModel,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SingleInstance => "single_instance",
            Self::EnumeratedSet => "enumerated_set",
            Self::BoundedDomain => "bounded_domain",
            Self::BoundedCorpus => "bounded_corpus",
            Self::SampledPopulation => "sampled_population",
            Self::TemporalWindow => "temporal_window",
            Self::Existential => "existential",
            Self::Universal => "universal",
            Self::FormalModel => "formal_model",
        }
    }

    /// Whether this scope can be established by sampling alone.
    ///
    /// §38.4 singles out universal claims: "Sampling alone is insufficient."
    #[must_use]
    pub const fn satisfiable_by_sampling(self) -> bool {
        match self {
            Self::Universal => false,
            Self::SingleInstance
            | Self::EnumeratedSet
            | Self::BoundedDomain
            | Self::BoundedCorpus
            | Self::SampledPopulation
            | Self::TemporalWindow
            | Self::Existential
            | Self::FormalModel => true,
        }
    }
}

impl FromStr for ScopeKind {
    type Err = ObligationError;
    fn from_str(s: &str) -> Result<Self, ObligationError> {
        Self::ALL
            .into_iter()
            .find(|k| k.as_str() == s)
            .ok_or_else(|| ObligationError::UnknownScopeKind {
                found: s.to_owned(),
                known: Self::ALL
                    .iter()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            })
    }
}

impl fmt::Display for ScopeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What was concluded about one obligation (SAS §38.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Established,
    Refuted,
    NotEstablished,
    AcceptedWithResidualRisk,
    NotApplicable,
}

impl Disposition {
    pub const ALL: [Self; 5] = [
        Self::Established,
        Self::Refuted,
        Self::NotEstablished,
        Self::AcceptedWithResidualRisk,
        Self::NotApplicable,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Established => "established",
            Self::Refuted => "refuted",
            Self::NotEstablished => "not_established",
            Self::AcceptedWithResidualRisk => "accepted_with_residual_risk",
            Self::NotApplicable => "not_applicable",
        }
    }

    /// Whether this disposition permits a `satisfied` resolution (§38.6).
    ///
    /// §38.6: a delivery normally resolves satisfied only when all required
    /// obligations are established, or accepted with residual risk under
    /// sufficient authority. `NotApplicable` is deliberately NOT here — §38.5
    /// requires an authorized reason for it, so whether it permits satisfaction
    /// depends on that authorization and cannot be decided by the disposition
    /// alone.
    #[must_use]
    pub const fn permits_satisfied(self) -> bool {
        match self {
            Self::Established | Self::AcceptedWithResidualRisk => true,
            Self::Refuted | Self::NotEstablished | Self::NotApplicable => false,
        }
    }
}

impl FromStr for Disposition {
    type Err = ObligationError;
    fn from_str(s: &str) -> Result<Self, ObligationError> {
        Self::ALL
            .into_iter()
            .find(|d| d.as_str() == s)
            .ok_or_else(|| ObligationError::UnknownDisposition {
                found: s.to_owned(),
                known: Self::ALL
                    .iter()
                    .map(|d| d.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            })
    }
}

impl fmt::Display for Disposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One acceptance obligation (SAS §38.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Obligation {
    pub id: String,
    pub statement: String,
    /// §38.4 — the bound on the claim. Required: 134 of 134 obligations in this
    /// repository already state one.
    pub scope: String,
    /// How the claim would be established. Required for the same reason.
    pub evidence: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_kind: Option<ScopeKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disposition: Option<Disposition>,
}

impl Obligation {
    fn validate(&self) -> Result<(), ObligationError> {
        if self.scope.trim().is_empty() {
            return Err(ObligationError::MissingScope {
                id: self.id.clone(),
            });
        }
        if self.evidence.trim().is_empty() {
            return Err(ObligationError::MissingEvidence {
                id: self.id.clone(),
            });
        }
        // §38.4 — a universal claim cannot rest on sampling.
        if self.scope_kind == Some(ScopeKind::Universal) {
            let lower = self.evidence.to_lowercase();
            for word in [
                "sample",
                "sampling",
                "spot check",
                "a few",
                "representative",
            ] {
                if lower.contains(word) {
                    return Err(ObligationError::UniversalBySampling {
                        id: self.id.clone(),
                        method: word.to_owned(),
                    });
                }
            }
        }
        Ok(())
    }
}

/// The obligations of one Warrant.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ObligationSet {
    pub obligations: Vec<Obligation>,
}

impl ObligationSet {
    #[must_use]
    pub fn ids(&self) -> BTreeSet<&str> {
        self.obligations.iter().map(|o| o.id.as_str()).collect()
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Obligation> {
        self.obligations.iter().find(|o| o.id == id)
    }

    /// §38.6 — aggregate dispositions into a resolution recommendation.
    ///
    /// Returns `None` when any obligation has no disposition: a delivery whose
    /// obligations have not all been disposed has not been assessed, and
    /// answering `satisfied` or `not_satisfied` for it would be inventing a
    /// conclusion. §38.1 exists precisely so that a single verdict cannot stand
    /// in for the set.
    #[must_use]
    pub fn aggregate(&self) -> Option<bool> {
        if self.obligations.is_empty() {
            return None;
        }
        if self.obligations.iter().any(|o| o.disposition.is_none()) {
            return None;
        }
        Some(
            self.obligations
                .iter()
                .all(|o| o.disposition.is_some_and(Disposition::permits_satisfied)),
        )
    }

    /// Obligations with no disposition yet.
    #[must_use]
    pub fn undisposed(&self) -> Vec<&str> {
        self.obligations
            .iter()
            .filter(|o| o.disposition.is_none())
            .map(|o| o.id.as_str())
            .collect()
    }

    /// Check that every `obligation_refs` entry resolves (RQ-050).
    ///
    /// These dangled unchecked until now: a milestone could cite proof that was
    /// never written.
    pub fn check_references(
        &self,
        milestone_refs: &BTreeMap<String, Vec<String>>,
    ) -> Result<(), ObligationError> {
        let ids = self.ids();
        for (milestone, refs) in milestone_refs {
            for r in refs {
                if !ids.contains(r.as_str()) {
                    return Err(ObligationError::DanglingObligationRef {
                        milestone: milestone.clone(),
                        obligation: r.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

/// Parse obligations from an assurance atom.
///
/// The shape is the one the corpus already uses:
///
/// ```text
/// ### OBL-001 — the statement
/// - **scope:** ...
/// - **evidence:** ...
/// ```
///
/// Fitted to the corpus rather than imposed on it: all 134 existing obligations
/// parse without being rewritten.
pub fn parse(source: &str) -> Result<ObligationSet, ObligationError> {
    let mut obligations: Vec<Obligation> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut current: Option<Obligation> = None;
    // Which bullet we are accumulating into, for multi-line values.
    let mut field: Option<&'static str> = None;

    let push = |current: &mut Option<Obligation>,
                obligations: &mut Vec<Obligation>,
                seen: &mut BTreeSet<String>|
     -> Result<(), ObligationError> {
        if let Some(o) = current.take() {
            if !seen.insert(o.id.clone()) {
                return Err(ObligationError::DuplicateId { id: o.id });
            }
            o.validate()?;
            obligations.push(o);
        }
        Ok(())
    };

    for line in source.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("### ") {
            push(&mut current, &mut obligations, &mut seen)?;
            field = None;
            // `OBL-001 — statement` or `OBL-001 - statement`.
            let (id, statement) = rest
                .split_once('—')
                .or_else(|| rest.split_once(" - "))
                .unwrap_or((rest, ""));
            let id = id.trim().to_owned();
            if !id.starts_with("OBL-") {
                // A heading that is not an obligation ends the current one.
                continue;
            }
            current = Some(Obligation {
                id,
                statement: statement.trim().to_owned(),
                scope: String::new(),
                evidence: String::new(),
                scope_kind: None,
                note: None,
                disposition: None,
            });
            continue;
        }

        // A `##` heading closes the obligation list for this section.
        if trimmed.starts_with("## ") {
            push(&mut current, &mut obligations, &mut seen)?;
            field = None;
            continue;
        }

        let Some(o) = current.as_mut() else { continue };

        let bullet = trimmed
            .strip_prefix("- **")
            .and_then(|r| r.split_once(":**"))
            .map(|(k, v)| (k.trim().to_lowercase(), v.trim().to_owned()));

        match bullet {
            Some((key, value)) => {
                field = match key.as_str() {
                    "scope" => {
                        o.scope = value;
                        Some("scope")
                    }
                    "evidence" => {
                        o.evidence = value;
                        Some("evidence")
                    }
                    "note" => {
                        o.note = Some(value);
                        Some("note")
                    }
                    "disposition" => {
                        o.disposition = Some(Disposition::from_str(&value)?);
                        None
                    }
                    "scope kind" | "scope_kind" => {
                        o.scope_kind = Some(ScopeKind::from_str(&value)?);
                        None
                    }
                    // An unrecognised bullet is prose, not an error: the corpus
                    // carries rationale bullets and dropping them would be worse
                    // than ignoring them.
                    _ => None,
                };
            }
            None if !trimmed.is_empty() => {
                // Continuation of the current field's value.
                match field {
                    Some("scope") => {
                        o.scope.push(' ');
                        o.scope.push_str(trimmed);
                    }
                    Some("evidence") => {
                        o.evidence.push(' ');
                        o.evidence.push_str(trimmed);
                    }
                    _ => {}
                }
            }
            None => field = None,
        }
    }
    push(&mut current, &mut obligations, &mut seen)?;

    Ok(ObligationSet { obligations })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r"# Assurance

## Acceptance Obligations

### OBL-001 — the workspace builds
- **scope:** the workspace members, on Linux x86-64 only.
- **evidence:** `cargo build` exit status.

### OBL-002 — the gate has been observed to REJECT
- **scope:** the gate as a control, not the code under it.
- **evidence:** a recorded run in which the gate exits non-zero.
- **note:** OBL-001 alone is satisfiable by a gate that checks nothing.

## Gate Adequacy

Not required at basic.
";

    #[test]
    fn parses_the_corpus_shape() {
        let set = parse(SAMPLE).expect("valid");
        assert_eq!(set.obligations.len(), 2);
        assert_eq!(set.obligations[0].id, "OBL-001");
        assert_eq!(set.obligations[0].statement, "the workspace builds");
        assert!(set.obligations[0].scope.contains("Linux x86-64"));
        assert!(set.obligations[1].note.is_some());
    }

    /// OW-WAR-0016 OBL-001: the schema must fit the REAL corpus, not a fixture.
    /// All obligations in this repository parse without being rewritten.
    #[test]
    fn every_obligation_in_this_repository_parses() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/warrants");
        let mut total = 0;
        let mut warrants = 0;
        for entry in std::fs::read_dir(dir).expect("warrants") {
            let path = entry.expect("entry").path().join("atoms/60-assurance.md");
            if !path.is_file() {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("readable");
            let set = parse(&text).unwrap_or_else(|e| panic!("{} invalid: {e}", path.display()));
            assert!(
                !set.obligations.is_empty(),
                "{} declares no obligations",
                path.display()
            );
            total += set.obligations.len();
            warrants += 1;
        }
        assert!(warrants >= 40, "expected the whole corpus, saw {warrants}");
        // Measured before the parser was written; if this drops, obligations are
        // being lost rather than parsed.
        assert!(
            total >= 130,
            "expected ~134 obligations across the corpus, parsed {total}"
        );
    }

    /// §38.4 — scope is required, because an unbounded claim is not checkable.
    #[test]
    fn an_obligation_without_scope_is_refused() {
        let bad = "### OBL-001 — a claim\n- **evidence:** something\n";
        assert_eq!(
            parse(bad),
            Err(ObligationError::MissingScope {
                id: "OBL-001".to_owned()
            })
        );
    }

    #[test]
    fn an_obligation_without_evidence_is_refused() {
        let bad = "### OBL-001 — a claim\n- **scope:** everything\n";
        assert_eq!(
            parse(bad),
            Err(ObligationError::MissingEvidence {
                id: "OBL-001".to_owned()
            })
        );
    }

    #[test]
    fn duplicate_obligation_ids_are_refused() {
        let bad = "### OBL-001 — a\n- **scope:** s\n- **evidence:** e\n\
                   ### OBL-001 — b\n- **scope:** s\n- **evidence:** e\n";
        assert_eq!(
            parse(bad),
            Err(ObligationError::DuplicateId {
                id: "OBL-001".to_owned()
            })
        );
    }

    /// §38.4 — "Sampling alone is insufficient" for a universal claim.
    #[test]
    fn a_universal_claim_established_by_sampling_is_refused() {
        let bad = "### OBL-001 — everything works\n\
                   - **scope kind:** universal\n\
                   - **scope:** every input\n\
                   - **evidence:** a representative sample of inputs\n";
        assert!(matches!(
            parse(bad),
            Err(ObligationError::UniversalBySampling { .. })
        ));
    }

    #[test]
    fn a_bounded_claim_may_use_sampling() {
        let ok = "### OBL-001 — the corpus decodes\n\
                  - **scope kind:** sampled_population\n\
                  - **scope:** a sample of the corpus\n\
                  - **evidence:** a representative sample\n";
        parse(ok).expect("sampling is legitimate for a sampled population");
        assert!(!ScopeKind::Universal.satisfiable_by_sampling());
        assert!(ScopeKind::SampledPopulation.satisfiable_by_sampling());
    }

    // --- §38.6 aggregation ---

    fn with_dispositions(ds: &[Option<Disposition>]) -> ObligationSet {
        ObligationSet {
            obligations: ds
                .iter()
                .enumerate()
                .map(|(i, d)| Obligation {
                    id: format!("OBL-{:03}", i + 1),
                    statement: "x".to_owned(),
                    scope: "s".to_owned(),
                    evidence: "e".to_owned(),
                    scope_kind: None,
                    note: None,
                    disposition: *d,
                })
                .collect(),
        }
    }

    /// §38.1's whole point: a verdict cannot stand in for the set. An
    /// undisposed obligation means the delivery has not been assessed.
    #[test]
    fn aggregation_refuses_to_conclude_with_an_undisposed_obligation() {
        let set = with_dispositions(&[Some(Disposition::Established), None]);
        assert_eq!(set.aggregate(), None, "no verdict without full disposition");
        assert_eq!(set.undisposed(), vec!["OBL-002"]);
    }

    #[test]
    fn aggregation_matches_38_6() {
        assert_eq!(
            with_dispositions(&[
                Some(Disposition::Established),
                Some(Disposition::AcceptedWithResidualRisk)
            ])
            .aggregate(),
            Some(true),
            "§38.6: established, or accepted with residual risk"
        );
        assert_eq!(
            with_dispositions(&[Some(Disposition::Established), Some(Disposition::Refuted)])
                .aggregate(),
            Some(false)
        );
        assert_eq!(
            with_dispositions(&[Some(Disposition::NotEstablished)]).aggregate(),
            Some(false)
        );
    }

    /// §38.5 — `not_applicable` requires an authorized reason, so the
    /// disposition alone must not permit satisfaction.
    #[test]
    fn not_applicable_does_not_permit_satisfaction_by_itself() {
        assert!(!Disposition::NotApplicable.permits_satisfied());
        assert_eq!(
            with_dispositions(&[Some(Disposition::NotApplicable)]).aggregate(),
            Some(false)
        );
    }

    #[test]
    fn an_empty_set_has_no_verdict() {
        assert_eq!(ObligationSet::default().aggregate(), None);
    }

    /// RQ-050 — dangling `obligation_refs` are refused. They dangled unchecked
    /// until this Warrant.
    #[test]
    fn a_dangling_obligation_ref_is_refused() {
        let set = parse(SAMPLE).expect("valid");
        let mut refs = BTreeMap::new();
        refs.insert("M1".to_owned(), vec!["OBL-999".to_owned()]);
        assert_eq!(
            set.check_references(&refs),
            Err(ObligationError::DanglingObligationRef {
                milestone: "M1".to_owned(),
                obligation: "OBL-999".to_owned()
            })
        );

        let mut good = BTreeMap::new();
        good.insert("M1".to_owned(), vec!["OBL-001".to_owned()]);
        assert_eq!(set.check_references(&good), Ok(()));
    }

    /// §38.3 and §38.5 vocabularies, transcribed as external expectations.
    #[test]
    fn vocabularies_match_the_sas() {
        assert_eq!(
            ScopeKind::ALL
                .iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>(),
            [
                "single_instance",
                "enumerated_set",
                "bounded_domain",
                "bounded_corpus",
                "sampled_population",
                "temporal_window",
                "existential",
                "universal",
                "formal_model",
            ]
        );
        assert_eq!(
            Disposition::ALL
                .iter()
                .map(|d| d.as_str())
                .collect::<Vec<_>>(),
            [
                "established",
                "refuted",
                "not_established",
                "accepted_with_residual_risk",
                "not_applicable",
            ]
        );
    }
}
