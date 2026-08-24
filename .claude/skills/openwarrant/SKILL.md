---
name: openwarrant
description: Author, check, compile, and verify Work Authorization Records (Warrants) with the `war` CLI. Use whenever a repository contains an `openwarrant.toml`, a `docs/warrants/` directory, or the user mentions Warrants, WARs, OpenWarrant, `war check`, or asks to record/close a unit of authorized work. Also use before claiming a Warrant is complete — the rules on self-verification are load-bearing and easy to violate by accident.
---

# OpenWarrant

A Warrant is one unit of authorized work, compiled from source atoms into
projections, and closed only on evidence.

**Read [`AGENTS.md`](../../../AGENTS.md) in the repository root first.** It is the
single source for the rules below; this file exists to get you there and to make
the five prohibitions unmissable. If the two ever disagree, `AGENTS.md` wins.

## Detect

You are in an OpenWarrant repository if any of these exist:

```bash
ls openwarrant.toml docs/warrants/ 2>/dev/null
war --version
```

## The five prohibitions

These are enforced by the tool. Attempting to work around any of them
manufactures a false completion, which is the one failure this system exists to
prevent.

1. **Never verify your own work.** You are the performer. `war verify` refuses a
   verdict where verifier == performer and does not write the file. Changing the
   `performer` field to get past it is falsifying a record.
2. **Never write a disposition you did not receive** from an independent
   verifier via `war verify --response`.
3. **`UNKNOWN` is neither failure nor pass.** A check that could not run says so.
4. **Never edit anything under `generated/`.** Edit atoms, then `war compile`.
5. **Never change a document to make a tool go green.** Establish which is wrong
   first.

`war resolve` requires `--dry-run` and cannot close a Warrant. A human does that.

## The loop

```bash
war new "What this accomplishes"      # scaffolds the Warrant
# edit the atoms — the real work is described here
war check <alias>                     # deterministic, agent-free
war compile && war check --generated  # projections, then drift
war verify <alias> --performer <you>  # emit a request for an INDEPENDENT verifier
war verify <alias> --response <file>  # ingest verdicts
war resolve --dry-run <alias>         # what still blocks closure
```

## Writing obligations

Obligations in `60-assurance.md` are the unit of completion. Each needs a
**bounded scope** — §38.4: a claim is bounded by its evidence.

```markdown
### OBL-001 — the parser refuses a duplicate ordinal

- **scope:** manifests exercised by the fixtures in `conformance/`.
- **evidence:** a planted duplicate, and the specific error it produces.
```

"The parser works" is not an obligation. And pair every passes-claim with a
refusal obligation: a claim that something succeeds is satisfiable by code that
always succeeds.

## When stuck

Report what you established, what you did not, and stop. Two of thirteen
requirements honestly met beats thirteen claimed and eleven wrong.
