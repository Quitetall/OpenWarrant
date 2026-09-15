---
schema: oh.war/atom/v1
warrant_uuid: 01a06025-b26f-7950-be1c-c830b2a215af
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. §47.1 complete on `StageDispatch`: `api_version`, `workspace_basis_ref`,
   `context_manifest_ref`, `submission_schema_ref`, a
   `CapabilityAuthorization {policy_ref, digest}` in place of the flat digest,
   and `dispatch_digest`. `validate` requires the refs and the api version.
2. `compile_dispatch` in `openwarrant-compiler`, pure: takes the IR, the basis,
   the milestone, the stage, the attempt, the context manifest, the envelope,
   the capability authorization and a caller-supplied `dispatch_id`; returns a
   `StageDispatch` with `attempt_basis_digest`, `context_manifest_digest` and
   `dispatch_digest` computed under their §65 domains. Deterministic for fixed
   ids — the same inputs give the same bytes.
3. §47.2's duties discharged in the compiler: objective from the milestone,
   instructions from the work order's Premade Instructions, non-goals from its
   Frozen Surfaces, `input_artifacts`/`required_outputs` from the stage's
   ports, `obligation_refs` from the milestone, required normative sources
   derived from the required atoms and checked against `omitted_subgraphs`,
   prior failure evidence carried from the attempt.
4. `war dispatch <alias> <stage-id> [--attempt-kind K] [--emit PATH]`: builds
   a `ContextManifest` from the Warrant's atoms (each pinned by content
   digest, holder `git` at the current commit), an `Attempt`, and emits
   canonical JSON. Refuses a stage with no `executor_ref` as `blut` does.
5. `render.rs::stage_dispatch` keeps its disclaimer and names `war dispatch`.

## Frozen Surfaces

The §47.1 field names. `same_normative_contract_as`. `DigestDomain::ALL` —
fifteen, unchanged. Nothing in `blut.rs`.

## Premade Instructions

- The digest is computed over the dispatch with `dispatch_digest` empty, then
  written in. Computing it over itself is a fixed-point nobody can verify.
- A required atom that is not in the context manifest is a compile error, not
  a warning. §33.6.
- Do not invent a capability policy. Record that none is declared.
- Regenerate `deliverables.toml` last.

## Autonomy and Escalation

Tier T2. Escalate if two compilations of one stage with fixed ids differ in
bytes — that is an unsorted collection or a clock, and either is a design
defect.

## Rollback

Revert. `StageDispatch` returns to a type nothing produces, which is the
honest state it was in.
