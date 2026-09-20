---
schema: oh.war/atom/v1
warrant_uuid: 01a01bda-15b8-7086-ae76-38561d173119
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §64 (format basis, schema pack), §69 (semantic versioning, additive evolution, breaking change, unknown extensions), §83.4 (generated TypeScript types).

## Prerequisites

OW-WAR-0020 and OW-WAR-0023 resolved — gate and Dispatch schemas are part of the pack.

## Assumptions and Unknowns

- **Evidenced premise.** The Rust types are the single source; generation is mechanical.
- **Blocking unknown.** Which JSON Schema generator, and whether it is stable enough to be a build dependency. §80 makes library selection an ADR when binding, and a schema generator binds the published contract.

## Constraints and Invariants

- **The pack is digested** and pinned by `FormatBasis`.
- **Additive evolution does not bump major** (§69.2); a breaking change does (§69.3).
- **Unknown namespaced extensions survive** (§69.4) — already honoured by the IR and must hold across the pack.
- **Rust is the source of truth**; TypeScript consumes.
