// SPDX-License-Identifier: AGPL-3.0-or-later
//! Gate Run semantics (SAS §44) and gate invalidation (§45). RQ-054, RQ-057.
//!
//! # The one rule this module exists for
//!
//! SAS §99 acceptance criterion 19: **unaskable gates cannot pass.**
//!
//! §44 opens "A Gate Run SHALL separate three results" and then gives three
//! independent vocabularies — askability (§44.1), execution status (§44.2), and
//! verdict (§44.3). §44.5 says only one combination of the three satisfies a
//! required pass:
//!
//! ```yaml
//! askability: "askable"
//! execution_status: "completed"
//! verdict: "pass"
//! ```
//!
//! That is a conjunction over 2 × 6 × 3 = 36 possible triples, exactly one of
//! which passes. [`GateRun::satisfies_required_pass`] is that conjunction, and
//! `every_triple_except_one_fails_a_required_pass` enumerates all 36 rather than
//! sampling — because criterion 19 is a claim about the whole space, and a claim
//! about a whole space tested on three examples is a claim about three examples.
//!
//! # Two vocabularies that are not the same vocabulary
//!
//! OW-WAR-0020's Intent describes "§44.2's execution statuses" as the ten-item
//! list `malformed, foreign_working_directory, missing_tool, missing_script,
//! missing_crate, mutating, timeout, failed, passed, not_run`. Those ten are
//! **§96.4's migration classes**, not §44.2's execution statuses, which are the
//! six in [`ExecutionStatus`]. §44.4's own worked example shows where the ten
//! actually live:
//!
//! ```yaml
//! askability: "not_askable"
//! execution_status: "not_run"
//! verdict: "unknown"
//! reason_code: "missing_tool"
//! ```
//!
//! `missing_tool` is a [`ReasonCode`]. Both vocabularies are implemented, both
//! are total, and [`ReasonCode::migration_class`] is the mapping §96.4 requires —
//! which is what makes "SHALL not collapse *could not ask* into *failed*"
//! checkable rather than aspirational. OW-ADR-0006 records the discrepancy and
//! why the specification won.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// §45's outcomes live in §24's vocabulary, not a second one.
///
/// §24 already defines resolution standing as valid / disputed / annulled, and
/// §45 says dependents "become disputed according to policy" and that an
/// authorized action later "resolves the dispute or annuls the resolution" —
/// the same three words. A parallel enum here would have been a second source of
/// truth for one concept.
pub use crate::state::ResolutionStanding;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GateRunError {
    #[error("unknown {vocabulary} {found:?}; SAS defines {known}")]
    UnknownTerm {
        vocabulary: &'static str,
        found: String,
        known: String,
    },
    #[error(
        "run {run:?} is not_askable but records verdict {verdict}. §44.4 pairs \
         not_askable with verdict unknown: a gate that could not be asked has no \
         result to report, and reporting one is a fabrication"
    )]
    UnaskableWithResult { run: String, verdict: Verdict },
    #[error(
        "run {run:?} is not_askable but execution_status is {status}. A gate that \
         could not be asked did not run; §44.4 shows not_run and invalid"
    )]
    UnaskableButExecuted {
        run: String,
        status: ExecutionStatus,
    },
    #[error(
        "run {run:?} is askable and completed but verdict is unknown. A completed \
         execution produced an answer; `unknown` here hides which one"
    )]
    CompletedWithoutVerdict { run: String },
    #[error(
        "run {run:?} did not complete (execution_status {status}) but records \
         verdict {verdict}. Only a completed execution yields pass or fail (§44.5)"
    )]
    IncompleteWithVerdict {
        run: String,
        status: ExecutionStatus,
        verdict: Verdict,
    },
    #[error(
        "run {run:?} is not_askable but records no reason_code. \"Could not ask\" \
         with no reason recorded is the state §96.4 forbids collapsing into \
         \"failed\" — and with no reason there is nothing to stop it"
    )]
    UnaskableWithoutReason { run: String },
    #[error(
        "gate {gate:?} declares itself mutating (§44.8) and cannot run in a routine \
         check. A mutating verification action must declare effects, authority, and \
         compensation, and be dispatched deliberately"
    )]
    MutatingGateQuarantined { gate: String },
    #[error(
        "gate {gate:?} was given a raw shell string but does not own shell parsing. \
         §44.7 permits a raw command only through a gate that explicitly owns shell \
         parsing and classification; structured argument vectors are preferred"
    )]
    UnownedShellString { gate: String },
    #[error("receipt for run {run:?} omits {field}, which §44.6 requires")]
    ReceiptIncomplete { run: String, field: &'static str },
    #[error(
        "receipt for run {run:?} names noncanonical producer actor {actor:?}; leading or trailing whitespace creates an identity alias"
    )]
    ReceiptProducerNotCanonical { run: String, actor: String },
    #[error("receipt for run {run:?} has a blank, duplicate, or noncanonical {field} inventory")]
    ReceiptInventoryNotCanonical { run: String, field: &'static str },
    #[error("receipt for run {run:?} has {observed} items in {field}; the maximum is {limit}")]
    ReceiptInventoryTooLarge {
        run: String,
        field: &'static str,
        observed: usize,
        limit: usize,
    },
    #[error(
        "receipt for run {run:?} declares {declared} selected tests but its canonical manifest contains {observed}"
    )]
    ReceiptSelectedTestCountMismatch {
        run: String,
        declared: u64,
        observed: usize,
    },
    #[error(
        "receipt for run {run:?} has noncanonical or nonmonotonic chronology {started_at:?} -> {completed_at:?}"
    )]
    ReceiptChronologyInvalid {
        run: String,
        started_at: String,
        completed_at: String,
    },
    #[error("receipt for run {run:?} uses schema {found:?}; expected {expected:?}")]
    ReceiptSchemaMismatch {
        run: String,
        found: String,
        expected: &'static str,
    },
    #[error("receipt for run {run:?} uses kind {found:?}; expected {expected:?}")]
    ReceiptKindMismatch {
        run: String,
        found: String,
        expected: &'static str,
    },
    #[error(
        "receipt for run {run:?} records exit result {exit_result:?} with verdict {verdict}; a zero exit code is pass and every nonzero code or signal is fail"
    )]
    ReceiptExitVerdictMismatch {
        run: String,
        exit_result: GateExitResult,
        verdict: Verdict,
    },
    #[error("test-selection observation for run {run:?} omits {field}")]
    SelectionObservationIncomplete { run: String, field: &'static str },
    #[error(
        "test-selection observation for run {run:?} uses schema {found:?}; expected {expected:?}"
    )]
    SelectionObservationSchemaMismatch {
        run: String,
        found: String,
        expected: &'static str,
    },
    #[error(
        "test-selection observation for run {run:?} uses kind {found:?}; expected {expected:?}"
    )]
    SelectionObservationKindMismatch {
        run: String,
        found: String,
        expected: &'static str,
    },
    #[error(
        "test-selection observation for run {run:?} has a blank, duplicate, or noncanonical manifest"
    )]
    SelectionObservationManifestNotCanonical { run: String },
    #[error(
        "test-selection observation for run {run:?} declares {declared} selected tests but contains {observed}"
    )]
    SelectionObservationCountMismatch {
        run: String,
        declared: u64,
        observed: usize,
    },
}

