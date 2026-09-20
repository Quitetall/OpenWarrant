---
schema: oh.war/atom/v1
warrant_uuid: 01a06446-1e04-7e93-99af-04ad037dbc46
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS v0.1.0-draft.1, sha256 `aad5256cb59e3e589313b7e2d5b48360ad8c85cf1c1d65d21f9260e692dfe8e5`.
- §98 Phase 1 (ten deliverables; Exit "OpenWarrant development uses
  WARs"); §34 traceability to the Roadmap; §59.2 committed generated views.
- RQ-022.

## Measured on 2026-09-02

- Phase 1 members: 17 Warrants, none with the `exit` slug; the projection
  reports `not derivable`.
- The ten §98 deliverables: `war init`, `war new`, `war check`, `war
  compile` are commands; the manifest, authored atom profile, canonical
  IR, full Markdown parent and canonical JSON are `openwarrant-core` /
  `openwarrant-compiler` modules with committed output for 56 Warrants;
  the generated drift gate is the `corpus` step of `cargo xtask gate`
  (`war check --generated`), green in CI on every push since #41.
- Warrants in the corpus: 56. Authorized: 54 (OW-WAR-0059 and 0060 await
  the owner). Resolved through §56 with a bound receipt: 2 (0010, 0020).
- Commit subjects on `main` naming a Warrant: 20 of 87. The rest name a
  SAS section, a fix, a records batch or a dependency bump. Recorded as
  the starting number, not as a pass.

## Assumptions carried in

- "Uses WARs" is read as: the corpus exists, compiles, is authorized, and
  can close — not as "every commit cites one". The second reading is a
  stricter rule this repository has not adopted; the number is recorded so
  adopting it later starts from a measurement.
- A resolution of this Warrant is a resolution of the Exit. §98 defines no
  other discharge.
