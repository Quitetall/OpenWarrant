---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5eca-7b20-86b5-8a94924160af
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

SAS §86 says file-native commands SHALL use temporary files, fsync where
policy requires, atomic rename, prestate digest checks, and no partial
generated parent publication. §87.2 says controlled writes SHALL avoid
symlink races. The product spec lists "local storage, immutable artifact
retention/export, crash recovery" and "retained artifact migration" among
the engineering contracts still to specify.

What the code does on 2026-09-23:

- **Some writers are atomic, one at a time.** Temp file, fsync and rename
  are hand-written in `authority_cmd/store.rs` (with a directory fsync),
  `progress_viewer.rs`, `projects.rs`, `install.rs`, `run_cmd.rs`,
  `perform.rs`, `sdk.rs` and `document/draft.rs`. There is no shared
  helper.
- **The records that carry authority are plain `fs::write`:**
  `authorization.toml` and `judgments.toml` (`authorize.rs`
  `write_toml`), `resolution.toml` (`resolution_cmd.rs`), verification
  records (`verify.rs`), gate receipts (`gate_cmd.rs`), and every
  generated view and corpus projection (`compile.rs`). A crash mid-write
  leaves a truncated record in place. For `compile.rs` that is exactly the
  partial generated parent §86 forbids.
- **The journal is appended without fsync.** A torn final line is reported
  as `journal.malformed`, the same finding as a corrupted middle line, and
  every later append to that Warrant is refused. Nothing says the damage is
  only an incomplete last write, or what to do.
- **A leftover temporary file is invisible.** A planted
  `authorization.toml.tmp` in a Warrant directory produced no finding.
- **No write checks its prestate.** A record changed by another process
  between read and write is overwritten.

## Desired Outcome

- **One write path.** `atomic.rs` writes a record the §86 way: a temp file
  created exclusively in the target's directory, write, fsync, rename,
  fsync of the directory. It refuses a symlinked target. An optional
  prestate digest makes it refuse, by name, when the target changed since it
  was read.
- **The authority-bearing writers use it:** authorization, judgments,
  resolution, verification, gate receipts, generated views and corpus
  projections, and the response drafts `war sign` writes before renaming.
- **Journal appends are durable:** fsync after each append.
- **Crash damage is named, not guessed at.** `war check` reports:
  - `journal.torn-tail`: the final line is unparseable and has no
    newline. The message gives the byte offset and the exact truncation
    that removes only the unfinished bytes. A bad line anywhere else stays
    `journal.malformed`.
  - `storage.stray-temp` (warning): a temp file of the helper's naming is
    left under `docs/`. It names the file and the record it was meant to
    replace.
- **`docs/STORAGE.md`** states the write protocol, each crash point and
  what `war check` says after it, and the rule for retained artifacts (see
  Non-goals).

## Non-goals

- **Migration tooling for retained artifacts.** This Warrant writes down one
  rule in `docs/STORAGE.md`: a signed or resolved record is never
  rewritten to move it to a new schema; a reader accepts every schema
  version committed in the repository or refuses it by name. It also lists
  the record schemas present at HEAD. A migration command, and readers for
  versions not yet written, are a later Warrant. That work depends on
  OW-WAR-0032 (schema pack) and OW-WAR-0111 (preservation archives).
- A repair command. `journal.torn-tail` names the truncation; running it
  is a person's act. Whether `war` should repair it is the owner's
  question (20-basis U-001).
- Atomicity across several files. An act that writes a record, a journal
  line and an attestation can still stop between them. Each file is whole,
  and `war check` already reports several of the mismatches. A
  transaction across files is Knowledge Fabric's (§86 items 1–7) or
  Liminal's (§86, last sentence), not a new coordinator here.
- The other 200-odd `fs::write` calls: tests, scaffolding in `init`, and
  one-off outputs. They move only if they write a record.
