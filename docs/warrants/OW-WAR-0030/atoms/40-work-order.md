---
schema: oh.war/atom/v1
warrant_uuid: 01a01bda-15b7-7767-970e-1e270c168858
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. Full export per §68.2.
2. Import reconstructing the IR.
3. Byte-stable round-trip test over every populated section.
4. Historical-record inclusion.
5. `war export`.

## Frozen Surfaces

The export envelope and `war_export_digest`.

## Premade Instructions

- Extend the existing minimal round-trip test rather than writing a second one; that test caught a real defect and is the right shape.
- Every new IR section gets a round-trip case in the same commit that adds it.

## Autonomy and Escalation

Tier T1.

## Rollback

Revert. Export returns to `WAR.json`, which is the IR and not the archive.
