// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deliverables, artifact provenance, and derived reports (SAS §37).
//!
//! # The sentence that shapes this module
//!
//! §37.3: *"A generated report SHALL NOT replace its source observations or
//! bytes."*
//!
//! A report is a claim about evidence, and once the evidence is gone the claim
//! cannot be checked. [`DerivedReport`] therefore cannot be constructed without
//! raw evidence references, and validation refuses one that has none.
//!
//! §37.4 makes the matching point about submissions: *"The submission manifest is
//! normally not a deliverable. It is a claim envelope."* [`SubmissionManifest`]
//! is a separate type from [`Deliverable`] for that reason — a performer
//! describing what it did is not the thing it was asked to produce.

use serde::{Deserialize, Serialize};

use crate::vocab::vocabulary;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DeliverableError {
    #[error("unknown {vocabulary} {found:?}; SAS defines {known}")]
    UnknownTerm {
        vocabulary: &'static str,
        found: String,
        known: String,
    },
    #[error("deliverable {id:?} states no {field}, which §37.1 requires")]
    DeliverableIncomplete { id: String, field: &'static str },
    #[error(
        "deliverable {id:?} is content_addressed but its artifact records no content \
         digest. Content addressing with no digest is a claim about identity with \
         nothing establishing it"
    )]
    ContentAddressedWithoutDigest { id: String },
    #[error(
        "deliverable {id:?} requires provenance and its artifact omits {field}. \
         §37.2 lists what registration SHALL record"
    )]
    ProvenanceIncomplete { id: String, field: &'static str },
    #[error(
        "deliverable {id:?} cites obligation {obligation:?}, which is not declared. \
         A deliverable pointing at proof nobody wrote is the defect §38 exists to \
         prevent"
    )]
    DanglingObligationRef { id: String, obligation: String },
    #[error(
        "report {id:?} references no raw evidence. §37.3: a report derived from \
         evidence SHALL reference the raw evidence, and a generated report SHALL \
         NOT replace its source observations or bytes"
    )]
    ReportWithoutEvidence { id: String },
    #[error(
        "report {id:?} declares itself a replacement for its sources. §37.3 forbids \
         exactly that: once the observations are gone the report cannot be checked"
    )]
    ReportReplacesSources { id: String },
    #[error(
        "submission {id:?} is declared a deliverable. §37.4: the submission manifest \
         is normally not a deliverable — it is a claim envelope, and treating a \
         performer's description of its work as the work is the substitution §40.7 \
         forbids"
    )]
    SubmissionAsDeliverable { id: String },
}

vocabulary!(
    /// What kind of thing a deliverable is (§37.1's `kind`).
    ///
    /// §37.1 shows `git_commit` and does not enumerate the rest, so this list is
    /// STRICTER than the specification: an unlisted kind fails to parse rather
    /// than being carried through.
    ///
    /// That is a deliberate local choice, not a reading of the SAS. A free-text
    /// kind cannot be reasoned about, and adding a variant is a one-line change
    /// with a compiler error pointing at every place that must consider it. If a
    /// real deliverable kind turns up that does not fit, add it — do not widen
    /// the type to a string.
    DeliverableKind, "deliverable kind", DeliverableError, {
        GitCommit => "git_commit",
        File => "file",
        Document => "document",
        Dataset => "dataset",
        Package => "package",
        Report => "report",
    }
);

/// §37.2's provenance record. Eleven fields, all of them required when a
/// deliverable sets `provenance_required`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ArtifactProvenance {
    pub producer: String,
    pub producing_attempt: String,
    pub contract_digest: String,
    #[serde(default)]
    pub input_digests: Vec<String>,
    pub tool_or_runtime_identity: String,
    pub creation_method: String,
    pub content_digest: String,
    pub media_type: String,
    pub classification: String,
    pub retention: String,
    pub source_holder: String,
}

impl ArtifactProvenance {
    /// The scalar fields §37.2 requires, in the specification's order.
    ///
    /// `input_digests` is excluded deliberately: an artifact produced from no
    /// inputs is legitimate, and requiring a non-empty list would push authors to
    /// invent one.
    fn required(&self) -> [(&'static str, &str); 10] {
        [
            ("producer", &self.producer),
            ("producing_attempt", &self.producing_attempt),
            ("contract_digest", &self.contract_digest),
            ("tool_or_runtime_identity", &self.tool_or_runtime_identity),
            ("creation_method", &self.creation_method),
            ("content_digest", &self.content_digest),
            ("media_type", &self.media_type),
            ("classification", &self.classification),
            ("retention", &self.retention),
            ("source_holder", &self.source_holder),
        ]
    }

    pub fn validate(&self, deliverable_id: &str) -> Result<(), DeliverableError> {
        for (field, value) in self.required() {
            if value.trim().is_empty() {
                return Err(DeliverableError::ProvenanceIncomplete {
                    id: deliverable_id.to_owned(),
                    field,
                });
            }
        }
        Ok(())
    }
}

/// §37.1's deliverable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deliverable {
    pub id: String,
    pub title: String,
    pub kind: DeliverableKind,
    pub target_ref: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub content_addressed: bool,
    #[serde(default)]
    pub provenance_required: bool,
    #[serde(default)]
    pub obligation_refs: Vec<String>,
    #[serde(default)]
    pub provenance: Option<ArtifactProvenance>,
}

