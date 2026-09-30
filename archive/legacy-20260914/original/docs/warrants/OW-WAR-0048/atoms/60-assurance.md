---
schema: oh.war/atom/v1
warrant_uuid: 01a021a7-8436-7aff-a005-a43eeea25886
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the canonical IR is byte-identical across two real hosts
- **scope:** §91.1 test 1, over this repository's 49 Warrants, on
  `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **external system:** GitHub Actions runners, both images recorded by
  label and OS version.
- **evidence:** a CI run on both triples producing byte-identical canonical JSON,
  compared by digest across hosts. Two runs on one host do not satisfy this, and
  the two Warrants that previously recorded that caveat are updated to cite this
  run instead.

### OBL-002 — the inverted runner-tier decision is recorded, not left stale
- **scope:** `.github/workflows/ci.yml`'s runner comment.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the comment states that the self-hosted plan is
  abandoned because the repository is public, citing the security inversion the
  comment itself predicted. A stale instruction that has already fired is worse
  than no instruction.

### OBL-003 — a real Liminal compiler answered over the versioned protocol
- **scope:** one Liminal profile, pinned.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **external system:** Liminal, by exact commit, invoked as
  `--protocol oh.war/liminal-v1`.
- **evidence:** a recorded invocation and its output for the compatibility
  corpus. An in-process stand-in is not an adapter.

### OBL-004 — parity is measured across the whole corpus, observables declared first
- **scope:** §82.3, over the full compatibility corpus.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** `compiled_by_both == corpus_size`, an observable set
  recorded BEFORE the run, and zero differences. A shortfall is reported as
  "N of M", never as a percentage.

## Gate Adequacy

Required at `controlled`.

**Adversarial question: could parity be declared while the adapters actually
disagree?** OW-WAR-0040 closed three versions of this by construction — sampling
reads as full coverage, an empty observable set asserts nothing, and cutover is
gated on parity holding.

The version left open is the one this Warrant adds: two hosts is two. Byte
identity on Linux x86-64 and Darwin arm64 is real evidence and is not a proof of
determinism. A third platform could diverge, and §38.4 would refuse a universal
claim from two data points — which is why OBL-001's scope names the two triples
rather than claiming portability.

**Executed attacks:** none yet — this Warrant has not been executed.

## Residual Risk

Discharging §91.1 test 1 will make two older Warrants' recorded caveats
obsolete. Those caveats are currently honest; leaving them in place after the
real run would make them false in the opposite direction. OBL-001 requires the
update, and it is the kind of follow-through that gets skipped once the
interesting part is done.
