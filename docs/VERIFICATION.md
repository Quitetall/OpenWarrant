# Verification

A Warrant closes only on obligations an **independent** verifier has
settled (SAS §46, §56.1 requirement 10). The performer cannot be that
verifier: `war verify` refuses a verdict whose verifier is the performer,
and a performer's report cannot satisfy an independent gate (RQ-053).

This repository's verifier is `tools/verifier/claude-verifier.sh`
(OW-WAR-0117), configured in `openwarrant.toml`:

```toml
[verify]
verifier_argv = ["tools/verifier/claude-verifier.sh"]
```

## Running it

```bash
war evidence record <alias>                   # the cited gates, receipts minted
war verify <alias> --performer <who did the work> --run
war resolve <alias> --dry-run                 # what still blocks closure
```

`--run` compiles the bundle, hands its path to the wrapper, and ingests
what the wrapper prints through the same seam as `war verify --response`.
Set `CLAUDE_PERFORMER_MODEL` to the model that did the work (for this
repository's agent work, `claude-opus-5-5`); set `CLAUDE_VERIFIER_LOG` to a
directory to keep the raw answer and a run record.

## What the verifier sees

Only the bundle `war` compiled (`oh.war/verification-bundle/v1`): the
authorized atoms, each deliverable's bytes (excerpted past
`max_excerpt_bytes`, with the full digest), the plants that name the
Warrant, its gate runs and prior verifications. No transcript, no
rationale, no journal (§46.2).

## How it is kept blind and powerless

| control | how | checked by |
|---|---|---|
| no built-in tool | `--tools ""`, an allowlist of nothing; `--disallowedTools` naming Bash, Read, Write, Edit and the rest as well | `62-verifier.sh`, the recorded argv |
| no MCP server | `--strict-mcp-config` with no config | the same |
| no settings, hooks or project context | `--restricted`, and the process runs in an empty temporary directory, so no project `CLAUDE.md` and no project auto-memory is loaded | the same (argv and working directory) |
| no session kept | `--no-session-persistence` | the same |
| the model cannot vouch for itself | the verifier's name and every independence flag are printed by the script; anything the model says about who it is or what it may do is discarded | `62-verifier.sh`: flags identical with and without such claims |
| unknown is not pass | an unparseable answer, a skipped obligation, or a disposition other than `established` / `refuted` / `not_established` becomes `not_established` | `62-verifier.sh`: garbage, skip, `probably` |
| a failed run records nothing | a non-zero `claude` exit is the wrapper's exit; `war verify --run` reports `verify.verifier-failed` and ingests nothing | `62-verifier.sh` |

Observed on 2026-09-23 with Claude Code 2.1.280: under exactly these flags,
from an empty directory, the model reported no callable tool and no
`CLAUDE.md` or memory content. That is the model's own account, recorded
here as an observation; the plant pins the flags, which is what the claim
rests on.

## What its independence is, and is not

`openwarrant.toml`'s `[independence]` declares what this arrangement
provides, flag by flag with the reason beside each. `war check` reads it to
report §46.3's minimums: `basic` and `controlled` are met. Each ingested
verdict is still judged on its **own** flags (`verify.rs`,
`admissible_for`); the declaration never overrides a verdict.

- `distinct_model_required` is true on a verdict only when
  `CLAUDE_PERFORMER_MODEL` is set and differs from the verifier's model.
  It is never assumed.
- `distinct_human_required` is always false. No human verifies here, and
  no view may claim four-eyes review that did not happen (§27.4). `high`
  assurance is not met by this verifier.
- The verifier can be wrong. The prompt makes `not_established` the answer
  whenever evidence is absent or only asserted, but an `established` is a
  model's judgment over the bundle, bounded by what the bundle carries.
  The obligations' own plants remain the check on the work.

## Reading its verdicts

Each verdict lands in `docs/warrants/<alias>/verifications/<OBL>.toml`
under the actor `claude-verifier (<model>)`, kind `agent`, with the
evidence paragraph it wrote. `not_established` is a true state, not a
failure: it says the bundle did not show the claim. The usual causes are a
deliverable excerpted past what the verifier needed, a gate that was never
recorded, or an obligation whose evidence is a plant the bundle does not
carry.
