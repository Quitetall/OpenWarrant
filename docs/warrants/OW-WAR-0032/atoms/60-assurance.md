---
schema: oh.war/atom/v1
warrant_uuid: 01a01bda-15b8-7086-ae76-38561d173119
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — schemas generate from the types
- **scope:** the IR, manifest, Dispatch, Submission, and gate schemas.
- **evidence:** generated output validates the existing corpus.

### OBL-002 — the pack is digested and pinned
- **scope:** §64.
- **evidence:** `FormatBasis.digest` matches the assembled pack.

### OBL-003 — generated schemas cannot drift
- **scope:** the generation step.
- **evidence:** a plant editing a generated schema by hand, refused.

### OBL-004 — TypeScript consumes rather than reimplements
- **scope:** §83.4.
- **evidence:** generated TS types exist and KF imports them.

## Gate Adequacy

Not required at `basic`; `controlled` when executed since the pack is the published contract. Asked: generated schemas match our types, which is not the same as matching the SAS. A type that misreads the specification generates a schema that misreads it identically.

## Residual Risk

Generation propagates a misreading faithfully. Only conformance against the SAS's own examples catches that.
