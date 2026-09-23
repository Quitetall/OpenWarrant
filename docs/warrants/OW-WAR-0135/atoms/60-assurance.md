---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-602f-7321-ad70-864c543b2927
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the mark is decided, and every requirement names its evidence
- **scope:** the ADR and `docs/assurance/baseline-v1.toml`. No claim that
  the baseline is the right one; that is the owner's acceptance.
- **gate:** `gate://document.review@1.0.0`
- **evidence:**
  - the ADR is accepted by the owner, not by its author;
  - each baseline requirement names the record or finding that evidences
    it, and what `UNKNOWN` means for it;
  - the ADR states Q-001's answer and the rejected options.

### OBL-002 — a Warrant that meets the baseline gets a mark that names what it binds
- **scope:** `war mark` on a scratch corpus with one resolved Warrant
  built to meet baseline v1.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** an `oh.war/mark/v1` statement naming the baseline version,
  the resolution digest, the attestation digest, the locator commit, the
  contract digest and the obligation ids; each equals the value
  recomputed from the corpus.

### OBL-003 — no mark without a human-signed resolution
- **scope:** the same scratch corpus, with the plants below.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** each of these gets no mark, and names the requirement it
  fails:
  - a Warrant with no resolution ("implementation finished, unreviewed",
    Q47);
  - a resolution with no attestation (a TTY signature);
  - a resolution whose attestation does not verify;
  - a resolution resting on a performer-only verdict (RQ-053).

### OBL-004 — UNKNOWN is not met, and a weaker baseline is not the same mark
- **scope:** `war mark` against baseline v1 and a planted repository
  baseline.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - a requirement no record can answer (a resolution with no locator) is
    reported `UNKNOWN` and the mark is refused;
  - a repository baseline that adds a requirement produces a mark naming
    both;
  - a repository baseline that removes a v1 requirement produces no v1
    mark; it is refused by name.

### OBL-005 — a mark goes stale when what it binds moves
- **scope:** `war mark --verify` on the mark from OBL-002.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - after the resolution file is edited, `--verify` names the resolution
    binding;
  - after an in-scope source change (OW-WAR-0134's finding), it names the
    candidate;
  - unchanged, it verifies. This is the control against a `--verify`
    that always fails.

## Gate Adequacy

**Adversarial question:** could an artifact pass every declared gate while
a mark exists for a result no human accepted, or for a baseline weaker than
the one it names?

Counterexamples the author considered while drafting (not executed):

1. A `war mark` that emits for any resolved Warrant passes OBL-002. Closed
   by OBL-003's four refusals.
2. A mark statement hand-written to disk. Under Q-001 (a) no stored mark
   is trusted; `--verify` recomputes. Under (c), the same holds for the
   cache.
3. A baseline that is right in form and wrong in substance. Not closable
   by a gate; it is the owner's acceptance in OBL-001.

The §39.2 outcome and the executed attacks are left to the blind review
this level requires. The author does not record them.
