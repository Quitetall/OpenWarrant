# Working in an OpenWarrant repository

Instructions for an AI agent operating in a repository that uses OpenWarrant.
Read this before creating, editing, checking, or closing a Warrant.

This file is the single source for these rules. Editor-specific skills should
point here rather than restate them, so there is one place to correct.

---

## What you are and are not permitted to do

You are a **performer**. You may draft, execute, report, and review.

You may **not** authorize, verify your own work, or resolve. That is not a
policy preference — the tool enforces it, and working around it produces exactly
the false completion the system exists to prevent.

Five rules. Breaking any of them is worse than doing nothing.

### 1. Never verify your own work

§51.2 forbids self-completion; RQ-053 forbids a performer's report from
satisfying an independent gate. If you wrote it, you cannot clear it.

`war verify` will refuse a verdict whose verifier equals the performer, and it
will **not write the file**. Do not try to satisfy it by changing the `performer`
field — that is falsifying a record, not passing a check.

### 2. Never write a disposition you did not receive

An obligation's disposition comes back from an independent verifier through
`war verify --response`. Hand-writing `disposition: established` into an
assurance atom is the substitution §40.7 forbids: a judgment standing in for the
observation it should rest on.

### 3. Unknown is not failure, and it is not pass

Law 15. A check that could not run reports `UNKNOWN`. Degrading it to `ERROR`
makes a sound Warrant look broken; degrading it to `PASS` makes an unasked
question look answered. Both are lies with different shapes.

If you cannot establish something, say so and stop. "Probably fine" is not a
result.

### 4. Never edit a generated file

Files under `generated/` are projections. Edit the **atoms** and recompile.
`war check --generated` will catch a hand-edit, and the correct response is to
revert your edit, not to regenerate over it.

The same applies to a Warrant that has been authorized: an authorized contract
revision is immutable (§28.7). Amend by creating a new revision.

And to a **resolved** Warrant's delivered files: `deliverables.toml` pins their
bytes, and `war check` refuses drift. You may **request** a correction
(`war correct <alias> <D-id>` emits what a human would sign); you may not sign
one, and you may never regenerate a resolved Warrant's `deliverables.toml` —
that stales the resolution. OW-WAR-0064 / OW-ADR-0012.

### 5. Never change a document to make a tool happy

If a checker and a document disagree, establish which is wrong **before**
changing either. Editing a correct record so a linter goes green falsifies the
record — and a green checker that is wrong is worse than a red one that is right.

If the tool is wrong, fix the tool and say so.

---

## The loop

```bash
war next                                   # whose act is next, and the command — read this first
war new "What this work accomplishes"      # creates docs/warrants/OW-WAR-NNNN/
# edit the atoms — this is where the real work is described
war check <alias>                          # deterministic, no agent, no network
war compile && war check --generated       # projections written; no drift
war authorize <alias>                      # the REQUEST: what a human would sign
#   ── stop. A human runs `war sign <alias>` (terminal) or `--ssh-sign` (dialog). ──
war pins --resolved-only                   # before editing anything: what a resolution pins
# deliver; declare it in deliverables.toml
war evidence record <alias>                # run the cited gates; mint §44.6 receipts
war verify <alias> --performer <you>       # the request for an INDEPENDENT verifier
# hand the request to something that is not you — a separate context, never your own
war verify <alias> --response <file>       # ingest the verdicts
war resolve --dry-run <alias>              # what still blocks closure; §38.6 beside the thirteen
war resolve <alias>                        # the REQUEST
#   ── stop. A human runs `war sign <alias>`. ──
```

Every command takes `--json` and answers with one `oh.war/report/v1` envelope.
`war next --json` names every pending act with its actor; no action it hands an
agent is a signature.

Four acts are a human's and only a human's — authorize, resolve, accept a SAS
revision, correct a resolved Warrant's delivered file. You emit the request; the
tool refuses your signature by kind (§27.2), whatever the response file says.

### Drafting — both paths reach the same gauntlet

A vague sentence becomes a reviewable draft (§74) without you writing files
under `docs/warrants/` by hand:

```bash
war plan "add a changelog"                       # the REQUEST: corpus, ADRs, questions (oh.war/draft-request/v1)
# you are the drafter: answer it with an oh.war/draft-proposal/v2 file — operations carry
# role, ordinal, path, body; relations; evidence claims; durable choices; blocker questions
war plan --proposal draft.json --reviewed        # §74.4's gauntlet, nothing applied
war plan --proposal draft.json --reviewed --apply   # creates the Warrant through `war new`; records plan/
war plan "add a changelog" --draft --reviewed --apply   # or: the configured [plan] drafter_argv answers
```

