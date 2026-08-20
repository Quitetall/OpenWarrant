// SPDX-License-Identifier: AGPL-3.0-or-later
//! Context items, trust classes, precedence, and the context manifest (SAS §33).
//!
//! # The sentence this module is built around
//!
//! §33.6: *"A required context item SHALL never be silently dropped to fit an AI
//! context budget."*
//!
//! That is the failure this exists to make impossible. A required item that does
//! not fit is a readiness failure, never an omission — [`ContextManifest`]
//! separates `included`, `omitted`, and `unresolved`, and validation refuses a
//! manifest that omits something required.
//!
//! §33.8 is the same rule one level up: *"Compaction SHALL NOT launder untrusted
//! influence."* A summary inherits the trust, classification, and taint of its
//! source set, so summarising four trusted sources and one model-generated one
//! produces a model-generated summary — not a trusted one.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::vocab::vocabulary;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ContextError {
    #[error("unknown {vocabulary} {found:?}; SAS defines {known}")]
    UnknownTerm {
        vocabulary: &'static str,
        found: String,
        known: String,
    },
    #[error(
        "context item {id:?} is required but was omitted. §33.6: a required context \
         item SHALL never be silently dropped to fit an AI context budget — if it \
         does not fit, readiness fails"
    )]
    RequiredItemOmitted { id: String },
    #[error(
        "context item {id:?} is required but did not resolve. §33.5: every required \
         item SHALL resolve before readiness"
    )]
    RequiredItemUnresolved { id: String },
    #[error(
        "context item {id:?} has role {role} and no content digest. A normative \
         reference that is not pinned can change under the Warrant that cites it"
    )]
    NormativeNotPinned { id: String, role: ContextRole },
    #[error(
        "context item {id:?} names no holder. §33.1 requires one: without it there \
         is nothing to resolve and nothing to re-resolve later"
    )]
    NoHolder { id: String },
    #[error(
        "an unresolved conflict remains between {a:?} and {b:?}, both at precedence \
         {precedence}. §33.4: equal-precedence conflicts block readiness unless an \
         explicit resolution exists"
    )]
    UnresolvedEqualPrecedence {
        a: String,
        b: String,
        precedence: Precedence,
    },
    #[error(
        "summary {id:?} claims trust {claimed} but its sources include {actual}. \
         §33.8: summaries inherit the trust of their source set, and compaction \
         SHALL NOT launder untrusted influence"
    )]
    SummaryLaundersTrust {
        id: String,
        claimed: TrustClass,
        actual: TrustClass,
    },
    #[error("a WAR SHALL declare source precedence (§33.4) and this one declares none")]
    NoPrecedenceDeclared,
}

vocabulary!(
    /// §33.2's nine context roles.
    ContextRole, "context role", ContextError, {
        Governing => "governing",
        Normative => "normative",
        Input => "input",
        Evidence => "evidence",
        Historical => "historical",
        Informative => "informative",
        NegativeControl => "negative_control",
        ToolDefinition => "tool_definition",
        Policy => "policy",
    }
);

impl ContextRole {
    /// Whether the role carries normative force, and therefore must be pinned to
    /// an immutable revision (§32.2: "normative references are immutable").
    #[must_use]
    pub const fn is_normative(self) -> bool {
        matches!(self, Self::Governing | Self::Normative | Self::Policy)
    }
}

vocabulary!(
    /// §33.3's six trust classes.
    ///
    /// Ordered from most to least trusted, which is what makes
    /// [`TrustClass::weakest`] meaningful. §33.3 also warns that "trust,
    /// classification, and authority are separate dimensions" — this type is
    /// trust alone, and nothing here should be read as an access decision.
    TrustClass, "trust class", ContextError, {
        AuthoritativeInternal => "authoritative_internal",
        AuthoritativeExternal => "authoritative_external",
        InternalUnverified => "internal_unverified",
        ExternalUntrusted => "external_untrusted",
        PerformerGenerated => "performer_generated",
        ModelGenerated => "model_generated",
    }
);

impl TrustClass {
    /// The least trusted class in a set — what §33.8 makes a summary inherit.
    ///
    /// `Ord` follows declaration order, which runs most-trusted first, so the
    /// weakest is the maximum.
    #[must_use]
    pub fn weakest<'a>(classes: impl IntoIterator<Item = &'a Self>) -> Option<Self> {
        classes.into_iter().copied().max()
    }
}

