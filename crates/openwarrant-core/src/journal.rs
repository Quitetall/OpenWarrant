// SPDX-License-Identifier: AGPL-3.0-or-later
//! The local draft journal (SAS §66) and portable preservation (§68).
//! RQ-082, RQ-083, RQ-084.
//!
//! # The journal must never become a second ledger
//!
//! §66.2 is the whole design constraint: *"Before KF registration, the journal is
//! repository-local draft history. After registration, Knowledge Fabric actions
//! are authoritative. The local journal becomes a cache of action requests and
//! receipts and SHALL not become a competing ledger."*
//!
//! So [`Journal`] carries a [`JournalAuthority`], and after registration an
//! append that is not a cached request or receipt is refused. A local file that
//! keeps accepting authoritative-looking events after federation is precisely the
//! competing ledger the sentence forbids, and it would be discovered only when
//! the two disagreed about something that mattered.
//!
//! §66.3 adds the smaller trap: *"The local clock is not authoritative
//! `recorded_at`."* [`JournalEvent`] has `occurred_at` and no `recorded_at`, so
//! the field a reader would trust simply is not there to be filled in wrongly.
//!
//! # Round trip is a comparison, not a hope
//!
//! §68.3: export into an empty instance, reconnect the bytes, re-export, and
//! compare — *"SHALL preserve semantic and digest identity."*
//! [`RoundTrip::verify`] is that comparison, and it fails on a digest difference
//! rather than on a size or field-count heuristic. A round trip that only checks
//! that something came back is not checking a round trip.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::vocab::vocabulary;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum JournalError {
    #[error("unknown {vocabulary} {found:?}; SAS defines {known}")]
    UnknownTerm {
        vocabulary: &'static str,
        found: String,
        known: String,
    },
    #[error(
        "event {id:?} of type {event_type:?} was appended after Knowledge Fabric \
         registration. §66.2: after registration KF actions are authoritative and \
         the local journal SHALL not become a competing ledger — only cached \
         action requests and receipts may be appended"
    )]
    CompetingLedger { id: String, event_type: String },
    #[error("event {id:?} omits {field}, which §66.3 requires")]
    EventIncomplete { id: String, field: &'static str },
    #[error(
        "event {id:?} carries a local `recorded_at`. §66.3: the local clock is not \
         authoritative recorded_at — Knowledge Fabric assigns it (§67.2)"
    )]
    LocalRecordedAt { id: String },
    #[error(
        "the journal is not append-only: event {id:?} at position {position} was \
         modified or removed. A journal that can be rewritten records what someone \
         currently believes, not what happened"
    )]
    NotAppendOnly { id: String, position: usize },
    #[error("duplicate event id {id:?} in the journal")]
    DuplicateEventId { id: String },
    #[error("export omits {field}, which §68.2 requires")]
    ExportIncomplete { field: &'static str },
    #[error(
        "round trip failed: {difference}. §68.3 requires semantic AND digest \
         identity after export, reconnection of preserved bytes, and re-export"
    )]
    RoundTripDiffers { difference: String },
    #[error(
        "round trip compared nothing: the re-export digest is empty. A round trip \
         that only checks something came back is not checking a round trip"
    )]
    RoundTripVacuous,
}

vocabulary!(
    /// §66.2's two authority regimes.
    JournalAuthority, "journal authority", JournalError, {
        /// Before registration: repository-local draft history.
        LocalDraftHistory => "local_draft_history",
        /// After registration: a CACHE of KF requests and receipts, never a ledger.
        CacheOfFederatedActions => "cache_of_federated_actions",
    }
);

vocabulary!(
    /// What a journal event records.
    ///
    /// The split matters after registration: `action_request` and
    /// `action_receipt` are cache entries and stay legal; everything else would
    /// be the journal asserting a fact of its own.
    EventClass, "event class", JournalError, {
        DraftHistory => "draft_history",
        ActionRequest => "action_request",
        ActionReceipt => "action_receipt",
    }
);

impl EventClass {
    /// §66.2 — whether this class may still be appended after registration.
    #[must_use]
    pub const fn permitted_after_registration(self) -> bool {
        matches!(self, Self::ActionRequest | Self::ActionReceipt)
    }
}

/// §66.3's event envelope.
///
/// There is deliberately no `recorded_at`: §66.3 says the local clock is not
/// authoritative for it, and a field that cannot be filled correctly should not
/// exist to be filled at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEvent {
    pub v: u32,
    pub id: String,
    pub warrant_uuid: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub class: EventClass,
    pub actor_ref: String,
    /// When the thing happened, by the local clock. NOT `recorded_at`.
    pub occurred_at: String,
    #[serde(default)]
    pub payload: String,
    #[serde(default)]
    pub idempotency_key: String,
}

