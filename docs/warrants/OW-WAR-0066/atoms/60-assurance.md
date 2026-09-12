---
schema: oh.war/atom/v1
warrant_uuid: 01a0935b-6870-73d1-bbbb-ca87591adbd4
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a service stage runs, receipts, and submits only what a run may request
- **scope:** STAGE-001 under `war run`.
- **gate:** `gate://ops.echo@1.0.0`
- **evidence:** a gate run and a §44.6 receipt whose subject is `dispatch:<digest>`; a submission requesting `verify`; a `submission.recorded` journal event.

### OBL-002 — the battery passes as a stage
- **scope:** STAGE-002 under `war run`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** the battery's receipt with verdict pass, bound to the stage's dispatch digest.

## Gate Adequacy

Required at `basic`.

**Adversarial question:** can a run submit its own completion? No: a
submission's requested action is validated by name and `resolve` is refused
before anything is written; `war submit` refuses the same way for an
external submission, and one naming a dispatch this Warrant never compiled.

- **outcome:** no_counterexample

## Residual Risk

- Wall time is the only bound; a run that fills the disk is not stopped.
