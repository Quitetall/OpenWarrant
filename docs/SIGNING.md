# Signing

A human's signature is the only thing that authorizes a Warrant, resolves
one, corrects a delivered file, invalidates a Gate Definition version, or
accepts a SAS or roadmap revision (SAS §27.2). The tool drafts the paperwork
and checks it; it never signs.

## One act

```bash
war sign <target> --dry-run      # what the real ingest would say; writes nothing
war sign <target> --ssh-sign     # your agent's dialog is the signature
```

`<target>` is a Warrant alias (authorize or resolve), `<alias>/<D-id>` (a
correction), a SAS version, `roadmap`, or `<gate_id>@<version>` (a gate
invalidation, below). Each act costs two dialogs:
- one for the response;
- one for its attestation (the DSSE envelope `war attest --verify` checks).

## Many acts: the batch

```bash
war sign --batch --ssh-sign                        # every pending act that can be batched
war sign --batch OW-WAR-0001,OW-WAR-0002 --ssh-sign  # just these
war sign --batch --dry-run --as "<you>"            # the whole batch judged; writes nothing
```

A batch is one signature over a list.

**What happens:**
1. Every act's response is drafted exactly as `war sign` would draft it
   alone.
2. Each is judged by that act's own ingest in dry-run mode.
3. The drafts are listed — each by file name and exact sha256 — in one
   `oh.war/batch/v1` document under `docs/authority/batches/`.
4. You sign that document once.
5. Every act is judged again, then recorded through its own ingest.
6. The whole batch is attested in one envelope.

That is two dialogs for any number of acts.

**What you are agreeing to** is the list on the screen and in the batch
document: each act, its target, and the exact response it records. A
response whose bytes differ by one byte from what the batch lists is not
covered. `war check` then reads it as unsigned.

