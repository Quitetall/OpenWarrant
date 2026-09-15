---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1eb7-78d3-8eb0-45b5463010df
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The revised product needs qualify four-phase release and freeze stable source set. Legacy command behavior does not
establish this SDK/provider contract.

## Desired Outcome

Collect all four phase exits, including real Phase 3 webapp, first-party app integration and real-user observations. Freeze exact source set and review manifest; remaining gaps prevent qualification. Do not treat old two-phase compiler tests as the whole Stable gate.

## Scope

Phase 4 qualification; P2-CONFORMANCE and P2-CONTEXT-12 retained; all four phase exits. Owner: OpenWarrant release + independent reviewers. Primary phase: 4.
Mixed-phase work delivers its standalone SDK subset before provider integration.

## Non-goals

No ownership of a second semantic compiler, no inferred permissions, no historical
signature rewriting. Phase 3 workflow delivery and Phase 4 promotion remain
separate from a Phase 1 library result. No work outside the stated outcome.

## SAS and Roadmap Traceability

SAS RC.3 and its migration map assign this scope. WAR-SAS-RQ-050 remains the
legacy trace for bounded obligations, not an assertion that RC.3 is accepted.
