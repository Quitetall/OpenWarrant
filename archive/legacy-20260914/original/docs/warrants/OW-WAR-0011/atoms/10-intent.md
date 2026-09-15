---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd1-b0d3-7157-b2b2-9c2625d43897
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

`war check` says WELL-FORMED and explicitly refuses to say READY, because §32
defines readiness as including Preflight and Preflight does not exist. Every
report carries that caveat. A Warrant can be perfectly well-formed and completely
unexecutable — its gates may name tools that are absent, its context may be
unresolvable, its authority may be missing.

## Desired Outcome

`war preflight` exercises the real actor path (§32.7) and readiness becomes a claim about executability, not just about the record. The WELL-FORMED caveat is removed because it is no longer needed.

## Scope

Contract, context, graph, runtime, gates, and authority readiness (§32.1–§32.6), and the Preflight command.

## Non-goals

- No execution. Preflight proves a stage COULD run; running it is OW-WAR-0023.
- No gate execution; Preflight checks a gate is ASKABLE, which is §44.1 and
  arrives with OW-WAR-0020.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-035` — see the SAS section named in Basis. Complete.
