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
`war_done`, `war_create`, `war_note` and `war_status`; `war admin mcp --all`
serves the rest.
Existing flat command spellings remain compatible.

Authority requirements are optional unless the Warrant or repository requires
them. Preserve signed records and use the supported authority workflow when
requested. A person signs with `war sign`; an agent does not.

## Levels

One item, three levels, derived from its fields and not stored. A **Ticket**
(the default) is done when the agent says so. A **Warrant** has a trigger,
such as `war create "..." --test "<command>"`, and `war done` runs it and
closes the work only on a pass. An **Order** has a required human signature
(`war create "..." --sign`) and is done when the trigger, if any, passed and a
person signed. `war plan promote <id>` adds the signature in place, and
`war plan promote <id> --test "<command>"` adds a test: same ID, no new
document. `--path`, `--tool` and `--command` on `create` set permissions and do
not change the level. A person at a terminal can add `--interactive` to any of
the six commands; an agent passes arguments, and without a terminal the flag
is refused by name. `war context <target>` takes a Warrant, a document, a
section (`file.md#slug`), a few words or a code path, and fits the budget.
