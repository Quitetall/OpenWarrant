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

## Evidence

§40's records for what this Warrant has done. The first one is the reason it
exists.

### EV-001 — the §91.2 test 10 plants
- **class:** evidence
- **kind:** gate_run_output
- **origin:** gate_runner
- **admissibility:** controlled_measurement
- **digest:** sha256:pending-receipt-binding
- **method:** conformance/plant.sh — three plants, one per sub-rule:
  `atom.generated-as-source`, `atom.unknown-jurisdiction`,
  `atom.jurisdiction-mismatch`
- **occurred at:** 2026-08-22

### EV-002 — the misclassified ADR atoms
- **class:** evidence
- **kind:** static_analysis
- **origin:** verifier
- **admissibility:** independent
- **digest:** sha256:pending-receipt-binding
- **method:** the test 10 implementation, run against the corpus for the first
  time; the finding is the checker's output, not a search made to confirm it
- **occurred at:** 2026-08-22

### OBS-001 — all six ADR atoms claimed they could be written by a binding Warrant
- **class:** observation
- **evidence:** EV-002
- **method:** §16.1 places the `adr` role under `bound`; every ADR atom declared
  `authored`, and the scaffold fixture in `adr.rs` emitted `authored` too, so the
  misclassification reproduced on every new decision
- **admissibility:** independent

### OBS-002 — test 10 is implemented, not narrowed
- **class:** observation
- **evidence:** EV-001
- **method:** three plants against the shipped binary, each asserting an exit
  code, a named rule and a named detail
- **admissibility:** controlled_measurement

### INF-001 — the claim in OW-WAR-0005 is now true, so there is nothing to amend
- **class:** inference
- **kind:** deductive
- **premises:** OBS-002
- **claim:** ow-war-0005-obl-002
- **reasoning:** OBL-001 asks for the false claim to be corrected by amendment
  rather than by editing. A §31 amendment records a semantic diff — a change to
  what an obligation says. Nothing about OW-WAR-0005's OBL-002 changed: it
  enumerates §91.2 items 7, 8, 9, 10, 12 and 16, and item 10 is now implemented.
  The claim was false because the implementation was missing, and supplying the
  implementation is the stronger correction. Amending the scope to remove 10
  would have made the record true by shrinking what it promised.
- **admissibility:** controlled_measurement

### JDG-001 — four tests are narrowed, each for a different reason
- **class:** judgment
- **kind:** scope_narrowing
- **actor:** QuiteTall
- **acting role:** author
- **meaning:** §91.2 tests 11, 13, 14 and 15 are narrowed by OW-ADR-0007. Test 11
  is unreadable — its definition is in no file in this repository. Test 13 is
  schema-blocked — `AtomEntry` has no revision field to check. Tests 14 and 15
  are blocked on a decision nobody has made. Implementing something against an
  unread specification, a missing field or an undecided question would produce a
  rule that passes its own tests and certifies nothing.
- **basis:** OBS-002, INF-001
- **authority:** authorized
- **limitations:** one actor, so this judgment is not independently reviewed —
  §27.4 says role separation by one person is not organizational independence

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
