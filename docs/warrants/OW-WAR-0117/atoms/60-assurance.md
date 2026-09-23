---
schema: oh.war/atom/v1
warrant_uuid: 01a0d049-9bcc-7a20-9d41-db7d11e0fb0d
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — the model chooses verdicts and nothing else
- **scope:** `tools/verifier/claude-verifier.sh`, exercised by
  `62-verifier.sh`'s fake `claude`. No claim about any real model's
  judgment.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - a fixture answer's dispositions and evidence appear in the response;
  - a fixture answer that also names a verifier, an actor and independence
    flags set false produces a response whose verifier and flags are the
    script's, byte for byte the same as without them.

### OBL-002 — anything unclear is not_established, and a failure writes nothing
- **scope:** the wrapper and `war verify --run` on a scratch corpus.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:** each of these yields `not_established` for every
  obligation it touches:
  - an answer that is not JSON;
  - an answer that omits an obligation;
  - an answer with disposition `probably`.

  And a fake `claude` that exits 1 makes `war verify --run` exit non-zero,
  with no verification written under the Warrant.

### OBL-003 — the verifier holds no tool and keeps no session
- **scope:** the argv the wrapper passes to `claude`, as recorded by the fake.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - the recorded argv contains `--disallowedTools` whose list names Bash,
    Read, Write and Edit;
  - it contains `--no-session-persistence`;
  - the plant fails if either is removed.

### OBL-004 — independence is claimed only where it is true
- **scope:** the wrapper's `distinct_model_required`, and `war check`'s
  independence findings on this repository.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - without `CLAUDE_PERFORMER_MODEL`, and with it equal to the verifier's
    model, the response says `distinct_model_required = false`;
  - with a different one, it says `true`;
  - `distinct_human_required` is `false` in every case;
  - after the declaration, `war check` reports the `basic` and `controlled`
    levels by the rule its minimums produce (sufficient or insufficient,
    by name), and no longer reports `independence.undeclared`.

## Gate Adequacy

Required at `basic`. The load-bearing plant is OBL-001's: a verifier whose
model could write its own independence would make every later resolution a
performer's report under another name.
