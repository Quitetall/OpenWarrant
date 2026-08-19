---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdc-5a1f-7db2-82a1-a1f728931dd3
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §96 (preserve bytes, map semantics, no fabricated proof, preserve unknown classes), §97 (adopt do not replace, typed manifests, mark generated parents, source maps, cutover).

## Prerequisites

OW-WAR-0006, OW-WAR-0019 and OW-WAR-0020 resolved — the gate classes need somewhere typed to land.

## Assumptions and Unknowns

- **Evidenced premise.** The corpus and its gate-class distribution are measured,
  not estimated.
- **Blocking unknown.** LamQuant's ADR corpus is still changing. Importing a
  moving target means the import is a snapshot, and the snapshot's commit must be
  recorded or the import is unreproducible.

## Constraints and Invariants

- **Bytes are preserved** (§96.1). The original body is the authored source.
- **No fabricated proof** (§96.3). A textual `gate_cmd` becomes
  `legacy_declared_unqualified` until parsed, bound, executed, and receipted.
- **All ten classes survive** (§96.4). 'Could not ask' never becomes 'failed' —
  the 23 missing-tool gates must not import as failures.
- **A legacy Complete is a historical claim**, not a resolution.
