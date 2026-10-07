---
schema: oh.war/atom/v1
warrant_uuid: 01a0d049-9bcc-7a20-9d41-db7d11e0fb0d
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `tools/verifier/claude-verifier.sh`: the wrapper.
   - Reads the bundle path, runs `claude -p` with every tool disallowed and
     no session persistence, and prints an
     `oh.war/verification-response/v1`.
   - The verdicts come from the model. The verifier identity and the
     independence flags come from the script.
   - An unparseable answer, a missing obligation or an unknown disposition
     becomes `not_established`. A non-zero `claude` exit is the wrapper's
     exit.
   - `CLAUDE_VERIFIER_LOG` keeps a run record: version, model, times, the
     raw answer.
2. `openwarrant.toml`:
   - `[verify] verifier_argv = ["tools/verifier/claude-verifier.sh"]`.
   - `[independence]` declares what the wrapper provides by construction,
     each flag with a one-line reason as a comment, and
     `distinct_human_required = false`.
3. `conformance/plants.d/62-verifier.sh`. A fake `claude` on PATH records
   its argv and answers from a fixture. On a scratch corpus:
   - the verdicts pass through;
   - a model answer claiming independence or a verifier name changes
     nothing the script writes;
   - garbage and a skipped obligation become `not_established`;
   - an unknown disposition becomes `not_established`;
   - a failing `claude` makes `war verify --run` write nothing;
   - the argv carries `--disallowedTools` naming Bash, Read, Write and Edit,
     and `--no-session-persistence`;
   - `distinct_model_required` is false without `CLAUDE_PERFORMER_MODEL`,
     and true with a different one.
4. `docs/VERIFICATION.md`: how to run it, what its independence is and is
   not, and how to read its verdicts.

## Frozen Surfaces

`oh.war/verification-bundle/v1`, `oh.war/verification-response/v1`,
`verify.rs`'s admissibility, and every record schema.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:
- any change to what `admissible_for` accepts;
- any flag the wrapper cannot justify by construction.

## Rollback

Remove `[verify]` and restore `[independence]`. Verdicts already recorded
stay recorded, with the verifier and flags they were ingested under.
