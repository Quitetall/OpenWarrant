// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR atoms and their lifecycle (SAS §19).
//!
//! §4.3: "**ADR** continues to mean **Architecture Decision Record**.
//! OpenWarrant SHALL NOT redefine ADR." Every normative decision is a
//! first-class record with its own identity, lifecycle, and source atom.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::frontmatter::{self, FrontmatterError};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AdrError {
    #[error("frontmatter: {0}")]
    Frontmatter(#[from] FrontmatterError),
    #[error("missing required frontmatter key {key:?}")]
    MissingKey { key: &'static str },
    #[error(
        "unknown ADR status {found:?}; expected one of \
         proposed, accepted, superseded, deprecated, rejected, withdrawn, falsified"
    )]
    UnknownStatus { found: String },
}

/// Where an ADR sits in its lifecycle.
///
/// `Falsified` is a first-class terminal state, not a synonym for rejected. A
/// decision whose claim was disproved by measurement is a RESULT — the record
/// keeps the measurement that disproved it — whereas rejected means never
/// accepted in the first place. Collapsing them would lose the distinction
/// between "we tried it and it did not hold" and "we declined to try it".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdrStatus {
    Proposed,
    Accepted,
    Superseded,
    Deprecated,
    Rejected,
    Withdrawn,
    Falsified,
}

impl AdrStatus {
    pub const ALL: [Self; 7] = [
        Self::Proposed,
        Self::Accepted,
        Self::Superseded,
        Self::Deprecated,
        Self::Rejected,
        Self::Withdrawn,
        Self::Falsified,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Superseded => "superseded",
            Self::Deprecated => "deprecated",
            Self::Rejected => "rejected",
            Self::Withdrawn => "withdrawn",
            Self::Falsified => "falsified",
        }
    }

    /// Whether this decision still governs new work (§24.4 currency).
    #[must_use]
    pub const fn is_current(self) -> bool {
        match self {
            Self::Accepted => true,
            // Proposed is NOT current: a proposed ADR cannot satisfy an
            // accepted-decision prerequisite (§91.4 test 28).
            Self::Proposed
            | Self::Superseded
            | Self::Deprecated
            | Self::Rejected
            | Self::Withdrawn
            | Self::Falsified => false,
        }
    }

    /// The Appendix A section this status is listed under.
    #[must_use]
    pub const fn overview_section(self) -> &'static str {
        match self {
            Self::Proposed => "Proposed",
            Self::Accepted => "Accepted and Current",
            Self::Superseded | Self::Deprecated => "Superseded",
            Self::Rejected | Self::Withdrawn | Self::Falsified => {
                "Rejected, Withdrawn, or Falsified"
            }
        }
    }
}

impl FromStr for AdrStatus {
    type Err = AdrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == s)
            .ok_or_else(|| AdrError::UnknownStatus {
                found: s.to_owned(),
            })
    }
}

impl fmt::Display for AdrStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One ADR, parsed from its source atom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdrRecord {
    pub uuid: String,
    pub local_alias: String,
    pub title: String,
    pub status: AdrStatus,
    pub decided: Option<String>,
    /// Warrants this decision governs (§19.4).
    pub governs: Vec<String>,
    /// The atom body, frontmatter stripped.
    pub body: String,
    /// Repository-relative source path.
    pub source: String,
}

impl AdrRecord {
    /// Parse an ADR atom.
    ///
    /// The title is taken from the body's first `#` heading rather than from a
    /// frontmatter field, so there is exactly one place a title can live and it
    /// is the place a human reads. A separate `title:` key would be a second
    /// source that could disagree with the heading.
    pub fn parse(source: &str, text: &str) -> Result<Self, AdrError> {
        let fm = frontmatter::parse(text)?;
        let get = |key: &'static str| -> Result<String, AdrError> {
            fm.scalar(key)
                .map(str::to_owned)
                .ok_or(AdrError::MissingKey { key })
        };

