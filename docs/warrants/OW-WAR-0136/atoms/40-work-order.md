---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-604b-7d63-8453-39847a16c82c
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. **Custody** (`sign.rs`, `attest.rs`, `lib.rs`).
   - `attest_after` for `resolve` adds each file under `gate_run_refs`,
     and its `.run.toml`, stdout and stderr, as Statement subjects.
   - `war attest --custody <alias>` reports each §41.5 field per relied-on
     receipt: collector (runner), original digest (the attested subject),
     transfer method, storage event (the commit that added the file),
     instrument (runtime environment), qualification (the gate's
     `qualification_ref`), transformations, access history, derivative
     lineage (the resolution). Each is present or `UNKNOWN` with a
     reason. A moved subject is `attest.custody-drift`.
   - `--record` writes `docs/warrants/<alias>/custody-audit.toml` naming
     the auditor. It refuses the performer.
2. **Invalidation** (`invalidation.rs`, `lib.rs`, `sign.rs` per Q-001).
   - `war gate invalidate <gate>@<version> --grounds <text>` emits the
     request: the gate, its definition digest, and every resolution the
     sweep would dispute, by alias.
   - Ingest (per Q-001) refuses the performer of any Warrant it reaches,
     writes `docs/gates/invalidations/<gate>@<version>.toml`, runs
     `propagate_invalidation` over every Warrant's `gate-runs/` and
     `resolution.toml`, and writes
     `docs/warrants/<alias>/disputes/DSP-NNN.toml` for each: §56.4's six
     fields, with owner = the resolver who signed.
   - The Gate Definition file is not edited (§43.3).
3. **Standing** (`resolution_cmd.rs`, `evidence.rs`).
   - `war check` reports `resolution.disputed` for a resolution with an
     open dispute; `resolution.toml` is read, never rewritten.
   - A receipt from an invalidated gate is not admissible for requirement
     5 on an unresolved Warrant.
4. `conformance/plants.d/58-invalidation.sh` (new): the plants in
   Assurance, on a scratch corpus.
5. `docs/INVALIDATION.md` (new): custody audit, invalidation and dispute —
   what each does, who may do it, and what it never rewrites.
6. Under Q-002 (b) only: `docs/gates/ops.exit-demo@1.0.0.yaml` and the
   live demonstration recorded as this Warrant's evidence.

## Frozen Surfaces

- `oh.war/resolution/v1` and every `resolution.toml`. Standing is derived
  from disputes, never written into the record.
- Every recorded receipt, run and attestation.
- `propagate_invalidation`'s semantics; it is called, not changed.
- The four existing human act kinds and their ingest.

## Autonomy and Escalation

Tier T2. Stop and escalate on:
- Q-001 (a) or (b): the new act kind and its SAS and AGENTS.md wording are
  the owner's to accept before code lands;
- any invalidation of a gate in this repository other than
  `ops.exit-demo@1.0.0`;
- any path that writes to a `resolution.toml` or an existing attestation.

## Rollback

Delete the invalidation and dispute records written by the demonstration.
Nothing else was rewritten, so standing returns to what the resolutions
say. Remove the new commands. Resolve attestations minted with receipt
subjects remain valid: a verifier that does not know the extra subjects
still checks the ones it knows.
