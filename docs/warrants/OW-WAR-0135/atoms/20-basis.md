---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-602f-7321-ad70-864c543b2927
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- Product spec (`docs/design/openwarrant-product-spec.md`): Q43, Q44, Q45,
  Q46, Q47, Q49, C10, and "Engineering contracts still to specify". This is
  the only source for the mark; SAS 1.1.0 has none.
- §27.2: an agent SHALL NOT resolve a delivery or accept residual risk.
  Whatever the mark is, an agent cannot earn it on its own authority.
- §46.3 minimums: `controlled` needs "blind process or agent review";
  `high_assurance` needs an independent accountable person.
- §56.1 (the thirteen) and §56.2 (the record and `locator`).
- §53.3: a decision proposal becomes a proposed ADR before it is
  normative. M1 is that ADR.
- OW-ADR-0015: attestations for ssh-signed acts; `war attest --verify`.
- Code read for this draft: `resolution_cmd.rs` (what a resolution binds),
  `attest.rs` (what a resolve attestation's subjects are),
  `verify.rs` (`admissible_for`). No mark code exists
  (`rg -i 'assurance.?mark' crates` is empty).
- OW-ADR-0021: declaring an existing file (`lib.rs`) makes this Warrant its
  owner on authorization.

## Assumptions

- A-001: every requirement a baseline v1 would plausibly hold can be read
  from records that already exist, or from OW-WAR-0133/0134's findings:
  - resolution outcome `satisfied` and standing `valid`;
  - an attestation that verifies (an ssh-signed resolution, not a TTY
    one);
  - every obligation `established` by an admissible independent verdict;
  - locator present and `worktree_clean`;
  - no `acceptance.candidate-moved` for the candidate (OW-WAR-0134);
  - every relied-on receipt admissible (OW-WAR-0133).

  Confidence: medium. The ADR may add a requirement no record answers
  yet; M2 then reports it `UNKNOWN`, never met.
- A-002: a mark statement is a derived record and needs no new schema pack
  version beyond `oh.war/mark/v1`. Confidence: medium; if Q-001 selects a
  signed act, it is a new act kind and changes the authority surface.

## Unknowns

- **Blocking unknown — Q-001: is the mark issued, or derived?** Options:
  - (a) derived: `war mark` computes it from the signed resolution and
    other records. No new human act; the resolution *is* Q46's acceptance.
    Anyone can recompute it; nobody can grant it.
  - (b) issued: a separate human act, "mark", signed after resolution,
    with its own attestation. A fifth human act, and a SAS change.
  - (c) derived, but written to disk by `war mark --record` as a committed
    statement, so it can be cited by digest.

  Recommendation: (a), with (c)'s record as a cache that `--verify`
  recomputes. Q46's human act already exists; a second signature over the
  same decision adds friction and no information.
- **Blocking unknown — Q-002: baseline v1's contents.** A-001 is the
  proposal. The owner decides each line, in particular:
  - whether `controlled` independence (§46.3) is the floor, or `basic`
    counts;
  - whether Q43's "fixtures before implementation" is in v1. Recommend
    not in v1: no record today shows the order, so it would be `UNKNOWN`
    for every Warrant.
- U-003 (non-blocking): where a repository declares its strengthened
  baseline. Proposed: `openwarrant.toml [mark] baseline = "v1"` plus
  `extra = [...]`. The ADR settles it.

## Residual risks

- R-001: a derived mark is only as strong as the resolution under it. A
  resolution signed without reading (OW-WAR-0046's meaning shows the owner
  signing on an agent's wording) earns the same mark. Q46 accepts that;
  the mark says what the human act was, not that it was careful.
- R-002: independence here is an agent verifier (OW-WAR-0117). A mark at
  `controlled` never claims a human reviewed the code (Q46, §27.4).
