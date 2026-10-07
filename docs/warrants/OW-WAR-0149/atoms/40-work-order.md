---
schema: oh.war/atom/v1
warrant_uuid: 01a0f502-4941-70a1-a446-e1eb77dff191
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Shared contract and ownership

This directory is the single shared contract. Provider implementation Warrants and adapter specifications must point here by UUID/revision instead of restating it. Changes must preserve a reviewable shared revision and explicit participant responses. No participant is silently considered to have approved another project's change.

### OpenWarrant

1. Persist provider receipts and an attributable capture/import observation through the SDK and CLI. Record the actual provider interface version, binary/source identity, command or transport, exit/status and exact original receipt reference. Failures must leave prior records intact.
2. Bind capture to the actual recorded dispatch: Warrant UUID, contract digest, dispatch digest, stage id and attempt id. Read these values from the dispatch record; do not accept an arbitrary caller assertion as proof.
3. Keep provider receipt identity separate from local file-content identity. Consume the provider's defined receipt validation; do not invent a new canonical preimage or hash domain.
4. Match every runtime stage in the current compilation basis to its eligible completed attempt and provider observation. Reject wrong provider, wrong contract, wrong dispatch, wrong stage, wrong attempt, altered receipt, stale source or mixed-job references. Missing provider support or unavailable observation is UNKNOWN.
5. Assess Katana realized capabilities against the authorized set. An emitted receipt is not evidence of sandbox enforcement unless the provider actually establishes it. Unknown cost stays unknown; a mandatory hard spend cap refuses an unmetered run.
6. Resolution uses this assessment, rather than counting all runtime stages as automatically unmet. No runtime stages is a genuine pass only for a valid required milestones atom. Missing or invalid milestones stays unmet.

### Katana

Emit the existing OpenWarrant minimum receipt from an actual terminal session: session identity, Dispatch digest, provider-owned PromptIR digest, provider/model identity, event-log head, realized capabilities, confinement, usage, artifact references, terminal runtime status, provider receipt digest and taint-label references. Define and expose receipt verification. Preserve cancellation, halt and failure as distinct states. Do not label a token count as a metered dollar spend.

### BLUT

Emit a receipt that binds the actual executed PlanSpec/job to the requested dispatch and exact registry identity. Supply terminal status, artifact references, lineage reference and provider receipt identity through its defined verification interface. Keep the lineage stream authoritative in BLUT; OpenWarrant stores references and permitted projections. An accepted typecheck is not an execution receipt.

## Deliverables

- One versioned shared adapter contract and participant implementation references.
- SDK/CLI receipt capture, durable storage and basis assessment with precise refusal/UNKNOWN diagnostics.
- Provider-owned Katana and BLUT receipt/verification surfaces at exact source revisions.
- Real positive integration runs and public-seam negative controls; source manifests, local/hosted checks and honest progress records.

## Start, limits and rollback

Drafting, inspection and isolated prototype tests may start by prompt. Verified provider integration requires all relevant participants to accept the shared adapter revision and provider identity, plus any explicit local start requirements. A receipt import cannot authorize work, confer assurance or merge code.

Use separate worktrees, serialized writers, the owner's configurable budget and protected fixtures. No paid model call without reliable cost tracking. Do not alter resolved pins or signed atoms; request successor/correction acts when needed. On interruption, retain capture history and partial progress. Revert unaccepted implementation through Git, retaining observations of failures.
