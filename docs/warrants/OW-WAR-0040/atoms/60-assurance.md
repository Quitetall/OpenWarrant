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
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the repository is present. This obligation exists because the
  Warrant is otherwise unstartable, and saying so is cheaper than discovering it.

### OBL-001 — the adapter produces canonical IR
- **scope:** the corpus under `docs/warrants/`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** IR produced and validated.

### OBL-002 — parity is byte-identical over the WHOLE corpus, and located when it is not
- **scope:** every Warrant and ADR, not a sample. Two declared observables: the
  bytes, and the semantic IR fields enumerated in advance.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** byte comparison per document, with the count asserted; and a
  semantic IR comparison over the fields declared before the run. **Zero
  differences on both.** The threshold does not move — one recorded difference
  refuses parity, as it always did. The semantic observable exists because byte
  parity says *whether* two adapters agree and cannot say *how* they disagree,
  so a difference is otherwise a refusal with no diagnosis attached. It is
  declared in advance exactly as the byte observables are, and it never
  substitutes for the byte comparison when the two disagree (AM-001).

### OBL-003 — cutover requires measured parity
- **scope:** §82.4.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the criterion is a number met, not a judgement made.

## Gate Adequacy

Required at `controlled`.

**Adversarial question: could parity be declared while the two adapters actually
disagree?** Yes, in three ways, and each is now closed by construction rather
than by care.

Parity measured on a SAMPLE reads exactly like parity measured on the corpus, so
`validate` requires `compiled_by_both == corpus_size` and names the shortfall
("12 of 40") rather than reporting a percentage. Parity with no declared
observables asserts nothing at all, so an empty `declared_observables` is
refused — §82.3's phrase is "declared observable parity", and declaring the
observables afterwards is choosing the ones that matched. And cutover is gated on
parity holding, because §82.4 permits it only once Liminal is qualified.

- **outcome:** gate_added, gate_strengthened, gap_accepted

`gap_accepted`: no Liminal compiler exists to run against. The parity harness is
implemented and tested; it has never compared two real adapters, and it cannot
until Liminal ships. That is recorded rather than hidden behind "resolved" — see
PRODUCTION_ROADMAP.md's note on what alpha resolution means.

**Executed attacks:**
- declared parity over 12 of 40 Warrants; refused, with the shortfall named
- declared parity with an empty observable set; refused, because it asserts nothing
- declared parity with one recorded difference; refused, and cutover refused with it
- attempted cutover against unmeasured parity; refused

## Residual Risk

Unstartable until Liminal is checked out — the only Warrant in the roadmap that is blocked on something outside this repository entirely.
