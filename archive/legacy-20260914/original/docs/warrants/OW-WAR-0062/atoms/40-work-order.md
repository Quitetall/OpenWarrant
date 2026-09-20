---
schema: oh.war/atom/v1
warrant_uuid: 01a064fc-a6f2-7c43-bed9-fa248b771712
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. SAS §6.10, "The levels, and the one rule about SAS and Warrant": the
   level table (object, what it is, written by, governed by, read by), the
   rule, the two decisions, the counter-examples, and the correspondence
   table. Proposed and accepted as 0.1.0-draft.3.
2. `docs/DEFINITIONS.md`: the rule and one paragraph per object.
3. README: the status section replaced by a pointer to the computed ladder
   and to the definitions; the false counts removed.
4. `war new`: three comment lines at the top of every new manifest naming
   the rule and where it lives.
5. A core test that reads the SAS and fails if §6.10's heading, the rule's
   sentence, or the two decision questions are absent.

## Frozen Surfaces

§6.1–§6.9 as they stand. §106 — unchanged. The manifest schema.

## Premade Instructions

- The rule is stated once in the controlled document and restated
  elsewhere; where they differ, the SAS wins and the restatement is fixed.
- No new §106 row.

## Autonomy and Escalation

Tier T2. Escalate if the owner reads the rule as architecture-changing.

## Rollback

Revert; 0.1.0-draft.3 stays accepted and immutable, so a rollback is a
0.1.0-draft.4 that removes §6.10.
