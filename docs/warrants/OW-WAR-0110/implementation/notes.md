# Producer implementation progress

TypeScript declarations now render from the existing 15 JSON schema families via
war schemas; a separate manifest binds source pack, source schema and declaration
hashes. Existing schemas/pack.json and schemas/oh.war bytes remain unchanged.
Initial generation refused nullable type lists; support was added rather than
altering source schemas. Current outputs compile under KF TypeScript 6.0.3 in
strict/no-emit mode. Producer fixtures prove required-field, enum and scalar
refusals through @ts-expect-error checks. Rust tests cover unsupported keyword/ref
refusal, required/optional fields, tuple shape and deterministic output.

Unfinished: generated-drift refusal observation, wider generator edge cases,
actual pinned KF package imports and runtime regressions, full gate and hosted
integration. No OW32 assurance or historical disposition changed.
