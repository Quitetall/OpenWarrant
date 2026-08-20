---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd3-caf4-7b46-8ed4-7e9634593bec
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the rationale graph parses
- **scope:** §35's node classes and edge types.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** parsed values asserted per class.

### OBL-002 — blocking unknowns and circular validation are REFUSED
- **scope:** a Warrant authorized with an unresolved blocking unknown; a premise
- **gate:** `gate://software.repo.war-check@1.0.0`
  whose support chain returns to itself.
- **evidence:** two plants, two refusals naming their rules.

## Gate Adequacy

Not required at `basic`. Asked: a rationale graph can be complete, acyclic, and wrong. Structure is not soundness, and no tool supplies soundness.

## Residual Risk

Authoring burden. If structured rationale is onerous, authors will write the minimum that parses, and a graph written to satisfy a parser carries less than the prose it replaced.
