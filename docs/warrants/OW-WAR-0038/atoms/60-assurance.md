---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdc-5a1f-7db2-82a1-a1f728931dd3
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — bytes and provenance survive
- **scope:** all 167 ADRs at a named commit.
- **evidence:** byte equality with the source, and the commit recorded.

### OBL-002 — no gate is fabricated
- **scope:** the 23 gates naming absent tools.
- **evidence:** each imports as `legacy_declared_unqualified`. Not one imports as
  a gate.

### OBL-003 — all ten classes survive
- **scope:** §96.4's vocabulary against the measured distribution.
- **evidence:** counts match the source; specifically `missing_tool` does not
  appear as `failed`.

### OBL-004 — a legacy Complete is a historical claim
- **scope:** frozen ADRs with no admissible evidence.
- **evidence:** each imports with resolution standing absent, not satisfied.

## Gate Adequacy

Required at `controlled` when executed — this is where 167 unverified historical
claims meet a system that represents verified ones.

**Could the import launder an unverified claim?** That is the entire risk. An
importer that maps `Complete — frozen` onto a WAR resolution would convert
prose into apparent proof for 51 records at once. OBL-004 is the control and it
is a single mapping decision; it must be reviewed by a human who has read §96.3,
not merely tested.

**Executed attacks:** recorded here when run.

## Residual Risk

The corpus moves. An import is a snapshot, and a snapshot presented as current is its own false claim.
