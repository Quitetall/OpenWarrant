---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd7-eb6b-7e8f-88ff-6849c6e49e2f
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

Katana owns the agent runtime and PromptIR (RQ-062), and OpenWarrant must not
duplicate it (RQ-064). What OpenWarrant owes is its side: compiling a Dispatch
Katana can execute, and consuming the runtime receipt Katana returns.

Neither exists. Katana also has no checkout on this host, so the seam can be
built but not exercised.

## Desired Outcome

OpenWarrant compiles a Dispatch to Katana's expected shape and consumes its runtime receipt, without owning PromptIR, capabilities, or the agent loop.

## Scope

The runtime seam (§48.1), the PromptIR boundary (§48.2), capability declaration (§48.3), runtime receipts (§48.4), and taint (§48.5).

## Non-goals

- **No PromptIR.** §48.2 gives Katana ownership; constructing one here would be
  the duplication RQ-064 forbids.
- No live execution — Katana is not checked out. Integration is beta.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-044` — Complete; governing section in Basis.
- `WAR-SAS-RQ-062` — Complete; governing section in Basis.
