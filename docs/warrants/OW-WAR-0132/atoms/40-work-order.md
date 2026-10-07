---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5fdc-7b93-ab15-695cd70572f1
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `crates/openwarrant-cli/src/frontier.rs`:
   - a stage with an open blocking question is `blocked`, with
     `waiting_on: ["Q-nnn"]`;
   - if the register holds no non-agent actor, the row also carries UNKNOWN
     `question.no-responder`;
   - an answered question no longer blocks.
2. `crates/openwarrant-cli/src/perform.rs`, before compiling:
   - refuse `perform.question-open`, naming the question and its stage;
   - refuse `perform.spend-unenforceable` when `hard_spend_cap` is set;
   - refuse `perform.unmetered-not-allowed` when `allow_unmetered` is not
     true;
   - refuse `perform.repair-limit` or `perform.recovery-limit` by the
     counts below.

   After every performance, journal `perform.ended` with stage,
   dispatch_id, outcome, seconds, and `spend: "unknown"`.

   - Repairs: performances after the first `answered` one on the stage.
   - Recoveries: performances after an ending that was not `answered`,
     since the last `answered`.
3. `crates/openwarrant-core/src/config.rs`: `[perform]` gains
   `allow_unmetered: Option<bool>`, `hard_spend_cap: Option<String>`,
   `max_repairs: u32` and `max_recoveries: u32`. 0 means the default U-001
   settles, and the defaults are stated in the doc comments.
4. `openwarrant.toml`: this repository sets `allow_unmetered = true`, with
   a comment saying cost is unknown.
5. `docs/HOTLINE.md`:
   - how a question blocks and unblocks;
   - no responder is UNKNOWN;
   - the mid-run rule from U-002;
   - the time, spend and attempt defaults and what each counts;
   - what is not counted (A-001).
6. `conformance/plants.d/52-hotline-defaults.sh`, the plants each
   obligation names.

- (AM-002) `conformance/plants.d/60-perform-cancel.sh` and
  `conformance/plants.d/83-tokens.sh`: their scratch configs append a
  `[perform]` table; each gains `allow_unmetered = true`, so they test what
  they test and not the new default.
- (AM-002) `crates/openwarrant-cli/src/next.rs`: `war next` reports
  `question.no-responder` beside a question-blocked stage, and does not
  offer a stage blocked on an open question as an agent action.
- (AM-002) `crates/openwarrant-cli/src/init/mod.rs`: a new program's
  `openwarrant.toml` states `allow_unmetered = true` under `[perform]`, with
  a comment that spend is then recorded as unknown; a repository without
  the key is still refused, by name.

## Frozen Surfaces

- `oh.war/question/v1`, `oh.war/stage-dispatch/v1` and
  `oh.war/stage-submission/v1`.
- `war answer`'s actor-kind refusal.
- The frontier's JSON schema (`oh.war/frontier/v1`). `waiting_on` already
  exists, and a question id is a new value in it, not a new field.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:
- any default other than U-001's answer;
- anything that would let an agent's text count as an answer;
- metering any real spend.

## Rollback

Revert the declared files. `perform.ended` journal lines stay as history,
and the old code ignores the event type.
