---
schema: oh.war/atom/v1
warrant_uuid: 01a01bcf-b0e1-71e3-a237-3554610254d3
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §23 (milestones and stages), §23.4 (executor kinds), §23.5 (named ports), §23.6 (stage versus milestone), §26 (responsibility tiers), §104.3 (the worked example).

## Prerequisites

OW-WAR-0002 resolved (atom parsing exists).

## Assumptions and Unknowns

- **Blocking unknown.** The milestones atom is YAML, and OW-ADR-0002 rejected a
  YAML library for FRONTMATTER on the grounds that frontmatter is six flat keys.
  A milestone graph is not six flat keys — it is nested sequences of mappings.
  Whether the restricted reader is extended or a real parser is adopted for
  structured atoms (§62.1 explicitly permits YAML or canonical JSON for
  machine-dense atoms) is an implementation ADR, and it blocks this Warrant.

## Constraints and Invariants

- **A dangling reference fails closed.** A milestone naming a stage that does not
  exist is an error, not a warning: it means the plan references work nobody
  defined.
- **The milestone graph is acyclic.** `depends_on` cycles make completion order
  undefined.
- **Executor kind and responsibility tier are orthogonal** (§26.5). A T1 stage may
  be executed by any kind of actor; conflating the two would let a tier imply an
  executor.
