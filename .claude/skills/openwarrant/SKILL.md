---
name: openwarrant
description: "OpenWarrant shared workflow: tickets, Warrants and the war CLI. Use for explicit OpenWarrant work, or when a war skill needs the tracking or sign-off details. Ordinary coding needs none of it."
---

# OpenWarrant

Ordinary coding needs no Warrant and no ticket: edit, build and test as in any
repository. OpenWarrant is optional planning and tracking. Read the target
repository's AGENTS.md for its own conventions.

- A **ticket** tracks a piece of work as a checklist: `war prime`, `war ready`,
  `war claim <item>`, do it, `war done <item> --note "..."`. No step needs a
  signature. [Tickets](references/tickets.md) has the loop.
- A **Warrant** is a work plan; optional. A Warrant whose type has a sign-off
  step adds a person's approval, and the tool holds those rules.
- Finished and verified are separate: report finished work as finished, and
  as unverified until an independent check says more.
- The repository's `CLAUDE.md` and `AGENTS.md` sections are records too
  (`md:CLAUDE.md#testing`): a plan can cite one, and
  `war impact md:CLAUDE.md#testing` lists what cites it. The block between
  `<!-- openwarrant:begin -->` and `<!-- openwarrant:end -->` is kept by
  `war agents-md --block`; the rest of each file is yours to edit.

## Locate and select

1. Confirm the repository, current changes and `war --version` / `--help`.
   In this source repository, prefer the checkout's `target/debug/war` over an
   older one on PATH.
2. For intent routing, read [war](../war/SKILL.md). Live CLI output outranks a
   skill example. `war overview` and `war frontier` describe records; neither
   is a gate on ordinary work.
3. Apply the requested scope. Behaviour the installed CLI lacks is reported as
   unsupported.

## Load by task

| When | Read |
| --- | --- |
| Tracking work as tickets | [tickets](references/tickets.md) |
| Remaining work, status, next steps | [progress](references/progress.md) |
| Drafting or applying a proposal | [drafting](references/drafting.md) |
| Authoring or checking RC.3 documents and supplied records | [SDK artifacts](references/sdk-artifacts.md) |
| Migrating existing docs or upgrading their OpenWarrant edition | [war-migrate](../war-migrate/SKILL.md) |
| Implementing a planned Warrant's scope | [execution](references/execution.md) |
| Sign-off, verification, or a `war sign` that failed | [verification](references/verification.md) |
| Signed records and corrections | [sign-off loop](references/loop.md) |
| MCP transport | [MCP](references/mcp.md) |

Report what you observed. Actors, signatures, verifier verdicts, test results
and completion come from the records and tools that hold them; what could not
be observed is UNKNOWN. Generated views are rebuilt with `war compile`. Other
work goes ahead while one scope waits.

Methods/provenance: [adaptation record](../openwarrant/ADAPTATIONS.md),
[upstream license](../openwarrant/LICENSE.mattpocock). Skills choose a process
and its records; what an agent can run is the harness's setting, not a skill's.
