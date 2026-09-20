---
schema: oh.war/atom/v1
warrant_uuid: 01a0948f-2334-7dc2-abe2-0839b7d82e1d
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `crates/openwarrant-cli/src/questions.rs`: `war ask <alias> <stage> "<question>"` (agent; records `questions/Q-nnn.toml`, journal `question.asked`), `war answer <alias> Q-nnn "<answer>"` (human; journal `question.answered`), `war questions [--open]` across every Warrant, `war answers <alias> <stage>` for the performer.
2. MCP tools `war_ask`, `war_answers`, `war_questions` (read and agent-permitted writes only).
3. `war watch` raises open questions beside pending signatures.
4. `crates/openwarrant-cli/src/perform.rs`: `war perform <alias> <stage>` compiles the Dispatch, spawns `[perform] performer_argv` with it on stdin (the oh.war/eval-perform/v1 shape), bounds it in wall time, ingests the returned submission through `war submit`; `--all` walks the frontier's open agent stages.
5. `war board` and the platform's board view: program, objectives, Warrants, stages with frontier state, and the numbered approval list where each row is the exact `war sign` command.
6. `/war-grill` gains a batch mode that asks every open question of every Warrant in one round.
7. Plants for each refusal: an agent answering, a performer that signs, a submission asking to resolve, a question with no stage.

## Frozen Surfaces

`oh.war/stage-dispatch/v1`, `oh.war/stage-submission/v1`, the report envelope, the signing seam.

## Premade Instructions

- A question is asked by an agent and answered by a human; the tool refuses
  the reverse by actor kind, as `war sign` does.
- The board renders commands, never buttons that sign.
- The performer is a separate process over the seam, like the drafter.

## Autonomy and Escalation

Tier T2. Every design branch the owner did not settle on 2026-09-12 is a
`/war-grill` question before code.

## Rollback

Delete the commands; the records they wrote stay as history.
