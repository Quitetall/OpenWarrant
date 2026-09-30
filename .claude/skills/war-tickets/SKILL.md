---
name: war-tickets
description: "War tickets: split a Warrant into bounded stages with real input dependencies and observable stop points."
---

# war-tickets

Read [shared workflow](../openwarrant/SKILL.md). Method: Matt Pocock's tracer-bullet
decomposition; [provenance](../openwarrant/ADAPTATIONS.md).

1. Read the Warrant's required outcome, constraints and available interfaces.
2. Draft the smallest independently testable stages. Each names inputs, output,
   positive/refusal checks, context pointers and work-stop boundary. Use child
   Warrants only for separate reviewable outcomes, not one per file.
3. Record only real prerequisite edges. Preparation can precede an unavailable
   implementation input. Qualification gates do not become universal work gates.
   Shared-file writers serialize; isolate each implementation Warrant's worktree.
4. Apply in-scope decomposition to unsigned records and run `war check <alias>`.
   Signed contracts need their actual amendment process. Ask about material scope
   changes, not ordinary stage granularity already delegated by the request.
5. Return stage graph, actual validation and remaining input gaps. Implementation
   starts only if requested; `war frontier` describes legacy scheduling state,
   not proof of permission or completion of the new workflow.

Expand then contract for wide refactors: add new interface, migrate bounded caller
batches, remove old interface after those batches. Keep each stage demonstrable.
