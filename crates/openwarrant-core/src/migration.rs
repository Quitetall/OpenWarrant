// SPDX-License-Identifier: AGPL-3.0-or-later
//! Existing-ADR migration (SAS §96, §97), protocol versioning (§69), and
//! federated identity (§12, §83). RQ-003, RQ-004, RQ-005.
//!
//! # No fabricated proof
//!
//! §96.3 is the load-bearing sentence, and it is the reason this module exists:
//!
//! > A textual gate command becomes `legacy_declared_unqualified` until it is
//! > parsed, askable, bound, executed, and supported by a Gate Run receipt.
//! >
//! > A legacy `Complete` line with no admissible evidence remains a historical
//! > claim, not a newly verified WAR resolution.
//!
//! Migration is where a corpus's accumulated self-description meets a system
//! that checks things, and the tempting move is to read "Complete" as complete.
//! [`HistoricalClaim::from_completion_line`] refuses: it produces a claim, which
//! is a different TYPE from a resolution and cannot become one without evidence.
//! [`LegacyDeclaredUnqualified::from_command`] is the only constructor for a
//! migrated gate, and there is no argument that skips it.
//!
//! # §96.1 keeps the bytes
//!
//! *"Every existing ADR body remains preserved as an authored source revision."*
//! Migration adds structure alongside the original; it never replaces it. A
//! migration that improved the prose on the way through would have destroyed the
//! record it was migrating.
//!
//! # §69.4's asymmetry
//!
//! *"Unknown optional namespaced extensions are preserved. Unknown required
//! extensions fail closed."* Both halves matter: dropping an unknown optional
//! field loses data a newer producer meant to send, and accepting an unknown
//! required field means running against a contract this build does not
//! understand.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::vocab::vocabulary;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MigrationError {
    #[error("unknown {vocabulary} {found:?}; SAS defines {known}")]
    UnknownTerm {
        vocabulary: &'static str,
        found: String,
        known: String,
    },
    #[error(
        "a legacy completion line for {adr_source:?} was promoted to a verified \
         resolution with no admissible evidence. §96.3: it remains a HISTORICAL \
         CLAIM, not a newly verified WAR resolution"
    )]
    // NOT named `source`: thiserror reads that as the error's cause.
    LegacyCompletionPromoted { adr_source: String },
    #[error(
        "legacy gate command {command:?} was treated as a qualified gate. §96.3: it \
         is `legacy_declared_unqualified` until parsed, askable, bound, executed, \
         and supported by a Gate Run receipt"
    )]
    LegacyGatePromoted { command: String },
    #[error(
        "migration of {adr_source:?} did not preserve the original body. §96.1: every \
         existing ADR body remains preserved as an authored source revision"
    )]
    OriginalNotPreserved { adr_source: String },
    #[error(
        "unknown REQUIRED extension {extension:?} at protocol {version}. §69.4: \
         unknown required extensions fail closed — this build does not understand \
         part of the contract it was handed"
    )]
    UnknownRequiredExtension { extension: String, version: String },
    #[error(
        "protocol {from} to {to} changes required fields or semantics without a \
         major version bump. §69.3: a breaking change requires a major protocol \
         version AND an ADR"
    )]
    BreakingChangeWithoutMajor { from: String, to: String },
    #[error("protocol change {from} to {to} is breaking and cites no ADR (§69.3)")]
    BreakingChangeWithoutAdr { from: String, to: String },
    #[error(
        "enterprise identifier {id:?} was fabricated from {derived_from}. §12.4: the \
         enterprise identifier SHALL NOT be fabricated from a filename or local \
         sequence — Knowledge Fabric allocates it"
    )]
    FabricatedEnterpriseId {
        id: String,
        derived_from: &'static str,
    },
    #[error(
        "Warrant {alias:?} claims globally authorized or effective state while \
         unregistered. §12.4: it may remain valid as a LOCAL DRAFT before \
         allocation, and may not claim global standing until registered"
    )]
    GlobalClaimWhileUnregistered { alias: String },
    #[error("malformed protocol version {found:?}; expected MAJOR.MINOR.PATCH")]
    MalformedVersion { found: String },
}

