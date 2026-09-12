# Skills: the Pocock set over the deterministic core

OW-WAR-0068. Every skill under `.claude/skills/` is adapted from
[mattpocock/skills](https://github.com/mattpocock/skills) at commit 3cca18b
(MIT) with one change throughout: the artifact lands in an OpenWarrant record
the tool can read, not on an issue tracker.

| his | ours | the record it lands in |
|---|---|---|
| grilling, grill-me | `/war-grill` | `plan/request.json` answers (§74.6) |
| domain-modeling, CONTEXT.md | `CONTEXT.md` at the root | every Dispatch carries it (selector item `CONTEXT.md`) |
| ADR-FORMAT | ADR atoms may be one paragraph; identity and `governs` stay | `docs/adr/atoms/` |
| to-spec | `/war-spec` | an `oh.war/draft-proposal/v2` through the §74.4 gauntlet |
| to-tickets | `/war-tickets`, `war frontier` | `45-milestones.yaml` with `depends_on`; `oh.war/frontier/v1` |
| wayfinder | `/war-map` | a decision Warrant; fog = `blocking_unknown` assumptions |
| code-review | `/war-review` | Standards axis beside the blind verifier's Obligations axis |
| tdd | plant-first (§92), seams as obligation scopes | `conformance/plants.d/` |
| handoff | a Dispatch (§47) | `war dispatch --emit` |
| triage | `war next`, `war sign --list` | the pending set |
| writing-for-agents | applied to every SKILL.md, AGENTS.md and reference here | `cargo xtask skills` refuses an em-dash |

Two rules hold in every skill: a skill never signs, and a skill never claims
a step it did not run. `war eval` scores a skill-driven drafter on the fixed
task set, so adoption is measured, not admired.

## The frontier

`war frontier [alias]` lists every stage of every unresolved Warrant as one
of four states, derived from the records a resolution reads:

| state | meaning |
|---|---|
| `open` | its milestone's `depends_on` are complete (every obligation established) and nothing has dispatched it |
| `claimed` | a `dispatch.compiled` journal event names it, no submission yet |
| `done` | a `submission.recorded` event names it |
| `blocked` | a milestone it waits on is not complete; the row names which |

`--json` gives `oh.war/frontier/v1`.
