---
schema: oh.war/atom/v1
warrant_uuid: 01a0d289-0c62-7303-ac9d-95783cd1c50a
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

Written for the recommended options of U-001 to U-004. If another option is
chosen, this revision is amended before the work is dispatched.

## Deliverables

1. `crates/openwarrant-cli/src/intake.rs`: the intake record and its readers.
   - `oh.war/intake/v1`: `tracker` (`github`), `id`, `url`, `title`,
     `body_sha256` and `fetched_at`. The body is the draft request's
     sentence, and it is not evidence.
   - It reads `--issue-file <path>` (the output of `gh issue view --json`).
     An input with no `number` or no `title` is refused by name.
   - `--issue <id>` runs `[intake] fetch_argv` with `{id}` substituted. With
     no `[intake]` table, it is refused by name and no process is started.
   - It writes nothing to a tracker. No code path runs a tracker write
     subcommand.
2. `crates/openwarrant-cli/src/plan.rs`: `war plan --issue <id>` and
   `--issue-file <path>`, both beside the sentence.
   - An applied draft records `plan/intake.json` beside `plan/request.json`.
   - With `[intake] policy_approval = true`, an intake apply records
     `review: policy` in `pipeline.json`. It never records `reviewed`
     (U-001).
   - A proposal whose blockers remain open allocates no alias. It writes
     `docs/intake/<tracker>-<id>/` (or `sentence-<digest>/`) holding the
     request, the intake record and the questions (U-002).
3. `crates/openwarrant-cli/src/questions.rs`: `war questions`, `war answer`
   and `war next` list and answer intake questions beside Warrant questions.
   Each intake question names the command that re-drafts it.
4. `crates/openwarrant-cli/src/repo.rs`: the `[intake]` table:
   `fetch_argv`, `policy_approval` and `fetch_timeout_secs`. All are absent
   by default.
5. `crates/openwarrant-cli/src/commit.rs`: when the changed records belong to
   an issue-linked Warrant, add `Closes #N` for a resolution and `Refs #N`
   otherwise, exactly once each (U-003).
6. `conformance/fixtures/drafter/claude-drafter.sh`: a too-vague answer goes
   in `unresolved_questions` (`id`, `question`, `removes_blocker: true`),
   not `blockers`.
7. `tools/friction/measure.sh`: intake rows on the scratch program, using a
   fixture drafter and a fixture issue file.
   - `human-file-issue`: counted, not timed.
   - `plan-issue`: the tool's part.
   - `human-next-contact`: one queue row, counted.
8. `docs/friction/baseline-2.json`: the script's unedited output from a
   release build.
9. `docs/FRICTION.md`: the intake rows and the comparison with a prompt and a
   Jira ticket. It says whether each target in 10-intent is met, missed or
   not measured.
10. `docs/agents/issue-tracker.md`: how an issue becomes a Warrant, and what
    closes it.
11. `conformance/plants.d/54-intake.sh`: every obligation in 60-assurance.
    It uses a fake `gh` on PATH that records its argv and answers from a
    fixture, and a fake drafter.

## Frozen Surfaces

- `oh.war/draft-request/v1` and `oh.war/draft-proposal/v2` (except the
  optional intake record written beside them).
- `sign.rs`, `authorize.rs`, `resolve.rs`: this Warrant changes nothing that
  signs, authorizes or resolves.
- The web UI and TUI code.
- `baseline-1.json`.

## Premade Instructions

- One path from input to atoms: `plan::apply`. Intake builds a `Request` and
  hands it to the path that exists. It never writes an atom itself.
- No network in any default path. Plants run with no network and no `gh`
  except the fake.
- No token, credential or tracker URL with a secret is written to any record.
  Only `url` is kept, and it is the issue's public URL.

## Autonomy and Escalation

Tier T2. Escalate rather than decide:
- any answer to U-001 to U-004 other than the recommendation;
- any change that would make intake produce an authorization, a signed
  record or a queue act other than `Authorize` or `Answer`;
- any overlap with OW-WAR-0142.

## Rollback

- Remove `--issue`, `--issue-file` and `[intake]`. `war plan "<sentence>"`
  behaves as it did before.
- Existing `plan/intake.json` and `docs/intake/` records stay as history and
  are ignored.
- `baseline-2.json` stays, because a baseline is never removed.
