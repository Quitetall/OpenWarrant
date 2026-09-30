---
schema: oh.war/atom/v1
warrant_uuid: 01a021a7-8436-7aff-a005-a43eeea25886
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

§82, §91.1 tests 1 and 2, §98 Phase 8.

## Prerequisites

OW-WAR-0040 and OW-WAR-0003 resolved.

## Assumptions and Unknowns

- **Evidenced premise.** The two-host run needs no new hardware. The
  matrix already exists in `release.yml` with two distinct target triples, and
  public repositories get free minutes.
- **Blocking unknown.** No Liminal compiler profile exists to compare against.
  Liminal is a repository under the same owner but is not checked out here, and
  §82.2 requires a pinned compiler invoked over a versioned process protocol.
  *Resolution requirement:* a Liminal checkout that accepts
  `--protocol oh.war/liminal-v1` and compiles the compatibility corpus.
- **Accepted residual risk.** Two hosts is two. Byte-identical output on Linux
  x86-64 and Darwin arm64 does not prove determinism everywhere.
  *Consequence if false:* a third platform diverges and the canonical form was
  never canonical.

## Constraints and Invariants

- **Parity is measured across the whole corpus** (§82.3). A sample
  reads exactly like a full run and asserts far less.
- **Observables are declared in advance.** Choosing them afterwards is choosing
  the ones that matched.
- **`ci.yml`'s runner decision has inverted.** Its own comment says that if the
  repository is ever public, a self-hosted runner becomes a security problem
  because a fork pull request executes attacker code on the workstation. That
  condition has fired and the comment is now stale guidance.