vocabulary!(
    /// §33.4's recommended default precedence, strongest first.
    Precedence, "precedence", ContextError, {
        LawAndExternalObligation => "law_and_external_obligation",
        OrganizationPolicy => "organization_policy",
        SecurityAndQualityPolicy => "security_and_quality_policy",
        AuthorizedWarContract => "authorized_war_contract",
        GoverningAdr => "governing_adr",
        SasRequirement => "sas_requirement",
        NormativeTechnicalSource => "normative_technical_source",
        InformativeSource => "informative_source",
        PerformerSuggestion => "performer_suggestion",
    }
);

/// Where a context item physically lives (§33.1).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Holder {
    pub kind: String,
    #[serde(default)]
    pub repository: String,
    /// A full revision identifier. §33.5: a draft may say "current main", and
    /// authorization SHALL resolve it to an exact revision.
    #[serde(default)]
    pub commit_sha: String,
    #[serde(default)]
    pub path: String,
}

impl Holder {
    #[must_use]
    pub fn is_declared(&self) -> bool {
        !self.kind.trim().is_empty()
    }

    /// §33.5 — whether this still points at a moving target.
    ///
    /// "current main" is legitimate in a draft and is exactly what authorization
    /// must resolve away. Detecting it is what stops a Warrant being authorized
    /// against a branch name.
    #[must_use]
    pub fn is_floating(&self) -> bool {
        let sha = self.commit_sha.trim();
        sha.is_empty() || !(sha.len() >= 40 && sha.bytes().all(|b| b.is_ascii_hexdigit()))
    }
}

/// §33.1's context item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextItem {
    pub id: String,
    pub role: ContextRole,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub holder: Holder,
    #[serde(default)]
    pub content_digest: String,
    #[serde(default)]
    pub selector_sections: Vec<String>,
    #[serde(default)]
    pub classification: String,
    pub trust: TrustClass,
    #[serde(default)]
    pub taints: Vec<String>,
    #[serde(default)]
    pub precedence: Option<Precedence>,
}

impl ContextItem {
    pub fn validate(&self) -> Result<(), ContextError> {
        if !self.holder.is_declared() {
            return Err(ContextError::NoHolder {
                id: self.id.clone(),
            });
        }
        // §32.2 — normative references are immutable. A governing source with no
        // content digest can be edited under the Warrant that cites it.
        if self.role.is_normative() && self.content_digest.trim().is_empty() {
            return Err(ContextError::NormativeNotPinned {
                id: self.id.clone(),
                role: self.role,
            });
        }
        Ok(())
    }
}

/// §33.6's compiled manifest.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ContextManifest {
    #[serde(default)]
    pub workspace_basis_ref: String,
    #[serde(default)]
    pub workspace_basis_digest: String,
    #[serde(default)]
    pub included: Vec<ContextItem>,
    /// §33.7 — omitted, WITH a reason. An omission with no reason is a drop.
    #[serde(default)]
    pub omitted: Vec<Omission>,
    #[serde(default)]
    pub unresolved: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<Conflict>,
    #[serde(default)]
    pub effective_classification: String,
    #[serde(default)]
    pub policy_digest: String,
    #[serde(default)]
    pub compiler_digest: String,
}

/// An item left out of a projection, and why (§33.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Omission {
    pub id: String,
    /// §33.7 requires a selection reason. Kept non-optional so an omission
    /// cannot be recorded without one.
    pub reason: String,
    #[serde(default)]
    pub required: bool,
}

/// Two sources that disagree (§33.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conflict {
    pub a: String,
    pub b: String,
    pub precedence_a: Precedence,
    pub precedence_b: Precedence,
    /// An explicit resolution, if one exists. §33.4 blocks readiness without it
    /// when the precedences are equal.
    #[serde(default)]
    pub resolution: Option<String>,
}

impl Conflict {
    /// Which source wins, or `None` when they are equal and unresolved.
    #[must_use]
    pub fn winner(&self) -> Option<&str> {
        match self.precedence_a.cmp(&self.precedence_b) {
            // Declaration order is strongest-first, so the SMALLER ordinal wins.
            std::cmp::Ordering::Less => Some(&self.a),
            std::cmp::Ordering::Greater => Some(&self.b),
            std::cmp::Ordering::Equal => None,
        }
    }

    #[must_use]
    pub fn blocks_readiness(&self) -> bool {
        self.winner().is_none() && self.resolution.is_none()
    }
}

