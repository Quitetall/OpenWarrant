---
schema: oh.war/atom/v1
warrant_uuid: 01a01bda-15b8-7086-ae76-38561d173119
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

`FormatBasis` names a schema pack `openwarrant-schema-pack` version `0.1.0` that does not exist as an artifact. §64 requires it to transitively pin core schema, profiles, vocabularies, gate schemas, and the Dispatch and Submission protocols. §83.4 requires KF's TypeScript to consume GENERATED schemas rather than reimplement WAR semantics — impossible while nothing is generated.

## Desired Outcome

The schema pack is a real, digested artifact. JSON Schema is generated from the Rust types, TypeScript consumes it, and protocol versioning is enforced.

## Scope

Schema pack assembly and digesting (§64), semantic versioning and additive evolution (§69.1–§69.2), breaking-change handling (§69.3), and unknown extensions (§69.4).

## Non-goals

- No TypeScript implementation of WAR semantics. §77.3 and §83.4 keep TS as an integration layer.

## SAS and Roadmap Traceability

No §106 requirement maps here directly; enabling work named in the Production Roadmap.
