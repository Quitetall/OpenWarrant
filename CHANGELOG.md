# Changelog

Notable changes to OpenWarrant. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/), with the caveat below.

## [Unreleased]

### Added

- FORMAT-1.0 revision 4, proposed (OW-ADR-0035, `status: proposed`; the editor
  accepts it): new items are masters `docs/work/<id>/<id>-<slug>.md` with `w-`/`d-`
  hash ids, `spec` and `roadmap` are the only fixed ids, human numbers are display
  ordinals, and work orders and documents stand on one four-tier ladder (§4.6).
  `war check` refuses a document `type` that is not its derived tier
  (`format.type-tier`), a `kind` that disagrees with the id
  (`format.kind-mismatch`), a second Roadmap, and the master `spec` beside
  `SPEC.md` (`format.document-duplicate`). Built-in profiles `design` and `guide`.
  `war context` ranks a higher tier first within a distance. Corpus cases 069,
  070 and 174-177.
- `war compile` writes the views of the masters in `docs/work/`: one page per
  document (`docs/pages/<id>.md`), the open work orders by Roadmap phase
  (`docs/ROADMAP.md`) and the closed ones (`docs/HISTORY.md`), numbered from
  `docs/generated/numbers.tsv` or shown by id. Nothing is written for a
  repository without masters. `conformance/views.toml` lists `docs/pages/**`.

### Fixed

- Two `runtime_capture::protected_executable` tests failed on GitHub's `ubuntu-26.04`
  runners, where `/usr/bin/true` links to a multicall coreutils binary that the
  sealed-descriptor check rightly refuses as linked. The tests now use the real
  file and call it as `true`. The check itself is unchanged.

### Fixed

- MCP `war_start` always delivers its complete required context packet in both
  structured content and a text fallback. `json: false` cannot discard required
  rules. Claim-only replies and other tools keep their terse presentation.
- The war skill and MCP instructions use the Ticket/Warrant/Order model and
  explain failed checks, required context, questions and human-signature handoff.
- CLI and MCP record checks share one route for native work at every level,
  legacy records and work read in place. Native Ticket/Warrant/Order IDs now
  receive their actual record checks over MCP.
- MCP report outcomes mark failed or unavailable checks with the tool error
  flag while retaining their named verdicts and diagnostics.

- `war init` and `war admin agents-md --block` wrote the openwarrant block into a
  `CLAUDE.md` that already imports `AGENTS.md` (`@AGENTS.md`), so Claude Code read it
  twice. Such a `CLAUDE.md` now gets no block, a copy written before is removed, and
  `war admin doctor` warns `doctor.agents-block-duplicate` until it is.

### Changed

- Who may answer a question follows the ladder (OW-ADR-0034, superseding OW-WAR-0069's
  human-only rule below an Order): on a Ticket or a Warrant, any actor except the
  asker (`question.own`), recorded with its kind; on an Order, signed text, an
  intake question or `--needs-human`, a person (`question.agent`). With no
  `docs/authority/roles.toml`, a person is whoever is at a terminal with a git
  identity and in no agent session; a register still decides when there is one.
  `war plan answer --as` is optional.
- `war admin mcp` serves seven tools by default (`war_create`, `war_start`,
  `war_done`, `war_status`, `war_check`, `war_context`, `war_note`), with the
  CLI's folds as arguments: `war_status` takes `next`, `ready` and `view`,
  `war_start` takes `claim`, `war_done` takes `release` and `reopen`. The
  default `tools/list` is 4,904 bytes, down from 32,530. `--all` (or `[mcp]
  tools = "all"`) serves all 42 tools. Input schemas are flat and each
  description is one line.
