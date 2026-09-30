---
schema: oh.war/atom/v1
adr_uuid: 6f0d3c8e-4b1a-4d2e-9c7f-0a1b2c3d4e5f
local_alias: OW-ADR-0013
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a021a2-b570-7f57-85b2-0f8189873d9e"
---

# ADR OW-0013: Draft Proposal v2 — operations carry their payloads

## Status

Proposed by the performer under the 1.0 plan (slice A4); adopted when the owner
accepts it.

## Context

`oh.war/draft-proposal/v1` (§74.2) lists a planner's `atom_operations` as bare
operation names from §74.3's closed list. That proved two things that matter:
the list is closed (an unknown operation is refused by name), and the struct is
the agent's entire output surface (`deny_unknown_fields` — a proposal carrying
`authorized_by` or `enterprise_id` is refused at parse, never silently dropped).

It could not be applied. `create_atom` with no role, no ordinal and no body
creates nothing. `war plan` therefore emitted a request, validated a returned
proposal through §74.4's first four steps, and stopped. OW-WAR-0042's exit — a
vague request becomes a reviewable draft with no direct file mutation — had
never been reached, because nothing on the tool's side could turn a proposal
into files without a human typing them.

## Decision

`oh.war/draft-proposal/v2`, in a new core module beside v1 (`drafting.rs` is a
pinned deliverable of resolved Warrants and is untouched):

- `operations: [AtomOperationRequest]`, each carrying the operation from §74.3's
  same closed list plus the fields that operation needs — `role`, `ordinal`,
  `path`, `body` for `create_atom`; `target` for revise/retire; `relation_kind`
  and `relation_ref` for `add_relation`; `adr_title` and `body` for
  `propose_adr`. Validation names the missing field and the operation.
- `proposed_identity: {title, profile, assurance}` — what the Warrant will be.
- Every v1 rule kept: evidence claims classed and cited, durable choices produce
  an ADR draft, unresolved blocker questions stop a non-interactive run, an
  invented `war://` blocks. `deny_unknown_fields` stays on every struct.
- A `create_atom` path is a file name, never a route (`/` and `..` refused).

v1 still parses and validates. Only `--apply` refuses it, by name
(`plan.v1-has-no-payloads`).

`war plan --apply` applies a reviewed v2 proposal inside
`docs/warrants/<alias>/` and `docs/adr/atoms/` and nowhere else, through
`war new` and per-operation writers. The model never writes a file (§74.5). The
configured drafter (`[plan] drafter_argv`) is watched for exactly that — the
working tree before and after the run — and a drafter that touched it is
refused with its proposal discarded.

## Consequences

- Both drafting paths reach the same gauntlet: the harness-as-agent writes a v2
  file; a configured process returns one over stdin/stdout (§75.2). §74.4's
  eight steps run in order either way; review is a recorded step, not a flag's
  side effect.
- An applied draft records its own provenance under `plan/` — request,
  proposal, drafter run, pipeline — and three journal events, so OW-WAR-0042 can
  finally be discharged by a recorded run rather than a fixture.
- `revise_atom`, `retire_atom`, `add_binding` and `remove_binding` are
  validated and recorded but refused at apply in this build, by name: a fresh
  Warrant has nothing to revise or retire, and no binding vocabulary exists yet.
  Refusing with the reason is better than a silent no-op.
