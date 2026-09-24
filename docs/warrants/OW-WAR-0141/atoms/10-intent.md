---
schema: oh.war/atom/v1
warrant_uuid: 01a0d289-0c62-7303-ac9d-95783cd1c50a
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

The owner, 2026-09-24: "nobody's gonna run that command to just do work when
they could just prompt or hit a jira ticket. Can we keep that in mind?
Reduced friction? Compare us to jira tickets."

- Starting work elsewhere costs about 10 s (a prompt) or about 1 min (a Jira
  ticket). After that, the person does nothing more to start the work.
- In `war`, the tool's part is milliseconds (`docs/friction/baseline-1.json`,
  OW-WAR-0118). All the friction is in how the human reaches each act:
  reading an agent's message, copying a long `war sign --batch … --as …
  --root …` command, confirming key dialogs.
- This Warrant is the **intake** half: how work *starts*. What exists today,
  read from the source:
  - `war plan "<sentence>" --draft --reviewed --apply` drafts from a
    sentence through the configured `[plan] drafter_argv`. The draft goes
    through §74.4's gauntlet and is recorded under `plan/`. A Warrant that
    checks clean then shows up in `war sign --list`, the `war ui` Queue and
    the TUI as an authorization that waits for a human. No extra step is
    needed for that.
  - An agent that gets a prompt can reach the same path over MCP
    (`war_plan_request`, `war_plan_validate`, a reviewed `war_plan_apply`).
- What is missing:
  - **No path from a ticket.** `war plan` takes only a sentence. Nothing reads
    an issue, and nothing records which issue a Warrant came from.
  - **A question from the drafter has nowhere to go.** A blocker question
    appears in `war plan`'s output and makes it exit 1. It does not reach
    `war questions`, `war next` or the queue, because questions exist only
    under a Warrant (`questions.rs`). Also, the shipped drafter
    (`conformance/fixtures/drafter/claude-drafter.sh`) is told to answer a
    request that is too vague with a `blockers` field. `DraftProposalV2` is
    `deny_unknown_fields` and has no such field, so that answer is refused as
    malformed instead of becoming a question. This was observed in the
    source, not in a run.
  - **`--reviewed` claims a review.** On an unattended path, nobody made that
    review. Zero ceremony must not be bought with a false record.
  - **Closing does nothing to the ticket.** A resolved Warrant does not close
    or update the issue it came from.
  - **Intake is not measured.** `tools/friction/measure.sh` has no intake
    row.

## Desired Outcome

- **From a sentence or an issue to a drafted Warrant, the human writes no
  file.** The input is a prompt to an agent, `war plan "<sentence>"`, or an
  issue, read either from a file (`gh issue view --json` output) or through
  an optional, configured fetch command.
- **The next contact is one line.** It is either the Warrant's authorization
  row in the approval queue (`war ui` Queue, the TUI, `war next`), or one
  question when the input was too thin to draft. A thin input never becomes
  an invented Warrant.
- **The Warrant carries its issue.** A record names the tracker, the id, the
  URL and a digest of the text it was drafted from.
- **Closing the Warrant closes the ticket.** A resolution's commit carries
  the tracker's closing reference, and other commits carry a plain
  reference. `war` makes no network write to do this (U-003).
- **Intake authorizes nothing.** The drafted Warrant waits for a human
  authorization, like any other. Whether one signature can cover a class of
  such Warrants is OW-WAR-0142's decision (standing authorization). This
  Warrant does not overlap it.

## Friction target

Measured with OW-WAR-0118's `tools/friction/measure.sh`, extended with intake
rows, and recorded as `docs/friction/baseline-2.json`. The comparison is a
Jira ticket: the person writes the ticket, and nothing else is needed to
start the work.

- **Human, from input to queue row: no more than filing the ticket.** The
  prompt or the issue is the only human step. On the intake path the
  record counts 0 files edited, 0 `war` commands typed, 0 dialogs and 0
  requests read before the draft is in the queue. The script counts human
  steps and does not time them (OW-WAR-0118), so the time is a count
  compared with the ~10 s / ~1 min reference and is not measured.
- **Next contact: exactly one queue row per input.** That row is one
  authorization or one question, never more.
- **Tool, excluding the drafter model:** median ≤ 1,000 ms for the intake
  sequence on the scratch program, with a fixture drafter. Drafter (model)
  latency is reported separately as waiting. When no real model is run, it
  is `not_measured`.
- If the recorded numbers miss a target, `docs/FRICTION.md` says so. A
  target is not tuned to fit.

## Non-goals

- Standing or class authorization, and any change to what `war sign`,
  `war authorize` or the queue's signing does. That is OW-WAR-0142.
- A native tracker client, stored credentials, webhooks, or a server that
  listens for tracker events.
- A Jira adapter. Jira is the comparison. GitHub Issues is the intake
  tracker (`docs/agents/issue-tracker.md`). The record's `tracker` field
  leaves room for another tracker's adapter.
- Writing to a tracker from `war`: comments, labels, closing through an API.
- Changing the web UI or TUI code. A queue row and a question already render
  through the readers they share with `war sign --list` and
  `war questions`.

## SAS and Roadmap Traceability

- RQ-071 (`war plan` returns a structured proposal) and RQ-072 (proposals are
  validated before writes): intake reaches the same §74.4 gauntlet and adds
  no second write path.
- RQ-070 (the CLI works file-native and offline for drafts): network access
  is optional and off by default.
- Roadmap: `OW-PHASE-2` (Agent planner). Its exit is "a vague engineering
  request produces a reviewable valid draft without direct model file
  mutation". This Warrant extends "request" to include a ticket.
