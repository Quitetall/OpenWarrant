---
schema: oh.war/atom/v1
warrant_uuid: 01a09351-e725-73b2-8960-1d263af327da
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS §33.7: the Dispatch compiler SHALL record a budget.
- SAS §47.2: select only stage-relevant context.
- `openwarrant_core::tokens` names its method: `oh.war/token-estimate/bytes-div-4/v1`.

## Measured

Word and byte counts of the documents this repository actually dispatches
and projects, taken with `wc` at the commit that authored this Warrant; the
memo carries them.

## Assumptions

- A budget is a bound on reading cost, so a conservative (over-estimating)
  method is the safer error.
