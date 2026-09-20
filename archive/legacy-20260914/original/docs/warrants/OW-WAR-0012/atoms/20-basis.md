---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd2-7d30-7dc6-a3b7-92bb556e3569
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §33 in full: context items, roles, trust classes, precedence, completeness, manifest, projection, summary provenance.

## Prerequisites

OW-WAR-0009 resolved — the context manifest is part of what a contract binds.

## Assumptions and Unknowns

- **Evidenced premise.** §33 fully enumerates roles, classes, and precedence.
- **Accepted residual risk.** Trust class is DECLARED by the author. Nothing
  verifies that a source labelled first-party measurement actually is one.

## Constraints and Invariants

- **A summary names its source** (§33.8). A summary with no provenance is
  inadmissible, because it is indistinguishable from an assertion.
- **Precedence is declared, not inferred** (§33.4). Where two context items
  conflict and precedence does not resolve it, that is an error, not a coin flip.
