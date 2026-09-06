---
schema: oh.war/atom/v1
warrant_uuid: 01a074c1-3eb6-7ac2-a2d2-1fac35901cdd
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance obligations

### OBL-001 - canonical typed effect shape

- **scope:** `EffectRecord` fixtures in `wmd-sim`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** wrong commit phase, payload over 4 KiB, unknown entity, and
  property type/range violations refuse. Tests: `wrong_phase_is_refused`,
  `oversized_payload_is_refused_before_buffering`,
  `unknown_entity_is_rejected_without_event`, and
  `property_type_and_range_violations_are_rejected_without_mutation`.

### OBL-002 - duplicate and atomic refusal

- **scope:** one bounded `EffectBuffer` and atomic groups up to 256 members.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** duplicate canonical sort key refuses before mutation; invalid
  atomic member rejects every group member and publishes no event; 256-member
  groups commit, while 257-member groups reject every member. Tests:
  `duplicate_sort_key_is_refused_before_mutation`,
  `invalid_atomic_member_rejects_entire_group`,
  `maximum_sized_atomic_group_commits_all_members`, and
  `oversized_atomic_group_rejects_every_member`.

### OBL-003 - deterministic commit events

- **scope:** WMD H2 effect fixtures at commit `bc38378`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** performer artifact `wmd-wp008-order-bc38378.log` records the
  insertion-order permutation and boundary tests; full matrix is in
  `wmd-local-ci-bc38378.log`. This is not independent verification.

## Gate adequacy and residual risk

Ordering and refusal plants are required because an always-success commit would
not establish authority. The OpenWarrant gate receipt is corpus-integrity only;
WMD log is performer observation. Independent verification and human
authorization are outstanding. Physics and network effects remain future
packages.
