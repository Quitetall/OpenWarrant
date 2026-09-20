# KF consumer observation

Producer: OpenWarrant 8d8b81ce (full identity in consumer source.json).
Consumer: openhuman-knowledge-fabric 3b59c94c4f38bcfcb05f433ccb939a5a7ae4b3d9, branch codex/ow110-generated-types.
Node v24.21.0, pnpm 11.21.0, TypeScript 6.0.3. Dependencies installed offline,
frozen lockfile, lifecycle scripts disabled; no paid/model calls.

All 15 generated declaration files and their manifest copied byte-identically.
Normal KF warrants build imports the generated submission field type at the real
register_warrant_submission path. Exhaustive runtime vocabulary binds to the
producer enum; invalid inputs refuse, including prototype property names.
Public package exports submission and dispatch types. Structural types do not
replace runtime schema, authorization or evidence checks. Profile/assurance
schemas currently use strings; no misleading enum assurance claimed for them.

Observed exit 0:

- tsc --build packages/warrants and packages/orchestrator.
- strict no-emit compile of public package positive/negative fixtures and the
  modified database test (ES2023, NodeNext).
- ESLint over package sources and modified tests; Prettier on authored files.
- Vitest: 24 tests, including 8 actual PostgreSQL Warrant action tests. Invalid
  next action leaves lifecycle/version unchanged and writes no submission row;
  valid submission then proceeds. Generated artifact hash/membership checks pass.

First database run failed in the new test's result-access code: Tx.query returns
an array, not a pg QueryResult. Corrected test to Tx.one, reran; final log retained.
No application workaround or removed assertion.

Producer drift experiment used an independent disposable clone: untouched
schemas --check exit 0; planted comment in stage-dispatch.ts exit 2 with
schemas.drift. Implementation source and existing JSON pack were unchanged.

Not complete: expanded generator construct/refusal coverage, full producer gate,
hosted integration gates and merge. This is implementation evidence, not an
independent verification verdict or human acceptance. Remaining historical OW32
obligations are not closed by these bounded results.
