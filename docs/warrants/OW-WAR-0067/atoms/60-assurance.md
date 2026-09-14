---
schema: oh.war/atom/v1
warrant_uuid: 01a09482-1257-7b62-aa9d-f559317d167a
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the tree is relicensed and the register names the attestation namespace
- **scope:** `LICENSE`, `NOTICE`, `Cargo.toml`, every SPDX header, `docs/authority/allowed_signers`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `cargo xtask gate`'s spdx and licences steps green; `war attest --all` green with the new namespace.

### OBL-002 — every signature is the owner's, made at the workstation
- **scope:** the SAS acceptance, three authorizations, and OW-WAR-0042's resolution.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** each response's `.sig` verifies under `war sign --verify`; each act has a DSSE attestation; the journal records them.

### OBL-003 — corrections cover every drift and the tag exists
- **scope:** every `deliverable.digest-drift` on a resolved Warrant; `v1.0.0`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war check` reports zero drift; the battery reads all plants green; `git tag -l v1.0.0`.

## Gate Adequacy

Required at `basic`.

**Adversarial question:** could an agent perform any stage here and have it
pass? No: `war sign` needs the key, the key needs the dialog, and a response
without a `.sig` fails `--verify`. The performer's own attempt on 2026-09-12
is on record as refused.

- **outcome:** no_counterexample

## Residual Risk

- The owner may sign out of order; a correction signed before the relicense
  lands is superseded by another. Cost, not damage.
