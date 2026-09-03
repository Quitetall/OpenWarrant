// SPDX-License-Identifier: AGPL-3.0-or-later
//! Governance of the SAS itself (SAS §101; §34.1, §34.3, §34.4).
//!
//! # The SAS is a document, and until now nothing held it
//!
//! §101 says the SAS becomes a controlled document whose accepted revisions
//! are immutable (101.2), whose architecture-changing revisions require an ADR
//! (101.3), and whose mirrors state the exact accepted revision and digest
//! (101.6). The digest `aad5256c…` appeared in six places in this repository —
//! all prose. Nothing computed it, nothing compared it, and §14's "SAS
//! revision" was a Basis input no compilation carried.
//!
//! This module is the record type. A [`SasRevision`] pins one version of the
//! SAS to the sha256 of its bytes, carries a snapshot of §106 so two revisions
//! can be compared from records alone, and moves from `proposed` to `accepted`
//! exactly once, by a human, through [`SasRevision::accept`].
//!
//! # Requirement identifiers are append-only
//!
//! §34.1 calls them *stable*; §34.3 forbids editing the SAS to tick boxes;
//! §34.4 step 4 preserves a requirement that turned out to be wrong. Together
//! those mean a §106 row may be added and may be retitled, and may never be
//! removed or renumbered. [`Section106Diff::check_stability`] refuses a
//! revision that drops an id. Superseding a requirement is a Warrant's act
//! (§34.2 `supersession`), never an edit to the index.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::contract::ActorKind;
use crate::traceability::RequirementRef;

pub const SAS_REVISION_SCHEMA: &str = "oh.war/sas-revision/v1";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SasError {
    #[error("an accepted SAS revision is immutable (§101.2); {version} is already accepted")]
    Immutable { version: String },
    #[error("only a proposed revision may be accepted; {version} is {state}")]
    NotProposed {
        version: String,
        state: SasRevisionState,
    },
    #[error(
        "{actor:?} is an agent. §101 makes the SAS a controlled document and §27.2 forbids \
         an agent authorizing what it proposed; a SAS revision is accepted by a human"
    )]
    AgentAcceptance { actor: String },
    #[error("an acceptance must record the acting role")]
    MissingActingRole,
    #[error("an acceptance must record what accepting meant")]
    MissingMeaning,
    #[error(
        "revision {version} changes §106 — {summary} — which §101.3 calls an \
         architecture-changing revision, and no ADR is referenced"
    )]
    ArchitectureChangeWithoutAdr { version: String, summary: String },
    #[error(
        "revision removes requirement {id}. §34.1 identifiers are stable and §34.4 step 4 \
         preserves a requirement that turned out to be wrong: a row is added or retitled, \
         never removed or renumbered. Supersede it through a Warrant (§34.2), not by editing \
         the index"
    )]
    IdRemoved { id: String },
    #[error("a SAS revision must name a version")]
    MissingVersion,
    #[error("a SAS revision must carry a sha256 digest of the document bytes")]
    MissingDigest,
    #[error("§106 could not be read from the document: no `| <PREFIX>-SAS-RQ-NNN | … |` rows")]
    NoRequirementIndex,
}

/// Where a revision sits (§101.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SasRevisionState {
    Proposed,
    Accepted,
}

impl std::fmt::Display for SasRevisionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
        })
    }
}

/// A human accepting a revision (§101.2, §101.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SasAcceptance {
    pub accepted_by: String,
    pub actor_kind: ActorKind,
    pub acting_role: String,
    pub meaning: String,
    pub effective_time: String,
    /// Required when the revision is architecture-changing (§101.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adr_ref: Option<String>,
}

/// One revision of the SAS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SasRevision {
    pub schema: String,
    pub version: String,
    /// Repository-relative path of the document.
    pub source: String,
    /// sha256 of the exact bytes, lowercase hex, no prefix.
    pub sha256: String,
    pub state: SasRevisionState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predecessor: Option<String>,
    /// §106 as it stood in this revision: id → requirement text. Carried so
    /// two revisions can be compared from records alone.
    #[serde(default)]
    pub requirements: BTreeMap<String, String>,
    /// §101.3 — decided at proposal from the §106 diff against the predecessor.
    #[serde(default)]
    pub architecture_changing: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<SasAcceptance>,
}

