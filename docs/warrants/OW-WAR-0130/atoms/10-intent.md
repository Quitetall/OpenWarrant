---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5fa2-7203-a94c-690e843d1319
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The product spec lists "batch atomicity, idempotency, and
compatibility/version negotiation" as engineering contracts still to specify.
Each has a partial answer in `war` today, and each has a hole.

**Batch atomicity.** OW-WAR-0072's batch act (`batch_cmd.rs`) promises all
or nothing. Since commit 77f8fc6e it keeps that promise for every failure the
process survives: step 6 copies each directory the batch writes before the
first record, and any refused ingest, retire failure or I/O error restores
them byte for byte and refuses as `batch.incomplete` (planted in
`96-batch.sh`). This Warrant found that gap while drafting; it was closed
inside OW-WAR-0072 because it was that Warrant's own claim.

What remains is a process that does not survive: power loss or `kill -9`
between the first record and the last. The copies live in a temporary
directory and die with the process, and nothing on disk says a batch was
half-recorded. `docs/SIGNING.md` names `git checkout` as the manual undo.

**Idempotent acts.** The journal primitive (`journal_cmd::append`) keys each
event by `sha256(uuid, type, payload)` and returns an ERROR on a repeated
key. Commands write their record first and journal it second (for example
`verify.rs`, which writes the verification file and then the
`verification.recorded` event). A retry of the same act can therefore:

- rewrite the record;
- fail on the journal;
- exit non-zero after doing the work.

§67.4 asks the opposite: an equivalent retry replays the first result, and a
conflicting reuse is rejected. No command's retry behaviour has been
inventoried or planted.

**Version negotiation.** docs/COMPATIBILITY.md says what a later 1.x reads
from 1.0. It says nothing about the reverse:

- an older `war` meeting a record, or a repository, written by a newer one;
- records that `deny_unknown_fields` (batch, attestation, drafting v2,
  roadmap and others), which refuse even an optional extension, where §69.4
  asks that unknown optional namespaced extensions be preserved;
- a newer-major record, which surfaces as each reader's own parse error.
  Nothing says "this repository needs war ≥ X" in one place.

## Desired Outcome

- **An interrupted batch is found, named and undone.** A crash between the
  first record and the last leaves a durable `.recording` marker listing
  each path and its prior sha256; the next batch, and `war check`, name it
  (`batch.interrupted`), and `war sign --batch --recover <id>` restores the
  prior bytes. In-process failures are already all or nothing (77f8fc6e).
- **An equivalent retry of an act replays.** It writes nothing new, exits 0,
  and says it replayed. A conflicting retry is refused and writes nothing.
  This is built once, in the journal primitive, and each act is planted for
  it.
- **`war` and its records negotiate in one place,** by the mechanism the
  owner picks in U-001. The Work Order is drafted for the recommended
  option.

## Non-goals

- Changing what a batch may contain, or the batch record
  (`oh.war/batch/v1`). OW-WAR-0072 owns both.
- Knowledge Fabric idempotency keys (`kf.rs`, §67). KF's side is Phase 4.
- Katana or BLUT replay (§52.2, §91.9 tests 59–63). OW-WAR-0045 cites those
  tests for the runtime. This Warrant covers `war`'s own file-native acts.
- A `v2` of any record, or `war migrate` work.
- The schema pack and its versioning (§64, §69.1–§69.3). OW-WAR-0032 owns
  them.
- An fsync policy. §86 asks for fsync only "where required by policy", and
  no policy requires it yet.
