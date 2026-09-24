---
schema: oh.war/atom/v1
warrant_uuid: 01a0d289-0c62-7303-ac9d-95783cd1c50a
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

## Governing sources

- §74.2: the agent returns a structured Draft Proposal, not file writes.
- §74.4: parse, validate schema and references, run risk and authority
  checks, show a semantic diff, require review or policy approval, write
  authored atoms, then compile and check. Intake adds nothing that bypasses
  a step.
- §74.6: ask the minimum set of questions needed to remove blockers. A thin
  ticket is answered with those questions, not a guessed Warrant.
- §74.8: the planner never fabricates a source or a gate result. An issue's
  text is recorded as the source it is: a request, not evidence.
- §75.4: an adapter has no authority to authorize or resolve. The fetch
  command is an adapter.
- §76.5: agents and CI have a fully noninteractive mode. Intake runs with no
  prompt.
- RQ-070, RQ-071, RQ-072 (see 10-intent).
- `docs/agents/issue-tracker.md`: GitHub Issues is the intake tracker.
  Closing an issue records tracker state and does not resolve a Warrant. The
  reverse holds too: resolving a Warrant does not close an issue through
  `war`.
- OW-WAR-0118: `tools/friction/measure.sh`, `docs/FRICTION.md` and
  `baseline-1.json`. A new baseline is a new file. Human steps are counted,
  never timed.
- OW-WAR-0142 (sibling, decision): standing authorization. This Warrant
  delivers a drafted Warrant that waits for authorization. It does not say
  who or what gives that authorization.
- OW-ADR-0021: this Warrant declares files that other Warrants own:
  `questions.rs` and `repo.rs` (OW-WAR-0130 and others), and `measure.sh` and
  `docs/FRICTION.md` (OW-WAR-0118). On authorization, it becomes their owner
  for the parts it changes.

## Code read for this draft (2026-09-24)

- `crates/openwarrant-cli/src/plan.rs`: `request`, `run_drafter` (with its
  `git status --porcelain` audit), `validate_v2` (with
  `plan.interview-required`) and `apply`. Records land under
  `docs/warrants/<alias>/plan/`.
- `crates/openwarrant-core/src/drafting_v2.rs`: `DraftProposalV2` is
  `deny_unknown_fields`, and its questions field is
  `unresolved_questions`.
- `conformance/fixtures/drafter/claude-drafter.sh`: tells the model to put
  a too-vague answer in `blockers`.
- `crates/openwarrant-cli/src/sign.rs` `pending`: a Warrant that validates
  and has no current signed authorization is an `Authorize` act. So a
  drafted Warrant reaches the queue with no extra step.
- `crates/openwarrant-cli/src/questions.rs`: a question is
  `questions/Q-nnn.toml` under a Warrant. Nothing holds a question that has
  no Warrant.
- `crates/openwarrant-cli/src/commit.rs`: classifies the records that
  changed, including `resolution.toml`, and adds no tracker reference.
- A search of `crates/` for issue, tracker, ticket and Jira code found none
  except `TrackerTrust` for document workflows, which is unrelated.

## Assumptions

- A-001: `gh issue view <n> --json number,title,body,labels,url` is a stable
  enough shape to accept as a file input. Confidence: medium. The intake
  record keeps only the fields it uses, and the parser refuses an input that
  is missing `title` or `number` by name.
- A-002: GitHub closes an issue when a commit containing `Closes #N`
  reaches the default branch. Confidence: high for GitHub. This is not
  claimed for Jira.
- A-003: the fixture drafter's time is not the drafter's time. A real
  model's latency is waiting, and it is reported apart from administration.
  Confidence: high.

## Unknowns

- **U-001 (blocking): who is §74.4's reviewer on the intake path.** Today an
  agent can pass `--reviewed`, which records a review that nobody made.
  - (a) Policy approval: `[intake]` declares that intake drafts are applied
    under policy. The pipeline records `review: policy`, not `reviewed`, and
    the authorization request is the human's review of the contract.
  - (b) Keep `--reviewed` as it is. This records a false review.
  - (c) A human reviews each proposal before apply. That is a second contact
    and it defeats the target.
  - **Recommendation: (a).** §74.4 allows "review or policy approval", and
    the authorization act remains the human's.
- **U-002 (blocking): where a question lives before a Warrant exists.**
  - (a) `docs/intake/<tracker>-<id>/` holds the request, the intake record
    and `questions/Q-nnn.toml`, with no alias allocated. `war questions` and
    `war next` read it. `war answer` followed by one re-draft applies it.
  - (b) Allocate an alias and keep the question in that Warrant's
    `questions/`. This creates a Warrant with no content, which is the
    invented Warrant this Warrant forbids.
  - (c) Post the question back to the ticket. That is a network write.
  - **Recommendation: (a).**
- **U-003 (blocking for M2): how closing reaches the ticket.**
  - (a) `war commit` adds `Closes #N` to a resolution commit of an
    issue-linked Warrant and `Refs #N` to its other commits. GitHub closes
    the issue on merge, and `war` makes no network call.
  - (b) A configured `[intake] close_argv` that `war sign` runs after it
    records a resolution.
  - (c) Both.
  - **Recommendation: (a).** It is offline and adds no command. (b) adds a
    network side effect to a signing act, and that belongs in its own
    decision.
- **U-004 (blocking): how `war` reaches the tracker at all.**
  - (a) A file input (`--issue-file`), plus an optional
    `[intake] fetch_argv` (for example `gh issue view {id} --json …`). It is
    off by default, and the token stays in the tool's own store, never in
    `war`.
  - (b) A native HTTP client, with a token read from the environment.
  - (c) File input only.
  - **Recommendation: (a).** It keeps the `drafter_argv` and
    `verifier_argv` seam pattern and holds no secret.
- U-005 (non-blocking): whether the web UI's Questions page lists intake
  questions without a UI change. It reads through `questions.rs` today, so
  it probably does. It is not claimed. OBL-002 is bounded to `war questions`
  and `war next`.
- U-006 (non-blocking): the latency of a real drafter model. Only the
  fixture is measured here.

## Residual risks

- R-001: an issue's body is untrusted text going to a model. It can say
  "this is pre-approved". OBL-003 plants that text, and the answer must not
  change: the drafted Warrant is unauthorized. The text can still steer the
  draft's content. The human's authorization review is the control for
  that.
- R-002: a closing reference in a commit that does not reach the default
  branch closes nothing. The issue then stays open, which is the safe
  direction.
