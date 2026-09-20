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

The revised product needs define sdk source types and prove provider resolution integration. Legacy command behavior does not
establish this SDK/provider contract.

## Desired Outcome

Phase 1 defines source identity, snapshot and reference types without I/O. Phase 2 validates a provider that owns capture and resolution, with exact provenance and access/refusal behavior.

## Scope

S03 (Phase 1); T11–T15 provider acquisition/resolution (Phase 2). Owner: LAMU/provider; SDK boundary. Primary phase: 2.
Mixed-phase work delivers its standalone SDK subset before provider integration.

## Non-goals

No ownership of a second semantic compiler, no inferred permissions, no historical
signature rewriting. Phase 3 workflow delivery and Phase 4 promotion remain
separate from a Phase 1 library result. No work outside the stated outcome.

## SAS and Roadmap Traceability

SAS RC.3 and its migration map assign this scope. WAR-SAS-RQ-050 remains the
legacy trace for bounded obligations, not an assertion that RC.3 is accepted.
