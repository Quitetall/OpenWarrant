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