macro_rules! vocabulary {
    ($name:ident, $label:literal, { $($variant:ident => $text:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }
        }

        impl FromStr for $name {
            type Err = GateRunError;
            fn from_str(s: &str) -> Result<Self, GateRunError> {
                Self::ALL.iter().copied().find(|v| v.as_str() == s).ok_or_else(|| {
                    GateRunError::UnknownTerm {
                        vocabulary: $label,
                        found: s.to_owned(),
                        known: Self::ALL.iter().map(|v| v.as_str()).collect::<Vec<_>>().join(", "),
                    }
                })
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

vocabulary!(Askability, "askability", {
    Askable => "askable",
    NotAskable => "not_askable",
});

vocabulary!(ExecutionStatus, "execution status", {
    NotRun => "not_run",
    Completed => "completed",
    Timeout => "timeout",
    InfrastructureError => "infrastructure_error",
    Cancelled => "cancelled",
    Invalid => "invalid",
});

vocabulary!(Verdict, "verdict", {
    Pass => "pass",
    Fail => "fail",
    Unknown => "unknown",
});

vocabulary!(ReasonCode, "reason code", {
    Malformed => "malformed",
    ForeignWorkingDirectory => "foreign_working_directory",
    MissingTool => "missing_tool",
    MissingScript => "missing_script",
    MissingCrate => "missing_crate",
    Mutating => "mutating",
    Timeout => "timeout",
    Failed => "failed",
    Passed => "passed",
    NotRun => "not_run",
    ZeroSelectedTests => "zero_selected_tests",
});

/// §96.4's ten preserved migration classes, in the specification's order.
///
/// `zero_selected_tests` is a §44.4 reason code and is deliberately NOT in this
/// list: §96.4 enumerates exactly ten and migration preserves those ten.
pub const MIGRATION_CLASSES: [ReasonCode; 10] = [
    ReasonCode::Malformed,
    ReasonCode::ForeignWorkingDirectory,
    ReasonCode::MissingTool,
    ReasonCode::MissingScript,
    ReasonCode::MissingCrate,
    ReasonCode::Mutating,
    ReasonCode::Timeout,
    ReasonCode::Failed,
    ReasonCode::Passed,
    ReasonCode::NotRun,
];

impl ReasonCode {
    /// Whether §96.4 preserves this class through migration.
    #[must_use]
    pub fn is_migration_class(self) -> bool {
        MIGRATION_CLASSES.contains(&self)
    }

    /// Whether this class means the gate could not be ASKED, as opposed to
    /// having been asked and answered.
    ///
    /// BROADER than the migration vocabulary: `zero_selected_tests` is a
    /// could-not-ask class and is not one of §96.4's ten. Use
    /// [`Self::is_migration_class`] to decide migration behaviour and this to
    /// decide whether a result exists.
    ///
    /// §96.4: migration "SHALL not collapse 'could not ask' into 'failed'." This
    /// predicate is what makes that sentence testable — every class is on exactly
    /// one side of it, and `failed` is on the other side from all seven
    /// could-not-ask classes.
    #[must_use]
    pub const fn is_could_not_ask(self) -> bool {
        match self {
            Self::Malformed
            | Self::ForeignWorkingDirectory
            | Self::MissingTool
            | Self::MissingScript
            | Self::MissingCrate
            | Self::Mutating
            | Self::NotRun
            | Self::ZeroSelectedTests => true,
            // Asked and answered, or asked and cut short having started.
            Self::Timeout | Self::Failed | Self::Passed => false,
        }
    }

    /// Translate one LamQuant `adr_exec` label into a §96.4 class.
    ///
    /// §96.4 names its ten classes in snake_case. The corpus about to be
    /// imported does not: `tools/adr_exec.py` writes SCREAMING-KEBAB, and those
    /// labels are what `tools/adr-gate-ceilings.json` records. Without a stated
    /// translation an importer has to guess at that seam, and guessing is what
    /// §96.4 exists to stop — so the mapping is written down, once, here.
    ///
    /// Measured against LamQuant at `ee950b80`: `adr_exec.py` emits exactly nine
    /// labels — PASS, FAIL, TIMEOUT, MALFORMED, MUTATING, FOREIGN-CWD,
    /// MISSING-TOOL, MISSING-SCRIPT, MISSING-CRATE — and all nine appear below.
    ///
    /// §96.4's tenth, `not_run`, has NO legacy label, deliberately. `adr_exec`
    /// cannot emit it: a tool that ran produced one of the nine. `not_run` is the
    /// class for a gate that was declared and never executed at all, which per
    /// LamQuant ADR 0186 finding F2 was the state of every gate in that corpus
    /// for months. The importer must supply it from the ABSENCE of a result,
    /// never from a label — which is exactly why it is missing from this table.
    ///
    /// Returns `None` for an unrecognised label rather than a default. An
    /// importer that cannot name a result reports it; §96.4 forbids folding the
    /// unknown into `failed`.
    #[must_use]
    pub fn from_legacy_execution_label(label: &str) -> Option<Self> {
        Some(match label {
            "PASS" => Self::Passed,
            "FAIL" => Self::Failed,
            "TIMEOUT" => Self::Timeout,
            "MALFORMED" => Self::Malformed,
            "MUTATING" => Self::Mutating,
            "FOREIGN-CWD" => Self::ForeignWorkingDirectory,
            "MISSING-TOOL" => Self::MissingTool,
            "MISSING-SCRIPT" => Self::MissingScript,
            "MISSING-CRATE" => Self::MissingCrate,
            _ => return None,
        })
    }

    /// The §44 triple this legacy class migrates to.
    ///
    /// Defined on ALL variants, not only the ten migration classes, so that
    /// `zero_selected_tests` has an answer too.
    ///
    /// Total by construction: adding a variant without deciding its migration
    /// stops the crate compiling rather than defaulting it to `failed`, which is
    /// the collapse §96.4 forbids.
    #[must_use]
    pub const fn migration_target(self) -> (Askability, ExecutionStatus, Verdict) {
        use ExecutionStatus as E;
        match self {
            Self::Passed => (Askability::Askable, E::Completed, Verdict::Pass),
            Self::Failed => (Askability::Askable, E::Completed, Verdict::Fail),
            Self::Timeout => (Askability::Askable, E::Timeout, Verdict::Unknown),
            Self::Malformed | Self::ZeroSelectedTests => {
                (Askability::NotAskable, E::Invalid, Verdict::Unknown)
            }
            Self::ForeignWorkingDirectory
            | Self::MissingTool
            | Self::MissingScript
            | Self::MissingCrate
            | Self::Mutating
            | Self::NotRun => (Askability::NotAskable, E::NotRun, Verdict::Unknown),
        }
    }
}

/// A Gate Run: §44's three separated results, plus §44.4's reason code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateRun {
    pub id: String,
    /// The `<gate_id>@<version>` this run exercised.
    pub gate: String,
    pub askability: Askability,
    pub execution_status: ExecutionStatus,
    pub verdict: Verdict,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<ReasonCode>,
}

impl GateRun {
    /// §44.5 — the ONLY combination that satisfies a required pass.
    ///
    /// This is SAS §99 criterion 19 ("unaskable gates cannot pass") expressed as
    /// a conjunction rather than as a promise. It is deliberately written as one
    /// expression over all three fields: a version that checked `verdict` first
    /// and the others "if relevant" is how an unaskable gate ends up passing.
    #[must_use]
    pub fn satisfies_required_pass(&self) -> bool {
        self.askability == Askability::Askable
            && self.execution_status == ExecutionStatus::Completed
            && self.verdict == Verdict::Pass
    }

    /// Whether this run leaves a REQUIRED obligation unresolved rather than
    /// answered. RQ-054: a required unknown blocks resolution.
    ///
    /// This is NOT a synonym for "unaskable". An askable run that timed out or
    /// hit an infrastructure error is also a blocking unknown, and reporting one
    /// of those as "could not ask" is as wrong as reporting it as a failure. Ask
    /// [`Self::askability`] when the question is whether the gate was reachable.
    #[must_use]
    pub fn is_blocking_unknown(&self) -> bool {
        self.verdict == Verdict::Unknown
    }

    /// Coherence across the three vocabularies (§44.1–§44.5).
    ///
    /// The vocabularies are independent, which means most of their 36 triples are
    /// nonsense. Refusing the incoherent ones is what stops a caller from
    /// constructing `not_askable + completed + pass` and having it satisfy §44.5.
    pub fn validate(&self) -> Result<(), GateRunError> {
        match self.askability {
            Askability::NotAskable => {
                if self.verdict != Verdict::Unknown {
                    return Err(GateRunError::UnaskableWithResult {
                        run: self.id.clone(),
                        verdict: self.verdict,
                    });
                }
                if !matches!(
                    self.execution_status,
                    ExecutionStatus::NotRun | ExecutionStatus::Invalid
                ) {
                    return Err(GateRunError::UnaskableButExecuted {
                        run: self.id.clone(),
                        status: self.execution_status,
                    });
                }
                if self.reason_code.is_none() {
                    return Err(GateRunError::UnaskableWithoutReason {
                        run: self.id.clone(),
                    });
                }
            }
            Askability::Askable => {
                if self.execution_status == ExecutionStatus::Completed {
                    if self.verdict == Verdict::Unknown {
                        return Err(GateRunError::CompletedWithoutVerdict {
                            run: self.id.clone(),
                        });
                    }
                } else if self.verdict != Verdict::Unknown {
                    return Err(GateRunError::IncompleteWithVerdict {
                        run: self.id.clone(),
                        status: self.execution_status,
                        verdict: self.verdict,
                    });
                }
            }
        }
        Ok(())
    }

    /// Build the run a legacy §96.4 class migrates to, preserving the class.
    #[must_use]
    pub fn from_migration_class(id: &str, gate: &str, class: ReasonCode) -> Self {
        let (askability, execution_status, verdict) = class.migration_target();
        Self {
            id: id.to_owned(),
            gate: gate.to_owned(),
            askability,
            execution_status,
            verdict,
            reason_code: Some(class),
        }
    }
}

/// Versioned wire identity for §44.6 Gate receipts.
pub const GATE_RECEIPT_SCHEMA: &str = "oh.war/gate-receipt/v1";
pub const GATE_RECEIPT_KIND: &str = "gate_receipt";
pub const TEST_SELECTION_OBSERVATION_SCHEMA: &str = "oh.war/test-selection-observation/v1";
pub const TEST_SELECTION_OBSERVATION_KIND: &str = "test_selection_observation";

/// Gate receipts summarize evidence; larger inventories belong in a referenced
/// artifact rather than inline in the receipt. These ceilings bound the work
/// performed by canonicality and duplicate validation while leaving room for
/// large repositories and test suites.
pub const GATE_RECEIPT_MAX_SUBJECT_DIGESTS: usize = 4_096;
pub const GATE_RECEIPT_MAX_FIXTURE_DIGESTS: usize = 4_096;
pub const GATE_RECEIPT_MAX_SELECTED_TESTS: usize = 4_096;
pub const GATE_RECEIPT_MAX_RAW_EVIDENCE_REFS: usize = 4_096;

/// Exact process outcome supporting a completed Gate verdict.
///
/// A verdict is a semantic conclusion; an exit result is an OS observation.
/// Keeping them as separate typed fields prevents `"pass"` from masquerading
/// as both facts and preserves signal termination on platforms that expose it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GateExitResult {
    ExitCode { code: i32 },
    Signal { signal: i32 },
}

/// Unknown optional fields preserved on a Gate receipt.
///
/// Receipt extensions follow the repository-wide evolution rule: keys must be
/// namespaced with nonempty text on both sides of the first `.`. This keeps an
/// older reader from silently accepting new required vocabulary while still
/// allowing optional producer metadata to survive a read/write cycle.
pub type GateReceiptExtensions = crate::legacy_disposition::NamespacedExtensions;

impl GateExitResult {
    #[must_use]
    pub const fn passed(self) -> bool {
        matches!(self, Self::ExitCode { code: 0 })
    }
}

/// Runner-owned observation of tests selected by an exact execution adapter.
///
/// Gate Definition selection is planned policy. This object records what the
/// qualified adapter selected for one completed process. A receipt duplicates
/// the count and manifest required by §44.6, then binds these exact bytes so a
/// resolver can require planned and observed selection to agree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestSelectionObservation {
    pub schema: String,
    pub kind: String,
    pub run_id: String,
    pub gate_definition_digest: String,
    pub adapter: String,
    pub selected_test_count: u64,
    pub selected_test_manifest: Vec<String>,
}

impl TestSelectionObservation {
    pub fn validate(&self) -> Result<(), GateRunError> {
        if self.schema != TEST_SELECTION_OBSERVATION_SCHEMA {
            return Err(GateRunError::SelectionObservationSchemaMismatch {
                run: self.run_id.clone(),
                found: self.schema.clone(),
                expected: TEST_SELECTION_OBSERVATION_SCHEMA,
            });
        }
        if self.kind != TEST_SELECTION_OBSERVATION_KIND {
            return Err(GateRunError::SelectionObservationKindMismatch {
                run: self.run_id.clone(),
                found: self.kind.clone(),
                expected: TEST_SELECTION_OBSERVATION_KIND,
            });
        }
        for (field, value) in [
            ("run_id", self.run_id.as_str()),
            (
                "gate_definition_digest",
                self.gate_definition_digest.as_str(),
            ),
            ("adapter", self.adapter.as_str()),
        ] {
            if value.is_empty() || value.trim() != value {
                return Err(GateRunError::SelectionObservationIncomplete {
                    run: self.run_id.clone(),
                    field,
                });
            }
        }
        let selected: BTreeSet<&str> = self
            .selected_test_manifest
            .iter()
            .map(String::as_str)
            .collect();
        if self.selected_test_manifest.is_empty()
            || self.selected_test_manifest.len() > GATE_RECEIPT_MAX_SELECTED_TESTS
            || selected.len() != self.selected_test_manifest.len()
            || self
                .selected_test_manifest
                .iter()
                .any(|test| test.is_empty() || test.trim() != test)
        {
            return Err(GateRunError::SelectionObservationManifestNotCanonical {
                run: self.run_id.clone(),
            });
        }
        if usize::try_from(self.selected_test_count).ok() != Some(self.selected_test_manifest.len())
        {
            return Err(GateRunError::SelectionObservationCountMismatch {
                run: self.run_id.clone(),
                declared: self.selected_test_count,
                observed: self.selected_test_manifest.len(),
            });
        }
        Ok(())
    }
}

/// §44.6's receipt. Every field the specification lists, none optional.
///
/// Deliberately NOT `Default`: a receipt of empty strings and a zero test count
/// is not an unfilled receipt, it is a claim that a gate ran in no environment
/// with no arguments and selected no tests. Receipts are constructed field by
/// field or not at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateReceipt {
    pub schema: String,
    pub kind: String,
    pub run_id: String,
    /// SHA-256 of the exact serialized `.run.toml` bytes written beside this
    /// receipt, not a second semantic serialization of the parsed object.
    pub gate_run_digest: String,
    pub gate_definition_digest: String,
    pub gate_binding_digest: String,
    pub subject_digests: Vec<String>,
    pub fixture_digests: Vec<String>,
    /// Runner tool or instrument identity.
    pub runner: String,
    /// Stable actor identity that produced the gate evidence. This is separate
    /// from `runner` so authorship checks never compare an actor to a binary or
    /// command name.
    pub producer_actor: String,
    pub runtime_environment: String,
    pub arguments: Vec<String>,
    pub working_directory: String,
    pub started_at: String,
    pub completed_at: String,
    pub exit_result: GateExitResult,
    pub selected_test_count: u64,
    pub selected_test_manifest: Vec<String>,
    pub selection_observation_ref: String,
    pub selection_observation_digest: String,
    pub raw_evidence_refs: Vec<String>,
    pub stdout_ref: String,
    pub stdout_digest: String,
    pub stderr_ref: String,
    pub stderr_digest: String,
    pub resource_usage: String,
    pub verdict: Verdict,
    pub receipt_digest: String,
    #[serde(flatten, default)]
    pub extensions: GateReceiptExtensions,
}

