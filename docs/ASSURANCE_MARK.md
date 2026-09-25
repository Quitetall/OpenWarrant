# The assurance mark

What `war mark` says about a resolved Warrant, what it does not, and how a mark
is checked again later (OW-WAR-0135; OW-ADR-0025; product spec Q43–Q47, Q49).

Status on this branch: **contingent, and not in force.** OW-ADR-0025 is
`proposed`: the owner has not answered OW-WAR-0135 Q-001 (is a mark issued or
derived?) or Q-002 (what baseline v1 holds). The command is built against the
ADR's recommendation — a mark *derived* from records already signed, with a
written copy that is only a cache (Q-001 (a) with (c)), and baseline v1 as
`docs/assurance/baseline-v1.toml` proposes, with the `controlled` independence
floor. That file says `status = "proposed"`, and a baseline that is not
`accepted` earns no mark. So in this repository today, `war mark` evaluates
every requirement, names what is met, and then refuses with
`mark.baseline-not-accepted`. It never reports a mark.

## What a mark says

One sentence: **this accepted result, at this commit, in this scope, met this
baseline.**

- *One accepted result.* One resolution of one Warrant, signed by a human and
  attested (Q44, Q46). Not the Warrant's other revisions, not other Warrants.
- *At this commit.* The resolution's `[locator]` commit, with the delivered
  bytes committed there, and the candidate still the tree the human accepted.
- *In this scope.* The obligations the Warrant declares, each established by
  an independent verdict; the statement lists them, each with its verdict's
  digest.
- *This baseline.* A named version (`v1`), by the digest of the file that
  defines it, plus any extension the repository adds.

## What a mark does not say

- **Not the codebase, and not a release.** A mark is about one resolution. A
  release that contains a marked Warrant is not marked (Q44).
- **Not that every line was read.** The human act is the resolution: a person
  reviewed the outcome, the verification findings and the remaining risks,
  and accepted the result (Q46). Independent verification supplied the
  technical review. At the `controlled` floor that verifier is a blind process
  or agent review (§46.3), never a claim that a human reviewed the code.
- **Not that the resolution was careful.** A derived mark is only as strong as
  the resolution under it (OW-WAR-0135 R-001). A resolution signed on an
  agent's wording earns the same mark; the mark says what the human act was.
- **Not authority.** Nobody grants a mark and it grants nothing. It is a
  reading of acts already signed; anyone can recompute it, and a recomputation
  over unchanged records gives the same bytes.
- **Not history the records do not show.** A resolution made before
  attestations (OW-ADR-0015) or before the locator (OW-ADR-0021) earns no
  mark: BL-002 is unmet, BL-004 UNKNOWN. Qualifying later is a new resolution
  with the records v1 reads, never a mark over the old one (Q43).
- **Not "mark before merge".** The mark is computable. Requiring it before a
  merge is repository policy (Q49), and nothing here enforces it.

## The command

```bash
war mark <alias>                     # evaluate; print the statement only when earned
war mark <alias> --json              # oh.war/mark-evaluation/v1, with `mark` when earned
war mark <alias> --baseline <id>     # another baseline: docs/assurance/baseline-<id>.toml
war mark <alias> --record            # write <warrant>/mark-<baseline>.json, only when earned
war mark <alias> --verify            # recompute the recorded mark; name every binding that moved
war mark <alias> --verify --file F   # the same, for a mark file somewhere else
```

It exits 0 only when a mark is earned (or, with `--verify`, when the recorded
mark still recomputes).

## Evaluation

Each requirement names a `check`. Each check answers exactly one of:

