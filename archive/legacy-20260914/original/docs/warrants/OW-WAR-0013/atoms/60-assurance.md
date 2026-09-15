---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd2-7d30-792c-ba72-6b74daa59f19
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — references resolve
- **scope:** every `sas://` and `roadmap://` ref in this repository.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** resolution against §106 and the Production Roadmap.

### OBL-002 — an unresolvable reference is REFUSED
- **scope:** a `sas://WAR-SAS-RQ-999`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant and its refusal.

### OBL-003 — requirements with no Warrant are reported
- **scope:** all 57 requirements in §106.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the Overview lists the uncovered ones by identifier.

## Gate Adequacy

Not required at `basic`. Asked: resolution proves a requirement exists, not that the Warrant addresses it. A Warrant could claim RQ-050 and do nothing about it, and this check would pass. Only gates close that gap.

## Residual Risk

Parsing requirement identifiers out of a prose specification is brittle to reformatting. If §106's table shape changes, resolution breaks — loudly, which is the right direction.
