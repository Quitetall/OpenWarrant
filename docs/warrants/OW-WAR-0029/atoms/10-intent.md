---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd8-e8d8-707d-b64a-27ffb62e8931
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

Every Warrant here has `enterprise_id = ""`, and `war check` refuses any locally-set value because §91.3 test 20 forbids fabricating one. That refusal is correct and it is also the whole of federation: there is no way to legitimately OBTAIN an identifier, and no cross-repository reference resolution (RQ-005).

## Desired Outcome

A Warrant registers with KF, receives an official enterprise identifier, and keeps Git as Source Holder (RQ-004). Cross-repository relations resolve through federation.

## Scope

Registration, enterprise identifier allocation (§12.4), the federation record (§12.5), offline creation preserved (§12.6), and stable references (§12.7).

## Non-goals

- No transfer of source authority. §12.5 and RQ-004 are explicit that registration adds authority without taking Git's.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-003` — Complete; governing section in Basis.
- `WAR-SAS-RQ-004` — Complete; governing section in Basis.
- `WAR-SAS-RQ-005` — Complete; governing section in Basis.
