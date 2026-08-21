---
schema: oh.war/atom/v1
warrant_uuid: 01a021a6-0dfd-7e45-b33b-749b95409cd1
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. One compatible WAR stage graph lowered into BLUT PlanSpec.
2. A real BLUT execution, with status, artifact and lineage receipts.
3. A plant for §91.7 test 47 — an unsupported lowering fails rather than
   degrades — against the shipped binary.
4. Evidence that no BLUT lineage was copied into the Warrant.

## Frozen Surfaces

§49.2's adapter duties and the lineage-reference shape.

## Premade Instructions

- Attempt an INCOMPATIBLE lowering first and record the refusal. A
  successful lowering proves the happy path; the refusal proves the adapter is a
  control rather than a translator.
- Grep the resulting Warrant for lineage content. Finding any is the defect.

## Autonomy and Escalation

Tier T1 — the adapter's refusals decide completion.

## Rollback

Revert the adapter invocation. Stage graphs remain lowerable and unlowered.
