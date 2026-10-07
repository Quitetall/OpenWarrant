---
schema: oh.war/atom/v1
warrant_uuid: 01a0d31e-63a1-7092-a4d2-0937d817adcb
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- §44.8: a mutating verification action must declare effects, authority and
  compensation; it cannot run merely because a document contains a command
  string. `askability_of` implements the stricter reading: never.
- §43 and RQ-056: gate definitions are separately governed and versioned;
  a new behaviour is a new version, not an edit.
- RQ-054: required unknown gate results block resolution — why a gate that
  can never answer blocks every Warrant that cites it.
- `docs/gates/ops.conformance.plants@1.0.0.yaml` and `conformance/lib.sh`'s
  clean-tree guard and `restore`.

## Assumptions

- A-001: a `git clone --local --no-hardlinks` of `HEAD` into a temp
  directory carries everything the battery reads (it reads only tracked
  files and the built binary). The binary is shared through the clone's
  `./target` link to the same target directory. Confidence: high — the
  battery ran green this way on 2026-09-24 (463 plants, two runs).
- A-002: plants that run `cargo` inside the battery need
  `CARGO_TARGET_DIR` exported; the script sets it to the repository's
  resolved `./target`. Confidence: high (the 2026-09-24 11-minute lock
  wait).

## Unknowns

- U-001 (non-blocking): the right wall time. `@1.0.0` says 1800 s; the
  battery took 25–45 min on 2026-09-23/24 under load. `@1.1.0` declares
  3600 s.

## Residual risks

- R-001: a receipt tests `HEAD`. A performer who records evidence with
  uncommitted work would get a receipt for the commit, not the work. The
  receipt names the commit; the verifier and the resolution bind the
  contract. Committing before evidence is the rule the docs state.