impl ContextManifest {
    /// §33.5 and §33.6 — everything required is present, resolved, and not
    /// omitted; and no equal-precedence conflict is left open.
    pub fn validate(&self) -> Result<(), ContextError> {
        for item in &self.included {
            item.validate()?;
        }
        // The rule this module exists for.
        if let Some(dropped) = self.omitted.iter().find(|o| o.required) {
            return Err(ContextError::RequiredItemOmitted {
                id: dropped.id.clone(),
            });
        }
        let unresolved: BTreeSet<&str> = self.unresolved.iter().map(String::as_str).collect();
        for item in &self.included {
            if item.required && unresolved.contains(item.id.as_str()) {
                return Err(ContextError::RequiredItemUnresolved {
                    id: item.id.clone(),
                });
            }
        }
        if let Some(c) = self.conflicts.iter().find(|c| c.blocks_readiness()) {
            return Err(ContextError::UnresolvedEqualPrecedence {
                a: c.a.clone(),
                b: c.b.clone(),
                precedence: c.precedence_a,
            });
        }
        Ok(())
    }
}

/// §33.8 — a summary and the sources it was compacted from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub id: String,
    pub claimed_trust: TrustClass,
    #[serde(default)]
    pub claimed_taints: Vec<String>,
    /// The sources this summary was produced from.
    #[serde(default)]
    pub source_trust: Vec<TrustClass>,
    #[serde(default)]
    pub source_taints: Vec<String>,
}

impl Summary {
    /// The trust this summary is entitled to: the weakest of its sources.
    #[must_use]
    pub fn inherited_trust(&self) -> Option<TrustClass> {
        TrustClass::weakest(&self.source_trust)
    }

    /// Every taint of every source, inherited (§33.8).
    #[must_use]
    pub fn inherited_taints(&self) -> BTreeSet<&str> {
        self.source_taints.iter().map(String::as_str).collect()
    }

