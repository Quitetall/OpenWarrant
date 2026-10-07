---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5eca-7b20-86b5-8a94924160af
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `crates/openwarrant-cli/src/atomic.rs` (new):
   - `write(path, bytes)`: exclusive temp file
     `.<name>.<pid>.<random>.war-tmp` in the target's directory, write,
     fsync, rename, fsync the directory.
   - Refuses when the target is a symlink (`storage.symlink-target`).
   - `write_if(path, bytes, prestate)`: refuses with
     `storage.prestate-moved` when the target's current SHA-256 differs
     from `prestate`, or when it exists and `prestate` says absent.
   - `stray(root)`: lists leftover `.war-tmp` files under `docs/`.
   - The debug-only fault hook: `OPENWARRANT_FAULT=after-temp` aborts
     after the temp file is written and before the rename;
     `OPENWARRANT_FAULT=pause-before-rename:<ms>` sleeps there.
2. Writers routed through it, each reading the prestate it later writes:
   - `authorize.rs`: `write_toml` (authorization, judgments);
   - `resolution_cmd.rs`: `resolution.toml`;
   - `verify.rs`: verification records;
   - `gate_cmd.rs`: gate receipts;
   - `compile.rs`: generated views and corpus projections;
   - `sign.rs`: the response draft written before its rename.
3. `journal_cmd.rs`:
   - fsync after each append;
   - `journal.torn-tail` (error) for an unparseable final line with no
     newline, naming the byte offset and the `truncate -s` that removes
     only it; any other bad line stays `journal.malformed`;
   - an append to a Warrant whose journal has a torn tail is refused, as
     today, now naming `journal.torn-tail`.
4. `check.rs`: `storage.stray-temp` (warning) for each file
   `atomic::stray` returns.
5. `docs/STORAGE.md`: the protocol; each crash point with what is on disk
   after it and what `war check` says; the retained-artifact rule and the
   list of record schemas present at HEAD (10-intent, Non-goals).
6. `conformance/plants.d/53-storage.sh`, on scratch programs with the
   debug binary:
   - `OPENWARRANT_FAULT=after-temp` during `war sign <alias>
     --ssh-sign` (throwaway key): `authorization.toml` is absent or
     byte-identical to before, and `war check` reports
     `storage.stray-temp` naming the temp file;
   - the same during `war compile`: every committed generated view is
     byte-identical to before, and `war check --generated` reports no
     partial file;
   - `pause-before-rename` during `war sign <alias> --ssh-sign`, with the
     plant writing its own bytes to `authorization.toml` meanwhile: the
     command exits non-zero with `storage.prestate-moved` and the plant's
     bytes survive;
   - `authorization.toml` replaced by a symlink before `war sign`:
     refused with `storage.symlink-target`, the link's target unchanged;
   - a torn final journal line gives `journal.torn-tail` with the right
     offset; a bad middle line gives `journal.malformed`; running the
     named `truncate` makes `war check` pass again;
   - a grep finds no `fs::write(` outside `#[cfg(test)]` code in the six
     routed files for a record path.

## Frozen Surfaces

Every record schema and its bytes on disk (the helper changes how a file is
written, not what); `oh.war/report/v1`; the signing seam and what a
signature covers; `journal.rewritten` and `journal.malformed` as they
stand for committed lines.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:
- any change to what a record contains;
- a repair command for the journal (20-basis U-001);
- moving the fault hook out of `cfg(debug_assertions)`;
- routing any writer not listed in Deliverables.

## Rollback

Revert the routed writers to `fs::write` and remove `atomic.rs`, the
`storage.stray-temp` rule and `journal.torn-tail`. Records written in
the meantime are byte-identical to what `fs::write` would have written, so
nothing on disk needs to change.
