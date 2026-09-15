---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd7-eb6b-7cc7-a7d3-bb8ba8239f96
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §51 (Stage Submission, no self-completion, performer claim status), §52 (initial, replay, repair, restart, attempt lineage).

## Prerequisites

OW-WAR-0023 resolved.

## Assumptions and Unknowns

- **Evidenced premise.** §52 defines all four kinds and their distinctions.
- **Accepted residual risk.** Distinguishing a repair from a restart depends on
  what changed, which is an authored classification like §30's amendment classes.

## Constraints and Invariants

- **No self-completion** (§51.2). A Submission carries a performer CLAIM; only
  verification and resolution produce an outcome.
- **A prior attempt retains its basis** (RQ-034), already required by OW-WAR-0009.
- **The four attempt kinds are distinct** (RQ-045) and none defaults to another.
