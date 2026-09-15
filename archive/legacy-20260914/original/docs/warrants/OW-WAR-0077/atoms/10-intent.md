---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1e8c-7ec1-a56f-990a6ac8f353
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The RC.2 build requires the bounded result below; the existing implementation
does not establish this new contract merely by having similar legacy commands.

## Desired Outcome

1. A caller obtains one immutable source lock and unambiguous references to exact document units and opaque evidence bytes.
2. A reviewer can reproduce the named positive and refusal observations at
   capture_sources and resolve_refs with an explicit filesystem shell.

## Scope

Phase 1: F03, T11, T12, T13, T14, T15.

## Non-goals

No work outside this bounded result. No historical signatures are rewritten.
No tracker TUI, service, browser, agent launcher, hotline or sibling integration
is required before Stable 1.0. Those workflows belong to Phase 3.

## SAS and Roadmap Traceability

The current SAS requirement WAR-SAS-RQ-050 governs bounded acceptance obligations.
The candidate feature/case mapping above defines the prospective RC.2 behavior.
The implementation roadmap records this Warrant's prerequisite edges and phase.
