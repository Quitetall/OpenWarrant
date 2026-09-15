---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1ea8-78c3-a946-d800169df834
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the scoped behavior is observed
- **scope:** all public SDK operations through direct driver and noninteractive CLI; fixed skill artifact cases; S01–S08; SDK-01–SDK-24; FOOTER-01–FOOTER-08; Phase 1 portions of T01–T56.
- **evidence:** Run complete Phase 1 inventory, including S07 consumer types and S08 skill artifacts. Cross-provider semantic execution is Phase 2. Prove caller parity and truthful reports; document supported/unsupported capability per operation. Record exact inputs, outputs, exit codes and source/fixture/build revisions through the public boundary.

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

`cargo run -p openwarrant-core --example sdk_probe -- --scope 85 --fixtures conformance/sdk` after the driver exists; test public library and CLI parity where applicable.
The driver is planned, not claimed shipped. Phase exit also runs `cargo xtask gate`
on the pinned toolchain; authoritative new gate registration remains separate.
