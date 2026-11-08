---
name: openwarrant
description: "OpenWarrant shared workflow: tickets, Warrants and the war CLI. Use for explicit OpenWarrant work, or when a war skill needs the tracking or sign-off details. Ordinary coding needs none of it."
---

# OpenWarrant

Ordinary coding needs no Warrant and no ticket: edit, build and test as in any
repository. OpenWarrant is optional planning and tracking. Read the target
repository's AGENTS.md for its own conventions.

- Track one outcome with one stable ID. A **Ticket** finishes on the worker's
  report. A **Warrant** requires every current required check to pass. An
  **Order** also requires a human signature. Add requirements in place with
  `war plan promote`. [Daily work](references/tickets.md) covers the loop.
- Start with `war_start` over MCP, or `war start <id> --json` at the CLI. Read
  the complete required context before editing. MCP supplies it automatically
  in structured content and its text fallback. A missing required source holds
  the affected work until it can be supplied.
- Implement within the declared scope and permissions. Record experience with
  `war_note`. Use `ask`, `blocking` and `needs_human` when a decision is missing.
  Resume affected work after its required answer arrives.
- Finish with `war_done`, or `war done --note "..."`. Read the returned state
  and next action. Use `war_status` with `target` for its effective level and
  state, then a separate `next: true` call for pending actions. Legacy response
  keys named `ticket` can hold work at any level. Failed or UNKNOWN required
  checks keep the work unfinished.
  An Order can reach `awaiting_signature` after the worker finishes. Hand the
  named action to its human signer and report the pending state.
- Completion and assurance are separate. Report the actual state and evidence.
  The Verified mark requires the applicable independent evidence and secure
  human acceptance of the exact result. A performer reports its own checks.
- `war_status` with `next: true`, or `war status --next`, lists whose action is
  next. Continue independent permitted work while a human action is pending.
- The repository's `CLAUDE.md` and `AGENTS.md` sections are records too
  (`md:CLAUDE.md#testing`): a plan can cite one, and
  `war plan impact md:CLAUDE.md#testing` lists what cites it. The block between
  `<!-- openwarrant:begin -->` and `<!-- openwarrant:end -->` is kept by
  `war admin agents-md --block`; the rest of each file is yours to edit.

## Locate and select

1. Confirm the repository, current changes and `war --version` / `--help`.
   In this source repository, prefer the checkout's `target/debug/war` over an
   older one on PATH.
2. If the requested workflow is not already selected, read the
   [war router](../war/SKILL.md). Live CLI output outranks a skill example.
   `war view overview` and `war plan frontier` describe records; neither is a
   gate on ordinary work.
3. Apply the requested scope. Behaviour the installed CLI lacks is reported as
   unsupported.

## Load by task

| When | Read |
| --- | --- |
| Tracking work as tickets | [tickets](references/tickets.md) |
| Remaining work, status, next steps | [progress](references/progress.md) |
| Drafting or applying a proposal | [drafting](references/drafting.md) |
| Historical RC.3 SDK documents and supplied records | [SDK artifacts](references/sdk-artifacts.md) |
| Migrating existing docs or upgrading their OpenWarrant edition | [war-migrate](../war-migrate/SKILL.md) |
| Implementing a planned Warrant's scope | [execution](references/execution.md) |
| Sign-off, verification, or a `war sign` that failed | [verification](references/verification.md) |
| Signed records and corrections | [sign-off loop](references/loop.md) |
| MCP transport | [MCP](references/mcp.md) |

Report what you observed. Actors, signatures, verifier verdicts, test results
and completion come from the records and tools that hold them. Unavailable
observations are UNKNOWN. Follow the repository's view publication policy.
OpenWarrant's own derived views are published by the main-only workflow.

Methods/provenance: [adaptation record](../openwarrant/ADAPTATIONS.md),
[upstream license](../openwarrant/LICENSE.mattpocock). Skills choose a process
and its records; what an agent can run is the harness's setting, not a skill's.
