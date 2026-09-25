# Signing

A human's signature is the only thing that authorizes a Warrant, resolves
one, corrects a delivered file, or accepts a SAS or roadmap revision (SAS
§27.2). The tool drafts the paperwork and checks it; it never signs.

## One act

```bash
war sign <target> --dry-run      # what the real ingest would say; writes nothing
war sign <target> --ssh-sign     # your agent's dialog is the signature
```

`<target>` is a Warrant alias (authorize or resolve), `<alias>/<D-id>` (a
correction), a SAS version, or `roadmap`. Each act costs two dialogs:
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

## When a Warrant is amended

An amendment makes a new revision, and authorizing it replaces the
Warrant's `authorization.toml`. Every attestation over the revision it
replaces names the old bytes, so `war` keeps them beside the new record as
`authorization.<digest8>.toml` (§34.4: supersede, never erase), and `war
attest --verify` finds them there by digest. The kept file is history, not a
second authorization: nothing reads it as one. If a file under that name
already exists with other bytes, the act is refused
(`authorize.retire-collision`) and nothing is written (OW-WAR-0144).

## From the web UI

`war ui`'s Queue shows each act's dry-run verdict and runs the same commands
from a button. When two or more acts would record, it also offers **Sign these
N in one dialog**: `war sign --batch=<the listed targets> --ssh-sign`. It is
dry-run as a batch first. Your key's dialog is still the signature
(docs/WEBUI.md).

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
