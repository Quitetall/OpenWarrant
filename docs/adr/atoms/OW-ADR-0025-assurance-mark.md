---
schema: oh.war/atom/v1
adr_uuid: 01a0d678-e727-742f-a0fb-77f9324d9ffc
local_alias: OW-ADR-0025
role: adr
jurisdiction: bound
order: 30
classification: internal
status: proposed
governs:
  - "war://01a0d04c-602f-7321-ad70-864c543b2927"
---

# ADR OW-0025: The assurance mark — baseline v1, how a mark comes to exist, and what it binds

## Status

Proposed by the performer under OW-WAR-0135. **Not decided.** Two blocking
questions of that Warrant have no recorded answer from the owner:

- **Q-001**, whether a mark is issued or derived (options (a), (b), (c)
  below);
- **Q-002**, the contents of baseline v1, in particular the independence
  floor and whether "fixtures before implementation" is a v1 requirement.

Until the owner answers them and accepts this ADR, it records options and a
recommendation, and it governs nothing. `docs/assurance/baseline-v1.toml`
carries `status = "proposed"` for the same reason. When the owner answers,
the Decision section is rewritten to the answer, the options not taken stay
below as the record of the choice, and the baseline file's status becomes
`accepted` in the same change. OW-WAR-0135's M2 (`war mark`) does not start
before that.

## Context

The product spec promises an optional assurance mark
(`docs/design/openwarrant-product-spec.md`, "Optional assurance and
autonomous work") and leaves its engineering contract open ("Engineering
contracts still to specify": the exact versioned baseline and its evidence,
issuance, validation, and binding). What it has decided:

- **Q44.** A mark covers one accepted Warrant result, tied to its exact code
  revision and stated scope. Not a release, not a repository.
- **Q45.** OpenWarrant defines a versioned baseline. A repository may
  strengthen it, but not weaken it and keep the same mark.
- **Q46.** A human reviews the outcome, the verification findings and the
  remaining risks, and explicitly accepts the exact result. Independent
  verification supplies the technical review. The human act does not claim
  every line was read.
- **Q47.** "Implementation finished, unreviewed" carries no mark.
- **Q43.** Later qualification keeps the real history. A practice that was
  not followed stays unmet; qualifying later does not make it met.
- **C10, Q10.** Permission to run is not acceptance. Delegated authority
  does not earn the mark.
- **Q49.** The bundled default policy requires a mark before merging into
  main. Enforcing that is repository policy, not this ADR.

SAS 1.1.0 does not mention a mark. The records a mark would rest on already
exist, and each is signed or checked by something that is not the mark:

| record | what it already says | who guards it |
|---|---|---|
| `resolution.toml` | outcome, standing, contract digest, the human resolver, the meaning signed (§56.2) | the human act (§27.2), `resolution.stale` |
| `attestations/resolve-<n>.dsse.json` | an in-toto Statement over the resolution, SSHSIG-signed with the resolver's key | `war attest verify` (OW-ADR-0015) |
| `verifications/<OBL>.toml` | each obligation's disposition and the verifier's independence | `admissible_for` (§46.3, RQ-053) |
| `[locator]` in `resolution.toml` | the commit the delivered bytes were at, and whether they were committed | OW-ADR-0021 |
| `war pins --candidate` | whether the candidate still is the tree the human accepted | `acceptance.candidate-moved` (OW-WAR-0134) |
| `gate-runs/*.receipt.json` | a gate ran, bound to this contract, and passed | `evidence.admissible` (OW-WAR-0133) |

So a mark could be computed today by anyone, meaning anything. This ADR
fixes what it means.

## Q-001 — issued or derived

### (a) Derived — recommended

`war mark <alias>` computes the mark from records that already exist. No
new human act: the signed resolution *is* Q46's acceptance. Anyone can
recompute a mark; nobody can grant one. The mark adds no authority. It is a
reading of authority already exercised, against a named baseline.

- Consequence: the four human act kinds stay four, and SAS 1.1.0 needs no
  change for the mark to exist in this repository.
