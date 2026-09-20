---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1e89-7e62-b59f-4ae8f6dfee3c
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the scoped behavior is observed
- **scope:** author_document/edit_document followed by public parse_document and validate_document; S02; T54–T56; FOOTER-01–FOOTER-08.
- **evidence:** Emit readable text first, compact footer metadata last. Preserve untouched units and explicit identity/revision changes; refuse title disagreement, duplicate fields and partial invalid output. Record exact inputs, outputs, exit codes and source/fixture/build revisions through the public boundary.

### OBL-002 — the boundary refuses invalid inputs
- **scope:** malformed, incomplete, conflicting, over-limit and authority-confused inputs for the same boundary; every mapped historical refusal remains in its original profile scope.
- **evidence:** observed named refusals, unchanged prior output on failed writes, and mutations of real content rather than fixture-name selection. Positive checks alone are insufficient.

### OBL-003 — independent evidence supports qualification of the exact result
- **scope:** qualification only, for this exact result and OBL-001/002; not a condition for reporting unverified work complete.
- **evidence:** independent reproduction/review, applicable baseline conditions and secure human acceptance of the exact subject. Late review does not invent pre-work facts.

### OBL-004 — Warrant records stay structurally valid
- **scope:** this Warrant's authored records/references; no runtime feature or assurance claim.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** structural check and existing planted refusals, with independently bounded scope.

## Gate Adequacy

These implementation observations are NOT RUN until attached evidence proves them.
No disposition is declared here. Unverified completion records actual OBL-001/002
outcomes and notes qualification gaps; legacy resolution may still require all
four obligations. `war check` cannot establish runtime behavior or independent
verification. Unknown observations cannot pass.

**Adversarial question:** could constant success, fixture-name lookup, copied
performer assertions or an unobserved test establish the claim? Each named control
requires its own observed refusal and exact evidence.

## Planned acceptance entry point

`cargo run -p openwarrant-core --example sdk_probe -- --scope 76 --fixtures conformance/sdk` after the driver exists; test public library and CLI parity where applicable.
The driver is planned, not claimed shipped. Phase exit also runs `cargo xtask gate`
on the pinned toolchain; authoritative new gate registration remains separate.
