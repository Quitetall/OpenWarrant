---
schema: oh.war/atom/v1
warrant_uuid: 01a03d4e-1f7b-7680-8fa7-f84f8dda0962
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — execution preserves the declared dependency order
- **scope:** ADRs 0159, 0158, 0074, and 0075 and their bound implementation
  revisions.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** each downstream stage cites a passing predecessor receipt; no
  downstream implementation revision predates the predecessor it consumes.

### OBL-002 — ABIR2 is the single source-agnostic biosignal boundary
- **scope:** ADR 0159's dataset, recording, stream, block, and tensor seams.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** ADR 0159's exact gate receipt plus cross-source fixtures proving
  trainers and processors do not branch on EDF, BIDS, DICOM, NWB, BCS, or LMA.

### OBL-003 — storage profiles make resource and version behavior explicit
- **scope:** ADR 0158 profiles across disk, training throughput, and bounded
  memory use.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** versioned profiles pass ADR 0158's gate against the settled 0159
  boundary; unsupported combinations fail rather than selecting hidden defaults.

### OBL-004 — production migration is complete, reproducible, and reversible
- **scope:** ADR 0074 production consumers and all retained source formats.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a clean rerun produces the same target identities, legacy inputs
  retain a declared rollback path, and no production default moves before its
  compatibility gate passes.

### OBL-005 — datapath optimization is measured end to end
- **scope:** ADR 0075 ingest through storage, reload, decode, and evaluation.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** predeclared resource profiles compare actual training time, disk
  space, and peak memory against a recorded baseline; raw receipts back every
  reported number and intermediate-only metrics do not qualify.

### OBL-006 — missing sealed inputs remain explicit blockers
- **scope:** recipes requiring a sealed BCS2 snapshot and every claim derived
  from those recipes.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** absence of a sealed snapshot produces `not-run` or `could-not-ask`,
  never a substituted unsealed input or a passing qualification claim.

## Gate Adequacy

Required at `basic` while this Warrant is an implementation draft. Boundary and
production-cutover decisions are T1 acts and require human authorization after
their evidence exists; an agent may implement and measure but cannot authorize
the transition.