**What is left out, and named:**
- an act that needs a decision the batch cannot make (a resolution's
  outcome, a correction's kind) — sign it alone with its flag;
- a SAS acceptance — its ingest has no dry run, so a batch holding one could
  not promise all-or-nothing;
- an act for another role than the batch's — that is a second batch.

**Why one refusal refuses everything.** A batch applied in part would record
acts you signed as a list you did not sign: the list with a hole in it. So:
- one act that the dry run would refuse means nothing is signed;
- one record that moves between drafting and your dialog means nothing is
  recorded. The signed batch is kept as `.refused.json`, as evidence of
  what was attempted.
- one act that fails while recording means nothing is recorded: every
  directory the batch wrote (the responses and each act's Warrant or
  roadmap) is copied before the first write and put back byte for byte,
  and the batch is refused as `batch.incomplete`.

What this does not cover: a process killed in the middle of recording
(power loss, `kill -9`). That is crash recovery, OW-WAR-0130's work; until
it lands, `git status` shows what an interrupted batch left, and `git
checkout` of those paths undoes it.

**How the check believes it.** `war check` believes a batched record the way
it believes a singly signed one, by a signature over its exact bytes:
- the response carries no `.sig` of its own;
- a batch whose signature verifies as the record's actor lists that
  response's sha256.

Nothing in the record claims the batch; the batch claims the record.

## One sitting: authorize and resolve after the work is done

The agent finishes the work without asking you for anything; you then do
every act for it in one sitting. That works because a signature does not
make recorded evidence stale: the tree rule skips the records a human act
writes (see "The reuse rule" in `docs/RESOLVING.md`).

**Before the sitting (the agent, no signature):**
1. Deliver, and declare it in `deliverables.toml`.
2. `war verify <alias> --run` (or `--response` from an independent
   verifier), commit.
3. `war evidence record <alias>`, `war compile`, commit — **last**. A
   verification record is not an authority record: committing one after the
   evidence moves the tree a no-`inputs` receipt names, and the agent
   records again.

**The sitting, in this order:**
1. A SAS or roadmap acceptance, if one governs the work, alone
   (`war sign <version> --ssh-sign`). Accepting one changes what the corpus
   is judged against, so it is bound like source: it stales every
   tree-bound receipt, and the agent must record evidence again before
   step 3. Keep acceptances out of a sitting that resolves.
2. Authorize: `war sign --batch --ssh-sign` (every pending authorization),
   or `war sign <alias> --ssh-sign`. Commit nothing yet if you like; the
   records are skipped either way.
3. Resolve: `war sign --batch --ssh-sign` again (or `war sign <alias>
   --ssh-sign`; `--outcome …` when §38.6 forbids `satisfied`).
4. Commit what the sitting wrote.

**Why authorize and resolve cannot be one batch.** Three rules, each on
purpose, put them in two:
- a resolution is offered only for an authorized revision (`war sign
  --list` shows it once the authorization is recorded, not before);
- its §56.1 requirements are judged when it is drafted, and requirement 1
  is the authorization it would follow;
- a batch is one signer in one role, and authorizer and resolver are two
  roles.

So a sitting is two batches with nothing between them, not one. Each is
two dialogs. `conformance/plants.d/55-evidence-signing.sh` runs this
sitting end to end with a throwaway key.

## When a Warrant is amended

An amendment makes a new revision, and authorizing it replaces the
Warrant's `authorization.toml`. Every attestation over the revision it
replaces names the old bytes, so `war` keeps them beside the new record as
`authorization.<digest8>.toml` (§34.4: supersede, never erase), and `war
attest --verify` finds them there by digest. The kept file is history, not a
second authorization: nothing reads it as one. If a file under that name
already exists with other bytes, the act is refused
(`authorize.retire-collision`) and nothing is written (OW-WAR-0144).

## Invalidating a gate

```bash
war gate invalidate <gate_id>@<version> --grounds "<why>"      # the request; writes nothing
war sign <gate_id>@<version> --grounds "<why>" --dry-run        # what the ingest would say
war sign <gate_id>@<version> --grounds "<why>" --ssh-sign       # HUMAN: the act
```

Nothing in the records asks for an invalidation, so it is never in `war
sign --list` or `--all`: it exists when you name the gate and the grounds.
The grounds are your words, signed as written; every dispute repeats them.
The screen lists every resolution the sweep reaches — directly through a
receipt, or through a resolved parent — and the response names them, so
your signature covers exactly that list. Only a human holding `resolver`
who is not the performer may sign it; it is never batched. What it writes
and what it never rewrites: docs/INVALIDATION.md.

## From the web UI

`war ui`'s Queue shows each act's dry-run verdict and runs the same commands
from a button. When two or more acts would record, it also offers **Sign these
N in one dialog**: `war sign --batch=<the listed targets> --ssh-sign`. It is
dry-run as a batch first. Your key's dialog is still the signature
(docs/WEBUI.md).

## Routine work under a standing authorization

> Contingent: built on OW-WAR-0142's branch against the recommended answers
> to its Q-001 (A), Q-002 (as drafted) and Q-003 (refuse). The owner has not
> answered them, and SAS 1.2.0 (§28.8) is proposed, not accepted.

A standing authorization is one signature over a *class* of routine work
(OW-ADR-0029). Each Warrant inside the class is then authorized in your name
with no further act, and you read each change once, at resolution.

```bash
war standing propose class.toml              # validate and place it; covers nothing yet
war standing show [routine]                  # terms, every glob and what it matches today
war sign standing:routine@1 --dry-run        # every refusal, nothing written
war sign standing:routine@1 --ssh-sign       # 2 dialogs, once per class
war standing apply <alias> [--dry-run]       # the agent's act: inside → authorized; outside → refused
war sign --batch A,B,C --ssh-sign            # the round's resolutions: 2 dialogs
war sign standing:routine@1 --revoke --ssh-sign
```

What the class states, every term required and no other accepted:

- `paths`: globs a covered Warrant may declare. A glob that could reach an
  authority path is refused when proposed, when signed and when applied —
  `docs/authority/**`, `openwarrant.toml`, the SAS, the roadmap, the ADRs,
  the gates and `conformance/**`, the guards, any `Cargo.toml` or
  `Cargo.lock`, every authorization record and generated projection, and the
  code that decides authority (`standing.rs`, `authorize.rs`, `sign.rs`,
  `batch_cmd.rs`, `authority_check.rs`, `attest.rs`, `verify.rs`,
  `resolve.rs`, `resolution_cmd.rs`, `ownership.rs`, `standing_cmd.rs`). The
  set is a constant in the tool; no file extends it.
- `profile = "delivery"`, `assurance = "basic"`.
- `gates`: what obligations may cite; always includes
  `gate://ops.conformance.plants@1.1.0` and
  `gate://software.repo.war-check@1.0.0`.
- `budget`: per-stage `budget_tokens` (≤ 24000) and `wall_time_seconds`
  (≤ 1800), `max_stages`, `max_deliverables`. An agent stage that states no
  `budget_tokens` is refused: its budget is unbounded.
- `expires_at` (≤ 90 days after you sign) and `max_warrants` (≤ 50).
- `meaning`: your words for what the signature grants.

There is no term for a residual risk, an ADR or a resolution, so a covered
Warrant carries none of them and is always resolved by a human; a policy
service is refused by name (`resolve.standing-needs-human`).

A Warrant names its class in its manifest (`[standing] ref =
"standing://routine@1"`) or with `--class`. `apply` refuses, by the term
broken and writing nothing: a path outside the globs, `controlled`, a
`decision`, an ADR atom, an accepted residual risk, a gate the class does not
name, a stage over budget, after `expires_at`, past `max_warrants`, after a
revocation, under a class nobody signed, and a path whose current owner is an
authorized Warrant that has not resolved (`standing.in-flight-owner`: that
Warrant resolves first, or you sign the change on its own).

Nothing about a covered authorization is trusted. `war check` re-derives it
every run: the class's signature over its exact bytes (`standing.unsigned`
when a byte moved), the record's time against the expiry, count and
revocation, and the Warrant as it stands against every term
(`standing.outside-class`). A wider class is a new revision you sign again;
Warrants covered under the old one keep it. Records made before a revocation
stand (`standing.revoked`, a warning).

