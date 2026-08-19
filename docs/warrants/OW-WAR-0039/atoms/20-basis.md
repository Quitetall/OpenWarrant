---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdc-5a1f-771a-a8ed-983d14257cfe
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §94 (telemetry and unit economics), §95 (untracked-work detection).

## Prerequisites

OW-WAR-0031 resolved — the journal is where events land.

## Assumptions and Unknowns

- **Evidenced premise.** §94 enumerates the measurements and the derived metrics.
- **Accepted residual risk.** Measuring authoring cost requires instrumenting a
  human, which is approximate at best.

## Constraints and Invariants

- **Detection never fabricates a relation** (§95). An untracked commit is
  reported, never retroactively attached to a Warrant.
- **Metrics that do not move are defective.** A count that stays fixed while the
  quantity changes is the failure ADR 0180 named, and it applies here.
- **Untracked work is reported, not blocked.** Blocking would push work outside
  the tool entirely.
