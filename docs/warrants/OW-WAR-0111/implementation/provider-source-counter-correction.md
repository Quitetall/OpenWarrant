# Correction: provider ledger revisions are not source contract revisions

Earlier OW111 adapter observations labeled KF provider record 2 as a source
revision without retained history. That interpretation was wrong. KF stores
proposal and authorization as separate ledger revisions. Both rows can reference
the same source contract revision in their retained canonical IR.

A native OW66 service dispatch exposed the bug: its source contract revision is 1,
while its fixture provider authorization record is 2. The former adapter refused
the genuine binding. Red test log remains retained. Corrected provider commit
`32d3afe91490d50a13835a0bcaa73cefa4070bae` reads source identity from canonical IR and retains provider record numbers
separately. Unknown source identity never falls back to a provider number.
The same retained package now correctly maps provider 1 -> source 1 and provider
2 -> source 1. Earlier archives and output observations remain unchanged; this
note corrects their interpretation. Distinct source revisions still require
matching retained source identities and digests.

A real PostgreSQL round trip now preserves the native service receipt and dispatch
after source shutdown. Exported JSONB text, dispatch records and database snapshot
digest survive import/re-export unchanged. Unknown manifest keys refuse import.
The underlying process was `true` in a disposable OW clone: execution seam proof,
not feature correctness, real project permission or human assurance. Provider
permissions and keys are disposable fixtures. Original stream bytes remain in the
retained OW archive; this test does not restore them through KF object storage.

49 targeted tests, build, focused lint and test TypeScript checks passed. The
corrected provider head is pushed to PR8; new full local/hosted gates remain pending.
OW full local gate at 0fc15c6c passed 14 steps and 309 controls, covering the service
stream fix and journal-reference resolver. No Warrant resolution is inferred.

Full local `pnpm gate` at corrected provider commit 32d3afe9 exited 0.
Exact log retained as `provider-counter-full-gate.log.gz`. Hosted gates remain
pending; local evidence does not replace the CI-only secret-history scan.
