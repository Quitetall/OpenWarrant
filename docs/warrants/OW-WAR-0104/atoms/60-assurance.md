---
schema: oh.war/atom/v1
warrant_uuid: 01a0b2a1-a595-7541-b5ae-17edf7d2bf96
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — complete read-only board
- **scope:** CLI fixture and current repository board readers.
- **evidence:** all corpus Warrants and frontier rows retained; approvals equal the
  pending signing list; no fixture bytes change; corrupt question input refuses.

### OBL-002 — safe HTML and web presentation
- **scope:** standalone HTML serializer and reference workbench authenticated API/UI.
- **evidence:** no executable/network HTML content, escaped hostile strings, actual
  HTTP payload parity, authentication/write refusal, browser stage/approval display
  and stale-content removal after failed refresh. Offline-file rendering is a
  separate observation and must not be claimed from serializer tests alone.

### OBL-003 — durable batch questions and integration
- **scope:** adapted war-grill skill and repository integration.
- **evidence:** every queue item keeps its identity and unanswered status; no invented
  human answers. Focused tests, package suite, lint and full gate with exact subjects.
