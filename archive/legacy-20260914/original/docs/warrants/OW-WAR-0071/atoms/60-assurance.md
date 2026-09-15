---
schema: oh.war/atom/v1
warrant_uuid: 01a0983e-32d7-7473-b6e1-fbef061a1e5f
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the revision is a recorded, accepted revision of this SAS
- **scope:** `docs/sas/WAR_Software_Architecture_Specification.md`, `docs/sas/revisions/1.1.0.toml`, `docs/sas/generated/NORMATIVE.{md,json}`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `war sas propose 1.1.0` records the digest and its §106 diff against 1.0.0; `war check` reports no `sas.stale-digest` and no `sas.repin-unknown-requirement`; the normative projection recompiles and carries the new revision's digest; the acceptance is the owner's ssh signature, attested.

### OBL-002 — the batch rule is written so a tool can refuse by it
- **scope:** the new §27.6 and its §106 rows.
- **gate:** `gate://document.review@1.0.0`
- **evidence:** every clause of §27.6 names a refusal a reader can test — subject is a canonical list, acts individually permitted, a moved digest invalidates the whole batch, an agent signing is refused, each record cites the batch digest. An independent reviewer confirms no clause is unfalsifiable, and that no clause weakens §27.2's human-only rule.

### OBL-003 — a rendering is defined as authority-free
- **scope:** the new §76.6, `docs/PROJECTION_CONTRACT.md`.
- **gate:** `gate://document.review@1.0.0`
- **evidence:** the text says a rendering issues commands and performs no act; the projection contract cites it; the existing plant that refuses a projection reaching for the network is named as the precedent it extends.

### OBL-004 — the ADR §101.3 requires exists and governs this revision
- **scope:** `docs/adr/atoms/OW-ADR-0019-sas-1-1-0-batch-act-and-renderings.md`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the ADR is `status: accepted`, `governs` this Warrant's uuid, states the rejected alternatives (drop `-c`; per-act signing; a time-boxed session token) and why; `war check` finds no `sas.adr-missing` when the revision is proposed with `--adr OW-ADR-0019`.

## Gate Adequacy

Required at `basic`. The document gate is the load-bearing one: this Warrant
delivers text whose only defect mode is a rule nobody can test.

**Adversarial question:** does a batch act let a signer authorize something
they did not read? The batch document is the subject of the signature, so
what they signed is exactly the list; the refusal on a moved digest closes
the gap between drafting and signing. The residual risk is a signer who does
not read a list they signed — the same risk as a signer who does not read one
record, and no rule in any specification closes that one.

**Second adversarial question:** could a rendering become an authority by
accident? Only by holding a key. §76.6 forbids it and the implementation
Warrants carry the plants; a terminal that holds a key is a defect with a
name, which is what the rule buys.
