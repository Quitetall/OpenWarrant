---
schema: oh.war/atom/v1
warrant_uuid: 01a021a4-be74-73ce-8f9d-105d80ab82fc
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

§47, §48, §50, §51, §52, §53, §91.7 tests 43–51, §91.9 tests 59–63.

## Prerequisites

OW-WAR-0023, OW-WAR-0024, OW-WAR-0025 and OW-WAR-0026 resolved.

## Assumptions and Unknowns

- **Evidenced premise.** The Dispatch compiler, the receipt schema and
  the capability comparison are implemented and unit-tested.
- **Blocking unknown.** Katana is not checked out on this host. §75.1 names it
  the preferred adapter and §48.1 requires a versioned protocol or subprocess
  adapter; neither exists here.
  *Resolution requirement:* a Katana checkout on the development host, built, and
  able to accept a Dispatch and return a §48.4 receipt.
- **Accepted residual risk.** "Stateless" is asserted by construction — the
  Dispatch is the only packet we SEND. An agent with its own memory of this
  repository is not stateless, and we cannot prove it does not have one.
  *Consequence if false:* the agent succeeds using knowledge the Dispatch did not
  carry, and the Dispatch looks sufficient when it is not.

## Constraints and Invariants

- **The Dispatch is the only packet** (§47). Anything the agent needed
  and did not receive is a defect in the compiler, not a hint to pass separately.
- **No self-completion** (§51.2). The type cannot express `resolve`; a text
  request for it is refused by name.
- **Realizing more capability than authorized is a containment failure** (§48.3),
  not a detail.
