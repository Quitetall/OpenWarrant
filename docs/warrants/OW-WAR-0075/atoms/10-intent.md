---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1e85-7a81-ad46-75d006c36f89
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The revised product needs parse and validate human-first openwarrant documents. Legacy command behavior does not
establish this SDK/provider contract.

## Desired Outcome

Parse footer RC.3 and explicit legacy RC.2 documents; preserve exact spans and original bytes. Validate field meanings, kind minimums, extensions and validity separate from readiness.

## Scope

S01; T01–T10; FOOTER-01–FOOTER-08. Owner: OpenWarrant SDK. Primary phase: 1.
Mixed-phase work delivers its standalone SDK subset before provider integration.

## Non-goals

No ownership of a second semantic compiler, no inferred permissions, no historical
signature rewriting. Phase 3 workflow delivery and Phase 4 promotion remain
separate from a Phase 1 library result. No work outside the stated outcome.

## SAS and Roadmap Traceability

SAS RC.3 and its migration map assign this scope. WAR-SAS-RQ-050 remains the
legacy trace for bounded obligations, not an assertion that RC.3 is accepted.
