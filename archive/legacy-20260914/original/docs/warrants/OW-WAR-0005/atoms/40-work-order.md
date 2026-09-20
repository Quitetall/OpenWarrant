---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-71ab-a5db-0f2c062305af
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. A `Diagnostic` model carrying severity (`PASS` / `WARN` / `ERROR` /
   `UNKNOWN`), the rule, the file, and the offending value.
2. `war check <alias>` running every Phase 1 check and printing the §71.7 shape.
3. A readiness conclusion that is explicit about what it does not cover.
4. `war check --generated` wired in from OW-WAR-0004.
5. The §91 Phase 1 conformance subset in `conformance/`, moved out of unit tests
   and onto real source trees, since the control under test is now a binary
   reading files.
6. `cargo xtask gate` extended to run the conformance suite.
7. Bootstrap closure: `war check` green over all five Warrants.

## Frozen Surfaces

- The severity vocabulary. Adding a severity, or removing `UNKNOWN`, changes
  what a verdict means to every reader and every downstream consumer.
- The exit-code contract.

## Premade Instructions

- Report **every** diagnostic the checker can see, not the first. A checker that
  stops at the first ERROR makes the author re-run it once per defect, and this
  fleet has already been burned by a failing test that named one of two defects
  while the second stayed green and wrong.
- `UNKNOWN` is a distinct severity from the start. Retrofitting it later means
  auditing every check that already collapsed it into something else.
- Every conformance fixture ships with its planted violation. A positive fixture
  without a matching negative proves only that the happy path runs.
- Do not print "READY" without naming what readiness excludes in Phase 1.

## Resources and Capabilities

Repository-local filesystem read. Writes confined to scratch trees under
`conformance/`. No network. No secrets.

## Autonomy and Escalation

Tier T1. The severity vocabulary, the exit-code contract, and the readiness
wording are all decisions about what the system asserts, not how it is built.

## Rollback

Revert. The four preceding Warrants remain resolved and their deliverables keep
working; what is lost is the single verdict over them.
