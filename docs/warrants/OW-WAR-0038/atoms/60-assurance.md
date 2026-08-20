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
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** byte equality with the source, and the commit recorded.

### OBL-002 — no gate is fabricated
- **scope:** the 23 gates naming absent tools.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** each imports as `legacy_declared_unqualified`. Not one imports as
  a gate.

### OBL-003 — all ten classes survive
- **scope:** §96.4's vocabulary against the measured distribution.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** counts match the source; specifically `missing_tool` does not
  appear as `failed`.

### OBL-004 — a legacy Complete is a historical claim
- **scope:** frozen ADRs with no admissible evidence.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** each imports with resolution standing absent, not satisfied.

## Gate Adequacy

Required at `controlled` when executed — this is where 167 unverified historical
claims meet a system that represents verified ones.

**Could the import launder an unverified claim?** That is the entire risk. An
importer that maps `Complete — frozen` onto a WAR resolution would convert
prose into apparent proof for 51 records at once. OBL-004 is the control and it
is a single mapping decision; it must be reviewed by a human who has read §96.3,
not merely tested.

- **outcome:** no_counterexample_found, gap_accepted

**Executed attacks:**
- planted an ADR with an unknown status; refused by `adr.malformed`
- planted an ADR missing a required frontmatter key; refused by `adr.malformed`
- hand-edited the generated ADR overview; refused by `adr-overview.drift`
- and in unit tests, the rule that matters most: a legacy `Complete` line with no admissible evidence cannot be promoted to a resolution, and a legacy gate command stays `legacy_declared_unqualified` until all five of §96.3's conditions hold — each tested by removing one

## Residual Risk

The corpus moves. An import is a snapshot, and a snapshot presented as current is its own false claim.
