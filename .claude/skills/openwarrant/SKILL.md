---
name: openwarrant
description: Author, check, draft, compile, and verify Work Authorization Records (Warrants) with the `war` CLI or its MCP server. Use whenever a repository contains an `openwarrant.toml` or `docs/warrants/`, or the user mentions Warrants, WARs, OpenWarrant, `war check`, or asks to plan, record, or close a unit of authorized work. Also use before claiming a Warrant is complete: the rules on self-verification are load-bearing and easy to violate by accident.
---

# OpenWarrant

A Warrant is one unit of authorized work: source atoms compiled into
projections, authorized and resolved by a human, closed only on evidence.

**Read [`AGENTS.md`](../../../AGENTS.md) in the repository root first.** It is
the single source of the rules; this file gets you there and makes the five
prohibitions unmissable. If the two disagree, `AGENTS.md` wins.

## Detect

```bash
ls openwarrant.toml docs/warrants/ 2>/dev/null && war --version
war next            # whose act is next, agent or human: read this before anything
```

## The five prohibitions (enforced by the tool; details in AGENTS.md)

1. **Never verify your own work.** `war verify` refuses verifier == performer.
2. **Never write a disposition you did not receive** via `war verify --response`.
3. **`UNKNOWN` is neither failure nor pass.** It blocks.
4. **Never edit anything under `generated/`**, nor a file `war pins --resolved-only` lists.
5. **Never change a document to make a tool go green.**

Four acts are a human's only: authorize, resolve, accept a SAS revision,
correct a resolved Warrant's file. You emit the request; a human signs with
`war sign … --ssh-sign`. The tool refuses your signature by kind (§27.2).

## The loop

```bash
war next && war new "What this accomplishes"
# edit the atoms
war check <alias> && war compile && war check --generated
war authorize <alias>                 # request → STOP: human signs
war pins --resolved-only              # then deliver; declare in deliverables.toml
war evidence record <alias>
war verify <alias> --performer <you>  # request for an INDEPENDENT verifier
war verify <alias> --response <file>
war resolve --dry-run <alias> && war resolve <alias>   # request → STOP: human signs
```

Every command takes `--json` (one `oh.war/report/v1` envelope).

## References

| when | read |
|---|---|
| the full loop with every stop, corrections, `--json` | [references/loop.md](references/loop.md) |
| turning a vague sentence into a Warrant (`war plan`, v2 proposals, a configured drafter) | [references/drafting.md](references/drafting.md) |
| working through the MCP server: tools, refusals, resources | [references/mcp.md](references/mcp.md) |
| obligations, independence, receipts, the thirteen requirements | [references/verification.md](references/verification.md) |

## Sibling skills (OW-WAR-0068, after mattpocock/skills)

| you want | skill |
|---|---|
| a vague request settled before drafting | `/war-grill` (answers land in the draft request) |
| the conversation turned into a proposal, no interview | `/war-spec` |
| a Warrant broken into stages with blocking edges; what can start now | `/war-tickets`, `war frontier` |
| a two-axis review, Standards beside Obligations, verifier kept blind | `/war-review` |
| a chunk too big for one session, planned as decisions | `/war-map` |

`CONTEXT.md` at the root is the glossary every Dispatch carries; use its words.

## When stuck

Report what you established, what you did not, and stop. Two of thirteen
requirements honestly met beats thirteen claimed and eleven wrong.
