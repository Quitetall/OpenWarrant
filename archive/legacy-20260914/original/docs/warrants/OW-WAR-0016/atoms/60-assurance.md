---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd3-caf5-7161-9490-5d957bdf0d7d
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the schema fits the real corpus
- **scope:** all obligations currently written in this repository. Stated as 21 when this Warrant was authored; the corpus grew to 134 when the roadmap Warrants were written, and the count is measured at run time rather than hard-coded.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** each migrates without losing its bounded-scope statement.

### OBL-002 — an unbounded universal claim is REFUSED
- **scope:** an obligation asserting a universal with no declared scope.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant and its refusal.

### OBL-003 — dangling obligation_refs are REFUSED
- **scope:** milestone `obligation_refs`, which dangle unchecked today.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant and its refusal.

### OBL-004 — a verdict without dispositions is REFUSED
- **scope:** a resolution asserting satisfied with no per-obligation disposition.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant and its refusal. This is §38.1's whole point.

## Gate Adequacy

Required at `controlled` when this is executed — obligations are the mechanism by
which everything else claims completion.

**Could this pass while a Warrant still closes on a false claim?** Yes, and this
is the honest limit: an obligation can be well-formed, bounded, and satisfied by
evidence that does not actually support it. Structure makes the claim legible;
only verifier independence (OW-WAR-0021) and gate runs (OW-WAR-0020) make it
sound.

- **outcome:** no_counterexample_found, gap_accepted

**Executed attacks:**
- planted an obligation with no declared scope; refused by `obligations.invalid`
- planted `obligation_refs: ["OBL-999"]` on a milestone; refused by `obligations.dangling-ref`
- the second is the one that matters: a milestone citing proof nobody wrote passed unnoticed until this Warrant

## Residual Risk

The migration is the risk. Twenty-one obligations written as careful prose, converted by someone in a hurry, become twenty-one fields that parse.
