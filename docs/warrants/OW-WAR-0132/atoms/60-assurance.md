---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5fdc-7b93-ab15-695cd70572f1
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — an open blocking question stops its stage, and only its stage
- **scope:** `war frontier` and `war perform` on a scratch corpus, over
  OW-WAR-0068's open agent stages and a fixture performer.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - After `war ask OW-WAR-0068 STAGE-001 "…" --blocking`,
    `war frontier --json` shows STAGE-001 `blocked` with `waiting_on`
    containing the question id. `war perform OW-WAR-0068 STAGE-001` is
    refused `perform.question-open` and spawns nothing (the fixture's
    marker is absent).
  - Under `war perform --all`, another open stage is still performed.
  - A non-blocking question on the same stage does not block it. The rule
    is not blanket.
  - After `war answer … --as <human>`, the stage is open again and
    `war perform` runs.

### OBL-002 — no responder is UNKNOWN, never silent and never answered
- **scope:** `war frontier` and `war next`, with `roles.toml` rewritten to
  hold only agent-kind actors, in a scratch copy.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - With a blocking question open and no non-agent actor, both commands
    report UNKNOWN `question.no-responder` naming `roles.toml`, and the
    stage is still blocked.
  - With the human restored, the finding is gone.
  - `war answer --as <agent>` is still refused, and the question file is
    byte-identical afterwards.

### OBL-003 — spend is unknown, never zero, and an unenforceable cap refuses
- **scope:** `war perform` with the `[perform]` spend keys, on a scratch
  corpus. No claim about real metered spend: nothing meters it.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - Every `perform.ended` line carries `"spend":"unknown"`, and a grep of
    the journal finds no `"spend":0` or `"spend":"0"`.
  - `hard_spend_cap = "1.00 USD"` gives `perform.spend-unenforceable`,
    with no Dispatch compiled and nothing spawned.
  - With `allow_unmetered` removed, `perform.unmetered-not-allowed` (under
    U-001's recommendation) names the key. With it `true`, the stage runs.

### OBL-004 — repairs and recoveries are counted apart and bounded
- **scope:** `war perform` on one stage, repeated with the fixture
  performers `echo-submission.sh` (answered) and `says-nothing.sh`
  (failed), with the limits set low in a scratch config.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - With `max_recoveries = 1`: two failed performances in a row, and the
    third is refused `perform.recovery-limit`.
  - With `max_repairs = 1`: answered, answered, then refused
    `perform.repair-limit`.
  - The journal's `perform.ended` outcomes match each run.
  - A failed run does not consume a repair. With `max_repairs = 1`:
    answered, failed, then answered again is permitted.

## Gate Adequacy

Required at `basic`. The load-bearing obligation is OBL-001. A blocking
question that does not stop its stage is a hotline that only decorates, and
the marker file shows no performer ran. OBL-002 carries Law 15's weight: no
responder must not read as "waiting normally".
