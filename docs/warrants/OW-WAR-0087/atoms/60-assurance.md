---
schema: oh.war/atom/v1
warrant_uuid: 01a09e54-1eae-7a43-92ec-9c6140696a23
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the scoped behavior is observed
- **scope:** noninteractive CLI compared with the same public SDK calls; S06; SDK-01–SDK-06; Phase 1 CLI parity.
- **evidence:** Expose parsing, validation, authoring, record and integrity operations with explicit input/output, one JSON envelope, nonzero refusal and no hidden prompts/signing/model call. Preserve legacy command meanings. Record exact inputs, outputs, exit codes and source/fixture/build revisions through the public boundary.

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

`cargo test -p openwarrant-cli --test sdk_cli` drives the actual CLI against
`conformance/sdk/cli/cases.json` and compares public SDK results. The acceptance
driver belongs to the CLI crate so Cargo supplies the exact built CLI binary;
no core-library dependency on an installed CLI is introduced. Phase exit also runs `cargo xtask gate`
on the pinned toolchain; authoritative new gate registration remains separate.
