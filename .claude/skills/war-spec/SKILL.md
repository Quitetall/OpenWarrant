---
name: war-spec
description: "War spec: turn settled conversation into a bounded Warrant with context, constraints and testable outcomes."
---

# war-spec

Read [shared workflow](../openwarrant/SKILL.md) and
[drafting transport](../openwarrant/references/drafting.md).
Method: Matt Pocock's spec synthesis; [provenance](../openwarrant/ADAPTATIONS.md).

1. Reuse settled outcomes, applicable ADRs and actual repository interfaces. Name
   missing material decisions; infer routine implementation details within scope.
2. Draft one reviewable outcome with scope/non-goals, exact context pointers,
   explicit constraints, input prerequisites and positive/refusal evidence plan.
   Mark qualification-only conditions separately from named execution gates.
3. Use the format supported by the selected tool: current `war plan` reads a v2
   proposal into legacy atoms; RC.3 footer authoring remains SDK build scope.
   Proposed architecture belongs in linked ADR drafts, not an invented acceptance.
4. Validate the proposal, inspect every operation and fix actual defects. If the
   user requested drafting/applying, an agent may apply its reviewed proposal via
   `war plan --proposal <file> --reviewed --apply`. This is not a human signature.
5. Return artifact path, validation result and unresolved material decisions.
   Planning completion says nothing about implementation being complete.

Ask only when a missing answer changes required behavior, scope or authority.
Concrete context pointers are useful; retain exact revisions and symbol/unit IDs
where available instead of banning source paths or copying entire files.
