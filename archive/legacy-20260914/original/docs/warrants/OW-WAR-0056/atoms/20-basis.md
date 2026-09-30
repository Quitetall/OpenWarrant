---
schema: oh.war/atom/v1
warrant_uuid: 01a06025-b26f-7950-be1c-c830b2a215af
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS v0.1.0-draft.1, sha256 `aad5256cb59e3e589313b7e2d5b48360ad8c85cf1c1d65d21f9260e692dfe8e5`.
- §47.1 dispatch schema — twenty-three top-level fields, quoted in full in the SAS.
- §47.2 — eight compiler duties: select only stage-relevant context; preserve
  every required normative source; record omitted subgraphs; preserve
  provenance; enforce classification; include prior failure evidence for
  repair; produce deterministic canonical bytes; record the Dispatch digest.
- §47.3 — actor projections may differ in representation, never in contract.
- §33 context model; §52 attempt semantics; §65 digest domains.
- RQ-042, RQ-043, RQ-045.

## Measured on 2026-09-01

- `StageDispatch` (`execution.rs`) carries 20 of §47.1's fields. Missing:
  `api_version`, `workspace_basis_ref`, `context_manifest_ref`,
  `submission_schema_ref`; `capability_authorization {policy_ref, digest}` is
  flattened to a single digest; there is no `dispatch_digest`.
- `DigestDomain::Dispatch` and `::ContextManifest` are declared and never
  computed by any code path.
- `ContextManifest` is constructed only in its own tests.
- `render.rs::stage_dispatch` concatenates two atoms and does not consult
  `MilestoneGraph`.
- `blut.rs::lower` is the closest existing analogue: it walks the validated
  graph, refuses a stage with no `executor_ref`, and emits an external artifact.

## Assumptions carried in

- Context is selected from the Warrant's own atoms. The five required atoms
  are the required normative sources; optional atoms are omitted with a
  reason. Nothing outside the Warrant is pulled in — §33.6 forbids silently
  dropping a required item, and the cheapest way to honour that is to start
  from a set whose required members are all known.
- The capability policy is not modelled in this repository. The dispatch
  records `policy_ref = "policy://none-declared"` with an empty digest rather
  than a digest of nothing.
- The attempt is `initial` unless the caller says otherwise. A `repair` must
  carry prior failure evidence and `Attempt::validate` already refuses one
  that does not.
