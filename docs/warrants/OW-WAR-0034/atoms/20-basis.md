---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdb-2e72-7188-bfa4-d386a25f4b66
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §74 (agent inputs, output, atom operations, validation before application, no direct model mutation), §75 (adapter protocol, trait shape, isolation).

## Prerequisites

OW-WAR-0016 resolved — a proposal may create obligations.

## Assumptions and Unknowns

- **Evidenced premise.** §74 and §75 specify both sides.
- **Accepted residual risk.** Validation catches malformed proposals, not misguided ones. A well-formed proposal to do the wrong thing passes.

## Constraints and Invariants

- **No direct model mutation** (§74.5, RQ-072). The adapter returns data; the CLI writes. No type in the agent crate carries a writable path.
- **Validation precedes application** (§74.4), always.
- **Adapter isolation** (§75.4): an adapter runs as a separate process.
