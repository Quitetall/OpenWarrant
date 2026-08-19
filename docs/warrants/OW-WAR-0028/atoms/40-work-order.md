---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd8-e8d8-7118-8a05-0e8bf46ef650
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. The controlled-action vocabulary of §67.
2. The action envelope with concurrency token and idempotency key.
3. Refusal of any direct state write.
4. Server-time stamping.

## Frozen Surfaces

The action vocabulary and the envelope. Both are the KF wire contract.

## Premade Instructions

- Make a direct status write unrepresentable in the API, not merely discouraged.
- Never stamp a controlled action with a local clock.

## Autonomy and Escalation

Tier T1.

## Rollback

Revert. Federation remains absent; local state is unaffected.
