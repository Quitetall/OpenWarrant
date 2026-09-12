---
schema: oh.war/atom/v1
warrant_uuid: 01a0948f-2334-7dc2-abe2-0839b7d82e1d
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — a question is a record, and only a human answers it
- **scope:** `questions/` under a Warrant, the journal, `war watch`.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a plant where an agent-kind actor answers is refused by name; `war questions --open` lists a planted question; `war watch --once` names it.

### OBL-002 — an agent stage is performed over the seam and lands as a submission
- **scope:** `war perform`, the configured performer.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** a recorded run with the fixture performer: Dispatch in, submission out, journal `submission.recorded`; a performer requesting `resolve` is refused (§51.2).

### OBL-003 — the board renders commands, never signs
- **scope:** `war board`, the platform's board view.
- **gate:** `gate://software.repo.war-check@1.0.0`
- **evidence:** the numbered list's rows equal `war sign --list`; the HTML reaches for nothing (the existing plant) and contains no signing call.

## Gate Adequacy

Required at `basic`.

**Adversarial question:** could the hotline become a channel through which
an agent approves its own work? Only if an answer could be an
authorization; an answer is a journal event with an actor kind, and the
tool refuses `human` acts from an `agent` actor at ingest.

- **outcome:** gap_accepted

## Residual Risk

- A performer given the hotline can stall waiting on an answer; the wall
  time bound ends it and the submission says `block`.
