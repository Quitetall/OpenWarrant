---
schema: oh.war/atom/v1
warrant_uuid: 01a01bda-15b7-7767-970e-1e270c168858
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §68 (one-file canonical export, export contents, round trip), §69 (protocol versioning).

## Prerequisites

OW-WAR-0022 resolved — an export without resolution is not §68.2's export.

## Assumptions and Unknowns

- **Evidenced premise.** The IR already round trips minimally and the serde asymmetry that broke it once is fixed and pinned by a test.
- **Accepted residual risk.** Each new IR section is another chance to reintroduce that asymmetry.

## Constraints and Invariants

- **Historical records remain available** (RQ-084). Superseded, disputed, and annulled records are exported, not pruned.
- **Round trip is byte-stable** (§68.3), asserted against a re-export rather than by inspection.
- **Unknown extensions survive** (§69.4).
