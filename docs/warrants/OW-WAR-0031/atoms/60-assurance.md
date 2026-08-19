---
schema: oh.war/atom/v1
warrant_uuid: 01a01bda-15b8-7209-89bb-f36dde06b52c
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — material events are journalled
- **scope:** §66.4.
- **evidence:** each material event produces an entry.

### OBL-002 — append-only
- **scope:** §66.
- **evidence:** a plant attempting rewrite, refused.

### OBL-003 — state stops being derived
- **scope:** OW-WAR-0008 OBL-003.
- **evidence:** the Overview reports recorded state for journalled Warrants.

## Gate Adequacy

Not required at `basic`. Asked: a local file is editable outside the tool, so append-only binds the API, not the filesystem. §66.2's non-authority is the honest framing.

## Residual Risk

Filesystem-level tamperability, inherent to a local journal and acknowledged by the SAS.
