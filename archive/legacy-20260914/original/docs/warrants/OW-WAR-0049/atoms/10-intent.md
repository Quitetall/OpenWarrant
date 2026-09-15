---
schema: oh.war/atom/v1
warrant_uuid: 01a021a7-8437-7b0a-871a-e65c0b221555
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

Alpha closed with three named gaps carried forward and one defect nobody
had noticed.

The named gaps are in the roadmap's "Phase 1 is resolved but not whole" section.
§91.2 test 10 is not implemented. §91.2 tests 11, 13, 14 and 15 are recorded as
explicitly out of scope, and `check.rs` names tests 14 and 15 in a note whose
only job is to say they are unchecked. OW-WAR-0005 M4 records untracked work
under §95, committed against this repository's own obligation.

The defect is worse, because it is a false claim inside a resolved Warrant.
OW-WAR-0005's OBL-002 lists §91.2 test 10 in its scope. The roadmap records the
same test as not implemented. There is no implementation. A resolved Warrant is
therefore claiming coverage it does not have, in the repository whose entire
purpose is preventing exactly that.

## Desired Outcome

The §91.2 residue implemented or explicitly narrowed, the false claim in
OW-WAR-0005 corrected by amendment rather than quietly edited, and OW-WAR-0005 M4's
untracked work dispositioned.

## Scope

§91.2 tests 10, 11, 13, 14, 15; the OW-WAR-0005 M2 and M4 gaps; and §28's amendment path for correcting a resolved Warrant.

## Non-goals

- No silent edit of OW-WAR-0005. §31 says an amendment records what
  changed and why; editing an obligation in place would erase the fact that it
  was wrong.

## SAS and Roadmap Traceability

- Closes the three carried-forward Phase 1 milestones named in the roadmap.
- §91.2 tests 10, 11, 13, 14, 15.
