---
name: war-map
description: Plan a chunk of work too big for one session as a decision Warrant whose fog is blocking unknowns, resolved one at a time until the way is clear. Use when the user says wayfinder, map this out, this is too big, or a request has more open decisions than a grilling can settle in one sitting.
disable-model-invocation: true
---

# war-map

After mattpocock/skills `engineering/wayfinder` (3cca18b, MIT). His map is an
issue with decision tickets as children; ours is a Warrant of the `decision`
profile whose **fog** is `rationale.toml` assumptions classed
`blocking_unknown`, each with the resolution requirement that clears it.
`war check` reads them, `war resolve --dry-run` reports them, and the corpus
projection lists them under gaps; nothing about the map is prose only.

## Plan, don't do

The destination is a decision, a spec, or a change made in place; name it
first with a grilling round, because it fixes the scope. The map is done when
nothing is left to decide before someone goes and does the thing.

## Chart the map

1. Grill to name the destination (`war-grill`), breadth-first across the
   whole space. If no fog remains, you do not need a map: stop and ask.
2. `war new "<destination>" --profile decision`. Intent states the
   destination; Non-goals is the **out of scope** list.
3. One stage per decision you can state precisely now (executor `human` for
   HITL grilling and prototypes, `agent` for AFK research), milestones as the
   blocking edges. A question you cannot yet phrase sharply is fog: one
   `blocking_unknown` assumption, resolution requirement stating what would
   make it a stage.
4. Fire research stages as subagents; each captures cited findings under
   `evidence/` and is recorded by a submission.

## Work through the map

One decision per session. `war frontier <alias>` names what is takeable;
`war dispatch` claims it. Resolve the decision with the human, record it: the
answer goes in the stage's submission and, if durable, in a `propose_adr`.
Clear the fog it settles: turn the assumption into stages, or mark it
resolved. A decision that turns out to sit past the destination is out of
scope: close its stage and write the one line in Non-goals.

## Never

The agent never stands in for the human's side of a decision. A skill never
signs.
