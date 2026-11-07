# War skills

Say `war` in an OpenWarrant request: `war progress`, `war grill this`,
`war spec this`, `war tickets`, `war migrate`, `war review`, or `war start OW-WAR-0075`.
The [war router](../.claude/skills/war/SKILL.md) selects only the needed method.
Bare `war` shows progress. These are chat cues; only documented CLI commands
are shell commands. Explicit skill invocation is `/war` in supporting Claude
harnesses or `$war` in supporting Codex harnesses. Automatic discovery depends on
installation and the harness; a rare keyword alone is not a trigger guarantee.

## Current command

```sh
war overview                 # unresolved legacy records, titles, states, gaps
war progress --json          # same view, one machine report
war overview --all            # include recorded resolutions
```

The view is built from live records, not committed status files. It does not award
Verified status or claim that every unresolved record is unfinished code. Use
`war status <alias> --json` for details, `war next --json` for pending human acts,
`war questions --open --json` for questions and `war frontier --json` for legacy
stages. For a source checkout, use `target/debug/war` after building.

## Methods

| Request | Artifact |
| --- | --- |
| war grill | Durable questions/answers and unresolved decisions |
| war spec | Bounded Warrant/proposal with exact context and evidence plan |
| war tickets | Stages with real input dependencies and declared work stops |
| war map | Decision Warrant/context with options and resolution criteria |
| war migrate | Existing system docs or older OpenWarrant records converted to the selected edition, with retained originals, source mappings and validation report |
| war review | Separate standards and independent obligation findings |
| war start / continue | Bounded implementation, tests, notes and generated status link |

Prompt-only work can finish unverified. Explicit action gates remain binding.
Common Verified mark requires independent evidence and secure human acceptance.
Current legacy signing, resolution and performer commands keep their own enforcement;
skills do not pretend those commands already support new SDK completion records.

`war migrate` is the chat entry for the [migration skill](../.claude/skills/war-migrate/SKILL.md).
It preserves original nonconforming specs, decisions, plans and work records in
an archive, then creates new conforming OpenWarrant documents from their content.
New documents form the working set and link to their exact original sources.
It resolves the target edition, validates new outputs and reports incomplete work.
An archive or compatibility wrapper alone is not completed conversion. Existing context files
retain their native roles. In a shell, today's `war migrate` is the narrower
legacy ADR importer; that command alone does not complete system migration.

## Installation and checks

Claude plugin discovers `.claude/skills/`. Codex project discovery uses the
`.agents/skills/` links to the same files. Repository AGENTS.md also points at
`war` when the user's prompt contains that engineering intent. Existing global
skill installations are not overwritten; start a fresh harness session if its
skill inventory was cached before installation.

The suite adapts Matt Pocock methods at a fixed revision, with a distributed
[provenance table and MIT notice](../.claude/skills/openwarrant/ADAPTATIONS.md).
The full earlier command walkthrough is [legacy skills](../archive/legacy-20260914/original/docs/SKILLS.md).
[Audit](design/war-skills-audit.md) distinguishes structural checks, executed CLI
cases and model/harness evaluations that have not run.

<!-- planted -->

<!-- planted -->

<!-- planted -->

<!-- planted -->
