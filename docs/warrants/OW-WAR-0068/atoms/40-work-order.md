---
schema: oh.war/atom/v1
warrant_uuid: 01a09482-125b-7703-9b55-354c754ec661
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `.claude/skills/war-grill/SKILL.md`: frontier rounds whose answers land in the draft request (§74.6), then `war plan --draft`.
2. `CONTEXT.md` at the root in Pocock's format, read by the drafter and carried by the Dispatch selector as a glossary; `docs/DEFINITIONS.md` folds into it.
3. `.claude/skills/war-spec/SKILL.md`: conversation to `oh.war/draft-proposal/v2`, no interview.
4. `.claude/skills/war-tickets/SKILL.md` and `war frontier`: spec to milestones with `depends_on`; the command lists open, unblocked stages.
5. `.claude/skills/war-review/SKILL.md`: standards axis (Fowler baseline plus this repo's rules) beside the spec axis (obligations), as separate subagents, never reranked.
6. `.claude/skills/war-map/SKILL.md`: wayfinder as a decision Warrant whose fog is `blocking_unknown` assumptions.
7. Plant-first TDD and handoff folded into the existing `openwarrant` skill's references; writing-for-agents applied to every SKILL.md, AGENTS.md and reference here.
8. `war eval` tasks exercising 1, 3, 4 with a fixture agent; results in the baseline.

## Frozen Surfaces

`oh.war/draft-proposal/v2`, the milestones schema, the report envelope.

## Premade Instructions

- Steal the wording where it is better than ours and credit the file; rewrite
  where a record replaces a tracker.
- A skill ends in an artifact `war check` can read, or it is not done.
- No em-dashes in any document an agent reads.

## Autonomy and Escalation

Tier T2: the owner reviews each skill's first recorded run.

## Rollback

Delete the skill directories; the core is untouched.
