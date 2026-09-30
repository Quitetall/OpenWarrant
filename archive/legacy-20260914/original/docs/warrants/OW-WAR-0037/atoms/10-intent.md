---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdb-2e72-7530-adc8-260579bdad3b
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

There is no way to see what changed between two contract revisions except reading both. A textual diff of a generated parent shows rendering churn alongside meaning; §71.10 asks for a SEMANTIC difference, over the IR rather than the Markdown.

## Desired Outcome

`war diff` reports what changed semantically between two revisions or two Bases — which obligations moved, which atoms changed, whether the contract digest moved and why.

## Scope

`war diff --from contract:N --to contract:M` over the IR, and semantic diff for agent proposals (§74.2).

## Non-goals

- No merge, no conflict resolution. Diff reports; it does not reconcile.

## SAS and Roadmap Traceability

No §106 requirement maps here directly; enabling work named in the Production Roadmap.
