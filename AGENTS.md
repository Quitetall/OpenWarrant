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

### 5. Never change a document to make a tool happy

If a checker and a document disagree, establish which is wrong **before**
changing either. Editing a correct record so a linter goes green falsifies the
record — and a green checker that is wrong is worse than a red one that is right.

If the tool is wrong, fix the tool and say so.

---

## The loop

```bash
war new "What this work accomplishes"     # creates docs/warrants/<NS>-WAR-NNNN/
# edit the atoms — this is where the real work is described
war check <alias>                          # deterministic, no agent, no network
war compile                                # write the projections
war check --generated                      # confirm no drift
war verify <alias> --performer <you>       # emits a request for an INDEPENDENT verifier
# hand the request to something that is not you
war verify <alias> --response <file>       # ingest the verdicts
war resolve --dry-run <alias>              # see what still blocks closure
```

`war resolve` **requires** `--dry-run` and cannot record a resolution. Recording
one needs an authorizer, an acting role, and a stated meaning, and no agent may
invent those. A human closes the Warrant.

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

- `docs/sas/` — the governing specification, if the repository carries one.
  Section references throughout the tool (`§46.2`, `RQ-053`) cite it.
- `war <command> --help` — every command documents the section it implements.
- `CONTRIBUTING.md` — the gate, the toolchain pin, and the rules for changing
  this repository itself.
