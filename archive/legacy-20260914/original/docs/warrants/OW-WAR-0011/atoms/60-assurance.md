---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd1-b0d3-7157-b2b2-9c2625d43897
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — all six dimensions are addressed
- **scope:** §32.1 contract, §32.2 context, §32.3 graph, §32.4 runtime,
- **gate:** `gate://software.repo.war-check@1.0.0`
  §32.5 gates, §32.6 authority.
- **evidence:** each reports pass or unknown; none is silently omitted.

### OBL-002 — an unchecked dimension BLOCKS
- **scope:** a dimension Preflight cannot exercise locally.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant; the verdict is not-ready with the dimension named.

### OBL-003 — the report names what it could not exercise
- **scope:** every Preflight run.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the runtime dimension reports unknown while no live actor exists,
  on every run, rather than being quietly dropped.

## Gate Adequacy

Not required at `basic`, but this Warrant is a candidate for `controlled` when written for real: a Preflight that passes wrongly authorizes execution of work that cannot run.

## Residual Risk

Preflight is partial until Phase 5 by construction. The honest failure mode is a Warrant that passes local Preflight and fails immediately against a live runtime.
