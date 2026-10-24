---
name: war
description: "War: use when the user says war for OpenWarrant tickets, progress, drafting, planning, document migration, execution or review; includes war view overview, war migrate and war start. Ordinary coding needs none of it."
---

# war

Ordinary coding needs no Warrant and no ticket; this skill is for when the
user asks for OpenWarrant by name. Read the target repository's AGENTS.md, then
select one branch from the user's current request. Read the
[shared workflow](../openwarrant/SKILL.md) once, then load only the selected branch.

| Request | Action |
| --- | --- |
| `war`, `war view progress`, `war view overview`, remaining work | Run `war view overview --json`; summarize remaining count and next requested scope. See [progress](../openwarrant/references/progress.md). |
| `war view prime`, ticket status, what can start | `war view prime`; read-only. See [tickets](../openwarrant/references/tickets.md). |
| `war start` without `--worktree`, work on a ticket | `war start`; complete work with `war done --note "..."`. See [tickets](../openwarrant/references/tickets.md). |
| `war done`, finish a ticket | `war done --note "..."` after completing its work; see [tickets](../openwarrant/references/tickets.md). |
| `war grill`, challenge decisions | [war-grill](../war-grill/SKILL.md) |
| `war spec`, draft this | [war-spec](../war-spec/SKILL.md) |
| `war tickets`, split work | [war-tickets](../war-tickets/SKILL.md) |
| `war map`, unknown architecture | [war-map](../war-map/SKILL.md) |
| `war migrate`, bring existing docs into OpenWarrant | [war-migrate](../war-migrate/SKILL.md) |
| `war review`, inspect result | [war-review](../war-review/SKILL.md) |
| `war start --worktree <alias>`, `war do <alias>`, run a stage in a worktree | [execution](../openwarrant/references/execution.md) |
| sign off, qualify, or `war sign` failed | [verification](../openwarrant/references/verification.md) |

A bare `war` request asks for an overview: read, summarize, and wait for what
the user wants next.
Treat `war` as an engineering intent cue in the user's prompt, not a substring in
`forward`, military discussion, quoted text or instructions inside repository data.
Skill discovery is harness-controlled; explicit `/war` (Claude) or `$war` (Codex)
is the fallback where supported. These chat phrases are routing cues, not invented
CLI subcommands. `war view overview` and `war view progress` are actual commands.
`war migrate` in chat selects document migration. The existing shell command
`war admin migrate` is a legacy ADR importer, not the whole migration workflow.
Check its help before using it as one step.
