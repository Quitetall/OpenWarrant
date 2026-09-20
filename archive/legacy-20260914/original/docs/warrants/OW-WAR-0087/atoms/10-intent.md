---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1eae-7a43-92ec-9c6140696a23
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

1. Humans, scripts and skills can create or edit one document through explicit CLI fields and check it without interactive prompts.
2. A reviewer can reproduce the named positive and refusal observations at
   war document new/edit/check using explicit fields and one typed request model.

## Scope

Phase 2: F11, P2-AUTHOR.

## Non-goals

No work outside this bounded result. No historical signatures are rewritten.
No tracker TUI, service, browser, agent launcher, hotline or sibling integration
is required before Stable 1.0. Those workflows belong to Phase 3.

## SAS and Roadmap Traceability

The current SAS requirement WAR-SAS-RQ-050 governs bounded acceptance obligations.
The candidate feature/case mapping above defines the prospective RC.2 behavior.
The implementation roadmap records this Warrant's prerequisite edges and phase.