impl SasRevision {
    /// A proposed revision. `requirements` is §106 as read from the bytes.
    #[must_use]
    pub fn proposed(
        version: impl Into<String>,
        source: impl Into<String>,
        sha256: impl Into<String>,
        predecessor: Option<String>,
        requirements: BTreeMap<String, String>,
        architecture_changing: bool,
    ) -> Self {
        Self {
            schema: SAS_REVISION_SCHEMA.to_owned(),
            version: version.into(),
            source: source.into(),
            sha256: sha256.into(),
            state: SasRevisionState::Proposed,
            predecessor,
            requirements,
            architecture_changing,
            acceptance: None,
        }
    }

    pub fn validate(&self) -> Result<(), SasError> {
        if self.version.trim().is_empty() {
            return Err(SasError::MissingVersion);
        }
        if self.sha256.len() != 64 || !self.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(SasError::MissingDigest);
        }
        if self.state == SasRevisionState::Accepted {
            let Some(a) = &self.acceptance else {
                return Err(SasError::NotProposed {
                    version: self.version.clone(),
                    state: self.state,
                });
            };
            Self::check_acceptance(
                &self.version,
                self.architecture_changing,
                &self.requirements,
                a,
            )?;
        }
        Ok(())
    }

    fn check_acceptance(
        version: &str,
        architecture_changing: bool,
        requirements: &BTreeMap<String, String>,
        a: &SasAcceptance,
    ) -> Result<(), SasError> {
        if a.actor_kind == ActorKind::Agent {
            return Err(SasError::AgentAcceptance {
                actor: a.accepted_by.clone(),
            });
        }
        if a.acting_role.trim().is_empty() {
            return Err(SasError::MissingActingRole);
        }
        if a.meaning.trim().is_empty() {
            return Err(SasError::MissingMeaning);
        }
        if architecture_changing && a.adr_ref.as_deref().unwrap_or("").trim().is_empty() {
            return Err(SasError::ArchitectureChangeWithoutAdr {
                version: version.to_owned(),
                summary: format!("{} requirement(s) in §106", requirements.len()),
            });
        }
        Ok(())
    }

    /// §101.2 — accepting consumes the proposal and returns an immutable
    /// accepted revision. There is no method that mutates an accepted one.
    pub fn accept(self, acceptance: SasAcceptance) -> Result<Self, SasError> {
        match self.state {
            SasRevisionState::Accepted => {
                return Err(SasError::Immutable {
                    version: self.version,
                });
            }
            SasRevisionState::Proposed => {}
        }
        Self::check_acceptance(
            &self.version,
            self.architecture_changing,
            &self.requirements,
            &acceptance,
        )?;
        Ok(Self {
            state: SasRevisionState::Accepted,
            acceptance: Some(acceptance),
            ..self
        })
    }

    #[must_use]
    pub const fn is_accepted(&self) -> bool {
        matches!(self.state, SasRevisionState::Accepted)
    }
}

/// §98's phases as the document states them: (number, title, Exit sentence).
///
/// Read at run time rather than copied into a constant, so that a SAS revision
/// adding an Exit (0.1.0-draft.2 gave Phases 9 and 10 theirs) changes what the
/// projection reports without a code change. A phase with no `Exit:` block, or
/// whose block has no bullet, yields `None` — the projection then says "no
/// Exit" rather than inventing one.
#[must_use]
pub fn section_98(text: &str) -> Vec<(u8, String, Option<String>)> {
    let mut out: Vec<(u8, String, Option<String>)> = Vec::new();
    // `in_phase` is true only between a `### Phase N` heading and the next
    // heading of any level, so an `Exit:` in some other section can never
    // attach its bullet to a phase (found by review).
    let mut in_phase = false;
    let mut in_exit = false;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("### Phase ") {
            let (num, title) = rest.split_once(" — ").unwrap_or((rest, ""));
            in_exit = false;
            in_phase = false;
            if let Ok(n) = num.trim().parse::<u8>() {
                out.push((n, title.trim().to_owned(), None));
                in_phase = true;
            }
            continue;
        }
        if line.starts_with('#') {
            in_exit = false;
            in_phase = false;
            continue;
        }
        if in_phase && line.trim() == "Exit:" {
            in_exit = true;
            continue;
        }
        if in_exit
            && let Some(bullet) = line.trim_start().strip_prefix("- ")
            && let Some(last) = out.last_mut()
            && last.2.is_none()
        {
            last.2 = Some(bullet.trim().to_owned());
            in_exit = false;
        }
    }
    out
}

