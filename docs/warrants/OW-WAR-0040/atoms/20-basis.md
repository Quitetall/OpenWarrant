---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdc-5a20-77b7-800c-673a6394651b
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §11.3 (Liminal is the eventual semantic substrate), §82 (source adapters, Liminal adapter, adapter parity, cutover), §97.5.

## Prerequisites

OW-WAR-0032 resolved — parity is measured against a pinned schema pack.

## Assumptions and Unknowns

- **Blocking unknown, and the largest in the roadmap.** Liminal has no checkout
  on this host. Its CST/HIR/CIR path, Workspace Basis representation, and
  Jurisdiction model are described in the SAS but unavailable. This Warrant
  cannot start until a checkout exists, and saying otherwise would be
  planning against a document rather than a system.

## Constraints and Invariants

- **Parity is MEASURED, not asserted** (§82.3). Both adapters produce
  byte-identical canonical IR for the same sources, or there is no cutover.
- **The old compiler stays as an oracle during parity** (§97.5).
- **Exact source preservation survives** (§62.2) across adapters.
