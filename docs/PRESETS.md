# Presets, roles and official Warrants

How much rigour a repository asks for is a **repo preset** plus **roles**
(OW-WAR-0148 M14). Someone allowed to make a kind of Warrant makes it: an
admin can vibe, and an outsider writes a formal Warrant that `war check`
passes. Ordinary coding still needs no Warrant at all.

A repository with no `[preset]` and no `[roles]` table behaves exactly as it
did before presets existed. Plain `war init` writes neither.

## Choosing a preset

```bash
war init --vibe          # or --team, or --regulated
war admin preset         # what is in force, and what follows from it
war admin preset team    # switch an existing repository (`none` removes it)
```

| | vibe | team | regulated |
|---|---|---|---|
| least every tick shows (`ticks`) | claimed | claimed | observed |
| signatures (`signing`) | never | batched at release | at merge |
| PR gate (`pr_requires_official`) | reported, not required | required | required |
| `war done` on an unclaimed item | claims it first | refused, `ticket.not-claimed` | refused |
| signing examples under `docs/authority/` | none | written | written |
| admin, maintain make alone | vibe | vibe | tested |
| write makes alone | tested | tested | formal |
| triage, read, outsiders make alone | formal | formal | formal |

`war init --<preset>` appends this to `openwarrant.toml`:

```toml
[preset]
name = "team"
ticks = "claimed"
signing = "release"
pr_requires_official = true

[roles]
admin = "vibe"
maintain = "vibe"
write = "tested"
triage = "formal"
read = "formal"
none = "formal"
```

