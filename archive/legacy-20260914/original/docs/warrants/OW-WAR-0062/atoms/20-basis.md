---
schema: oh.war/atom/v1
warrant_uuid: 01a064fc-a6f2-7c43-bed9-fa248b771712
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS v0.1.0-draft.2, sha256 `daa1c89a3ca01cb7d2a219f8d08a85ec36250094d15324cd79248019a4621d9e`,
  the revision this Warrant amends into 0.1.0-draft.3.
- §4 Naming; §6 System hierarchy (6.1–6.9); §34 traceability; §98 phases;
  §101 governance; §106 requirement index.
- RQ-022.

## Measured on 2026-09-03

- §6 names nine levels and defines each in one to three sentences. The
  words "same", "kind" and "class" do not occur in §6. No sentence says when
  to write a SAS and when to write a Warrant.
- The README's status section stated "49 Warrants, 40 of them resolved" and
  "Alpha complete"; the corpus carried 20 resolution records, all written
  in the preceding two days, and none on the day the README was written.
- `docs/` had no definitions or glossary page.
- `war new`'s manifest template began with the schema line and carried no
  guidance about what a Warrant is for.

## Assumptions carried in

- §6.10 is a definitions subsection; it adds no §106 row and removes none,
  so 0.1.0-draft.3 is not architecture-changing under §101.3 and needs no
  ADR. If the owner reads "same class of artifact" as an architectural
  claim, an ADR should accompany the next revision.
- The level-for-level correspondence table is descriptive: it says which
  part of a SAS plays the role of which part of a Warrant. It does not make
  the SAS parse as a Warrant.