- Other MCP replies use the CLI's terse text: a finding reads `refused (rule):
  ...`, and a nonzero exit ends `exit N, verdict`. `json: true` (or `[mcp]
  structured = true`) returns the `oh.war/report/v1` envelope.
- The AGENTS.md that `war init` writes is 3,157 bytes, down from 9,775. It
  links `docs/agents/legacy-warrant-workflow.md`, which now holds the full
  rules. The managed pointer block is shorter, and so is this repository's
  AGENTS.md.

### Added

- Questions find their answerer (OW-ADR-0034). `war plan note <id> --ask "..."`
  (`--blocking`, `--needs-human`, `--recommend`) asks of the holder of the parent
  work, else the dispatcher, else the creator, else anyone; `war plan note <id>
  --answer Q-001 "..."` answers. The party sees it at the top of `war status`, in
  `war status --next` with the answer command, in `war view prime` and in Mission
  Control; the asker hears the answer once, on its next `war` call or `war start`.
  Both travel as W3 notices, across clones with `[claims] remote`. MCP `war_note`
  takes `ask` and `answer`. Ticket questions live in `docs/questions/<id>/`.
- The formal path reads a single-file Warrant: authorize, evidence record,
  verify, resolve, compile, check, status and pins take `docs/warrants/<alias>.md`,
  with its state in `docs/warrants/<alias>/`. Its contract digest covers the
  document's exact bytes, so an edit after authorization lapses it
  (docs/design/one-format.md).
- `war plan new <title>` writes a master, `docs/work/<id>/<id>-<slug>.md`,
  with a merge-safe hash id (`w-<hex>`). The slug is cosmetic: the item is found
  by id and renaming the file moves no digest. `--format legacy`, `--preset`,
  `--parent` and the old spelling `war new` still write a manifest and atoms;
  `--format 1.0` still writes `docs/warrants/<alias>.md`.
- `war plan expand <id>` moves `docs/warrants/<id>.md` into its directory
  beside its state, byte for byte: the same id and contract, so its signatures
  stand.
- `war done` journals the work session's usage as one `work.usage` event, read from
  the harness's own local record: Claude Code's transcript
  (`CLAUDE_CODE_SESSION_ID`) or Codex's rollout (`CODEX_THREAD_ID`). Only usage
  numbers are read, from what was appended since `war start` marked the session; a
  session with no readable record is journalled `"unknown"` with its reason, never 0.
  `war done --usage <file|->` reports it instead; `[usage] record = false` turns it
  off. Provider data, unverified: it gates nothing.
- `war status --usage` (and `--json`, `oh.war/usage/v1`): recorded tokens by ticket,
  actor, model and day. Mission Control shows each claim's tokens and recent work in
  Watch, and a Usage view in Records. Dollars only from `[usage.prices]` with
  `prices_as_of`, as `~$ est., prices of <date>`, and `STALE` past 90 days; cache
  tokens are priced only with their own rate.
- With `[claims] remote`, every view sees claims held on other machines:
  `war status`, `war view ready`, `next`, `prime`, `tickets` and Mission
  Control show the holder, workspace and lease, marked `elsewhere` (`REMOTE` in
  Mission Control), and ready work never includes them. Read at most once per
  `[claims] remote_ttl` seconds (default 30); a remote that cannot be read is
  `UNKNOWN` (`work.claims-remote-unknown`), never free. No remote, no network.
- `war init --claims-remote <REMOTE>` sets `[claims] remote`, on a new or an
  existing repository. docs/TICKETS.md has a "Two machines" how-to.
- `tools/measure-tokens.sh` measures what war's own words cost an agent.
  Tests and plant 230 fail if the default `tools/list` grows past 6,400 bytes
  or the init AGENTS.md grows past 4,000.

### Fixed

- A change pushed to the default branch from one clone now reaches the claims
  it concerns in another clone, once, on that clone's next `war` call, even
  when the first clone held no claim watching it.

### Changed

- Views are written on main only. Pull requests carry masters; fast CI refuses
  one that adds or edits a view (`views.pr`) or touches a resolved
  Warrant's projection (`views.record`), and checks the views compiled fresh in
  a scratch worktree. The Views workflow compiles and commits them to main
  with a deploy key, appends new ids to an append-only number index, and
  reports `views: not published` until the owner's setup (docs/STORAGE.md) is
  done. Fast CI also runs on `merge_group`. One list: `conformance/views.toml`.

## [1.0.0-alpha.7] — 2026-11-07

### Added

- `war status --json` (`single_file`), `war status --verbose` and `war plan
  frontier` list single-file Warrants and Orders.

- The MCP tool `war_create` takes `sign`, as `war create --sign` does.

- `-I` is the short form of `--interactive` on all six commands. `-i` stays
  `--item` on `create`.

- `--interactive` on `init`, `create`, `start`, `done`, `status` and `check`. A
  caller with no terminal, or under `--json`, is refused by name and never
  prompted.
- `war status --next`, `--ready` and `--view`, `war start --claim`, and `war done
  --release` and `--reopen` are the commands they fold, envelope included. The
  old spellings keep working. A retiring top-level spelling says so on stderr at
  a terminal, or anywhere with `OPENWARRANT_DEPRECATIONS=warn`; a pipe sees
  nothing. `docs/COMMANDS.md` has a parity table, and a test holds it to the
  command tree.
- `war init --ci` writes one opt-in workflow running `war check --changed` and
  the repository's tests on each pull request. `--test` names the tests;
  otherwise they are found from what the repository holds.
- `war create --sign` makes an Order, one document like the other levels.
- `war check`, `war status`, `war view show` take a single-file Warrant's alias,
  and `war view next` lists open single-file and read-in-place work under
  `elsewhere`.
- Work across repositories (`docs/INTERFACES.md`). `[repos.<name>]` names a
  submodule or sibling, and a pin is read from its gitlink, a `pin` field or
  `Cargo.lock`. `war context hpr:CL-step` and `war view prime hpr:t-abc` read from
  the other repository's objects at that commit; what can't be read is UNKNOWN.
  `war create --depends-on` holds an item out of `ready`, and prime says *done
  upstream, not pinned here* until the pin moves. `war done` warns on a Ticket and
  refuses a Warrant or an Order. `war check` counts open clauses and flags late
  ones, `--clauses` runs them, `--record` writes `docs/interfaces.lock`, and
  `war status` shows drift in one line. `--change breaking|compatible --clause`
  makes a change request, and `war plan impact` lists claims on both sides. Notices
  follow the pin, and reach other clones through `refs/openwarrant/notices`.
- `war plan types add interfaces` adds an `interface` document type: numbered
  clauses two sides build against, each saying how it's verified, with an open
  value declared `tbd` or `tbr`. `docs/INTERFACES.md` shows it.
- `war view prime --prompt <text|->` and the plugin's UserPromptSubmit hook put
  the brief of the item a prompt names in context before the first turn. A
  prompt names an item by id, a capital code (`T8`), a double-quoted phrase or
  its whole title, through the lookup `war start` uses. Several matches add one
  line and no brief (on the first prompt only); none adds nothing, nor does a
  Warrant you hold. It reads, claims nothing, stays within 6,144 bytes with any
  cut declared, adds the same brief once a session, respects `[tickets]
  session_start = false`, and a hook failure never holds a prompt.
- `war admin agents-md --block --ready <N>` lists up to five ready Warrants in
  the block of an `AGENTS.md`, for a harness with no prompt hook. Only running
  it refreshes the list: `war` never does so by itself, a plain `--block` keeps
  it, and `CLAUDE.md` gets none. `--ready 0` removes it.

- Live updates to running agents, on one machine (t-3c16c W3): `war start` keeps
  the claim's source manifest beside it, every `war` write and `check --changed`
  compare it with the default branch, and the affected agent's next `war` call
  prints one line, once (`notes` under `--json`). `war start` then prints only
  what changed and records it as seen; `war view prime` keeps the line until
  then. `war plan impact` names the claims a change reaches. See
  [Tickets](docs/TICKETS.md#when-something-changes-under-you).
- `war done` holds a change and a widening to the item's level (t-3c16c W3).
  A Ticket warns and closes. A Warrant is refused until the change is seen and
  held, then runs its trigger on the new state; a widening of its `paths`,
  `x.tools` or `x.commands` since `war start` is flagged and journalled. An
  Order is held the same, and a widening or a change to what was signed is
  refused until a human signs a new revision. What cannot be read answers
  `UNKNOWN` on a Warrant or an Order and warns on a Ticket. An approval's
  statement now binds what the item declares; an item that declares none reads
  as it did, and an approval of one that does goes stale once.

- `war create --test "<command>"`, repeatable, and `tests` on MCP `war_create`:
  a Warrant can be given its test in the command that makes it. It is the same
  `## CHK-…` section `war plan add --test` writes, and `war done` runs it and
  finishes the work only on a pass. A test that cannot be written is refused
  before anything exists. Not with `--legacy` or `--records`.
- Any ticket can declare the tools and commands its work may run, as well as
  its paths: `war create --path 'src/**' --tool Read --command 'cargo test'`.
  `war start` turns them into session permissions. They never change how a
  ticket ends, and none declared means no limit.
- `war context` answers what used to end in a refusal, so an agent no longer
  falls back to reading a whole file (`oh.war/unit-context/v1`). A long
  document gives its units that fit, ranked, with every cut declared. A unit is
  named as `COMBAT.md#friendly-fire`, and `## Heading {#id}` keeps that id when
  the heading is reworded. Words (`war context "friendly fire"`) and code paths
  find the units that match, locally and with no model. `--strict` refuses a
  cut and `--full` gives everything. An explicit `--budget` still refuses as
  Format 1.0 section 10.3 says, until that text changes.
- `war context` keeps its compiled graph under the git common directory: on
  this repository a repeat call takes about 170 ms instead of 6 s or more.
  `OPENWARRANT_NO_CACHE` turns it off.

### Changed

- A notice now reaches a claim that depends on, or cites, an item in another repository
  (`hpr:t-abc`), as it did for a clause. So a Warrant that depends on one now holds at
  `war done` until the change is seen, then until the pin holds it.

- `war plan promote` adds a field in place and writes no document: a sign-off
  item, so the work is an Order, or with `--test` a test, so it is a Warrant.
  The id stays. Drafting a directory Warrant from a ticket, which is what it
  did before, is `war plan new --from <ticket>`.

- `war init` no longer writes `schemas/`: the schemas are in the binary, and an
  archive of a repository with no copy retains the binary's pack.
  `war init --schemas` writes a copy, on a new repository or one already set up.

- Bare `war status` is a few lines of counts: the SAS revision, Warrants,
  open tickets, ready work, acts awaiting a signature and blocking questions.
  On this repository, 37,841 bytes to 180. A figure that cannot be read prints
  `?`, says why on stderr and exits 2. `war status --verbose` is the old
  output, byte for byte; `--json`, `--timeline`, `--pending` and `war status
  <id>` do not change. A script that read the projection from bare `war status`
  needs `--verbose`.
