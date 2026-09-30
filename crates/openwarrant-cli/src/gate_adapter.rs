// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact adapters allowed to mint recorded Gate evidence.
//!
//! A Gate Definition states planned selection. Recording needs stronger proof:
//! runner-owned code must know the exact invocation and emit what it selected
//! after that process completed. Unknown output schemas and invocations remain
//! runnable as probes, but cannot mint evidence.

use openwarrant_core::gate::GateDefinition;
use openwarrant_core::{
    ExecutionStatus, GateRun, TEST_SELECTION_OBSERVATION_KIND, TEST_SELECTION_OBSERVATION_SCHEMA,
    TestSelectionObservation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GateRecordingAdapter {
    WarCheckV1,
    BonsaiEvidenceV1,
    #[cfg(test)]
    FixtureDirectV1,
    #[cfg(test)]
    FixtureRustcVersionV1,
    #[cfg(test)]
    FixtureBonsaiStdinV1,
    #[cfg(test)]
    FixtureRequiredPassV1,
    #[cfg(test)]
    FixtureLegacyPassV1,
    #[cfg(test)]
    FixtureBindingAuthorityPreflightV1,
    #[cfg(test)]
    FixtureDecorativeEvidencePreflightV1,
}

impl GateRecordingAdapter {
    pub(crate) fn for_definition(definition: &GateDefinition) -> Result<Self, String> {
        let adapter = if exact_definition(
            definition,
            "software.repo.war-check@1.0.0",
            "artifact://openwarrant-cli/war-check",
            "schema://war-check-report/v1",
            &["./target/debug/war", "check", "--generated"],
            &["check://warrant-corpus"],
        ) {
            Self::WarCheckV1
        } else if exact_definition(
            definition,
            "software.repo.bonsai-evidence@1.0.0",
            "artifact://openwarrant-cli/war-bonsai-verify-evidence",
            "schema://bonsai-evidence/v1",
            &[
                "./target/debug/war",
                "bonsai",
                "verify-evidence",
                "--evidence",
                "-",
            ],
            &["check://bonsai-evidence-document"],
        ) {
            Self::BonsaiEvidenceV1
        } else {
            #[cfg(test)]
            {
                if exact_definition(
                    definition,
                    "fixture.pass@1.0.0",
                    "fixture",
                    "fixture",
                    &["fixture"],
                    &["test://fixture-pass"],
                ) {
                    return Ok(Self::FixtureDirectV1);
                }
                if exact_definition(
                    definition,
                    "fixture.pass@1.0.0",
                    "fixture://rustc-version",
                    "schema://fixture/v1",
                    &["rustc", "--version"],
                    &["test://rustc-version"],
                ) {
                    return Ok(Self::FixtureRustcVersionV1);
                }
                if exact_definition(
                    definition,
                    "software.repo.bonsai-evidence@1.0.0",
                    "fixture://bonsai-stdin-adapter",
                    "schema://bonsai-evidence/v1",
                    &[
                        "/bin/sh",
                        "-c",
                        "cat >/dev/null",
                        "bonsai-fixture",
                        "--evidence",
                        "-",
                    ],
                    &["check://bonsai-evidence-document"],
                ) {
                    return Ok(Self::FixtureBonsaiStdinV1);
                }
                if exact_definition(
                    definition,
                    "fixture.pass@1.0.0",
                    "artifact://fixture-runner",
                    "schema://fixture-gate/v1",
                    &["fixture"],
                    &["test://fixture-pass"],
                ) {
                    return Ok(Self::FixtureRequiredPassV1);
                }
                if exact_definition(
                    definition,
                    "fixture.legacy-pass@1.0.0",
                    "artifact://fixture-runner",
                    "schema://fixture-gate/v1",
                    &["fixture-pass"],
                    &["check://fixture-pass"],
                ) {
                    return Ok(Self::FixtureLegacyPassV1);
                }
                if exact_definition(
                    definition,
                    "fixture.pass@1.0.0",
                    "fixture://binding-authority-preflight",
                    "schema://fixture/v1",
                    &["/bin/sh", "-c", "printf ran > execution-marker"],
                    &["test://binding-authority-preflight"],
                ) {
                    return Ok(Self::FixtureBindingAuthorityPreflightV1);
                }
                if exact_definition(
                    definition,
                    "fixture.pass@1.0.0",
                    "fixture://decorative-evidence-preflight",
                    "schema://fixture/v1",
                    &["/bin/sh", "-c", "printf ran > execution-marker"],
                    &["test://decorative-evidence-preflight"],
                ) {
                    return Ok(Self::FixtureDecorativeEvidencePreflightV1);
                }
            }
            return Err(format!(
                "Gate {} has no exact execution adapter that can observe selected tests; generic definitions may run but cannot record evidence",
                definition.key()
            ));
        };
        Ok(adapter)
    }

    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::WarCheckV1 => "adapter://openwarrant/war-check@1.0.0",
            Self::BonsaiEvidenceV1 => "adapter://openwarrant/bonsai-evidence@1.0.0",
            #[cfg(test)]
            Self::FixtureDirectV1 => "adapter://openwarrant/fixture-direct@1.0.0",
            #[cfg(test)]
            Self::FixtureRustcVersionV1 => "adapter://openwarrant/fixture-rustc-version@1.0.0",
            #[cfg(test)]
            Self::FixtureBonsaiStdinV1 => "adapter://openwarrant/fixture-bonsai-stdin@1.0.0",
            #[cfg(test)]
            Self::FixtureRequiredPassV1 => "adapter://openwarrant/fixture-required-pass@1.0.0",
            #[cfg(test)]
            Self::FixtureLegacyPassV1 => "adapter://openwarrant/fixture-legacy-pass@1.0.0",
            #[cfg(test)]
            Self::FixtureBindingAuthorityPreflightV1 => {
                "adapter://openwarrant/fixture-binding-authority-preflight@1.0.0"
            }
            #[cfg(test)]
            Self::FixtureDecorativeEvidencePreflightV1 => {
                "adapter://openwarrant/fixture-decorative-evidence-preflight@1.0.0"
            }
        }
    }

    pub(crate) fn validate_raw_evidence(self, references: &[String]) -> Result<(), String> {
        match self {
            Self::BonsaiEvidenceV1 => {
                if references.len() == 1 && references[0].starts_with("file:") {
                    Ok(())
                } else {
                    Err(
                        "Bonsai execution adapter requires exactly one content-bound file evidence reference"
                            .to_owned(),
                    )
                }
            }
            #[cfg(test)]
            Self::FixtureBonsaiStdinV1 => {
                if references.len() == 1 && references[0].starts_with("file:") {
                    Ok(())
                } else {
                    Err(
                        "Bonsai execution adapter requires exactly one content-bound file evidence reference"
                            .to_owned(),
                    )
                }
            }
            #[cfg(test)]
            Self::FixtureRequiredPassV1 => {
                if references.len() == 1 && references[0].starts_with("artifact://") {
                    Ok(())
                } else {
                    Err(
                        "fixture required-pass adapter requires exactly one content-bound artifact evidence reference"
                            .to_owned(),
                    )
                }
            }
            _ if references.is_empty() => Ok(()),
            _ => Err(format!(
                "execution adapter {} cannot prove raw evidence consumption",
                self.id()
            )),
        }
    }

    fn selected_tests(self) -> &'static [&'static str] {
        match self {
            Self::WarCheckV1 => &["check://warrant-corpus"],
            Self::BonsaiEvidenceV1 => &["check://bonsai-evidence-document"],
            #[cfg(test)]
            Self::FixtureDirectV1 => &["test://fixture-pass"],
            #[cfg(test)]
            Self::FixtureRustcVersionV1 => &["test://rustc-version"],
            #[cfg(test)]
            Self::FixtureBonsaiStdinV1 => &["check://bonsai-evidence-document"],
            #[cfg(test)]
            Self::FixtureRequiredPassV1 => &["test://fixture-pass"],
            #[cfg(test)]
            Self::FixtureLegacyPassV1 => &["check://fixture-pass"],
            #[cfg(test)]
            Self::FixtureBindingAuthorityPreflightV1 => &["test://binding-authority-preflight"],
            #[cfg(test)]
            Self::FixtureDecorativeEvidencePreflightV1 => &["test://decorative-evidence-preflight"],
        }
    }

    pub(crate) fn observe(
        self,
        definition: &GateDefinition,
        run: &GateRun,
    ) -> Result<TestSelectionObservation, String> {
        if run.execution_status != ExecutionStatus::Completed {
            return Err(format!(
                "execution adapter {} cannot observe selection for incomplete run {}",
                self.id(),
                run.id
            ));
        }
        // Re-admit after execution. Callers cannot swap a definition between
        // preflight and minting and retain this adapter's authority.
        if Self::for_definition(definition)? != self {
            return Err(format!(
                "execution adapter {} no longer matches Gate Definition {}",
                self.id(),
                definition.key()
            ));
        }
        let selected_test_manifest: Vec<String> = self
            .selected_tests()
            .iter()
            .map(|test| (*test).to_owned())
            .collect();
        let observation = TestSelectionObservation {
            schema: TEST_SELECTION_OBSERVATION_SCHEMA.to_owned(),
            kind: TEST_SELECTION_OBSERVATION_KIND.to_owned(),
            run_id: run.id.clone(),
            gate_definition_digest: definition.digest.clone(),
            adapter: self.id().to_owned(),
            selected_test_count: u64::try_from(selected_test_manifest.len())
                .map_err(|_| "selected-test observation count does not fit u64".to_owned())?,
            selected_test_manifest,
        };
        observation
            .validate()
            .map_err(|error| format!("execution adapter produced invalid selection: {error}"))?;
        Ok(observation)
    }
}

fn exact_definition(
    definition: &GateDefinition,
    key: &str,
    implementation_ref: &str,
    output_schema_ref: &str,
    argv: &[&str],
    selected_tests: &[&str],
) -> bool {
    definition.key() == key
        && definition.implementation_ref == implementation_ref
        && definition.output_schema_ref == output_schema_ref
        && exact_strings(&definition.argv, argv)
        && exact_strings(&definition.selection_manifest, selected_tests)
}

fn exact_strings(actual: &[String], expected: &[&str]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(actual, expected)| actual == expected)
}