impl JournalEvent {
    pub fn validate(&self) -> Result<(), JournalError> {
        for (field, value) in [
            ("id", &self.id),
            ("warrant_uuid", &self.warrant_uuid),
            ("type", &self.event_type),
            ("actor_ref", &self.actor_ref),
            ("occurred_at", &self.occurred_at),
        ] {
            if value.trim().is_empty() {
                return Err(JournalError::EventIncomplete {
                    id: self.id.clone(),
                    field,
                });
            }
        }
        Ok(())
    }

    /// Refuse a `recorded_at` arriving in a payload from outside the type.
    pub fn reject_local_recorded_at(&self) -> Result<(), JournalError> {
        if self.payload.contains("\"recorded_at\"") {
            return Err(JournalError::LocalRecordedAt {
                id: self.id.clone(),
            });
        }
        Ok(())
    }
}

/// §66's append-only local journal.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Journal {
    #[serde(default)]
    pub events: Vec<JournalEvent>,
    #[serde(default)]
    pub registered: bool,
}

impl Journal {
    /// §66.2 — which regime the journal is under.
    #[must_use]
    pub const fn authority(&self) -> JournalAuthority {
        if self.registered {
            JournalAuthority::CacheOfFederatedActions
        } else {
            JournalAuthority::LocalDraftHistory
        }
    }

    /// Append, refusing anything §66.2 forbids.
    pub fn append(&mut self, event: JournalEvent) -> Result<(), JournalError> {
        event.validate()?;
        event.reject_local_recorded_at()?;
        if self.events.iter().any(|e| e.id == event.id) {
            return Err(JournalError::DuplicateEventId { id: event.id });
        }
        // The rule this module exists for.
        if self.registered && !event.class.permitted_after_registration() {
            return Err(JournalError::CompetingLedger {
                id: event.id,
                event_type: event.event_type,
            });
        }
        self.events.push(event);
        Ok(())
    }

    /// Verify the journal has only been appended to, against a prior snapshot.
    ///
    /// A journal that can be rewritten records what someone currently believes,
    /// not what happened — so the check is that the earlier events are still
    /// there, unchanged, in the same order.
    pub fn verify_append_only(&self, prior: &[JournalEvent]) -> Result<(), JournalError> {
        for (position, old) in prior.iter().enumerate() {
            match self.events.get(position) {
                Some(current) if current == old => {}
                _ => {
                    return Err(JournalError::NotAppendOnly {
                        id: old.id.clone(),
                        position,
                    });
                }
            }
        }
        Ok(())
    }
}

/// §68.2's export contents, as a checklist.
pub const EXPORT_CONTENTS: [&str; 15] = [
    "complete identity",
    "source manifest",
    "exact atom revisions and digests",
    "Compilation Basis",
    "canonical IR",
    "contract revisions",
    "actions and relevant audit receipts",
    "ADR refs and accepted bodies where policy permits",
    "runtime receipt refs",
    "artifacts",
    "assurance case",
    "resolution and standing",
    "evidence manifest",
    "schema and compiler identity",
    "optional signatures and checkpoints",
];

/// §68.1's one-file canonical export.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PortableExport {
    /// Which of §68.2's contents this export actually carries.
    #[serde(default)]
    pub present: BTreeSet<String>,
    /// §68.1 — small records embedded, large evidence by content address.
    #[serde(default)]
    pub embedded_record_count: usize,
    #[serde(default)]
    pub referenced_evidence_digests: Vec<String>,
    pub document_digest: String,
}

impl PortableExport {
    /// §68.2 — every required content present.
    ///
    /// "optional signatures and checkpoints" is the one entry the SAS marks
    /// optional, so it is the one entry not required here.
    pub fn validate(&self) -> Result<(), JournalError> {
        for item in EXPORT_CONTENTS {
            if item.starts_with("optional ") {
                continue;
            }
            if !self.present.contains(item) {
                return Err(JournalError::ExportIncomplete { field: leak(item) });
            }
        }
        Ok(())
    }
}

/// §68.2's entries are `&'static str` already; this keeps the error type's
/// `&'static str` field honest without allocating a leak per call.
fn leak(item: &'static str) -> &'static str {
    item
}

/// §68.3's round trip.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RoundTrip {
    pub original_digest: String,
    pub reexport_digest: String,
    /// Whether the preserved evidence bytes were reconnected before re-export.
    /// §68.3 says to reconnect them; comparing without doing so compares two
    /// exports that both lack the same thing.
    #[serde(default)]
    pub evidence_reconnected: bool,
    /// Semantic differences found beyond the digest.
    #[serde(default)]
    pub semantic_differences: Vec<String>,
}