- `war view prime`, `war view tickets` and the terse `war status` read all work
  through one list (`crate::work`, `docs/design/one-view.md`). An item's level,
  Ticket, Warrant or Order, is derived from its fields and never stored. `prime`
  marks a non-default level (` · Warrant`) and adds one line counting open work
  outside the ticket store; `war status` adds `Open by level:`; `--json` rows
  carry `level`. `war view warrants` now lists single-file Warrants, which no
  list showed before.

### Fixed

- `war done <ticket> --check` was refused with `ticket.check-nothing` when the
  ticket's test came from `war plan add <ticket> --test`. As docs/TYPES.md says,
  that test applies to every item, one added later included.

- `war create` printed `created docs/tickets/t-xxxx/` for a document at
  `docs/tickets/t-xxxx.md`. It prints the document's path.

- The AGENTS.md `war init` writes teaches the six commands and the three levels,
  and no longer `war note`, `war add` or `war claim`.

- A finished Order never reached its signer. `war done` on one with a signature
  item was refused and ticked nothing, and no list named it. It now ticks the
  agent's items, runs any test, records the note, leaves the signature open and
  exits 0, saying `t-x waits on a person: \`war sign t-x/i-y --ssh-sign\``.
  `ticket_state` reads `awaiting_signature`. `war sign --list`, `war status`,
  `war view next` and the signing queue name it, and the signature closes it. A
  failing test queues nothing, and an agent still cannot tick the signature.

- `war plan add` on an Order put the new item under the signature milestone,
  which made an agent's work a person's. It now goes with the other items.

- A note from Mission Control on work held in a worktree was written in the app's own
  checkout, on another branch, and the holder never read it, while the form promised
  its next `war` command. It is now written in the holder's workspace, on a ticket or
  any of its items, and the form names the commands that show it. An answer now says
  plainly that it reaches a worktree only once it is merged across.

- Dispatch's detached launcher failed with `[go] allowed_acts = "declared"`, which
  writes no session files and so left no directory for `.openwarrant/start.log`. The log
  is a session file now, so it's no longer committed with the work or counted against a
  worktree. A generic harness given by path no longer makes the path the claim's actor.

- `war view tickets --type change-request` was refused as an unknown type.

- Only ids starting `CL-` were picked out of a ticket's text as clauses to watch across
  repositories. Any `name:id` is now looked up in what the interface declares.

- `war done` could disown its own claim. With no harness session variable set,
  `war start` in one terminal and `war done` in another read as two holders,
  and `war done` answered that nobody held the work. With no named session,
  the worktree now decides. A named session still decides by its name.

- A single-file ticket's own sections, such as `## Design notes`, reached no
  brief: `war start`, `war view prime` and the prompt hook showed only its
  preamble, so an agent never saw them. In the adoption study this failed the
  same two items in both reps, each a false completion. Every section now
  reaches the brief. An approval of a ticket with such a section goes stale
  once, because its statement now covers that text.

- A claim whose `git push` to the claims remote timed out no longer says
  "nothing was claimed": the push may have landed. It stays UNKNOWN
  (`work.claim-remote-unreachable`), says the outcome is unknown, and says how
  a claim that landed clears: a retry from the same session, or its lease
  running out. Timed-out ref deletes and lease renewals say the same.

## [1.0.0-alpha.6] — 2026-11-05

### Added

- **Mission Control preview (`war tui`)**, not a finished workspace. This entry
  first described it as finished; what follows is what shipped.
  - `Pane::Observe` (`o`): Herdr agent lists and transcripts, read over Herdr's
    Unix socket (`~/.config/herdr/herdr.sock`), fleet and worktree counts, and
    the open questions list work. Token and dollar figures are fabricated
    constants, not telemetry. `LEASES: fresh` is a fixed label: lease, scope and
    collision details do not show, and there are no writer heartbeats. "Critical
    path" only means priority 0 or 1. Milestone bars are not progress: a
    phase's Warrant counts as done unless the roadmap lists it as open work.
  - `Pane::Orchestrate` (`O`): batch SSH signing works (`[s]`). Create,
    dispatch and answer do not work from the TUI, and its list of models
    launches nothing. Use `war create`, `war start --worktree` and
    `war plan answer`.
  - The no-Herdr fallback pane does not work.
  - Terminal-only guard (`tui.no-tty`), zero signing key material in UI memory, strictly shelling out to child `war sign --ssh-sign` processes.
- Herdr client (`openwarrant_cli::herdr`): reads agents and transcripts from a
  running Herdr, default session only. Herdr is optional.

### Changed

