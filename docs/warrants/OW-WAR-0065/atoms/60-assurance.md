---
schema: oh.war/atom/v1
warrant_uuid: 01a074c1-3eac-7bf2-92ec-9fdddfa15045
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance obligations

### OBL-001 - capability-gated admission

- **scope:** `ActionTimeline::admit` over H2 action fixtures.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** capability unavailable refuses; duplicate process and invalid
  duration refuse; available capability admits one process.

### OBL-002 - bounded lifecycle state

- **scope:** startup, active, recovery, complete, interrupted phases for H2
  process IDs.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** transition tests show canonical end ticks, terminal stability,
  interruption refusal after terminal state, and 1,024-action ceiling.

### OBL-003 - deterministic H2 transitions

- **scope:** WMD H2 scenario at commit `765ab4b`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `./scripts/ci.sh` local receipt; repeated scenario has identical
  transition stream and checksum.

## Gate adequacy and residual risk

The gate must include refusal plants, not only happy-path transitions. Local
receipts are performer observations; independent verification and human
authorization are outstanding. Multi-entity action economics remain future
scope.
