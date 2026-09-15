---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd3-caf4-72ab-919d-d2b2aeba76b8
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing Sources

SAS §37: deliverable definition, artifact provenance, derived reports, performer submission.

## Prerequisites

OW-WAR-0009 resolved — deliverables are contract content.

## Assumptions and Unknowns

- **Evidenced premise.** `artifact_digest` already exists as a domain and the
  digest machinery is proven.
- **Accepted residual risk.** An artifact's digest proves identity, not
  correctness. A wrong artifact digests perfectly.

## Constraints and Invariants

- **A derived report is not the artifact** (§37.3). Conflating them lets a
  summary stand in for the thing summarised, which is the substitution §40.7
  prohibits at the epistemic level.
- **Artifact provenance names its producer** (§37.2).
