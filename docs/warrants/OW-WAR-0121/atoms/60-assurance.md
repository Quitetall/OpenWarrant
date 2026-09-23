---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5eca-7b20-86b5-8a94924160af
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a crash mid-write never leaves a partial record
- **scope:** the six routed writers, with the debug binary and
  `OPENWARRANT_FAULT=after-temp`, on scratch programs, Linux. No claim for
  a crash inside the kernel's rename, or for another OS.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - during `war sign <alias> --ssh-sign`, `authorization.toml` is absent
    or byte-identical to before;
  - during `war compile`, every committed file under `generated/` and the
    corpus projections is byte-identical to before;
  - in both, `war check` then reports `storage.stray-temp` naming the
    temp file.

### OBL-002 — a record changed since it was read is not overwritten
- **scope:** `atomic::write_if` as used by `authorize.rs` when `war sign
  <alias> --ssh-sign` records an authorization (throwaway key, scratch
  program), with `pause-before-rename`. The same helper serves
  `resolution_cmd.rs`; no separate claim is made for it.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** the plant writes its own bytes to `authorization.toml`
  during the pause; the command exits non-zero with
  `storage.prestate-moved`, and the plant's bytes are still there.

### OBL-003 — a symlinked target is refused
- **scope:** `atomic::write`, exercised through `war sign` on a scratch
  program.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** with `authorization.toml` a symlink to a file outside the
  program, `war sign` reports `storage.symlink-target` and the linked
  file's bytes are unchanged.

### OBL-004 — a torn journal tail is named, and only it
- **scope:** `journal_cmd.rs`'s check on a scratch program.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - a partial event appended with no newline gives `journal.torn-tail`
    with the byte offset where it starts;
  - the same bytes placed between two good lines give
    `journal.malformed`, not `journal.torn-tail`;
  - the truncation the message names makes `war check` pass, and no
    parsed event is lost.

### OBL-005 — every authority-bearing writer goes through the helper
- **scope:** `authorize.rs`, `resolution_cmd.rs`, `verify.rs`,
  `gate_cmd.rs`, `compile.rs`, `sign.rs` at delivery.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** the plant's grep finds no `fs::write(` for a record path
  outside `#[cfg(test)]` in those files, and fails when one is planted
  back.

## Gate Adequacy

Required at `controlled`.

**Adversarial question:** can the new write path be turned against the
records it protects? Three attacks, each a plant that must be seen to
refuse: a symlink placed where a signed record will be written (OBL-003); a
record changed by another process while an act runs (OBL-002); and the
fault hook used to leave a half-written record that a later command reads
as whole (OBL-001, where the destination is checked byte for byte).

**Second adversarial question:** does crash recovery ever delete history?
`journal.torn-tail` fires only on an unparseable last line with no newline,
the plant shows a bad middle line is not treated so, and the tool itself
truncates nothing.

- **outcome:** gate_added

## Residual Risk

- A crash between the files of one act leaves each file whole and the set
  inconsistent (20-basis R-001).
- macOS and Windows semantics are assumed, not exercised.
- The fault hook is product code in debug builds (20-basis R-003).
