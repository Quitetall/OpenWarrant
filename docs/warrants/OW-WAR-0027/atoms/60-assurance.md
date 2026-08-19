---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd8-e8d8-714f-973c-6decbf23e65d
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — lowering produces a valid PlanSpec
- **scope:** WAR stage graphs using named typed ports.
- **evidence:** BLUT accepts the lowered PlanSpec.

### OBL-002 — lineage receipts are consumed
- **scope:** §49.2.
- **evidence:** a plant fabricating a receipt, refused.

### OBL-003 — verified against real BLUT
- **scope:** the checkout at `training/engine`.
- **evidence:** an integration run, not a mock. This adapter has no excuse for being unverified.

## Gate Adequacy

Not required at `basic`. Asked: lowering can be valid and semantically wrong — a PlanSpec BLUT accepts but that computes the wrong thing. Only the stage's own gates catch that.

## Residual Risk

BLUT's PlanSpec version may move. Pinning it is a dependency-management problem for beta.
