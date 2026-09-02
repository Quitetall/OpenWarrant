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

When all thirteen are met, the resolution is the third two-half seam:

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