- **met** — the record exists and says what the requirement asks;
- **unmet** (`mark.requirement-unmet`) — the record says otherwise, or its
  absence is the answer (no resolution is Q47's "unreviewed", not a gap);
- **UNKNOWN** (`mark.requirement-unknown`) — no record can answer. Law 15:
  UNKNOWN is never met. It refuses the mark and is reported apart from unmet.

A mark exists only when every requirement is met, under a baseline that is
`accepted` and not weakened. Otherwise `mark.refused` names each unmet and each
UNKNOWN requirement.

| id | check | met when | unmet when | UNKNOWN when |
|---|---|---|---|---|
| BL-001 | `resolution.satisfied` | resolved `satisfied`, standing `valid` | no resolution, another outcome or standing | never |
| BL-002 | `resolution.attested` | a resolve attestation naming this `resolution.toml` verifies (as `war attest --verify` does), is signed by the resolver, and the resolver is a `human` in `roles.toml` | no attestation, a signature or subject digest that does not verify, a signer other than the resolver, a resolver that is not human | `ssh-keygen` is not available |
| BL-003 | `obligations.independently_established` | every declared obligation has a verdict `established` and admissible at the floor | a missing verdict, another disposition, a self-verification (RQ-053), independence below the floor, or no declared obligation at all | never |
| BL-004 | `resolution.located` | `[locator]` with a forty-hex commit and `worktree_clean = true` | `worktree_clean = false` | no `[locator]`, or a commit that is not forty lowercase hex |
| BL-005 | `acceptance.unchanged` | `acceptance::assess` at `HEAD` says unchanged (OW-WAR-0134) | `acceptance.candidate-moved` | `acceptance.unknown` |
| BL-006 | `evidence.admissible` | every `gate_run_refs` receipt is admissible against the resolution's contract digest (OW-WAR-0133); a resolution that relied on none meets it vacuously | a receipt missing, invalid, not a pass, or stale | a receipt whose reuse cannot be judged |

The floor BL-003 judges every verdict at is the baseline's
`independence_floor`, whatever the Warrant's own assurance level.

BL-005 compares the locator commit with the commit at `HEAD`. Uncommitted
changes are not the candidate; commit them, or they are not judged.

A `check` this build does not implement is UNKNOWN, never dropped. That is
also what a requirement no record can answer yet looks like: a repository that
adds "fixtures before implementation" (Q43) under a check nothing implements
gets UNKNOWN on it, and no mark, until a record exists.

## The statement

`oh.war/mark/v1`, every field a value a record holds or the digest of a file in
the tree:

| field | value |
|---|---|
| `warrant` | alias and uuid |
| `baseline` | `id`, the path and sha256 of the baseline file, its `independence_floor`, and each extension's id, path and sha256 |
| `resolution` | the path and sha256 of `resolution.toml` |
| `attestation` | the path and sha256 of the resolve attestation that verified |
| `commit` | `locator.commit_sha` |
| `contract_digest` | `resolution.contract_digest` |
| `obligations` | each declared obligation, with its verdict file and that file's sha256 |
| `requirements` | each requirement id, `met` |

No timestamp and no tool build: the same records always give the same bytes.

## Recording and verifying

`--record` writes the statement to `docs/warrants/<alias>/mark-<baseline>.json`,
and only when a mark is earned (`mark.not-recorded` otherwise; nothing is
written). The file is inside the Warrant's own records, so committing it does
not move the candidate.

A written mark is **never trusted.** `--verify` evaluates the Warrant again and
compares every field with the file. Each difference is `mark.stale` naming the
binding that moved:

- the resolution edited → `resolution` (and BL-002, since its attestation no
  longer matches);
- an in-scope source change committed after the resolution → `candidate`
  (BL-005);
- a hand-edited mark → the field that was edited;
- a baseline that is no longer accepted, or has been weakened → `baseline`.

Nothing moved → `mark.verified`. No file → `mark.no-record` (UNKNOWN: there is
nothing to compare).

## Strengthening, and not weakening

A repository names its baseline, and optionally an extension, in
`openwarrant.toml`:

```toml
[mark]
baseline = "v1"
extra = "docs/assurance/repository.toml"   # optional
```

The extension has the baseline's form, `extends = "v1"`, and adds requirements
with ids of its own. A mark under it names **both** (`v1` + the extension, each
by digest), and lists every requirement.

Weakening is refused by name (`mark.baseline-weakened`), and the result is not
a v1 mark:

- a repository copy of `baseline-v1.toml` with a requirement removed, its
  `check` changed, or marked `optional`, or with a lower `independence_floor`,
  compared with the v1 this build ships;
- an extension that redefines a v1 requirement's id, or lowers the floor.

A repository may run a weaker baseline under its own name
(`--baseline <id>`, `docs/assurance/baseline-<id>.toml`). It may not call it
`v1`.

## What becomes true when the owner decides

If the owner accepts OW-ADR-0025 as recommended, the ADR's Decision section and
`docs/assurance/baseline-v1.toml`'s `status` change together, in the owner's
change. `war mark` then earns marks in this repository with no change to its
code. If the owner answers otherwise — an issued mark (Q-001 (b)), a
`basic` floor, a different requirement list — this command is rebuilt to that
answer, and this page with it.
