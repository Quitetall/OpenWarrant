---
schema: oh.war/atom/v1
warrant_uuid: 01a0d289-1e9a-7621-b9fe-dc0738889647
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — an amendment keeps the signed revision, and its attestation still verifies
- **scope:** `war sign <alias> --ssh-sign` for revision 1 and revision 2 of
  one scratch Warrant, with a throwaway key.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - after revision 2, `authorization.<digest8>.toml` exists and its sha256
    equals the revision-1 file's, byte for byte;
  - `war attest --all --verify` passes with `attest.subject-archived` for
    the revision-1 envelope;
  - control: the kept file removed, the same verify reports
    `attest.subject-drift` naming `authorization.toml` — the retention is
    what makes it pass.

### OBL-002 — nothing is overwritten and a dry run keeps nothing
- **scope:** the same scratch corpus.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - a planted `authorization.<digest8>.toml` with other bytes makes the
    revision-2 authorization refuse `authorize.retire-collision`, and no
    file changes;
  - `war sign <alias> --dry-run` for revision 2 leaves the Warrant
    directory byte-identical.

### OBL-003 — the kept file is not a second authorization
- **scope:** `war check` and `war sign --list` on the scratch corpus after
  revision 2.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** `war check` reports one authorization at revision 2 and no
  error; `war sign --list` shows no act for the Warrant's authorization.

## Gate Adequacy

Required at `basic`. OBL-001's control is the load-bearing one: without it
a verify that passes for another reason would pass the obligation.
