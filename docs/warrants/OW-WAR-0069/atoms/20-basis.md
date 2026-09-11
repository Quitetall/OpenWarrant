---
schema: oh.war/atom/v1
warrant_uuid: 01a09274-78ce-74b8-b6e8-7b8ea92afa04
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

- Governing source: `docs/sas/WAR_Software_Architecture_Specification.md`.
- §14 Workspace Basis — the enumerated compilation inputs and the prohibition on
  silently mixing independently changing ones.
- §65 Digest domains — the `ALL` set and §65.2 domain-separated preimages.
- §33 Context model.
- §6.10 — a Warrant is a bounded intervention inside one program; this Warrant is
  an OpenWarrant tool change, not a change to any consuming program.
- OW-ADR-0003 — atoms are restricted YAML and refuse nested maps, which is why
  pin data is TOML alongside `deliverables.toml` rather than an atom.
- Branch base: `feat/battery-split` at `77953b9`, which carries the
  `conformance/plant.sh` split into `lib.sh` + `plants.d/`.

## Evidence from a consuming program

The need is observed, not hypothetical. The WeaponsOfMageDestruction platform
repository pins two submodules through `integration.lock.toml`: two revisions,
two different Rust toolchains (1.96.0 and 1.97.1), an interface schema version,
a named determinism profile, an adapter quantization schema, and SHA-256 digests
of three source packs. Its `check-pins.py` enforces 40-hex revisions and refuses
`stable`, `nightly`, and version ranges.

That gate caught a real divergence: a merge proceeded while local `main` was two
commits behind its remote, and the pin check failed loudly rather than letting a
stale revision propagate into a release record.

What that file cannot do is the gap this Warrant closes: it fails a gate, but no
contract digest moves, so no record reads as amended when a pin changes.

## Prerequisites

None blocking. OW-WAR-0064 (`war correct`) is on the base branch but is not a
prerequisite: this Warrant adds new files and one additive enum variant, and
touches no file pinned by a resolved Warrant.

## Blocking unknowns

- U-001: whether adding a `DigestDomain` variant is an architecture-changing
  revision under §101.3 and therefore requires an ADR before the SAS revision is
  accepted. Narrowed: the reviewing session determined that it is — a new digest
  domain changes what can be digested, which is protocol meaning. The ADR
  deliverable is retained on that basis. This is a reader determination recorded
  as such, not a disposition; §101.3 remains the owner's to apply at acceptance.
