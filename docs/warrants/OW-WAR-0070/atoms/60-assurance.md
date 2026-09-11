---
schema: oh.war/atom/v1
warrant_uuid: 01a09274-ad79-7414-a583-cdecca67ed90
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

All obligations are unverified. No disposition, gate receipt, verdict,
authorization, or resolution is asserted by this draft.

### OBL-001 — the fields are additive and existing records still parse

- **scope:** every fixture in `conformance/` and every Warrant in
  `docs/warrants/` on this branch. No claim about records outside the repository.
- **evidence:** the full battery passing unchanged with the fields added and no
  record edited to accommodate them.

### OBL-002 — decision debt is distinguishable by field, not prose

- **scope:** assumptions carrying the typed decision resolution requirement.
- **evidence:** a fixture with two blocking unknowns, one a decision debt and one
  not, separated by a query over fields alone with no text matching.

### OBL-003 — an unreserved debt renders as a named gap

- **scope:** the Gaps view's input, not its rendering. No claim about the app.
- **evidence:** a fixture assumption requiring a decision with no `future_ref`
  appearing in the projection as a gap, and the contradiction refusal firing when
  `future_ref` names an already-accepted record.

### OBL-004 — independent verification

- **scope:** this Warrant's obligations.
- **evidence:** a `war verify --response` verdict from a verifier that is not
  the performer and did not draft these atoms.

## Gate Adequacy

**Adversarial question: could every obligation pass while the system is wrong?**

Yes, and the most important way is structural.

Every obligation here is satisfied by a debt that someone *declared*. Nothing
detects a decision that is required and was never declared at all. That is the
failure this Warrant is motivated by — a hand-written table omitting a row — and
it does not close it. It moves the debt from prose into a checkable field, so an
omission is at least omission from a structured record rather than from a
paragraph. A program can still be wrong by staying silent, and no plant here
catches that.

Second: the contradiction refusal proves a paid debt cannot be recorded as
unpaid. It does not prove the converse. A debt whose decision was written and
never linked stays open in the corpus forever with nothing objecting.

Third: OBL-002's separation is over fixtures this Warrant authors. Fixtures the
author designs to be separable are weak evidence that real programs' debts are.

**Executed attacks:** none. Execution is pending.