- An agent finds and closes the work it was asked for without lookups, under
  any harness. The adoption study (2026-10-29) traced Claude Code and opencode
  (DeepSeek V4.1 Flash, GLM 5.3 Flash) sessions spending two to three turns on
  turning "do item T8" into a ticket id, and reading MCP answers twice.
  - `war start`, `war done`, `war view prime`, `war note` and the other ticket
    commands take words of a Warrant's title or labels where they took an id:
    `war start T8`, `war start monthly budget`. Open Warrants are searched
    first; words that fit several are refused `ticket.ambiguous` with each id,
    and words that fit none `ticket.unknown`. MCP: `target` (or `ticket`, `id`).
  - The MCP tools refuse a parameter they do not know (`unknown field`, naming
    the ones they take) instead of ignoring it: an ignored `ticket` made
    `war_start` take the next Warrant instead of the one named.
  - A ticket tool's MCP text is what the CLI prints (a brief in Markdown, `done
    t-x (n/n)` in one line); the `oh.war/report/v1` envelope stays its
    structured content. A `war_start` answer drops from about 5 KB to the 3 KB
    brief, a `war_done` from about 800 bytes to one line.
  - In a linked git worktree the default actor is the performer named for its
    worktree (`claude@w2`): agents side by side in worktrees share the claims,
    and under one name the second resumed the first one's Warrant as its own.
    `--as` and `OPENWARRANT_ACTOR` still name the actor outright.
  - `war_answers` says it is for a Warrant's stages; a light Warrant's answers
    are its notes, which `war_start` already shows.

- The ticket loop an agent runs is two calls, and what it reads is only the
  work it takes. The adoption pilot (2026-10-22) measured alpha.5's loop at 47%
  more tokens than a plain `CLAUDE.md` for the same items: the agent read the
  backlog three times over (`war view prime`, `war next`/`ready`, `war context`)
  and ticked each item with its own call.
  - `war start` takes the next ready Warrant whole and prints its brief: the
    description, the open items, every note, what it depends on, and the
    command that closes it. Holding one already, it prints that one again, so an
    interrupted session resumes. `war start <id>` takes that one. The worktree
    and harness launch (M15) is `war start --worktree <id>`. MCP: `war_start`.
  - `war done <ticket>` ticks every open item in one write, or none: each is
    judged as an item's done is (claim, blockers, checks, least tick), and an
    item that fails refuses the whole. `war done` with no id closes the one
    Warrant you hold (`ticket.nothing-held`, `ticket.several-held` otherwise). A
    note is one note on the Warrant.
  - `war view prime` is one line per open Warrant (progress, who holds it, what
    it waits on) and done work by id; `war view prime <id>` is one in full, now
    with a Format 1.0 ticket's journal notes. On an 11-Warrant repository:
    20,736 bytes to 1,576.
  - `war view ready` and `war next` print one line per ready Warrant with
    `war start <id>`, not every item (4,481 and 7,778 bytes to 282 and 558);
    their JSON keeps a row per item.
  - The managed block, the AGENTS.md template, the plugin's skills, the MCP
    instructions and the docs teach `war start` / `war done`. Plant 186 holds
    prime, ready and start to a byte budget.
  - Fewer turns for the same work: the instructions say to run `war start`
    with the first look at the code and to close with the last test run
    (`<tests> && war done --note "..."`, which closes only when they pass).
    Run alone, both work as before, as do `war claim <item>` and
    `war done <item>`.
  - On by default (`[tickets] session_start = false` turns it off): the
    plugin's SessionStart hook runs `war view prime --session`, which shows
    the agent the Warrant it holds (a resumed session needs no call to find
    it), or the open list and how to take the next. With nothing open it
    prints nothing and the hook adds nothing.

## [1.0.0-alpha.5] — 2026-10-20

### Added

- **OpenWarrant Format 1.0, draft 3 (revision 2)** (`docs/format/FORMAT-1.0.md`):
  checks and KPIs, tick levels, specifications, profile data and ASCII
  headings. The conformance corpus has 206 cases; `war` and the independent
  Python reader pass every one. OW-ADR-0032, Format 1.0 as the standard, is
  accepted; the edition itself is not frozen.
- Working-form Warrants appear in `war view overview`, the JSON snapshot, the
  offline HTML and the local progress app, from the same corpus state as
  `war next`. Tick evidence and milestone minima show beside completion, not
  as it: a checked item whose proof is below a configured minimum stays
  checked and the unmet minimum is shown.
- Checks and triggers under Format 1.0 draft 3 (FORMAT-1.0 §12). A Format
  1.0 ticket's `check` and `kpi` records run through one runner: `war check
  run [<alias>|<alias>/<CHK-id>]` (the `manual` trigger), `war done` (the
  `done` trigger, each check's default: the item is ticked `observed` only
  when its gating checks pass, and a fail or an UNKNOWN run ticks nothing),
  `war check fire <trigger>` (`commit`, `paths:<glob>`, `pr.merged`,
  `ci:<name>`, `schedule:<cron>`; what a hook and a CI step call), `war
  check install-hook` (git's `post-commit`; a hook that is not
  OpenWarrant's is refused, `check.hook-exists`), `war view watch --checks`
  (fires `commit` as HEAD moves and `schedule:<cron>` in UTC) and the
  action's `fire` input. Each firing records `trigger.fired` (the check
  pinned at its revision, with the subject) and `check.run` (verdict, the
  check's revision, each evaluated item's `[id, revision]`, the output's
  sha256, the output kept in the evidence directory); a firing already
  recorded runs nothing. `war check evaluate` applies recorded runs: a
  pass on the check's current revision ticks each open item whose revision
  it names at `observed` (`work.item_done` with `runs` and `episode`) and
  raises a done one below it (`work.tick_raised`); a fail makes the check
  `failing` (provenance `recorded`) and, with `reopen: true`, reopens each
  done item it names (`work.item_reopened` with `episode`); an UNKNOWN run
  is `check.unknown` and ticks nothing. A done's `runs` naming a passing
  `check.run` back `observed`.

- Work tracking under Format 1.0 draft 3 (FORMAT-1.0 §11). A Format 1.0
  ticket (`docs/tickets/<id>.md` or `<id>/ticket.md`) is worked by `war
  claim`, `done`, `add`, `note`, `admin release` and the new `war admin
  reopen <item> --reason` (`work.item_reopened`), its journal written in
  the `v: 2` envelope (`document_uuid`, an object payload, RFC 8785 lines,
  `work.*` types). A directory ticket keeps writing `v: 1` and `ticket.*`;
  a reader reads both, in one journal, and a `ticket.*` type as its
  `work.*` name. `war check` now reads every ticket's journal
  (`work.journal-torn`, `-malformed`, `-rewritten`) and warns of a tick
  level the journal does not back (`work.tick-unbacked`). The kernel state
  set gains `blocked`, `claimed`, `failing` (from a recorded `check.run`)
  and `partially_accepted`, appended to `oh.war/model/v1`'s states.
  `war next --at <RFC 3339>` answers as of a given time. Under draft 3
  revision 1: a Format 1.0 ticket with no items is claimed and done as
  itself (its done a `work.item_done` naming it, no line added); an item
  is `blocked` while its document's `depends_on` is not done; a `part_of`
  child counts as an item of its parent's phase; a journal line lacking a
  member, with a `v` other than 1 or 2, or whose key does not match is
  `work.journal-malformed`, a `v` above 2 UNKNOWN `work.journal-newer`,
  and only a committed line missing, changed or reordered is
  `work.journal-rewritten`; of two conflicting events the later
  `occurred_at` (then the greater `id`) wins. `[claims]
  claim_lease_minutes`/`claim_ttl_minutes` and `[next] prior_secs` are
  read first, the old `[tickets]`/`[go]` keys after. The estimate median
  of an even count is the lower middle value, and a missing `created_at`
  sorts last.

- `war plan states [--at <RFC 3339>]` (FORMAT-1.0 §11.1, draft 3 revision
  1): each state of the work documents by its family (`phase`,
  `checklist`, ...) and provenance (`computed`, `recorded`, `declared`),
  one `checklist` state per item, with the journal findings it was read
  under; every state of a document whose journal holds a line newer than
  v2 is UNKNOWN and left out (`unknown`). `war claim`, `done` and `note`
  take a hidden `--at` for replaying a recorded history (the conformance
  corpus's tracking cases now run against war). A Format 1.0 document's
  claim, done, reopen and release carry `episode` (§11.2).

### Changed

- `war view prime` is a briefing however much a repository has written: a
  description's first paragraph and a note are cut to one line of at most 280
  characters, only each open ticket's latest note is shown (with how many came
  before and `war view prime <ticket>` for them), and of the tickets done
  before the recent window only the ten latest are named, then
  `and N more: war view tickets --state done`. The JSON keeps every id. On
  this repository: 34 KB to 12 KB.
- AGENTS.md sends an agent to `war view prime` first and `war context <id>`
  for a task's records; `docs/generated/CURRENT.md` (1.88 MB) stays the
  whole-corpus reference to search, not the first read. OW-WAR-0113 AM-004
  amends OBL-002 to say so, authorized as revision 5; plant 69 holds the three
  documents for people to CURRENT.md first and AGENTS.md to prime first.
- `war status` and `war next` do about half the work (debug build, this
  repository: `next` 6.1 to 3.6 CPU seconds, `status` 5.9 to 3.3). Schema
  scans skip files that cannot name a newer major, and verification records
  and journals are decoded once per process while the file is unchanged.
  Hosted and recording runs still read their actual inputs.
- The release bundle carries the full committed source as `sdk-source.tar.gz`
  inside a `candidate-bundle/v2` capsule; the installer still reads v1 and
  bounds the source before extracting it.
- FORMAT-1.0 §11 as the conformance corpus reads it: a Format 1.0
  document's `war note` is one `work.note_added` event `{target, note}`
  and no longer edits the document (`war show` lists the journal's notes);
  the same note from the same actor replays, from another it is refused
  `work.idempotency-conflict` at the line that records it. An unterminated
  last journal line is `work.journal-torn` even when it parses. Journal,
  stale-revision and whole-document claim refusals name the faulty line
  or the holder's claim file. `war done --if-rev` on an item nobody holds
  claims it first (compare-and-set keeps two finishers apart); without a
  base an unclaimed done is still refused. An expired claim no longer
  blocks a claim on its document or item. A done suffix's unknown tick
  level (`[claimed]`) is refused `format.item` (§3.8). A profile file in
  7.5's shape without `format` governs nothing and no longer stops the
  legacy registry: its documents read `format.profile-unknown`, with a
  `profile.format-missing` warning. `observed` is backed by a done naming
  passing `check.run`s by `run` id (§11.4).

- `war next`, `war view ready` and `war evidence go` share FORMAT-1.0
  §11.5's order: due (inherited from what waits on an item), then the
  remaining path (critical path first), then priority, `created_at`, id and
  the item's position. A claim past its TTL whose lease is still live no
  longer leaves its item in the ready set: it is live, and `war claim
  --steal` still takes it. `oh.war/ticket-ready/v1` rows carry `due` and
  `remaining_secs`.

- Rule ids renamed under `work.` (FORMAT-1.0 §13.2), each message naming
  the old id for one release (`(formerly ...)`): `work.stale-revision`
  (`warrant.stale-revision`), `work.claimed-by-other`
  (`ticket.claimed-by-other`), `work.claim-remote-unreachable`
  (`ticket.claim-remote-unreachable`, now UNKNOWN rather than an error),
  `work.journal-rewritten` (`journal.rewritten`), `work.journal-malformed`
  (`journal.malformed`), `work.journal-torn` (`journal.torn-tail`),
  `work.idempotency-conflict` (`journal.idempotency-conflict`). The same
  fact in a `v: 1` and a `v: 2` line is one fact: payloads are compared as
  RFC 8785 bytes.

### Fixed

- A repository with no Warrant (`war init` alone, as ordinary work keeps
  one) is checked and compiled for what it does hold. `war check` reads its
  Format 1.0 documents, records, declared documents and type index, where it
  used to stop at `corpus.empty`: a misspelled frontmatter key checked clean.
  `war admin compile` writes the agent index (`docs/generated/llms.txt`,
  FORMAT-1.0 §10.2) and declared documents, and `war check --generated`
  compares the index. Format corpus case 212 (`bare = true`, run where `war
  init` alone ran) fails before this and passes after.
- A replaced Warrant is no longer shown as current in corpus status or
  offered again by `war next`. Default views leave retired records out;
  `war view overview --all` keeps them, with their currency shown.

## [1.0.0-alpha.4] — 2026-10-16

### Added

- **OpenWarrant Format 1.0, draft 2** (`docs/format/FORMAT-1.0.md`, OW-ADR-0032
  proposed): a single-file Warrant with an optional manifest, a model that
  carries content and a `content_digest` over meaning (RFC 8785), dotted
  extension names carried in `ext`, JSON schemas under `schemas/format/1.0/`,
  and a conformance corpus with an independent Python reader
  (`conformance/format/`). Spec text CC-BY-4.0; schemas and corpus Apache-2.0.
- `war context <target>`: a deterministic, budgeted context packet for an
  agent, and `docs/generated/llms.txt`, an index of every record with its
  state that ticket commands keep current incrementally.
- One-line installs: cargo-binstall metadata, a rendered Homebrew formula
  attached to each release, and pinned GitHub Action references.
- Native runtime receipts, verifier packets and acceptance schemas
  (experimental, beside the unchanged v1 schemas).
- `[independence] single_actor = true` (issue #132): a repository where one
  person holds every role declares it once. It is the same as nine `false`
  dimensions and claims nothing: `war check` replaces the per-run
  `independence.insufficient` WARN with one NOT CHECKED note, `war status`
  reports `independence: single actor (declared)` (`"independence"` under
  `--json`), and the compliance score does not move. Beside a dimension set
  `true` it is refused by name, `independence.single-actor-contradicted`,
  and the WARN stays. No preset writes it.

### Changed

- `war init` with no flags writes the `vibe` preset: no signatures, the PR
  gate reported but not required, and `war done` claims an unclaimed item
  itself. `--no-preset` keeps the old bytes; `--program` writes no preset
  unless one is named.
- The release bundle's source archive leaves out the repository's own
  evidence (`archive/`, `artifacts/`, `docs/evidence/`, and Warrants'
  `implementation/`, `gate-runs/` and `verifications/`). Build inputs,
  standard text, skills and conformance fixtures keep their original paths.
  Complete project history remains in Git; historical repository audits need
  a full checkout.
- Verification-history reads avoid repeated compact-record parsing and stop
  scanning source when changed gate evidence already excludes an old review.
  Matching evidence still reaches every existing binding and independence check.
- Shared preparation gates run once across Warrant scheduling rounds.
- `war sign` (issue #132): a refusal that nobody is eligible names the
  register by the path it resolved, says whether it is absent (pointing at
  the `roles.toml.example` beside it) or present with no one eligible for
  the act, and `war sign --all` stops after the first structural refusal
  (no register, no ssh principal, an unreadable trust source) with
  `sign.sweep-stopped` counting the acts not attempted, instead of repeating
  it per act. Under `--ssh-sign` the signer's principal is resolved before
  the sign screen is drawn, so `sign.ssh-principal` refuses before the act
  is shown.

### Fixed

- Git source checks preserve the real index, retain deleted-source paths when
  files move into excluded bookkeeping, and keep hosted and recorded contexts
  from borrowing other contexts' cached observations.
- `war init --guided` no longer loops at "prepare the first Warrant": the
  sign step is reachable, and declining a signature stops the setup instead
  of asking again.
- The SSH verdict cache is bounded (4096 entries, 90 days unused); pruning
  only ever costs a re-check, never a different verdict.
- Simultaneous `war perform` starts are fenced so exactly one writer runs;
  a checkout is not reused while a retained writer may still run.
- Oversized or ambiguous verifier packets and answers are refused before
  any backend call or write.
- rustls updated to 0.23.46 (RUSTSEC-2026-0346 to 0349).

## [1.0.0-alpha.3] — 2026-10-09

### Added

- One compiled model (OW-WAR-0148 M1). `war plan model` emits
  `oh.war/model/v1`, the records, relations, states and diagnostics every
  client reads from one corpus built once per process.
- Typed records and relations (M3): `## <ID> · <type>` record atoms, a
  closed core set of relation kinds, and `war plan impact <record>`, which
  names what a change affects.