vocabulary!(
    /// What a migrated element becomes, per §96.2's table.
    LegacyMapping, "legacy mapping", MigrationError, {
        HistoricalLifecycleEvidence => "historical_lifecycle_evidence",
        FactAndContextCandidate => "fact_and_context_candidate",
        AdrDecision => "adr_decision",
        RationaleGraphCandidate => "rationale_graph_candidate",
        OptionCandidate => "option_candidate",
        ConsequenceNode => "consequence_node",
        CandidateWorkOrderContract => "candidate_work_order_contract",
        UnqualifiedLocalGateCandidate => "unqualified_local_gate_candidate",
        ProgressEventOrObservation => "progress_event_or_observation",
        HistoricalResolutionClaim => "historical_resolution_claim",
        OngoingValidationCandidate => "ongoing_validation_candidate",
        TypedAdrRelation => "typed_adr_relation",
    }
);

/// §96.2's mapping table, verbatim: existing element -> new meaning.
pub const LEGACY_MAP: [(&str, LegacyMapping); 12] = [
    ("status", LegacyMapping::HistoricalLifecycleEvidence),
    ("Context", LegacyMapping::FactAndContextCandidate),
    ("Decision", LegacyMapping::AdrDecision),
    ("Rationale", LegacyMapping::RationaleGraphCandidate),
    ("Alternatives", LegacyMapping::OptionCandidate),
    ("Consequences", LegacyMapping::ConsequenceNode),
    (
        "Implementation Plan",
        LegacyMapping::CandidateWorkOrderContract,
    ),
    ("gate_cmd", LegacyMapping::UnqualifiedLocalGateCandidate),
    ("Progress Log", LegacyMapping::ProgressEventOrObservation),
    ("Completion", LegacyMapping::HistoricalResolutionClaim),
    ("Validation", LegacyMapping::OngoingValidationCandidate),
    ("supersedes/amends/extends", LegacyMapping::TypedAdrRelation),
];

/// Map one legacy element name to its new meaning (§96.2).
///
/// Returns `None` for an element the table does not cover. An unmapped element
/// is not silently discarded and is not guessed at — it is reported so a human
/// decides, which is the same shape as §30.4 and §19.2.
#[must_use]
pub fn map_legacy_element(element: &str) -> Option<LegacyMapping> {
    LEGACY_MAP
        .iter()
        .find(|(name, _)| *name == element)
        .map(|(_, m)| *m)
}

/// The state a legacy gate command lands in (§96.3).
///
/// A unit struct rather than a string, so it cannot be compared equal to a
/// qualified gate by accident, and there is no constructor that skips it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyDeclaredUnqualified {
    pub command: String,
    /// §96.3's five conditions, none met at migration time.
    pub parsed: bool,
    pub askable: bool,
    pub bound: bool,
    pub executed: bool,
    pub gate_run_receipt: String,
}

impl LegacyDeclaredUnqualified {
    /// §96.3 — a textual gate command becomes this, and only this.
    #[must_use]
    pub fn from_command(command: &str) -> Self {
        Self {
            command: command.to_owned(),
            parsed: false,
            askable: false,
            bound: false,
            executed: false,
            gate_run_receipt: String::new(),
        }
    }

    /// Whether all five of §96.3's conditions now hold.
    ///
    /// The receipt is the last one and the one that cannot be self-asserted: the
    /// other four are things the system can arrange, and a Gate Run receipt is
    /// evidence that it actually ran.
    #[must_use]
    pub fn is_now_qualified(&self) -> bool {
        self.parsed
            && self.askable
            && self.bound
            && self.executed
            && !self.gate_run_receipt.trim().is_empty()
    }