/// §106 as a map, parsed from the document's own table rows.
#[must_use]
pub fn section_106(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("| ") else {
            continue;
        };
        let mut cells = rest.split(" | ");
        let (Some(id), Some(title)) = (cells.next(), cells.next()) else {
            continue;
        };
        if let Ok(r) = RequirementRef::parse(id) {
            out.insert(
                r.canonical(),
                title.trim_end_matches(" |").trim().to_owned(),
            );
        }
    }
    out
}

/// What changed in §106 between two revisions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section106Diff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    /// `(id, before, after)`.
    pub retitled: Vec<(String, String, String)>,
}

impl Section106Diff {
    #[must_use]
    pub fn between(prev: &BTreeMap<String, String>, next: &BTreeMap<String, String>) -> Self {
        let mut d = Self::default();
        for (id, title) in next {
            match prev.get(id) {
                None => d.added.push(id.clone()),
                Some(old) if old != title => {
                    d.retitled.push((id.clone(), old.clone(), title.clone()));
                }
                Some(_) => {}
            }
        }
        for id in prev.keys() {
            if !next.contains_key(id) {
                d.removed.push(id.clone());
            }
        }
        d
    }

    /// §101.3 — any change to §106 changes required semantics.
    #[must_use]
    pub fn is_architecture_changing(&self) -> bool {
        !(self.added.is_empty() && self.removed.is_empty() && self.retitled.is_empty())
    }