What it costs, counted by `conformance/plants.d/69-standing.sh` over three
routine Warrants: the class, 2 signatures once; before work, 0; the round's
resolutions, 2. The same three as ordinary Warrants, batched: 4 (an
authorize batch and a resolve batch), and none can start before the
authorize batch.

## Load the key with confirmation

```bash
ssh-add -c ~/.ssh/<your key>
```

Without `-c`, anything that can reach your agent can sign as you. `war`
cannot check this. Test Deny before you trust Allow.

## After you sign: when the candidate changes before merge

A resolution records what you accepted, including a `[locator]`: the commit
the delivered bytes were at when you signed. Between your signature and the
merge the tree can still move: the target branch moves and the Warrant branch
is rebased or merged, a conflict is resolved by hand, a squash produces a
commit the locator does not name.

```bash
war pins --candidate                      # HEAD, against every resolved Warrant
war pins --candidate <rev> --base <rev>   # only resolutions not already on <base>
```

For each resolved Warrant it answers one of:
- `unchanged`: every path that moved since the locator is out of scope;
- `acceptance.candidate-moved`: an in-scope path moved, or a pinned
  deliverable's bytes no longer match the digest you signed for. Each path is
  named, and the command exits non-zero;
- `acceptance.unknown`: the resolution has no locator (resolved before
  OW-ADR-0021), or git cannot read the locator commit. It is never reported
  as unchanged, and the command exits non-zero;
- `acceptance.landed` (with `--base`): the resolution is already on the base,
  so it merged earlier and is not a candidate now.

In scope means what the gates the Warrant cites read: each Gate Definition
may declare `inputs` globs, and a gate that declares none puts the whole
tree in scope. The Warrant's own records (its directory, the corpus
projections, its signed responses) are not the candidate.

CI runs `war pins --candidate HEAD --base HEAD^1 --resolved-only` on every
pull request, against the merge commit. A moved acceptance fails the job
before merge, not after.

**What the finding means.** The candidate about to land is not the one you
accepted, in a way the Warrant's gates would see.

**What it does not mean.** The resolution is not wrong. It is still true of
the candidate it accepted, and nothing writes to `resolution.toml`, disputes
it, or annuls it.

**What it requires next (Q-001, answered (b)).** An in-scope change before
merge requires re-running the gates and the independent verifier on the new
candidate: re-run the cited gates, then hand `war verify <alias>` to a
verifier that is not the performer and ingest its answer with
`war verify <alias> --response <file>`. When every obligation is
re-established on that candidate, the finding clears for that candidate only;
a further in-scope commit raises it again. Your acceptance carries forward;
no new signature is asked of you.

**When a pinned deliverable itself changed (Q-001 (a)).** A new human
re-acceptance is required, and today that act is a correction:
`war correct <alias> <D-id>` emits what you sign. Re-verification does not
clear this case.

Two limits, stated so they are not mistaken for coverage:
- `war evidence record` replays a gate run that is still admissible for the
  contract, so re-running a gate on the new tree is the verifier's to demand
  until receipts are bound to the tree (OW-WAR-0133). The finding clears on
  the ingested verification.
- A squash or rebase merge can make a locator unreachable once the branch is
  deleted; from then on the answer is `acceptance.unknown`, for good.
