---
schema: oh.war/atom/v1
warrant_uuid: 01a021a2-b570-7f57-85b2-0f8189873d9e
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. A drafting agent reachable over §75.2's process protocol, named
   and version-pinned.
2. A recorded end-to-end run: one vague sentence in, one validated Draft
   Proposal out, applied through all eight §74.4 steps.
3. Plants for §91.8 tests 52–58 against the shipped binary.
4. A recorded case where the agent proposed a durable choice and it became a
   proposed ADR rather than a Work Order paragraph.

## Frozen Surfaces

`oh.war/draft-proposal/v1`, `oh.war/draft-request/v1`, and §74.3's operation vocabulary.

## Premade Instructions

- The agent is a separate process. If it can import this crate, it
  is not testing the seam.
- Every one of §91.8's seven tests gets a plant, not a `#[test]`. Test 53 (an
  agent cannot authorize) and test 54 (an agent cannot allocate an enterprise
  ID) are the two that matter most, because both are refusals of authority.
- Record the vague sentence verbatim. A request rewritten to suit the agent has
  not tested the vague-request claim.

## Autonomy and Escalation

Tier T2 — a human reviews every proposal before it is applied, per §74.4 step 6.

## Rollback

Revert. `war plan` returns to emitting a request and stopping, which is honest about having no agent.
