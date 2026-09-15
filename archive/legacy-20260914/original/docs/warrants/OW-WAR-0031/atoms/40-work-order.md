---
schema: oh.war/atom/v1
warrant_uuid: 01a01bda-15b8-7209-89bb-f36dde06b52c
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. Append-only journal with the §66.3 envelope.
2. The §66.4 material event set.
3. State recorded from journal events rather than derived.
4. `.openwarrant/state/` per §59, tracked or not by repository policy.

## Frozen Surfaces

The event envelope and the material-event list.

## Premade Instructions

- Append-only means no rewrite path exists in the API, not that one is discouraged.
- Once state is journalled, OW-WAR-0008's derived/recorded marker must start saying 'recorded'.

## Autonomy and Escalation

Tier T2, except append-only enforcement which is T1.

## Rollback

Revert. State returns to derived, which OW-WAR-0008 already labels honestly.