impl GateReceipt {
    /// §44.6 — a receipt missing a required field is not a receipt.
    ///
    /// Scalar fields only. The list fields are legitimately empty (a gate with no
    /// fixtures has no fixture digests), and requiring them non-empty would push
    /// authors to invent entries, which is worse than an honest empty list.
    pub fn validate(&self) -> Result<(), GateRunError> {
        if self.schema != GATE_RECEIPT_SCHEMA {
            return Err(GateRunError::ReceiptSchemaMismatch {
                run: self.run_id.clone(),
                found: self.schema.clone(),
                expected: GATE_RECEIPT_SCHEMA,
            });
        }
        if self.kind != GATE_RECEIPT_KIND {
            return Err(GateRunError::ReceiptKindMismatch {
                run: self.run_id.clone(),
                found: self.kind.clone(),
                expected: GATE_RECEIPT_KIND,
            });
        }
        let required: [(&'static str, &str); 17] = [
            ("gate_run_digest", &self.gate_run_digest),
            ("gate_definition_digest", &self.gate_definition_digest),
            ("gate_binding_digest", &self.gate_binding_digest),
            ("runner", &self.runner),
            ("producer_actor", &self.producer_actor),
            ("runtime_environment", &self.runtime_environment),
            ("working_directory", &self.working_directory),
            ("started_at", &self.started_at),
            ("completed_at", &self.completed_at),
            ("selection_observation_ref", &self.selection_observation_ref),
            (
                "selection_observation_digest",
                &self.selection_observation_digest,
            ),
            ("stdout_ref", &self.stdout_ref),
            ("stdout_digest", &self.stdout_digest),
            ("stderr_ref", &self.stderr_ref),
            ("stderr_digest", &self.stderr_digest),
            ("resource_usage", &self.resource_usage),
            ("receipt_digest", &self.receipt_digest),
        ];
        for (field, value) in required {
            if value.trim().is_empty() {
                return Err(GateRunError::ReceiptIncomplete {
                    run: self.run_id.clone(),
                    field,
                });
            }
        }
        if self.producer_actor.trim() != self.producer_actor {
            return Err(GateRunError::ReceiptProducerNotCanonical {
                run: self.run_id.clone(),
                actor: self.producer_actor.clone(),
            });
        }
        for (field, values, required, limit) in [
            (
                "subject_digests",
                &self.subject_digests,
                true,
                GATE_RECEIPT_MAX_SUBJECT_DIGESTS,
            ),
            (
                "fixture_digests",
                &self.fixture_digests,
                false,
                GATE_RECEIPT_MAX_FIXTURE_DIGESTS,
            ),
            (
                "selected_test_manifest",
                &self.selected_test_manifest,
                true,
                GATE_RECEIPT_MAX_SELECTED_TESTS,
            ),
            (
                "raw_evidence_refs",
                &self.raw_evidence_refs,
                false,
                GATE_RECEIPT_MAX_RAW_EVIDENCE_REFS,
            ),
        ] {
            if values.len() > limit {
                return Err(GateRunError::ReceiptInventoryTooLarge {
                    run: self.run_id.clone(),
                    field,
                    observed: values.len(),
                    limit,
                });
            }
            let canonical: BTreeSet<&str> = values.iter().map(String::as_str).collect();
            if (required && values.is_empty())
                || canonical.len() != values.len()
                || values
                    .iter()
                    .any(|value| value.is_empty() || value.trim() != value)
            {
                return Err(GateRunError::ReceiptInventoryNotCanonical {
                    run: self.run_id.clone(),
                    field,
                });
            }
        }
        if usize::try_from(self.selected_test_count).ok() != Some(self.selected_test_manifest.len())
        {
            return Err(GateRunError::ReceiptSelectedTestCountMismatch {
                run: self.run_id.clone(),
                declared: self.selected_test_count,
                observed: self.selected_test_manifest.len(),
            });
        }
        if !crate::legacy_disposition::is_canonical_utc(&self.started_at)
            || !crate::legacy_disposition::is_canonical_utc(&self.completed_at)
            || self.started_at > self.completed_at
        {
            return Err(GateRunError::ReceiptChronologyInvalid {
                run: self.run_id.clone(),
                started_at: self.started_at.clone(),
                completed_at: self.completed_at.clone(),
            });
        }
        let exit_and_verdict_agree = match self.exit_result {
            GateExitResult::ExitCode { code: 0 } => self.verdict == Verdict::Pass,
            GateExitResult::ExitCode { .. } | GateExitResult::Signal { .. } => {
                self.verdict == Verdict::Fail
            }
        };
        if !exit_and_verdict_agree {
            return Err(GateRunError::ReceiptExitVerdictMismatch {
                run: self.run_id.clone(),
                exit_result: self.exit_result,
                verdict: self.verdict,
            });
        }
        Ok(())
    }
}

/// §44.7 — how a gate's command is expressed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Invocation {
    /// Preferred: a structured argument vector, no shell involved.
    ArgVector(Vec<String>),
    /// Permitted only through a gate that explicitly owns shell parsing.
    ShellString(String),
}

