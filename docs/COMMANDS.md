# The command surface

`war --help` lists eleven daily verbs and, below them, five groups. Every
other command lives in one of the groups. Every spelling `war` has ever
accepted still works, with the same behaviour and the same output; it is
only absent from help (OW-WAR-0148 M12).

```text
Commands:
  init    Set up this repository: openwarrant.toml, record directories, AGENTS.md
  create  Create a Warrant (a ticket) and print its id: a title is enough
  next    What is ready, and whose step it is
  claim   Take an item or a whole Warrant, so no other agent works it
  done    Tick an item done in its Warrant's checklist and release the claim
  add     Add an item to a Warrant, or a test, KPI or milestone
  note    Append a dated note to a Warrant: context for whoever comes next
  edit    Change a Warrant's type, labels, epic or priority
  show    Show a Warrant by its id
  status  Where the work stands, from the records
  check   Check the records deterministically, without any agent
  help    Print this message or the help of the given subcommand(s)

More:
  plan      Shape the work: drafts, records, projections, roadmap, questions
  sign      The human acts: sign, authorize, resolve, correct, amend, attest
  evidence  Prove the work: gates, verification, stage runs, KPIs, prepare
  view      Look at the work: web UI, terminal app, board, lists, timeline
  admin     Set up and maintain: compile, doctor, pins, import, export, hooks
```

A new repository reaches its first done with four of the daily verbs:

```sh
war init
war create "Fix the login redirect"     # prints its id, t-xxxx
war claim t-xxxx                        # so no other agent finishes it too
war done t-xxxx
```

The M12 target was three, as in Beads. `war done` ticks only what you have
claimed (OW-WAR-0147 OBL-003: a done that needed no claim would let two
agents finish one item), and M12 changes placement and help only, so the
claim stays. Plant 132 records the path as it is.

## The rules

- **At most twelve daily verbs.** Eleven today; `war start` (M15) is the
  twelfth. `help` is clap's own and is not counted. Plant 130 counts them,
  and every group's members.
- **Every earlier spelling is a hidden alias.** `war board` and `war view
  board` parse to the same clap variant and run the same code, so their
  output is the same bytes. There is no deprecation warning on stdout or
  stderr: scripts and plants compare output. Plant 131 runs each one both
  ways and compares the bytes.
- **The envelope's `command` field does not move.** Each command keeps the
  value it always printed, under either spelling: `war view warrants
  --json` and `war tickets --json` both say `"command": "tickets"`. A new
  spelling has no value of its own; it prints its leaf command's. The one
  new command, `war view timeline`, is `war status --timeline` and says
  `"command": "status"`, as that one does.
- **Messages name the new spellings.** Remedies, hints and suggested
  commands say `war admin compile`, not `war compile`; a test parses every
  one against the real clap tree and refuses a hidden spelling. Text in a
  file a resolved Warrant pins (`war admin pins --resolved-only`) cannot be
  edited and keeps the earlier spelling, which still works.
- **MCP tool names do not change** (`war_create`, `war_next`, …); only their
  descriptions name the new spellings.

## Where every command went

The inventory below is every top-level spelling of `war` at 9dc413e2 (86
commands, three aliases), with its home now. "Envelope" is the `command`
value of the `--json` envelope, the same under either spelling.

### Daily verbs (top level, listed)

| Spelling | Envelope |
|---|---|
| `war init` | (prints lines; no envelope, as before) |
| `war create` | `create` |
| `war next` | `next` |
| `war claim` | `claim` |
| `war done` | `done` |
| `war add` | `add` |
| `war note` | `note` |
| `war edit` | `edit` |
| `war show` | `show` |
| `war status` | `status` |
| `war check` | `check` |

### `war plan …`

| New spelling | Earlier spelling (hidden) | Envelope |
|---|---|---|
| `war plan "<sentence>"`, `--proposal`, `--apply`, `--records` | the same | `plan.request`, `plan.validate`, `plan.interview`, `plan.apply` |
| `war plan new` | `war new` | `new` |
| `war plan promote` | `war promote` | `promote` |
| `war plan render` | `war render` | `render` |
| `war plan impact` | `war impact` | `impact` |
| `war plan model` | `war model` | `model` |
| `war plan state` | `war state` | `state` |
| `war plan roadmap` | `war roadmap` | `roadmap` |
| `war plan types`, `war plan types add` (M18, new) | `war types` (hidden, as every group member) | `types`, `types.add` |
| `war plan type` (M18, new) | `war type` (hidden, as every group member) | `type` |
| `war plan frontier` | `war frontier` | `frontier` |
| `war plan questions` | `war questions` | `questions` |
| `war plan ask` | `war ask` | `ask` |
| `war plan answer` | `war answer` | `answer` |
| `war plan answers` | `war answers` | `answers` |

