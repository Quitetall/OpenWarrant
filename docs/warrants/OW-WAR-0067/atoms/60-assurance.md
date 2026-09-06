---
schema: oh.war/atom/v1
warrant_uuid: 01a074c1-3ebf-7ae3-982e-828776450229
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance obligations

### OBL-001 - bounded command admission

- **scope:** public `Scenario` values and `Scenario::new` over H2 commands.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** empty, over-limit, non-monotonic, and public-field bypass
  scenarios refuse at runner boundary.

### OBL-002 - H2 vertical path

- **scope:** one synthetic entity and one capability/action/effect definition.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** property resolves, capability derives, action transitions,
  effect commits, and one domain event publishes.

### OBL-003 - replay identity

- **scope:** checksum domain `wmd-h2-scenario-v2`, seed 7 golden fixture.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** repeated run has identical events/checksum and pinned checksum
  `2e0a23743bd8554435372b2d7b8f153ddb5487c903c81843b109fe15d0e81b54`.

### OBL-004 - smoke and divergence boundary

- **scope:** 2,048 seeds and stream-byte first-difference comparator.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** 2,048-seed smoke passes; full IDs and effect sort fields enter
  divergence bytes; no v1 persistence consumer exists in subject repository.

## Gate adequacy and residual risk

The gate must test malformed input and replay identity, not only successful
output. Local receipts are performer observations; independent verification and
human authorization are outstanding. Full envelope persistence and migrations
remain H6 scope.
