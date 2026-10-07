# Resolving a Warrant

Everything up to this point can be done by a tool. This cannot.

`war resolve --dry-run <alias>` evaluates §56.1's thirteen requirements. Four of
them read records that only a human may write, because §27.2 says an agent SHALL
NOT authorize a proposed WAR, accept organizational residual risk, or resolve a
delivery. No amount of implementation changes that — the commands below exist to
put the decision in front of you, not to make it.

## What is blocking, right now

Measured across the fifty Warrants in this repository:

| Requirement | Warrants blocked | Who can clear it |
|---|---|---|
| exact authorized Contract Revision | 50 | you |
| resolver holds the role | 50 | you |
| required judgments exist | 48 | you |
| residual risks have sufficient authority | 48 | you |
| every required gate has admissible result | 12 | needs an amendment (below) |
| required deliverables exist | 9 | undelivered or unmapped work |
| artifact digests verify | 9 | same nine |
| no required unknown remains | 3 | blocked on external repositories |
| no blocker remains | 3 | same three |
| runtime receipts match the basis | 2 | needs Katana / a BLUT run |

## Step 1 — grant the roles

There is no `war authority grant`. A command that could write
`docs/authority/roles.toml` at will would let an agent assign itself the roles
§27.2 exists to withhold, through the same tool that later checks whether the
assignment is valid. Authority has to enter from outside — so the one tool
that writes this file does it only from a human's answers typed at a terminal,
once, and never edits it afterwards (OW-ADR-0021, Consequences):

```bash
war init                              # at a terminal: asks your name, your key, and writes both files once
# or by hand:
cp docs/authority/roles.toml.example docs/authority/roles.toml
$EDITOR docs/authority/roles.toml     # put your name in, keep `claude` as performer only
```

Until this file exists, every `war authorize` request prints:

```
# NOBODY may authorize this: docs/authority/roles.toml grants the
# authorizer role to no one.
```

## Step 2 — read what you are signing

```bash
war authorize OW-WAR-0030 > /tmp/OW-WAR-0030.request.toml
```

The request carries the exact contract digest, which of §28.5's seventeen
elements that digest actually covers, the obligations, and every residual risk
the Warrant declared, with the consequence if it turns out false. It carries no
recommendation and no suggested wording for `meaning` — §42 says an approval with
no stated meaning is invalid, and pre-filling it would make you a signatory to
text an agent wrote.

## Step 3 — sign it

**The short way, from a terminal you are sitting at:**

```bash
war sign --list                 # what awaits a signature, corpus-wide; writes nothing
war sign OW-WAR-0030            # one screen: title, obligations, every residual risk
                                # with its consequence, the digest. [y/N]
```

`y` drafts the response below from the record's own facts, writes it to
`docs/authority/responses/`, and runs the same ingest a hand-written response
goes through — every refusal still applies. `N` writes nothing. `--edit` opens
the draft in `$EDITOR` first; `--meaning "…"` appends your own words. An
authorization response records `signed_via = "tty"`; a resolution or SAS
acceptance carries the same provenance in its `meaning` until OW-WAR-0064
lets their pinned response types move.

It **refuses without a terminal**: an agent's shell has none, so the drafting
agent cannot run it (§27.2). That is a speed bump, not cryptography — a
pseudo-terminal defeats it. There is no `--yes`, on purpose.

**From inside an agent session (Claude Code's `!`, a pipe, anywhere without a
terminal), use the key instead:**

```bash
ssh-add -c ~/.ssh/id_ed25519            # ONCE per login: confirmation ON
war sign OW-WAR-0030 --ssh-sign          # a dialog asks you; no prompt, no TTY
war sign OW-WAR-0030 --verify            # later: does the .sig still verify?
```

