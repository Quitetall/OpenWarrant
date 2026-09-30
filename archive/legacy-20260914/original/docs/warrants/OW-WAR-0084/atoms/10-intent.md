---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1ea5-78c2-be44-8a1bef260a77
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

1. A caller can use mapped legacy material and successor revisions while retaining original bytes, identities, digest domains and honest unsupported facts.
2. A reviewer can reproduce the named positive and refusal observations at
   import_legacy and successor-history validation over retained legacy fixtures.

## Scope

Phase 1: F10, T49, T50, T51, T52, T53.

## Non-goals

No work outside this bounded result. No historical signatures are rewritten.
No tracker TUI, service, browser, agent launcher, hotline or sibling integration
is required before Stable 1.0. Those workflows belong to Phase 3.

## SAS and Roadmap Traceability

The current SAS requirement WAR-SAS-RQ-050 governs bounded acceptance obligations.
The candidate feature/case mapping above defines the prospective RC.2 behavior.
The implementation roadmap records this Warrant's prerequisite edges and phase.
