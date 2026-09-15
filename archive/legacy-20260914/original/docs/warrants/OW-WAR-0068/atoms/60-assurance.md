---
schema: oh.war/atom/v1
warrant_uuid: 01a09482-125b-7703-9b55-354c754ec661
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — shaping skills land records, not chat
- **scope:** war-grill, CONTEXT.md, war-spec.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a recorded run where grilling answers appear in `plan/request.json`, a Dispatch manifest lists the glossary, and a war-spec proposal passes §74.4 steps 1–4.

### OBL-002 — breakdown and review are readable by the tool
- **scope:** war-tickets, `war frontier`, war-review, war-map.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war frontier` output agrees with the milestones graph in plants; a review names both axes; a map Warrant's fog is `blocking_unknown` assumptions `war check` reads.

### OBL-003 — the superset is measured
- **scope:** `war eval` with skill-driven fixture agents; every agent-read document here.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** eval tasks for grill, spec and tickets in the baseline; a plant refuses an em-dash in `.claude/skills/**`.

## Gate Adequacy

Required at `basic`.

**Adversarial question:** can a skill make a Warrant look further along than it
is? Only by signing or by claiming a step; the first is refused by the tool,
the second is what the recorded run exposes.

- **outcome:** gap_accepted

## Residual Risk

- Stolen wording drifts from upstream; each skill credits the commit it came from so a diff is possible.
