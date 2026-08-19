---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd8-e8d8-714f-973c-6decbf23e65d
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §49 (lowering, adapter duties, authority), §11.5.

## Prerequisites

OW-WAR-0007 and OW-WAR-0023 resolved — named typed ports are what lower to PlanSpec ports.

## Assumptions and Unknowns

- **Evidenced premise.** BLUT is checked out locally at `training/engine`, so unlike the Katana seam this one can be tested against the real system rather than a description.
- **Accepted residual risk.** BLUT's PlanSpec IR has its own version; a mismatch is a beta integration concern.

## Constraints and Invariants

- **BLUT owns the DAG** (§49.3). Reimplementing scheduling here is the duplication RQ-064 forbids.
- **Lineage receipts are consumed, not minted**, exactly as with Katana.
- **Named typed ports map to PlanSpec ports** — §23.5's ports exist for this.
