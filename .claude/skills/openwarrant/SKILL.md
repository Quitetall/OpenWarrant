---
name: openwarrant
description: "OpenWarrant shared workflow and compatibility entry. Use for explicit OpenWarrant work or when a war skill needs execution and assurance rules."
---

# OpenWarrant

Read the target repository's AGENTS.md. Keep its directory scope and harness rules.
A Warrant describes one reviewable outcome. Completion and assurance are separate.
Prompt-only work can finish unverified; explicit action gates bind their named acts.
Human acceptance plus independent evidence is required for the common Verified mark.

## Locate and select

1. Confirm repository, current changes and available `war --version`/`--help`.
   In this source repo, prefer the checkout's built `target/debug/war` over stale PATH.
2. For intent routing, read [war](../war/SKILL.md). For records, read live CLI output;
   command availability outranks a skill example. Do not infer readiness from an old
   generated file or from `frontier` showing `open`.
3. Apply only the requested scope. New-model behavior not implemented by the installed
   CLI stays explicitly unsupported; never relabel a legacy signed act as prototype work.

## Load by task

| When | Read |
| --- | --- |
| Remaining work, status, next steps | [progress](references/progress.md) |
| Drafting or applying a proposal | [drafting](references/drafting.md) |
| Migrating existing docs or upgrading their OpenWarrant edition | [war-migrate](../war-migrate/SKILL.md) |
| Implementing or completing a bounded scope | [execution](references/execution.md) |
| Human qualification or legacy signing requests | [verification](references/verification.md) |
| Existing authorized/resolved legacy records or corrections | [legacy loop](references/loop.md) |
| MCP transport | [MCP](references/mcp.md) |

Keep exact required context and source revisions; retrieve background only when needed.
Never invent an actor, signature, independent disposition, test result or completion.
Unknown observations stay UNKNOWN. Preserve signed history and regenerate projections
through their tool. Independent work may continue when another scope is blocked.

Methods/provenance: [adaptation record](../openwarrant/ADAPTATIONS.md),
[upstream license](../openwarrant/LICENSE.mattpocock). Skills choose process and artifacts;
they do not acquire permissions or sandbox an agent.
