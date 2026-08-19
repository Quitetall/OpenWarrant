---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd0-d818-7d1b-969f-79f172735b78
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-000 — local authorization is defined, not assumed
- **scope:** how a revision becomes authorized with no KF present.
- **evidence:** a written decision; §27.2 forbids self-authorization, so silence
  here would ship exactly that.

### OBL-001 — an authorized revision is immutable
- **scope:** every mutating path on a contract revision.
- **evidence:** the type system or an explicit refusal; a test that mutation is
  unrepresentable or rejected.

### OBL-002 — amendment attempts are REFUSED
- **scope:** editing an authorized revision; a Progress Log entry that changes
  contract content.
- **evidence:** two plants, two refusals.

### OBL-003 — prior attempts retain their basis
- **scope:** an attempt recorded under revision N, after revision N+1 is authorized.
- **evidence:** the attempt still cites revision N's digest.

## Gate Adequacy

Required at `controlled`.

**Could this pass while a contract is still effectively mutable?** Yes — the
source atoms remain editable files. Immutability here binds the REVISION record,
not the filesystem. Someone editing an atom and recompiling produces a different
digest, which the children detect (that check already exists), but nothing
prevents the edit. True immutability needs either KF custody or signatures, both
of which are later. This Warrant delivers detection, not prevention, and must say
so where a reader will see it.

**Executed attacks:** recorded here when run.

## Residual Risk

Detection rather than prevention, as above. Local authorization is a placeholder whose weakness is bounded only by OBL-000's honesty.
