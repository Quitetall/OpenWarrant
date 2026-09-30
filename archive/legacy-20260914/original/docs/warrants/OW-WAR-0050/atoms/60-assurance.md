---
schema: oh.war/atom/v1
warrant_uuid: 01a0399d-05b9-7ad0-b8dc-bf1a226fa641
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — scope changes move the Warrant contract
- **scope:** compiler Basis, canonical IR, and generated Warrant projection.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a positive control proving a sidecar changes contract digest;
  generated-view drift check proves it cannot be edited separately.

### OBL-002 — evidence binds exact candidate and machine policy
- **scope:** `war bonsai check`, scope parser, git binding, JSON evidence.
- **gate:** `gate://software.repo.bonsai-evidence@1.0.0`
- **evidence:** plants for out-of-scope diff, policy-digest drift, non-HEAD
  candidate, unavailable Bonsai, and clean candidate.

### OBL-003 — only scope and architecture block the pilot
- **scope:** Bonsai policy and adapter finding filter.
- **gate:** `gate://software.repo.bonsai-evidence@1.0.0`
- **evidence:** plant showing `contract-*` error fails while a non-architecture
  error remains recorded but non-blocking.

### OBL-004 — PR workflow remains report-only pending qualification
- **scope:** PR template, CI workflow, and rollout ADR.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** report phase builds the adapter from protected base, materializes
  the candidate only as data, then clones canonical public Bonsai at Warrant-
  bound full revision, verifies its checkout, and runs a locked build without a
  private credential. Missing candidate, source, revision, build, executable,
  or machine result is typed `unknown`. Blocking requires an external human
  review and
  administrator-owned GitHub settings; those records remain outside this draft
  Warrant.

## Gate Adequacy

Required at `basic`: this is an implementation draft with no authorization or
resolution. Promotion to blocking enforcement requires an independent verifier
and human administrator confirmation of GitHub settings.
