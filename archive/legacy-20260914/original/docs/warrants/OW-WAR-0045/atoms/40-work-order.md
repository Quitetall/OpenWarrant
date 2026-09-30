---
schema: oh.war/atom/v1
warrant_uuid: 01a021a4-be74-73ce-8f9d-105d80ab82fc
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. A Katana adapter reachable over §48.1's seam, version-pinned.
2. One compiled Dispatch, executed, with a §48.4 receipt carrying all eleven
   minimum fields and the correct Dispatch digest.
3. A Stage Submission that requests one of §51.2's five actions.
4. Plants for §91.7 tests 43–51 and §91.9 tests 59–63.
5. A recorded capability comparison showing realized ⊆ authorized.

## Frozen Surfaces

`oh.war/stage-dispatch/v1`, `oh.war/stage-submission/v1`, and §48.4's minimum receipt fields.

## Premade Instructions

- Deliberately omit something the agent needs, and record what it
  does. §53.1 says it should raise a BLOCKER, not improvise. An agent that
  invents the missing piece has told you the Dispatch was insufficient and that
  nothing detected it.
- Attempt a submission requesting `resolve`. It must be refused by name.
- Run a repair attempt with no prior failure evidence and confirm refusal — §52.3
  says a repair sees what failed, and one that does not is a retry.

## Autonomy and Escalation

Tier T2 — a real external executor acts on this repository's behalf.

## Rollback

Revert the adapter. Dispatches remain compilable and unexecuted, which is the current honest state.
