// SPDX-License-Identifier: Apache-2.0
//! The correction act: how a delivered artifact of a RESOLVED Warrant moves for
//! a reason, leaving the original record intact (OW-WAR-0064; SAS §34.4 applied
//! to deliverables; OW-ADR-0012).
//!
//! # Why this exists
//!
//! `deliverables.toml` pins the bytes of each delivered file, and a resolution
//! binds `sha256(deliverables.toml)` into its own record. Once resolved, the pin
//! has no authorized way to change: `war check` raises `deliverable.digest-drift`
//! and its stated remedy — regenerate the record — would move the resolution's
//! `artifact_manifest_digest` and stale the resolution. Three defects hit that
//! wall in one day. The gate was right to refuse; what was missing was the act
//! that lets a delivered artifact move FOR A REASON.
//!
//! # The shape
//!
//! A correction is an append-only record beside `deliverables.toml`, never an
//! edit to it. Each correction supersedes exactly one digest — the recorded one,
//! or the previous correction's `new_digest` — so the records form a chain whose
//! head is the digest the file must have now. §34.4's rule for requirements is
//! applied verbatim: supersede, never erase; the original digest stays in
//! `deliverables.toml`, in every correction's `superseded_digest`, and in what
//! `war show` prints.
//!
//! [`chain_head`] is the ONE function that decides what the current digest ought
//! to be. `war check`, `war correct`, `war sign` and `war show` all call it, so
//! they cannot disagree about whether a file has drifted or been corrected.

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CORRECTION_SCHEMA: &str = "oh.war/correction/v1";

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
/// What kind of change a correction admits. Recorded, not enforced: a repair
/// path may later admit an added refusal more readily than a behaviour change,
/// because a new refusal cannot invalidate an already-accepted record — it can
/// only stop the next bad one — while a behaviour change alters what accepted
/// records meant. The distinction is Knowledge Fabric's, from the exchange that
/// surfaced the gap, and it is written down so the later decision has data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CorrectionKind {
    BehaviourChange,
    AddedRefusal,
}

impl std::fmt::Display for CorrectionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::BehaviourChange => "behaviour-change",
            Self::AddedRefusal => "added-refusal",
        })
    }
}

impl std::str::FromStr for CorrectionKind {
    type Err = CorrectionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "behaviour-change" | "behaviour_change" => Ok(Self::BehaviourChange),
            "added-refusal" | "added_refusal" => Ok(Self::AddedRefusal),
            other => Err(CorrectionError::UnknownKind(other.to_owned())),
        }
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
/// One correction: the deliverable, what it supersedes, what it becomes, why,
/// and who decided.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Correction {
    /// UUIDv7, minted at ingest.
    pub id: String,
    pub deliverable_id: String,
    pub target_ref: String,
    /// 1 for the first correction of a deliverable, then 2, 3, … with no gaps.
    pub sequence: u32,
    /// `sha256:<64 hex>` — the recorded digest, or the prior correction's `new_digest`.
    pub superseded_digest: String,
    /// `sha256:<64 hex>` — what the file's bytes are now.
    pub new_digest: String,
    pub reason: String,
    pub kind: CorrectionKind,
    /// `person://<actor>` — the human who signed. Never an agent.
    pub authorized_by_ref: String,
    pub acting_role_ref: String,
    pub effective_at: String,
    pub recorded_at: String,
}

/// The persisted record: `docs/warrants/<alias>/corrections/<D-id>-<n>.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrectionRecord {
    pub schema: String,
    pub warrant: String,
    pub correction: Correction,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CorrectionError {
    #[error("unknown correction kind {0:?}; known: behaviour-change, added-refusal")]
    UnknownKind(String),
    #[error("{field} must be `sha256:` followed by 64 lowercase hex characters, got {found:?}")]
    NotADigest { field: &'static str, found: String },
    #[error("a correction that supersedes a digest with itself corrects nothing")]
    SameDigest,
    #[error("a correction with no reason is a change nobody can answer for")]
    ReasonEmpty,
    #[error("sequence must be at least 1")]
    SequenceZero,
    #[error("{field}: {why}")]
    Timestamp { field: &'static str, why: String },
    #[error("{field} must not be empty")]
    Empty { field: &'static str },
}

/// Why a chain of corrections does not resolve to a head.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ChainError {
    #[error(
        "corrections are numbered 1..n without gaps; expected sequence {expected}, found {found}"
    )]
    Gap { expected: u32, found: u32 },
    #[error(
        "correction {sequence} supersedes {claimed}, but the digest at that point was {expected}: \
         it supersedes nothing that happened"
    )]
    SupersededNeverDelivered {
        sequence: u32,
        claimed: String,
        expected: String,
    },
}

fn is_sha256(s: &str) -> bool {
    s.strip_prefix("sha256:").is_some_and(|h| {
        h.len() == 64
            && h.bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    })
}

impl Correction {
    /// Structural validity of one record, independent of its neighbours.
    pub fn validate(&self) -> Result<(), CorrectionError> {
        for (field, value) in [
            ("id", &self.id),
            ("deliverable_id", &self.deliverable_id),
            ("target_ref", &self.target_ref),
            ("authorized_by_ref", &self.authorized_by_ref),
            ("acting_role_ref", &self.acting_role_ref),
        ] {
            if value.trim().is_empty() {
                return Err(CorrectionError::Empty { field });
            }
        }
        if !is_sha256(&self.superseded_digest) {
            return Err(CorrectionError::NotADigest {
                field: "superseded_digest",
                found: self.superseded_digest.clone(),
            });
        }
        if !is_sha256(&self.new_digest) {
            return Err(CorrectionError::NotADigest {
                field: "new_digest",
                found: self.new_digest.clone(),
            });
        }
        if self.superseded_digest == self.new_digest {
            return Err(CorrectionError::SameDigest);
        }
        if self.reason.trim().is_empty() {
            return Err(CorrectionError::ReasonEmpty);
        }
        if self.sequence == 0 {
            return Err(CorrectionError::SequenceZero);
        }
        for (field, value) in [
            ("effective_at", &self.effective_at),
            ("recorded_at", &self.recorded_at),
        ] {
            crate::timestamp::validate_rfc3339_utc(value).map_err(|e| {
                CorrectionError::Timestamp {
                    field,
                    why: e.to_string(),
                }
            })?;
        }
        Ok(())
    }
}

