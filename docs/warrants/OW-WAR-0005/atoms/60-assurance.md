---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-71ab-a5db-0f2c062305af
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

Assurance level `controlled`; adequacy review required (§39.4) and recorded.

## Acceptance Obligations

### OBL-001 — `war check` produces the §71.7 output shape

- **scope:** the five Warrants under `docs/warrants/`.
- **evidence:** stdout matching the documented shape, and an exit code
  consistent with the highest severity reported.

### OBL-002 — every Phase 1 conformance item PASSES

- **scope:** §91.1 items 1–6, §91.2 items 7, 8, 9, 10, 12, 16, and §91.4 items
  26–27. Enumerated, so the set cannot quietly shrink.
- **evidence:** the conformance suite's per-item report.

### OBL-003 — every planted violation is REJECTED BY ITS INTENDED CONTROL

- **scope:** one planted violation per item in OBL-002.
- **evidence:** for each, a run in which the check fails AND the failure names
  the rule the plant violated.
- **why "by its intended control" is in the obligation:** a plant that fails for
  the wrong reason — a malformed fixture rejected by the TOML parser rather than
  by the duplicate-ordinal rule — proves nothing about the rule it was meant to
  exercise, while looking exactly like success in a pass/fail summary.

### OBL-004 — `war check` is green over this repository's own Warrants

- **scope:** universal over `docs/warrants/`, enumerated at run time rather than
  listed, so a sixth Warrant is covered on the day it is added.
- **evidence:** exit 0 with no ERROR diagnostics.

### OBL-005 — the Phase 1 bootstrap is closed

- **scope:** the claim that OpenWarrant development now proceeds through
  Warrants OpenWarrant compiles.
- **evidence:** OBL-004 green, plus `war compile` over the same five leaving
  `git diff --exit-code` clean.
- **note:** this obligation is about a process claim, and the temptation is to
  mark it satisfied because the code works. It is satisfied only when the next
  unit of work actually opens as a Warrant.

## Gate Adequacy

**Adversarial question:** could `war check` report green over a Warrant that is
not fit to authorize?

Yes, in three ways, all accepted and named:

1. **It validates the record, not the work.** A Warrant whose acceptance gates
   are nonsense passes `war check` completely. Gate execution is Phase 6. This is
   the single largest gap and it is inherent to Phase 1's scope.
2. **Readiness excludes Preflight** (§32.7). A Warrant can be well-formed and
   unexecutable. Mitigated by refusing to print an unqualified "READY".
3. **Six §91.2 items are out of scope** — bound-atom resolution, Source Holder
   ambiguity, classification propagation, parent-edit mapping. A Warrant
   malformed in one of those ways passes.

- **outcome:** no_counterexample_found, gap_accepted

**Executed attacks:**
- the whole plant battery IS an attack on `war check`: 36 planted violations, each expected to be rejected by a NAMED rule and a NAMED detail
- the `pass` plant is the negative control: the unmodified 40-Warrant corpus must exit 0, without which a checker that rejects everything would score identically
- four plants share the rule `manifest.invalid`, so matching the rule alone would let the duplicate-ordinal plant 'pass' while actually being caught by the unknown-role branch; the detail string is what distinguishes them

## Residual Risk

- The `decision` profile remains unexercised; all five Warrants are `delivery`,
  so `war check`'s handling of a decision Warrant is untested by the corpus.
- Conformance runs on one host, so §91.1 test 1's "two supported hosts" is
  satisfied by two runs rather than two architectures. Stated in the claim, not
  hidden in it.
