---
schema: oh.war/atom/v1
warrant_uuid: 01a06025-b26f-7950-be1c-c830b2a215af
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

Produce a Stage Dispatch — *"the only packet given to a stateless actor"*
(§47) — from a Warrant, a stage, an attempt and a context manifest, with the
digest §47.2 requires and the bytes §47.2 requires to be deterministic.

## What exists, and what does not

Every type §47 names exists and validates: `StageDispatch`, `ResourceEnvelope`,
`ContextManifest`, `ContextItem`, `Attempt`, `AttemptKind`. `DigestDomain`
declares `Dispatch` and `ContextManifest` among §65's fifteen. **No function
produces a dispatch.** `StageDispatch` is constructed field-by-field in its own
unit tests and nowhere else; the `stage_dispatch` view prints the milestones
and work-order atoms verbatim under a banner that says, correctly, *"the stage
graph, not a Stage Dispatch"*. OW-WAR-0023 claims RQ-042 and RQ-043 complete
on the strength of the type.

## What this delivers

`compile_dispatch` in the compiler crate — the §47 sibling of `blut::lower`,
which already walks the milestone graph, resolves executor references, maps
ports and pins provenance for §49. And `war dispatch <alias> <stage>`, which
builds the inputs from the Warrant's own atoms and emits the packet.

## What this refuses to do

It does not make the `stage_dispatch` VIEW compile a dispatch. A dispatch
carries a fresh UUIDv7 `dispatch_id` and `attempt_id`; it is non-deterministic
by construction and cannot be a committed, drift-checked projection. The view
keeps its disclaimer and names the command that does the work.

It does not execute anything. Phase 5's Exit — *"one WAR stage can be compiled,
executed by a stateless Katana agent, and returned"* — is OW-WAR-0045's. This
Warrant is the first verb of that sentence.
