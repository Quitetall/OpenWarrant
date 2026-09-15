---
schema: oh.war/atom/v1
warrant_uuid: 01a021a2-b571-794a-acf0-1559844cf662
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

§98 Phase 3's exit is "no managed normative decision exists only inline."
OW-WAR-0038 delivered §96's importer: the §96.2 mapping table, `HistoricalClaim`
as a type distinct from `Resolution`, and `legacy_declared_unqualified` with no
constructor that skips it.

It has never been run. The LamQuant corpus held 167 ADRs when measured at
`5369da81` on 2026-08-17 and holds more now, and every one of them still exists
only as prose in another repository.

§96.3 is the rule this Warrant exists to honour under pressure: a legacy
`Complete` line with no admissible evidence remains a historical claim, and a
textual gate command is `legacy_declared_unqualified` until it is parsed,
askable, bound, executed, and backed by a Gate Run receipt. Migration is exactly
where that rule is most tempting to bend, because bending it makes the numbers
look better.

## Desired Outcome

The LamQuant ADR corpus imported at a named commit, with every byte
preserved, every gate class preserved, and no completion claim promoted to a
resolution.

## Scope

§96 and §97 in full, §91.4 test 24, and §91.5 tests 30–35.

## Non-goals

- No repair of LamQuant's ADRs. Importing a wrong decision faithfully is
  correct; improving it during import destroys the record.
- No promotion of any historical claim. That needs evidence, which is §98 Phase 6.

## SAS and Roadmap Traceability

- Discharges §98 Phase 3 exit and §99 criteria 7, 8 and 11.
- §96.4's ten preserved classes are already implemented and tested.
