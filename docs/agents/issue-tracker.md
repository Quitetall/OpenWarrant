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
