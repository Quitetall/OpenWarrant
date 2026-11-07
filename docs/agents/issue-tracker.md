# Issue tracker: GitHub

Use GitHub Issues in `Quitetall/OpenWarrant` for reports, requests, and discussion.
Use the `gh` CLI from this checkout; confirm the remote before tracker writes.

## Warrant workflow

GitHub issues are intake records. Link the relevant Warrant by its repository-local
alias when work enters the governed workflow. Read `AGENTS.md` and
`CONTRIBUTING.md` before performing that work.

When a Matt Pocock skill says "publish to the issue tracker", create an issue
for the requested proposal. For a Warrant draft, use `/war-spec` and `war plan`.
For stages and blocking edges inside a Warrant, use `/war-tickets` and
`war frontier`. Use `/war-map` for a governed decision map.

Closing an issue records tracker state; it does not resolve a Warrant. Follow
`AGENTS.md` for authorization, independent verification, and signing.

## From an issue to a Warrant (OW-WAR-0141)

An issue becomes a drafted Warrant through `war plan`, the same path a sentence
takes. Nobody writes a file.

- **From a file.** `gh issue view <n> --json number,title,body,url > issue.json`,
  then `war plan --issue-file issue.json --draft --apply`. An issue with no
  `number` or no `title` is refused by name, and nothing is written.
- **Fetched.** `war plan --issue <n> --draft --apply` runs `[intake] fetch_argv`
  from `openwarrant.toml`, with `{id}` replaced by the number. It is off by
  default: with no `[intake]` table, `--issue` is refused and starts no process.

  ```toml
  [intake]
  fetch_argv = ["gh", "issue", "view", "{id}", "--json", "number,title,body,url"]
  fetch_timeout_secs = 30      # absent or 0 means 30
  policy_approval = true       # see "Who reviews the draft"
  ```

  The command is a read. A `fetch_argv` that names `close`, `comment`, `edit`,
  `api` or another write word is refused before it runs. Any token stays in
  `gh`'s own store; `war` passes the environment through and records none of
  it. A fetch that fails or runs past `fetch_timeout_secs` makes `war plan`
  exit non-zero and write nothing.

An agent reaches the same path over MCP: `war_plan_request`,
`war_plan_validate` and `war_plan_apply` take `issue_file` (relative to the
repository root) or `issue` in place of `sentence`, under the same refusals
and review rule.

The issue's title and body become the draft request's sentence, and the
configured `[plan] drafter_argv` drafts from it. The applied Warrant records
where it came from in `plan/intake.json` (`oh.war/intake/v1`): the tracker,
the issue number, its public URL (without any user-info or query), its title
and the sha256 of its body. The body is a request, not evidence (§74.8). Labels
are not kept: a label such as `approved` means nothing to `war`.

**Who reviews the draft.** §74.4 asks for "review or policy approval" before
atoms are written. With `[intake] policy_approval = true`, an intake apply is
recorded as `review: policy` in `plan/pipeline.json`, even if `--reviewed` is
passed; nobody read the proposal, and the record says so. Without it, the apply
needs `--reviewed` from a person who did read it, or it is refused at the review
step and nothing is written.

**What the human sees next.** Exactly one row. Either:

- the drafted Warrant's authorization, in `war sign --list` and `war next`
  (the `war ui` Queue and the TUI read the same pending list). Intake
  authorizes nothing: the Warrant waits for a human's signature like any
  other, whatever the issue's text says; or
- one question, when the issue is too thin to draft. The drafter answers with
  `unresolved_questions`, and no alias is allocated. The question waits under
  `docs/intake/github-<n>/questions/`, beside the request and the intake record
  (`docs/intake/sentence-<digest>/` for a sentence). `war questions` and
  `war next` list it as a human `answer` act. A human answers with
  `war answer github-<n> Q-001 "<answer>" --as <actor>`, and the question
  record names the command that drafts the input again with its answers.

## What closes the issue

Only a resolution does, and only through git. When the records of a Warrant
with `plan/intake.json` change, `war commit` ends the message it drafts with
the tracker's reference, once per issue:

- `Closes #<n>` when the Warrant's `resolution.toml` is among the changes.
  GitHub closes the issue when that commit reaches the default branch;
- `Refs #<n>` for any other change: the authorization, a delivery, a question.

A commit that never reaches the default branch closes nothing, and the issue
stays open. `war` makes no network call and never writes to the tracker:
no comment, no label, no close through an API. The reverse also holds: closing
the issue by hand does not resolve the Warrant.

## Tracker operations

- Read an issue: `gh issue view <number> --comments`.
- Fetch fields: `gh issue view <number> --json number,title,body,labels,comments`.
- List issues: `gh issue list --state open --json number,title,body,labels`.
- For authorized writes, use `gh issue create`, `edit`, `comment`, or `close`.
  Put multiline bodies and comments in a temporary file and use `--body-file`.
- Use `docs/agents/triage-labels.md` for role-to-label mappings.
- Read `SECURITY.md` before reporting a security issue.

Local setup does not authorize publishing issues or sending messages. Follow
the user's scope for external writes.

## Pull requests as a triage surface

**PRs as a request surface: no.**

## Wayfinding operations

For an explicitly requested GitHub issue map, use one parent labelled
`wayfinder:map` and children labelled `wayfinder:<type>` (`research`,
`prototype`, `grilling`, or `task`).

Use native sub-issues and issue dependencies through `gh api` where available.
Dependencies use the blocker's database `id`, not its issue number. Otherwise,
use a parent task list, a `Part of #<map>` pointer in each child, and
`Blocked by: #<number>` lines naming its blockers.

Eligible children are open, unassigned, and have no open blockers. Read the live
tracker before claiming work. Assign, record answers, and update tracker state
within the user's authorized scope. For Warrant execution eligibility, consult
`war next` and `war frontier`.
