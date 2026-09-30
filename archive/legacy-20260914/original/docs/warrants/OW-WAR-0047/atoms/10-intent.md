---
schema: oh.war/atom/v1
warrant_uuid: 01a021a6-0dfd-7e45-b33b-749b95409cd1
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

§98 Phase 7's exit is "compatible computational WARs execute without
duplicating BLUT." OW-WAR-0027 delivered §49's lowering, the adapter duties, and
the lineage receipt as a REFERENCE rather than a copy.

Nothing has been lowered. BLUT is checked out at `training/engine` in the
LamQuant tree and is the most accessible of the four neighbours, which makes this
the cheapest live integration and a good early beta target.

The word "duplicating" is the whole risk. §49.3 says BLUT's execution lineage
stays authoritative in BLUT. An adapter that copies lineage across produces a
second answer to a question that should have one, and the copies diverge
silently.

## Desired Outcome

One compatible stage graph lowered into BLUT, executed, and its lineage
referenced — not reproduced — in the Warrant.

## Scope

§49 in full and §91.7 test 47.

## Non-goals

- No BLUT feature work. The adapter is ours; the engine is not.
- No incompatible lowering. §49.2 says reject, not degrade.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-063` — Complete; §49 governs.
- Discharges §98 Phase 7 exit and §99 criterion 16.
