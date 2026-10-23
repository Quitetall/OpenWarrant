# Adoption study

Does OpenWarrant make better agent context than a plain `CLAUDE.md`, plan
files, Spec Kit, Beads or Jira? `PROTOCOL.md` is the design and the measures, fixed
before any run; `BACKLOG-FORMAT.md` is the format of the study's backlogs.

| file | |
|---|---|
| `PROTOCOL.md` | arms, repositories, scenarios, measures, analysis, threats, cost |
| `run.py` | the harness: prepare, validate, seed, parity, plan, execute, report |
| `arms.py` | the six arms: the same backlog in each tool's own form, and a reader per tool |
| `jira_arm.py` | the Jira arm over the REST API: seed, read, clean up, by a label per cell |
| `fixture_agent.py` | scripted agents for zero-cost dry runs (`reference`, `noop`, `claim`) |
| `repos/tally/`, `repos/knightlm/` | backlog, source, hidden tests, reference solutions, `validate.sh` |
| `repos/_selftest/` | a two-item repository for testing the harness itself |

```bash
python3 run.py prepare                 # pinned tools, offline crates, pytest for the sandbox
python3 run.py validate tally          # every check fails on the seed, passes with its solution
python3 run.py parity tally            # every arm holds every item's words
python3 run.py seed tally openwarrant /mnt/4tb/tmp/look   # see what an agent would see
python3 run.py plan --pilot --out pilot.json
python3 run.py execute pilot.json --run-id dry --agent fixture-reference   # free
ADOPTION_AUTH=token secrets run -- python3 run.py execute pilot.json \
    --run-id pilot-1 --agent claude --budget-usd 60                        # costs tokens
python3 run.py report pilot-1
```

Auth: `ADOPTION_AUTH=token` passes `CLAUDE_CODE_OAUTH_TOKEN` (the subscription);
`ADOPTION_AUTH=api` passes `ANTHROPIC_API_KEY`, plus `ADOPTION_WORKSPACE_ID` as the
`anthropic-workspace-id` header when the key is not scoped to a workspace. An
API error stops the run instead of being scored against an arm.

Agents run in a bubblewrap sandbox; see "Isolation and safety" in the protocol.
Raw transcripts go to `/mnt/4tb/artifacts/OpenWarrant/keep/adoption/<run-id>/`.
