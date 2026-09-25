---
name: war-tickets
description: Break a Warrant's work into tracer-bullet stages with blocking edges in its milestones atom, and read what can start now with `war frontier`. Use when the user says to-tickets, break this down, what can I start, or a Warrant has one stage doing everything.
disable-model-invocation: true
---

# war-tickets

After mattpocock/skills `engineering/to-tickets` (3cca18b, MIT). His tickets
are files on a tracker with a `Blocked by` line; ours are stages in
`45-milestones.yaml`, the blocking edges are `depends_on`, and the frontier
is a command over the same records the resolution reads.

## Vertical slices

Each stage cuts a narrow but complete path through every layer: schema,
command, projection, plant. A completed stage is demoable or verifiable on
its own and fits one fresh context window (`budget_tokens`). Prefactoring
comes first: make the change easy, then make the easy change.

A **wide refactor** (a rename whose blast radius fans across the tree) is not
a slice. Sequence it expand then contract: one stage adds the new form beside
the old; one stage per batch migrates callers; one stage deletes the old
form, blocked by every batch.

## Process

1. Read the Warrant's atoms and `CONTEXT.md`. Use its words in stage titles.
2. Draft the milestones: one milestone per checkpoint with the obligations
   that close it, `depends_on` for the edges, stages under it with
   `executor_kind`, `context_sections`, `budget_tokens`, and for a service
   stage `executor_ref` plus `wall_time_seconds`.
3. Quiz the user with the list: title, blocked by, what it delivers. Ask
   whether the granularity is right, whether each edge genuinely gates, what
   to merge or split. Iterate until approved.
4. Write `45-milestones.yaml`. Then:

```bash
war check <alias>        # milestones.valid: acyclic, no dangling refs
war frontier <alias>     # OPEN is what can start now
```

5. Work the frontier: `war dispatch <alias> <stage>` claims a stage (the
   journal records it); a submission closes it. Never resolve more than the
   frontier allows.

## Never

An unauthorized Warrant's milestones may change freely; an authorized one's
change through an amendment (§28.7). A skill never signs.
