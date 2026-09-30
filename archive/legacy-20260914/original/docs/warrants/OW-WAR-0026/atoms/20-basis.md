---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd7-eb6b-7e8f-88ff-6849c6e49e2f
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §48 (runtime seam, PromptIR ownership, capabilities, runtime receipt, taint), §11.4.

## Prerequisites

OW-WAR-0023 resolved.

## Assumptions and Unknowns

- **Blocking unknown.** Katana's receipt schema is fixed by Katana at a commit
  the SAS names (`651ba435`), and that repository is not available on this host.
  The seam must be built against the SAS's description and marked unverified
  until a checkout exists.

## Constraints and Invariants

- **PromptIR is Katana's** (§48.2). OpenWarrant emits a Dispatch, never a prompt.
- **Receipts are consumed, not minted.** A receipt OpenWarrant wrote is not a
  runtime receipt.
- **Taint propagates** (§48.5).
- **Katana may return BLAKE3** (§65.1); the algorithm is always explicit.
