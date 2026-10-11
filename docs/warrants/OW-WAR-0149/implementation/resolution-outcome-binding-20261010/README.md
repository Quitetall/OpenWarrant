# Recorded resolution outcomes and roles

Implementation: `ff5af6d1`; refreshed against main at `879b9a332f613fd8779454e50a31bc42a0b2c9fd` without changing the tree. Main includes the fully gated inline runtime import change at `b376e5103daf8cd850f44d867a82a044c6054cbd`. This is performer evidence, not independent verification or human acceptance.

## Reproduced defect and fix

The public signature verifier accepted a genuinely signed `satisfied` response as support for a record whose outcome was `cancelled`. The RED log retains this counterexample. Test keys are software fixtures held in memory; synthetic assignments do not establish human enrollment or consent.

When a resolution record exists, signature queries now match its exact v1 schema, subject, contract digest, resolver, role reference, common outcome and profile outcome. The same matcher controls the existing-record signature-supplementation branch of ingestion. Signing readiness and corpus checks use the shared verifier, so they receive this check too. Wrong outcomes and roles cannot supply the existing record's signature. The individual/batch signature verification and configured trust-root selection still run afterward.

A legitimate later signature may use a later time and explanatory meaning. These two fields remain distinct from the historical act, and the record stays byte-for-byte unchanged. This is v1 compatibility, not an assertion that v1 signs a complete record digest. Tampering with the record's outcome is refused. An unreadable or unsupported record returns `Unavailable` instead of a signed verdict.

## Checks and limits

Three targeted authority-check tests pass, including genuine signature checks, wrong digest/subject/actor/schema/outcome/profile/role controls, an unchanged-record supplementation case, record tampering and unavailable-record cases. The shared ingestion matcher is exercised directly.

The library suite reached its explicit 300-second execution limit: 444 of 446 listed cases had reported passes. The two remaining cases passed in isolated reruns: the Bonsai qualification pilot and the migration completion-claim refusal. The interrupted aggregate run remains UNKNOWN as a complete suite run; it is not reported as an overall pass. All-target/all-feature CLI Clippy passes with warnings denied, and the CLI binary builds. Full hosted release gates remain required. Logs and exact uncompressed hashes are in `observations.json`.

Native response ingestion is still unavailable. Native evidence, snapshot and policy bindings need a versioned signing/ingestion path with fresh reassessment; v1 cannot carry them. No assurance, disposition or human signature is fabricated. OW95 has no participants and no sessions.
