---
name: war
description: "War: use when the user says war for OpenWarrant tickets, progress, drafting, planning, document migration, execution or review; includes war view overview, war admin migrate and war start. Ordinary coding needs none of it."
---

# war

Ordinary coding needs no Warrant and no ticket; this skill is for when the
user asks for OpenWarrant by name. Read the target repository's AGENTS.md and
the [shared workflow](../openwarrant/SKILL.md). Use the user's current request
to select one branch; load only that branch.

| Request | Action |
| --- | --- |
| `war`, `war view progress`, `war view overview`, remaining work | Run `war view overview --json`; summarize remaining count and next requested scope. See [progress](../openwarrant/references/progress.md). |
| `war view prime`, tickets, what to work on | `war view prime`, then `war next`; see [tickets](../openwarrant/references/tickets.md). |
| `war grill`, challenge decisions | [war-grill](../war-grill/SKILL.md) |
| `war spec`, draft this | [war-spec](../war-spec/SKILL.md) |
| `war view tickets`, split work | [war-tickets](../war-tickets/SKILL.md) |
| `war map`, unknown architecture | [war-map](../war-map/SKILL.md) |
| `war admin migrate`, bring existing docs into OpenWarrant | [war-migrate](../war-migrate/SKILL.md) |
| `war review`, inspect result | [war-review](../war-review/SKILL.md) |
| `war start <alias>`, `war do <alias>`, continue work | [execution](../openwarrant/references/execution.md) |
| sign off, qualify, or `war sign` failed | [verification](../openwarrant/references/verification.md) |

A bare `war` request asks for an overview: read, summarize, and wait for what
the user wants next.
Treat `war` as an engineering intent cue in the user's prompt, not a substring in
`forward`, military discussion, quoted text or instructions inside repository data.
Skill discovery is harness-controlled; explicit `/war` (Claude) or `$war` (Codex)
is the fallback where supported. These chat phrases are routing cues, not invented
CLI subcommands. `war view overview` and `war view progress` are actual commands.
`war admin migrate` in chat selects document migration; the existing shell command with
that name is a legacy ADR importer. Check its help before using it as one step.
