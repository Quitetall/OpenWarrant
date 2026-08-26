---
schema: oh.war/atom/v1
warrant_uuid: 01a03d4e-1fce-7952-8d99-048355e95d11
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work order

No required order. Schedule by contention and calendar. What follows is per-ADR
scope and its check, not a sequence.

## 0077 — paper venue (calendar-driven, schedule first)

Pivot to IEEE TBME, hardware-first framing. TBioCAS declined it as algorithmic
rather than circuits, so the reframing is the work, not the resubmission.

Verify: `gate_cmd` passes; submission artifacts build from the composed doc tree
rather than a hand-maintained copy.

## 0116 — Optimum v2 as a frozen learned lossless peer

GPU-bound. Do not run concurrently with 0009.

Verify: `gate_cmd` passes, and the peer comparison is end-to-end — compress,
store, decompress, evaluate. Intermediate metrics are not evidence here.

## 0054 — beat H.BWC on its own datasets and metrics

Read the BLX1 result first: it already achieves peer parity, 7/7 corpora, 2.457%
smaller, and SUPERSEDES the earlier 5/8 niche verdict. Scope 0054 against that,
not against the older framing.

Verify: `gate_cmd` passes. IP disclosure filed BEFORE any publication.

## 0009 — latent dimension scaling

GPU-bound. Do not run concurrently with 0116.

Decoder capacity scales WHEN JOINT — tier7 (0.59) beats tier5 (0.56). The
frozen-encoder saturation result was an ARTIFACT; do not re-derive the ceiling
from it.

Verify: decide whether a `gate_cmd` is warranted on the merits. If the claim is
a scaling curve, the gate is the curve's reproduction, not a single point.

## 0015 — Eagle validation platform spec

Concordance is codec-agnostic (CSP+LDA motor imagery). A delta-accuracy result
only earns a keep once the neural/lossy path actually ships — until then it is
measurement, not a decision.

Verify: decide whether a `gate_cmd` is warranted, same test as 0009.

## 0060 — research subtree ledger

Housekeeping; use it to fill gaps between GPU-bound runs.

Verify: `gate_cmd` passes; the ledger distinguishes speculation from measurement
explicitly, since that is its entire purpose.

## Standing constraints for every stage here

- Doc edits go to `docs/atoms/<subsystem>/*.md`, never to a composed parent —
  parents carry a GENERATED banner and hand-edits are clobbered. Then
  `python3 tools/doc_compose.py --build`.
- Before committing docs, run all three gates:
  `doc_tree_lint.py && doc_compose.py && doc_views.py`.
- Retire by sequester, never delete: `tools/doc_compose.py --deprecate <atom>`.
