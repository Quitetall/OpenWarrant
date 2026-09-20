---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdc-5a1f-7db2-82a1-a1f728931dd3
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. An importer mapping §96.2's elements to WAR/ADR structures.
2. `legacy_declared_unqualified` for every textual gate command.
3. All ten execution classes preserved.
4. Historical resolution claims typed as claims.
5. The source commit recorded so the import is reproducible.

## Frozen Surfaces

The legacy-class vocabulary. It is what stops the import from laundering unverified claims into verified ones.

## Premade Instructions

- Import the 23 missing-tool gates and assert they land as
  `legacy_declared_unqualified`. That is the single most important test here.
- Record the source commit. An import from 'the corpus' is not reproducible.
- Do not clean up the corpus to make it importable; §96 exists precisely to
  import it as it is.

## Autonomy and Escalation

Tier T1. This decides how 167 historical claims are represented.

## Rollback

Revert. The corpus stays in LamQuant, unimported, which is where it is now.