- Fixed kernel states, plus declared states that only refine one (M4):
  `war plan state`.
- Tickets on the record kernel (M5): types, labels, epics, filters and
  search; `war create --issue` reads a GitHub issue, and opt-in
  `[intake.writeback]` comments on and closes it when the work is done.
- Projections declared as data (M6): PRD, architecture view, test plan and
  agent packet render from one set of records (`war plan render`).
- From a prompt to typed records (M7): `war plan --records`.
- `war admin host` (M8) speaks `oh.war/liminal-v1`: pure, over in-memory
  files, with conformance fixtures. The host is trusted for the observations
  it supplies; a missing one fails closed.
- Tickets are Warrants (M10): one id space, `war view warrants`, and
  `war admin import|export` for Beads JSONL, OpenSpec and Spec Kit, plus
  read-in-place `[[adapters]]` that write nothing into those folders.
- Concurrency (M11): claims shared across a clone's worktrees, leases with
  `war admin heartbeat`, `--if-rev` compare-and-set
  (`warrant.stale-revision`), cross-machine claims through a git ref,
  `warrant.alias-duplicate`, `war admin renumber`, and a ticket merge driver.
- Optional parts and the tick ladder (M13): `war add <id> --test|--kpi|
  --milestone`; every tick records how it was earned (claimed < observed <
  independent < signed); `war done --check`; `war evidence kpi run`.
