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

Required at `controlled` when executed — a Dispatch grants authority. Asked: a Dispatch can be complete and still authorize more than intended, because the capability list is authored. Nothing derives least privilege automatically.

## Residual Risk

Under-projection surfaces only at execution. Until a live runtime exists (beta), Dispatch completeness is argued rather than demonstrated.
