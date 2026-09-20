---
schema: oh.war/atom/v1
warrant_uuid: 01a01bcf-b0e0-78dd-978f-097d07fe380b
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — relations parse and survive a round trip
- **scope:** ADR atoms in this repository declaring `supersedes` or `superseded_by`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** parse results plus a canonical round trip.

### OBL-002 — cycles and conflicts are REFUSED
- **scope:** a planted supersession cycle, and a planted ADR declaring
- **gate:** `gate://software.repo.war-check@1.0.0`
  `status: accepted` while another supersedes it.
- **evidence:** two plants, two observed refusals naming the specific rule.

### OBL-003 — the untracked work is recorded
- **scope:** the ADR Overview shipped in `3678455`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a Progress Log entry naming the commit and the obligation it
  violated. Not a fix — a record, because §95 forbids fabricating the relation
  that should have existed.

## Gate Adequacy

Not required at `basic` (§25.1). Asked anyway: could OBL-001 and OBL-002 pass while an ADR corpus is still misleading? Yes — nothing here checks that a superseding ADR actually addresses what it supersedes. That is a semantic judgement no parser can make, and it is left to review.

## Residual Risk

Adopting shipped work sets a precedent this project should not repeat. The record in OBL-003 is the mitigation, and it is a weak one.