- Presets, roles and official Warrants (M14): `war init --vibe|--team|
  --regulated`; `[roles]` mirroring GitHub levels; `war sign approve` and
  batched `war sign release`; `war check --pr`, shipped as the GitHub
  Action `.github/actions/openwarrant-check`, requires an official Warrant
  at the author's level. Under `--vibe`, `war done` claims for you.
- The native graph executor (M15): `war start <id>` runs one node in its own
  worktree with session-only harness settings; `war evidence go` runs the
  frontier until nothing is ready, with a concurrency cap, budgets, retries,
  and estimates learned from the journal. Executors are pluggable (a local
  harness argv, GitHub Agent HQ, Gas Town via the Beads export); every
  result comes back through `war submit`'s refusals.
- CLAUDE.md and AGENTS.md (M16): `war admin agents-md --block` keeps a small
  managed block; their `##` sections become `instruction` records that
  Warrants can cite and `war plan impact` follows.
- Every development document is a type (M18): `roadmap`, `spec` and `adr`
  profiles read today's stores unchanged; ROADMAP.md is a projection;
  `release`, `incident` and exit-report types in the core; `ops` and
  `quality` packs (`war plan types add <pack>`). Development docs no type
  claims appear as `document` records and count against the score's
  document coverage; `war plan type <file> <type>` adopts one in place.

- The compliance score and the file ledger (OW-WAR-0148 M17; docs/SCORE.md,
  docs/LEDGER.md). `war status` ends with a score from 1 to 1000, a
  scorecard of seven dimensions (commits and merged PRs citing a Warrant,
  the file ledger, typed documents, and the tick ladder's tested, verified
  and approved shares), a level, and next steps; `--json` carries it as
  `compliance` (`oh.war/score/v1`). The weights are published and versioned
  (`oh.war/score-weights/v1`, `war admin score --weights`); a dimension
  that cannot be measured reads UNKNOWN and earns 0. `war admin score
  --in-toto` emits an unsigned in-toto Statement v1 (predicate
  `https://openwarrant.dev/attestation/work-score/v1`, subject
  `gitCommit`), and `--verify` checks one. `war check --floor <rev>`, and
  `war check --pr` when the base says `[score] floor = true`, refuse a
  change that lowers the level (`score.floor`). `war admin compile` writes
  the badge, report page, statement and a per-compile trend under the
  self-ignoring `.openwarrant/score/`, never under `generated/`. The file
  ledger: per-file atoms under `docs/ledger/` (`war admin ledger record`,
  three writer modes, folded past `[ledger] keep` or a token budget),
  compiled into the gitignored `.openwarrant/ledger.jsonl` that `war view
  prime` points to; `war check` refuses a committed JSONL
  (`ledger.committed`) and a malformed atom (`ledger.atom`). `war admin
  ledger import/export` read and write Agent Trace v0.1 records and git-ai
  v3 notes (`refs/notes/ai`). The Action scores the checkout into the job
  summary and its `score`, `level` and `score-dir` outputs. Plants 170-173.
- A document's type chooses its capabilities (OW-ADR-0031, OW-WAR-0148 M2).
  `oh.war/profile/v1` gains `capabilities`, from the closed set `structure,
  links, claims, acceptance, evidence, verification, authorization,
  resolution, stages`, with prerequisites checked by name
  (`profile.capability-prerequisite`, `profile.capability-unknown`,
  `profile.capabilities`), and `satisfied_outcome`, `falsifiable_claims` and
  `standing_coverage`, which replace the profile-name matches in `war sign`,
  resolution recording, standing classes and `war promote`. `delivery`
  selects every capability; `decision` every one but `stages`, so §56.1
  requirement 12 reads "not applicable: no `stages` capability" for a
  decision Warrant and OW-WAR-0071 can resolve. A kind lacking a capability
  is refused the act by name (`capability.absent`), offered nothing for it by
  `war next`, and its rule families are named `capability.not-applicable`.
- `oh.war/manifest/v1` gains `profile_digest` (optional, omitted when absent):
  `war new` writes the sha256 of the profile file a Warrant is composed
  against, and `war check` reports `profile.pin-drift` — a warning unsigned,
  an error once authorized. A manifest without it keeps its bytes, digest and
  checks. `ResolutionChecks` gains `not_applicable`, omitted when empty.

### Changed

- A small surface (M12): `war --help` lists 12 daily verbs (`init create
  next start claim done add note edit show status check`) and five groups
  (`plan`, `sign`, `evidence`, `view`, `admin`). Every earlier spelling
  still works, hidden, with byte-identical output; messages and docs use the
  new ones (docs/COMMANDS.md).

- Ordinary work is ordinary (M9, t-51280). Everything an agent reads says
  first that ordinary coding needs no Warrant and no ticket: the AGENTS.md
  template, the skills, the MCP instructions, and `war next` / `war ready`
  with nothing ready ("nothing tracked; work freely"). Rules for a Warrant
  with a sign-off step live in a section scoped to it, as what the tool
  refuses. `war next` lists an agent's acts before the acts a person signs.
  `war init` asks nothing: without `--namespace` it derives one from the
  directory name, writes no signing setup, and `--guided` is the opt-in
  conversation. The plugin's edit guard acts only in an OpenWarrant
  repository's own trees. Every `sign.*` refusal names what is missing,
  carries a remedy and ends "This blocks only the sign-off, not your work";
  `war doctor` probes the signing setup without signing, `war doctor
  --fix-signing` repairs it at a terminal (writing only absent files), and
  `war doctor`/`war prime` warn `install.version-skew` when AGENTS.md or the
  plugin is newer than the binary. Guarded by plant 101-agent-text (a lint
  over all shipped agent text) and the opt-in `war eval ordinary`.
- Verification bundles are bounded (t-9f7e). `oh.war/verification-bundle/v2`
  carries its estimate over the whole JSON against `[verify]
  max_bundle_tokens` (default 48000). A Warrant over it gets one bundle per
  obligation — the deliverables it lists or names, excerpted by numbered
  lines with whole-file digests, and `obligation_evidence` saying what was
  not carried or was cut — and `war verify --run` calls the verifier once
  per bundle. OW-WAR-0112's single 241k-token bundle, which timed the
  verifier out, is now seven of at most 48k.
- A bundle no longer carries a gate receipt's `working_directory` (t-ade4).
  It was the absolute path of the checkout the gate ran in, so a bundle's
  bytes, digest and token estimate depended on where the repository sat:
  `war eval` measured the same fixture at 3043 and 3042 tokens as a scratch
  program's PID and nanosecond digits varied. The receipt in the tree keeps
  the field, and its `receipt_digest` still covers it.

