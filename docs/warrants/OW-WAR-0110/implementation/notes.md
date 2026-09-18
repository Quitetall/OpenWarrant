# Producer implementation progress

TypeScript declarations now render from the existing 15 JSON schema families via
war schemas; a separate manifest binds source pack, source schema and declaration
hashes. Existing schemas/pack.json and schemas/oh.war bytes remain unchanged.
Initial generation refused nullable type lists; support was added rather than
altering source schemas. Current outputs compile under KF TypeScript 6.0.3 in
strict/no-emit mode. Producer fixtures prove required-field, enum and scalar
refusals through @ts-expect-error checks. Rust tests cover unsupported keyword/ref
refusal, required/optional fields, tuple shape and deterministic output.

Completed follow-up: retained generated-drift refusal, five generator tests covering
refs/unions/maps/arrays/tuples/nullability and unsupported shapes, and actual pinned
KF imports/build/runtime tests. See kf-consumer.md and ow32-scope-map.md. Full gate
and hosted merge integration remain pending. No OW32 assurance or historical
disposition changed.

Full producer gate completed in independent clone /mnt/4tb/tmp/ow110-full-gate
at ce6ef1fb6a5613ed57cc4d6637b81dc42d24f20f: Rust 1.97.1, exit 0, 14 steps
and 308 plants passed. Retained gate-ce6ef1f.log.gz. Later changes are evidence
and scope-map documentation; final hosted head checks still required. KF consumer
798094c6 additionally regenerates measured test inventory through its normal tool.
