---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd1-b0d3-7be9-a602-9d2144de440d
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §30 (autonomy envelope), §31 (amendment record), §26 (responsibility tiers).

## Prerequisites

OW-WAR-0009 resolved — an amendment is a transition between contract revisions.

## Assumptions and Unknowns

- **Evidenced premise.** §30 enumerates the three classes and §30.4 specifies
  ambiguity behaviour, so this is enforcement of a stated rule.
- **Accepted residual risk.** Classification depends on reading a change's intent.
  A mechanical classifier will be conservative and will escalate things a human
  would have waved through.

## Constraints and Invariants

- **Ambiguity escalates** (§30.4). An unclassifiable change is NOT local by
  default; defaulting to local would make the envelope decorative.
- **An auto-authorized revision is still a revision** — it creates a new one
  (RQ-033), it does not edit in place.
