---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-604b-7d63-8453-39847a16c82c
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a replaced receipt is caught after signing
- **scope:** resolutions signed with `war sign --ssh-sign` after this
  change, on a scratch corpus with a test key. Not the 29 existing
  resolutions (Basis A-004).
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - the resolve attestation's subjects include every `gate_run_refs`
    receipt, its run, stdout and stderr, each with the digest on disk;
  - after one receipt is replaced by a freshly minted one that reseals,
    `war attest --verify` and `war attest --custody` both fail and name
    that file;
  - control: untouched, both pass.

### OBL-002 — the custody audit reports each §41.5 field, and UNKNOWN is not present
- **scope:** `war attest --custody` on the scratch resolution from
  OBL-001, and on one of this repository's existing resolutions.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - all nine §41.5 fields are listed for each receipt, each present with
    its value or `UNKNOWN` with a reason (access history is `UNKNOWN`:
    git records no reads);
  - on the existing resolution, original digest is
    `UNKNOWN (not attested)`, not present;
  - `--record` by the performer is refused with `SelfAct`, and nothing is
    written.

### OBL-003 — one invalidation disputes every dependent resolution and nothing else
- **scope:** a scratch corpus of four resolved Warrants: A and B rest on
  gate G's runs; C is A's child (§20.2); D rests only on gate H.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - invalidating G writes exactly three disputes, for A, B and C (C
    transitively), each with §56.4's six fields;
  - D gets none. This is the control against a sweep that disputes
    everything;
  - `war check` reports `resolution.disputed` for A, B and C, and not D.

### OBL-004 — nothing historical is rewritten, and an invalidated gate stops counting
- **scope:** the same scratch corpus, before and after the invalidation.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - every `resolution.toml`, receipt, run, attestation and G's definition
    file is byte-identical before and after;
  - on an unresolved Warrant E citing G, G's receipt is no longer
    admissible for requirement 5, and the reason names the invalidation.

### OBL-005 — no step is performed by the actor who produced the work
- **scope:** every act this Warrant adds, on the scratch corpus.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** each is refused, and writes nothing:
  - an invalidation whose actor is the performer of A;
  - under Q-001 (a) or (b): an invalidation signed by an agent-kind actor;
  - a custody audit recorded by the performer;
  - and, unchanged from today, a resolution by the performer.

### OBL-006 — the exit, in this repository (Q-002 b only)
- **scope:** this repository, `ops.exit-demo@1.0.0`, and the one
  Warrant resolved against it. Withdrawn by amendment if Q-002 selects (a).
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:**
  - the demo resolution's attestation verifies, signed by the owner;
  - its custody audit is recorded by an actor other than its performer;
  - the owner invalidates `ops.exit-demo@1.0.0`; the demo resolution, and
    only it, is disputed;
  - `war attest --verify --all` still passes for the 29 existing
    resolutions, and `war check` reports none of them disputed.

## Gate Adequacy

**Adversarial question:** could an artifact pass every declared gate while
an invalidated gate leaves a dependent resolution looking valid, or while
the performer performs one of the exit's steps?

Counterexamples the author considered while drafting (not executed):

1. A sweep that disputes every resolution passes OBL-003's positive half.
   Closed by D, the control.
2. A sweep that stops one hop out. Closed by C, the transitive case.
3. Disputes written by rewriting `standing` in `resolution.toml`: the
   attestation would then drift. Closed by OBL-004's byte comparison.
4. A receipt swapped for another that reseals over its own fields. The
   seal alone cannot catch it; OBL-001 plants exactly this.
5. The 29 existing resolutions: their receipts are not attested and the
   audit cannot make them so. Accepted and stated (A-004, R-003).

The §39.2 outcome and the executed attacks are left to the blind review
this level requires. The author does not record them.
