# Real corpus response retention — 2026-10-10

The archive reader treated every TOML file below `verifications/` as one stored
verdict. Actual OW-WAR-0011 also retains response envelopes under
`verifications/responses/`. Those valid, populated historical responses were
reported as unreadable verdicts. Read responses through the existing response
type and schema/Warrant identity check. Preserve their exact bytes; do not ingest
verdicts, reissue judgments, authenticate a verifier or award assurance.

A public CLI regression failed on that exact misclassification, then passed with
v1/v2 source envelopes and malformed, unknown-schema and wrong-Warrant controls.
All 24 public preservation tests passed (Rust 1.97.1), as did strict all-feature,
all-target CLI Clippy and formatting. The first test command used a nonexistent
standalone target; that tooling error and the subsequent genuine red result are
both retained. The repository's integration target is `all`.

The real OW-WAR-0011 archive now retains its assurance-case source category.
Its action/audit category remains unavailable: journal event
`01a06512-d465-7982-ae0c-f13d8a9ca285` references
`sha256:d6a9d2cd4d280bb185ee14d2d2fa07f485dd746b1f412f0f94050a2c54b8086c`,
and no retained record in the sampled archive has those exact bytes. Do not
render a replacement verdict and call it recovered history. Complete import
refused with `preservation: required coverage unavailable`; no destination was
created. This is a successful refusal, not complete corpus recovery.

Additional probes remain bounded failures or partial observations: OW149 exceeded
the default 4,096-record history budget; OW148 reports action/audit, artifact and
runtime-reference gaps. Those probes do not qualify the repository or satisfy the
remaining real-corpus round trip. Original records and signed scope are unchanged.

Raw archive bytes and full CLI reports are retained at
`/home/brianklam/Projects/OpenWarrant/docs/runtime-evidence/native-kf-recovery-20261010/`.
See `observation.json` for source hashes and bounded results. OW111 stays in
progress. Format adoption, independent qualification and full hosted gates remain
separate requirements.