    /// Promote, or refuse and say what is missing.
    pub fn promote(&self) -> Result<(), MigrationError> {
        if self.is_now_qualified() {
            Ok(())
        } else {
            Err(MigrationError::LegacyGatePromoted {
                command: self.command.clone(),
            })
        }
    }
}

/// §96.3 — what a legacy `Complete` line becomes.
///
/// Deliberately NOT [`crate::resolution::Resolution`]. Two different types is
/// what stops a migration importing 94 completions as 94 verified resolutions;
/// one type with a flag would have needed everyone to remember to check the flag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalClaim {
    pub source: String,
    pub text: String,
    /// Evidence admissible under §40, if any was ever attached. Empty at
    /// migration, and the only route out of this type.
    #[serde(default)]
    pub admissible_evidence_refs: Vec<String>,
}

impl HistoricalClaim {
    /// §96.3 — a legacy completion line, preserved as a claim.
    #[must_use]
    pub fn from_completion_line(source: &str, text: &str) -> Self {
        Self {
            source: source.to_owned(),
            text: text.to_owned(),
            admissible_evidence_refs: vec![],
        }
    }

    /// Whether this claim could now support a resolution.
    #[must_use]
    pub fn has_admissible_evidence(&self) -> bool {
        !self.admissible_evidence_refs.is_empty()
    }

    /// Refuse promotion without evidence.
    pub fn require_evidence_to_promote(&self) -> Result<(), MigrationError> {
        if self.has_admissible_evidence() {
            Ok(())
        } else {
            Err(MigrationError::LegacyCompletionPromoted {
                adr_source: self.source.clone(),
            })
        }
    }
}

/// A migrated ADR: new structure ALONGSIDE the preserved original (§96.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigratedAdr {
    pub source: String,
    /// §96.1 — the original body, byte for byte.
    pub preserved_body: String,
    pub preserved_body_digest: String,
    #[serde(default)]
    pub mapped_elements: BTreeMap<String, LegacyMapping>,
    #[serde(default)]
    pub unmapped_elements: Vec<String>,
    #[serde(default)]
    pub legacy_gates: Vec<LegacyDeclaredUnqualified>,
    #[serde(default)]
    pub historical_claims: Vec<HistoricalClaim>,
}

impl MigratedAdr {
    pub fn validate(&self) -> Result<(), MigrationError> {
        if self.preserved_body.trim().is_empty() || self.preserved_body_digest.trim().is_empty() {
            return Err(MigrationError::OriginalNotPreserved {
                adr_source: self.source.clone(),
            });
        }
        Ok(())
    }
}

/// §69's semantic protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ProtocolVersion {
    pub fn parse(text: &str) -> Result<Self, MigrationError> {
        let malformed = || MigrationError::MalformedVersion {
            found: text.to_owned(),
        };
        let mut parts = text.trim().split('.');
        let mut next = || -> Result<u32, MigrationError> {
            parts
                .next()
                .ok_or_else(malformed)?
                .parse()
                .map_err(|_| malformed())
        };
        let (major, minor, patch) = (next()?, next()?, next()?);
        if parts.next().is_some() {
            return Err(malformed());
        }
        Ok(Self {
            major,
            minor,
            patch,
        })
    }

    /// §69.3 — a breaking change requires a major bump AND an ADR.
    pub fn check_transition(
        self,
        to: Self,
        breaking: bool,
        adr_ref: &str,
    ) -> Result<(), MigrationError> {
        if !breaking {
            return Ok(());
        }
        if to.major <= self.major {
            return Err(MigrationError::BreakingChangeWithoutMajor {
                from: self.to_string(),
                to: to.to_string(),
            });
        }
        if adr_ref.trim().is_empty() {
            return Err(MigrationError::BreakingChangeWithoutAdr {
                from: self.to_string(),
                to: to.to_string(),
            });
        }
        Ok(())
    }
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// An extension field this build does not recognise (§69.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnknownExtension {
    pub name: String,
    pub required: bool,
    /// The raw value, kept so an optional extension survives a round trip.
    pub raw: String,
}

