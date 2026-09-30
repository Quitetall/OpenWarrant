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

The revised product needs expose document and record primitives through cli. Legacy command behavior does not
establish this SDK/provider contract.

## Desired Outcome

Expose parsing, validation, authoring, record and integrity operations with explicit input/output, one JSON envelope, nonzero refusal and no hidden prompts/signing/model call. Preserve legacy command meanings.

## Scope

S06; SDK-01–SDK-06; Phase 1 CLI parity. Owner: OpenWarrant CLI. Primary phase: 1.
Mixed-phase work delivers its standalone SDK subset before provider integration.

## Non-goals

No ownership of a second semantic compiler, no inferred permissions, no historical
signature rewriting. Phase 3 workflow delivery and Phase 4 promotion remain
separate from a Phase 1 library result. No work outside the stated outcome.

## SAS and Roadmap Traceability

SAS RC.3 and its migration map assign this scope. WAR-SAS-RQ-050 remains the
legacy trace for bounded obligations, not an assertion that RC.3 is accepted.
