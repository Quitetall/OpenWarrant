---
schema: oh.war/atom/v1
warrant_uuid: 01a0ad2f-c0fd-7471-a00e-c7a9eb8f89d1
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

### OBL-001 — previous authority governs transitions
- **scope:** public SDK and canonical candidate records exercised by authority_transition tests.
- **evidence:** authorized update/recovery pass; self-grants, wrong parent/repository/sequence,
  malformed keys/records, empty signers and oversized input refuse with named diagnostics.

### OBL-002 — CLI verifies and activates exact signed proposals
- **scope:** disposable Ed25519 keys and local CLI store fixtures on Linux.
- **evidence:** approval and reload succeed; unsigned, forged, changed, replayed and concurrent
  updates refuse; fake PATH verifier cannot turn invalid signature into success.

### OBL-003 — storage and isolation claims remain bounded
- **scope:** reference CLI and Linux sandbox fixture.
- **evidence:** same-account protected bootstrap refuses; unprotected test mode is labeled;
  task writes succeed inside sandbox while outside authority bytes remain inaccessible.
  No claim about deployed host account ACLs, root access or actual human key custody.

### OBL-004 — migration preserves history and ergonomic drafting
- **scope:** CLI fixtures and operator commands.
- **evidence:** exact legacy bytes export unchanged; no automatic grants; draft/propose are
  inert; one approve --activate invocation completes signed activation in fixture mode.

Independent verdicts and human assurance remain separate from performer test reports.
