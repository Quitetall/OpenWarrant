---
schema: oh.war/atom/v1
warrant_uuid: 01a09351-e725-73b2-8960-1d263af327da
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the memo's numbers are measured, not asserted
- **scope:** every figure in `docs/research/tokenizer-approximation.md`.
- **gate:** `gate://document.review@1.0.0`
- **evidence:** each figure has the command that produced it beside it, and a reviewer re-ran at least one.

### OBL-002 — the decision follows from the numbers
- **scope:** the memo's Decision section against its Measurements section.
- **gate:** `gate://document.review@1.0.0`
- **evidence:** an independent reviewer records `established` for this obligation; the gate refuses closure without it.

## Gate Adequacy

Required at `basic`.

**Adversarial question:** can a memo pass this gate while saying nothing?
Not with placeholders (refused), not without citations that exist, not
without a reviewer who is not the author; it can pass while being wrong,
which is what OBL-002's reviewer is for.

- **outcome:** no_counterexample

## Residual Risk

- The gate does not judge the argument; the reviewer does.
