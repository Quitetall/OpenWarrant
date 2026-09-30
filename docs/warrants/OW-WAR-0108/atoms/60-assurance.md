---
schema: oh.war/atom/v1
warrant_uuid: 01a0b475-2a0e-7531-bb15-aee5303d9553
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

### OBL-001 — provider retains owned dataset bytes
- **scope:** standard cookbook materialize_dataset_path and declared source identities.
- **evidence:** regression fails before fix; retained bytes survive source deletion;
  invalid input preserves previous output; symlink destination cannot redirect writes;
  provider checks pass at exact repair revision before integration.

### OBL-002 — original graph executes and invalid ports refuse
- **scope:** original OW47 three-record corpus and two-stage PlanSpec, one real local run.
- **evidence:** done status, both artifact receipts, equal expected bytes and external
  lineage reference; mapped port passes and incompatible kind refuses naming the port.

### OBL-003 — integration preserves provenance and history
- **scope:** new OW47 implementation records and this unsigned integration Warrant.
- **evidence:** original failures retained, signed sources unchanged, no lineage graph
  copied, projections/checks/full gate and exact machine-scope evidence pass. Unavailable
  provider checks remain visible and prevent integration completion claims.
