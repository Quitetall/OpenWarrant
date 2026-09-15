---
schema: oh.war/atom/v1
warrant_uuid: 01a021a4-be74-73ce-8f9d-105d80ab82fc
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

§98 Phase 5's exit is "one WAR stage can be compiled, executed by a
stateless Katana agent, and returned without authority confusion." OW-WAR-0023,
0024, 0025 and 0026 delivered the Dispatch schema, actor projection, Stage
Submission, attempt semantics, the four remedies, and the Katana receipt.

No Dispatch has been executed by anything. §91.7's nine tests and §91.9's five
have zero citations. The three phrases in the exit criterion are each a separate
claim: STATELESS (the Dispatch is the only packet), EXECUTED (something real ran
it), and WITHOUT AUTHORITY CONFUSION (§51.2's no-self-completion held under
pressure from a real agent that wanted to be done).

## Desired Outcome

One stage compiled into a Dispatch, executed by a real Katana agent that
received nothing but that Dispatch, returning a Submission that requests a next
action and does not resolve anything.

## Scope

§47, §48, §51, §52, §53, §91.7 tests 43–51, §91.9 tests 59–63.

## Non-goals

- No gate execution against the result. Closing a delivery is OW-WAR-0046.
- No multi-stage orchestration. One stage discharges the exit.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-042`, `WAR-SAS-RQ-043` — Complete; §47 governs.
- `WAR-SAS-RQ-044` — Complete; §48 governs.
- `WAR-SAS-RQ-045` — Complete; §51 and §52 govern.
- Discharges §98 Phase 5 exit and §99 criteria 14, 15 and 18.