### `war sign …`

| New spelling | Earlier spelling (hidden) | Envelope |
|---|---|---|
| `war sign <target> …` (`--ssh-sign`, `--list`, `--dry-run`, `--batch`, …) | the same, unchanged | `sign` |
| `war sign authorize` | `war authorize` | `authorize.request`, `authorize` |
| `war sign resolve` | `war resolve` | `resolve.request`, `resolve.dry_run`, `resolve` |
| `war sign correct` | `war correct` | `correct.request`, `correct` |
| `war sign amend` | `war amend` | `amend` |
| `war sign attest` | `war attest` | `attest`, `attest.custody` |
| `war sign standing` | `war standing` | `standing` |
| `war sign sas` | `war sas` | `sas`, `sas.accept.request` |
| `war sign inbox` | `war inbox` | `inbox` |
| `war sign authority` | `war authority` | `authority` |

### `war evidence …`

| New spelling | Earlier spelling (hidden) | Envelope |
|---|---|---|
| `war evidence record` | the same | `evidence` |
| `war evidence gate` | `war gate` | `gate`, `gate.invalidate` |
| `war evidence verify` | `war verify` | `verify.request`, `verify`, `verify.bundle`, `verify.run` |
| `war evidence prepare` | `war prepare` | `prepare` |
| `war evidence run` | `war run` | `run` |
| `war evidence perform` | `war perform` | `perform` |
| `war evidence submit` | `war submit` | `submit` |
| `war evidence kpi` | `war kpi` | `kpi` |
| `war evidence mark` | `war mark` | `mark` |
| `war evidence document` | `war document` | `document.review` |
| `war evidence eval` | `war eval` | `eval.run`, `eval.verify`, `eval.ordinary` |

### `war view …`

| New spelling | Earlier spelling (hidden) | Envelope |
|---|---|---|
| `war view ui` | `war ui` | (a server; `ui` devices: its own) |
| `war view tui` | `war tui` | (a terminal app; `--json` refused) |
| `war view board` | `war board` | `board` |
| `war view console` | `war console` | `console` |
| `war view watch` | `war watch` | `watch` |
| `war view overview` (alias `progress`) | `war overview`, `war progress` | `overview` |
| `war view warrants` (aliases `tickets`, `ls`) | `war warrants`, `war tickets`, `war ls` | `tickets` |
| `war view ready` | `war ready` | `ready` |
| `war view prime` | `war prime` | `prime` |
| `war view timeline` | `war status --timeline` (new name) | `status` |

### `war admin …`

