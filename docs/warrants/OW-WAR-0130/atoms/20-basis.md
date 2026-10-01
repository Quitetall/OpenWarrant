---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5fa2-7203-a94c-690e843d1319
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- §67.4: equivalent retries replay the first committed result; conflicting
  reuse of an idempotency key is rejected.
- §66.3: the journal event envelope carries `idempotency_key`.
- §86: file-native commands SHALL use temporary files, atomic rename and
  prestate digest checks, with no partial generated parent publication.
- §69.4: unknown optional namespaced extensions are preserved; unknown
  required extensions fail closed. §69.1–§69.3 (pack versioning) are
  OW-WAR-0032's.
- §27.2: the batch records human acts. Restoring prior bytes undoes this
  tool's own partial write before the batch completes. It never decides an
  act.
- docs/COMPATIBILITY.md: the 1.x forward rules; this Warrant adds the
  reverse direction.
- docs/SIGNING.md and OW-WAR-0072: the batch's six steps and the promise
  that one refusal refuses everything.
- Product spec, "Engineering contracts still to specify": "batch atomicity,
  idempotency, and compatibility/version negotiation".
- OW-ADR-0021: on authorization this Warrant governs the paths it declares.
  - `journal_cmd.rs` is a delivered path of resolved OW-WAR-0031 (D-002).
  - `repo.rs` is declared by authorized OW-WAR-0114.
  - `config.rs` is declared by draft OW-WAR-0113.
  - Whichever of these is authorized later governs.

## What exists (read 2026-09-23, branch `claude/hub`)

- `batch_cmd.rs` step 6 (lines ~339–378): the record loop, the `continue`,
  the `?` on rename, and `batch.recorded` as PASS with "k of n".
- `journal_cmd.rs` `append`: an ERROR on a duplicate idempotency key. The
  key is derived from `(warrant_uuid, event_type, payload)`, so actor and
  time are outside it.
- `verify.rs` ~255–275: the verification file is written before its
  journal line.
- `serde(deny_unknown_fields)` on 30 or more record types, including
  `batch.rs`, `attestation.rs`, `drafting_v2.rs`, `roadmap.rs` and
  `document/records.rs`.
- No `[project]` key names a `war` version. `status.rs` and `progress.rs`
  report the running `war_version` and compare it with nothing.

## Assumptions

- A-001: every act that records through `journal_cmd::record` can be made
  idempotent by checking its journal key before its first write.
  Confidence: medium. STAGE-001's inventory tests this act by act. An act it
  cannot cover is a finding for an amendment, not a silent widening.
- A-002: a batch writes only files under this repository, so rollback means
  restoring prior bytes kept in a temporary file. Confidence: high.

## Unknowns

- U-001 (**blocking authorization**): the version-negotiation mechanism.
  The owner answers before this Warrant is signed, because the deliverable
  set depends on the answer. The Work Order is drafted for option B.
  - A: records only. Each frozen record's schema major is checked against
    what this `war` knows. A newer major is UNKNOWN
    (`compat.newer-record`), never ERROR or PASS. No repository key.
  - B (recommended): A, plus `[project] requires_war = "<semver req>"` in
    `openwarrant.toml`. It is checked at discovery, before any record is
    read, and a mismatch is refused `compat.war-too-old` naming both
    versions.
  - C: B, plus relaxing `deny_unknown_fields` to preserve unknown optional
    `x-…` fields per §69.4. That touches every record type listed above.
    It is far wider and belongs in its own Warrant.
- U-002 (non-blocking): whether a replayed act appends a `*.replayed`
  journal event or nothing. Recommendation: nothing. The replay wrote
  nothing, and the report says so.

## Residual risks

- R-001: a crash during rollback leaves some records restored and others
  not. The `.recording` marker survives, and the next `war sign --batch`
  refuses and names the batch and every path it lists, so the state is
  visible. It is not repaired automatically.
- R-002: replaying instead of refusing could hide a real duplicate. The
  conflicting case (same key, different actor) stays a refusal, and OBL-002
  plants it.
