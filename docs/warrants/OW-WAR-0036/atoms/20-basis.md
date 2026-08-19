---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdb-2e72-7951-bda1-e484a57f9940
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §19.1 (every normative decision is first class), §19.2 (what is not a new ADR), §19.5 (ADR creation during execution), §74.7 (decision detection).

## Prerequisites

OW-WAR-0006, OW-WAR-0010 and OW-WAR-0034 resolved — §19.2's exclusion depends on the autonomy envelope.

## Assumptions and Unknowns

- **Blocking unknown.** Distinguishing a normative decision from an execution choice is exactly §30.1's local-choice question in a different costume. The two must share one classifier or they will disagree, and a disagreement between them is unresolvable by either.

## Constraints and Invariants

- **§19.2's exclusions are honoured.** A choice already authorized by the autonomy envelope is an execution choice, not a new decision. Over-detection makes the system unusable.
- **Ambiguity escalates**, consistent with §30.4.
- **Detection proposes; it never authors.** A generated ADR is a draft.
