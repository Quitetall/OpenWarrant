# Packet 2 — Liminal

**To:** the Liminal team
**From:** OpenWarrant (`Quitetall/OpenWarrant`), 2026-09-03
**Blocks:** OW-WAR-0040 (the adapter and parity harness) and OW-WAR-0048 (the
Phase 8 exit)
**Governing text:** SAS §82 (source adapters), §97.5 (cutover)

> **Answered 2026-09-03.** Liminal replied. Brian has decided Liminal will build
> the compiler and the pieces this packet needs — that decision is his, and it is
> recorded here as his, not as anything either agent committed. No version pin and
> no observable set were sent, deliberately and correctly: the IR they would
> describe does not exist yet, so a set enumerated today would be fabricated.
> Four things below misdescribed Liminal and are corrected. One of their findings
> is a risk to OUR gate, not to their schedule, and it is §9.

## 1. What we are asking for

We need a **pinned Liminal compiler profile that emits canonical IR for our
document corpus**, and we need **your declared observable set in advance of the
comparison**. Phase 8's exit is "Liminal is the single production document
semantic compiler." We built the harness that would qualify you for that. It has
never compared two adapters, because only one exists.

The order is deliberate and it is the whole design: **declare the observables,
then run the comparison.** A parity claim whose observables were chosen after
seeing the diff is not a measurement, and our harness refuses it.

## 2. The governing text

**§82.2 Liminal adapter** — "The final adapter invokes a pinned Liminal compiler
profile through a versioned process protocol." The SAS's illustrative command is
`liminal-compiler --protocol oh.war/liminal-v1`.

**That command names nothing that exists**, and the error is ours, not theirs.
Liminal ships `liminald` and `lim`; there is no `liminal-compiler`. The
`liminal-protocol` crate is 26 lines and says of itself "RESERVED —
implementation begins Phase 2". A future SAS revision should correct §82.2's
illustration, which is a §101 governance change and not something to patch in
prose here.

**§82.3 Adapter parity** — "Before cutover, the Markdown compatibility corpus
SHALL be compiled by both adapters and compared for declared observable parity."

**§82.4 Cutover** — once qualified: Liminal becomes production semantic
compiler; the Markdown adapter becomes importer/test adapter; one production
definition remains.

**§97.5** — "The old compiler remains a compatibility oracle during measured
parity. After acceptance, one production compiler remains."

We are not retiring the Markdown adapter on a promise. It stays as the oracle
until the numbers hold, and then it becomes a test adapter rather than being
deleted.

## 3. What we have already built

- The Markdown v1 adapter (§82.1), in production, compiling the whole corpus.
- The **parity harness** (§82.3) and the **cutover gate** (§82.4). The gate is
  not advisory: cutover is a number met, not a judgement made.
- Four adversarial attacks against our own gate, all currently refused by
  `AdapterParity::validate` / `permit_cutover` and covered by unit tests (these
  are type-level refusals, not corpus plants — the distinction matters to us and
  we would rather state it than let you assume the stronger form):
  - parity declared over 12 of 40 Warrants — refused, with the shortfall named;
  - parity declared with an **empty observable set** — refused, because it
    asserts nothing;
  - parity declared with one recorded difference — refused, and cutover refused
    along with it;
  - cutover attempted against unmeasured parity — refused.

## 4. The corpus

The compatibility corpus is everything under `docs/warrants/` and `docs/adr/` in
this repository, at whatever commit we pin for the run. As of 2026-09-03 that is
**59 Warrant directories, 296 atom files, and 11 ADR atoms**. Parity is required
over the **whole** corpus, not a sample — OW-WAR-0040 OBL-002 says "every
Warrant and ADR, not a sample", and the count is asserted by the harness rather
than trusted.

The atoms are constrained Markdown with YAML frontmatter. `docs/adr/atoms/` and
any `docs/warrants/OW-WAR-*/atoms/*.md` are representative; two ADRs describe the
exact subset we accept:

- `OW-ADR-0002-frontmatter-subset.md`
- `OW-ADR-0003-structured-atom-subset.md`

Read those two before profiling. They are short, and they are the difference
between "compiles our documents" and "compiles Markdown".

## 4b. What Liminal actually is today

Checked against Liminal's source by their team, 2026-09-03, and recorded because
three of these four were wrong in the first draft of this packet.

