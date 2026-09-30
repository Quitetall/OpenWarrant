---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd2-7d30-792c-ba72-6b74daa59f19
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. §106 requirement index resolution.
2. Refusal of unresolvable `sas://` and `roadmap://` refs.
3. A requirement-status record distinct from Warrant claims.
4. Coverage reporting: which of the 57 requirements have no Warrant at all.

## Frozen Surfaces

The `sas://` and `roadmap://` URI forms (§105).

## Premade Instructions

- Report the requirements with NO Warrant. That number is the honest measure of
  how far from feature-complete the system is, and it belongs in the Overview.
- Do not let a Warrant's claim promote a requirement's status.

## Autonomy and Escalation

Tier T2, except the claim/status separation which is T1.

## Rollback

Revert. Coverage returns to being claimed and unverified, as labelled today.
