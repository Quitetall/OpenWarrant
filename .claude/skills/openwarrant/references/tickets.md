# Daily work

Use one Warrant for one outcome. New work is one expandable Markdown source
with a stable ID. `ticket` and `t-...` are compatibility names, not a second
work model. Ordinary coding may remain untracked.

```bash
war init                                       # once per repository
war create "Add password reset"                # title-only work is valid
war start t-3f2a --json                         # claim and receive required context
# implement, then run the relevant checks
war done t-3f2a --note "Implementation and checks"
war status --json                              # read-only progress
war check                                      # record integrity, not code qualification
```

For an agent, `start --json` carries the complete required task packet. Keep
its exact rules, dependencies and source revisions. Missing required context
is a refusal, not permission to guess. Human output is a short brief.

Extend the same Warrant with `war plan add`, `war plan note`, and
`war plan edit`. Checklists and checks do not change its ID. `create` returns
`source`, the maintained Markdown path; journals record work history. Legacy
atom directories remain readable, and `create --legacy` requests that encoding.

`war start` takes ready work or resumes the claim you hold. Explicit Warrant
requirements still bind. Normally `done` completes work unverified without a
signature. Declared required checks must all pass for the current definitions,
subjects and code; optional checks report results. Title-only work with no
completion checks uses `done`. Completion does not award an assurance mark.

Use `--as <task-role>` or `OPENWARRANT_ACTOR` for a readable actor name. Names
grant no authority. Shared-checkout path conflicts are refused. Use
`war start <id> --worktree --print` for a managed isolated workspace; overlapping
isolated work still requires integration checks. An expired lease does not
prove that its writer stopped.

Find work with `war view prime`, `war view warrants --search "<words>"`,
`war view next`, and `war view show <id>`. Over MCP, use `war_start`,
`war_done`, `war_create`, `war_add`, `war_note`, and the read-only view tools.
Existing flat command spellings remain compatible.

Authority requirements are optional unless the Warrant or repository requires
them. Preserve signed records and use the supported authority workflow when
requested. `war plan promote` is a legacy directory adapter, not a required
step to turn a ticket into a Warrant or to complete ordinary work.
