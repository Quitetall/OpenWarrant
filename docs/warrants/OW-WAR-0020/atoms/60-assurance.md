---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd5-a42a-766a-916c-656c6b37998b
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — an unaskable gate cannot satisfy a required pass
- **scope:** §44.1, §44.5, §99 criterion 19. Amended by OW-ADR-0006: the original
  wording required a verdict to be *unrepresentable* when unaskable, which
  contradicts §44.3 and §44.4.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** all 36 askability × execution-status × verdict triples enumerated,
  with exactly one satisfying a required pass; and validation refusing an
  unaskable run that records pass or fail.

### OBL-002 — the two vocabularies survive intact
- **scope:** §44.2's six execution statuses and §96.4's ten migration classes.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant per locally reachable reason code, each reported as
  itself, and `missing_tool` landing on a different rule from `failed`. Plus a
  test asserting every could-not-ask class migrates to `not_askable` and never to
  a failure verdict. `missing_crate` is reachable only when cargo itself is
  absent, which is recorded as a limitation rather than faked with a heuristic.

### OBL-003 — a required unknown BLOCKS resolution
- **scope:** RQ-054.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `blocking_required_runs` naming the unknown run, and a plant in
  which an unaskable gate exits non-zero as UNKNOWN rather than as a pass.

### OBL-004 — invalidation propagates
- **scope:** RQ-057, §45.
- **evidence:** invalidating a gate run marks every resolution resting on it as
  invalidated, transitively.

### OBL-005 — a mutating gate is quarantined
- **scope:** §44.8.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant declaring `mutating: true`, refused before any process is
  spawned, however completely the mutation is declared.

## Gate Adequacy

Required at `controlled`, and this is the Warrant where a defect is worst: a
false PASS here manufactures a resolution.

**Adversarial question: could the obligations pass while a gate result is still
false?** Yes, in the way that matters most. A gate can be askable, execute
cleanly, return exit 0, and measure the wrong thing. Nothing in §44 verifies that
a gate's result corresponds to the obligation it is cited against. §39's adequacy
review is the only control that asks that question, and it is a human one.

- **outcome:** counterexample_found, gate_strengthened, obligation_narrowed, gap_accepted

`counterexample_found` and `obligation_narrowed`: the drafted OBL-001 required a
verdict to be unrepresentable when unaskable, and OBL-002 required "all ten
execution statuses". Both contradict §44 — the ten are §96.4's migration classes,
and §44.3 defines `unknown` as a verdict. The obligations were narrowed to what
the specification actually says, and OW-ADR-0006 records why.

A second counterexample came from external review, on code already committed:
the runner routed unknowns on "is the verdict unknown" rather than on askability,
so an askable gate that TIMED OUT was reported as "could not ask, so there is no
result to report". That is the same collapse this Warrant exists to prevent,
running in the opposite direction — and the plants did not catch it, because none
of them timed out. There are now three rules and a plant for each.

The deadline was also not a deadline: elapsed time was compared after the process
had already been waited to completion, so a gate running past it would have been
reported `timeout` while having actually finished, discarding a real result. The
runner now polls and kills, and a gate declares its own deadline.

`gap_accepted`: `missing_crate` is reachable only when cargo itself is absent, so
it has no local plant. Faking one would mean a heuristic that guesses from
cargo's exit code, which would sometimes report a real failure as an unknown —
worse than the gap.

**Executed attacks:**
- pointed a gate's argv at a nonexistent tool; reported `not_askable` / `missing_tool`, on a DIFFERENT rule from a failing gate
- pointed a gate's argv at a nonexistent `.sh`; reported `missing_script`, not `missing_tool`
- emptied a gate's argv; reported `not_askable` / `invalid` / `malformed`
- declared a gate `mutating`; quarantined before any process was spawned
- ran a gate that exits non-zero; reported `askable` / `completed` / `fail` — proving the failure and unaskable paths do not collapse
- ran a gate that never answers, against a declared 1-second deadline; killed at the deadline and reported on a THIRD rule, `gate-run.no-result` — neither "could not ask" nor "failed"
- enumerated all 36 §44 triples; exactly one satisfies a required pass
- migrated all ten §96.4 classes; every could-not-ask class landed on `not_askable` + `unknown`, and only `passed` satisfied a required pass

## Residual Risk

Mutation detection is declarative — a gate that mutates without declaring it will not be quarantined. Detecting undeclared mutation reliably would need sandboxing, which is hardening and therefore beta.