`--apply` refuses a proposal nobody reviewed, a v1 proposal (no payloads), an
unanswered blocker question, an invented `war://`, and a drafter that touched
the working tree. Answer questions with `--answer Q-001="..."`.

### Over MCP

`war mcp` serves the same surface to any harness over stdio: every read, every
request half, and the writes an agent may make (`war_new`, `war_evidence_record`,
`war_compile`, `war_gate_run`, `war_journal_backfill`, a reviewed
`war_plan_apply`). Each tool answers with the `oh.war/report/v1` envelope. It
registers **no** signing, ingesting, `sas propose`, `kf`, `telemetry`,
`migrate`, `export`, `bonsai`, `init`, `gate --record` or `plan --draft` tool —
`war mcp --describe` prints the table and the refusal list. Resources:
`warrant://<alias>[/status|/journal]`, `status://corpus`, `sas://current`,
`pins://all`, `next://`.

Claude Code: the repository is also a plugin (`.claude-plugin/`). It ships the
`openwarrant` skill, this server (`.mcp.json`), a `PreToolUse` guard that
denies an edit to a file `war pins --resolved-only` lists or to anything under
`generated/`, and a `Stop` check that blocks ending the turn while `war check`
reports errors. `claude plugin marketplace add <path-to-repo>` then
`/plugin install openwarrant@openwarrant`.

### Writing the atoms

A `delivery` Warrant has five authored atoms. What each is for:

| atom | what belongs in it |
|---|---|
| `10-intent.md` | the problem, the desired outcome, and what is explicitly **out** of scope |
| `20-basis.md` | governing sources, prerequisites, and unknowns — including blocking ones |
| `40-work-order.md` | deliverables, frozen surfaces, autonomy limits, rollback |
| `45-milestones.yaml` | acceptance checkpoints and dispatchable stages |
| `60-assurance.md` | acceptance obligations, each with a **bounded scope** |

Obligations are the unit of completion. Each needs an id, a statement, a scope,
and the evidence that would settle it:

```markdown
### OBL-001 — the parser refuses a duplicate ordinal

- **scope:** manifests exercised by the fixtures in `conformance/`. No claim
  about manifests using fields none of them use.
- **evidence:** a planted duplicate, and the specific error it produces.
```

**State the bound.** §38.4: a claim is bounded by its evidence. "The parser
works" is not an obligation; "the parser refuses a duplicate ordinal, over these
fixtures" is.

**Pair every claim with a refusal.** An obligation asserting something passes is
satisfiable by code that always returns success. Add the obligation that the
control has been observed to *reject*, or you have tested nothing.

---

## Things that look like progress and are not

- **Marking your own obligations established.** Zero verified obligations is a
  true state. Fabricated dispositions are a false one, and much harder to undo.
- **Making `war check` green by narrowing what it checks.** If a check is
  inconvenient, it is usually load-bearing.
- **Deleting a failing plant.** The plant exists because the control needs to be
  seen refusing something.
- **Writing an obligation you already know you can satisfy.** Assurance is not a
  formality to be routed around.

## When you are stuck

Say what you established, what you did not, and stop. A Warrant that honestly
reports two of thirteen requirements met is more useful than one that claims
thirteen and is wrong about eleven.

---

## Reference

- `docs/sas/generated/NORMATIVE.md` — every SHALL, SHALL NOT, SHOULD and MAY
  of the governing specification with its section, compiled and drift-checked.
  Read this, not the whole document: it is about a third of the tokens, and
  the sentence is what binds. Section references throughout the tool
  (`§46.2`, `RQ-053`) resolve into it; `docs/sas/` holds the document itself.
- `war <command> --help` — every command documents the section it implements.
- `CONTRIBUTING.md` — the gate, the toolchain pin, and the rules for changing
  this repository itself.
- `docs/THREAT_MODEL.md` — what the signing path defends, what it leaves to
  the operator, and which test or plant exercises each control.
- `QUICKSTART.md` — an empty directory to a resolved Warrant, every step one
  command, the two human steps marked; `docs/EXAMPLES/` walks real records.

## Skills over the core (OW-WAR-0068)

Adapted from mattpocock/skills (MIT), each ending in a record the tool reads:
`/war-grill` (answers land in the draft request), `/war-spec` (conversation to
a v2 proposal), `/war-tickets` and `war frontier` (stages with blocking edges;
what can start now), `/war-review` (Standards beside the blind verifier's
Obligations), `/war-map` (a decision Warrant whose fog is blocking unknowns).
`CONTEXT.md` at the root is the glossary every Dispatch carries: use its
words. A skill never signs, and never claims a step it did not run.
