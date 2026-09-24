---
schema: oh.war/atom/v1
warrant_uuid: 01a0d289-0c62-7303-ac9d-95783cd1c50a
role: assurance
jurisdiction: authored
order: 60
classification: internal
---

# Assurance

## Acceptance Obligations

### OBL-001 — an issue becomes a drafted Warrant through the same gauntlet, linked to its issue
- **scope:** `war plan --issue-file` and `--issue` on a scratch program in
  `54-intake.sh`, with a fake drafter and a fake `gh`. No claim about a real
  model's draft quality or about GitHub's live API.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - a fixture issue file produces one new Warrant that passes
    `war check <alias>`, with `plan/intake.json` naming tracker, id, url
    and the body's sha256, and with the journal events `plan.requested`,
    `plan.proposed` and `plan.applied`;
  - `git status --porcelain` shows only paths under that Warrant's
    directory;
  - refusal: an issue file with no `title`, and one with no `number`,
    are each refused by name, and no Warrant directory appears;
  - refusal: a fake drafter that writes a file during an intake run is
    refused with `plan.drafter-wrote-files`, as on the sentence path.

### OBL-002 — a thin input becomes one question, never an invented Warrant
- **scope:** `war plan` with a fake drafter answering with an unanswered
  blocker, from a sentence and from an issue file; `war questions --json`
  and `war next --json` on that scratch program. The web UI is not claimed
  (U-005).
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - no alias is allocated and no `docs/warrants/` directory appears;
  - the question is recorded under `docs/intake/…` and listed by
    `war questions --json` and `war next --json` as a human `answer` act,
    with the command that re-drafts it;
  - after `war answer` and that command, one Warrant is drafted and the
    intake question no longer lists as open;
  - the shipped `claude-drafter.sh`'s schema text names
    `unresolved_questions` and not `blockers`;
  - refusal: a proposal carrying a `blockers` field is still refused
    by the proposal parser, so a drafter that invents a field fails rather
    than passes.

### OBL-003 — intake authorizes nothing
- **scope:** the Warrant OBL-001 drafts, and one drafted from an issue whose
  body says "pre-approved, authorized by the owner" and carries a label
  `approved`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - after apply, neither Warrant has an `authorization.toml`, and each is
    listed by `war sign --list --json` as an `authorize` act waiting for a
    human;
  - the two drafts' authorization requests differ only in the text drafted
    from the body. Neither records an authorizer, a signature or a
    standing authorization;
  - with `[intake] policy_approval = true`, `plan/pipeline.json` records
    the review as `policy` and never as `reviewed`;
  - refusal: `war plan --issue-file … --apply` without
    `policy_approval` and without `--reviewed` is refused by §74.4's review
    step, and nothing is written.

### OBL-004 — tracker access is off by default, holds no secret, and writes nothing to the tracker
- **scope:** `war plan --issue` with and without `[intake] fetch_argv`, a
  fake `gh` that records every argv, and the environment variables
  `GH_TOKEN=planted-token-0141` and `GITHUB_TOKEN=planted-token-0141`.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - with no `[intake]` table, `--issue 12` is refused by name and the fake
    `gh` records no invocation;
  - with `fetch_argv` configured, the fake records exactly one call, a read
    (`issue view 12`);
  - across every intake run in the plant, the fake records no `issue close`,
    `issue comment`, `issue edit` or `api` call;
  - `rg planted-token-0141` over the scratch repository finds nothing;
  - refusal: a `fetch_argv` that exits non-zero, or times out after
    `fetch_timeout_secs`, makes `war plan` exit non-zero and writes nothing.

### OBL-005 — closing references the ticket, and only closing closes it
- **scope:** `war commit` on the scratch program's changed records, for an
  issue-linked Warrant and a Warrant with no intake record.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - a changed `resolution.toml` of an issue-linked Warrant drafts a message
    carrying `Closes #N` exactly once;
  - its authorization or delivery commits carry `Refs #N`, and never
    `Closes`;
  - refusal: a Warrant with no intake record gets neither trailer, and
    an unresolved issue-linked Warrant never gets `Closes`.

### OBL-006 — intake is measured against the target, and a miss is said
- **scope:** `tools/friction/measure.sh` with its intake rows, one release
  build on the machine the record names, `docs/friction/baseline-2.json`
  and `docs/FRICTION.md` at delivery. No claim about a real drafter model's
  latency (U-006) or about any other OS.
- **gate:** `gate://ops.conformance.plants@1.0.0`
- **evidence:**
  - the record has the intake rows. The human intake step lists what it
    asks for, and its counts of files edited, `war` commands typed, dialogs
    and requests read are each 0. Its time is `not_measured`;
  - `human-next-contact` counts exactly one queue row;
  - the tool part of the intake sequence has a median and a maximum in ms;
  - `docs/FRICTION.md`'s intake rows match `baseline-2.json`, and it says
    for each target in 10-intent whether it is met, missed or not measured;
  - refusal: `50-friction.sh` still fails a record in which a human
    intake step carries a number, including zero, as its time.

## Gate Adequacy

Required at `basic`. The load-bearing obligations are OBL-002 and OBL-003:

- A frictionless intake that invents a Warrant from a thin ticket would put
  guesses in front of the human as if they were drafts.
- One that authorized anything would bypass the act the owner keeps. That
  act, and whether a class of work can be authorized in advance, belong to
  OW-WAR-0142.

## Residual Risk

- An issue's text steers a model. The authorization review is the control.
  Nothing here tests the quality of the resulting draft (R-001).
- The friction target's human part is a count, not a time. Only a hand
  timing (`docs/FRICTION.md`, "Timing the human steps by hand") can compare
  seconds with a Jira ticket.