    /// §34.1 / §34.4 — no id may disappear.
    pub fn check_stability(&self) -> Result<(), SasError> {
        if let Some(id) = self.removed.first() {
            return Err(SasError::IdRemoved { id: id.clone() });
        }
        Ok(())
    }

    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "{} added, {} removed, {} retitled",
            self.added.len(),
            self.removed.len(),
            self.retitled.len()
        )
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn section_98_reads_titles_and_exit_bullets_and_says_none_when_absent() {
        let text = "### Phase 0 — Telemetry shim\n\nDeliver:\n\n- x;\n\nExit:\n\n- real distributions.\n\n### Phase 1 — Compiler\n\nDeliver:\n\n- y.\n\n## 99. Next\n";
        let p = super::section_98(text);
        assert_eq!(p.len(), 2);
        assert_eq!(
            p[0],
            (
                0,
                "Telemetry shim".to_owned(),
                Some("real distributions.".to_owned())
            )
        );
        assert_eq!(p[1], (1, "Compiler".to_owned(), None));
    }

    #[test]
    fn section_98_ignores_an_exit_outside_a_phase_section() {
        let text = "### Phase 0 — Alpha\n\nDeliver:\n- x.\n\n### Phase 1 — Beta\n\nDeliver:\n- y.\n\n## 99. Prose\n\nExit:\n\n- stray bullet.\n";
        let p = super::section_98(text);
        assert_eq!(p.len(), 2);
        assert_eq!(
            p[1].2, None,
            "a stray Exit in §99 must not become Phase 1's"
        );
        let text2 =
            "### Phase 0 — Alpha\n\nExit:\n\n- real.\n\n#### Note\n\nExit:\n\n- not this.\n";
        assert_eq!(super::section_98(text2)[0].2.as_deref(), Some("real."));
    }

    /// §6.10 — the one rule about SAS and Warrant. A test rather than prose,
    /// so the rule cannot be edited out of the controlled document without a
    /// red gate: it was misread once in practice (a Warrant written in the
    /// SAS's style in place of a program's SAS), and the sentence is what
    /// prevents the next time.
    #[test]
    fn section_6_10_states_that_a_sas_and_a_warrant_are_the_same_class_of_artifact() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/sas/WAR_Software_Architecture_Specification.md");
        let text = std::fs::read_to_string(path).expect("SAS");
        assert!(text.contains("### 6.10 The levels, and the one rule about SAS and Warrant"));
        assert!(
            text.contains("same class of artifact"),
            "the rule's sentence is gone"
        );
        assert!(
            text.contains("Starting a program?") && text.contains("Doing work inside a program?")
        );
    }

    #[test]
    fn section_98_of_the_real_document_has_eleven_phases_and_exits_for_all_of_them() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/sas/WAR_Software_Architecture_Specification.md");
        let text = std::fs::read_to_string(path).expect("SAS");
        let p = super::section_98(&text);
        assert_eq!(p.len(), 11);
        let missing: Vec<u8> = p
            .iter()
            .filter(|(_, _, e)| e.is_none())
            .map(|(n, _, _)| *n)
            .collect();
        assert!(missing.is_empty(), "phases without an Exit: {missing:?}");
    }

    use super::*;

    fn rows(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    fn acceptance(kind: ActorKind, adr: Option<&str>) -> SasAcceptance {
        SasAcceptance {
            accepted_by: "brian".to_owned(),
            actor_kind: kind,
            acting_role: "owner".to_owned(),
            meaning: "This revision is normative.".to_owned(),
            effective_time: "2026-09-02T00:00:00Z".to_owned(),
            adr_ref: adr.map(str::to_owned),
        }
    }

    fn proposed(arch: bool) -> SasRevision {
        SasRevision::proposed(
            "0.1.0-draft.1",
            "docs/sas/SAS.md",
            "a".repeat(64),
            None,
            rows(&[("WAR-SAS-RQ-001", "x")]),
            arch,
        )
    }

    #[test]
    fn section_106_reads_only_requirement_rows() {
        let text = "| ID | Requirement |\n|---|---|\n| WAR-SAS-RQ-001 | Every WAR has identity |\n| not-an-id | nope |\n";
        let m = section_106(text);
        assert_eq!(m.len(), 1);
        assert_eq!(m["WAR-SAS-RQ-001"], "Every WAR has identity");
    }

    #[test]
    fn an_accepted_revision_cannot_be_accepted_again() {
        let r = proposed(false)
            .accept(acceptance(ActorKind::Human, None))
            .expect("accepts");
        assert!(r.is_accepted());
        assert!(matches!(
            r.accept(acceptance(ActorKind::Human, None)),
            Err(SasError::Immutable { .. })
        ));
    }

    #[test]
    fn an_agent_may_not_accept() {
        assert!(matches!(
            proposed(false).accept(acceptance(ActorKind::Agent, None)),
            Err(SasError::AgentAcceptance { .. })
        ));
    }

    #[test]
    fn an_architecture_changing_revision_needs_an_adr() {
        assert!(matches!(
            proposed(true).accept(acceptance(ActorKind::Human, None)),
            Err(SasError::ArchitectureChangeWithoutAdr { .. })
        ));
        assert!(
            proposed(true)
                .accept(acceptance(
                    ActorKind::Human,
                    Some("docs/adr/atoms/OW-ADR-0012.md")
                ))
                .is_ok()
        );
    }

    #[test]
    fn a_removed_id_is_refused_and_an_added_or_retitled_one_is_not() {
        let prev = rows(&[("WAR-SAS-RQ-001", "a"), ("WAR-SAS-RQ-002", "b")]);
        let next = rows(&[("WAR-SAS-RQ-001", "a2"), ("WAR-SAS-RQ-003", "c")]);
        let d = Section106Diff::between(&prev, &next);
        assert_eq!(d.added, vec!["WAR-SAS-RQ-003"]);
        assert_eq!(d.removed, vec!["WAR-SAS-RQ-002"]);
        assert_eq!(d.retitled.len(), 1);
        assert!(d.is_architecture_changing());
        assert!(
            matches!(d.check_stability(), Err(SasError::IdRemoved { id }) if id == "WAR-SAS-RQ-002")
        );

        let same = Section106Diff::between(&prev, &prev);
        assert!(!same.is_architecture_changing());
        assert!(same.check_stability().is_ok());
    }

    #[test]
    fn validate_refuses_a_bad_digest_and_an_accepted_without_acceptance() {
        let mut r = proposed(false);
        r.sha256 = "short".to_owned();
        assert!(matches!(r.validate(), Err(SasError::MissingDigest)));
        let mut r = proposed(false);
        r.state = SasRevisionState::Accepted;
        assert!(matches!(r.validate(), Err(SasError::NotProposed { .. })));
    }
}
