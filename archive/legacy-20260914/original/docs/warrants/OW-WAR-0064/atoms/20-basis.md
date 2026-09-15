---
schema: oh.war/atom/v1
warrant_uuid: 01a06a12-0aa2-7503-b589-67cf75905be4
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing text

- SAS v0.1.0-draft.3, sha256 `742dfd066b8df579116ebbd36e19a4b57dc08` (prefix;
  the revision record carries the full digest).
- §28.3 proposal snapshot; §28.5 contract digest; §31 amendments; §34.4
  architecture-change discovery; §44.5 required passing result; §56.2
  resolution records; §92 conformance.
- RQ-020, RQ-084.

## The shape already exists, for a different object

§34.4 answers this question for REQUIREMENTS: open an ADR, propose a controlled
revision, supersede or amend affected WARs, and *preserve the original
requirement and evidence history*. Supersede rather than erase.

Nothing says what the same four steps are for a DELIVERABLE. That asymmetry is
the gap, and §34.4 is the model to follow rather than a precedent to invent
around.

## Measured on 2026-09-03

- `deliverable.digest-drift` is an ERROR in `war check`, with the remedy stated
  as "regenerate the record, or restore the artifact". Neither is available for
  a resolved Warrant: regenerating moves `artifact_manifest_digest`, which the
  resolution record pins.
- Three defects blocked in one day, in three files, across FOUR resolved
  Warrants: 0056 (dispatch.rs), 0058 (sas.rs), and 0055 with 0057, which pin
  status.rs between them — one defect touching two Warrants.
- 29 Warrants are resolved. Every file they pin is in the same position.

## A distinction that came from outside

Knowledge Fabric's reading, recorded because it may shape the act: the three
defects differ in KIND. A behaviour change (the `"OW"` fallback) alters what
already-accepted records mean. Adding a refusal (validating `effective_time`)
cannot invalidate a previously accepted record — it can only stop the next bad
one. A correction path may legitimately admit the second more readily than the
first.

## Assumptions carried in

- The resolution record stays immutable and stays satisfied. Only the artifact
  pin is superseded.
- A correction is authored, authorized and recorded by the same two-half seam as
  every other authority act (§27.2): an agent may request, only a human may
  grant.
