---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdc-5a1f-7db2-82a1-a1f728931dd3
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The LamQuant repository holds 167 ADRs that OpenWarrant is meant to succeed.
Importing them is the entire point of Phase 3, and it is the most dangerous
operation in the roadmap: §96.3 forbids fabricating proof, and the corpus is full
of exactly the claims that would tempt an importer to fabricate.

Measured in that corpus: 51 accepted or in-progress ADRs with no `gate_cmd`;
of the 94 that declare one, 43 pass and 51 do not, with 23 naming a tool, script,
or crate absent from the tree. Those 23 must import as `legacy_declared_unqualified`
(§96.3), never as gates.

## Desired Outcome

The 167 ADRs import as authored source revisions with their bytes preserved. Every gate class survives. A legacy `Complete` line with no admissible evidence imports as a historical claim, not as a WAR resolution.

## Scope

Byte preservation (§96.1), semantic mapping (§96.2), no fabricated proof (§96.3), unknown-class preservation (§96.4), and the §97 atom/parent migration including cutover.

## Non-goals

- No re-verification. The importer records what the corpus says, not whether it is true.
- No deletion. §97.1 is adopt, do not replace.

## SAS and Roadmap Traceability

No §106 requirement maps here directly; enabling work named in the Production Roadmap.
