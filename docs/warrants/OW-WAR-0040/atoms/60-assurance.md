---
schema: oh.war/atom/v1
warrant_uuid: 01a01bdc-5a20-77b7-800c-673a6394651b
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-000 — Liminal is available
- **scope:** a checkout on the build host.
- **evidence:** the repository is present. This obligation exists because the
  Warrant is otherwise unstartable, and saying so is cheaper than discovering it.

### OBL-001 — the adapter produces canonical IR
- **scope:** the corpus under `docs/warrants/`.
- **evidence:** IR produced and validated.

### OBL-002 — parity is byte-identical over the WHOLE corpus
- **scope:** every Warrant and ADR, not a sample.
- **evidence:** byte comparison per document, with the count asserted.

### OBL-003 — cutover requires measured parity
- **scope:** §82.4.
- **evidence:** the criterion is a number met, not a judgement made.

## Gate Adequacy

Required at `controlled` when executed — cutover replaces the production
compiler.

**Could parity pass while the adapters differ?** Yes, if the corpus does not
exercise a construct. Byte parity over documents we happen to have is not parity
over the format. The mitigation is that the Markdown adapter stays as an oracle
(§97.5), so a divergence discovered later is detectable rather than silent.

**Executed attacks:** recorded here when run.

## Residual Risk

Unstartable until Liminal is checked out — the only Warrant in the roadmap that is blocked on something outside this repository entirely.
