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
  duration and negative cost refuse; available capability admits one process.
  Tests: `unavailable_capability_refuses_action_admission`,
  `duplicate_start_is_refused_by_action_timeline`,
  `invalid_duration_refuses_action_admission`, and
  `negative_cost_refuses_action_admission`.

### OBL-002 - bounded lifecycle state

- **scope:** startup, active, recovery, complete, interrupted phases for H2
  process IDs.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** transition tests show canonical end ticks, terminal stability,
  successful interruption, interruption refusal after terminal state, tick
  overflow refusal, and 1,024-action ceiling. Tests:
  `terminal_action_is_stable_and_cannot_be_interrupted`,
  `active_action_can_be_interrupted_once`, `phase_end_overflow_is_refused`, and
  `timeline_capacity_refuses_action_1025`.

### OBL-003 - deterministic H2 transitions

- **scope:** WMD H2 scenario at commit `bc38378`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** performer artifact `wmd-wp007-lifecycle-bc38378.log` records H2
  lifecycle/refusal tests; full matrix is in `wmd-local-ci-bc38378.log`. This is
  not independent verification.

## Gate adequacy and residual risk

The OpenWarrant gate receipt is a corpus-integrity check and does not run WMD
tests. WMD log is performer observation; independent verification and human
authorization are outstanding. Multi-entity action economics remain future
scope.
