---
schema: oh.war/atom/v1
warrant_uuid: 01a03d4e-1fce-7952-8d99-048355e95d11
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — quantitative claims come from bound evidence, not session narrative
- **scope:** every rate, quality, memory, throughput, corpus, and comparison
  number produced by this Warrant.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** each claim cites a Truth Ledger entry or raw run receipt with
  exact candidate, configuration, split, corpus, and evaluator identity.

### OBL-002 — GPU-bound campaigns execute without hidden contention
- **scope:** ADR 0009 and ADR 0116 training or evaluation runs on the shared
  workstation.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** run receipts record preflight RAM, VRAM, active jobs, and CI
  state; overlapping heavyweight runs invalidate rather than merely annotate a
  campaign.

### OBL-003 — learned-lossless and H.BWC comparisons are end-to-end and bit-fair
- **scope:** ADRs 0116 and 0054 over every declared held-out corpus and peer
  configuration.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** compress, stored bytes, decompress, and evaluation are bound in
  one receipt per corpus; aggregate wins cannot substitute for corpus-complete
  results, and publication remains blocked until required IP disclosure exists.

### OBL-004 — latent scaling tests the joint-training claim rather than a superseded proxy
- **scope:** ADR 0009 candidate family, splits, capacity tiers, and scaling
  curve.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** subject-held-out joint-training runs reproduce the declared
  curve; frozen-encoder saturation and listed negative-result configurations
  cannot be retried without a recorded new falsifiable basis.

### OBL-005 — publication and validation artifacts build from authoritative sources
- **scope:** ADR 0077 paper deliverables and ADR 0015 Eagle specification.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** composed-source builds pass without hand-maintained copies;
  codec-agnostic measurements remain measurements until a shipping codec path
  satisfies its declared promotion gate.

### OBL-006 — research records preserve epistemic status
- **scope:** ADR 0060 ledger and every research artifact linked by this Warrant.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** speculation, observation, measurement, falsification, and
  decision remain distinct typed records; retirement sequesters history instead
  of deleting it.

## Gate Adequacy

Required at `basic` while this Warrant remains an implementation draft. Venue,
publication, IP-disclosure, and promotion decisions remain human acts. Agent
campaigns may produce proposals and receipts but cannot authorize those outcomes.
