---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd3-caf4-7b46-8ed4-7e9634593bec
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §35 (rationale nodes and edges), §36 (assumptions and unknowns), §36.4 (circular validation prohibited).

## Prerequisites

OW-WAR-0012 resolved — facts are context items.

## Assumptions and Unknowns

- **Evidenced premise.** §35 enumerates node classes and edge types.
- **Blocking unknown.** Whether rationale is authored as structured data or
  extracted from prose. Extraction is fragile; structure is a burden on authors.
  This must be decided, not defaulted.

## Constraints and Invariants

- **A blocking unknown BLOCKS** (§36.3). It is not a note.
- **Circular validation is prohibited** (§36.4) and must be DETECTED — a premise
  whose support chain returns to itself is an error.
- **Claims narrow, never widen** (§36.5) as evidence is bounded.
