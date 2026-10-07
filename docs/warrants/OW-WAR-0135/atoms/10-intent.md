---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-602f-7321-ad70-864c543b2927
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The product spec promises an optional assurance mark and leaves most of it
open:

- decided (Q43–Q47, Q49):
  - the mark covers one accepted Warrant result, tied to its exact code
    revision and stated scope; never a whole release or repository (Q44);
  - OpenWarrant defines a versioned baseline; a repository may strengthen
    it but not weaken it and keep the same mark (Q45);
  - a human reviews the outcome, verification findings and remaining risks
    and explicitly accepts the exact result; independent verification
    supplies the technical review; the human act does not claim every line
    was read (Q46);
  - "implementation finished, unreviewed" carries no mark (Q47);
  - later qualification keeps the real history; a practice not followed
    stays unmet (Q43);
  - the default policy requires the mark before merging into main (Q49).
- open ("Engineering contracts still to specify"): the baseline's exact
  requirements and the evidence for each, mark issuance and validation,
  what the mark binds, and Q8–Q9 delegation under the mark.

Nothing in the code implements a mark. SAS 1.1.0 does not mention one. The
parts a mark would rest on do exist:
- a human-signed resolution with an attestation (all 29 recorded
  resolutions carry `attestations/resolve-1.dsse.json`);
- §46 independent verification, with independence judged per verdict;
- the resolution's `locator` naming the commit.

So a mark could be computed today by anyone, meaning anything. That is the
gap: no baseline, no definition of issuance, no binding.

## Desired Outcome

- **M1, a decision.** One ADR, accepted by the owner, that fixes:
  - baseline v1: each requirement, and the record that evidences it;
  - issuance: whether a mark is a new signed act or a derived statement
    over records already signed (Q-001);
  - binding: what digests a mark names, so it can be re-verified later;
  - how a repository strengthens the baseline without weakening it.
- **M2, after M1 is accepted.** `war mark <alias>` evaluates a resolved
  Warrant against a named baseline version:
  - every requirement met: an `oh.war/mark/v1` statement naming the
    baseline version, the resolution, its attestation, the commit and the
    obligations in scope;
  - any requirement unmet or `UNKNOWN`: no mark, and each unmet
    requirement by name.
  - `war mark <alias> --verify` re-checks a mark's bindings against the
    tree today.
- The mark is never a substitute for the human act. If Q-001 selects a
  derived mark, it can only exist over a human-signed resolution.

## Non-goals

- Enforcing "mark before merge" (Q49) in CI. This Warrant makes the mark
  computable. Enforcing it is repository policy, and OW-WAR-0134's CI step
  is where it would go.
- Q8–Q9 delegation semantics. Named in the ADR as out of v1; no delegated
  act earns the mark.
- A SAS revision adding an RQ for the mark. Proposed as a follow-up if the
  owner wants the mark normative beyond this repository.
- Marking releases or repositories (Q44 rules that out).
- Retroactive marks that claim a process history the records do not show
  (Q43).
