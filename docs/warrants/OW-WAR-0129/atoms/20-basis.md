---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5f85-7410-9631-757ffda02081
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- RQ-046 (§106): a Dispatch declares its token estimate and budget;
  exceeding the budget is a refusal.
- §47.1: `tokens` is inside the Dispatch digest; the estimate is the
  compiler's by the named method; the budget is the stage's
  `budget_tokens` or `[context] default_budget_tokens`.
- §47.2: the compiler SHALL record the token estimate and budget, and
  refuse to emit a Dispatch whose estimate exceeds its budget, naming the
  largest items.
- §33.6: a required context item is never silently dropped to fit a
  budget. The fix a refusal asks for is a cut the author makes, not one
  the compiler makes.
- docs/COMPATIBILITY.md: `stage-dispatch/v1` is frozen; nothing here adds a
  field to it.
- OW-ADR-0021: this Warrant declares both `dispatch.rs` files. Resolved
  OW-WAR-0056 delivered them (D-002, D-003). On authorization this Warrant
  becomes their owner, and 0056's pins become historical.
- OW-WAR-0114's gap table: RQ-046, "§106, unaddressed", placed at
  `PHASE-5/budget-refusal`.

## What exists (read 2026-09-23, branch `claude/hub`)

- `crates/openwarrant-cli/src/dispatch.rs` lines 198–231: the estimate, the
  refusal and its message.
- `crates/openwarrant-core/src/tokens.rs`: `estimate`, `METHOD`,
  `TokenAccount`.
- `crates/openwarrant-compiler/src/dispatch.rs`: `DispatchInputs.tokens` is
  an `Option`; `compile_dispatch` copies it through.
- `crates/openwarrant-cli/src/dispatch_bundle_cmd.rs`: `create` reads the
  Dispatch JSON and never looks at `tokens`.
- `perform.rs` and `run_cmd.rs` call `crate::dispatch::run` and return on a
  refusal before spawning. This is read from source, not yet planted.

## Assumptions

- A-001: every caller of `compile_dispatch` can supply per-item byte counts.
  The CLI's selector already reports them (`selection.bytes`). Confidence:
  high; there is one production caller.
- A-002: requiring a token account at compile time breaks no stored record.
  Stored Dispatches are read, not recompiled. Confidence: high.
- A-003: no Dispatch compiled before C2 is still waiting to be bundled or
  performed in this corpus. If one is, the bundle refusal names it, and it
  is recompiled. Confidence: medium.

## Unknowns

- U-001 (non-blocking): whether raising `budget_tokens` should need a
  recorded reason. The refusal message says "with a reason", but the
  milestones grammar has no field for one. Adding one changes
  `oh.war/milestones/v1`. Out of scope here; owner's call later.
- U-002 (non-blocking): no registered gate runs `cargo test`. The compiler's
  unit tests are evidence a verifier can rerun, but no gate receipt covers
  them. The plants reach the compiler through `war dispatch`, and they carry
  the obligations.

## Residual risks

- R-001: bytes ÷ 4 misjudges non-prose context (dense JSON, CJK text). The
  refusal is exact against the estimate and only approximate against a real
  tokenizer. OW-WAR-0065 owns the method.
- R-002: one refusal per stage can hide a second over-budget stage behind
  it in `war perform --all`. OBL-002 requires `--all` to report every one.