impl Invocation {
    /// §44.7 — a raw shell string needs a gate that owns shell parsing.
    pub fn validate(&self, gate: &str, gate_owns_shell_parsing: bool) -> Result<(), GateRunError> {
        match self {
            Self::ArgVector(_) => Ok(()),
            Self::ShellString(_) if gate_owns_shell_parsing => Ok(()),
            Self::ShellString(_) => Err(GateRunError::UnownedShellString {
                gate: gate.to_owned(),
            }),
        }
    }
}

/// §44.8 — a mutating verification action changes state while measuring it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MutationDeclaration {
    #[serde(default)]
    pub effects: Vec<String>,
    #[serde(default)]
    pub authority: String,
    #[serde(default)]
    pub compensation: String,
}

impl MutationDeclaration {
    /// §44.8 — "It cannot run merely because an old document contains a command
    /// string." A routine check never runs a mutating gate, however well declared.
    pub fn quarantine(gate: &str) -> Result<(), GateRunError> {
        Err(GateRunError::MutatingGateQuarantined {
            gate: gate.to_owned(),
        })
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self.effects.is_empty()
            && !self.authority.trim().is_empty()
            && !self.compensation.trim().is_empty()
    }
}

/// A resolution, for the purpose of §45's propagation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependentResolution {
    pub id: String,
    /// Gate run ids this resolution materially rests on.
    #[serde(default)]
    pub rests_on_runs: Vec<String>,
    /// Other resolutions this one materially rests on.
    #[serde(default)]
    pub rests_on_resolutions: Vec<String>,
    #[serde(default = "DependentResolution::valid")]
    pub standing: ResolutionStanding,
}

impl DependentResolution {
    const fn valid() -> ResolutionStanding {
        ResolutionStanding::Valid
    }
}

