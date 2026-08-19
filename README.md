# OpenWarrant

**WAR — Work Authorization Record.** The human noun is a **Warrant**; the CLI is
`war`.

A Warrant is one semantic work object compiled from ordered source atoms into
role-specific projections. It carries the authority for a bounded piece of work,
the basis that work was authorized on, what was actually attempted, what
independent methods observed, and what the organization concluded — in one
auditable record.

> A WAR does not claim that work is correct because an agent stopped working or a
> command exited zero.

## Status

**Phase 1, in progress.** Building the file-native compiler. Today `war init`
works; the manifest parser, canonical IR, projections, and `war check` are the
next four Warrants.

Nothing here is stable. The protocol has no allocated enterprise identifier yet
(SAS §101.5), and the canonical-JSON implementation that will bind every
cross-system digest is deliberately unchosen until its implementation ADR lands.

## Governing specification

`docs/sas/WAR_Software_Architecture_Specification.md`, v0.1.0-draft.1,
sha256 `aad5256cb59e3e589313b7e2d5b48360ad8c85cf1c1d65d21f9260e692dfe8e5`.

The copy in this repository is byte-identical to the drafted document. Section
references throughout the source (`§65.2`, `RQ-014`, …) cite it.

## Layout

```text
crates/openwarrant-core/       domain types and validators, no I/O   (SAS §79.1)
crates/openwarrant-compiler/   manifest → IR → canonicalize → render (SAS §79.2)
crates/openwarrant-agent/      planning protocol surface only        (SAS §79.3)
crates/openwarrant-cli/        the `war` binary                      (SAS §79.4)
xtask/                         cargo xtask gate                      (SAS §92)
docs/sas/                      the governing specification
docs/warrants/                 this repository's own Warrants
```

## Building

```bash
cargo build --workspace
cargo xtask gate
```

`cargo xtask gate` is the aggregate gate: fmt, clippy, the test suite, and the
license check. It exits zero only when every step passes, and it reports **every**
failing step rather than the first.

## License

**AGPL-3.0-or-later** today. The intent is to relicense to **Apache-2.0** when
this goes public.

That intent constrains the code now, not later. Every dependency must be MIT
and/or Apache-2.0 — a copyleft dependency adopted today could not be relicensed
afterwards — and `cargo deny check licenses` enforces it as part of the gate
rather than as a review habit. Copyright is kept consolidated for the same
reason: a relicense is only yours to make if the copyright is.
