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
