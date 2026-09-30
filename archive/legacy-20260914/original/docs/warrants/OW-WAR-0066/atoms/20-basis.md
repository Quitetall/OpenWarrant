---
schema: oh.war/atom/v1
warrant_uuid: 01a0935b-6870-73d1-bbbb-ca87591adbd4
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS §47 (Dispatch), §51 (Stage Submission; §51.2 a performer may not
  request its own completion), §44.6 (runtime receipts).
- `docs/gates/ops.echo@1.0.0.yaml` and `docs/gates/ops.conformance.plants@1.0.0.yaml`.

## Prerequisites

- A built binary and the battery: `cargo build --workspace`.

## Assumptions

- The receipt's subject is the dispatch digest: the run is evidence about
  what was dispatched, not about the contract as a whole.
