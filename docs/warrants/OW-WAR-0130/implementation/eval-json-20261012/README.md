# Eval setup output stays inside its child process

`war evidence eval run --json` previously called the setup library directly.
The initializer printed terminal hints before the outer JSON report, so a strict
JSON client could not read successful evals. The actual regression failed with
`stdout is not one JSON value: expected value at line 1 column 1`.

Setup now runs through the current CLI in a child process. Its output is captured
and its exit status checked; failed setup still becomes an error. No initializer
API, task/result schema, frozen envelope key or authority record is changed.

The new public-command regression checks a successful fixture eval, its canonical
result, a refused unknown task and the absence of a refused result file. **All eight
JSON-envelope tests pass**, as do all-target CLI Clippy with denied warnings and
formatting. All four amended run fixture tasks also emit a single JSON response
and reach their expected `would_satisfy` rung. These fixtures retain their six
unmet authority requirements; they provide no real agent or independent judgment.

[observation.json](observation.json) records exact source/binary hashes, source basis,
toolchain, red/green logs and scope. The dirty debug build is labeled honestly.
The initial wrong Cargo target and failed first fix are retained, rather than
counted as passing controls. The original OW130 historical retry evidence and
independent findings remain unchanged. Required CI, exact-subject gate evidence,
independent reassessment, delivery reconciliation and human closure remain pending.