Every key may be edited. `war admin preset <name>` rewrites `[preset]` with
the new preset's values and keeps every other byte of the file; it writes
`[roles]` only when the file has none, and keeps yours otherwise
(`--reset-roles` writes the preset's, keeping `[roles.roster]`). A value
that is not one of the names above is refused by `war check`
(`preset.config`), naming the key.

Under **vibe**, a person and their agents reach a first done in three
commands: `war init --vibe`, `war create "..."`, `war done <id>`. `war done`
on an item nobody holds and nothing blocks claims it, then ticks it; the
journal records both (`ticket.claimed`, `ticket.item_done`). An item someone
else holds, or one that waits on another, is still refused by name. Without
the vibe preset the claim stays a separate step (OW-WAR-0147 OBL-003).

Under **regulated**, a plain `war done` is refused
(`ticket.tick-below-minimum`, naming "the repo preset's [preset] ticks
minimum"): every item ticks with `war done <item> --check`, so every item
needs a test.

## Kinds and roles

A Warrant's **kind** is read from the record, never declared:

| kind | what it is |
|---|---|
| `vibe` | a title (and a checklist); a light Warrant with no test |
| `tested` | a light Warrant with a test, or a KPI that decides a tick (`war add <id> --test "<command>"`) |
| `formal` | a directory Warrant (`war plan new`) that `war check` passes. One that `war check` refuses counts as its title, `vibe`, until it passes |

The **roles** mirror GitHub's repository roles: `admin`, `maintain`,
`write`, `triage`, `read`, and `none` for someone with no role (an
outsider). Each maps to the least formal kind that role may create alone;
anything at or above it is allowed. Because `formal` is the top, every role
may make a formal Warrant alone.

## The roster

Trust comes from two places. In CI it is the person's GitHub role, read
through the API. Locally it is the **roster**: the authority register that
already exists, `docs/authority/roles.toml` (each human and their
`ssh_principal`) and `docs/authority/allowed_signers` (their key), or the
protected store when `[authority]` names one (docs/AUTHENTICATION.md). There
is no second key store. `[roles.roster]` only says which role a principal's
signature counts as:

```toml
[roles.roster]
ada = "maintain"     # the principal, as roles.toml spells it
```

It grants no signing right and holds no key: a principal still signs only
with the key `allowed_signers` (or the store) binds to it.

## Official Warrants

A Warrant is **official** when someone allowed made it, or approved it:

1. **made by** someone whose role allows its kind (an admin's title-only
   Warrant; a writer's tested one; anyone's formal one);
2. **approved with a signature**: `war sign approve <id> --ssh-sign` by a
   roster key whose `[roles.roster]` role allows the kind, verified against
   the roster;
3. **approved in review**: an approving GitHub review, at the PR's head
   commit, by someone whose role allows the kind. This is read through
   `gh api` by `war check --pr` in CI. Anywhere else it is UNKNOWN, never a
   pass.

`war show <id>` and `war status <id>` end with an `## Official` section, and
bare `war status` with one line per Warrant (and `official` under
`--json`), only when a preset or `[roles]` is configured:

```text
## Official

official (signed): approved by Ada (ada, maintain); maintain may make a vibe Warrant
kind: vibe (a title and checklist, no test)
GitHub review: unknown here; `war check --pr` reads it from GitHub in CI
preset: team
```

From a checkout, "made by" reads the name a light Warrant recorded at
creation (`created_by`) and the roster's role for it: a name anyone can
type, so the section says so, and the PR gate does not use it (below).
When nothing in the checkout makes a Warrant official, the section says
"not established here" and names what would: never "not official", because
a review the checkout cannot read might.

Nothing here restricts work. An unofficial Warrant is created, claimed and
ticked like any other; a fork is unrestricted. Officialness is what the PR
gate reads.

### Approving: `war sign approve <id>`

```bash
war sign approve t-3f2a                       # the request: no key, no response written
war sign approve t-3f2a --dry-run             # what would be signed, and whether it would record
war sign approve t-3f2a --ssh-sign --as Ada   # the person's approval
```

The request names what an approval binds and who may sign it, records
`ticket.approval_requested` in the Warrant's journal (once per plan), and
runs `[notify]`; `war next` and `war sign inbox` then list it as a person's
act. The signature writes
`docs/authority/responses/<id>.approval.response.toml` and its `.sig`, made
by the person's key through the ssh agent, and keeps it only when it
verifies as theirs (act `approve`, response schema
`oh.war/warrant-approval-response/v1`); then `ticket.approved` is
journalled. The tool holds no key.

An approval binds a statement of the plan: the Warrant's identity, title,
description, items' text and tests. Ticking an item or adding a note leaves
it standing. Changing the plan makes it stale: it no longer counts, and the
`## Official` section says so. A directory Warrant is not approved this way
(`approve.directory`): it is official when it is formal, and its
authorization is `war sign <alias>`.

Refused: a person whose role does not allow the kind, or who has no
principal or no `[roles.roster]` entry (`approve.no-eligible-approver`); an
item instead of a Warrant (`approve.item`); every `sign.*` refusal a
signature can meet. Each ends "This blocks only the approval, not your
work."

## The PR gate: `war check --pr <number>`

A pull request passes when it cites a Warrant that is official at its
author's level. The Warrant and its code may arrive in the same PR.

```bash
war check --pr 12                                   # in CI: GITHUB_REPOSITORY names the repository
war check --pr 12 --repo owner/name --summary "$GITHUB_STEP_SUMMARY" --comment
```

It runs `gh` and nothing else, so its token is `gh`'s (`GH_TOKEN` in CI).
In order:

| call | what it gives |
|---|---|
| `gh api repos/{o}/{r}/pulls/{n}` | the author, the body, the base and head commits |
| `gh api -H "Accept: application/vnd.github.raw" repos/{o}/{r}/contents/openwarrant.toml?ref=<base>` | `[preset]` and `[roles]` as the **base** has them |
| `gh api repos/{o}/{r}/collaborators/{author}/permission` | the author's role (`role_name`, else `permission`) |
| `gh api --paginate repos/{o}/{r}/pulls/{n}/commits` | `Warrant: <id>` trailers |
| `gh api --paginate repos/{o}/{r}/pulls/{n}/files` | Warrants the PR adds; whether it changes the roster |
| `gh api --paginate repos/{o}/{r}/pulls/{n}/reviews` | each reviewer's latest review |
| `gh api repos/{o}/{r}/collaborators/{reviewer}/permission` | each approving reviewer's role |

- **Citing.** A `Warrant: <id>` line (or `Warrants:`, ids separated by
  commas or spaces) in the PR body or any commit message, or a Warrant
  manifest the PR adds (`docs/tickets/<id>/manifest.toml`,
  `docs/warrants/<alias>/manifest.toml`).
- **The author's level.** Every cited Warrant is judged with the PR's
  author as its author: official when the author's role allows its kind,
  or when an allowed approver approved it. A Warrant already on the base
  branch is judged the same way: its recorded `created_by` is a name
  anyone can type, so the gate does not credit it. A maintainer's plan an
  outsider picks up is official once the maintainer approves (a review, or
  `war sign approve`).
- **The base's policy.** `[preset]` and `[roles]` are read from the base
  commit through the API, so a PR cannot loosen them to pass itself. A base
  with no `openwarrant.toml` is held to the team preset's defaults, and the
  gate says so (`pr.no-policy`).
- **Reviews.** Each reviewer's latest review that is not a comment;
  an approval counts only at the PR's head commit (an approval of an earlier
  commit does not approve code pushed after it) and only by a role that may
  make the Warrant's kind. A writer's approval of an outsider's title-only
  Warrant counts for nothing.
- **Signed approvals.** Verified against the checkout's roster, with the
  approver's role from the base's `[roles.roster]`. A PR that changes
  `docs/authority/roles.toml` or `allowed_signers` gets no credit for a
  roster approval (`pr.roster-changed`): the roster that would verify it is
  the one being changed.
- **Signing at merge.** Under `signing = "merge"` (regulated), an official
  Warrant also needs a verified signature by merge time: a `war sign
  approve` for a light one, a signed authorization for a directory one.

| rule | severity | when |
|---|---|---|
| `pr.official` | pass | a cited Warrant is official at the author's level |
| `pr.not-required` | pass | the base's preset does not require it (vibe) |
| `pr.cited` | pass or warn | one cited Warrant, and why it is or is not official |
| `pr.not-official` | error | it cites Warrants, none official; names the outsider, and what would pass |
| `pr.no-warrant` | error | it cites none and adds none |
| `pr.warrant-unknown` | warn | a cited id names no Warrant in the checkout |
| `pr.roster-changed` | warn | it changes the roster, so signed approvals are not counted |
| `pr.no-policy` | warn | the base has no openwarrant.toml |
| `pr.unknown` | UNKNOWN | a `gh` call failed, timed out or answered something unreadable, and what it would have told could decide the verdict |
| `pr.number`, `pr.repo` | error | not a PR number; not `owner/name` |
| `pr.summary-failed`, `pr.comment-failed` | warn | the summary or comment was not written; the verdict stands |

Every refusal ends "It passes with ...", naming the ways: a Warrant the
author's role allows alone, an approving review at the head commit by the
roles that may, or `war sign approve` by a roster key with such a role.
UNKNOWN exits 2, like a refusal, and is never a pass: the gate passes once
GitHub answers. With `--json` the envelope's command is `check.pr` and its
result `oh.war/pr-check/v1`, carrying every `gh` call made.

`--summary <file>` appends a Markdown summary (in CI, the job summary);
`--comment` posts it on the PR with `gh api -X POST
repos/{o}/{r}/issues/{n}/comments`. Either failing is a warning.

**Why `war check --pr`, not `war evidence pr-check`.** It is a check of
the records against a PR, run in CI the way `war check --generated` is.
Without `--pr`, `war check` still reads no network.

The GitHub Action that runs it as a required status check is
`.github/actions/openwarrant-check/action.yml`; docs/INSTALL.md, "The PR
gate as a required status check", sets it up with a ruleset.

## Signing: never, at release, at merge

Signing is asynchronous and follows the preset (decision 7). Agents never
wait on a signature: every request returns at once and the work goes on.

- **never** (vibe): nothing asks for one. Officialness is the GitHub
  identity in CI. `war sign release <tag>` says so (`release.nothing-asked`)
  and still names any act waiting.
- **release** (team, and the default where no preset is set): one sitting,
  one signature.
- **merge** (regulated): the PR gate wants a signed Warrant, so each is
  signed as it merges.

```bash
war sign release v1.2.0               # the request: every act waiting, the one batch, who signs
war sign release v1.2.0 --dry-run     # the batch judged as the signature would judge it
war sign release v1.2.0 --ssh-sign --as Ada
```

`war sign release` lists every act `war sign --list` shows, drafts and
judges the batch exactly as `war sign --batch` would (OW-WAR-0072), and
names the acts that sign alone (a SAS acceptance, a gate invalidation) or
in a second batch (another signer or role). The request writes nothing,
asks no key, and runs `[notify]`. With `--ssh-sign` it is that batch,
signed once, each response's meaning saying "Signed at release <tag>."
Refused: a tag that is not one word (`release.tag`); a batch with nothing in
it (`batch.empty`), and every `batch.*` refusal. Item sign-offs (`war sign
<t-x/i-y>`) and approvals are signed one at a time, not in the release
batch.

## Being told something waits

- **`war next` and `war sign inbox`** list a person's acts: signatures as
  always, and, under a preset, each approval someone asked for (`approve`,
  judged by its own dry run) and, under signing at release with two or
  more acts waiting, the one `war sign release <tag>` that signs them.
- **The PR gate's** summary and comment (above).
- **`[notify]`**, off unless set:

  ```toml
  [notify]
  argv = ["notify-send", "OpenWarrant", "{message}"]
  timeout_secs = 10
  ```

  It runs when an act is handed to a person: an approval or a release
  batch requested (`approval.requested`, `release.requested`), an
  authorization or resolution request emitted (`authorize.requested`,
  `resolve.requested`), a blocking question asked (`question.asked`), a PR
  the gate refused (`pr.approval-needed`). `{event}`, `{subject}`,
  `{message}` and `{command}` are filled in each element, and ride in the
  environment as `OPENWARRANT_NOTIFY_EVENT`, `_SUBJECT`, `_MESSAGE` and
  `_COMMAND`. It is an argv, never a shell string. One that fails, times
  out or cannot start is reported (`notify.failed`, a warning) and blocks
  nothing; the exit code is the command's own.
- **The agent's end-of-session summary.** Under a preset, the Claude Code
  plugin's stop hook shows the person what waits (a `systemMessage` naming
  the first acts `war next` lists), and the openwarrant skill asks the
  agent to say so when it ends a session. Neither holds the turn.

## Readings this implements

Where decisions 4 to 8 leave room, these are the readings chosen, each the
one that keeps ordinary work frictionless:

- "Official at the author's level" is read as: judged with the PR's author
  as the Warrant's author, official when that author's role allows the
  kind or an allowed approver approved it. Approval substitutes for
  formality; it is not required on top of it.
- `formal` is the top kind, so every role may make a formal Warrant alone,
  and a directory Warrant that passes `war check` is always official.
- The regulated preset's roles are stricter than decision 5's defaults
  (admin and maintain need a test; write needs a formal Warrant). Decision
  5's table is the vibe and team presets', and `[roles]` overrides either.
- `vibe` turns the PR gate off: `war check --pr` reports and passes.
- With no preset, `war check --pr` and `war sign release` use the team
  preset's values, since running either is asking for them.
