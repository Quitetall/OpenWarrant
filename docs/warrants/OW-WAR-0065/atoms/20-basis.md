---
schema: oh.war/atom/v1
warrant_uuid: 01a074c1-3eac-7bf2-92ec-9fdddfa15045
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- SAS v0.1.0-draft.3, sha256 `742dfd066b8df579116ebbd36e19a4b57dc0849bf913c37b716e690e29069a8a`.
- `sas://WAR-SAS-RQ-022`, `WAR-SAS-RQ-044`, `WAR-SAS-RQ-051`.
- WMD ADR-0002 and ADR-0011; WMD package `WP-007-action-timelines.md`.

## Repository and revision

- Subject repository: `WeaponsOfMageDestruction/wmd` (local checkout).
- Deliverable revision: WMD `67b98c4` (includes WP-007 implementation and H2
  integration tests).
- Acceptance command: `./scripts/ci.sh`; performer log `wmd-local-ci-67b98c4.log`
  records 89 tests passing,
  deny checks passing, and deterministic CLI checksum retained.

## Constraints and unknowns

Only stable semantic IDs and `MasterTick` may enter action state. Capability
records are derived before admission. Human authorization and independent
verification are not supplied by this performer and remain unresolved.
