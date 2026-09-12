---
schema: oh.war/atom/v1
warrant_uuid: 01a09482-1257-7b62-aa9d-f559317d167a
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `LICENSE` (Apache-2.0 text), `NOTICE`, `Cargo.toml` `license = "Apache-2.0"`,
   every `SPDX-License-Identifier` header, `docs/adr/atoms/OW-ADR-0017-relicense-apache-2-0.md`.
2. `docs/sas/revisions/1.0.0.toml` in state `accepted`, with its attestation.
3. `authorization.toml` for OW-WAR-0064, OW-WAR-0065, OW-WAR-0066.
4. `docs/warrants/OW-WAR-0042/` applied from `evidence/run-1/proposal.json`,
   verified blind, resolved.
5. `docs/authority/allowed_signers` with `oh.war/dsse` in the principal's namespaces.
6. One `corrections/<D-id>-<n>.toml` per drifted deliverable of a resolved Warrant.
7. The tag `v1.0.0` on the commit that carries 1–6.

## Frozen Surfaces

Every record already signed. Nothing here edits a signed record; corrections
supersede.

## Premade Instructions

- Order: 1 first (bytes move), then 5, then 2, 3, 4, then 6 on final bytes,
  then 7. `war sign --list` is the queue; `war sign --all --ssh-sign` walks
  it one dialog at a time.
- Each `--meaning` may say the decision was taken remotely on 2026-09-12; the
  signature is what makes it an act.
- After 1 the performer finishes the relicense mechanically and re-pins every
  unresolved Warrant on final bytes before 6.

## Autonomy and Escalation

Tier T3: every stage is a human's. The performer prepares, never signs.

## Rollback

Nothing to roll back: an unsigned stage leaves no record. A signed act is
superseded, never erased (§34.4).
