# Candidate compact verification storage

Proposed ADR0030 now defines a separate v3 stored envelope binding both the
unchanged v2 archive references and exact executable-view references. The SDK
codec is explicit opt-in. It does not activate CLI ingestion or the active
schema pack. Existing v1/v2 decoding remains unchanged and refuses this format.

Observed on Rust 1.97.1: four candidate-record tests and two existing v1/v2
record tests passed with the schema feature; all 54 verification-subject CLI
regressions passed; Clippy over core and CLI, all targets, with the core schema
feature and warnings denied passed. The initial missing public module was
observed before implementation. Candidate controls cover exact reference sets,
missing/duplicate references, invalid hashes and relative paths, cross-role
file collisions, altered subject bindings, strict unknown-field refusal,
future-version refusal, old-reader refusal, and preservation of the active pack.

The JSON schema was emitted by the Rust schema exporter and tested against it;
no stand-in canonicalizer or digest was introduced. Metadata matching does not
read or authenticate the files and does not establish independent assurance.
No real disposition or human act was written. Executable views, compact response
transport, journal/storage integration and qualification remain unfinished.

Raw checks and the exporter output are retained in
`/mnt/4tb/artifacts/OpenWarrant/keep/conformance-continuation-20261014/compact-record-20261015/`.

## Registry and official exporter integration

The full CI run 38827560179 at cdbbbc18 failed one of fourteen gate steps:
the candidate v3 schema was absent from the schema registry. The same validator
reproduced 263 passes and one registry failure locally. A public CLI control
also reproduced that `war admin schemas` did not publish v3.

The candidate is now registered and published/drift-checked by the official
exporter beside v2, outside the unchanged active 0.2.0 pack. The targeted format
validator reports 264 passes, zero failures; all 55 verification-subject CLI
controls pass with schema enabled. The new control plants a changed v3 schema
and observes the named drift refusal, while checking the active pack against
the shipped bytes. Schema-enabled CLI Clippy over all targets passes.
The old failed CI is retained as `failed-ci-38827560179.log` in the raw evidence
directory above. A fresh full gate is still required; this corrects registry
integration and does not activate compact verification or award assurance.
