---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdb-2e72-7530-adc8-260579bdad3b
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §71.10 (`war diff`), §28.6 (revision ancestry), §74.2 (semantic diff in proposals).

## Prerequisites

OW-WAR-0009 resolved — there must be two revisions to diff.

## Assumptions and Unknowns

- **Evidenced premise.** The IR is canonical, so a structural diff is well-defined.
- **Accepted residual risk.** A semantic diff can be technically complete and still unreadable if it reports every field.

## Constraints and Invariants

- **Diff over the IR, not the Markdown.** Rendering churn is not change.
- **A contract-digest move is explained**, not merely reported — naming which field moved it.
- **Diff is read-only.**
