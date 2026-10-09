# Baseline source provenance and publication

Unverified OW-WAR-0041 continuation. No historical baseline, signed atom or
judgment is changed. This audit is retrospective; it is not a before-tuning
measurement, an independent verdict or a success-metric improvement claim.

## Retained historical observation

The original baseline was first committed at
`f3ac4b163a4aeea56eccbfb369be4eb6d2fc3627`. Its declared source commit is
`b54310897a72320b475e1da8d7d20f26972cce45`. The exact original bytes are retained
here, with their raw-byte identity in `comparison.json`.

A current `--derived` observation reads that exact retained parent tree. Six
original measured entries match: adequacy counterexamples (51), amendments (2),
evidence/gate citations (129), gate definitions (1), and the two lexical-history
candidate counts (46 each). The original zero for auto-authorizable fraction is
not established by the collector: it did not execute per-Warrant eligibility.
The current collector leaves it unmeasured. Changed UNKNOWN explanations are
collector-method differences, not newly observed events.

The existence of a Git-retained artifact proves retention. It does not establish
its collection method or the intended before-tuning cutoff. The old CLI read live
working sources and accepted `--commit` as a label. Therefore neither that field
nor this retrospective comparison independently proves OBL-001.

## Implementation

Both v1 and v2 publication paths now use the existing atomic no-overwrite
publisher. Identical byte replay succeeds. Different existing bytes, symlinks and
nonregular destinations refuse instead of rewriting evidence. The baseline's
schema, canonical serialized bytes and retained original records are unchanged.

CLI record/verify reports carry `source_basis`: live working tree with an
unresolved source commit for legacy v1; an exact resolved retained commit for v2.
Byte equality does not establish before-tuning timing or qualification. Human
output and help no longer describe the legacy commit label as a source pin.

## Scope still open

The SDK already defines eight exact derived calculations. The frozen CLI observes
four populations: amendments/Warrant, lexical untracked work, recorded adequacy
catch rate, and gate citations/definition. Human control time, amendment safety,
complete repair follow-up and resolved-cohort escape windows still need correlated
recorded observations. Git time, a correction's mere existence or a guessed cohort
cannot supply those facts. Independent review and any historical reconciliation
remain separate; no completion or assurance disposition is asserted.

## Controls

Rust 1.97.1: focused telemetry integrity tests and public CLI controls pass,
including identical replay, changed-byte/link refusal, truthful source metadata,
exact frozen history and missing-commit UNKNOWN. The first test run failed an old
human-output assertion that incorrectly called a legacy label a source pin; the
corrected expectation checks truthful byte-equality and live-source wording.
Initial and corrected outputs are retained. Full repository gate remains separate.

Final all-target CLI Clippy passes with warnings denied.

Complete CLI library unit suite: **348 tests passed, zero failures** on this source branch.

Selected battery wrapper: **2 controls passed, zero failures**, including the positive generated corpus and the public telemetry control family.
