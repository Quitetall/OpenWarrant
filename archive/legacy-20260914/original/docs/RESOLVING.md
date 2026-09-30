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

Nothing in this repository can write `docs/authority/roles.toml`, and there is no
`war authority grant`. A command that could write it would let an agent assign
itself the roles §27.2 exists to withhold, through the same tool that later
checks whether the assignment is valid. Authority has to enter from outside.

```bash
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

A receipt counts only while it reseals and is bound to the contract as it
compiles now; edit the contract and `war check` reports `evidence.stale-binding`
until a new run is recorded. A Warrant whose assurance atom cites no gate cannot
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
`verifications/bundle-<digest>.json` (`oh.war/verification-bundle/v1`): the
request with the authorized contract digest, every atom, each deliverable's
bytes (whole under `[verify] max_excerpt_bytes`, else the head with the full
digest and `truncated: true`), the plants that name the alias, the `#[test]`
names in Rust deliverables, the committed gate runs, the prior verifications,
and its own token estimate. Hand that one file to a separate context — another
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
