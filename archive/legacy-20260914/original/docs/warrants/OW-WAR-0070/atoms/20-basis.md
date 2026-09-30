---
schema: oh.war/atom/v1
warrant_uuid: 01a09762-3fa2-7dd0-b794-0190b44fe879
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS §74 — drafting: the drafter proposes, the human decides; a Warrant is "waiting on" whoever holds the next decision.
- SAS §75 — the division of acts between human and agent; only human-held acts count as "waiting on me".
- SAS lifecycle / state sections — the enumerated Warrant states and the transitions that only a human may perform (approve, adopt, accept evidence, close).
- SAS §99 — the record is the source of truth; the inbox is derived from records on disk, never from a side database.
- OW-ADR-0016 — the SAS 1.0.0 pin; the classifier must read the pinned state vocabulary, not an ad hoc one.

## Assumptions carried in

- The Warrant state and the pending-obligation set are already parseable by the existing `war` loader; no new parser is needed.
- Each state maps deterministically to exactly one "next act kind"; where a state admits both a human and an agent act, the human act wins (the Warrant appears in the inbox).
- "Me" is the single human operator of this checkout; no identity lookup is performed.
- Wait age is computed from the most recent recorded transition timestamp in the record; if none exists, age is shown as `unknown`, not omitted.
- Section numbers other than §74/§75/§99 must be confirmed against the pinned SAS 1.0.0 text before the basis is frozen.