`--ssh-sign` signs the response file's bytes with `ssh-keygen -Y sign` under
namespace `oh.war/response`, verifies at once against
`docs/authority/allowed_signers`, refuses if it does not verify, and writes a
`.sig` sidecar beside the response. It needs `ssh_principal = "…"` on your
`roles.toml` entry and a matching line in `allowed_signers` — both human-written
(see `allowed_signers.example`). **The human act is the agent's confirmation
dialog, which exists only if the key was loaded with `ssh-add -c`.** Without
`-c`, the AI agent's shell can reach your agent socket and sign as you, and `war`
cannot tell the difference. This is the one thing you must get right.

**The long way**, which is what `war sign` does for you and which still works:

Turn the request into a response. The `contract_digest` must be copied across
unchanged: if the Warrant is edited between reading and signing, the digest moves
and ingestion refuses, because §56.1 asks for the *exact* authorized revision.

```toml
schema = "oh.war/authorization-response/v1"
warrant = "OW-WAR-0030"
contract_digest = "82776d93…"        # copied from the request, verbatim
authorizer = "your-name"
acting_role = "owner"                 # §27.4 — the role you ACTUALLY exercised
meaning = "…"                         # what authorizing this means. Not optional.
effective_time = "2026-08-25T18:00:00Z"
independence = "none"                 # none | separate_role | organizational

# One judgment per residual risk in the request. Without these, requirements 9
# and 11 stay unmet — a declared risk with nothing accepting it.
[[judgment]]
id = "J-001"
kind = "residual_risk_acceptance"
statement = "…"
actor = "your-name"
acting_role = "owner"
meaning = "…"                         # §42: an approval with no meaning is invalid
authority = "authorized"
basis_refs = ["assumption://A-001"]   # the assumption id from the request
```

`independence = "none"` is the honest value for a sole-owner repository, and
§27.4 is explicit that role separation by one person is not organizational
independence. Recording `none` is not a failure; claiming otherwise would be.

```bash
war authorize OW-WAR-0030 --response /tmp/OW-WAR-0030.response.toml
```

A refused response writes **nothing**. A rejected authorization must not become a
file that later reads as authority.


## Step 4 — record the gate run, then resolve

Requirement 5 ("every required gate has admissible result") reads ONLY a
Warrant's own committed `gate-runs/`. Nothing under the gitignored
`docs/receipts/` counts. To record evidence for a Warrant whose obligations
are established:

```bash
war compile                                  # the gate checks the projection; make it fresh
war evidence record OW-WAR-0010              # runs every cited gate, mints §44.6 receipts into
                                             # docs/warrants/OW-WAR-0010/gate-runs/, bound to the
                                             # Warrant's current contract digest
war compile                                  # the receipt changed the projection; refresh before the next
```

A receipt counts only while it reseals, is bound to the contract as it
compiles now, and the source it ran over still holds; edit the contract or
that source and `war check` reports `evidence.stale-binding` until a new run
is recorded (see "When recorded evidence still counts" below). Commit the
receipt, the journal line and the refreshed projection together; none of
them moves the source a receipt names. A Warrant whose assurance atom cites no gate cannot
record evidence (OW-WAR-0016 today) — that needs an amendment naming a gate.

When all thirteen are met, the resolution is the third two-half seam. From a
terminal, `war sign OW-WAR-0010` does the whole of it; when §38.6 forbids
`satisfied` it refuses to guess and asks for `--outcome not_satisfied|cancelled|blocked`.
The long way:

```bash
war resolve OW-WAR-0010 > /tmp/OW-WAR-0010.resolution.request.toml   # what a signature binds; permitted outcomes; who may sign
```

Turn it into a response and ingest it. `satisfied` is accepted only when every
declared obligation is established by an admissible verification (§38.6); an
agent is refused as resolver whatever the file says; a second resolution is
refused — §56.4 dispute and §56.5 annulment change one, overwriting does not.

```toml
schema = "oh.war/resolution-response/v1"
warrant = "OW-WAR-0010"
contract_digest = "…"                 # copied from the request, verbatim
resolved_by = "your-name"
acting_role = "resolver"
common_outcome = "satisfied"          # one of the request's permitted_outcomes
profile_outcome = "delivered"
meaning = "…"                         # §56.2: what accepting asserts. Not optional.
effective_time = "2026-09-02T18:00:00Z"
```

