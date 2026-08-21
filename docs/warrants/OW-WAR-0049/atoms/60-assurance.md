---
schema: oh.war/atom/v1
warrant_uuid: 01a021a7-8437-7b0a-871a-e65c0b221555
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the false claim is corrected by amendment, not by editing
- **scope:** OW-WAR-0005's OBL-002 scope line, which claims §91.2 test 10.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a §31 amendment record naming the semantic diff, the
  reason, the authorizer and the artifact-admissibility decision. The original
  obligation text remains recoverable from history — a correction that erases
  the error is not a correction.

### OBL-002 — the §91.2 residue is implemented or narrowed, with the choice recorded
- **scope:** §91.2 tests 10, 11, 13, 14, 15.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** for each of the five, either a plant against the
  shipped binary, or a recorded narrowing in the owning obligation's scope with
  an ADR. Five outcomes, none of them silence.

### OBL-003 — the negative scope note matches reality
- **scope:** the `check.rs` note naming §91.2 tests 14 and 15 as unchecked.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** if the tests are implemented, the note is removed; if
  narrowed, it cites the narrowing. A note claiming something is unchecked after
  it has been checked is the same class of stale claim as the one this Warrant
  opens with.

### OBL-004 — the untracked work is dispositioned, with a reviewer
- **scope:** OW-WAR-0005 M4 — the ADR Overview shipped with no Warrant.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a §95 disposition naming a reviewer. Attaching the
  relation with an empty reviewer is refused, which is the control being relied
  on here.

## Gate Adequacy

Required at `controlled`, because this Warrant amends a resolved Warrant's
claim and that is the kind of edit that quietly improves a record.

**Adversarial question: could this close the residue while making the corpus less
honest?** Yes, and the mechanism is obvious and tempting. The cheapest way to
resolve every item here is to narrow each claim until it is trivially true and
delete the notes that recorded the gap. Every mechanical check would pass, the
warning count would drop, and the repository would have less information in it
than before.

OBL-001 and OBL-003 are written against precisely that: the amendment must
preserve the original text, and a note may be removed only when the thing it
described has actually changed.

**Executed attacks:** none yet — this Warrant has not been executed.

## Residual Risk

A narrowing recorded as an ADR is still a narrowing. If all five §91.2
residue tests end up narrowed rather than implemented, this Warrant will have
closed cleanly while the conformance suite is no better covered than before. That
outcome is legitimate and should be stated plainly in the resolution's `meaning`
rather than presented as five tests closed.
