---
schema: oh.war/atom/v1
adr_uuid: e5f4eed0-08f1-4d39-b8e7-4cb413a0ed5b
local_alias: OW-ADR-0008
role: adr
jurisdiction: bound
order: 30
classification: internal
status: accepted
decided: 2026-08-22
governs:
  - "war://OW-WAR-0049"
supersedes:
  - "adr://OW-ADR-0007"
---

# ADR OW-0008: §91.2 tests 11, 13, 14 and 15, dispositioned against the specification text

## Status

`Accepted 2026-08-22`

Supersedes OW-ADR-0007, which narrowed three of these four on a premise that was
false: that the SAS was not in this repository. It is, at
`docs/sas/WAR_Software_Architecture_Specification.md`, tracked, 138 KB.

## Context

§91.2's tests are one line each. Quoted, so the disposition below can be checked
against the source rather than against my summary of it:

> 11. Parent edit is refused or maps unambiguously to an authored atom.
> 13. Bound atom without exact revision fails authorization.
> 14. Source Holder ambiguity fails.
> 15. Higher-classification input raises effective classification.

None is unreadable. Two are implementable. Two are blocked, and on narrower
grounds than OW-ADR-0007 claimed.

## Decision

**Test 11 — IMPLEMENTABLE, not narrowed.** A parent is cited in a manifest with a
`contract_revision`; editing one must either be refused or resolve to a specific
authored atom. `ParentWithoutRevision` already enforces the citation half. The
remaining half is a rule about edits, and it is in scope for a Warrant that owns
the parent-child surface — OW-WAR-0043, not this one. Carried there rather than
narrowed here.

**Test 13 — SCHEMA-BLOCKED. The narrowing stands, and its reason was never the
missing text.** OW-WAR-0002's Intent already quoted this test correctly, so this
one was read all along. `AtomEntry` carries `ordinal`, `role`, `path`, `ref` and
`required`, and no revision field, so a bound atom cannot express an exact
revision and a rule refusing one that lacks it would refuse every bound atom that
could be written. §20.2's sibling rule for PARENTS is enforced
(`ParentWithoutRevision`); the atom-level one has nothing to check.
*Resolution requirement:* a revision on `AtomEntry` — a manifest schema change.

**Test 14 — BLOCKED ON A MISSING FIELD, not on a missing decision.** §13 is
explicit: *"Every atom or bound record SHALL declare its Source Holder."* Our
atoms declare none. `source_holder` exists in the model on `Deliverable` and
nowhere near an atom. So "Source Holder ambiguity fails" cannot fail today
because no atom asserts a holder to be ambiguous about. OW-ADR-0007 called this
undecided; it is undeclared, which is a different and cheaper problem.
*Resolution requirement:* a `source_holder` on the atom frontmatter and the §13
holder-kind vocabulary, then the ambiguity rule.

**Test 15 — BLOCKED ON AN UNSPECIFIED ORDERING, which is the one thing
OW-ADR-0007 got roughly right.** The rule is unambiguous: a higher-classification
input raises the effective classification, and `effective_classification` appears
in the SAS as a field. What the SAS does not give is the LATTICE — which values
outrank which. Every atom in this corpus says `internal`, so there is no pair to
order and no way to observe the rule working even if it were written.
*Resolution requirement:* a classification ordering, then a corpus containing
more than one value.

## Rationale

The correction matters more than the dispositions. Three of these four were
narrowed because I concluded from two failed globs — `docs/*SAS*` and
`docs/spec*` — that the specification was absent, and wrote that into an accepted
decision. The file is `docs/sas/`. A directory listing would have found it.

The failure mode is worth naming because this repository exists to catch it:
**absence of evidence was recorded as evidence of absence, and then cited.** The
search was the weak step, and nothing in the resulting ADR exposed that it rested
on a search rather than on a reading.

## Consequences

Test 11 moves to OW-WAR-0043 as implementable work rather than staying narrowed.
Tests 13, 14 and 15 remain narrowed with resolution requirements that name a
field, a field, and an ordering — all cheaper than "nobody has decided."

`docs/sas/` is now referenced from the README so the next search does not have to
be a lucky one.

## Validation

Each quoted line above can be checked against
`docs/sas/WAR_Software_Architecture_Specification.md` §91.2, which is the point:
this ADR's premises are verifiable from a file in the repository, which its
predecessor's were not.
