---
schema: oh.war/atom/v1
warrant_uuid: 01a09482-1257-7b62-aa9d-f559317d167a
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The 1.0 plan's performer half is landed (36 reviewed commits on
`feat/battery-split`). Seven acts remain, and every one is a human's: a
licence change, four signatures of acceptance and authorization, twenty-six
corrections, a reviewed application of a recorded draft with its
verification and resolution, an edit to a human-written register, and a
release tag. An agent attempting them is refused twice over: by the key's
confirm dialog and by the harness's permission classifier.

## Desired Outcome

The seven acts are on record as stages of one Warrant, each performed by the
owner at the workstation, in an order that signs final bytes last; the tag
`v1.0.0` exists; the battery reads 264/264.

## Scope

Exactly the seven acts, in the order the work order gives. The relicense's
mechanical remainder (headers, NOTICE, ADR, docs, re-pins) is the performer's
once the owner has changed `LICENSE` and the manifest's licence string.

## Non-goals

- No act here is performed by an agent, on any instruction. The owner's
  provisional approval is recorded in `rationale.toml` as a blocking unknown
  whose resolution is the signature itself.
- Publishing to crates.io is the release workflow's, gated on the tag and the
  licence.

## SAS and Roadmap Traceability

`sas://WAR-SAS-RQ-030` (authorized Contract Revisions are immutable: the
corrections supersede, never regenerate), `sas://WAR-SAS-RQ-036`,
`sas://WAR-SAS-RQ-085`; `roadmap://OW-PHASE-9/release`.