    /// §33.8 — compaction SHALL NOT launder untrusted influence.
    pub fn validate(&self) -> Result<(), ContextError> {
        let Some(actual) = self.inherited_trust() else {
            return Ok(());
        };
        if self.claimed_trust < actual {
            return Err(ContextError::SummaryLaundersTrust {
                id: self.id.clone(),
                claimed: self.claimed_trust,
                actual,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn item(id: &str, role: ContextRole, required: bool) -> ContextItem {
        ContextItem {
            id: id.into(),
            role,
            required,
            holder: Holder {
                kind: "git".into(),
                repository: "Quitetall/example".into(),
                commit_sha: "a".repeat(40),
                path: "docs/spec.md".into(),
            },
            content_digest: "sha256:abc".into(),
            selector_sections: vec!["Interface invariants".into()],
            classification: "internal".into(),
            trust: TrustClass::AuthoritativeInternal,
            taints: vec![],
            precedence: Some(Precedence::SasRequirement),
        }
    }

    /// §33.2, transcribed.
    #[test]
    fn the_context_roles_match_the_sas() {
        assert_eq!(
            ContextRole::ALL
                .iter()
                .map(|r| r.as_str())
                .collect::<Vec<_>>(),
            [
                "governing",
                "normative",
                "input",
                "evidence",
                "historical",
                "informative",
                "negative_control",
                "tool_definition",
                "policy",
            ]
        );
    }

    /// §33.3, transcribed, in the specification's order — which is also the
    /// most-to-least-trusted order `weakest` relies on.
    #[test]
    fn the_trust_classes_match_the_sas() {
        assert_eq!(
            TrustClass::ALL
                .iter()
                .map(|t| t.as_str())
                .collect::<Vec<_>>(),
            [
                "authoritative_internal",
                "authoritative_external",
                "internal_unverified",
                "external_untrusted",
                "performer_generated",
                "model_generated",
            ]
        );
    }

    /// §33.4's recommended default, transcribed, strongest first.
    #[test]
    fn the_precedence_order_matches_the_sas() {
        assert_eq!(
            Precedence::ALL
                .iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>(),
            [
                "law_and_external_obligation",
                "organization_policy",
                "security_and_quality_policy",
                "authorized_war_contract",
                "governing_adr",
                "sas_requirement",
                "normative_technical_source",
                "informative_source",
                "performer_suggestion",
            ]
        );
        assert!(
            Precedence::LawAndExternalObligation < Precedence::PerformerSuggestion,
            "declaration order must run strongest-first"
        );
    }

    /// THE rule: §33.6, a required item is never silently dropped.
    #[test]
    fn a_required_item_cannot_be_omitted_to_fit_a_budget() {
        let manifest = ContextManifest {
            omitted: vec![Omission {
                id: "CTX-001".into(),
                reason: "did not fit the model's context window".into(),
                required: true,
            }],
            ..ContextManifest::default()
        };
        assert_eq!(
            manifest.validate(),
            Err(ContextError::RequiredItemOmitted {
                id: "CTX-001".to_owned()
            }),
            "a required item was dropped to fit a budget"
        );
    }

    /// An optional item may be omitted, with a reason.
    #[test]
    fn an_optional_item_may_be_omitted_with_a_reason() {
        let manifest = ContextManifest {
            included: vec![item("CTX-001", ContextRole::Input, true)],
            omitted: vec![Omission {
                id: "CTX-002".into(),
                reason: "informative background, not needed for this stage".into(),
                required: false,
            }],
            ..ContextManifest::default()
        };
        assert_eq!(manifest.validate(), Ok(()));
    }

    #[test]
    fn a_required_item_that_did_not_resolve_blocks() {
        let manifest = ContextManifest {
            included: vec![item("CTX-001", ContextRole::Input, true)],
            unresolved: vec!["CTX-001".into()],
            ..ContextManifest::default()
        };
        assert!(matches!(
            manifest.validate(),
            Err(ContextError::RequiredItemUnresolved { .. })
        ));
    }

    /// §32.2 — a governing source that is not pinned can change underneath.
    #[test]
    fn a_normative_item_must_be_pinned() {
        for role in [
            ContextRole::Governing,
            ContextRole::Normative,
            ContextRole::Policy,
        ] {
            let mut i = item("CTX-001", role, true);
            i.content_digest = String::new();
            assert!(
                matches!(i.validate(), Err(ContextError::NormativeNotPinned { .. })),
                "{role} was allowed without a digest"
            );
        }
        // A merely informative source need not be pinned.
        let mut informative = item("CTX-002", ContextRole::Informative, false);
        informative.content_digest = String::new();
        assert_eq!(informative.validate(), Ok(()));
    }

    #[test]
    fn an_item_with_no_holder_is_refused() {
        let mut i = item("CTX-001", ContextRole::Input, true);
        i.holder = Holder::default();
        assert!(matches!(i.validate(), Err(ContextError::NoHolder { .. })));
    }

    /// §33.5 — "current main" is a draft phrase that authorization must resolve.
    #[test]
    fn a_floating_holder_is_detected() {
        let mut h = Holder {
            kind: "git".into(),
            repository: "Quitetall/example".into(),
            commit_sha: "main".into(),
            path: "docs/spec.md".into(),
        };
        assert!(h.is_floating(), "'main' is not an exact revision");
        h.commit_sha = String::new();
        assert!(h.is_floating());
        h.commit_sha = "a".repeat(40);
        assert!(!h.is_floating());
        h.commit_sha = "z".repeat(40);
        assert!(h.is_floating(), "not hex, so not a revision");
    }

    /// §33.4 — precedence decides, and equal precedence blocks.
    #[test]
    fn precedence_decides_and_ties_block() {
        let decided = Conflict {
            a: "policy".into(),
            b: "suggestion".into(),
            precedence_a: Precedence::OrganizationPolicy,
            precedence_b: Precedence::PerformerSuggestion,
            resolution: None,
        };
        assert_eq!(decided.winner(), Some("policy"));
        assert!(!decided.blocks_readiness());

        let tie = Conflict {
            a: "src-a".into(),
            b: "src-b".into(),
            precedence_a: Precedence::SasRequirement,
            precedence_b: Precedence::SasRequirement,
            resolution: None,
        };
        assert_eq!(tie.winner(), None);
        assert!(tie.blocks_readiness());

        let resolved = Conflict {
            resolution: Some("ADR OW-0007 chose src-a".into()),
            ..tie
        };
        assert!(
            !resolved.blocks_readiness(),
            "an explicit resolution unblocks (§33.4)"
        );
    }

    #[test]
    fn an_unresolved_tie_blocks_the_manifest() {
        let manifest = ContextManifest {
            conflicts: vec![Conflict {
                a: "src-a".into(),
                b: "src-b".into(),
                precedence_a: Precedence::GoverningAdr,
                precedence_b: Precedence::GoverningAdr,
                resolution: None,
            }],
            ..ContextManifest::default()
        };
        assert!(matches!(
            manifest.validate(),
            Err(ContextError::UnresolvedEqualPrecedence { .. })
        ));
    }

    // ---- §33.8, the laundering rule ----------------------------------------

    /// One model-generated source makes the whole summary model-generated.
    #[test]
    fn a_summary_cannot_launder_its_weakest_source() {
        let s = Summary {
            id: "SUM-001".into(),
            claimed_trust: TrustClass::AuthoritativeInternal,
            claimed_taints: vec![],
            source_trust: vec![
                TrustClass::AuthoritativeInternal,
                TrustClass::AuthoritativeInternal,
                TrustClass::AuthoritativeInternal,
                TrustClass::AuthoritativeInternal,
                TrustClass::ModelGenerated,
            ],
            source_taints: vec![],
        };
        assert_eq!(s.inherited_trust(), Some(TrustClass::ModelGenerated));
        assert!(
            matches!(s.validate(), Err(ContextError::SummaryLaundersTrust { .. })),
            "four trusted sources and one model-generated one produced a trusted summary"
        );
    }

    #[test]
    fn a_summary_claiming_its_inherited_trust_is_fine() {
        let s = Summary {
            id: "SUM-001".into(),
            claimed_trust: TrustClass::ModelGenerated,
            claimed_taints: vec![],
            source_trust: vec![
                TrustClass::AuthoritativeInternal,
                TrustClass::ModelGenerated,
            ],
            source_taints: vec!["untrusted-web".into()],
        };
        assert_eq!(s.validate(), Ok(()));
        assert!(s.inherited_taints().contains("untrusted-web"));
    }

    /// Claiming LESS trust than inherited is honest and allowed.
    #[test]
    fn a_summary_may_claim_less_trust_than_it_inherits() {
        let s = Summary {
            id: "SUM-001".into(),
            claimed_trust: TrustClass::ExternalUntrusted,
            claimed_taints: vec![],
            source_trust: vec![TrustClass::AuthoritativeInternal],
            source_taints: vec![],
        };
        assert_eq!(s.validate(), Ok(()));
    }

    /// A summary with NO sources inherits nothing and so launders nothing.
    ///
    /// This is deliberate rather than an oversight: §33.8 constrains what a
    /// summary may claim RELATIVE to its sources, and with no source set there is
    /// no relation to violate. A summary that cites no sources is a different
    /// problem — it is an unsourced assertion, which §35.2 handles.
    #[test]
    fn a_summary_with_no_sources_inherits_nothing() {
        let s = Summary {
            id: "SUM-002".into(),
            claimed_trust: TrustClass::AuthoritativeInternal,
            claimed_taints: vec![],
            source_trust: vec![],
            source_taints: vec![],
        };
        assert_eq!(s.inherited_trust(), None);
        assert_eq!(s.validate(), Ok(()));
    }

    #[test]
    fn weakest_of_nothing_is_nothing() {
        assert_eq!(TrustClass::weakest(&[]), None);
        assert_eq!(
            TrustClass::weakest(&[TrustClass::InternalUnverified]),
            Some(TrustClass::InternalUnverified)
        );
    }

    #[test]
    fn vocabularies_round_trip_and_name_their_alternatives() {
        for &r in ContextRole::ALL {
            assert_eq!(ContextRole::from_str(r.as_str()), Ok(r));
        }
        for &t in TrustClass::ALL {
            assert_eq!(TrustClass::from_str(t.as_str()), Ok(t));
        }
        for &p in Precedence::ALL {
            assert_eq!(Precedence::from_str(p.as_str()), Ok(p));
        }
        let err = ContextRole::from_str("advisory").unwrap_err();
        assert!(err.to_string().contains("governing"), "{err}");
    }

    #[test]
    fn a_manifest_round_trips_through_json() {
        let m = ContextManifest {
            workspace_basis_ref: "workspace://basis".into(),
            workspace_basis_digest: "sha256:w".into(),
            included: vec![item("CTX-001", ContextRole::Normative, true)],
            omitted: vec![],
            unresolved: vec![],
            conflicts: vec![],
            effective_classification: "internal".into(),
            policy_digest: "sha256:p".into(),
            compiler_digest: "sha256:c".into(),
        };
        let s = serde_json::to_string(&m).expect("serialize");
        assert_eq!(
            serde_json::from_str::<ContextManifest>(&s).expect("deserialize"),
            m
        );
    }
}
