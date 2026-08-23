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
- **Resolved unknown (2026-08-22, by OW-WAR-0041).** OBL-004's §95 disposition
  was blocked on a verb: `UntrackedWork::attach_relation` refuses an empty
  reviewer — the exact control OBL-004 relies on — and no command called it, so
  the refusal had never run.

  `war telemetry --attach <scope> --warrant <alias> --reviewer <who>` now calls
  it, and both directions are planted: no reviewer is refused by
  `RelationFabricated`, and an attachment WITH a reviewer succeeds. The positive
  control is not decoration — a build that refused every attachment would satisfy
  the refusal plant and look like a working review requirement.

  What remains for OBL-004 is a disposition of OW-WAR-0005 M4 through that verb,
  and the reviewer problem is unchanged: this repository has one actor, so the
  review §95 asks for is worth what §27.4 says it is worth. That is now a
  recording decision rather than a missing capability.

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
