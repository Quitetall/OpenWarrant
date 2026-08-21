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
- **Blocking unknown.** BLUT ships no verb that DESERIALIZES a `PlanSpec` JSON.
  Its binaries are `blut-dsl` (Starlark to PlanSpec, the authoring direction),
  the operator, TUI, notify, metrics and web. So nothing OpenWarrant emits has
  been through BLUT's parser, and "BLUT accepted this" cannot be claimed.
  *Resolution requirement:* a BLUT-side verb that reads a `PlanSpec` JSON and
  reports acceptance, plus a stage name compiled into a registered cookbook —
  `PlanSpecError::UnknownStage` refuses anything else, and this repository's
  Warrants name stages like `STAGE-002` that no cookbook has.

  This corrects what this Basis said when authored: "actionable today without
  new infrastructure". Structurally faithful lowering is actionable; acceptance
  by BLUT is not, and the two were run together.
- **Accepted residual risk.** BLUT's PlanSpec may drift. The adapter pins a
  registry digest, so drift surfaces as a refusal rather than a silent remap.
  *Consequence if false:* a stage name resolves to different work on different
  days, which is the failure the pinned registry exists to prevent.

## Constraints and Invariants

- **Reject, do not degrade** (§49.2). An incompatible port kind is
  refused, not coerced into something that runs and means something else.
- **BLUT's lineage stays in BLUT** (§49.3). We store a reference.
- **The registry is pinned.**
