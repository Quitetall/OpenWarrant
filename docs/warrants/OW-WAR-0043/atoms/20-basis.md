---
schema: oh.war/atom/v1
warrant_uuid: 01a021a2-b571-794a-acf0-1559844cf662
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

§96, §97, §19.2, §21, §91.4 test 24, §91.5 tests 30–35.

## Prerequisites

OW-WAR-0038 and OW-WAR-0006 resolved.

## Assumptions and Unknowns

- **Evidenced premise.** The importer is implemented, and §96.3's five
  promotion conditions are each tested by removal.
- **Blocking unknown.** LamQuant's ADR corpus is still changing — 167 at
  `5369da81`, more today. Importing a moving corpus produces an import of
  nothing in particular.
  *Resolution requirement:* a named LamQuant commit, agreed and frozen for the
  duration of the import.
- **Accepted residual risk.** §96.2's table maps twelve element names. An ADR
  section named something else is reported unmapped, and a human must decide.
  *Consequence if false:* an unmapped section is silently dropped, which §96.1
  forbids.

## Constraints and Invariants

- **Bytes are preserved** (§96.1). Migration adds structure alongside
  the original and never replaces it.
- **No fabricated proof** (§96.3). This is the constraint under the most
  pressure, because a corpus of historical claims looks worse than a corpus of
  resolutions and nobody is watching.
- **An unmapped element is reported, never guessed.**
