---
schema: oh.war/atom/v1
warrant_uuid: 01a0f423-98b4-72b0-9f94-6e8dda200655
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

- D1: source parser accepts the section-98 phase table used by LAMU and the
  explicit Phase/Objective/Exit table, preserving heading/bullet format.
- D2: corpus status uses the configured program's declared phases regardless of
  count. Missing declarations give no invented title or exit and carry a caveat;
  referenced undeclared phases remain visible with unknown exits.
- D3: public parser and CLI tests with admission and unknown/refusal cases,
  pinned-toolchain checks and live LAMU compilation evidence.
- D4: unverified report, generated views, exact correction requests for the four
  resolved pins, and a separate upstream candidate PR.

## Bounds and rollback

Do not sign, verify independently, resolve, or rewrite existing deliverable pins.
Use an isolated worktree. A rollback reverts this parser/projection candidate;
it changes no source records, signatures or resolution history.
