---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdb-2e72-7188-bfa4-d386a25f4b66
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. The agent request/response protocol (§74.1–§74.2).
2. Typed atom operations (§74.3).
3. Proposal validation before any write.
4. The adapter trait (§75.3) and the generic process seam (§75.2).

## Frozen Surfaces

The proposal protocol and the no-mutation boundary.

## Premade Instructions

- Keep the crate free of any writable-path type. The current emptiness is accidental enforcement; make it structural.
- Validate against the SAME validators `war check` uses, not a second copy.

## Autonomy and Escalation

Tier T1 — this is an authority boundary.

## Rollback

Revert. The agent crate returns to protocol surface only.