/// §45's sweep: invalidating a Gate Definition disputes what rested on it.
///
/// Returns the ids that became disputed. §45 clause 4 — "historical gate runs
/// remain preserved" — is why this returns a set and mutates nothing: no evidence
/// is rewritten, and the standing is a new fact recorded alongside it.
#[must_use]
pub fn propagate_invalidation(
    invalidated_gate: &str,
    runs: &[GateRun],
    resolutions: &[DependentResolution],
) -> BTreeSet<String> {
    let affected_runs: BTreeSet<&str> = runs
        .iter()
        .filter(|r| r.gate == invalidated_gate)
        .map(|r| r.id.as_str())
        .collect();

    let mut disputed: BTreeSet<String> = resolutions
        .iter()
        .filter(|r| {
            r.rests_on_runs
                .iter()
                .any(|id| affected_runs.contains(id.as_str()))
        })
        .map(|r| r.id.clone())
        .collect();

    // Transitive closure. A resolution resting on a disputed resolution is itself
    // disputed; without this, invalidation stops one hop from where it started
    // and the second hop keeps claiming to be standing.
    //
    // Fixed-point iteration, O(n²) worst case on a long dependency chain. That
    // is fine at corpus scale and would not be at institutional scale; it is
    // noted rather than optimised because the shape of an institutional
    // resolution graph is Knowledge Fabric's to decide (OW-WAR-0028).
    loop {
        let mut added = false;
        for r in resolutions {
            if disputed.contains(&r.id) {
                continue;
            }
            if r.rests_on_resolutions.iter().any(|d| disputed.contains(d)) {
                disputed.insert(r.id.clone());
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    disputed
}

/// Whether a set of required runs permits a resolution (RQ-054).
///
/// Returns the blocking run ids. Empty means resolution may proceed.
#[must_use]
pub fn blocking_required_runs(required: &[GateRun]) -> Vec<String> {
    required
        .iter()
        .filter(|r| !r.satisfies_required_pass())
        .map(|r| r.id.clone())
        .collect()
}

/// A count of runs by execution status, for reporting.
#[must_use]
pub fn tally_by_status(runs: &[GateRun]) -> BTreeMap<ExecutionStatus, usize> {
    let mut out = BTreeMap::new();
    for r in runs {
        *out.entry(r.execution_status).or_insert(0) += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(a: Askability, e: ExecutionStatus, v: Verdict) -> GateRun {
        GateRun {
            id: "GR-001".into(),
            gate: "a.b@1.0.0".into(),
            askability: a,
            execution_status: e,
            verdict: v,
            reason_code: None,
        }
    }

    /// SAS §99 criterion 19, over the WHOLE space rather than three examples.
    ///
    /// 2 askabilities × 6 execution statuses × 3 verdicts = 36 triples. §44.5
    /// names exactly one that satisfies a required pass. If a later refactor makes
    /// a second triple pass, this fails and names it.
    #[test]
    fn every_triple_except_one_fails_a_required_pass() {
        let mut passing = Vec::new();
        for &a in Askability::ALL {
            for &e in ExecutionStatus::ALL {
                for &v in Verdict::ALL {
                    if run(a, e, v).satisfies_required_pass() {
                        passing.push((a, e, v));
                    }
                }
            }
        }
        assert_eq!(
            passing,
            vec![(
                Askability::Askable,
                ExecutionStatus::Completed,
                Verdict::Pass
            )],
            "§44.5 names exactly one satisfying triple; found {passing:?}"
        );
        assert_eq!(
            Askability::ALL.len() * ExecutionStatus::ALL.len() * Verdict::ALL.len(),
            36,
            "the space changed size; criterion 19 must be re-argued, not re-run"
        );
    }

    /// The headline: no unaskable run passes, whatever else it records.
    #[test]
    fn no_unaskable_run_can_satisfy_a_required_pass() {
        for &e in ExecutionStatus::ALL {
            for &v in Verdict::ALL {
                assert!(
                    !run(Askability::NotAskable, e, v).satisfies_required_pass(),
                    "not_askable + {e} + {v} satisfied a required pass"
                );
            }
        }
    }

    /// §44.4's four worked examples, transcribed.
    #[test]
    fn the_sas_worked_examples_validate() {
        // Target failed.
        let failed = run(
            Askability::Askable,
            ExecutionStatus::Completed,
            Verdict::Fail,
        );
        assert_eq!(failed.validate(), Ok(()));
        assert!(!failed.satisfies_required_pass());

        // Tool missing.
        let mut missing = run(
            Askability::NotAskable,
            ExecutionStatus::NotRun,
            Verdict::Unknown,
        );
        missing.reason_code = Some(ReasonCode::MissingTool);
        assert_eq!(missing.validate(), Ok(()));
        assert!(!missing.satisfies_required_pass());

        // Zero tests selected.
        let mut zero = run(
            Askability::NotAskable,
            ExecutionStatus::Invalid,
            Verdict::Unknown,
        );
        zero.reason_code = Some(ReasonCode::ZeroSelectedTests);
        assert_eq!(zero.validate(), Ok(()));

        // §44.5's required pass.
        let pass = run(
            Askability::Askable,
            ExecutionStatus::Completed,
            Verdict::Pass,
        );
        assert_eq!(pass.validate(), Ok(()));
        assert!(pass.satisfies_required_pass());
    }

    /// A fabricated result on an unaskable gate is refused at construction.
    #[test]
    fn an_unaskable_run_cannot_record_a_verdict() {
        for v in [Verdict::Pass, Verdict::Fail] {
            let r = run(Askability::NotAskable, ExecutionStatus::NotRun, v);
            assert!(
                matches!(r.validate(), Err(GateRunError::UnaskableWithResult { .. })),
                "not_askable + {v} was accepted"
            );
        }
    }

    #[test]
    fn an_unaskable_run_that_claims_to_have_executed_is_refused() {
        let mut r = run(
            Askability::NotAskable,
            ExecutionStatus::Completed,
            Verdict::Unknown,
        );
        r.reason_code = Some(ReasonCode::MissingTool);
        assert!(matches!(
            r.validate(),
            Err(GateRunError::UnaskableButExecuted { .. })
        ));
    }

    /// "Could not ask" with no reason is the state §96.4 forbids collapsing —
    /// and with no reason recorded there is nothing to stop the collapse.
    #[test]
    fn an_unaskable_run_without_a_reason_code_is_refused() {
        let r = run(
            Askability::NotAskable,
            ExecutionStatus::NotRun,
            Verdict::Unknown,
        );
        assert!(matches!(
            r.validate(),
            Err(GateRunError::UnaskableWithoutReason { .. })
        ));
    }

    #[test]
    fn a_completed_run_must_say_which_way_it_went() {
        let r = run(
            Askability::Askable,
            ExecutionStatus::Completed,
            Verdict::Unknown,
        );
        assert!(matches!(
            r.validate(),
            Err(GateRunError::CompletedWithoutVerdict { .. })
        ));
    }

    #[test]
    fn a_timed_out_run_cannot_claim_a_verdict() {
        let r = run(Askability::Askable, ExecutionStatus::Timeout, Verdict::Pass);
        assert!(matches!(
            r.validate(),
            Err(GateRunError::IncompleteWithVerdict { .. })
        ));
    }

    // ---- §96.4 migration -------------------------------------------------

    /// The ten classes, transcribed in the specification's order.
    #[test]
    fn the_migration_vocabulary_matches_the_sas() {
        assert_eq!(
            MIGRATION_CLASSES
                .iter()
                .map(|c| c.as_str())
                .collect::<Vec<_>>(),
            [
                "malformed",
                "foreign_working_directory",
                "missing_tool",
                "missing_script",
                "missing_crate",
                "mutating",
                "timeout",
                "failed",
                "passed",
                "not_run",
            ]
        );
    }

    /// §96.4: migration "SHALL not collapse 'could not ask' into 'failed'."
    ///
    /// Every could-not-ask class must migrate to `not_askable`, and none of them
    /// may land on `Verdict::Fail`. This is the sentence, as a test.
    #[test]
    fn could_not_ask_never_collapses_into_failed() {
        for &class in &MIGRATION_CLASSES {
            let (askability, _, verdict) = class.migration_target();
            if class.is_could_not_ask() {
                assert_eq!(
                    askability,
                    Askability::NotAskable,
                    "{class} is a could-not-ask class but migrates to {askability}"
                );
                assert_ne!(
                    verdict,
                    Verdict::Fail,
                    "{class} collapsed into a failure verdict"
                );
                assert_eq!(verdict, Verdict::Unknown, "{class}");
            }
        }
        // And the one class that IS a failure still migrates to one.
        assert_eq!(
            ReasonCode::Failed.migration_target(),
            (
                Askability::Askable,
                ExecutionStatus::Completed,
                Verdict::Fail
            )
        );
    }

    /// Every migrated run is coherent, and only `passed` satisfies a required
    /// pass. A migration that manufactured a passing gate would be the worst
    /// possible outcome of this Warrant.
    #[test]
    fn every_migrated_class_is_coherent_and_only_passed_passes() {
        for &class in &MIGRATION_CLASSES {
            let r = GateRun::from_migration_class("GR-M", "a.b@1.0.0", class);
            assert_eq!(
                r.validate(),
                Ok(()),
                "{class} migrated to an incoherent run"
            );
            assert_eq!(
                r.satisfies_required_pass(),
                class == ReasonCode::Passed,
                "{class} satisfied a required pass: {r:?}"
            );
        }
    }

    /// Ten migration classes plus `zero_selected_tests`, which §44.4 uses but
    /// §96.4 does not enumerate.
    #[test]
    fn zero_selected_tests_is_a_reason_code_but_not_a_migration_class() {
        assert!(!ReasonCode::ZeroSelectedTests.is_migration_class());
        assert!(ReasonCode::MissingTool.is_migration_class());
        assert_eq!(ReasonCode::ALL.len(), MIGRATION_CLASSES.len() + 1);
    }

    // ---- §44.6, §44.7, §44.8 ---------------------------------------------

    fn full_receipt() -> GateReceipt {
        GateReceipt {
            schema: GATE_RECEIPT_SCHEMA.into(),
            kind: GATE_RECEIPT_KIND.into(),
            run_id: "GR-001".into(),
            gate_run_digest: format!("sha256:{}", "f".repeat(64)),
            gate_definition_digest: "sha256:a".into(),
            gate_binding_digest: "sha256:b".into(),
            subject_digests: vec!["sha256:c".into()],
            fixture_digests: vec![],
            runner: "gate_runner".into(),
            producer_actor: "service://gate-runner".into(),
            runtime_environment: "linux-x86_64 rustc 1.97.1".into(),
            arguments: vec!["cargo".into(), "test".into()],
            working_directory: "/repo".into(),
            started_at: "2026-08-19T00:00:00Z".into(),
            completed_at: "2026-08-19T00:00:04Z".into(),
            exit_result: GateExitResult::ExitCode { code: 0 },
            selected_test_count: 1,
            selected_test_manifest: vec!["t1".into()],
            selection_observation_ref: "artifact://selection".into(),
            selection_observation_digest: format!("sha256:{}", "e".repeat(64)),
            raw_evidence_refs: vec![],
            stdout_ref: "artifact://out".into(),
            stdout_digest: format!("sha256:{}", "a".repeat(64)),
            stderr_ref: "artifact://err".into(),
            stderr_digest: format!("sha256:{}", "b".repeat(64)),
            resource_usage: "4s cpu".into(),
            verdict: Verdict::Pass,
            receipt_digest: "sha256:d".into(),
            extensions: GateReceiptExtensions::default(),
        }
    }

    #[test]
    fn a_complete_receipt_validates_and_each_missing_scalar_is_named() {
        assert_eq!(full_receipt().validate(), Ok(()));
        // Every required scalar, blanked one at a time.
        // Function pointers, not boxed closures: the list is a table of
        // (field name, how to blank it), and a fn pointer says so without a
        // trait object.
        type Blank = (&'static str, fn(&mut GateReceipt));
        let blanks: [Blank; 17] = [
            ("gate_run_digest", |r| r.gate_run_digest.clear()),
            ("gate_definition_digest", |r| {
                r.gate_definition_digest.clear()
            }),
            ("gate_binding_digest", |r| r.gate_binding_digest.clear()),
            ("runner", |r| r.runner.clear()),
            ("producer_actor", |r| r.producer_actor.clear()),
            ("runtime_environment", |r| r.runtime_environment.clear()),
            ("working_directory", |r| r.working_directory.clear()),
            ("started_at", |r| r.started_at.clear()),
            ("completed_at", |r| r.completed_at.clear()),
            ("selection_observation_ref", |r| {
                r.selection_observation_ref.clear()
            }),
            ("selection_observation_digest", |r| {
                r.selection_observation_digest.clear()
            }),
            ("stdout_ref", |r| r.stdout_ref.clear()),
            ("stdout_digest", |r| r.stdout_digest.clear()),
            ("stderr_ref", |r| r.stderr_ref.clear()),
            ("stderr_digest", |r| r.stderr_digest.clear()),
            ("resource_usage", |r| r.resource_usage.clear()),
            ("receipt_digest", |r| r.receipt_digest.clear()),
        ];
        for (name, blank) in blanks {
            let mut r = full_receipt();
            blank(&mut r);
            match r.validate() {
                Err(GateRunError::ReceiptIncomplete { field, .. }) => {
                    assert_eq!(field, name, "wrong field named");
                }
                other => panic!("blanking {name} was accepted: {other:?}"),
            }
        }
    }

    #[test]
    fn a_receipt_refuses_noncanonical_producer_identity() {
        let mut receipt = full_receipt();
        receipt.producer_actor.push(' ');
        assert!(
            receipt.validate().is_err(),
            "surrounding whitespace must not create a second producer identity"
        );
    }

    fn full_selection_observation() -> TestSelectionObservation {
        TestSelectionObservation {
            schema: TEST_SELECTION_OBSERVATION_SCHEMA.to_owned(),
            kind: TEST_SELECTION_OBSERVATION_KIND.to_owned(),
            run_id: "GR-001".to_owned(),
            gate_definition_digest: format!("sha256:{}", "a".repeat(64)),
            adapter: "adapter://openwarrant/war-check@1.0.0".to_owned(),
            selected_test_count: 1,
            selected_test_manifest: vec!["check://warrant-corpus".to_owned()],
        }
    }

    #[test]
    fn test_selection_observation_is_required_exact_and_nonempty() {
        assert_eq!(full_selection_observation().validate(), Ok(()));

        let mut duplicate = full_selection_observation();
        duplicate
            .selected_test_manifest
            .push("check://warrant-corpus".to_owned());
        duplicate.selected_test_count = 2;
        assert!(matches!(
            duplicate.validate(),
            Err(GateRunError::SelectionObservationManifestNotCanonical { .. })
        ));

        let mut zero = full_selection_observation();
        zero.selected_test_manifest.clear();
        zero.selected_test_count = 0;
        assert!(matches!(
            zero.validate(),
            Err(GateRunError::SelectionObservationManifestNotCanonical { .. })
        ));

        let mut miscounted = full_selection_observation();
        miscounted.selected_test_count = 2;
        assert!(matches!(
            miscounted.validate(),
            Err(GateRunError::SelectionObservationCountMismatch { .. })
        ));

        let mut unknown = serde_json::to_value(full_selection_observation())
            .expect("serialize selection observation");
        unknown
            .as_object_mut()
            .expect("selection observation object")
            .insert("caller_claim".to_owned(), serde_json::json!(true));
        assert!(
            serde_json::from_value::<TestSelectionObservation>(unknown).is_err(),
            "unknown observation semantics must fail closed"
        );
    }

    #[test]
    fn a_receipt_refuses_duplicate_or_blank_semantic_inventory_items() {
        let plants: [fn(&mut GateReceipt); 6] = [
            |receipt| {
                receipt
                    .subject_digests
                    .push(receipt.subject_digests[0].clone())
            },
            |receipt| receipt.subject_digests.push(" ".to_owned()),
            |receipt| receipt.raw_evidence_refs = vec!["artifact://DEL-1".to_owned(); 2],
            |receipt| receipt.raw_evidence_refs.push(String::new()),
            |receipt| receipt.selected_test_manifest.push("t1".to_owned()),
            |receipt| receipt.selected_test_manifest.push(" ".to_owned()),
        ];

        for plant in plants {
            let mut receipt = full_receipt();
            plant(&mut receipt);
            assert!(
                receipt.validate().is_err(),
                "noncanonical receipt inventory was accepted: {receipt:?}"
            );
        }
    }

    fn canonical_inventory(prefix: &str, len: usize) -> Vec<String> {
        (0..len)
            .map(|index| format!("{prefix}{index:04}"))
            .collect()
    }

    #[test]
    fn a_receipt_refuses_every_oversized_inventory_with_a_structured_error() {
        type Plant = (&'static str, usize, fn(&mut GateReceipt));
        let plants: [Plant; 4] = [
            (
                "subject_digests",
                GATE_RECEIPT_MAX_SUBJECT_DIGESTS,
                |receipt| {
                    receipt.subject_digests = canonical_inventory(
                        "sha256:subject-",
                        GATE_RECEIPT_MAX_SUBJECT_DIGESTS + 1,
                    );
                },
            ),
            (
                "fixture_digests",
                GATE_RECEIPT_MAX_FIXTURE_DIGESTS,
                |receipt| {
                    receipt.fixture_digests = canonical_inventory(
                        "sha256:fixture-",
                        GATE_RECEIPT_MAX_FIXTURE_DIGESTS + 1,
                    );
                },
            ),
            (
                "selected_test_manifest",
                GATE_RECEIPT_MAX_SELECTED_TESTS,
                |receipt| {
                    receipt.selected_test_manifest =
                        canonical_inventory("test://", GATE_RECEIPT_MAX_SELECTED_TESTS + 1);
                    receipt.selected_test_count =
                        u64::try_from(receipt.selected_test_manifest.len())
                            .expect("test inventory length fits u64");
                },
            ),
            (
                "raw_evidence_refs",
                GATE_RECEIPT_MAX_RAW_EVIDENCE_REFS,
                |receipt| {
                    receipt.raw_evidence_refs = canonical_inventory(
                        "artifact://raw/",
                        GATE_RECEIPT_MAX_RAW_EVIDENCE_REFS + 1,
                    );
                },
            ),
        ];

        for (field, limit, plant) in plants {
            let mut receipt = full_receipt();
            plant(&mut receipt);
            assert_eq!(
                receipt.validate(),
                Err(GateRunError::ReceiptInventoryTooLarge {
                    run: receipt.run_id.clone(),
                    field,
                    observed: limit + 1,
                    limit,
                }),
                "oversized {field} inventory was accepted"
            );
        }
    }

    #[test]
    fn a_receipt_accepts_every_inventory_at_its_named_limit() {
        let mut receipt = full_receipt();
        receipt.subject_digests =
            canonical_inventory("sha256:subject-", GATE_RECEIPT_MAX_SUBJECT_DIGESTS);
        receipt.fixture_digests =
            canonical_inventory("sha256:fixture-", GATE_RECEIPT_MAX_FIXTURE_DIGESTS);
        receipt.selected_test_manifest =
            canonical_inventory("test://", GATE_RECEIPT_MAX_SELECTED_TESTS);
        receipt.selected_test_count = u64::try_from(receipt.selected_test_manifest.len())
            .expect("test inventory length fits u64");
        receipt.raw_evidence_refs =
            canonical_inventory("artifact://raw/", GATE_RECEIPT_MAX_RAW_EVIDENCE_REFS);

        assert_eq!(receipt.validate(), Ok(()));
    }

    #[test]
    fn a_receipt_refuses_noncanonical_or_rollback_chronology() {
        for (started_at, completed_at) in [
            ("2026-08-19T00:00:04Z", "2026-08-19T00:00:03Z"),
            ("2026-08-19T00:00:00+00:00", "2026-08-19T00:00:04Z"),
            ("2026-08-19T00:00:00Z", "not-a-time"),
        ] {
            let mut receipt = full_receipt();
            receipt.started_at = started_at.to_owned();
            receipt.completed_at = completed_at.to_owned();
            assert!(
                receipt.validate().is_err(),
                "invalid receipt chronology was accepted: {started_at} -> {completed_at}"
            );
        }
    }

    #[test]
    fn a_receipt_carries_exact_versioned_wire_identity() {
        let receipt = full_receipt();
        assert_eq!(receipt.schema, GATE_RECEIPT_SCHEMA);
        assert_eq!(receipt.kind, GATE_RECEIPT_KIND);

        let mut value = serde_json::to_value(&receipt).expect("serialize receipt");
        value
            .as_object_mut()
            .expect("receipt object")
            .remove("schema");
        assert!(
            serde_json::from_value::<GateReceipt>(value).is_err(),
            "unversioned alpha receipt must not deserialize as v1 evidence"
        );
    }

    #[test]
    fn every_required_receipt_array_must_be_present_on_the_wire() {
        for field in [
            "subject_digests",
            "fixture_digests",
            "arguments",
            "selected_test_manifest",
            "raw_evidence_refs",
        ] {
            let mut value = serde_json::to_value(full_receipt()).expect("serialize receipt");
            value.as_object_mut().expect("receipt object").remove(field);
            assert!(
                serde_json::from_value::<GateReceipt>(value).is_err(),
                "omitted required array {field:?} deserialized as an explicit empty inventory"
            );
        }
    }

    #[test]
    fn receipt_exit_result_is_typed_and_must_agree_with_verdict() {
        let receipt = full_receipt();
        let value = serde_json::to_value(&receipt).expect("serialize receipt");
        assert_eq!(value["exit_result"]["kind"], "exit_code");
        assert_eq!(value["exit_result"]["code"], 0);

        let mut nonzero_pass = receipt.clone();
        nonzero_pass.exit_result = GateExitResult::ExitCode { code: 7 };
        assert!(matches!(
            nonzero_pass.validate(),
            Err(GateRunError::ReceiptExitVerdictMismatch { .. })
        ));

        let mut signal_pass = receipt.clone();
        signal_pass.exit_result = GateExitResult::Signal { signal: 9 };
        assert!(matches!(
            signal_pass.validate(),
            Err(GateRunError::ReceiptExitVerdictMismatch { .. })
        ));

        let mut nonzero_unknown = receipt.clone();
        nonzero_unknown.exit_result = GateExitResult::ExitCode { code: 7 };
        nonzero_unknown.verdict = Verdict::Unknown;
        assert!(matches!(
            nonzero_unknown.validate(),
            Err(GateRunError::ReceiptExitVerdictMismatch { .. })
        ));

        let mut signal_unknown = receipt.clone();
        signal_unknown.exit_result = GateExitResult::Signal { signal: 9 };
        signal_unknown.verdict = Verdict::Unknown;
        assert!(matches!(
            signal_unknown.validate(),
            Err(GateRunError::ReceiptExitVerdictMismatch { .. })
        ));

        let mut zero_fail = receipt;
        zero_fail.verdict = Verdict::Fail;
        assert!(matches!(
            zero_fail.validate(),
            Err(GateRunError::ReceiptExitVerdictMismatch { .. })
        ));
    }

    #[test]
    fn receipt_preserves_namespaced_extensions_and_refuses_other_unknown_fields() {
        let mut value = serde_json::to_value(full_receipt()).expect("serialize receipt");
        value.as_object_mut().expect("receipt object").insert(
            "lab.trace".to_owned(),
            serde_json::json!({"attempt": "second"}),
        );

        let parsed: GateReceipt =
            serde_json::from_value(value).expect("namespaced extension is optional");
        let round_trip = serde_json::to_value(parsed).expect("serialize extended receipt");
        assert_eq!(
            round_trip["lab.trace"],
            serde_json::json!({"attempt": "second"})
        );

        let mut unknown = serde_json::to_value(full_receipt()).expect("serialize receipt");
        unknown
            .as_object_mut()
            .expect("receipt object")
            .insert("unexpected".to_owned(), serde_json::json!(true));
        assert!(
            serde_json::from_value::<GateReceipt>(unknown).is_err(),
            "unknown non-namespaced fields must fail closed"
        );
    }

    /// §44.7 — a raw shell string needs a gate that owns shell parsing.
    #[test]
    fn a_raw_shell_string_needs_a_gate_that_owns_shell_parsing() {
        let shell = Invocation::ShellString("rm -rf / && echo ok".into());
        assert!(matches!(
            shell.validate("a.b", false),
            Err(GateRunError::UnownedShellString { .. })
        ));
        assert_eq!(shell.validate("shell.owner", true), Ok(()));

        let vector = Invocation::ArgVector(vec!["cargo".into(), "test".into()]);
        assert_eq!(vector.validate("a.b", false), Ok(()));
    }

    /// §44.8 — a mutating gate never runs in a routine check, however complete
    /// its declaration. "It cannot run merely because an old document contains a
    /// command string."
    #[test]
    fn a_mutating_gate_is_quarantined_even_when_fully_declared() {
        let complete = MutationDeclaration {
            effects: vec!["writes to the database".into()],
            authority: "operator".into(),
            compensation: "restore from snapshot".into(),
        };
        assert!(complete.is_complete());
        assert!(matches!(
            MutationDeclaration::quarantine("a.b@1.0.0"),
            Err(GateRunError::MutatingGateQuarantined { .. })
        ));

        let partial = MutationDeclaration {
            effects: vec!["writes".into()],
            ..MutationDeclaration::default()
        };
        assert!(!partial.is_complete());
    }

    // ---- RQ-054 and §45 ---------------------------------------------------

    /// RQ-054 — a required unknown blocks resolution.
    #[test]
    fn a_required_unknown_blocks_resolution() {
        let mut unknown = run(
            Askability::NotAskable,
            ExecutionStatus::NotRun,
            Verdict::Unknown,
        );
        unknown.id = "GR-UNKNOWN".into();
        unknown.reason_code = Some(ReasonCode::MissingTool);
        let pass = run(
            Askability::Askable,
            ExecutionStatus::Completed,
            Verdict::Pass,
        );

        assert_eq!(
            blocking_required_runs(std::slice::from_ref(&pass)),
            Vec::<String>::new()
        );
        assert_eq!(
            blocking_required_runs(&[pass, unknown]),
            vec!["GR-UNKNOWN".to_owned()],
            "an unaskable required gate must block, not be skipped"
        );
    }

    fn resolution(id: &str, runs: &[&str], on: &[&str]) -> DependentResolution {
        DependentResolution {
            id: id.into(),
            rests_on_runs: runs.iter().map(|s| (*s).to_owned()).collect(),
            rests_on_resolutions: on.iter().map(|s| (*s).to_owned()).collect(),
            standing: ResolutionStanding::Valid,
        }
    }

    /// §45 — invalidation propagates, transitively.
    #[test]
    fn invalidation_propagates_transitively_and_stops_at_the_unrelated() {
        let runs = vec![
            GateRun {
                id: "GR-1".into(),
                gate: "a.b@1.0.0".into(),
                ..run(
                    Askability::Askable,
                    ExecutionStatus::Completed,
                    Verdict::Pass,
                )
            },
            GateRun {
                id: "GR-2".into(),
                gate: "other@1.0.0".into(),
                ..run(
                    Askability::Askable,
                    ExecutionStatus::Completed,
                    Verdict::Pass,
                )
            },
        ];
        let resolutions = vec![
            resolution("R-1", &["GR-1"], &[]),
            resolution("R-2", &[], &["R-1"]),
            resolution("R-3", &[], &["R-2"]),
            resolution("R-UNRELATED", &["GR-2"], &[]),
        ];

        let disputed = propagate_invalidation("a.b@1.0.0", &runs, &resolutions);
        assert!(disputed.contains("R-1"), "direct dependent not disputed");
        assert!(disputed.contains("R-2"), "one hop not disputed");
        assert!(
            disputed.contains("R-3"),
            "two hops not disputed — propagation stopped one hop short"
        );
        assert!(
            !disputed.contains("R-UNRELATED"),
            "invalidation spread to a resolution that did not rest on the gate"
        );
        assert_eq!(disputed.len(), 3);
    }

    /// §45 clause 4: "historical gate runs remain preserved." The sweep reports
    /// and rewrites nothing.
    #[test]
    fn invalidation_rewrites_no_history() {
        let runs = vec![GateRun {
            id: "GR-1".into(),
            gate: "a.b@1.0.0".into(),
            ..run(
                Askability::Askable,
                ExecutionStatus::Completed,
                Verdict::Pass,
            )
        }];
        let before = runs.clone();
        let resolutions = vec![resolution("R-1", &["GR-1"], &[])];
        let before_res = resolutions.clone();

        let _ = propagate_invalidation("a.b@1.0.0", &runs, &resolutions);

        assert_eq!(runs, before, "a gate run was modified");
        assert_eq!(
            resolutions, before_res,
            "a resolution was modified in place"
        );
    }

    #[test]
    fn a_cycle_between_resolutions_terminates() {
        let runs = vec![GateRun {
            id: "GR-1".into(),
            gate: "a.b@1.0.0".into(),
            ..run(
                Askability::Askable,
                ExecutionStatus::Completed,
                Verdict::Pass,
            )
        }];
        let resolutions = vec![
            resolution("R-1", &["GR-1"], &["R-2"]),
            resolution("R-2", &[], &["R-1"]),
        ];
        let disputed = propagate_invalidation("a.b@1.0.0", &runs, &resolutions);
        assert_eq!(disputed.len(), 2);
    }

    #[test]
    fn vocabularies_round_trip_through_their_strings() {
        for &v in Askability::ALL {
            assert_eq!(Askability::from_str(v.as_str()), Ok(v));
        }
        for &v in ExecutionStatus::ALL {
            assert_eq!(ExecutionStatus::from_str(v.as_str()), Ok(v));
        }
        for &v in Verdict::ALL {
            assert_eq!(Verdict::from_str(v.as_str()), Ok(v));
        }
        for &v in ReasonCode::ALL {
            assert_eq!(ReasonCode::from_str(v.as_str()), Ok(v));
        }
        assert!(ExecutionStatus::from_str("failed").is_err());
    }

    /// §44.2's six, transcribed. `failed` and `passed` are NOT execution
    /// statuses — they are verdicts and migration classes, and conflating them is
    /// what OW-ADR-0006 exists to prevent recurring.
    #[test]
    fn the_execution_statuses_match_the_sas() {
        assert_eq!(
            ExecutionStatus::ALL
                .iter()
                .map(|e| e.as_str())
                .collect::<Vec<_>>(),
            [
                "not_run",
                "completed",
                "timeout",
                "infrastructure_error",
                "cancelled",
                "invalid",
            ]
        );
    }

    #[test]
    fn a_run_round_trips_through_json() {
        let mut r = run(
            Askability::NotAskable,
            ExecutionStatus::NotRun,
            Verdict::Unknown,
        );
        r.reason_code = Some(ReasonCode::MissingCrate);
        let s = serde_json::to_string(&r).expect("serialize");
        assert_eq!(serde_json::from_str::<GateRun>(&s).expect("deserialize"), r);
    }

    #[test]
    fn a_tally_counts_by_execution_status() {
        let runs = vec![
            run(
                Askability::Askable,
                ExecutionStatus::Completed,
                Verdict::Pass,
            ),
            run(
                Askability::Askable,
                ExecutionStatus::Completed,
                Verdict::Fail,
            ),
            run(
                Askability::Askable,
                ExecutionStatus::Timeout,
                Verdict::Unknown,
            ),
        ];
        let t = tally_by_status(&runs);
        assert_eq!(t[&ExecutionStatus::Completed], 2);
        assert_eq!(t[&ExecutionStatus::Timeout], 1);
    }

    /// LamQuant's nine `adr_exec` labels, verbatim from `tools/adr_exec.py`.
    const LEGACY_LABELS: [&str; 9] = [
        "PASS",
        "FAIL",
        "TIMEOUT",
        "MALFORMED",
        "MUTATING",
        "FOREIGN-CWD",
        "MISSING-TOOL",
        "MISSING-SCRIPT",
        "MISSING-CRATE",
    ];

    /// Every legacy label resolves, and every one lands inside §96.4's ten.
    #[test]
    fn every_legacy_label_maps_into_a_migration_class() {
        for label in LEGACY_LABELS {
            let class = ReasonCode::from_legacy_execution_label(label)
                .unwrap_or_else(|| panic!("{label} does not map"));
            assert!(
                class.is_migration_class(),
                "{label} mapped to {class}, which §96.4 does not preserve"
            );
        }
    }

    /// The nine labels plus `not_run` cover §96.4's ten EXACTLY — no class left
    /// unreachable, none reachable twice. Folded to a canonical set rather than
    /// re-listing the ten, so the test cannot drift from MIGRATION_CLASSES.
    #[test]
    fn the_labels_plus_not_run_cover_the_ten_classes_exactly() {
        let mut mapped: Vec<ReasonCode> = LEGACY_LABELS
            .iter()
            .filter_map(|l| ReasonCode::from_legacy_execution_label(l))
            .collect();
        assert_eq!(mapped.len(), 9, "a label mapped to the same class twice");
        mapped.push(ReasonCode::NotRun);

        let mut expected = MIGRATION_CLASSES.to_vec();
        mapped.sort_by_key(|c| c.to_string());
        expected.sort_by_key(|c| c.to_string());
        assert_eq!(mapped, expected);
    }

    /// `not_run` is reachable from NO label — it comes from an absent result.
    #[test]
    fn not_run_has_no_legacy_label() {
        for label in LEGACY_LABELS {
            assert_ne!(
                ReasonCode::from_legacy_execution_label(label),
                Some(ReasonCode::NotRun),
                "{label} must not manufacture not_run"
            );
        }
        assert_eq!(ReasonCode::from_legacy_execution_label("NOT-RUN"), None);
    }

    /// An unrecognised label is reported, never guessed — and in particular
    /// never folded into `failed`, which is §96.4's named prohibition.
    #[test]
    fn an_unknown_label_is_not_quietly_failed() {
        for label in ["", "pass", "SKIP", "MISSING-BINARY", "OK"] {
            assert_eq!(
                ReasonCode::from_legacy_execution_label(label),
                None,
                "{label:?} was given a meaning it does not have"
            );
        }
    }

    /// §96.4: "could not ask" must not collapse into "failed". Asserted for the
    /// LamQuant vocabulary specifically, since that is the corpus being read.
    #[test]
    fn could_not_ask_labels_do_not_migrate_to_failed() {
        for label in [
            "MALFORMED",
            "MUTATING",
            "FOREIGN-CWD",
            "MISSING-TOOL",
            "MISSING-SCRIPT",
            "MISSING-CRATE",
        ] {
            let class = ReasonCode::from_legacy_execution_label(label).expect("maps");
            assert!(class.is_could_not_ask(), "{label} is a could-not-ask class");
            assert_ne!(class, ReasonCode::Failed, "{label} collapsed into failed");
        }
        // ...and the two that WERE asked and answered stay on the other side.
        for label in ["PASS", "FAIL"] {
            let class = ReasonCode::from_legacy_execution_label(label).expect("maps");
            assert!(!class.is_could_not_ask(), "{label} was asked and answered");
        }
    }
}
