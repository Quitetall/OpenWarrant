---
schema: oh.war/atom/v1
warrant_uuid: 01a01bcf-b0e0-78dd-978f-097d07fe380b
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

ADR atoms parse and compile into a generated Overview, but the relations between
decisions do not exist. `AdrRecord` has no `supersedes` field, so nothing records
what replaced what. §21.1 requires supersession to preserve the old record and
mark it non-current; §20.3 forbids a child outcome from becoming the parent's
supposed original rationale. Neither is enforceable today.

This Warrant also regularises a violation. The ADR Overview shipped in commit
`3678455` with NO Warrant authorizing it — untracked work under §95, committed
while OW-WAR-0005's OBL-005 required the next unit of work to open as a Warrant.
That work is adopted here rather than backdated.

## Desired Outcome

ADRs carry typed relations. A superseded ADR keeps its body, loses currency, and
names its successor. A supersession cycle fails closed. The Overview reports the
relation graph rather than a flat list.

## Scope

Typed ADR relations, supersession and deprecation semantics, currency propagation, and adoption of the previously untracked Overview work.

## Non-goals

- No cross-repository ADR resolution; that needs federation (OW-WAR-0029).
- No importer for foreign ADR corpora; that is OW-WAR-0038.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-024` — children do not rewrite parent rationale. Complete.
- `WAR-SAS-RQ-025` — supersession preserves the old record and marks it
  non-current. Complete.
