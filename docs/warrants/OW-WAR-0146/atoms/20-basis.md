---
schema: oh.war/atom/v1
warrant_uuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4e
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- §46.2: a blind verifier receives the contract, artifacts, gates, evidence
  and required context — the output of a gate run is that evidence.
- RQ-053 (performer reports cannot satisfy independent gates) and RQ-052
  (evidence, observation and judgment remain distinct): the verifier must
  see the gate's observation, not the performer's account of it.
- `bundle.rs` (`oh.war/verification-bundle/v1`): deliverables are already
  excerpted by `max_excerpt_bytes` with their full digest; this extends the
  same rule to gate output.
- OW-ADR-0021: `bundle.rs` is declared by OW-WAR-0127 (authorized, not yet
  delivered); on authorization this Warrant governs it, and 0127's later
  edit builds on these bytes.

## Assumptions

- A-001: adding fields to each `gate_runs` entry is additive to
  `oh.war/verification-bundle/v1`; no reader rejects unknown fields there.
  The plant checks the bundle still parses and verifies. Confidence: high.

## Residual risks

- R-001: a gate that prints secrets would now carry them into the bundle.
  The bundle is written under the Warrant's `verifications/` directory,
  where the gate output already sits in `gate-runs/`; nothing leaves the
  repository that was not already in it.