        let body = text[fm.body_offset..].trim_start_matches('\n').to_owned();
        let title = body
            .lines()
            .find(|line| line.starts_with("# "))
            .map(|line| line.trim_start_matches("# ").trim().to_owned())
            .unwrap_or_else(|| "(untitled)".to_owned());

        Ok(Self {
            uuid: get("adr_uuid")?,
            local_alias: get("local_alias")?,
            title,
            status: AdrStatus::from_str(&get("status")?)?,
            decided: fm.scalar("decided").map(str::to_owned),
            governs: fm.list("governs").unwrap_or_default().to_vec(),
            body,
            source: source.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "---\n\
schema: oh.war/atom/v1\n\
adr_uuid: 01a01927-8855-7046-9876-ef13ae754180\n\
local_alias: OW-ADR-0001\n\
role: adr\n\
jurisdiction: bound\n\
status: accepted\n\
decided: 2026-08-19\n\
governs:\n  - \"war://01a0-abc\"\n\
---\n\n# ADR OW-0001: Adopt something\n\n## Status\n\nAccepted.\n";

    #[test]
    fn parses_an_adr_atom() {
        let adr = AdrRecord::parse("docs/adr/atoms/x.md", SAMPLE).expect("parses");
        assert_eq!(adr.local_alias, "OW-ADR-0001");
        assert_eq!(adr.status, AdrStatus::Accepted);
        assert_eq!(adr.decided.as_deref(), Some("2026-08-19"));
        assert_eq!(adr.governs, vec!["war://01a0-abc"]);
        assert_eq!(adr.title, "ADR OW-0001: Adopt something");
        assert!(adr.body.starts_with("# ADR OW-0001"));
        assert!(
            !adr.body.contains("schema:"),
            "frontmatter must be stripped"
        );
    }

    #[test]
    fn missing_required_keys_fail_closed() {
        let without_status = SAMPLE.replace("status: accepted\n", "");
        assert_eq!(
            AdrRecord::parse("x", &without_status),
            Err(AdrError::MissingKey { key: "status" })
        );
        let without_uuid = SAMPLE.replace("adr_uuid: 01a01927-8855-7046-9876-ef13ae754180\n", "");
        assert_eq!(
            AdrRecord::parse("x", &without_uuid),
            Err(AdrError::MissingKey { key: "adr_uuid" })
        );
    }

    #[test]
    fn unknown_status_is_refused() {
        let bad = SAMPLE.replace("status: accepted", "status: probably-fine");
        assert_eq!(
            AdrRecord::parse("x", &bad),
            Err(AdrError::UnknownStatus {
                found: "probably-fine".to_owned()
            })
        );
    }

    /// §91.4 test 28: a proposed ADR cannot satisfy an accepted-decision
    /// prerequisite, so it is not current.
    #[test]
    fn only_accepted_is_current() {
        assert!(AdrStatus::Accepted.is_current());
        for status in AdrStatus::ALL.iter().filter(|s| **s != AdrStatus::Accepted) {
            assert!(!status.is_current(), "{status} must not be current");
        }
    }

    /// Falsified is its own terminal state, distinct from rejected.
    #[test]
    fn falsified_and_rejected_are_distinct() {
        assert_ne!(AdrStatus::Falsified, AdrStatus::Rejected);
        assert_eq!(
            AdrStatus::Falsified.overview_section(),
            AdrStatus::Rejected.overview_section(),
            "they share a section but not an identity"
        );
    }

    #[test]
    fn every_status_maps_to_an_appendix_a_section() {
        let sections = [
            "Proposed",
            "Accepted and Current",
            "Superseded",
            "Rejected, Withdrawn, or Falsified",
        ];
        for status in AdrStatus::ALL {
            assert!(
                sections.contains(&status.overview_section()),
                "{status} maps to an unknown section"
            );
        }
    }
}