/// The digest a deliverable's bytes must have now, given its recorded digest
/// and every correction on file.
///
/// `recorded` is the `deliverables.toml` digest (`sha256:…`). Corrections are
/// taken in `sequence` order and must be contiguous from 1; each must supersede
/// exactly the digest that was current before it. An empty slice returns
/// `recorded` unchanged, so callers need no special case for "never corrected".
pub fn chain_head(recorded: &str, corrections: &[Correction]) -> Result<String, ChainError> {
    let mut sorted: Vec<&Correction> = corrections.iter().collect();
    sorted.sort_by_key(|c| c.sequence);
    let mut head = recorded.to_owned();
    for (i, c) in sorted.iter().enumerate() {
        let expected = u32::try_from(i + 1).unwrap_or(u32::MAX);
        if c.sequence != expected {
            return Err(ChainError::Gap {
                expected,
                found: c.sequence,
            });
        }
        if c.superseded_digest != head {
            return Err(ChainError::SupersededNeverDelivered {
                sequence: c.sequence,
                claimed: c.superseded_digest.clone(),
                expected: head,
            });
        }
        head = c.new_digest.clone();
    }
    Ok(head)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(c: char) -> String {
        format!("sha256:{}", std::iter::repeat_n(c, 64).collect::<String>())
    }

    fn correction(seq: u32, from: &str, to: &str) -> Correction {
        Correction {
            id: "01a06a12-0aa2-7503-b589-67cf75905be4".to_owned(),
            deliverable_id: "D-002".to_owned(),
            target_ref: "crates/x.rs".to_owned(),
            sequence: seq,
            superseded_digest: from.to_owned(),
            new_digest: to.to_owned(),
            reason: "continuation lines were dropped".to_owned(),
            kind: CorrectionKind::BehaviourChange,
            authorized_by_ref: "person://Brian Lam".to_owned(),
            acting_role_ref: "role://authorizer".to_owned(),
            effective_at: "2026-09-11T00:00:00Z".to_owned(),
            recorded_at: "2026-09-11T00:00:01Z".to_owned(),
        }
    }

    #[test]
    fn a_valid_correction_validates_and_each_field_is_refused_on_its_own() {
        let ok = correction(1, &d('a'), &d('b'));
        ok.validate().expect("valid");
        let mut c = ok.clone();
        c.new_digest = d('a');
        assert_eq!(c.validate(), Err(CorrectionError::SameDigest));
        let mut c = ok.clone();
        c.superseded_digest = "sha256:ABC".to_owned();
        assert!(matches!(
            c.validate(),
            Err(CorrectionError::NotADigest {
                field: "superseded_digest",
                ..
            })
        ));
        let mut c = ok.clone();
        c.reason = "  ".to_owned();
        assert_eq!(c.validate(), Err(CorrectionError::ReasonEmpty));
        let mut c = ok.clone();
        c.sequence = 0;
        assert_eq!(c.validate(), Err(CorrectionError::SequenceZero));
        let mut c = ok.clone();
        c.effective_at = "soon".to_owned();
        assert!(matches!(
            c.validate(),
            Err(CorrectionError::Timestamp {
                field: "effective_at",
                ..
            })
        ));
        let mut c = ok;
        c.authorized_by_ref = String::new();
        assert!(matches!(
            c.validate(),
            Err(CorrectionError::Empty {
                field: "authorized_by_ref"
            })
        ));
    }

    #[test]
    fn an_empty_chain_is_the_recorded_digest() {
        assert_eq!(chain_head(&d('a'), &[]).unwrap(), d('a'));
    }

    #[test]
    fn a_two_step_chain_resolves_to_the_last_new_digest_regardless_of_input_order() {
        let c1 = correction(1, &d('a'), &d('b'));
        let c2 = correction(2, &d('b'), &d('c'));
        assert_eq!(
            chain_head(&d('a'), &[c2.clone(), c1.clone()]).unwrap(),
            d('c')
        );
        assert_eq!(chain_head(&d('a'), &[c1, c2]).unwrap(), d('c'));
    }

    #[test]
    fn a_gap_and_a_superseded_digest_that_never_existed_are_refused_by_name() {
        let c2 = correction(2, &d('a'), &d('b'));
        assert_eq!(
            chain_head(&d('a'), &[c2]),
            Err(ChainError::Gap {
                expected: 1,
                found: 2
            })
        );
        let wrong = correction(1, &d('z'), &d('b'));
        assert_eq!(
            chain_head(&d('a'), &[wrong]),
            Err(ChainError::SupersededNeverDelivered {
                sequence: 1,
                claimed: d('z'),
                expected: d('a'),
            })
        );
    }

    #[test]
    fn kind_round_trips_through_serde_and_from_str() {
        assert_eq!(
            "behaviour-change".parse::<CorrectionKind>().unwrap(),
            CorrectionKind::BehaviourChange
        );
        assert!("fix".parse::<CorrectionKind>().is_err());
        let json = serde_json::to_string(&CorrectionKind::AddedRefusal).unwrap();
        assert_eq!(json, "\"added-refusal\"");
    }
}
