---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd5-a42a-766a-916c-656c6b37998b
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §44 (askability, execution status, verdict, required passing result, receipts, shell strings, mutating gates), §45 (gate invalidation).

## Prerequisites

OW-WAR-0019 resolved.

## Assumptions and Unknowns

- **Evidenced premise.** The parent project has a measured corpus of exactly
  these statuses — 43 PASS, 26 FAIL, 12 MISSING-TOOL, 7 MISSING-SCRIPT, 4
  MISSING-CRATE, 1 MUTATING, 1 TIMEOUT — which is a real distribution to design
  against rather than a hypothetical.
- **Accepted residual risk.** A mutating gate (§44.8) changes state while
  measuring it. Detecting mutation reliably is hard; declaring it is easy and is
  what §44.8 asks for.

## Constraints and Invariants

- **Askability precedes verdict** (§44.1). A gate that could not be asked has NO
  verdict field, not a null one.
- **'Could not ask' never becomes 'failed'** (§96.4). The ten statuses stay ten.
- **A required unknown BLOCKS resolution** (RQ-054). Already honoured by the
  diagnostic model; now it must hold for gate results.
- **A mutating gate is quarantined** (§44.8), never run as part of a routine check.
- **Invalidation propagates** (§45, RQ-057): invalidating a gate run invalidates
  the resolutions that rested on it.
