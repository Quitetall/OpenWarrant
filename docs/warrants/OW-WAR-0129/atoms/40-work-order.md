---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f85-7410-9631-757ffda02081
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `crates/openwarrant-compiler/src/dispatch.rs`:
   - `DispatchInputs` takes the per-item byte counts in place of a finished
     `TokenAccount`, plus the budget. The compiler computes the estimate by
     `tokens::METHOD`.
   - Over budget returns `DispatchError::OverBudget`, naming the estimate,
     the budget, the method and the three largest items with their
     estimates.
   - No items and no budget is `DispatchError::TokensUnrecorded`. A
     Dispatch is never emitted without `tokens`.
   - Unit tests cover in budget, one token over, exactly at budget, and
     unrecorded.
2. `crates/openwarrant-cli/src/dispatch.rs`:
   - drop its own estimate-and-compare;
   - pass the selector's bytes and the budget to the compiler;
   - map `OverBudget` to the existing `dispatch.over-budget` diagnostic
     with the same message shape, so `83-tokens.sh` keeps passing
     unchanged.
3. `crates/openwarrant-cli/src/dispatch_bundle_cmd.rs`: `create` refuses a
   Dispatch whose `tokens` are absent (`bundle-tokens-unrecorded`) or whose
   `estimated_tokens` exceeds `budget_tokens` (`bundle-over-budget`). It
   writes nothing and reads no context first.
4. `conformance/plants.d/83-tokens.sh`, extended:
   - a stage with `budget_tokens: 10` under `war perform`, with a
     performer that writes a marker file: refused `dispatch.over-budget`,
     the marker is absent, no `dispatches/*.json` is left, and no new
     `dispatch.compiled` line is in the journal;
   - the same under `war perform --all` with two open stages, one over
     budget: the other is performed, the over-budget one is named;
   - a `service` stage under `war run` over budget: refused, and no gate
     receipt is written;
   - a Dispatch file with `tokens.estimated_tokens` raised above
     `budget_tokens`, and one with `tokens` deleted: `war dispatch-bundle
     create` refuses each by name and writes no bundle; the unedited file
     is bundled.

## Frozen Surfaces

- `oh.war/stage-dispatch/v1` and `TokenAccount`'s fields.
- `tokens::METHOD` and `estimate`.
- The `dispatch.over-budget` rule id and its "largest:" clause.
- `oh.war/milestones/v1`.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:
- any change to what counts toward the estimate;
- any new field on a record;
- a caller of `compile_dispatch` that cannot supply item bytes.

## Rollback

Revert the three source files and the plant. Dispatches compiled in the
meantime stay valid: their bytes and digests do not depend on where the
check ran.