impl RoundTrip {
    /// §68.3 — semantic AND digest identity.
    pub fn verify(&self) -> Result<(), JournalError> {
        if self.reexport_digest.trim().is_empty() || self.original_digest.trim().is_empty() {
            return Err(JournalError::RoundTripVacuous);
        }
        if !self.evidence_reconnected {
            return Err(JournalError::RoundTripDiffers {
                difference: "evidence bytes were not reconnected before re-export, so \
                             the comparison is between two exports missing the same data"
                    .to_owned(),
            });
        }
        if self.original_digest != self.reexport_digest {
            return Err(JournalError::RoundTripDiffers {
                difference: format!(
                    "digest {} became {}",
                    self.original_digest, self.reexport_digest
                ),
            });
        }
        if !self.semantic_differences.is_empty() {
            return Err(JournalError::RoundTripDiffers {
                difference: self.semantic_differences.join("; "),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn event(id: &str, class: EventClass) -> JournalEvent {
        JournalEvent {
            v: 1,
            id: id.into(),
            warrant_uuid: "01a018db-19fc-7f2a-8e39-69730f255e33".into(),
            event_type: "milestone.progress_recorded".into(),
            class,
            actor_ref: "local://git-user".into(),
            occurred_at: "2026-08-20T12:00:00Z".into(),
            payload: "{}".into(),
            idempotency_key: "k1".into(),
        }
    }

    /// §66.2 — THE rule. After registration the journal is a cache, not a ledger.
    #[test]
    fn after_registration_the_journal_cannot_record_facts_of_its_own() {
        let mut j = Journal::default();
        assert_eq!(j.authority(), JournalAuthority::LocalDraftHistory);
        assert_eq!(j.append(event("e1", EventClass::DraftHistory)), Ok(()));

        j.registered = true;
        assert_eq!(j.authority(), JournalAuthority::CacheOfFederatedActions);

        // Draft history is no longer appendable...
        let err = j.append(event("e2", EventClass::DraftHistory)).unwrap_err();
        assert!(matches!(err, JournalError::CompetingLedger { .. }), "{err}");
        assert!(err.to_string().contains("competing ledger"));

        // ...but cached requests and receipts are.
        assert_eq!(j.append(event("e3", EventClass::ActionRequest)), Ok(()));
        assert_eq!(j.append(event("e4", EventClass::ActionReceipt)), Ok(()));
    }

    #[test]
    fn the_event_classes_split_on_what_survives_registration() {
        assert!(!EventClass::DraftHistory.permitted_after_registration());
        assert!(EventClass::ActionRequest.permitted_after_registration());
        assert!(EventClass::ActionReceipt.permitted_after_registration());
    }

    /// §66.3 — the local clock is not authoritative `recorded_at`, so the field
    /// does not exist to be filled in wrongly.
    #[test]
    fn the_event_envelope_has_no_recorded_at_field() {
        let json =
            serde_json::to_string(&event("e1", EventClass::DraftHistory)).expect("serialize");
        assert!(json.contains("occurred_at"));
        assert!(
            !json.contains("recorded_at"),
            "the envelope offers a field the local clock cannot fill correctly"
        );
    }

    /// ...and one smuggled in through a payload is refused.
    #[test]
    fn a_recorded_at_smuggled_through_the_payload_is_refused() {
        let mut e = event("e1", EventClass::DraftHistory);
        e.payload = r#"{"recorded_at":"2026-08-20T12:00:00Z"}"#.into();
        assert!(matches!(
            e.reject_local_recorded_at(),
            Err(JournalError::LocalRecordedAt { .. })
        ));

        let mut j = Journal::default();
        assert!(j.append(e).is_err());
    }

    /// A journal that can be rewritten records what someone currently believes.
    #[test]
    fn a_rewritten_journal_is_detected() {
        let mut j = Journal::default();
        j.append(event("e1", EventClass::DraftHistory)).expect("e1");
        j.append(event("e2", EventClass::DraftHistory)).expect("e2");
        let snapshot = j.events.clone();

        // Appending is fine.
        j.append(event("e3", EventClass::DraftHistory)).expect("e3");
        assert_eq!(j.verify_append_only(&snapshot), Ok(()));

        // Editing an earlier event is not.
        j.events[0].payload = r#"{"revised":true}"#.into();
        match j.verify_append_only(&snapshot) {
            Err(JournalError::NotAppendOnly { position, .. }) => assert_eq!(position, 0),
            other => panic!("a rewritten event was accepted: {other:?}"),
        }

        // Nor is removing one.
        let mut truncated = Journal::default();
        truncated
            .append(event("e1", EventClass::DraftHistory))
            .expect("e1");
        assert!(matches!(
            truncated.verify_append_only(&snapshot),
            Err(JournalError::NotAppendOnly { .. })
        ));
    }

    #[test]
    fn a_duplicate_event_id_is_refused() {
        let mut j = Journal::default();
        j.append(event("e1", EventClass::DraftHistory))
            .expect("first");
        assert!(matches!(
            j.append(event("e1", EventClass::DraftHistory)),
            Err(JournalError::DuplicateEventId { .. })
        ));
    }

    // ---- §68 -------------------------------------------------------------

    /// §68.2's fifteen, transcribed.
    #[test]
    fn the_export_contents_match_the_sas() {
        assert_eq!(EXPORT_CONTENTS.len(), 15);
        assert_eq!(EXPORT_CONTENTS[0], "complete identity");
        assert_eq!(EXPORT_CONTENTS[14], "optional signatures and checkpoints");
    }

    fn full_export() -> PortableExport {
        PortableExport {
            present: EXPORT_CONTENTS.iter().map(|s| (*s).to_owned()).collect(),
            embedded_record_count: 40,
            referenced_evidence_digests: vec!["sha256:e".into()],
            document_digest: "sha256:doc".into(),
        }
    }

    /// Each required content, dropped one at a time.
    #[test]
    fn every_required_export_content_is_required() {
        assert_eq!(full_export().validate(), Ok(()));
        for item in EXPORT_CONTENTS {
            let mut e = full_export();
            e.present.remove(item);
            let result = e.validate();
            if item.starts_with("optional ") {
                assert_eq!(result, Ok(()), "{item} is marked optional by the SAS");
            } else {
                match result {
                    Err(JournalError::ExportIncomplete { field }) => assert_eq!(field, item),
                    other => panic!("export without {item:?} was accepted: {other:?}"),
                }
            }
        }
    }

    /// §68.3 — semantic AND digest identity.
    #[test]
    fn a_round_trip_that_changed_the_digest_fails() {
        let rt = RoundTrip {
            original_digest: "sha256:a".into(),
            reexport_digest: "sha256:b".into(),
            evidence_reconnected: true,
            semantic_differences: vec![],
        };
        let err = rt.verify().unwrap_err();
        assert!(err.to_string().contains("sha256:a"), "{err}");
        assert!(err.to_string().contains("sha256:b"), "{err}");
    }

    #[test]
    fn a_round_trip_with_matching_digests_and_no_differences_passes() {
        let rt = RoundTrip {
            original_digest: "sha256:a".into(),
            reexport_digest: "sha256:a".into(),
            evidence_reconnected: true,
            semantic_differences: vec![],
        };
        assert_eq!(rt.verify(), Ok(()));
    }

    /// §68.3 says to RECONNECT the preserved bytes first. Comparing without
    /// doing so compares two exports that both lack the same data, which passes
    /// for the wrong reason.
    #[test]
    fn a_round_trip_without_reconnecting_evidence_is_not_a_round_trip() {
        let rt = RoundTrip {
            original_digest: "sha256:a".into(),
            reexport_digest: "sha256:a".into(),
            evidence_reconnected: false,
            semantic_differences: vec![],
        };
        let err = rt.verify().unwrap_err();
        assert!(
            err.to_string().contains("not reconnected"),
            "matching digests passed while evidence was missing from both sides: {err}"
        );
    }

    /// A comparison against nothing is not a comparison.
    #[test]
    fn a_vacuous_round_trip_is_refused() {
        for (a, b) in [("", "sha256:a"), ("sha256:a", ""), ("", "")] {
            let rt = RoundTrip {
                original_digest: a.into(),
                reexport_digest: b.into(),
                evidence_reconnected: true,
                semantic_differences: vec![],
            };
            assert_eq!(rt.verify(), Err(JournalError::RoundTripVacuous));
        }
    }

    /// Digest identity alone is not enough if a semantic difference was found.
    #[test]
    fn a_semantic_difference_fails_even_with_matching_digests() {
        let rt = RoundTrip {
            original_digest: "sha256:a".into(),
            reexport_digest: "sha256:a".into(),
            evidence_reconnected: true,
            semantic_differences: vec!["resolution standing became valid".into()],
        };
        assert!(matches!(
            rt.verify(),
            Err(JournalError::RoundTripDiffers { .. })
        ));
    }

    #[test]
    fn vocabularies_round_trip() {
        for &a in JournalAuthority::ALL {
            assert_eq!(JournalAuthority::from_str(a.as_str()), Ok(a));
        }
        for &c in EventClass::ALL {
            assert_eq!(EventClass::from_str(c.as_str()), Ok(c));
        }
    }
}