- Consequence: a mark can exist only where a human-signed, attested
  resolution exists. There is no path to a mark that skips the human act,
  because the mark has no input of its own.

### (b) Issued — rejected in the recommendation

A fifth human act, "mark", signed after resolution, with its own
attestation. It adds a signature over the same decision the resolution
already records, a new act kind in `roles.toml`, and a SAS revision
(§27.2's list and §106). The friction is real and the information is none:
the human who would sign it has already signed the resolution's meaning.
If the owner selects (b), it is escalated as a SAS revision, not built
under OW-WAR-0135 (its Frozen Surfaces).

### (c) Derived, and written down — recommended as a cache only

As (a), and `war mark <alias> --record` writes the statement to
`docs/warrants/<alias>/mark-<baseline>.json`, so it can be committed and
cited by digest. A written mark is never trusted: `war mark --verify`
recomputes it from the records and compares. A hand-written or stale file
therefore earns nothing; it is a claim to be checked.

**Recommendation: (a), with (c)'s file as a cache that `--verify`
recomputes.** Without an answer, M2 writes nothing.

## Q-002 — baseline v1 (proposed)

Each requirement is evaluated to exactly one of three results:

- **met**: the named record exists and says what the requirement asks;
- **unmet**: the record exists and says otherwise, or the record's absence
  is itself the answer (no resolution is Q47's "unreviewed", not a gap in
  the evidence);
- **UNKNOWN**: no record can answer the question. Law 15: UNKNOWN is never
  met. It refuses the mark and is reported by name, separately from unmet.

A mark exists only when every requirement is met. The data form is
`docs/assurance/baseline-v1.toml`; the tool reads it, and does not
hard-code the list.

| id | requirement | evidence | unmet when | UNKNOWN when |
|---|---|---|---|---|
| BL-001 | The Warrant is resolved `satisfied`, standing `valid` | `resolution.toml`: `common_outcome`, `standing` | no resolution (Q47), any other outcome, or a standing other than `valid` | never: the file either exists or not |
| BL-002 | A human signed the resolution, and it is attested | `attestations/resolve-<n>.dsse.json`, verified as `war attest verify` does; `resolved_by_ref` names an actor whose kind in `roles.toml` is human | no attestation (a TTY signature has no key), the signature or a subject digest does not verify, or the actor is not a human | `ssh-keygen` is unavailable, so the signature cannot be checked |
| BL-003 | Every declared obligation is established by an admissible independent verdict | `verifications/<OBL>.toml` per obligation in `60-assurance.md`, judged by `admissible_for` at the floor below | an obligation has no verdict, a verdict other than `established`, a verifier equal to the performer (RQ-053), or independence below the floor | never |
| BL-004 | The resolution names the commit it accepted, and the delivered bytes were committed there | `[locator]` in `resolution.toml`: `commit_sha`, `worktree_clean` | `worktree_clean = false` | no `[locator]` (resolved before OW-ADR-0021), or a `commit_sha` that is not forty lowercase hex |
| BL-005 | The candidate is still the tree the human accepted | `acceptance::assess` (OW-WAR-0134) at the tree marked | `acceptance.candidate-moved` | `acceptance.unknown` (no locator, or a commit git cannot read) |
| BL-006 | Every gate run the resolution relied on is admissible | each `gate_run_refs` receipt, judged by `evidence::admissibility` against the resolution's contract digest (OW-WAR-0133) | a receipt is missing, invalid, not a pass, or stale against the contract | its reuse cannot be judged (`evidence::Standing::ReuseUnknown`: the gate's definition is not registered, or the fixtures it declares are not named in the receipt) |

**The independence floor (BL-003).** Proposed: `controlled` (§46.3: a blind
process or agent review), applied to every verdict whatever the Warrant's
own `assurance_level`. A `basic` Warrant can still earn the mark, but only
if its verdicts would have been admissible at `controlled`. The owner may
choose `basic` instead; then a verifier that is merely distinct from the
performer suffices, and the mark says less. Either way the floor is named in
the baseline file, so the mark names it too. Per R-002, a mark at
`controlled` never claims a human reviewed the code.

