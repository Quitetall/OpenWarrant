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
- **Blocking unknown.** Whether test 10's claim in OW-WAR-0005 should be narrowed
  or the feature implemented. Narrowing is honest and cheaper; implementing keeps
  the original promise.
  *Resolution requirement:* a decision recorded as an ADR, since it changes what
  a resolved Warrant claimed and §30.3 makes a completion-claim change a manual
  revision.
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