```bash
war resolve OW-WAR-0010 --response /tmp/OW-WAR-0010.resolution.response.toml
war compile                           # the Warrant now reads `resolved`; the Release axis moves
```

## When recorded evidence still counts

The contract digest does not cover the delivered code, the deliverable set or
a gate's fixtures (OW-ADR-0004). A receipt bound to the contract alone would
survive any edit to them. OW-WAR-0133 closes that gap.

### What a receipt names

`war evidence record` adds these entries to `subject_digests`, next to
`contract:sha256:…`. They are new entries, not new fields, so every receipt
already on disk still parses and still reseals.

| subject | what it names |
|---|---|
| `tree:<sha>` | `git rev-parse HEAD^{tree}` when the gate started |
| `worktree:dirty` | the working tree differed from that tree when the gate started |
| `inputs:sha256:<hex>` | the bytes of every file the Gate Definition's `inputs` globs match |
| `deliverables:sha256:<hex>` | the bytes of the Warrant's declared deliverables, in `D-` order |

The Bonsai evidence gate (`software.repo.bonsai-evidence`) verifies a
supplied `war bonsai check` document rather than the tree, so its receipt
also binds that document by bytes in `raw_evidence_refs` (t-dec1). Record it
with the reference:

```bash
war evidence record OW-WAR-0050 --gate software.repo.bonsai-evidence@1.0.0 \
    --evidence-ref "file:bonsai-evidence.json#sha256:$(sha256sum bonsai-evidence.json | cut -d' ' -f1)"
```

The document must be a passing report for the Warrant's current contract,
at the path the gate definition passes to `--evidence`, and committed (an
untracked file is `worktree:dirty`). Without the reference the gate is not
run and `evidence.bonsai-evidence-ref-required` names this remedy; a
reference to another file is `gate-run.bonsai-ref-not-read`; a reference is
refused for any other gate. The receipt carries one contract subject and the
deliverable subject beside it, and the observed tree.

`fixture_digests` holds one `<path>#sha256:<hex>` for each file the
definition lists under `fixtures`. It is empty only when the definition lists
none. A declared fixture that cannot be read stops the run before it starts.

### The reuse rule

For a Warrant that is not resolved, a recorded run counts toward requirement 5
only if all of these still hold:

1. **The contract.** The receipt names the current contract digest.
2. **The fixtures.** Every recorded fixture digest matches the file's bytes
   today.
3. **The source.** If the gate's definition declares `inputs`, the files those
   globs match must digest as they did. A definition may declare, for example,
   `inputs: ["crates/**", "Cargo.lock"]` and `fixtures: ["conformance/x.toml"]`.
   These are new keys, and a definition is immutable (§43.3), so a shipped
   gate that adopts them takes a new version. If the definition declares no
   inputs, the tree decides instead. The run holds only if nothing has changed
   since the recorded tree, and a run over a dirty tree has no tree to hold to.

The source comparison skips a Warrant's `gate-runs/` and `journal.jsonl`,
because recording evidence writes them. It also skips the projections
`war compile` writes (`generated/` under the warrants, ADR and SAS roots, and
`docs/generated/`), because the corpus status projects each run's
admissibility. Without these exclusions, committing a receipt would move the
tree the receipt names. `war check --generated` checks the projections
against their sources.

#### A signature does not move the tree (t-22fd)

The tree rule also skips the records a human act writes. Without that, a
signature recorded after the evidence staled every receipt of a gate with no
`inputs`, and every Warrant needed two sittings: authorize, then — after the
agent recorded evidence again — resolve. What is skipped, file by file:

