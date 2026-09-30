---
schema: oh.war/atom/v1
warrant_uuid: 01a0b4be-13af-7540-90a8-28d5c6d7ac59
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — complete content-bearing archive
- **scope:** every populated §68.2 category in a retained fixture and real corpus sample.
- **evidence:** exact bytes/digests preserved; omission of each required populated
  category refuses, including unknown required record kinds.

### OBL-002 — real isolated round trip
- **scope:** source-detached archive import and re-export, local destination and empty compatible KF instance.
- **evidence:** canonical bytes and semantics identical; missing external evidence,
  altered bytes and caller-only reconnect claims refuse.

### OBL-003 — history and authority remain distinct
- **scope:** superseded, disputed and annulled fixture records and imported signatures.
- **evidence:** all history survives; importer grants no authority; missing trust
  evidence stays visible and cannot establish verification.

### OBL-004 — input and version safety
- **scope:** archive parsing, evidence resolution and destination writes.
- **evidence:** traversal, duplicate paths, symlinks/special files, digest mismatch,
  unsupported version, oversize input and existing-destination overwrite refuse;
  valid bounded archive still imports.

### OBL-005 — compatible delivery and honest reporting
- **scope:** legacy envelope/digest, new format decision, CLI and shared KF contract.
- **evidence:** old pinned bytes unchanged; no false round-trip success; real
  producer/consumer observations retained. Formal acceptance remains separate.