impl Deliverable {
    /// Validate the deliverable, and resolve its obligation references against
    /// the obligations actually declared.
    pub fn validate(&self, declared_obligations: &[String]) -> Result<(), DeliverableError> {
        for (field, value) in [("title", &self.title), ("target_ref", &self.target_ref)] {
            if value.trim().is_empty() {
                return Err(DeliverableError::DeliverableIncomplete {
                    id: self.id.clone(),
                    field,
                });
            }
        }
        for obligation in &self.obligation_refs {
            if !declared_obligations.contains(obligation) {
                return Err(DeliverableError::DanglingObligationRef {
                    id: self.id.clone(),
                    obligation: obligation.clone(),
                });
            }
        }
        let provenance = self.provenance.clone().unwrap_or_default();
        if self.provenance_required {
            provenance.validate(&self.id)?;
        }
        if self.content_addressed && provenance.content_digest.trim().is_empty() {
            return Err(DeliverableError::ContentAddressedWithoutDigest {
                id: self.id.clone(),
            });
        }
        Ok(())
    }
}

/// §37.3's derived report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivedReport {
    pub id: String,
    /// The raw observations this report is about. §37.3 makes this the field
    /// that cannot be empty.
    #[serde(default)]
    pub raw_evidence_refs: Vec<String>,
    /// Whether the sources were discarded once the report existed. Recording it
    /// is what lets validation refuse it; a system that could not express the
    /// claim could not refuse it either.
    #[serde(default)]
    pub replaces_sources: bool,
}

impl DerivedReport {
    pub fn validate(&self) -> Result<(), DeliverableError> {
        if self.raw_evidence_refs.is_empty() {
            return Err(DeliverableError::ReportWithoutEvidence {
                id: self.id.clone(),
            });
        }
        if self.replaces_sources {
            return Err(DeliverableError::ReportReplacesSources {
                id: self.id.clone(),
            });
        }
        Ok(())
    }
}

/// §37.4's claim envelope — deliberately not a deliverable.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SubmissionManifest {
    pub id: String,
    #[serde(default)]
    pub artifact_refs: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub deviations: Vec<String>,
    #[serde(default)]
    pub requested_next_action: String,
    /// §37.4 says "normally not a deliverable". The exception has to be stated
    /// rather than assumed, so this defaults to false and validation refuses it.
    #[serde(default)]
    pub declared_as_deliverable: bool,
}

