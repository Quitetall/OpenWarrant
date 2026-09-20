---
schema: oh.war/atom/v1
adr_uuid: 2156d5d9-8674-4d79-a58a-280287c558d6
local_alias: OW-ADR-0007
role: adr
jurisdiction: bound
order: 30
classification: internal
status: falsified
decided: 2026-08-22
governs:
  - "war://OW-WAR-0049"
---

# ADR OW-0007: §91.2 tests 11, 13, 14 and 15 are narrowed, each for a different reason

## Status

`FALSIFIED 2026-08-22`, the same day it was accepted. Superseded by OW-ADR-0008.

## Why this is falsified rather than superseded

Its central premise was false. This ADR narrowed §91.2 test 11 as "UNREADABLE —
its definition does not exist in this repository", and narrowed tests 14 and 15
as blocked on decisions nobody had made.

**The specification is in this repository.** `docs/sas/WAR_Software_Architecture_Specification.md`,
138 KB, tracked by git, containing §91.2 in full. Test 11 reads: *"Parent edit is
refused or maps unambiguously to an authored atom."* Tests 14 and 15 read
*"Source Holder ambiguity fails"* and *"Higher-classification input raises
effective classification."*

I searched `docs/*SAS*` and `docs/spec*` and concluded from two failed globs that
the file did not exist, then wrote that conclusion into an accepted decision
governing a Warrant whose entire subject is claims that are not true. `ls docs/`
would have shown it.

Kept in full, and marked at the top rather than edited, because a decision made
on a false premise is worth more as a record than as a gap. OW-ADR-0008 restates
the dispositions against the text.

Governs OW-WAR-0049. Its OBL-002 requires each of §91.2 tests 10, 11, 13, 14 and
15 to end in "either a plant against the shipped binary, or a recorded narrowing
in the owning obligation's scope with an ADR. Five outcomes, none of them
silence." Test 10 got the plant. This records the other four.

## Context

Alpha carried these forward as "explicitly out of scope" without saying why any
of them was out of scope. Four tests behind one phrase is the shape a stale
claim takes: it reads as a decision and is actually an absence.

They are not one problem. Grouping them under a single narrowing would repeat
the mistake at a smaller scale, so each is stated separately with what would
have to change for it to be implementable.

## Decision

**Test 10 is NOT narrowed.** It is implemented and planted three ways —
`atom.generated-as-source`, `atom.unknown-jurisdiction` and
`atom.jurisdiction-mismatch`. OW-WAR-0005's OBL-002 claim about it is now true
rather than corrected, which is why no §31 amendment accompanies this ADR.

**Test 11 is narrowed as UNREADABLE.** Its definition does not exist in this
repository. The SAS is not vendored here, and no Warrant, ADR, roadmap entry or
comment states what test 11 requires — the only mentions are lists that name the
number. A test whose obligation cannot be read cannot be implemented, and cannot
honestly be narrowed on its merits either. *Resolution requirement:* the §91.2
text, at which point this narrowing is void.

This is recorded as the finding it is. An enumerated scope that includes an item
nobody can look up is not enumerated in the sense OW-WAR-0005's OBL-002 meant by
"enumerated, so the set cannot quietly shrink".

**Test 13 is narrowed as SCHEMA-BLOCKED.** "A bound atom without an exact
revision must FAIL authorization." `AtomEntry` carries `ordinal`, `role`,
`path`, `ref` and `required` — and no revision field at all. So a bound atom
cannot express an exact revision, and a rule refusing one that lacks it would
refuse every bound atom that could ever be written. §20.2's sibling rule for
PARENTS is enforced (`ParentWithoutRevision`); the atom-level one has no field
to check. *Resolution requirement:* a revision on `AtomEntry`, which is a
manifest schema change and outside a Warrant whose subject is closing residue.

**Tests 14 and 15 are narrowed as UNDERSPECIFIED HERE.** Source Holder ambiguity
(14) and classification propagation (15) both need a rule this repository has not
decided: which holder wins when two claim the same source, and what a
composition's classification becomes when its atoms disagree. `check.rs` has
carried a note naming them as unchecked since alpha, and that note is honest —
it is kept, now citing this ADR. *Resolution requirement:* a decision on each
question, not more code against an undecided one.

## Rationale

The alternative was to implement something for each and call the tests covered.
That is the substitution §40.7 forbids: a cheaper measurement standing in for the
one required. A rule written against an unread specification (11), a missing
field (13), or an undecided question (14, 15) would pass its own tests and
certify nothing.

Narrowing costs a visible admission. Implementing-something-anyway costs a claim
that looks like coverage, and this repository exists because that trade keeps
being made the wrong way.

## Consequences

OW-WAR-0005's OBL-002 scope stands unchanged: it enumerates §91.2 items 7, 8, 9,
10, 12 and 16, and every one of those is now implemented. Tests 11, 13, 14 and 15
were never in that obligation's scope — they are residue OW-WAR-0049 adopted, and
this ADR is where they stop being silent.

Three of the four narrowings name a concrete unblocking condition. Test 11's is
the cheapest and the most embarrassing: read the specification.

## Validation

`war check --generated` resolves this ADR and the plants for test 10 run in
`cargo xtask gate`. The narrowings themselves are records, not code, and are
validated by being cited from OW-WAR-0049's assurance atom rather than by a
gate — a narrowing that a gate could verify would be an implementation.
