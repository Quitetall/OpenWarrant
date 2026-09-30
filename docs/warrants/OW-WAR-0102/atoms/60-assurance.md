---
schema: oh.war/atom/v1
warrant_uuid: 01a0b277-4726-74c0-8e5e-9d4ef0ba32da
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — exact, deterministic scoped report
- **scope:** reference local execution store and configured Warrant inventory.
- **evidence:** real completed attempt report repeats unchanged across restart;
  changed inventory/source changes snapshot and does not count stale completion.

### OBL-002 — no false completion signal
- **scope:** report projection of final/nonfinal records and required checks.
- **evidence:** passed final committed result emits configured word; running,
  unknown, failed, missing revision/check and mismatched check never emit it.
  Unauthenticated request refuses without exposing report.

### OBL-003 — human and offline views
- **scope:** reference browser and self-contained HTML report.
- **evidence:** readable linked report, configurable brevity, pending inventory,
  exact evidence trail and escaped hostile text; direct browser observation.
