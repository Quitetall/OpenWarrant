---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f85-7410-9631-757ffda02081
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

RQ-046: a Dispatch declares its token estimate and budget, and exceeding the
budget is a refusal. §47.2 puts the refusal on the compiler.

Most of this exists, in one place only:

- `war dispatch` (CLI, `crates/openwarrant-cli/src/dispatch.rs`) estimates
  the selected context (`oh.war/token-estimate/bytes-div-4/v1`) and refuses
  with `dispatch.over-budget`, naming the three largest items. Commit
  31eb30ca; planted by `conformance/plants.d/83-tokens.sh`.
- `war perform` and `war run` compile through that same function.

What does not hold:

- **The compiler does not refuse.** `compile_dispatch`
  (`crates/openwarrant-compiler/src/dispatch.rs`) accepts any `tokens`,
  including `None` and an estimate above the budget. The rule lives in the
  one caller that happens to check first. A library caller emits an
  over-budget Dispatch without a word.
- **A Dispatch from elsewhere is not judged.** `war dispatch-bundle create`
  packages an already compiled Dispatch file without reading its `tokens`.
  A hand-edited packet, or one compiled before C2 with no `tokens`, is
  bundled as if it were in budget.
- **No plant shows the refusal stops an actor.** `83-tokens.sh` plants
  `war dispatch` only. Nothing shows that `war perform` or `war run` over
  budget starts no process and leaves no Dispatch on record.
- **No Warrant implements RQ-046.** `CORPUS_STATUS.md` lists it
  `unaddressed`.

## Desired Outcome

- The compiler is the one place the rule lives. It refuses to emit a
  Dispatch whose estimate exceeds its budget, naming the largest items, and
  refuses one with no token account at all. The CLI calls it and no longer
  keeps its own copy.
- `war dispatch-bundle create` refuses a Dispatch whose `tokens` are absent
  or over budget, before it captures anything.
- Every path that starts an actor (`war perform`, `war perform --all`,
  `war run`) is shown refusing before any process starts.
- This Warrant implements RQ-046, so the corpus stops reporting it
  unaddressed once it is authorized.

## Non-goals

- Measuring what a performer actually spends. The estimate is the
  compiler's, never a model's count (§47.1). Runtime usage and spend are
  OW-WAR-0132's.
- A new estimation method. OW-WAR-0065 (research memo on the tokenizer
  approximation) owns that; a new method gets a new id.
- Changing `oh.war/stage-dispatch/v1` or `TokenAccount`. Both are frozen
  (docs/COMPATIBILITY.md).
- Recording why a stage raised its `budget_tokens`. See U-001.
