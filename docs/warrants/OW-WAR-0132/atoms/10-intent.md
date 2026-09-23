---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5fdc-7b93-ab15-695cd70572f1
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The product spec leaves open "time/spend and recovery defaults,
repair/rebuttal accounting, hotline delivery and deduplication, and
behavior when no authorized responder is available." In `war` today:

- **A blocking question blocks nothing.**
  - `war ask --blocking` records `questions/Q-nnn.toml`, and `war watch`
    and `war inbox` surface it.
  - `war frontier` still lists the stage as open.
  - `war perform` starts it, and `--all` walks it.
  - The performer never sees that it was waiting on an answer.
- **Nobody may be able to answer.** `war answer` accepts any non-agent
  actor in `docs/authority/roles.toml`. A repository whose register names
  no human holds blocking questions forever, and nothing says so.
  OW-WAR-0069's Q-002 (what happens to a performer whose question is
  unanswered at its wall time) has been open since 2026-09-12.
- **Spend is invisible.**
  - Nothing meters spend, and no `[perform]` key declares a spend policy.
  - Dispatches carry an empty `spend_limit`.
  - A run records no cost at all. The spec wants "unknown", never zero,
    and wants a mandatory cap that cannot be enforced to refuse the run.
- **Attempts are uncounted.**
  - `war perform` on a stage that already has an accepted submission, or
    after a timeout, simply runs again.
  - There is no repair limit (the spec's fallback is three).
  - Nothing separates repair from recovery after an interrupted run.
  - Nothing in the journal says how a performance ended.

## Desired Outcome

- **A blocking question stops its stage and only its stage.**
  - `war frontier` shows the stage `blocked`, with `waiting_on` naming the
    question.
  - `war perform` refuses `perform.question-open`.
  - Every other stage stays runnable.
  - When the question is answered, the stage is open again.
- **No responder is a visible state.**
  - A blocking question with no human in the register able to answer is
    UNKNOWN `question.no-responder` in `war next` and `war frontier`,
    naming `roles.toml`.
  - It is never silently waiting, and never treated as answered.
- **Spend is honest.**
  - `[perform] allow_unmetered` must be set for `war perform` to run with a
    performer that reports no spend.
  - `[perform] hard_spend_cap`, when set, refuses the run
    (`perform.spend-unenforceable`), because no adapter meters spend yet.
  - Every performance journals `spend: "unknown"`, never `0`.
- **Attempts are counted, by kind.**
  - Each performance journals `perform.ended` with its outcome (answered,
    refused, timeout, cancelled, failed) and its seconds.
  - A re-performance after an accepted submission is a repair.
  - One after an ending without an accepted submission is a recovery.
  - Each is refused past its limit (`perform.repair-limit`,
    `perform.recovery-limit`), with defaults the owner settles in U-001.

## Non-goals

- Metering real spend, or a provider billing adapter. The honest state
  today is "unknown".
- An AI adviser answering technical questions. The reference app's
  `advice.py` (OW-WAR-0106) does this for its own surface. In `war`, only a
  human answers (§27.2).
- Rebuttal accounting. It needs the verification-repair loop, which
  OW-WAR-0107 builds for the reference app and which `war` does not have.
- Deduplicating an identical `war ask`. OW-WAR-0130 makes `war ask`
  idempotent.
- Cancellation and writer handoff. OW-WAR-0131.
