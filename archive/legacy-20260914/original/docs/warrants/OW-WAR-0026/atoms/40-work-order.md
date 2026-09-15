---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd7-eb6b-7e8f-88ff-6849c6e49e2f
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. Dispatch → Katana request mapping.
2. Runtime receipt consumption and validation.
3. Capability declaration passed through, defaulting to denied.
4. Taint propagation per §48.5.
5. An explicit 'unverified against a live Katana' marker until beta.

## Frozen Surfaces

The receipt-consumption boundary. Anything OpenWarrant computes about a run is an inference, not a receipt.

## Premade Instructions

- Build against the SAS description and MARK IT UNVERIFIED. A seam tested only
  against a mock is a seam tested against our own assumptions.
- Never construct a PromptIR, even for testing.

## Autonomy and Escalation

Tier T1 — the boundary is an ownership rule.

## Rollback

Revert. The seam is inert; nothing else depends on it.
