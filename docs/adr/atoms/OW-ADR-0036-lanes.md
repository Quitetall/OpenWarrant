---
schema: oh.war/atom/v1
adr_uuid: 01a1fafb-0b0f-7822-ad35-9db5cd8e0ff1
local_alias: OW-ADR-0036
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
---

# ADR OW-0036: Lanes: ordered paths of work that do not collide

## Status

Proposed 2026-11-20. Directed by the repository owner the same day: lanes decide
what work goes first and last, one runner works each lane, and agents stay where
they are best. Lanes ship in alpha.9 as the `war` extension (FORMAT-1.0 section 7),
which needs no format revision. They are put to the editor for the core, as a
minor version, only after the Argos release rounds have tested them.

## Context

The owner's working method is plan doc → lane briefs → lanes → gate → tag and
release. Tickets say what to do but not where. Two agents with good tickets can
still edit the same module. When one model writes an abstraction, a second
extends it and a third replaces it, the intent is lost.

Most of what a lane needs already exists:

- `paths` is a core key (FORMAT-1.0 3.3): the globs a piece of work touches.
- A claim carries its `paths` (`ticket::scopes::Coordination`), and
  `scopes::conflict` already refuses two overlapping claims when work is scheduled.
- `part_of` and `depends_on` are core relations. `war frontier` and `ready` follow
  them.
- `war done` holds an item to its level (`ticket::enforce`). For a Warrant, its
  trigger is the completion gate.

What is missing: a named, ordered path of work for one runner; overlap checked
when the plan is written, not only when work is taken; and a check that the work
stayed inside its paths.

## Decision

1. **A lane is a parent work item marked `war.lane: true`.** Its items are
   `part_of` it and run in `depends_on` order. One runner, an agent or an
   orchestrator, works a lane from start to end. An item may be a ticket or a
   Warrant. A Warrant's gate is its completion trigger, so the runner cannot pass it
   until the gate passes.
2. **The lane's `paths` are what it owns.** An item's own `paths`, if it has any,
   must sit inside its lane's.
3. **`war.for`** names who the work suits: an agent (`claude`, `codex`, `gemini`)
   or a role (`data-engineer`). It works on a lane or on any item. It is a
   recommendation only: nothing refuses an agent that does not match it.
4. **Lanes do not overlap.** `war check` refuses two lanes whose `paths` overlap
   (`lane.overlap`, using `scopes::overlaps`) unless one depends on the other.
   Lanes in sequence may share files. Lanes running at the same time may not.
5. **A lane's work stays in its paths.** `war done` on a lane item compares the
   lane claim's `base_revision` with the working tree. If any changed file lies
   outside the lane's `paths`, it refuses (`lane.outside-paths`) and names each
   file. Before any write, as `enforce` does.
6. **Widening is recorded, not silent.** `war lane widen <lane> <glob>` adds the
   glob to the lane's `paths` and journals `lane.widened` with its actor. It is
   refused if the lane would then overlap another lane running at the same time.
   It needs no signature.
7. **A join** is an item that `depends_on` the last item of several lanes: a gate,
   a tag, a release. Its actor is whoever the item names, so CI or a person.
8. **`war.lane` is critical** (FORMAT-1.0 7.3): ignoring it would make `war done`
   more permissive. A lane document therefore lists `requires: [war.lane]`, and
   `war` writes that line for you. `war.for` refuses nothing, so it is not critical.
9. **Commands:**
   - `war lane new <name> --paths <glob>… [--for <who>] [--after <lane>]`
   - `war create … --lane <name>` places the item last in the lane.
   - `war start --lane <name>` hands over the lane's next ready item and claims
     the lane's paths.
   - `war ready --for <who>` filters by `war.for`.
   - `war frontier` and `war status` show each lane, its next item and the joins.
   - The Claude Code plugin's edit guard also denies edits outside the held
     lane's paths.

## Options considered

- **New core keys `lane` and `for` (FORMAT revision 5).** The cleanest result, but
  it costs a 7-day review before alpha.9 could ship lanes, and before anyone has
  used them. Taken later, once tested.
- **Ownership on each ticket instead of on the lane.** Rejected. Inside a lane
  the work is sequential and cannot collide. Collisions happen between lanes, so
  ownership belongs there.
- **Warn instead of refuse when work leaves its paths.** Rejected. The agents
  this is for are the ones that ignore warnings. `widen` is the honest way out.
- **Restrict by symbol, not by file.** Deferred. Globs can be checked for every
  harness and language today.

## Consequences

- Lanes need no new format version. A reader that does not know `war.lane` reports
  every check over a lane document `UNKNOWN`, not `PASS`.
- `paths` on an item that is not in a lane keeps its current meaning: coordination,
  never a refusal at `war done`.
- Work outside lanes is unchanged. Ordinary coding still needs no item.
- The `war` prefix is listed in FORMAT-1.0 Appendix D by the editor. Until then,
  `war check` warns `format.prefix-unregistered`.
