---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd2-7d30-7dc6-a3b7-92bb556e3569
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `ContextItem` with role and trust class.
2. Declared precedence and conflict detection.
3. `context_manifest_digest` — one of the fifteen domains, currently unused.
4. Completeness checking per §33.5.
5. Summary provenance enforcement per §33.8.

## Frozen Surfaces

The trust-class vocabulary and the precedence rule. Both decide which of two conflicting sources is believed.

## Premade Instructions

- An unresolved conflict is an error. Do not add a tiebreak that is not in §33.4.
- A summary without provenance is refused at parse time, not at use time.

## Autonomy and Escalation

Tier T1 for the trust vocabulary and precedence; T2 for the plumbing.

## Rollback

Revert. Context returns to prose in the Basis atom.
