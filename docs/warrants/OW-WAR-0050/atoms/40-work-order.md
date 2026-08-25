---
schema: oh.war/atom/v1
warrant_uuid: 01a0399d-05b9-7ad0-b8dc-bf1a226fa641
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. Optional `scope.toml` enters the Compilation Basis, canonical IR, contract
   digest, and generated Warrant view.
2. `war bonsai check --warrant <alias> --base <sha> --head <sha> --bonsai <path>`
   emits `oh.war/bonsai-evidence/v1`.
3. A pilot `bonsai.toml` protects pure domain/compiler packages from process or
   network escape hatches.
4. Pull-request workflow uploads report-phase evidence and parses a named
   Warrant from the PR template.
5. Architecture, evidence, and rollout ADRs record the seams and human-owned
   settings still required before blocking rollout.

## Frozen Surfaces

The Bonsai executable interface stays generic. OpenWarrant owns this adapter;
no Warrant may provide a command string or executable path.

## Rollback

Remove the optional sidecar and adapter. Existing Warrants remain compilable
without scope, and the CI job is report-only during pilot qualification.
