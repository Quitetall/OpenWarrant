---
schema: oh.war/atom/v1
warrant_uuid: 01a03d4e-1f2d-75b3-8b0e-2a62c2acab59
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — gate discovery remains complete through every consolidation wave
- **scope:** `ci_local.GATE_GLOBS`, coverage inventory, and every gate-shaped
  tool moved from a former submodule root.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a negative control planted below a merged path is reported as
  orphaned before declaration; the same inventory passes after declaration.

### OBL-002 — the Knowledge Fabric oracle observes the same LamQuant contract before and after collapse
- **scope:** `lamquant-compat` code prefixes, required tools, Git materialization,
  and all thirteen pre-collapse module identities.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** bound pre-collapse and post-wave oracle receipts with identical
  required-observation coverage; missing Git links or merged paths fail closed.

### OBL-003 — every history wave is projection-preserving
- **scope:** each component merged under the ordered `josh-filter :prefix=`
  sequence in the work order.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** one exact-commit `verify_projection.py --strict` receipt per
  wave, plus a final extraction comparison covering all former repositories.

### OBL-004 — collapse removes dead pins without weakening surviving external boundaries
- **scope:** Git links and dependency pins retired by consolidation; BLUT,
  OpenECS, and liblsl remain external as ordered.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** pins and locks gates pass, one integration round converges, and
  no dependency to a merged component remains as a remote revision pin.

### OBL-005 — direction and exposure debt reach measured zero
- **scope:** all production edges declared in `modules.toml` and all three
  ceilings in `tools/module-ceilings.json`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** strict direction and exposure reports contain zero layer
  inversions, zero cycles, zero exposure edges, and zero exposure pin sites;
  ceilings are lowered to those measured values.

### OBL-006 — closure records follow evidence and preserve authority boundaries
- **scope:** ADR 0185 completion append, final repository rename, module-owner
  references, and Knowledge Fabric configuration.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** human-authorized completion follows OBL-001 through OBL-005;
  rename references resolve at exact revisions and no earlier ADR text is
  rewritten.

## Gate Adequacy

Required at `basic` while this Warrant remains an implementation draft. Wave
authorization, ceiling-zero acceptance, ADR completion, and repository rename
remain human decisions; agent-generated reports cannot satisfy those acts.