| act | records skipped |
|---|---|
| every signed act | `docs/authority/responses/**` (responses, `.sig`, retired copies) |
| authorize | `<alias>/authorization.toml`, `authorization.<digest8>.toml`, `judgments.toml`, `<alias>/attestations/**` |
| resolve | `<alias>/resolution.toml`, `<alias>/attestations/**` |
| correct | `<alias>/corrections/**` (the corrected file's bytes stay bound) |
| batch | `docs/authority/batches/**` |
| invalidate | `docs/gates/invalidations/**`, `<alias>/disputes/**` |
| standing accept or revoke | `docs/authority/standing/attestations/**` |
| SAS or roadmap accept | `revisions/attestations/**` under the SAS and roadmap roots |
| answer a question | `<alias>/questions/**` |

What stays bound, though a human writes it: `docs/authority/roles.toml` and
`allowed_signers` (trust roots, edited by hand), a standing class file (the
agent's proposal), and a SAS or roadmap revision record. Accepting a revision
changes the specification the corpus is judged against, so it is source, not
a record about the work; a SAS acceptance still stales tree-bound receipts,
and belongs before the evidence (`docs/SIGNING.md`, "One sitting").

**Why exclude, and not ask gates to declare inputs.** The gates this
repository cites most — `software.repo.war-check` and the battery — read the
whole corpus, authority records included: a signature can change what `war
check` says. Declaring that as `inputs` would bind them to every signature
and bring the two sittings back. What such a receipt would add about
authority records is already established elsewhere, and more strictly:

- each record is verified by its own act when it is written (the signature
  over its exact bytes, the digest it binds, the signer's role);
- a resolution judges the records it rests on live — the authorization of
  the exact contract (requirement 1, its signature re-verified), the
  judgments (9), the risk acceptances (11), the resolver's role (13);
- an invalidation stops a gate's receipts by name
  (`evidence.gate-invalidated`), not through the tree.

So the stated limit is: **a receipt of a gate that reads authority records
says nothing about authority records written after it.** `war check` run
now still reads every one of them.

Both mechanisms remain. The exclusion narrows only the tree fallback;
declared `inputs` are not narrowed. A gate whose verdict is about authority
records declares them in `inputs` and is held to them. Source code is not
touched by either: any other path that moves — a source file, an atom, a
deliverable, a Gate Definition, `roles.toml` — stales the receipt by name,
as before. `conformance/plants.d/55-evidence-signing.sh` shows both halves:
a receipt recorded, an authorization signed, the receipt still admissible
and the Warrant resolved in the same sitting; and a source byte, or
`roles.toml`, changed after the receipt, which stales it by name.

#### Working a ticket does not move the tree (t-5d82)

The ticket loop (`docs/TICKETS.md`) writes files as it is worked: `war claim`
a lock and a journal line, `war done` a ticked checklist line, `war note` a
dated line in the intent, `war add` an item, `war create` a new ticket. An
agent works its ticket after recording evidence and commits the ticket files
with the work, so each of those used to stale every receipt of a gate with no
`inputs`. The tree rule now skips them, file by file:

| written by | skipped |
|---|---|
| every ticket command | `<tickets>/<t-id>/journal.jsonl` |
| `war claim`, `release`, `steal` | the lock files directly inside the claims directory (it also ignores itself in git) |
| `war done`, `war add`, a claim naming a new line | `<tickets>/<t-id>/atoms/15-checklist.md` (the ticket profile's checklist file) |
| `war note`, `war create` | `<tickets>/<t-id>/atoms/10-intent.md` |
| `war create`, `war promote` | `<tickets>/<t-id>/manifest.toml` |

`<tickets>` is `[tickets] dir` and `<t-id>` a ticket id (`t-` and 3 to 16 of
`[0-9a-z]`). Nothing under the Warrants root is skipped, whatever
`[tickets] dir` says.

**Why the checklist and the intent too, and not only the journal and the
claims.** The journal and the claims are plainly bookkeeping. The checklist
and the intent are a person's durable document: plain Markdown, edited by
hand, and the file is the state. But they are a document *about* the work:
a ticked box is the state of the work written down, and a note is context
for the next person. Neither is the source a gate runs over. A ticket is the
working form — never compiled, authorized, verified or resolved — and no
Warrant, deliverable or Gate Definition in the corpus reads one as its
source; `war promote` copies a ticket's text into a new Warrant's intent,
and that Warrant's atoms are bound like any other. Excluding the journal
alone would not fix anything: `war done` and `war note` write the checklist
and the intent on every use. Telling a tick apart from a reword by comparing
the file's contents was rejected: the tool honours hand edits as written, so
a tick and a rewrite of the same line can arrive in one edit, and a content
rule would have to guess which it was.

**What a verdict could depend on, said once.** `war check` validates a
ticket's structure (`ticket.checklist-malformed`, `ticket.item-duplicate`,
`ticket.blocker-unknown`, `ticket.blocker-cycle`). So a `war check` receipt,
like a battery receipt, **says nothing about ticket files written after it**
— the same stated limit as for authority records above. `war check` run now
still reads every ticket, and the Claude Code plugin's Stop check blocks on
its errors. A gate whose verdict is about tickets declares them in `inputs` and
is held to them; declared `inputs` are not narrowed.

**What stays bound.** Anything else in a ticket's directory (an attachment,
another atom a person adds), `docs/TICKETS.md`, the ticket profile
`profiles/ticket.toml`, `openwarrant.toml` (where `[tickets]` is), and every
path outside the tickets and claims directories.
`conformance/plants.d/55-evidence-tickets.sh` shows both halves: a receipt
recorded, then `war claim`, `done`, `note` and `create` (uncommitted, then
committed), and the receipt still admissible naming its tree; and a source
byte, or a file in a ticket's directory the loop does not write, changed
after the receipt, which stales it by name.

#### A verification does not move the tree (t-fed6)

The independent verifier reads the receipts — they are in the bundle it is
handed — and writes its verdicts after them. Bound to the tree, that write
staled every receipt of a gate with no `inputs`; recording the evidence
again changed the receipts the verdicts had been made against, and the
verdicts were then about evidence that no longer existed. A loop with no
fixed point. The tree rule now skips what `war verify` writes under a
Warrant's `verifications/`, file by file:

| written by | skipped |
|---|---|
| `war verify --response`, `--run` (the ingest) | `<alias>/verifications/<name>.toml` — one obligation's verdict; every `*.toml` directly there is read as one |
| `war verify --bundle`, `--run` | `<alias>/verifications/bundle-<digest16>.json` — what the verifier was handed |
| `war verify --run` | `<alias>/verifications/responses/**` — the verifier's whole response, kept when the ingest accepted it |
| the ingest | `<alias>/journal.jsonl` — already an evidence record |

Nothing else between the evidence and the sitting writes into the
repository: `war verify` with none of `--response`, `--bundle` or `--run`
prints the request and writes nothing, and `war document review` (what
`document.review@1.0.0` runs) writes nothing — its run is recorded like any
gate's, under `gate-runs/`. Two outputs are outside the tool's reach, and
stay bound if they land in the tree: a response file handed to
`war verify --response` from anywhere but `<alias>/verifications/responses/`
(keep it there, or outside the repository), and the blind verifier's
diagnostic log (`tools/verifier/claude-verifier.sh` writes one only when
`CLAUDE_VERIFIER_LOG` names a directory — name one outside the repository or
an ignored one).

**Why these are records, not source.** A verification is a judgment about
the evidence, made by someone who did not produce it; no deliverable, atom
or Gate Definition is built from one. What a receipt would add about
verifications is already established elsewhere, and more strictly:

- each verdict is judged by its own ingest when it is written —
  admissibility against the assurance atom, independence from the performer,
  the verifier register — and a refused verdict is never written;
- a resolution judges them live: `satisfied` is accepted only when every
  declared obligation is established by an admissible verification (§38.6),
  read from `verifications/` each time the resolution is assessed.

So the stated limit is: **a receipt of a gate that reads verification
records says nothing about verification records written after it.** Two
gates cited here read them: `document.review@1.0.0` (every obligation of a
document Warrant needs an established independent verification) and
`software.repo.war-check@1.0.0` (`war check` reads the whole corpus,
verifications included). Run now, either still reads every one. This is also
what lets `document.review@1.0.0` be recorded after the verification it
needs without that verification staling the rest of the evidence
(`docs/SIGNING.md`, "One sitting").

**What stays bound.** Anything else under `verifications/` (a note, a
subdirectory other than `responses/`, a file named like a bundle that is not
one), a `verifications/` directory anywhere but directly in a Warrant, and
everything the subsections above leave bound. Declared `inputs` are not
narrowed: a gate whose verdict is about verifications declares them and is
held to them. `conformance/plants.d/55-evidence-verifications.sh` shows both
halves: a receipt recorded, then a verifier's response ingested, a bundle
written and a `--run` response kept (uncommitted, then committed), and the
receipt still admissible naming its tree; and a source byte, or a
non-record file under `verifications/`, changed after the receipt, which
stales it by name.

The deliverables digest is recorded and advisory. A gate is judged on what it
declares it reads. If a gate reads files outside its declared `inputs`, it can
keep a stale pass. The tree subject in the receipt shows that happened; it
does not prevent it.

| finding | `war check` rule | severity | requirement 5 |
|---|---|---|---|
| every subject holds | `evidence.admissible` | pass | counts |
| the contract, an input, a fixture or the tree has moved | `evidence.stale-binding`, naming the recorded subject | warning | does not count |
| the rule needs a subject the receipt does not name, or the source cannot be read: every receipt minted before OW-WAR-0133, a run over a dirty tree, a gate with no registered definition | `evidence.reuse-unknown` | warning | does not count |
| the receipt does not reseal, or disagrees with its run | `evidence.receipt-invalid` | error | does not count |

A stale or unknown run is a true record. It is not a failure, and it is not
evidence about the source as it stands now (Law 15). The fix for either is
`war evidence record <alias>`. The reuse rule never makes a `reuse-unknown`
receipt count. `war status` labels the two apart as well: a moved subject is
`stale_binding`, a reuse-unknown run is `reuse_unknown`.

A **resolved** Warrant is not re-evaluated. Its receipts are history, and its
resolution still binds them (RQ-059). `war check` reports them as admissible
with "resolved: the source is not re-evaluated". No resolution record is
rewritten.

### The compiler's three rules for a Dispatch's context

- **Source.** Every required atom is included whole, or the Dispatch is
  refused (`RequiredAtomUnaccounted`, `RequiredAtomOmitted`). Precedence is
  still assigned by item kind: a Warrant's atoms are `authorized_war_contract`
  and everything else is `informative_source`. No Warrant can declare
  precedence yet (§33.4; U-003 of OW-WAR-0133).
- **Omission.** A required item is never omitted, and every omission carries a
  reason (`context.rs`, `RequiredItemOmitted`). If a stage's budget cannot hold
  its required atoms, the whole Dispatch is refused by `dispatch.over-budget`,
  which names the largest items. Nothing is dropped to fit.
- **Conflict.** One mechanical kind is detected: a single source path included
  whole at two digests or at two revisions. The compiler refuses that Dispatch
  (`SourceConflict`) and names both versions. A section of a file is a
  selection, not a version, and never conflicts. Two different sources that
  disagree in prose are not detected, because no mechanical rule exists for
  that. The stage selector never includes one path whole twice, so today this
  refusal guards other callers of the compiler, not `war dispatch`.

  The manifest `war dispatch --emit-context` writes says what its conflict
  fields are a record of (AM-002). Beside `conflicts` it carries
  `conflict_check`: `checked` names `same-source-two-versions`, `found` lists
  what that check found (a path and its versions), and `unchecked` names
  semantic disagreement between different sources as not checked. An empty
  `found` means that one check ran and found nothing. It says nothing about
  the unchecked kind. A manifest with no `conflict_check` was compiled before
  it existed, and its `conflicts: []` is unchecked, not clean.

## Step 5 — correcting a delivered artifact after resolution

A resolution binds `sha256(deliverables.toml)`, so a resolved Warrant's pinned
files have no ordinary way to change: `war check` reports
`deliverable.digest-drift` and "regenerate the record" would stale the
resolution. The correction act (OW-WAR-0064, OW-ADR-0012) is the fifth two-half
seam, for exactly that case:

```bash
war correct OW-WAR-0056 D-002              # the request: recorded digest, chain head, the file now, who may sign
war sign OW-WAR-0056/D-002 --kind behaviour-change --meaning "continuation lines belong to their bullet"
```

`war sign` refuses to draft a correction without `--kind`
(`behaviour-change` | `added-refusal`) and a reason: those two are the human's
whole contribution and nothing guesses them. On `y` (or the ssh dialog) it runs
the same ingest as a hand-written response:

```toml
schema = "oh.war/correction-response/v1"
warrant = "OW-WAR-0056"
deliverable_id = "D-002"
superseded_digest = "sha256:…"     # the chain head from the request, verbatim
new_digest = "sha256:…"            # the file's bytes now, verbatim
reason = "…"
kind = "behaviour-change"
corrected_by = "your-name"
acting_role = "authorizer"
effective_time = "2026-09-11T12:00:00Z"
```

```bash
war correct OW-WAR-0056 D-002 --response /tmp/OW-WAR-0056.D-002.correction.toml
```

The record lands as `docs/warrants/OW-WAR-0056/corrections/D-002-1.toml`;
`deliverables.toml` is not edited and the resolution still verifies. A second
change is a second file, `D-002-2.toml`, superseding the first's `new_digest` —
never an edit: the journal witnesses each record's digest as written, and an
edited one fails `correction.edited`. Refused before anything is written: an
agent as signer, a Warrant that is not resolved, a file that has not drifted, a
`new_digest` that is not the file's bytes, a `superseded_digest` that is not the
chain head, an empty reason, a bad `effective_time`. `war show <alias> --view
status` lists every correction with the digest it superseded.

## Step 6 — signing what is already recorded

Every authority record is read as believed only when a human signature verifies
over it: `war check` reports `authority.signed` per act, and
`authority.unsigned`, `.signature-invalid`, `.actor-not-human` or
`.verify-unavailable` when it cannot. §56.1 requirement 1 and `war dispatch`
both depend on that verdict, so an unsigned authorization now stops the work it
was supposed to authorize.

A corpus written before that rule has records with no signature. They are not
rewritten and nothing is back-dated: each is offered as an ordinary pending act
that repeats what the record says.

```bash
war sign --list                  # "signature only, recorded satisfied" marks these
war sign --all --ssh-sign        # signs every one it can, one agent dialog each
```

What the act may not do is decide anything new. The resolution's outcome, the
correction's reason and kind, and the revision number come from the record, so
the signature can only say what was already said. `war check` reports
`authorize.signature-supplied`, `resolution.signature-supplied`,
`correction.signature-supplied` or `sas.signature-supplied`, the record file is
left byte-for-byte alone, and the journal gains a `*.signature_recorded` event
rather than a second `*.recorded` — the act was recorded then and signed now.

`--all` signs what is signable and stops for nothing else. An act that needs an
answer the batch cannot give — which §38.6 outcome, which correction kind, which
§101.3 ADR — is reported as `sign.needs-decision` with the command that asks for
it, and the sweep carries on:

```bash
war sign OW-WAR-0007 --ssh-sign --outcome not_satisfied
war sign OW-WAR-0005/D-001 --ssh-sign --kind behaviour-change --meaning "why this file moved"
war sign 1.0.0 --ssh-sign --adr OW-ADR-0016
```

Run `war sign --all --ssh-sign` again afterwards: it is idempotent, and when it
reports only `sign.needs-decision` rows there is nothing left it can sign.
`war console` is the same queue as a checklist, with presets for the reasons.

## Step 4 — check what actually happened

```bash
war resolve --dry-run OW-WAR-0030
```

Read the last line before the verdict. It reports §38.6 separately from the
thirteen, and the distinction is the one that matters:

> §38.6: OW-WAR-0030 would resolve NOT SATISFIED even once the §56.1
> requirements are met. 2 obligation(s) are on record as not established or
> refuted.

Requirement 4 asks whether every obligation was *dispositioned*. `not_established`
is a disposition, so it satisfies requirement 4. §38.6 asks what the answers
**were**. A Warrant can meet all thirteen requirements and still close
unsatisfied, and it should — 47 of the 50 here currently would.

That is not a reason to withhold authorization. It is a reason to know what you
are closing.

## The three cases authorization will not fix

**Twelve Warrants cite no gate.** OW-WAR-0001 through 0018 were authored before
the Gate Registry existed. Requirement 5 asks whether every required gate has an
admissible result, and a Warrant that names no gate has produced no mechanical
proof of anything. Adding gate bullets to their assurance atoms now would move
the contract digest to make a tool go green — it needs an amendment you
authorize, not a quiet edit.

**Nine Warrants have no deliverables record**, because two of them delivered
nothing (OW-WAR-0032's schema pack — there is no `schemas/` directory; OW-WAR-0040's
Liminal adapter) and seven discharge phase exits or deliver behaviour other
Warrants already claim. Pointing them at a file another Warrant delivered would
double-count one artifact as two deliveries.

**Three Warrants are blocked outside this repository.** OW-WAR-0026 needs a Katana
checkout, OW-WAR-0040 needs Liminal. Both record a §36.3 blocking unknown with the
resolution requirement stated. A blocking unknown is not a risk you can weigh and
accept — there is nothing to decide until the missing thing exists.

## Verification is already done

All 176 obligations across the corpus have been put to an independent verifier
and carry a recorded verdict: 50 established, 124 not established, 2 refuted. The
verifier holds eight of §46.1's nine independence dimensions — everything except
`distinct_human_required`, which is `false` and recorded as `false`. That clears
§46.3's minimum for `basic` and `controlled`, and does not clear it for `high`.

If you want a Warrant's obligations re-examined after changing the artifacts:

```bash
war verify <alias> --performer claude > request.toml
# hand request.toml to something that did not write the code
war verify <alias> --response verdicts.toml
```

## Handing the verification to someone who is not you

`war verify <alias> --performer <you> --bundle` writes
`verifications/bundle-<digest>.json` (`oh.war/verification-bundle/v2`): the
request with the authorized contract digest, every atom, each deliverable's
bytes (whole under `[verify] max_excerpt_bytes`, else the head with the full
digest and `truncated: true`), the plants that name the alias, the `#[test]`
names in Rust deliverables, the committed gate runs, the prior verifications,
and its own token estimate. A Warrant whose bundle exceeds `[verify]
max_bundle_tokens` gets one bundle per obligation instead, each carrying only
what that obligation lists or names, excerpted to fit with whole-file
digests, and saying what it did not carry. Hand that one file to a separate context — another
session, another model, a person — and ingest what comes back with
`war verify <alias> --response <file>`.

With `[verify] verifier_argv = ["…"]` in `openwarrant.toml`, `war verify
<alias> --run` does the hand-off itself: the command gets the bundle path,
runs under `verifier_timeout_secs`, and what it prints on stdout goes through
the same ingest as a hand-written response — a verifier that answers as the
performer is refused there, exactly as a human typing it would be.

## Signing a batch of corrections

A correction needs a kind, which is a judgement, and a reason, which the record
already holds. So the kind is the only thing to type:

```bash
war sign --list                                  # the queue
war sign --all --ssh-sign --kind behaviour-change # one dialog each, no typing
```

The reason is drafted from the commits that touched the file since that
Warrant resolved, and printed above the prompt before anything is signed.
`--meaning "..."` adds your sentence to every reason in the batch when the
record does not say enough. One dialog per signature is the ssh agent's doing
(`ssh-add -c`), and that is the control, not the friction: a signature nobody
confirmed is the thing the whole seam exists to prevent.
