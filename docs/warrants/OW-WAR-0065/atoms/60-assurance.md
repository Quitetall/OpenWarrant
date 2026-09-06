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
- **evidence:** performer artifact `wmd-wp007-lifecycle.log` records the H2
  lifecycle/replay test; full matrix is in `wmd-local-ci-765ab4b.log`. This is
  not independent verification.

## Gate adequacy and residual risk

The OpenWarrant gate receipt is a corpus-integrity check and does not run WMD
tests. WMD log is performer observation; independent verification and human
authorization are outstanding. Multi-entity action economics remain future
scope.
