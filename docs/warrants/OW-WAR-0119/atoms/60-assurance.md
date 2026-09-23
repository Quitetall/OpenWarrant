---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5e99-7663-90dd-3063ec23be35
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — one UUID names one Warrant, and one ADR
- **scope:** `identity.duplicate-uuid` over the Warrant manifests and ADR
  atoms of a scratch program, and of this repository's corpus.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - a Warrant directory copied under a new alias with its UUID unchanged
    makes `war check` exit non-zero with `identity.duplicate-uuid`
    naming both aliases;
  - two ADR atoms with one `adr_uuid` do the same;
  - the clean program and this corpus produce no such finding.

### OBL-002 — an atom belongs to its manifest's Warrant
- **scope:** `identity.atom-mismatch` over Markdown atoms.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** one atom's `warrant_uuid` edited to another Warrant's
  UUID gives `identity.atom-mismatch` naming the atom path. Restored, the
  finding is gone.

### OBL-003 — a committed UUID does not change
- **scope:** `identity.changed` against `HEAD`, in a scratch program
  with a committed Warrant.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - the manifest UUID replaced by a fresh UUIDv7, journal and atoms left
    alone, gives `identity.changed` naming both UUIDs;
  - a new, uncommitted Warrant gives no `identity.changed`;
  - the same check with no git repository reports `UNKNOWN`, not pass.

### OBL-004 — an alias is refused where an identity is required
- **scope:** `war://` references in `[[parents]]`, `[[supersedes]]`
  and ADR `governs`; `war blut`'s lowering.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - `[[supersedes]] ref = "war://<alias>"` and the same in
    `[[parents]]` each give an error `identity.alias-ref` whose message
    contains the UUID that alias has here;
  - an ADR `governs` entry in alias form gives the same rule as a
    warning, and `war check` on this corpus lists the six existing ones;
  - the `blut.rs` unit test asserts `stage_identity` parses as
    `war://` followed by a UUIDv7.

### OBL-005 — ADR identities are reported, never rewritten
- **scope:** `identity.adr-not-v7` on this repository's 23 ADR atoms.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - `war check --json` on this corpus lists `identity.adr-not-v7` once
    for each of the 13 ADRs with a v4 `adr_uuid`, and for no v7 one;
  - `git diff` over `docs/adr/` is empty after the work.

## Gate Adequacy

Required at `basic`. The load-bearing obligation is OBL-001: two records
with one identity make every `war://` reference, every pin and every
journal event ambiguous, and today nothing refuses it.
