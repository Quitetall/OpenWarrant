---
schema: oh.war/atom/v1
warrant_uuid: 01a0d04c-5fdc-7b93-ab15-695cd70572f1
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- RQ-044: agent authority is explicit and bounded. Time, spend and attempt
  limits are bounds.
- §55.7: a Stage SHALL declare spend, model turns and wall time, and budget
  exhaustion halts honestly.
- §53.1 and §53.5: a blocker needs a condition resolved. An open blocking
  question is that condition.
- §52.3 and §52.4: repair and restart are distinct attempt kinds.
  Recovery after an interrupted run is counted apart from repair (product
  spec).
- §27.2: an agent asks; only a human answers. `questions.rs` enforces it by
  actor kind.
- Law 15 (AGENTS.md rule 3): no responder is UNKNOWN, neither failure nor
  pass. Unmetered spend is "unknown", never 0.
- Product spec, "Verification, repair, and acceptance" and
  "Questions and changes of plan":
  - three repair cycles is the fallback;
  - time and spend limits may stop work earlier;
  - pause the affected stages and their dependents, and let independent
    work continue;
  - "if a mandatory limit, including a hard spend cap, cannot be enforced,
    refuse that execution".
- OW-WAR-0069 (authorized): the hotline and `war perform`. Its Q-002 is
  still unanswered; see U-002.
- OW-ADR-0021: this Warrant declares:
  - `frontier.rs`, owned by OW-WAR-0068;
  - `perform.rs`, also declared by OW-WAR-0131;
  - `config.rs`, also declared by OW-WAR-0113 and 0130;
  - `openwarrant.toml`, also declared by OW-WAR-0113 and 0117.

## What exists (read 2026-09-23, branch `claude/hub`)

- `questions.rs`: `Question { stage, blocking, answer }`; `answer` refuses
  an agent-kind actor.
- `frontier.rs`: states are open, claimed, done and blocked, where blocked
  means only "milestone depends_on unmet". No question is read.
- `perform.rs`:
  - the wall-time bound is the tighter of `[perform] performer_timeout_secs`
    and the stage's `wall_time_seconds`, else `[run]` 600;
  - there is no spend, attempt count or journal event beyond
    `dispatch.compiled`.
- `inbox/mod.rs` already computes `blocking_question` per Warrant, and
  `watch.rs` lists open questions, blocking first. Delivery to a human who
  looks exists. Delivery to one who does not look is U-003.

## Assumptions

- A-001: every performance of a stage goes through `war perform`, so its
  journal is a complete count. A Dispatch compiled with `war dispatch` and
  run by hand is not counted. That is named in `docs/HOTLINE.md`, not
  claimed. Confidence: medium.
- A-002: "a human able to answer" means an assignment in `roles.toml` whose
  `actor_kind` is not `agent`, which is what `war answer` checks.
  Confidence: high.

## Unknowns

- U-001 (**blocking authorization**): the defaults. The deliverable set
  does not change with the answer, but the obligations' numbers do.
  Recommendation, from the product spec where it speaks:
  - `max_repairs = 3`;
  - `max_recoveries = 2`;
  - `allow_unmetered` absent means refuse, with a remedy naming the key;
  - `war init` writes `allow_unmetered = true`, with a comment saying cost
    is unknown.

  Alternative: absent `allow_unmetered` means allow with a WARN. That is
  cheaper for existing repositories, but it is the default the spec says to
  avoid.
- U-002 (non-blocking; OW-WAR-0069 Q-002): may a performer ask mid-run?
  This Warrant assumes the recorded recommendation: yes, and only its own
  stage waits. A performer whose question is unanswered at its wall time
  ends with a submission requesting block and citing the question id. That
  is a performer's act that `war submit` already admits; nothing here
  writes it for the performer.
- U-003 (non-blocking): push delivery when nobody opens `war watch`,
  `war next` or the UI (mail, chat). Out of scope. The UNKNOWN is the
  signal, and a notifier is a later adapter.

## Residual risks

- R-001: a repository that relied on `war perform` running unmetered
  starts to refuse on upgrade if U-001 keeps "absent means refuse". The
  refusal names the one key to add, and `CHANGELOG.md` says so.
- R-002: attempts compiled outside `war perform` are uncounted (A-001).