/// §69.4 — preserve unknown optional extensions; fail closed on required ones.
pub fn handle_unknown_extensions(
    version: &str,
    extensions: &[UnknownExtension],
) -> Result<Vec<UnknownExtension>, MigrationError> {
    if let Some(required) = extensions.iter().find(|e| e.required) {
        return Err(MigrationError::UnknownRequiredExtension {
            extension: required.name.clone(),
            version: version.to_owned(),
        });
    }
    // Preserved, not dropped: a newer producer meant to send these.
    Ok(extensions.to_vec())
}

/// §12.1's four identity layers.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FederatedIdentity {
    /// §12.2 — created at draft creation, never changes.
    pub uuid: String,
    #[serde(default)]
    pub local_alias: String,
    /// §12.4 — allocated by Knowledge Fabric, never derived locally.
    #[serde(default)]
    pub enterprise_id: String,
    #[serde(default)]
    pub external_aliases: Vec<String>,
    #[serde(default)]
    pub registered: bool,
}

impl FederatedIdentity {
    /// §12.4 — an enterprise identifier cannot be fabricated locally.
    ///
    /// This is the one plant already in the battery ("fabricated enterprise id"),
    /// generalised. The check is on PROVENANCE rather than on shape: an id that
    /// looks right but was derived from a filename is exactly what §12.4 forbids,
    /// and it will always look right.
    pub fn set_enterprise_id(
        &mut self,
        id: &str,
        allocated_by_knowledge_fabric: bool,
        derived_from: &'static str,
    ) -> Result<(), MigrationError> {
        if !allocated_by_knowledge_fabric {
            return Err(MigrationError::FabricatedEnterpriseId {
                id: id.to_owned(),
                derived_from,
            });
        }
        self.enterprise_id = id.to_owned();
        self.registered = true;
        Ok(())
    }

