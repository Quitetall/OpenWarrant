---
schema: oh.war/atom/v1
warrant_uuid: 01a021a6-0dfd-7e45-b33b-749b95409cd1
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

§49, §91.7 test 47.

## Prerequisites

OW-WAR-0027 resolved.

## Assumptions and Unknowns

- **Evidenced premise.** BLUT is checked out at
  `/mnt/4tb/LamQuant/training/engine`, and its `PlanSpec` schema is readable at a
  pinned commit. The lowering is therefore written against a real schema rather
  than an invented one.
- **Resolved unknown (2026-08-21).** This Basis previously said "BLUT ships no
  verb that DESERIALIZES a `PlanSpec` JSON". That was wrong when written —
  `blut plan publish` has always parsed one and typechecked it fail-closed. The
  real gap was narrower: no verb did it *without writing a deployment row*.
  Stating the wide version made a missing feature look like a missing
  capability, which is how a solvable blocker comes to read as a permanent one.

  The narrow gap is closed. `blut plan check` landed on BLUT `main`
  (`7b60d21e`, refined `d6822563`), and `war blut --verify <binary>` now hands
  the lowered spec to a real BLUT binary and reports its verdict.

  This is the second correction to this Basis. The first retracted "actionable
  today without new infrastructure", which ran two claims together.
- **Resolved unknown (2026-08-21, later the same day).** This said "no stage
  this repository names is compiled into any BLUT cookbook, so a real binary
  REFUSES every lowering here". The premise was true and the conclusion was
  wrong, because a WAR stage id was never required to BE the BLUT stage name.
  The adapter used the id for want of anything else, so every lowering named
  `STAGE-NNN` — and the refusal was read as the pinned-registry rule working
  when it was the adapter guessing. With `executor_ref`, BLUT ACCEPTED a
  two-stage lowering (fingerprint `a2005e3c9535…`, exit 0).

  Recorded because the mistake is instructive: a correct refusal, from a real
  external tool, for a real reason, still supported the wrong conclusion about
  what was possible.
- **Blocking unknown.** Acceptance is not execution. BLUT typechecked the plan;
  nothing ran it, so there are no status, artifact or lineage receipts.
  *Resolution requirement:* a BLUT run of this accepted plan, which needs a real
  corpus for `materialize_dataset_path` and a decision about whether this
  repository should be launching training jobs to satisfy its own Warrant.
  Until then OBL-001 has no receipts to point at.
- **Blocking unknown.** OBL-003 asks the Warrant to carry a `lineage_ref`, and
  there is no lineage to reference because nothing has run — unchanged by the
  acceptance above, since a typecheck produces no lineage. The *prohibition*
  half is discharged — `lineage.reproduced` refuses a Warrant that restates
  BLUT's lineage, and it is planted in both directions — but a reference to a
  job that never existed would be a fabricated identifier, which is the
  substitution §40.7 forbids. *Resolution requirement:* the same execution
  OBL-001 needs. The two unblock together or not at all.
- **Accepted residual risk.** BLUT's PlanSpec may drift. The adapter pins a
  registry digest, so drift surfaces as a refusal rather than a silent remap.
  *Consequence if false:* a stage name resolves to different work on different
  days, which is the failure the pinned registry exists to prevent.

## Constraints and Invariants

- **Reject, do not degrade** (§49.2). An incompatible port kind is
  refused, not coerced into something that runs and means something else.
- **BLUT's lineage stays in BLUT** (§49.3). We store a reference.
- **The registry is pinned.**
