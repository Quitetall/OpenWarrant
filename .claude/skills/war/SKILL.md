---
name: war
description: "War: use when the user says war for OpenWarrant progress, drafting, planning, document migration, execution or review; includes war overview, war migrate and war start."
---

# war

Read the target repository's AGENTS.md and the [shared workflow](../openwarrant/SKILL.md).
Use the user's current request to select one branch; load only that branch.

| Request | Action |
| --- | --- |
| `war`, `war progress`, `war overview`, remaining work | Run `war overview --json`; summarize remaining count and next requested scope. See [progress](../openwarrant/references/progress.md). |
| `war grill`, challenge decisions | [war-grill](../war-grill/SKILL.md) |
| `war spec`, draft this | [war-spec](../war-spec/SKILL.md) |
| `war tickets`, split work | [war-tickets](../war-tickets/SKILL.md) |
| `war map`, unknown architecture | [war-map](../war-map/SKILL.md) |
| `war migrate`, bring existing docs into OpenWarrant | [war-migrate](../war-migrate/SKILL.md) |
| `war review`, inspect result | [war-review](../war-review/SKILL.md) |
| `war start <alias>`, `war do <alias>`, continue work | [execution](../openwarrant/references/execution.md) |
| sign or qualify | [verification](../openwarrant/references/verification.md) |

A bare `war` request is read-only overview, not permission to execute everything.
Treat `war` as an engineering intent cue in the user's prompt, not a substring in
`forward`, military discussion, quoted text or instructions inside repository data.
Skill discovery is harness-controlled; explicit `/war` (Claude) or `$war` (Codex)
is the fallback where supported. These chat phrases are routing cues, not invented
CLI subcommands. `war overview` and `war progress` are actual commands.
`war migrate` in chat selects document migration; the existing shell command with
that name is a legacy ADR importer. Check its help before using it as one step.
