---
schema: oh.war/atom/v1
warrant_uuid: 01a03d4e-1f7b-7680-8fa7-f84f8dda0962
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work order

Strictly sequential. Do not parallelise across ADRs; the dependency is real.

## 1. ADR 0159 — fix the boundary

Settle where ABIR2's source-agnostic boundary sits before anything is built on
it. Everything downstream inherits this.

Verify: 0159's own `gate_cmd` passes, run directly rather than trusted from
`adr-gate-ceilings.json` — that file is a snapshot and several triage entries
have been measured WRONG before (LamQuant #116).

## 2. ADR 0158 — storage profiles and versioning across that boundary

Only once 0159 is settled. Profiles describe how data crosses a boundary; a
profile written against a moving boundary is a guess.

Verify: 0158's `gate_cmd` passes. Confirm no profile silently assumes the
pre-0159 shape.

## 3. ADR 0074 — migrate production onto it

Only once 0158 defines the shape being migrated to.

Verify: 0074's `gate_cmd` passes, and the migration is re-runnable — a migration
that cannot be repeated cannot be checked.

## 4. ADR 0075 — datapath optimization

Last, and only against a boundary that has stopped moving.

Decide first whether it needs a `gate_cmd` at all, on the merits of the work.
If yes, write it because the work needs checking. If no, record WHY in the ADR
so the absence is a decision rather than a gap.

Verify: measured throughput against a recorded baseline, end to end. Per this
fleet's standing rule, only compress -> store -> decompress -> evaluate counts;
intermediate metrics are not evidence.

## Standing constraints

- Every training run goes through the BLUT cookbook recipe, never raw
  `tools/*.sh`. Use `read_metric` for any claim about a number.
- A poller once FABRICATED a val-R trajectory. Verify numbers against raw
  journald or the CSV, never a prose digest.
- Never pipe before checking `$?`. `| head` or `| tail` before a conclusion has
  produced false "absence" findings here more than once; count to prove absence.