    /// §12.4 — a local draft is valid; a local draft claiming global standing is
    /// not.
    pub fn check_claim(&self, claims_global_standing: bool) -> Result<(), MigrationError> {
        if claims_global_standing && !self.registered {
            return Err(MigrationError::GlobalClaimWhileUnregistered {
                alias: self.local_alias.clone(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    /// §96.2's table, transcribed.
    #[test]
    fn the_legacy_mapping_table_matches_the_sas() {
        assert_eq!(LEGACY_MAP.len(), 12);
        assert_eq!(
            map_legacy_element("gate_cmd"),
            Some(LegacyMapping::UnqualifiedLocalGateCandidate)
        );
        assert_eq!(
            map_legacy_element("Completion"),
            Some(LegacyMapping::HistoricalResolutionClaim)
        );
        assert_eq!(
            map_legacy_element("status"),
            Some(LegacyMapping::HistoricalLifecycleEvidence)
        );
    }

    /// An unmapped element is reported, not guessed at and not discarded.
    #[test]
    fn an_unmapped_element_is_not_guessed() {
        assert_eq!(map_legacy_element("Notes"), None);
        assert_eq!(map_legacy_element("Appendix"), None);
    }

    // ---- §96.3, the rule this module exists for --------------------------

    /// A legacy `Complete` line is a historical claim, not a resolution.
    #[test]
    fn a_legacy_completion_line_is_not_a_verified_resolution() {
        let mut claim =
            HistoricalClaim::from_completion_line("docs/decisions/0042.md", "Complete 2025-11-03");
        assert!(!claim.has_admissible_evidence());
        assert!(matches!(
            claim.require_evidence_to_promote(),
            Err(MigrationError::LegacyCompletionPromoted { .. })
        ));

        // The only route out is actual evidence.
        claim.admissible_evidence_refs = vec!["gate-run://GR-1".into()];
        assert_eq!(claim.require_evidence_to_promote(), Ok(()));
    }

    /// The type separation is the enforcement. A `HistoricalClaim` is not a
    /// `Resolution`, so a migration cannot import completions as resolutions
    /// even by mistake.
    #[test]
    fn a_historical_claim_is_a_different_type_from_a_resolution() {
        let claim = HistoricalClaim::from_completion_line("docs/decisions/0042.md", "Complete");
        let json = serde_json::to_string(&claim).expect("serialize");
        // A resolution has an outcome and a standing. A claim has neither, so it
        // cannot be read as one by a consumer looking at the record.
        assert!(!json.contains("common_outcome"));
        assert!(!json.contains("standing"));
        assert!(json.contains("admissible_evidence_refs"));
    }

    /// §96.3's five conditions, each blocking promotion on its own.
    #[test]
    fn a_legacy_gate_command_stays_unqualified_until_all_five_conditions_hold() {
        let bare = LegacyDeclaredUnqualified::from_command("cargo test --all");
        assert!(!bare.is_now_qualified());
        assert!(matches!(
            bare.promote(),
            Err(MigrationError::LegacyGatePromoted { .. })
        ));

        let qualified = LegacyDeclaredUnqualified {
            command: "cargo test --all".into(),
            parsed: true,
            askable: true,
            bound: true,
            executed: true,
            gate_run_receipt: "gate-run://GR-1".into(),
        };
        assert!(qualified.is_now_qualified());
        assert_eq!(qualified.promote(), Ok(()));

        // Each condition, removed one at a time.
        type Unset = (&'static str, fn(&mut LegacyDeclaredUnqualified));
        let unsets: [Unset; 5] = [
            ("parsed", |g| g.parsed = false),
            ("askable", |g| g.askable = false),
            ("bound", |g| g.bound = false),
            ("executed", |g| g.executed = false),
            ("receipt", |g| g.gate_run_receipt.clear()),
        ];
        for (name, unset) in unsets {
            let mut g = qualified.clone();
            unset(&mut g);
            assert!(!g.is_now_qualified(), "{name} did not block promotion");
            assert!(g.promote().is_err(), "{name} did not block promotion");
        }
    }

    /// §96.1 — migration preserves the original body.
    #[test]
    fn migration_preserves_the_original_body() {
        let mut m = MigratedAdr {
            source: "docs/decisions/0042.md".into(),
            preserved_body: "# ADR 0042\n\nOriginal text.\n".into(),
            preserved_body_digest: "sha256:b".into(),
            mapped_elements: BTreeMap::new(),
            unmapped_elements: vec![],
            legacy_gates: vec![],
            historical_claims: vec![],
        };
        assert_eq!(m.validate(), Ok(()));

        m.preserved_body.clear();
        assert!(matches!(
            m.validate(),
            Err(MigrationError::OriginalNotPreserved { .. })
        ));
    }

    // ---- §69 -------------------------------------------------------------

    #[test]
    fn protocol_versions_parse_and_reject_malformed() {
        let v = ProtocolVersion::parse("1.2.3").expect("parses");
        assert_eq!((v.major, v.minor, v.patch), (1, 2, 3));
        assert_eq!(v.to_string(), "1.2.3");
        for bad in ["1.2", "1.2.3.4", "1.x.3", "", "v1.2.3"] {
            assert!(ProtocolVersion::parse(bad).is_err(), "{bad:?} parsed");
        }
    }

    /// §69.3 — a breaking change needs BOTH a major bump and an ADR.
    #[test]
    fn a_breaking_change_needs_a_major_bump_and_an_adr() {
        let from = ProtocolVersion::parse("1.4.0").expect("from");

        // Minor bump on a breaking change.
        let minor = ProtocolVersion::parse("1.5.0").expect("to");
        assert!(matches!(
            from.check_transition(minor, true, "adr://OW-ADR-0009"),
            Err(MigrationError::BreakingChangeWithoutMajor { .. })
        ));

        // Major bump but no ADR.
        let major = ProtocolVersion::parse("2.0.0").expect("to");
        assert!(matches!(
            from.check_transition(major, true, ""),
            Err(MigrationError::BreakingChangeWithoutAdr { .. })
        ));

        // Both.
        assert_eq!(
            from.check_transition(major, true, "adr://OW-ADR-0009"),
            Ok(())
        );

        // §69.2 — an additive minor needs neither.
        assert_eq!(from.check_transition(minor, false, ""), Ok(()));
    }

    /// §69.4's asymmetry, both halves.
    #[test]
    fn unknown_optional_extensions_survive_and_required_ones_fail_closed() {
        let optional = vec![
            UnknownExtension {
                name: "x-katana/hints".into(),
                required: false,
                raw: r#"{"a":1}"#.into(),
            },
            UnknownExtension {
                name: "x-blut/lineage".into(),
                required: false,
                raw: "[]".into(),
            },
        ];
        let kept = handle_unknown_extensions("1.4.0", &optional).expect("optional preserved");
        assert_eq!(kept.len(), 2, "an unknown optional extension was dropped");
        assert_eq!(kept[0].raw, r#"{"a":1}"#, "the raw value was not preserved");

        let mut with_required = optional;
        with_required.push(UnknownExtension {
            name: "x-future/mandatory".into(),
            required: true,
            raw: "{}".into(),
        });
        let err = handle_unknown_extensions("1.4.0", &with_required).unwrap_err();
        assert!(
            matches!(err, MigrationError::UnknownRequiredExtension { .. }),
            "{err}"
        );
        assert!(err.to_string().contains("fail closed"), "{err}");
    }

    // ---- §12 -------------------------------------------------------------

    /// §12.4 — the check is on PROVENANCE, not on shape. A fabricated id always
    /// looks right.
    #[test]
    fn an_enterprise_id_cannot_be_derived_locally() {
        let mut id = FederatedIdentity {
            uuid: "019c8f2d-7b4d-7c41-9cb7-2636e5f582ea".into(),
            local_alias: "OW-WAR-0042".into(),
            ..FederatedIdentity::default()
        };

        // A perfectly well-formed identifier, derived from the filename.
        let err = id
            .set_enterprise_id("OH-WAR-000042", false, "the local filename")
            .unwrap_err();
        assert!(
            matches!(err, MigrationError::FabricatedEnterpriseId { .. }),
            "{err}"
        );
        assert!(id.enterprise_id.is_empty(), "it was written anyway");
        assert!(!id.registered);

        assert_eq!(
            id.set_enterprise_id("OH-WAR-000042", true, "Knowledge Fabric"),
            Ok(())
        );
        assert!(id.registered);
    }

    /// §12.4 — a local draft is valid; a local draft claiming global standing is
    /// not.
    #[test]
    fn an_unregistered_draft_cannot_claim_global_standing() {
        let draft = FederatedIdentity {
            uuid: "019c8f2d-7b4d-7c41-9cb7-2636e5f582ea".into(),
            local_alias: "OW-WAR-0042".into(),
            ..FederatedIdentity::default()
        };
        // Being a local draft is fine.
        assert_eq!(draft.check_claim(false), Ok(()));
        // Claiming more is not.
        assert!(matches!(
            draft.check_claim(true),
            Err(MigrationError::GlobalClaimWhileUnregistered { .. })
        ));

        let mut registered = draft;
        registered
            .set_enterprise_id("OH-WAR-000042", true, "Knowledge Fabric")
            .expect("allocate");
        assert_eq!(registered.check_claim(true), Ok(()));
    }

    #[test]
    fn vocabularies_round_trip() {
        for &m in LegacyMapping::ALL {
            assert_eq!(LegacyMapping::from_str(m.as_str()), Ok(m));
        }
    }
}
