---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd0-d818-7d1b-969f-79f172735b78
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `ContractRevision` with draft / proposed / authorized states and ancestry.
2. Contract digest computed over §29's content, frozen at authorization.
3. Refusal of any mutation to an authorized revision.
4. Attempt-basis binding so a prior attempt cites its own revision.
5. `war check` reporting revision ancestry and any in-place amendment attempt.

## Frozen Surfaces

The contract digest's preimage — what is IN the contract per §29. Changing it changes every digest ever minted.

## Premade Instructions

- Delete `PHASE_1_CONTRACT_REVISION`; do not repurpose it.
- Authorization without an authority must be represented as what it is. If a
  local authorization is allowed, it is labelled `local`, and a KF-registered
  Warrant must be able to refuse one.
- Plant an in-place amendment and require refusal.

## Autonomy and Escalation

Tier T1. Everything here is protocol semantics with an immutability guarantee attached.

## Rollback

Revert. Existing pinned parent digests keep verifying, since the digest computation itself is unchanged.
