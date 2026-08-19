---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd6-ebf5-77d8-b498-a397823907f3
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a Dispatch is self-contained
- **scope:** §47.1.
- **evidence:** a compiled Dispatch is readable and complete with the repository
  absent — tested by moving it, not by inspection.

### OBL-002 — projection is actor-specific
- **scope:** §47.3.
- **evidence:** two actor roles receive different projections of one stage.

### OBL-003 — capabilities default to DENIED
- **scope:** §55.2.
- **evidence:** a plant requesting an unlisted capability, refused.

## Gate Adequacy

Required at `controlled` — a Dispatch grants authority.

**Adversarial question: could a Dispatch pass every declared gate while
authorizing more than was intended?** Yes. The capability list is authored, and
nothing derives least privilege automatically, so a Dispatch that is structurally
complete can still over-grant. Completeness is checkable; minimality is not.

- **outcome:** gap_accepted

Accepted with the gap named rather than closed: least-privilege derivation is
beta work against a live runtime, and claiming it here would be the kind of
unearned assurance this Warrant exists to prevent.

**Executed attacks:** none yet — the capability model does not exist until this
Warrant is executed, so there is nothing to attack. Recorded as absent rather
than omitted.

## Residual Risk

Under-projection surfaces only at execution. Until a live runtime exists (beta), Dispatch completeness is argued rather than demonstrated.
