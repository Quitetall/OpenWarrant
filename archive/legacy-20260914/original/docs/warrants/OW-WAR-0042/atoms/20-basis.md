---
schema: oh.war/atom/v1
warrant_uuid: 01a021a2-b570-7f57-85b2-0f8189873d9e
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

§74 in full, §75, §71.3, §71.4, and §91.8 tests 52–58.

## Prerequisites

OW-WAR-0034 and OW-WAR-0035 resolved.

## Assumptions and Unknowns

- **Evidenced premise.** The protocol is implemented on both sides and
  §74.4's gauntlet refuses to apply while any pre-write step is unrun.
- **Blocking unknown.** No drafting agent speaks
  `oh.war/agent-drafter/v1` today. Katana is the preferred adapter (§75.1) and
  is not cloned on this host.
  *Resolution requirement:* a process reachable on this machine that accepts the
  canonical request on stdin and returns a canonical Draft Proposal on stdout,
  named and version-pinned in the receipt.
- **Accepted residual risk.** §74.8's prohibition on fabricating a source is
  enforced against carelessness, not intent — a well-formed citation to a
  document that does not exist parses fine.
  *Consequence if false:* a plan cites evidence nobody has, and the reader
  believes it because it is well formed.

## Constraints and Invariants

- **The model never writes a file** (§74.5). Its whole effect surface
  is §74.3's seven operations, and anything else is an arbitrary write renamed.
- **A durable choice becomes a proposed ADR** (§74.7), never a Work Order
  paragraph.
- **An unreviewed proposal is not applicable** (§74.4 steps 5 and 6). Reporting
  that state as an error would train people to pass `--reviewed` to silence it.
