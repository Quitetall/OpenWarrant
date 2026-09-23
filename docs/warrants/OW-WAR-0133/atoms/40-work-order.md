---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5ff5-77b0-a389-10e0350f4b79
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `crates/openwarrant-cli/src/gate_cmd.rs`: receipt minting.
   - Add subjects `tree:<git tree sha>` and
     `deliverables:sha256:<digest>` beside the contract subject, and
     `worktree:dirty` when the tree is dirty.
   - Fill `fixture_digests` from the definition's declared fixtures. Leave
     it empty only when the definition declares none.
   - Under Q-001 (c): read the definition's `inputs` and add
     `inputs:sha256:<digest>`.
2. `crates/openwarrant-cli/src/evidence.rs`: admissibility.
   - A recorded run is admissible only while every subject the Q-001 rule
     reads still holds.
   - A moved subject is `evidence.stale-binding`, naming the subject. A
     missing one is `evidence.reuse-unknown`. Neither is a gate failure.
   - Resolved Warrants are not re-evaluated.
3. `crates/openwarrant-cli/src/context_select.rs` and
   `crates/openwarrant-compiler/src/dispatch.rs`: the context manifest.
   - Stop emitting `conflicts: []` for a list nobody checked, per Q-002.
   - Under Q-002 (c): record a conflict when one source path is included
     at two digests or revisions, and refuse the Dispatch.
4. `conformance/plants.d/55-evidence-reuse.sh` (new): the plants in
   Assurance, against the shipped binary on a scratch corpus.
5. `docs/RESOLVING.md`: a section "When recorded evidence still counts",
   with the reuse rule, the three compiler rules, and the diagnostic each
   produces.

## Frozen Surfaces

- `oh.war/resolution/v1` and every recorded resolution.
- The contract digest's preimage (OW-ADR-0004).
- The omission rule in `context.rs`: a required item is never omitted, and
  every omission has a reason.
- The receipt's existing fields and its seal. New subjects are new entries
  in `subject_digests`, not new fields.

## Autonomy and Escalation

Tier T2. Stop and escalate on:
- any change to the resolution record, or to requirement 5 for a resolved
  Warrant;
- a rule that would make a reuse-`UNKNOWN` receipt admissible;
- a Gate Definition version bump beyond the gates this repository ships.

## Rollback

Revert admissibility to contract-only. Receipts minted with the extra
subjects stay valid under the old rule, because it reads only the contract
subject. No record is rewritten.
