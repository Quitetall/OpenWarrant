---
schema: oh.war/atom/v1
warrant_uuid: 01a01bcf-b0e1-71e3-a237-3554610254d3
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. An implementation ADR resolving the structured-atom parser question.
2. `Milestone` and `Stage` types with executor kinds and responsibility tiers.
3. Named typed ports on stages (§23.5).
4. Validation: dangling refs, duplicate ids, dependency cycles.
5. `war check` reports the milestone/stage graph and its defects.
6. The Warrant Overview reports declared milestone COUNT rather than the word
   "declared".

## Frozen Surfaces

The executor-kind and responsibility-tier vocabularies (§23.4, §26). Both appear in Dispatch later; changing them changes what an actor is told about its own authority.

## Premade Instructions

- The parser decision is an ADR and precedes the code.
- Report every dangling reference, not the first.
- Do NOT infer milestone completion from anything. A milestone with no state
  model has no state, and guessing one is the failure this Warrant is fixing.

## Autonomy and Escalation

Tier T1. The vocabularies and the structured-atom parser choice both escalate.

## Rollback

Revert. The milestones atoms remain valid source; they simply go back to being unread — which is the status quo this Warrant exists to end.
