---
schema: oh.war/atom/v1
warrant_uuid: 01a021a7-8436-7aff-a005-a43eeea25886
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

§98 Phase 8's exit is "Liminal is the single production document semantic
compiler." OW-WAR-0040 delivered §82.3's parity harness and §82.4's cutover gate:
parity must cover the whole corpus, declare its observables in advance, and hold
before cutover is permitted.

The harness has never compared two adapters, because only one exists.

Alongside it sits the oldest unmet claim in the repository. §91.1 test 1 requires
two hosts to produce a byte-identical canonical IR. OW-WAR-0003 and OW-WAR-0005
both record satisfying it with two runs on ONE host, and both say so. That is not
what the test asks.

Going public changed this. CI minutes are now free, and `release.yml` already
carries a matrix of `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin` —
different operating systems and different architectures. The two-host run is
now free and was previously reported as blocked on hardware.

## Desired Outcome

§91.1 test 1 discharged for real on two genuinely different hosts, and
measured parity between the Markdown adapter and a Liminal profile across the
whole compatibility corpus.

## Scope

§82 in full, §91.1 tests 1 and 2, and the `ci.yml` runner-tier decision that going public inverted.

## Non-goals

- No cutover unless parity holds. §82.4 permits it only once qualified.
- No Liminal feature work.

## SAS and Roadmap Traceability

- `WAR-SAS-RQ-061` — Complete; §82 governs.
- Discharges §98 Phase 8 exit, §99 criterion 17, and §91.1 test 1.
