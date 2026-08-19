---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd5-a42a-766a-916c-656c6b37998b
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — an unaskable gate has NO verdict
- **scope:** §44.1.
- **evidence:** the type makes a verdict unrepresentable when unaskable; a test
  asserts it.

### OBL-002 — all ten statuses survive
- **scope:** the §96.4 vocabulary.
- **evidence:** one plant per status, each reported as itself. Specifically:
  `missing_tool` must not be reported as `failed`.

### OBL-003 — a required unknown BLOCKS resolution
- **scope:** RQ-054.
- **evidence:** a plant; resolution refuses.

### OBL-004 — invalidation propagates
- **scope:** RQ-057, §45.
- **evidence:** invalidating a gate run marks every resolution resting on it as
  invalidated, transitively.

### OBL-005 — a mutating gate is quarantined
- **scope:** §44.8.
- **evidence:** a declared-mutating gate is refused in a routine check run.

## Gate Adequacy

Required at `controlled`, and this is the Warrant where a defect is worst: a
false PASS here manufactures a resolution.

**Could the obligations pass while a gate result is still false?** Yes, in the
way that matters most: a gate can be askable, execute cleanly, return exit 0, and
measure the wrong thing. Nothing in §44 verifies that a gate's result corresponds
to the obligation it is cited against. §39's adequacy review is the only control
that asks that question, and it is a human one.

**Executed attacks:** recorded here when run.

## Residual Risk

Mutation detection is declarative — a gate that mutates without declaring it will not be quarantined. Detecting undeclared mutation reliably would need sandboxing, which is hardening and therefore beta.
