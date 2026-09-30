---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdb-2e72-740b-bb94-32bbdec1cc35
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §71.3 (`war plan`), §71.4 (`war interview`), §74.6 (interview generation), Appendix C.

## Prerequisites

OW-WAR-0034 resolved.

## Assumptions and Unknowns

- **Evidenced premise.** Appendix C gives the interaction shape.
- **Blocking unknown.** Question quality is the whole value and cannot be unit-tested. §99 criterion 2 says 'only unresolved high-value questions' — measuring that needs the telemetry of OW-WAR-0039 (clarification counts), so the two are linked.

## Constraints and Invariants

- **A draft is a draft.** `war plan` never authorizes.
- **Every proposal is validated before writing** (§74.4), inherited from OW-WAR-0034.
- **Questions are about unresolved decisions**, not about restating what the request already said.
