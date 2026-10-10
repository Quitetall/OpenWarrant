# Typed resolution signature candidates

Implementation: `ca4af5f696c43e4957e3c4a7408808d21929d1ac`. This is performer evidence, not independent verification, human acceptance or Warrant resolution.

## Reproduced defect and fix

A software fixture key genuinely signed a resolution response whose contract digest differed from the queried digest. The meaning mentioned the queried digest. Before the fix, the shared signature-candidate evaluator reported `Signed`. The RED log preserves that failure. The fixture key stays in memory; it does not establish human presence. Test files are removed by a cleanup guard.

Resolution candidates now parse as the existing v1 response and must match its schema, Warrant, resolver and exact contract digest. Prose and comments cannot supply those field values. This filter runs before individual and batch signature checks, for either configured authority backend. Existing cryptographic verification and trust-root selection remain in place. No response schema, hash domain, signed record or resolved deliverable manifest is changed. Other authority act types are outside this fix.

## Observed checks

- Three authority-check tests pass. The positive fixture is genuinely signed; wrong-digest, wrong-subject, wrong-actor and wrong-schema fixtures are also genuinely signed and are refused.
- All 446 CLI library tests pass.
- CLI all-target, all-feature Clippy passes with warnings denied.

Compressed logs and their uncompressed hashes are listed in `observations.json`. The failed pre-fix result remains available.

## Remaining work

This does not compare every recorded resolution field with its signed response. Outcome/role binding and legitimate historical signature supplementation need explicit tests. Native signing and ingestion still need versioned binding to selected capture evidence and snapshots, followed by fresh reassessment at ingestion. The existing native review packet remains read-only and cannot grant assurance. OW95 still has no participants; no sessions are recorded.
