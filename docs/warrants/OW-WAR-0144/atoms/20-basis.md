---
schema: oh.war/atom/v1
warrant_uuid: 01a0d289-1e9a-7621-b9fe-dc0738889647
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- RQ-030: authorized contract revisions are immutable. RQ-034: prior
  attempts retain their original contract basis.
- §34.4 (supersede, never erase), as `sign.rs`'s `retire_prior` already
  applies it to responses.
- `attest.rs`'s `archived_copy`: an attested subject whose path now holds
  other bytes is still verified when a file beside it, starting with the
  same stem, carries the attested digest.
- OW-ADR-0021: this Warrant declares `authorize.rs`; on authorization it
  governs that file.

## Assumptions

- A-001: `authorization.<digest8>.toml` is found by `archived_copy` (it
  starts with the stem `authorization`) and is not read as a second
  authorization by any loader. The plant checks the second half on
  `war check`. Confidence: high.
- A-002: eight hex digits do not collide between two revisions of one
  Warrant; on a collision the write is refused, never overwritten.
  Confidence: high.

## Residual risks

- R-001: a process killed between keeping the old file and writing the new
  one leaves both the kept copy and the old `authorization.toml`; that is
  the prior state plus a duplicate, never a loss. Crash recovery in
  general is OW-WAR-0130's.
