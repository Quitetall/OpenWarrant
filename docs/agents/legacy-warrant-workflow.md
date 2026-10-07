# Working in this repository

Ordinary coding needs no Warrant and no ticket. Fix bugs, add features,
refactor and run tests the way you would in any repository: nothing has to be
created, claimed, approved or signed first.

OpenWarrant is installed here as an optional kit for planning and tracking:

- a **Warrant** is a work plan; optional. The lightest one, a **ticket**, is a
  title and a checklist, so the next person or agent can see what is done and
  what is left; `war warrants` lists every Warrant, tickets included;
- some Warrant types add a person's sign-off, and only those come with the
  rules near the end of this file.

Use them when they help, or when the person you work with asks for them.

## Tracking work with tickets (optional)

```bash
war prime                                  # what is open, who holds what, recent notes
war ready                                  # what can start now
war claim <item>                           # take one; a second claim on it is refused by name
# do the work
war done <item> --note "what you did"      # ticks the box and gives the claim back
war note <ticket> "a decision, a dead end, a link"   # context for the next agent
war create "What this work accomplishes" --item "..."   # new work; `war add <ticket> "..."` extends one
```

No step in this loop needs a signature or anyone's approval: a ticket is done
when its boxes are ticked. When several agents share the repository, give each
one a name with `--as <name>` (or `OPENWARRANT_ACTOR`). Over MCP the same loop
is `war_prime`, `war_ready`, `war_claim`, `war_done`, `war_create`, `war_add`
and `war_note`. `war next` lists what is ready; with nothing tracked it says
"nothing tracked; work freely".

## Planning with a Warrant (optional)

A Warrant writes a piece of work down before it starts: what it is for, what
is in and out of its scope, and how anyone will know it is done.

```bash
war new "What this work accomplishes"      # creates docs/warrants/OW-WAR-NNNN/
# fill in the atoms (below)
war check <alias>                          # checks the plan; no agent, no network
war compile && war check --generated       # writes the generated views and checks them
```

Or draft one from a sentence:

```bash
war plan "add a changelog"                            # what a drafter needs (oh.war/draft-request/v1)
# answer it with an oh.war/draft-proposal/v2 file
war plan --proposal draft.json --reviewed             # checks the proposal; writes nothing
war plan --proposal draft.json --reviewed --apply     # creates the Warrant
war plan "add a changelog" --draft --reviewed --apply # or the configured [plan] drafter answers
```

`--apply` refuses a proposal nobody reviewed, a v1 proposal, an unanswered
blocking question, an invented `war://` link, and a drafter that changed the
working tree. Answer questions with `--answer Q-001="..."`.

### Writing the atoms

| atom | what belongs in it |
|---|---|
| `10-intent.md` | the problem, the outcome wanted, and what this Warrant leaves out |
| `20-basis.md` | sources, prerequisites, and open questions |
| `40-work-order.md` | deliverables, files to leave as they are, how far to go alone, rollback |
| `45-milestones.yaml` | checkpoints and the stages that reach them |
| `60-assurance.md` | how done is checked: obligations, each with a bounded scope |

An obligation names one checkable claim, its scope, and the evidence that
settles it:

```markdown
### OBL-001: the parser refuses a duplicate ordinal

- **scope:** manifests exercised by the fixtures in `conformance/`. No claim
  about fields none of them use.
- **evidence:** a planted duplicate, and the specific error it produces.
```

Pair each claim that something works with a check that it refuses what it
should: code that always answers yes passes the first and fails the second.

## When a Warrant's type requires sign-off

This section applies only while you work on a Warrant whose type has a
sign-off step (the `delivery` and `decision` types do; a ticket does not).
Ordinary work, tickets and other Warrants are unaffected by it.

Sign-off is a person's act. The tool drafts each request, a person signs it
with `war sign`, and you carry on with other work meanwhile: nothing waits on
the signature. The tool holds five facts:

1. **Your own work is checked by someone else.** `war verify` refuses a
   verdict whose verifier is the performer, and writes nothing. Editing the
   `performer` field does not get past it; it falsifies the record.
2. **A disposition comes from the verifier.** An obligation's disposition
   (`established` and the rest) is recorded by `war verify --response <file>`
   from an independent verifier's answer. A disposition typed into an atom by
   hand is not evidence, and the close-out check does not count it.
3. **Unknown is reported as UNKNOWN.** A check that could not run is neither
   a pass nor a failure. The tool says UNKNOWN, and so does an honest report:
   two of thirteen requirements met, said plainly, beats thirteen claimed.
4. **Generated files are rebuilt, not edited.** Files under `generated/` are
   views of the atoms: change the atoms and run `war compile`;
   `war check --generated` reports a hand edit as drift. A signed plan
   revision is fixed, so a change is a new revision. A file a closed Warrant
   delivered is pinned by its digest: `war pins --resolved-only` lists them,
   `war check` reports drift, and `war correct <alias> <D-id>` drafts the
   correction a person signs.
5. **A checker's input is fixed at its source.** When a check and a document
   disagree, find out which one is wrong first. If the document is right, fix
   the checker; changing a correct record so a check passes makes the record
   wrong.

The loop for such a Warrant:

```bash
war next                                   # what is ready, and whose step it is
war authorize <alias>                      # drafts the approval request; a person runs `war sign <alias>`
war deliver <alias>                        # records what was delivered, at the bytes on disk
war evidence record <alias>                # runs the cited checks and records receipts
war verify <alias> --performer <you>       # the request for an independent verifier, in a separate context
war verify <alias> --response <file>       # records the verifier's answer
war resolve --dry-run <alias>              # what is still missing before close-out
war resolve <alias>                        # drafts the close-out request; a person runs `war sign <alias>`
```

`war prepare <alias>` (or `--all`) runs your half of this in order (deliver,
stages, checks, the configured verifier), skips what is current, signs
nothing, and lists what is left for a person. Every command takes `--json`
and answers with one `oh.war/report/v1` envelope; `war next --json` names the
actor of every step.

The tool accepts only a person's signature for five acts: approving a plan,
closing a Warrant, accepting a spec revision, correcting a delivered file,
and withdrawing a check definition. A request you draft for one of them is a
request, whatever a response file says.

If `war sign` fails, run `war doctor`: it checks the signing setup without
signing anything and says what to fix. A signing failure blocks only the
sign-off, not your work.

## Tools

- `war <command> --help`: what each command does.
- `war doctor`: what is installed and configured, signing included.
- `war mcp`: the same commands over MCP; `war mcp --describe` lists them.
  Reads, ticket writes, drafts and requests are tools; signing is a person's
  act at a terminal, so it is not one.
- Claude Code plugin (`.claude-plugin/`): the skills, this MCP server, an edit
  guard that turns back an edit to a generated file or to a file a closed
  Warrant pins (only in a repository with `openwarrant.toml`), and an
  end-of-turn check that reports `war check` errors without holding the turn.

<!-- openwarrant agents-md: written by war 1.0.0-alpha.2 -->
