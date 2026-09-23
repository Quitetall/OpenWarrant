---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f85-7410-9631-757ffda02081
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the compiler refuses an over-budget Dispatch, and emits an in-budget one unchanged
- **scope:** `compile_dispatch` as reached through `war dispatch` on this
  corpus, and its unit tests. No claim about the estimate's accuracy
  against any real tokenizer (R-001).
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - `83-tokens.sh`'s existing plant (`budget_tokens: 10`) is still refused
    `dispatch.over-budget` with "largest:", and now the CLI has no
    comparison of its own. A grep of `crates/openwarrant-cli/src/dispatch.rs`
    finds no `estimated_tokens > budget_tokens`.
  - The unedited stage still compiles, and its packet's `tokens` equals the
    value before this change: same estimate, budget and method.
  - Unit tests: one token over budget is refused; exactly at budget is
    emitted; no token account is `TokensUnrecorded`.

### OBL-002 — over budget, no actor starts and nothing claims the stage
- **scope:** `war perform <alias> <stage>`, `war perform --all` and
  `war run`, on a scratch copy of the corpus with a fixture performer.
  No claim about a Katana or BLUT runtime.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - Over budget, `war perform` exits non-zero with `dispatch.over-budget`.
    The fixture performer's marker file does not exist, no
    `dispatches/*.json` was written, and the journal gained no
    `dispatch.compiled` line.
  - Under `--all`, with one over-budget and one in-budget open stage, the
    in-budget stage is performed and the over-budget one is named.
  - `war run` on an over-budget service stage writes no gate receipt.
  - The refusal is planted: remove `budget_tokens: 10` and the performer
    runs (the marker exists), so the plant is not a no-op.

### OBL-003 — a Dispatch that did not come from this compile is judged before it is bundled
- **scope:** `war dispatch-bundle create` over Dispatch files on disk.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - A Dispatch whose `estimated_tokens` was edited above its
    `budget_tokens` is refused `bundle-over-budget`.
  - One with `tokens` deleted is refused `bundle-tokens-unrecorded`.
  - Neither leaves a file at `--emit`.
  - The unedited Dispatch is bundled, so the refusal is not blanket.

## Gate Adequacy

Required at `basic`. The load-bearing obligation is OBL-002: the RQ is only
worth its name if an over-budget packet never reaches an actor, and the
marker file is what shows no actor started. OBL-001's unit tests have no
gate receipt of their own (U-002). Its gated evidence is the plant through
`war dispatch`.