**Considered and not in v1.**

- *Fixtures before implementation* (Q43). No record today shows the order
  in which fixtures and implementation were written, so it would be
  UNKNOWN for every Warrant and v1 could never be earned. It stays out of
  v1 rather than in and always UNKNOWN. A repository that wants it adds it
  (below), and its marks are then UNKNOWN on it until a record exists.
- *Human review of the full diff* (Q46's optional strengthening). A
  repository extension, not the baseline.
- *Delegated acts* (Q8–Q9). Out of v1. No delegated act earns the mark:
  BL-002 requires a human actor, and a §27.3 policy resolution has none.

## What a mark binds

A mark is an `oh.war/mark/v1` statement. Every field is a value the
records already hold, or a digest of a file in the tree, so it can be
recomputed later and compared:

| field | value |
|---|---|
| `warrant` | alias and uuid |
| `baseline` | `id` (`v1`), the sha256 of the baseline file, and each repository extension's id and sha256 |
| `resolution` | the path and sha256 of `resolution.toml` |
| `attestation` | the path and sha256 of the verifying `resolve-<n>.dsse.json` |
| `commit` | `locator.commit_sha` |
| `contract_digest` | `resolution.contract_digest` |
| `obligations` | the obligation ids in scope, each with the sha256 of its verdict file |
| `requirements` | each requirement id, `met` |

The statement carries no timestamp and no tool build, so recomputing it
over unchanged records yields the same bytes. `war mark --verify`
recomputes each field and names every one that moved: an edited
resolution names `resolution`; an in-scope source change names the
candidate (BL-005); an unchanged tree verifies.

A mark is about one resolution of one Warrant at one commit. It says
nothing about any other Warrant, the release that contains it, or the
repository (Q44).

## Strengthening, and not weakening

A repository declares its baseline in `openwarrant.toml`:

```toml
[mark]
baseline = "v1"
extra = "docs/assurance/repository.toml"   # optional
```

- The extra file holds requirements in the same form as the baseline's,
  with ids that do not collide with it. A mark under a strengthened
  baseline names the baseline **and** the extension (`v1` plus the
  extension's digest). It is a v1 mark and more, and says both.
- Removing a v1 requirement, or relaxing one (a lower independence floor,
  a requirement marked optional), is a different mark. The tool refuses to
  call it v1 and names each removed or relaxed requirement. A repository
  may run such a baseline under its own name; it may not name it `v1`.
- A baseline whose `status` is not `accepted` earns no mark: the tool
  refuses and says the baseline is not in force.

## Decision

*Pending Q-001 and Q-002.* If the recommendation is taken: (a) with (c)'s
cache, baseline v1 as the table above with the `controlled` floor, and the
strengthening rule above. The baseline's contents are fixed by this ADR and
its data file together; a later change to either is a new baseline version
and a new ADR (or a revision of this one), never an edit of `v1`.

## Consequences (if the recommendation is taken)

- The mark needs no new human act and no SAS change. It cannot exist over a
  resolution a human did not sign, because BL-001 and BL-002 read the
  signed record and its attestation, and nothing else can stand in for
  them.
- Every resolution recorded before attestations (OW-ADR-0015) or before
  the locator (OW-ADR-0021) earns no mark: BL-002 is unmet, BL-004 is
  UNKNOWN. That includes all 29 resolutions recorded today that lack a
  `[locator]`. This is Q43 working as intended: the records do not show
  the history a mark would claim. Such a Warrant qualifies later only
  through a new resolution with the records v1 reads.
- R-001: a derived mark is only as strong as the resolution under it. A
  resolution signed on an agent's wording earns the same mark. The mark
  says what the human act was, not that it was careful.
- Enforcing "mark before merge" (Q49) is repository policy. The mark is
  computable; OW-WAR-0134's CI step is where enforcement would go.
- Making the mark normative beyond this repository is a SAS revision that
  adds an RQ, proposed as a follow-up and not part of this ADR.
