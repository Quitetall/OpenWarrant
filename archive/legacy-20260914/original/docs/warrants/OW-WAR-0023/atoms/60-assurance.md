---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd6-ebf5-77d8-b498-a397823907f3
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a Dispatch is self-contained
- **scope:** §47.1.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a compiled Dispatch is readable and complete with the repository
  absent — tested by moving it, not by inspection.

### OBL-002 — projection is actor-specific
- **scope:** §47.3.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** two actor roles receive different projections of one stage.

### OBL-003 — capabilities default to DENIED
- **scope:** §55.2.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant requesting an unlisted capability, refused.

## Gate Adequacy

Required at `controlled` — a Dispatch grants authority.

**Adversarial question: could a Dispatch pass every declared gate while
authorizing more than was intended?** Yes. The capability list is authored, and
nothing derives least privilege automatically, so a Dispatch that is structurally
complete can still over-grant. Completeness is checkable; minimality is not.

- **outcome:** gap_accepted

Accepted with the gap named rather than closed: least-privilege derivation is
beta work against a live runtime, and claiming it here would be the kind of
unearned assurance this Warrant exists to prevent. What IS enforced is that a
projection cannot quietly change the contract it projects — which bounds
over-granting to what the contract already authorized, rather than eliminating
it.

**Executed attacks:**
- projected one dispatch for a human and one for Katana with different objectives, instructions and non-goals; accepted, because §47.3 permits representation to differ
- altered the contract digest, the contract revision, and the obligation refs in turn between two projections; each refused by `same_normative_contract_as`
- listed a required normative source in `omitted_subgraphs`; refused (§47.2), while omitting a non-required source was accepted, which is what projection is for
- compiled a repair dispatch carrying no prior failure evidence; refused — a repair that cannot see what failed is a retry

## Residual Risk

Under-projection surfaces only at execution. Until a live runtime exists (beta), Dispatch completeness is argued rather than demonstrated.
