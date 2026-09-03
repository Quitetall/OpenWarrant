---
schema: oh.war/atom/v1
warrant_uuid: 01a018db-19fc-72ba-87b3-c1bd1aec86a8
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

Assurance level `controlled`, so an adequacy review is required (§39.4,
RQ-055) and is recorded below rather than waived.

## Acceptance Obligations

### OBL-000 — the canonicalization library is selected by ADR and license-cleared
- **gate:** `gate://software.repo.war-check@1.0.0`

- **scope:** the single dependency implementing RFC 8785.
- **evidence:** a merged implementation ADR naming the crate and its version,
  plus `cargo deny check licenses` passing with it in the graph.

### OBL-001 — a valid Basis lowers to the IR
- **gate:** `gate://software.repo.war-check@1.0.0`

- **scope:** the five Warrants in `docs/warrants/`, under the `delivery` profile.
- **evidence:** lowering succeeds and the IR's `api_version` and `kind` are
  exactly the pinned constants.

### OBL-002 — canonical output conforms to RFC 8785
- **gate:** `gate://software.repo.war-check@1.0.0`

- **scope:** RFC 8785's published test vectors, plus the five Warrants.
- **evidence:** byte comparison against the RFC's vectors — **external
  expectations, never values captured from this implementation.**
- **why the distinction is load-bearing:** a snapshot test seeded from our own
  output asserts only that the code is deterministic. It would pass unchanged if
  the serialiser sorted keys by the wrong collation, and every digest built on it
  would be wrong in a way no test could see.

### OBL-003 — digests are domain-separated
- **gate:** `gate://software.repo.war-check@1.0.0`

- **scope:** all fifteen domains.
- **evidence:** one payload hashed under two different domains yields two
  different digests, for every pair. Not a sample — the full pairwise set; there
  are only 105 pairs and a sample would leave the untested pair as the one that
  collides.

### OBL-004 — canonical output is host-independent
- **gate:** `gate://software.repo.war-check@1.0.0`

- **scope:** two runs, differing in process, working directory, environment
  ordering, and locale.
- **evidence:** byte-identical canonical IR (§91.1 test 1).
- **known limitation, stated rather than implied:** "two hosts" is satisfied
  here by two runs on one machine. That does not exercise a different
  architecture, endianness, or libc. The claim is therefore bounded to
  same-host determinism until a second host is available, and OBL-004 must not
  be read as proving cross-platform canonicalization.

## Gate Adequacy

**Adversarial question:** could every obligation pass while a digest is
nevertheless wrong in a way that matters?

Two ways survive, both accepted with mitigations rather than dismissed:

1. **An IR section that is empty today and populated later.** If absent and
   empty serialise identically, the first Warrant to populate `execution` changes
   the digest of every Warrant compiled before it. Mitigated by an explicit test
   pinning the distinction, listed in the Work Order. This is the highest-value
   check in the Warrant and the easiest to forget.

2. **Correct canonicalization of an incorrect IR.** RFC 8785 conformance says
   nothing about whether the IR carries the right facts. Nothing in this Warrant
   detects a lowering that drops a field. Mitigated only partially, by
   round-trip tests over the five Warrants — which cannot catch a field that
   none of the five exercises.

**Executed attacks:** to be recorded here when run, per §39.3. An adequacy
review that lists questions without answers is a plan, not a review.

- **outcome:** no_counterexample_found, gap_accepted

**Executed attacks:**
- hand-edited a committed generated view; refused by `generated.drift`
- deleted a committed generated view; refused by `generated.missing`
- ran the six vendored RFC 8785 official vector pairs in `conformance/rfc8785/`; all serialise to the published canonical form
- pinned the absent-versus-empty distinction with a test, which is the failure named above and the easiest to forget

## Residual Risk

- Same-host determinism only, per OBL-004's limitation.
- The `decision` profile is unexercised: all five Warrants are `delivery`.
