---
schema: oh.war/atom/v1
warrant_uuid: 01a0983e-32d9-76e0-8417-2d69773249ca
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the signed subject is the list, canonically
- **scope:** `openwarrant-core/src/batch.rs`, `docs/authority/batches/`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** two drafts of the same act set produce byte-identical documents and the same digest; the digest is over RFC 8785 canonical JSON under its own `DigestDomain`; a `--verify` of the signature fails on a one-byte edit to the document (planted).

### OBL-002 — a batch is refused whole, and refused before any write
- **scope:** the nine refusals in the work order.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant per refusal; after each, `git status` is clean and no record, journal event or attestation exists. `batch.digest-moved` refuses a two-act batch when only the second act's digest moved, and names both digests.

### OBL-003 — a batch grants no authority a single act would not
- **scope:** the ingest loop, `sign.rs`'s per-act ingests.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** an agent-kind signer is refused by `batch.agent`; an act the signer's role cannot perform is refused by `batch.not-permitted`; a batch naming an act absent from `war sign --list` is refused by `batch.unknown-act`; each written record is byte-identical to the record the same act signed alone, plus `batch_ref`.

### OBL-004 — a foreign verifier can check a batch both ways
- **scope:** `attest.rs`, `docs/SIGNING.md`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war attest verify` accepts the batch envelope and each per-act envelope; a per-act envelope whose batch digest was edited is rejected by `attest.subject-drift`; a batch signature replayed under `oh.war/dsse` is rejected (the §85 namespace rule, planted).

## Gate Adequacy

Required at `basic`. The refusals are the deliverable; the happy path is one
line of it.

**Adversarial question:** does this let an agent sign? The signature is
`ssh-keygen -Y sign` against a key in the agent socket, exactly as before, and
`batch.agent` refuses an agent-kind signer by name. What changes is how many
dialogs one human decision costs — one, over a document that lists what they
decided.

**Second adversarial question:** could a batch be re-scoped between drafting
and signing? Only by editing the document, which invalidates the signature, or
by a bound digest moving, which refuses the whole batch. Both are planted.