## [1.0.0-alpha.2] — 2026-09-19

### Security

- An authority record is believed because a human signed it, not because the
  file says so. Every authorization, resolution, correction and SAS acceptance
  is read through `authority_check`: a response for that act, over that digest,
  whose `.sig` `ssh-keygen -Y verify` accepts as the principal `roles.toml`
  binds to the actor, who must be `human` there. In 1.0.0-alpha.1 a hand-written
  `authorization.toml` naming anyone as authorizer passed `war check` with zero
  errors and satisfied §56.1 requirement 1.
- §56.1 requirement 1 ("the EXACT authorized Contract Revision") requires that
  signature, so a forged record cannot carry a Warrant to a resolution.
- `war dispatch` refuses a contract no human signed (`dispatch.unauthorized`);
  `--prototype` on `dispatch`, `run` and `perform` compiles anyway and records
  `prototype://unauthorized` as the authority acted under. `war console`,
  `war mcp` and `war perform --all` never pass it — §27.2 leaves an agent no way
  to decide it may work under no authority.
- Eleven planted violations exercise it, including a signature replayed onto
  another Warrant and a principal no key answers for. `docs/THREAT_MODEL.md`
  entry 11 states the residual: an agent can still WRITE a record; the refusal
  is on the read path.

### Added

- `war sign` offers a recorded-but-unsigned act as an ordinary pending row for
  all four act kinds. The act repeats the record — outcome, reason, kind and
  revision number all come from it — so a signature can only say what was
  already said. Reported as `*.signature-supplied` and journalled as
  `*.signature_recorded`, never as a second `*.recorded`.

### Fixed

- `war sign --all` no longer aborts on one act's failure, and an act needing a
  decision the batch cannot make (`--outcome`, `--kind`, `--adr`) is a
  `sign.needs-decision` warning naming the command rather than an error.
- One response filename per act kind. An authorization and a resolution of the
  same Warrant both bind `contract_digest`; sharing `<alias>.response.toml` made
  whichever signed second retire the first.


- Add candidate portable Dispatch transport and offline capability membership
  refusal through `war dispatch-bundle` and the SDK. Exact context bytes travel
  with their commitments; no execution, authority or semantic closure is claimed
  (OW-WAR-0023; `docs/cli/dispatch-bundle.md`).

- Add partial read-only `war preflight <alias>` with all six SAS readiness
  dimensions. Unobserved checks remain UNKNOWN and block readiness; no execution
  permission or recorded state changes (OW-WAR-0011, remaining scope documented
  in `docs/cli/preflight.md`).

- Add read-only `war inbox`: human-waiting Warrants, blocking hotline questions,
  known/unknown transition ages, and a proposed versioned JSON result. Existing
  signing policy, records and schema pack remain unchanged (OW-WAR-0070).

## [1.0.0] — released when the `v1.0.0` tag exists; until then this is the plan's ledger

**The protocol is stable from 1.0.0.** Every `oh.war/*/v1` record shape, every
`DigestDomain` string, the RFC 8785 canonical form and the schema pack
`openwarrant-schema-pack 0.2.0` are frozen: a change is additive with
`#[serde(default)]`, or it is a `v2` with a migration. Digests minted by 1.0.0
are reproducible by every later 1.x. `docs/COMPATIBILITY.md` states the rules;
`docs/roadmap/RELEASE_1_0.md` maps SAS §99's twenty-five criteria to what
proves each. Two acts precede the tag and are the owner's: the relicense to
Apache-2.0 (`RELICENSING.md`) and `war sign 1.0.0 --ssh-sign` accepting the
SAS revision; `release.yml` refuses to publish crates under any other licence.

Delivered under the 1.0 plan (each entry one slice, each slice one commit):

- The correction act (OW-WAR-0064, OW-ADR-0012): a resolved Warrant's pinned
  file moves for a reason through `war correct` + `war sign <alias>/<D-id>`,
  never silently and never by regenerating the record.
- `--json` on every command: one `oh.war/report/v1` envelope, errors included
  (§76.4). `war pins` (what a resolution pins) and `war next` (whose act is
  next; never a signing act for an agent).
- `AGENTS.md` is a template `war init` writes; `war agents-md` rewrites it.
- Draft Proposal v2 (OW-ADR-0013): operations carry payloads; `war plan
  --draft` runs a configured drafter, `--apply` creates the Warrant and records
  request, proposal, drafter run and pipeline under `plan/`.
- An async runtime for the MCP transport only (OW-ADR-0014); `war mcp` serves
  every read, request half and agent-permitted write over stdio with no
  signing or ingesting tool registered.
- A Claude Code plugin: the `openwarrant` skill with references, the MCP
  server, a `PreToolUse` pin guard and a `Stop` check on `war check`.
- Attestations (OW-ADR-0015): every ssh-signed act leaves an in-toto
  Statement in a DSSE envelope beside its record, signed with the same key
  under `oh.war/dsse`; `war attest` verifies signatures and subject digests,
  and `cargo xtask gate` runs `war attest --all`.
- `[project] performer` in `openwarrant.toml` (default `claude`), never a
  flag; `authorize` and `resolve` refuse an `effective_time` that is not RFC
  3339 UTC; `roles.toml.example` shows a second human signer, and two
  eligible signers is a question (`sign.who`), a duplicate principal a refusal.
- `war init --program <name>`: a SAS the tool reads (§6.10, §98, §106), the
  authority examples, the `war check` gate, and a first Warrant with real
  atoms; `war check` on the scaffold exits 0. `QUICKSTART.md` and
  `docs/EXAMPLES/01-code.md`.
