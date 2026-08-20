---
schema: oh.war/atom/v1
warrant_uuid: 01a01bda-15b7-7767-970e-1e270c168858
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the export is complete
- **scope:** §68.2's five categories, for sections that exist.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a missing populated section fails.

### OBL-002 — the round trip is byte-stable
- **scope:** RQ-083.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** export, import, re-export; bytes identical.

### OBL-003 — history survives
- **scope:** RQ-084.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** superseded, disputed, and annulled records present after export.

## Gate Adequacy

Not required at `basic`. Asked: a byte-stable round trip proves our importer agrees with our exporter, not that a peer implementation would. Cross-implementation preservation is beta.

## Residual Risk

Self-consistency is not interoperability. Only a second implementation proves the format.
