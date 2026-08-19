---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdb-2e72-7530-adc8-260579bdad3b
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — diff is semantic, not textual
- **scope:** two revisions differing only in rendering.
- **evidence:** the diff reports NO semantic change.

### OBL-002 — digest movement is attributed
- **scope:** a revision whose contract digest moved.
- **evidence:** the causing field is named.

## Gate Adequacy

Not required at `basic`. Asked: a complete diff can still be unreadable, and an unreadable diff gets skipped, which is the same as not having one.

## Residual Risk

Readability is not testable and is the actual success criterion here.
