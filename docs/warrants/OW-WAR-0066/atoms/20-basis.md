---
schema: oh.war/atom/v1
warrant_uuid: 01a074c1-3eb6-7ac2-a2d2-1fac35901cdd
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- SAS v0.1.0-draft.3, sha256 `742dfd066b8df579116ebbd36e19a4b57dc0849bf913c37b716e690e29069a8a`.
- `sas://WAR-SAS-RQ-022`, `WAR-SAS-RQ-043`, `WAR-SAS-RQ-051`.
- WMD ADR-0002 and ADR-0013; WMD package `WP-008-effects-events.md`.

## Repository and revision

- Subject repository: `/home/brianklam/Desktop/WeaponsOfMageDestruction/wmd`.
- Deliverable revision: WMD `765ab4b` (includes WP-008 implementation and H2
  integration tests).
- Acceptance command: `./scripts/ci.sh`; observed locally with 88 tests passing,
  deny checks passing, and deterministic CLI checksum retained.

## Constraints and unknowns

Commit accepts only typed bounded payloads at phase ordinal 6. Atomic groups
reject as a whole; only successful effects publish events. Human authorization
and independent verification are not supplied by this performer and remain
unresolved.
