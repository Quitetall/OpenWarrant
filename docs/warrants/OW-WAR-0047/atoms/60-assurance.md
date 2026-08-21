---
schema: oh.war/atom/v1
warrant_uuid: 01a021a6-0dfd-7e45-b33b-749b95409cd1
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a real BLUT execution produced real receipts
- **scope:** one stage graph, one execution.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **external system:** BLUT, by exact commit.
- **evidence:** status, artifact and lineage receipts returned by BLUT, with the
  registry digest recorded and the backend and stage identities pinned.

### OBL-002 — an incompatible lowering was refused
- **scope:** §49.2 and §91.7 test 47.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant offering an incompatible port kind, refused by
  `LoweringRejected` naming the port, against the shipped binary. A refusal is
  the evidence; a successful lowering is not.

### OBL-003 — BLUT's lineage was referenced, not reproduced
- **scope:** §49.3, for the executed stage.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the Warrant carries a `lineage_ref` and a search of
  its committed bytes finds no lineage events or graph. A second copy of an
  authoritative fact is the defect being tested for.

## Gate Adequacy

Required at `basic` only, so §39.4 does not compel a review. Asked
anyway, briefly, because the answer is short.

**Could this pass while BLUT authority is duplicated?** Only by copying lineage
into the Warrant, which OBL-003 tests for directly by searching the committed
bytes. That is a mechanical check on a mechanical failure, and unusually for this
project, it is close to complete.

## Residual Risk

BLUT's PlanSpec may change shape faster than the adapter. The pinned
registry digest converts that into a refusal rather than a silent remap, which
trades availability for correctness deliberately.
