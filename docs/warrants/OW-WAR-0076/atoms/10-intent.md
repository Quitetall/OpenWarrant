---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1e89-7e62-b59f-4ae8f6dfee3c
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The revised product needs author and edit human-first openwarrant documents. Legacy command behavior does not
establish this SDK/provider contract.

## Desired Outcome

Emit readable text first, compact footer metadata last. Preserve untouched units and explicit identity/revision changes; refuse title disagreement, duplicate fields and partial invalid output.

## Scope

S02; T54–T56; FOOTER-01–FOOTER-08. Owner: OpenWarrant SDK. Primary phase: 1.
Mixed-phase work delivers its standalone SDK subset before provider integration.

## Non-goals

No ownership of a second semantic compiler, no inferred permissions, no historical
signature rewriting. Phase 3 workflow delivery and Phase 4 promotion remain
separate from a Phase 1 library result. No work outside the stated outcome.

## SAS and Roadmap Traceability

SAS RC.3 and its migration map assign this scope. WAR-SAS-RQ-050 remains the
legacy trace for bounded obligations, not an assertion that RC.3 is accepted.
