---
schema: oh.war/atom/v1
warrant_uuid: 01a01bd5-a42a-7dc9-819f-8ca614dc87eb
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — definitions are versioned and governed
- **scope:** §43.2's schema.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a definition without a version is refused.

### OBL-002 — an unqualified gate cannot be BOUND
- **scope:** §43.4.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant and its refusal.

### OBL-003 — an obligation citing a nonexistent gate is REFUSED
- **scope:** the failure the parent project shipped 23 times.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant citing a `- **gate:**` bullet that names an unregistered
  gate, refused by name. The citation channel is a declared bullet, not prose:
  scanning prose flagged this very sentence, and a gate found by pattern-matching
  text is the failure §43 exists to end (OW-ADR-0005).

### OBL-004 — bindings are digested
- **scope:** `oh.war/gate-binding/v1`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a digest distinct from every other domain.

## Gate Adequacy

Required at `controlled`.

**Adversarial question: could this pass while a gate is still a lie?** Yes, and
the limit is worth stating precisely. A Definition can be versioned, qualified,
and bound, and still describe a check that does not measure what it claims.
Qualification proves a gate CAN detect the fault classes it declares; it says
nothing about the fault classes it failed to declare. That is why §43.2's
`known_blind_spots` is populated here rather than left empty.

- **outcome:** counterexample_found, gate_strengthened, gap_accepted

The counterexample was found during implementation, on real data: the first
version of the citation check scanned assurance prose for `gate://`, and
immediately flagged OBL-003's own sentence describing its plant. A gate located
by pattern-matching text is precisely the "string, not a gate" failure this
Warrant exists to end, so the citation channel became a declared `- **gate:**`
bullet and the prose scan was deleted.

`gap_accepted` covers the second half: the registry here is local. §43.1 gives
Knowledge Fabric the authoritative one, and OW-ADR-0005 records why a candidate
stays labelled a candidate rather than being quietly promoted.

**Executed attacks:**
- planted an obligation citing an unregistered gate; refused by `gate.unresolved`
- planted a `draft` gate and bound it; refused by `gate.not-bindable` (§43.4)
- planted a definition with no version; refused by `gate.invalid` (§43.2)
- removed the negative control from the qualification record; refused, because a gate qualified only against bad input cannot be told apart from one that rejects everything
- flipped a `detected` value to `false`; refused as an undetected declared fault class
- ran the whole check against all 40 Warrants and 1 gate unmodified: 0 errors

## Residual Risk

A local registry standing in for the KF-owned one is provisional authority. If that provisionality is not visible in the record, a locally-qualified gate will be mistaken for an institutionally-qualified one.
