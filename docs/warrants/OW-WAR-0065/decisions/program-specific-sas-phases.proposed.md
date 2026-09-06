# Proposed decision: program-specific SAS phase authority

Status: proposed. Associated draft: OW-WAR-0065. No acceptance, authorization,
verification or resolution is recorded by this document. No ADR alias has been
allocated: the current CLI allocates Warrants but exposes no ADR allocator.

## Context

Liminal's existing phase sequence is -1–12; OpenWarrant's is 0–10. Phase numbers
identify program declarations. A global 0–10 bound, majority namespace, or
fallback OW table cannot govern a different program. Renumbering Liminal to fit
the parser would change the correct document to satisfy an incorrect tool.

## Proposed decision

Store parsed phase numbers as signed 32-bit integers. Require canonical decimal
spellings: zero is `0`, negative one is `-1`, with no plus, padding or negative
zero. Thus `roadmap://LIM-PHASE--1` names Phase -1. Existing nonnegative JSON
numbers and canonical reference strings retain their bytes.

Separate syntax from membership. `war check` compares the reference prefix with
`project.namespace` and the phase with declared phases in the selected SAS.
The current selection rule remains newest accepted revision, otherwise newest
proposed revision; SAS acceptance requests select their requested version and
digest explicitly. Historical Warrant authorization pins continue to hold
compiled contracts immutable; they do not replace the current program catalog.
Exactly one Markdown source must match the selected record's source, SHA-256
and requirement snapshot before its declarations supply authority. Missing,
ambiguous or mismatched authority is unavailable, with no alternate program or
revision fallback. Duplicate or malformed phase/requirement declarations refuse.

An unregistered document or matching proposed revision may be inspected as a
draft. Status labels it draft, does not derive phase achievement, and does not
establish SAS requirement satisfaction. Missing authority supplies no phase
table or requirement-title index. Normative requirements remain separate from
Warrant/evidence-derived completion.

## Consequences and limits

OpenWarrant's eleven declared phases still resolve; Liminal can retain all
fourteen. Consumers of status JSON gain `release.authority` with accepted,
draft, or unavailable state. Existing serialized nonnegative references remain
compatible. This does not define or qualify the compiler adapter, parity
observables, process protocol, or release pins; OW-WAR-0040 remains separate.

Historical accepted revisions remain immutable. If the selected revision's
bytes are unavailable locally, current authority is unavailable even if another
revision happens to parse. An immutable historical-source store is future work.

## Adoption blocker: prior delivered artifacts

The clean base8bc corpus passes `war check --generated`; this candidate introduces
10 `deliverable.digest-drift` refusals. The exact prior and candidate digests
are retained in `../evidence/adoption-blockers.json` and validation output in
`../evidence/VALIDATION.md`. Affected historical deliverables:

| Warrant | Deliverables |
|---|---|
| OW-WAR-0005 | D-001 |
| OW-WAR-0013 | D-001 |
| OW-WAR-0055 | D-001, D-003 |
| OW-WAR-0057 | D-002 |
| OW-WAR-0058 | D-001, D-002 |
| OW-WAR-0062 | D-005 |
| OW-WAR-0063 | D-001, D-003 |

OW-WAR-0064 proposes the missing correction act and explicitly names the
namespace defect addressed here. Human adoption must use that authorized act
when available, preserving old digests and resolutions. This proposal does not
silently refresh records or claim that the aggregate gate passed.
