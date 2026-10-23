# Observation output retention — 2026-10-23

The public CLI regressions first observed missing early refusal in six drivers,
a late-created destination overwritten with success, and dangling-link refusal
gaps. Three public CLI tests now pass across all seven drivers. External Cargo
and rustc facades in the publication race test are explicitly synthetic: they
exercise file output only, not LAMU behavior or qualification.

All seven real integration drivers also passed against LAMU participant
`2572894f28ce76c2c7e6c6c9ae6be51a7f400fa1`: source, conditions, selection,
projections, package, budgets and the complete adapter/second-consumer exchange.
The adapter observed unchanged inputs, deterministic output, tamper/duplicate
refusals and blocked readiness. These are Linux performer observations. No
independent disposition, human acceptance, model quality or Phase 2 exit follows.

The complete raw real observations, process streams and input hashes are retained
in `/mnt/4tb/artifacts/OpenWarrant/keep/integration-output-retention-20261023/`.
`summary.json` binds each observation by SHA-256. Four inventory tests also pass;
the public output regressions are now part of the existing Phase 1 runner.
