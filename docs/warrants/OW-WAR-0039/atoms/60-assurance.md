---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdc-5a1f-771a-a8ed-983d14257cfe
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the measurement set exists
- **scope:** §94's list.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** each measurement is recorded for a real operation.

### OBL-002 — detection finds the known case
- **scope:** commit `3678455`, the ADR Overview shipped with no Warrant.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the detector reports it. A detector that misses the one case we
  already know about has not been shown to work.

### OBL-003 — no relation is fabricated
- **scope:** §95.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant attempting auto-attachment, refused.

## Gate Adequacy

Not required at `basic`. Asked: detection finds commits with no WAR identity, so work done with a plausible-looking but wrong identity passes. Identity is asserted by the committer.

## Residual Risk

Human authoring cost is approximate. Treating it as precise would be its own false claim.