- Rules in pinned files, landed as one correction batch: `roadmap.status-claim`
  (a hand-written **resolved** is refused), `roadmap.wrong-namespace` (a
  `roadmap://` ref names this program's §98), `sas.effective-time`; the
  corpus-status fallback namespace comes from `openwarrant.toml`; the Dispatch
  compiler keeps a wrapped bullet whole (dispatch digests move accordingly).
- `war watch`: the pending set once (`--once`, with `--json`) or as a live
  diff of what appears and what is signed away; `--notify-send` raises a
  desktop notification. A poller on the record trees, no new crate.
- `docs/sas/generated/NORMATIVE.{md,json}` (`oh.war/sas-normative/v1`): every
  SHALL / SHALL NOT / MUST / SHOULD / MAY sentence of the SAS with its section
  and the document digest, compiled by `war compile` and drift-checked by
  `war check --generated`; agents read it instead of the document.
- `CORPUS_TIMELINE.json` (`oh.war/corpus-timeline/v1`: every journal event,
  sorted, with a per-day histogram) and `CORPUS_PENDING.json`
  (`oh.war/corpus-pending/v1`: the human acts awaiting a signature, with the
  command for each), compiled, drift-checked, canonical, published by Pages;
  `war status --timeline` / `--pending`.
- The projection contract (`docs/PROJECTION_CONTRACT.md`): `CORPUS_STATUS.json`
  Warrants carry identity, contract revision and digest, obligations with
  dispositions and gates, deliverables with digest state and correction
  chains, gate runs with §44.6 classes, amendments, unknowns and record
  paths; top level carries `repository_url` and `generated_by`. Additive only
  after 1.0; a §84 Knowledge Fabric mapping is documented.
- The progress platform: `CORPUS_STATUS.html` is a single-file, no-framework
  app over the three inlined projections — dashboard, objectives, warrants
  (filters in the URL), warrant drill-down, milestone DAG, requirements,
  timeline, gaps, pending (with the command to copy), evidence; hash router,
  theme, keyboard rows; every count links to its rows, never a ratio. Every
  route renders against the committed data under a node stub DOM in the
  battery.
- `war export --progress <dir>`: the progress bundle (`oh.war/progress-bundle/v1`)
  — the corpus projections, the SAS normative projection and every compiled
  `WAR.json`, copied byte-for-byte with a sha256 manifest and a bundle digest;
  `--verify-progress <dir>` checks one.
- Stage-relevant context (SAS §47.2): a stage declares `context_atoms`,
  `context_sections` (`<atom>#<heading>`), `context_artifacts` and
  `context_external`; the Dispatch's §33 manifest includes the required
  atoms plus what is declared — a declared section narrows its atom as a
  selector — and omits the rest with a true reason. A missing section is
  refused with the headings that exist; `war dispatch --emit-context` writes
  the manifest beside the packet.
- Token accounting (SAS §33.7): every Dispatch carries `tokens` (a bytes/4
  estimate of its selected context, the budget it was compiled under, the
  method id) inside its digest; a stage's `budget_tokens` or `[context]
  default_budget_tokens` (32 000) is the budget, and over budget is
  `dispatch.over-budget` naming the three largest items; each compile is
  journalled as `dispatch.compiled` with its size.
- `war verify --bundle` writes the verification bundle
  (`oh.war/verification-bundle/v1`: request, atoms, deliverable bytes, plants,
  test names, gate runs, prior verifications, token estimate) and `--run`
  hands it to a configured `[verify] verifier_argv`, ingesting the answer
  through the unchanged seam; `docs/RESOLVING.md` describes the hand-off.
- The document work kind: `war document review [alias]` and the gate
  `document.review@1.0.0` — a Markdown deliverable at its digest, citations
  that exist (URLs recorded, not fetched), no placeholder in prose, and an
  `established` verification from someone other than the performer. The
  example is OW-WAR-0065, a research memo on the token approximation;
  `docs/EXAMPLES/02-document.md` walks it.
- The ops / runs work kind: `war run <alias> <stage>` runs a `service` stage's
  gate under the stage's `wall_time_seconds` (or `[run]
  default_wall_time_seconds`), mints a §44.6 receipt whose subject is the
  dispatch digest, and writes a Stage Submission requesting `verify` or
  `block` — never resolution; `war submit` ingests an external submission
  under the same refusals. Gates `ops.echo` and `ops.conformance.plants`;
  the example is OW-WAR-0066, `docs/EXAMPLES/03-run.md`.

## [0.1.0] — 2026-08-24

### Changed — BREAKING for agents that emit Draft Proposals

`DraftProposal` (`oh.war/draft-proposal/v1`) now carries
`#[serde(deny_unknown_fields)]`. A proposal containing any field outside §74.2's
surface is REFUSED at parse rather than silently ignored.

This is deliberate and is §91.8 tests 53 and 54. Previously an agent could send
`enterprise_id` or `authorized_by` and serde dropped them without a word, so an
agent that believed it had allocated an identifier had no way to learn otherwise.
Silently ignoring an attempt to exceed authority is worse than refusing it.

The protocol version is unchanged because the accepted field set is unchanged —
what changed is that fields outside it are now an error. An agent emitting only
§74.2 fields is unaffected.



### Beta opened — 2026-08-21

Alpha closed with all 40 Warrants resolved and OpenWarrant public. Beta is the
act of running it against real systems; nine Warrants (OW-WAR-0041–0049) are
authored against the SAS's own §98 phase-exit criteria, and none is discharged.

Recorded here because it changes how the alpha claim should be read: measured
2026-08-20, of twenty types implementing §40's epistemic classes, §46
independence, §56 resolution and §44.6 receipts, **twenty are referenced by no
code in `war` or the compiler.** They are implemented and unit-tested; no command
calls them. The capability exists — which is what alpha claimed — but `war check`
does not enforce these rules on a corpus, and a reader could reasonably have
inferred it did. OW-WAR-0046 wires them in and is the first beta task.

The README's "Not implemented" section was stale in both directions and is
replaced by three explicit categories: reachable from the binary, implemented but
unreachable, and not implemented at all.

### Added — Phase 1: the file-native compiler

- `war init` — initialize repository configuration and directories (SAS §71.1).
- `war new` — create a draft Warrant, with `O_EXCL` alias allocation so
  concurrent invocations cannot collide (§71.2).
- `war check` — deterministic, agent-free validation with PASS / WARN / UNKNOWN /
  ERROR diagnostics and a readiness verdict (§71.7, RQ-074).
- `war compile` — the full Markdown parent and RFC 8785 canonical JSON, each
  carrying the §17.1 generated header (§71.8).
- `war check --generated` — drift detection against committed projections
  (§17.3, RQ-075).
- Manifest and atom parsing with fail-closed validation for missing required
  atoms, duplicate ordinals, unknown required roles, fabricated enterprise
  identifiers, and composition cycles (§91.2).
- RFC 8785 canonicalization and the fifteen domain-separated digest domains of
  §65, pinned against the official conformance vectors.
- Parent contract-digest verification: a child citing a parent whose contract has
  since changed is reported as resting on a basis it was never authorized
  against (§20.2).
- First-class ADR atoms and a **generated ADR Overview** (§19.6, RQ-021): the
  Appendix A shape with a summary table, lifecycle buckets, and the complete
  decision bodies concatenated as one audit document. Drift-checked like any
  other projection, because §19.7 forbids a manually maintained index.
- `cargo xtask gate` — the aggregate gate of §92: SPDX headers, build, fmt,
  clippy, tests, licenses, and a planted-violation battery that asserts each
  control rejects for its intended reason.

### Decisions

- **OW-ADR-0001** — adopt `serde_jcs` for RFC 8785, chosen by running both
  realistic candidates against the official vectors rather than on reputation.
- **OW-ADR-0002** — parse atom frontmatter with a restricted reader instead of a
  YAML library, because the Rust YAML ecosystem offers only stale or pre-1.0
  options and a six-key header does not justify the attack surface of YAML 1.2.

### Known gaps

Stated rather than left to be discovered:

- Gate execution is not implemented. `war check` validates the record, not the
  work; a Warrant whose acceptance gates are nonsense passes.
- Preflight (§32.7) does not exist, so the verdict says WELL-FORMED rather than
  READY.
- Bound atoms (`ref =`) cannot be resolved offline; federation is Phase 4.
- Two of the nine projections in §17.5 are implemented (`full_warrant`,
  `canonical_json`).
- Contract revisions are not implemented; every compilation is revision 1.
- ADR supersession is not modelled at all — `AdrRecord` has no `supersedes`
  field — so §91.4 test 25 (supersession is acyclic) is not covered. The
  `superseded` status exists as a lifecycle state, but nothing records what
  superseded what.
- Conformance runs on one host, so §91.1 test 1's "two supported hosts" is
  satisfied by two runs rather than two architectures.
