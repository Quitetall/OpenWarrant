---
schema: oh.war/atom/v1
warrant_uuid: 01a06446-1e04-7e93-99af-04ad037dbc46
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the ten Phase 1 deliverables exist and are exercised
- **scope:** §98 Phase 1's deliverable list, as it stands.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `PHASE1_EXIT.md` maps each of the ten to a command (`war
  --help`), a module, or the `corpus` gate step; the gate is green in CI.

### OBL-002 — the repository's own work is carried as Warrants
- **scope:** every Warrant under `docs/warrants/`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war check --generated` exit 0 on the committed corpus;
  an `authorization.toml` beside every Warrant (56 of 56 once 0059, 0060
  and this one are authorized — measured, not assumed).

### OBL-003 — at least one Warrant closed through §56 with a bound receipt
- **scope:** the corpus on the day of resolution.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `resolution.toml` for OW-WAR-0010 and OW-WAR-0020, each
  citing a `gate-runs/` receipt that `war check` reports as
  `evidence.admissible`.

### OBL-004 — commit traceability is measured and recorded
- **scope:** `main`'s history on the day of measurement.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the count and the command that produced it, in
  `PHASE1_EXIT.md`; the number is a record, and the obligation is that it
  was measured, not that it is high.

## Gate Adequacy

Required at `basic`.

**Adversarial question:** can Phase 1 read achieved on a sentence rather
than on a measurement? The attacks: a report that lists deliverables
without naming where each lives; an authorized count asserted rather than
counted; a projection that reads `Recorded` from an exit Warrant that is
merely `would_satisfy`.

**Executed attacks:** the third is OW-WAR-0055's plant (a resolution
record, not a rung, is what `Recorded` needs) and OW-WAR-0059's tests
(`resolved` is read only from a record binding the current contract). The
first two are the report's own form — every line has a command — and are
what the verifier checks.

- **outcome:** no_counterexample

## Residual Risk

- The Exit sentence is read as "the corpus exists, compiles, is authorized
  and can close", not "every commit cites a Warrant". If the owner meant
  the stricter reading, this Warrant discharges the wrong thing, and the
  measured 20-of-87 says how far the stricter reading is from true.
