---
schema: oh.war/atom/v1
warrant_uuid: 01a021a7-8437-7b0a-871a-e65c0b221555
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

§91.2, §28, §31, §95, and the roadmap's carried-forward milestones.

## Prerequisites

OW-WAR-0002 and OW-WAR-0005 resolved.

## Assumptions and Unknowns

- **Evidenced premise.** The gaps are already written down, with file and
  line references. This Warrant discovers nothing; it closes what was recorded.
- **Blocking unknown.** OBL-004's §95 disposition is not recorded. The machinery
  exists and is unreachable: `LifecycleRelation::attach_relation(warrant,
  reviewer)` refuses an empty reviewer — the exact control OBL-004 says it relies
  on — and no `war` command calls it, so the refusal has never run. *Resolution
  requirement:* a command that attaches a §95 relation, then a disposition of
  OW-WAR-0005 M4 through it. That is a new verb, not a record, which is why it
  did not ride along with the three obligations this Warrant did discharge.

  The harder half is not the verb. §95 wants a REVIEWER, and this repository has
  one actor. Naming that actor is permitted — §95 asks for a recorded reviewer,
  not an independent one — but it produces a disposition whose review is worth
  what §27.4 says it is worth, and that should be visible in the record rather
  than discovered later.

- **Resolved unknown (2026-08-22).** This asked whether test 10's claim in
  OW-WAR-0005 should be narrowed or the feature implemented. **Implemented.** It
  is three rules in `war check`, planted three ways, and the claim is now true
  rather than corrected — so no §30.3 revision was needed, because nothing a
  resolved Warrant claimed was changed. See INF-001.

  Left as a resolved entry rather than deleted, and kept because the question was
  real: narrowing was the cheaper honest option and was rejected on the merits,
  not overlooked.
- **Accepted residual risk.** Tests 14 and 15 concern Source Holder ambiguity and
  classification propagation, which may need federation to test properly.
  *Consequence if false:* they are narrowed again, and the narrowing is recorded
  again, which is tedious but not dishonest.

## Constraints and Invariants

- **A resolved Warrant is amended, not edited** (§28, §31). The
  amendment record carries the semantic diff, the reason, and the authorizer.
- **A completion-claim change is a manual revision** (§30.3) and usually needs an
  ADR.
- **The untracked work stays recorded.** §95 says a relation is not fabricated
  after the fact without review; dispositioning is not the same as erasing.
