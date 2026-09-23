---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5fa2-7203-a94c-690e843d1319
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `crates/openwarrant-cli/src/batch_cmd.rs`: step 6 survives a crash.
   In-process rollback (`batch.incomplete`) already exists (77f8fc6e); this
   adds what a killed process needs.
   - Before the first record, write and fsync
     `docs/authority/batches/<id>.recording`, listing every path the batch
     will write and each one's prior sha256, or "absent", with the prior
     bytes beside it under `<id>.recording.d/`.
   - Remove the marker only after the last record and the attestation.
   - While a marker exists, `war sign --batch` refuses `batch.interrupted`
     and `war check` errors with it, naming the batch and its paths.
   - `war sign --batch --recover <id>` restores every listed path to its
     prior bytes, or removes it, then keeps the batch as `.refused.json`.
2. `crates/openwarrant-cli/src/journal_cmd.rs`:
   - `record` returns `Recorded` or `Replayed`.
   - The same key with the same `actor_ref` is `Replayed`: nothing is
     written.
   - The same key with a different actor is refused
     `journal.idempotency-conflict`.
   - `already_recorded(dir, type, payload, actor)` lets a command check
     before its first write.
3. Idempotent acts, by calling `already_recorded` before the first write:
   `war ask`, `war submit`, `war verify --response`, `war evidence record`,
   and the single-act ingest that `war sign` and the batch share.
   - STAGE-001 inventories every act first. An act whose file is not
     declared here is a finding: an amendment or a child Warrant, not an
     undeclared edit.
   - Declared for this: `questions.rs`, `run_cmd.rs` (submit), `verify.rs`,
     `evidence.rs`, `authorize.rs`.
4. Option B of U-001:
   - `crates/openwarrant-cli/src/compat.rs` (new):
     - `requires_war` is checked against `CARGO_PKG_VERSION`;
     - the schema majors of the frozen records are recognised;
     - a newer major is UNKNOWN (`compat.newer-record`).
   - `crates/openwarrant-core/src/config.rs`: `[project] requires_war`,
     optional and validated as a semver requirement.
   - `crates/openwarrant-cli/src/repo.rs`: `Repository::discover` calls
     `compat::check` once, before any Warrant is loaded.
   - `docs/COMPATIBILITY.md`, a "Reading backward" section: `requires_war`,
     what an older `war` does with a newer record, and the §69.4 position
     today.
5. `conformance/plants.d/69-idempotency.sh`, the plants each obligation
   names.

## Frozen Surfaces

- `oh.war/batch/v1`, `oh.war/journal-event/v1` and every record schema.
- The idempotency key's preimage.
- Exit codes and the report envelope.
- The batch's judging steps 1–5 (OW-WAR-0072).

## Autonomy and Escalation

Tier T1: this touches how human acts are recorded. Escalate rather than
decide:

- any act the inventory finds cannot be made idempotent within the declared
  files;
- any rollback that would touch a record a human signed outside this batch;
- a change to `check.rs`, `sign.rs` or `lib.rs`. Other Warrants own them,
  and they are left out on purpose.

## Rollback

Revert the declared files. Batches and journal lines written in the meantime
stay readable by the old code: no format changes. A repository that set
`requires_war` removes the key.