- **No versioned process protocol yet.** See §2 above. The process boundary this
  packet would invoke is two phases out, not one.
- **No YAML reader anywhere in the tree.** Zero occurrences across every crate.
  Our two ADRs exist precisely to define a restricted YAML frontmatter grammar,
  so Liminal cannot currently read the header of a single OpenWarrant atom. That
  is a missing capability, not a profile setting — which means "point Liminal at
  the corpus" is not a small configuration step.
- **`liminal-cst` is not a Markdown parser.** 403 lines, self-described as
  "intentionally not a full grammar": a coarse, error-tolerant block scanner over
  Liminal's own v4 format. Its `emit()` returns `&str` — it hands the source
  back, so its losslessness is byte-preservation, not round-trip through a parse.
- **`liminal-cir`, the interchange layer that would produce our canonical IR, is
  tagged M19.3–M19.4.** Phase 1 (M18–M24) is authored but not authorized; every
  Phase 1 exit gate is `#[ignore]`d under their AM-17.2, including
  `canonical_round_trip_law_holds`. Authorization comes from an M17.6 GO/NO-GO
  gated on M17.5, a qualification campaign still running.

This supersedes OW-WAR-0040's OBL-000, which asked only that "the repository is
present". Presence was the wrong question.

## 5. What we need returned — artifacts, not assertions

1. **A pinned profile**, invocable as a versioned process, with an exact version
   string we can record. "Latest" is not a pin.
2. **The declared observable set, sent before the comparison run.** Which
   properties of the IR are claimed equal — field by field. This is the item
   most likely to be skipped and the one the gate most reliably catches.
3. **Canonical IR for the corpus**, produced by that profile at the pinned
   commit, in a form we can byte-compare per document.
4. **A statement of what the profile does *not* cover**, if anything. A named
   gap is workable. A gap we discover during comparison costs a round trip.

## 6. What we will refuse

- **Parity over a subset.** The corpus count is asserted, and a shortfall is
  named rather than rounded away.
- **Parity with an empty or post-hoc observable set.** An empty observable set
  "asserts nothing", in the code's own words, and is refused.
- **Parity with one recorded difference.** One is enough to refuse; there is no
  tolerance band, because nobody has been able to tell us what an acceptable
  semantic difference would be.
- **Cutover before measurement.** `permit_cutover` fails unless parity is
  established first.

## 9. A risk to OUR gate, raised by Liminal

Worth more than the four corrections, because it threatens the design of
OW-WAR-0040's OBL-002 rather than its schedule.

Our harness refuses parity with **one** recorded difference, over byte-compared
IR. Liminal has no chosen production Markdown frontend — no crate manifest
depends on one. The surveyed candidate, tree-sitter-md 0.5.3, was rejected for
production in their DG17.1 because upstream "warns that its Markdown output
retains known inaccuracies, and it has no lossless native serializer."

A zero-difference byte-parity claim over a frontend whose parser is **still
unchosen** is a design exposure on our side. Either the eventual parser is
byte-faithful over our corpus, or our zero-tolerance rule is unmeetable in
principle and we have written a gate nobody can pass — which is a worse failure
than a gate that is merely hard. This needs deciding before a comparison run is
scheduled, not during one.

The corresponding opening: our OW-ADR-0002/0003 strategy — a restricted reader
that refuses everything outside a documented subset — is a live candidate for
Liminal's own unchosen frontend, and our corpus being a bounded subset makes it a
more tractable first target than general Markdown. Their words, and explicitly
not a commitment to a design.

## 7. Something we are fixing on our side, so you do not have to

OW-WAR-0048 bundles a second, older claim: §91.1 test 1 requires **two hosts** to
produce a byte-identical canonical IR. Two of our earliest Warrants recorded
satisfying it with two runs on *one* host — and both honestly say so in their own
text.

That is ours, not yours. Going public made CI minutes free, and our release
workflow already carries an `x86_64-unknown-linux-gnu` / `aarch64-apple-darwin`
matrix — different OS, different architecture. We are discharging that half
independently. It is mentioned here only so that, when you read OW-WAR-0048, you
are not looking for a Liminal dependency that is not there.

## 8. How to send it back

A PR to this repository, a tarball, or a reachable build. Whatever the transport,
we need the profile version string and the digest of the IR bytes, because the
harness compares bytes and records what it compared.