| New spelling | Earlier spelling (hidden) | Envelope |
|---|---|---|
| `war admin compile` | `war compile` | `compile` |
| `war admin doctor` | `war doctor` | `doctor` |
| `war admin pins` | `war pins` | `pins` |
| `war admin preflight` | `war preflight` | `preflight` |
| `war admin diff` | `war diff` | `diff` |
| `war admin journal` | `war journal` | `journal` |
| `war admin deliver` | `war deliver` | `deliver` |
| `war admin dispatch` | `war dispatch` | `dispatch` |
| `war admin dispatch-bundle` | `war dispatch-bundle` | `dispatch-bundle` |
| `war admin commit` | `war commit` | `commit` |
| `war admin heartbeat` | `war heartbeat` | `heartbeat` |
| `war admin release` | `war release` | `release` |
| `war admin renumber` | `war renumber` | `renumber` |
| `war admin agents-md` | `war agents-md` | `agents_md` |
| `war admin import` | `war import` | `import` |
| `war admin export` | `war export` | `export`, `export.progress`, `export.progress.verify` |
| `war admin migrate` | `war migrate` | (prints lines, as before) |
| `war admin archive` | `war archive` | `archive` |
| `war admin bridge` | `war bridge` | `bridge` |
| `war admin host` | `war host` | (its own protocol, `oh.war/liminal-v1`) |
| `war admin sdk` | `war sdk` | (its own envelope, as before) |
| `war admin mcp` | `war mcp` | (JSON-RPC on stdio) |
| `war admin kf` | `war kf` | (prints the server's answer) |
| `war admin telemetry` | `war telemetry` | `telemetry` |
| `war admin bonsai` | `war bonsai` | (prints the evidence document) |
| `war admin blut` | `war blut` | `blut` |
| `war admin projects` | `war projects` | `projects` |
| `war admin update` | `war update` | `update` |
| `war admin version` | `war version` | `version` |
| `war admin schemas` (built with `--features schema`) | `war schemas` | `schemas` |
| `war admin merge-ticket` | `war merge-ticket` | `merge-ticket` |

### Top level, hidden, no group

| Spelling | Why |
|---|---|
| `war __release-check` | started detached by the release notice; never typed |
| `war` alone | the terminal app, as `war view tui` |

## Choices, and why

- **`ready` is folded into `next`.** `war next` already lists the ready
  items of light Warrants first, then the acts waiting on an agent or a
  person, so a newcomer needs one verb for "what now". That keeps the
  daily list at eleven with room for `war start`. The ready list alone is
  `war view ready`; `war ready` still works.
- **`prime` is under `view`.** It is the arriving agent's read, named in the
  instruction block `war admin agents-md --block` writes; it does not fit in
  the twelve without dropping a verb a person types daily. The block now
  says `war view prime`; a block written earlier says `war prime`, which
  still works.
- **`war sign` is both the signature and the group.** clap tells them apart
  cleanly: the group's members are matched only as the first word after
  `sign` (`args_conflicts_with_subcommands`), and no sign target (an alias,
  a SAS version, `<alias>/<D-id>`, `roadmap`, `<gate>@<version>`,
  `standing:<id>@<rev>`, `recover:<id>`, `t-…`) is ever a member's name. So
  `war sign <alias> --ssh-sign` works exactly as before, and `war sign
  --as alice authorize` still treats `authorize` as a target, as it always
  did. No other name was needed.
- **`war plan` is both the drafting request and the group.** The same rule:
  a member's name is matched only as the first word after `plan`. A
  one-word request that is also a member's name (`war plan model`) runs the
  member; `war plan -- model` drafts it.
- **`sas` moves whole into `sign`.** `sas accept` is the human act; propose,
  diff, status and repin prepare or read it, and one group keeps one
  command.
- **`authority` is under `sign`**: who may sign is itself a signed change.
- **`inbox` is under `sign`**: it lists the human acts waiting.
- **`mark` and `document` are under `evidence`**: the mark is evaluated from
  the evidence on record, and `document review` is what a document gate
  runs.
- **`promote` and `state` are under `plan`**: promoting a light Warrant
  drafts a directory one, and a declared state is part of the type's
  workflow model, beside `model` and `impact`.
- **`release` is under `admin`, beside `heartbeat`**: both manage claim
  leases rather than the work itself.
- **`diff` and `commit` are under `admin`**, beside `compile`: both read the
  compiled projections.
- **`dispatch` and `dispatch-bundle` stay under `admin`**, as planned: the
  agent's stage loop (`war next` hands an agent `war admin dispatch <alias>
  <stage>`) is plumbing a newcomer never types.
- **`merge-ticket` is listed under `admin`.** It was hidden at the top level
  because git, not a person, runs it; `war admin merge-ticket --install` is
  the spelling messages name. The git configuration it writes still calls
  `war merge-ticket`, so clones configured earlier keep merging.
- **`timeline` is new under `view`**: `war status --timeline` under the name
  a reader looks for.
- **`types` and `type` are new under `plan`** (OW-WAR-0148 M18): a type is
  how a document is planned, beside `model`, `render` and `roadmap`.
  `war plan types` lists the types, `war plan types add <pack>` installs a
  pack, `war plan type <file> <type>` adopts a document in place. Like every
  group member they also answer, hidden, at the top level (`war types add
  ops`, `war type <file> <type>`, the spellings the plan's decision table
  uses); messages name the `plan` spellings.

## What still names an earlier spelling, on purpose

Each of these still runs, so nothing here is broken; each keeps its bytes for
a reason that outweighs one more line in the new spelling.

- Files a resolved Warrant pins and no later Warrant governs (`war admin pins
  --resolved-only`): their bytes cannot move without a signed correction.
- Log and banner prefixes (`war ui: act …`, `war ui: pair a device …`):
  operators and plants read them; they name the program, not a suggestion.
- The self-signed certificate's common name, git's merge-driver line
  (`war merge-ticket %O %A %B %P`, already written into clones' git
  configuration) and the generated TypeScript header.
- The Claude Code hooks (`.claude/hooks/`): they may run an older `war`,
  which knows only the earlier spellings.
- The adopt Warrant's atoms and the program SAS template that `war init
  --program` writes: they are record content, not suggestions.
