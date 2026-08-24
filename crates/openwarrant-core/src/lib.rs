// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pure domain types and validators for Work Authorization Records.
//!
//! Governed by `WAR_Software_Architecture_Specification.md` v0.1.0-draft.1
//! (sha256 `aad5256cb59e3e589313b7e2d5b48360ad8c85cf1c1d65d21f9260e692dfe8e5`).
//! Section references in this crate cite that document.
//!
//! This crate holds no I/O (SAS §79.1). Everything here is a value, a parse, or
//! a validation over values — which is what lets the validators be tested against
//! in-memory fixtures instead of against a scratch directory.

#![forbid(unsafe_code)]

pub mod autonomy;
pub mod context;
pub mod deliverable;
pub mod preflight;
pub mod rationale;
pub mod traceability;
pub mod verification;
mod vocab;

pub mod adequacy;
pub mod adr;
pub mod config;
pub mod contract;
pub mod drafting;
pub mod epistemic;
pub mod execution;
pub mod frontmatter;
pub mod gate;
pub mod gate_run;
pub mod identity;
pub mod independence;
pub mod journal;
pub mod lifecycle;
pub mod manifest;
pub mod migration;
pub mod milestones;
pub mod obligation;
pub mod resolution;
pub mod role;
pub mod seam;
pub mod state;
pub mod structured;

pub use adequacy::{AdequacyError, AdequacyOutcome, AdequacyRequirement, AdequacyReview};
pub use adr::{AdrError, AdrRecord, AdrStatus};
pub use config::{ConfigError, GeneratedPolicy, Namespace, Paths, RepositoryConfig};
pub use contract::{
    ActorKind, Authorization, ContractCoverage, ContractElement, ContractError, ContractRevision,
    Independence, RevisionState,
};
pub use epistemic::{
    Adjudication, Admissibility, Claim, EpistemicError, EvidenceItem, EvidenceOrigin, Inference,
    InferenceKind, Judgment, JudgmentAuthority, Observation, PROHIBITED_SUBSTITUTIONS,
};
pub use frontmatter::{Frontmatter, FrontmatterError, Value};
pub use gate::{
    DetectionResult, EvidencePolicy, Fixture, GateBinding, GateDefinition, GateError,
    GateLifecycle, GateProvenance, GateRef, GateRegistry, Qualification,
};
pub use gate_run::{
    Askability, DependentResolution, ExecutionStatus, GateReceipt, GateRun, GateRunError,
    Invocation, MIGRATION_CLASSES, MutationDeclaration, ReasonCode, Verdict,
};
pub use identity::{IdentityError, LocalAlias, WarUuid};
pub use manifest::{
    AssuranceLevel, AtomEntry, MANIFEST_SCHEMA, Manifest, ManifestError, ParentCycle,
    ValidatedManifest, detect_parent_cycles,
};
pub use milestones::{
    ExecutorKind, MILESTONES_SCHEMA, Milestone, MilestoneError, MilestoneGraph, Port,
    ResponsibilityTier, Stage,
};
pub use obligation::{Disposition, Obligation, ObligationError, ObligationSet, ScopeKind};
pub use role::{AtomRole, Jurisdiction, Profile, RoleError, is_namespaced_extension_role};
pub use state::{
    CommonOutcome, Currency, ExecutionCondition, Phase, Provenance, ResolutionStanding, StateError,
    WarrantState,
};
pub use structured::{StructuredDoc, StructuredError, StructuredValue};
