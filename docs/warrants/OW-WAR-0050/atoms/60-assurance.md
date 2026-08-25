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
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** plants for out-of-scope diff, policy-digest drift, non-HEAD
  candidate, unavailable Bonsai, and clean candidate.

### OBL-003 — only scope and architecture block the pilot
- **scope:** Bonsai policy and adapter finding filter.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** plant showing `contract-*` error fails while a non-architecture
  error remains recorded but non-blocking.

### OBL-004 — PR workflow remains report-only pending qualification
- **scope:** PR template, CI workflow, and rollout ADR.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** workflow review by named external human plus GitHub check
  settings recorded outside this Warrant.

## Gate Adequacy

Required at `basic`: this is an implementation draft with no authorization or
resolution. Promotion to blocking enforcement requires an independent verifier
and human administrator confirmation of GitHub settings.
