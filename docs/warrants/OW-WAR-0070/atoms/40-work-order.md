---
schema: oh.war/atom/v1
warrant_uuid: 01a09274-ad79-7414-a583-cdecca67ed90
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. Two additive optional fields on core `Assumption`:
   - `bound_to` — a list of refs, minimum one, naming what this debt blocks.
     Each is either `roadmap://<NS>-PHASE-N/<slug>` or `war://<uuid>#<STAGE-id>`:
     stage-precise where a stage exists, phase-level where the work is not yet
     warranted. An empty list is a refusal, not an unreserved gap — a debt that
     blocks nothing is a statement about nothing;
   - `future_ref` — the reserved decision record that will carry it, absent when
     no reservation exists.
   No `deny_unknown_fields`, so records written before this change still parse
   and slice D1's `external_dependency` composes with it.
2. A typed value for the existing §36.3 `resolution_requirement` covering the
   decision case, so decision debt is distinguishable from other blocking
   unknowns by field rather than by prose.
3. Validation: every `bound_to` ref must resolve to a node that exists and the
   list must be non-empty; a `future_ref`
   naming a decision record that already exists and is accepted is a
   contradiction and is refused — debt that is already paid is not debt.
4. Surfacing in the slice D1 Gaps view: rows group by `external_dependency` and
   sort by the earliest `bound_to`. A row whose `resolution_requirement` is `adr`
   and which names no `future_ref` renders as an unreserved gap.
5. `conformance/plants.d/70-*.sh` for each refusal and for the unreserved-gap
   case, using `lib.sh` from the base branch.
6. A SAS revision narrating the decision case under §36.3, plus an ADR if U-001
   resolves that one is required.

## Allowed surfaces

`crates/openwarrant-core/src/` where `Assumption` and rationale parsing live;
`conformance/plants.d/`; the D1 projection contract where the Gaps view is
specified; `docs/sas/` revision; `docs/adr/` if required; this Warrant's records.

## Frozen surfaces

Every file pinned by a resolved Warrant, including `crates/openwarrant-cli/src/verify.rs`
(resolved OW-WAR-0046). Other Warrants' directories. `main`. The
`/mnt/4tb/OpenWarrant` checkout held by another session. Any consuming program's
repository.

## Autonomy limits

The performing agent may draft, build, run the battery, and report. It may not
authorize, verify its own work, or resolve.

## Coordination

Slice D1 edits the same type. Whoever lands second rebases. This Warrant does not
claim the D1 projection work; it supplies fields that view consumes.

## Procedure

1. Add both fields optionally and confirm existing fixtures still parse.
2. Add the typed resolution requirement and its validation.
3. Add the contradiction refusal for an already-accepted `future_ref`.
4. Plant each refusal and the unreserved-gap case.
5. Run the full battery; record raw output.

## Rollback

Both fields are optional and additive. Removing them leaves every record that
never used them byte-identical. No contract digest moves from this Warrant alone.
