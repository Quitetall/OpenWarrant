---
schema: oh.war/atom/v1
warrant_uuid: 01a074c1-3ebf-7ae3-982e-828776450229
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- SAS v0.1.0-draft.3, sha256 `742dfd066b8df579116ebbd36e19a4b57dc0849bf913c37b716e690e29069a8a`.
- `sas://WAR-SAS-RQ-022`, `WAR-SAS-RQ-045`, `WAR-SAS-RQ-050`, `WAR-SAS-RQ-054`.
- WMD ADR-0002, ADR-0013, ADR-0014, and H2 exit contract in `docs/release-plan.md`.
- WMD package `WP-009-scenario-replay.md`.

## Repository and revision

- Subject repository: `WeaponsOfMageDestruction/wmd` (local checkout).
- Deliverable revision: WMD `67b98c4` (includes WP-009 implementation, CLI proof,
  and replay tests).
- Acceptance command: `./scripts/ci.sh`; performer log `wmd-local-ci-67b98c4.log`
  records 89 tests passing,
  deny checks passing, and `wmd-sim-cli --h2 --seed 7` producing four
  transitions, one event, and pinned v2 checksum.

## Constraints and unknowns

Scenario command streams are bounded and monotonic; `run()` revalidates public
records at the trust boundary. Checksum identity is versioned `v2`; no prior v1
persistence consumer exists in this repository. Human authorization and
independent verification remain unresolved.