impl SubmissionManifest {
    pub fn validate(&self) -> Result<(), DeliverableError> {
        if self.declared_as_deliverable {
            return Err(DeliverableError::SubmissionAsDeliverable {
                id: self.id.clone(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn provenance() -> ArtifactProvenance {
        ArtifactProvenance {
            producer: "QuiteTall".into(),
            producing_attempt: "attempt://1".into(),
            contract_digest: "sha256:c".into(),
            input_digests: vec!["sha256:i".into()],
            tool_or_runtime_identity: "rustc 1.97.1".into(),
            creation_method: "cargo build --workspace".into(),
            content_digest: "sha256:d".into(),
            media_type: "application/x-git-commit".into(),
            classification: "internal".into(),
            retention: "indefinite".into(),
            source_holder: "git://Quitetall/OpenWarrant".into(),
        }
    }

    fn deliverable() -> Deliverable {
        Deliverable {
            id: "DEL-001".into(),
            title: "Scalar implementation commit".into(),
            kind: DeliverableKind::GitCommit,
            target_ref: "git://abc".into(),
            required: true,
            content_addressed: true,
            provenance_required: true,
            obligation_refs: vec!["OBL-001".into()],
            provenance: Some(provenance()),
        }
    }

    #[test]
    fn a_complete_deliverable_validates() {
        assert_eq!(deliverable().validate(&["OBL-001".to_owned()]), Ok(()));
    }

    /// The same defect OW-WAR-0016 caught for milestones: a reference to proof
    /// nobody wrote.
    #[test]
    fn a_deliverable_citing_an_undeclared_obligation_is_refused() {
        let mut d = deliverable();
        d.obligation_refs = vec!["OBL-999".into()];
        match d.validate(&["OBL-001".to_owned()]) {
            Err(DeliverableError::DanglingObligationRef { obligation, .. }) => {
                assert_eq!(obligation, "OBL-999");
            }
            other => panic!("dangling obligation ref accepted: {other:?}"),
        }
    }

    /// §37.2, one field at a time.
    #[test]
    fn each_missing_provenance_field_is_named() {
        type Blank = (&'static str, fn(&mut ArtifactProvenance));
        let blanks: [Blank; 10] = [
            ("producer", |p| p.producer.clear()),
            ("producing_attempt", |p| p.producing_attempt.clear()),
            ("contract_digest", |p| p.contract_digest.clear()),
            ("tool_or_runtime_identity", |p| {
                p.tool_or_runtime_identity.clear()
            }),
            ("creation_method", |p| p.creation_method.clear()),
            ("content_digest", |p| p.content_digest.clear()),
            ("media_type", |p| p.media_type.clear()),
            ("classification", |p| p.classification.clear()),
            ("retention", |p| p.retention.clear()),
            ("source_holder", |p| p.source_holder.clear()),
        ];
        for (name, blank) in blanks {
            let mut p = provenance();
            blank(&mut p);
            let mut d = deliverable();
            d.provenance = Some(p);
            match d.validate(&["OBL-001".to_owned()]) {
                Err(DeliverableError::ProvenanceIncomplete { field, .. }) => {
                    assert_eq!(field, name);
                }
                Err(DeliverableError::ContentAddressedWithoutDigest { .. })
                    if name == "content_digest" => {}
                other => panic!("provenance without {name} was accepted: {other:?}"),
            }
        }
    }

    /// An artifact produced from nothing is legitimate; requiring an input digest
    /// would push authors to invent one.
    #[test]
    fn an_artifact_with_no_inputs_is_still_valid() {
        let mut p = provenance();
        p.input_digests.clear();
        let mut d = deliverable();
        d.provenance = Some(p);
        assert_eq!(d.validate(&["OBL-001".to_owned()]), Ok(()));
    }

    #[test]
    fn content_addressing_without_a_digest_is_refused() {
        let mut d = deliverable();
        d.provenance_required = false;
        d.provenance = None;
        assert!(matches!(
            d.validate(&["OBL-001".to_owned()]),
            Err(DeliverableError::ContentAddressedWithoutDigest { .. })
        ));
    }

    // ---- §37.3 -----------------------------------------------------------

    /// THE rule: a report cannot stand in for the observations it describes.
    #[test]
    fn a_report_must_reference_its_raw_evidence() {
        let bare = DerivedReport {
            id: "REP-001".into(),
            raw_evidence_refs: vec![],
            replaces_sources: false,
        };
        assert!(matches!(
            bare.validate(),
            Err(DeliverableError::ReportWithoutEvidence { .. })
        ));

        let ok = DerivedReport {
            id: "REP-001".into(),
            raw_evidence_refs: vec!["evidence://stdout-1".into()],
            replaces_sources: false,
        };
        assert_eq!(ok.validate(), Ok(()));
    }

    #[test]
    fn a_report_that_replaces_its_sources_is_refused() {
        let r = DerivedReport {
            id: "REP-001".into(),
            raw_evidence_refs: vec!["evidence://stdout-1".into()],
            replaces_sources: true,
        };
        assert!(matches!(
            r.validate(),
            Err(DeliverableError::ReportReplacesSources { .. })
        ));
    }

    // ---- §37.4 -----------------------------------------------------------

    /// A performer's description of its work is not the work.
    #[test]
    fn a_submission_manifest_is_not_a_deliverable() {
        let mut s = SubmissionManifest {
            id: "SUB-001".into(),
            artifact_refs: vec!["artifact://a".into()],
            blockers: vec![],
            deviations: vec![],
            requested_next_action: "verify".into(),
            declared_as_deliverable: false,
        };
        assert_eq!(s.validate(), Ok(()));

        s.declared_as_deliverable = true;
        assert!(matches!(
            s.validate(),
            Err(DeliverableError::SubmissionAsDeliverable { .. })
        ));
    }

    #[test]
    fn vocabularies_round_trip() {
        for &k in DeliverableKind::ALL {
            assert_eq!(DeliverableKind::from_str(k.as_str()), Ok(k));
        }
        assert!(DeliverableKind::from_str("vibes").is_err());
    }

    #[test]
    fn a_deliverable_round_trips_through_json() {
        let d = deliverable();
        let s = serde_json::to_string(&d).expect("serialize");
        assert_eq!(
            serde_json::from_str::<Deliverable>(&s).expect("deserialize"),
            d
        );
    }
}
