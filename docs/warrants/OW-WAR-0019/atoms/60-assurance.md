---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd5-a42a-7dc9-819f-8ca614dc87eb
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — definitions are versioned and governed
- **scope:** §43.2's schema.
- **evidence:** a definition without a version is refused.

### OBL-002 — an unqualified gate cannot be BOUND
- **scope:** §43.4.
- **evidence:** a plant and its refusal.

### OBL-003 — an obligation citing a nonexistent gate is REFUSED
- **scope:** the failure the parent project shipped 23 times.
- **evidence:** a plant citing `gate://does-not-exist`, refused by name.

### OBL-004 — bindings are digested
- **scope:** `oh.war/gate-binding/v1`.
- **evidence:** a digest distinct from every other domain.

## Gate Adequacy

Required at `controlled`.

**Could this pass while a gate is still a lie?** Yes. A Definition can be
versioned, qualified, and bound, and still describe a check that does not measure
what it claims. Qualification proves a gate CAN run and produce a result; it does
not prove the result means anything. §46's verifier independence and §39's
adequacy review are what address that, and both are separate Warrants.

**Executed attacks:** recorded here when run.

## Residual Risk

A local registry standing in for the KF-owned one is provisional authority. If that provisionality is not visible in the record, a locally-qualified gate will be mistaken for an institutionally-qualified one.
