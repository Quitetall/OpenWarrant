---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-7f2a-8e39-69730f255e33
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. A Git repository at `Quitetall/OpenWarrant`, private, AGPL-3.0-or-later.
2. A Cargo workspace with the four v0 crates named in SAS §78.
3. `rust-toolchain.toml` pinning an exact version with `rustfmt` and `clippy`.
4. `deny.toml` allowing permissive licenses only, invoked by the gate.
5. `cargo xtask gate` — the aggregate gate of SAS §92.
6. `.github/workflows/ci.yml` exposing one stable required check named `gate`.
7. `war init`, working end to end.
8. The SAS imported byte-identical under `docs/sas/`.

## Frozen Surfaces

- The crate names and their assigned responsibilities. Renaming or merging a
  crate moves an authority boundary and requires an ADR.
- The digest-domain URIs in `openwarrant-compiler::digest`. They are protocol
  surface; a change to one invalidates every digest computed under it.
- The `gate` job name. It is the required status check, matched by name.

## Premade Instructions

- Split crates only at stable authority seams (§78). Do not create the remaining
  six crates from the eventual layout as empty shells.
- Do not implement RFC 8785 canonicalization under this Warrant.
- Every dependency added must be MIT and/or Apache-2.0.

## Resources and Capabilities

Local filesystem write within the repository, network for crate resolution, and
GitHub API access to create the repository. No secrets.

## Autonomy and Escalation

Tier T2 — implementation judgment. Library choices from the §80 recommendation
table are local decisions and do not require a new ADR, per §30.1, because §80
declares them nonbinding. A library that binds the wire format is NOT such a
choice and escalates.

## Rollback

Delete the repository. Nothing depends on it: OpenWarrant is a peer tool, not a
LamQuant submodule, and no other repository pins it.
